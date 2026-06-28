//! Replay execution loops and projection reset logic.

use mini_cqrs_es::{CqrsError, EventConsumer};
use parabellum_app::villages::VillageEvent;
use tracing::{info, warn};

use crate::es::advisory_lock::AdvisoryLock;
use crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
use crate::es::{ReportProjector, VillageProjector};

use super::filters::{accepts_event, is_report_event};
use super::{ReplayMode, ReplayRequest, ReplayService, ReplaySummary, ReplayTarget};

const DEFAULT_BATCH_SIZE: i64 = 500;

impl ReplayService {
    pub async fn replay(&self, request: ReplayRequest) -> Result<ReplaySummary, CqrsError> {
        info!(
            mode = ?request.mode,
            target = ?request.target,
            from_global_seq = request.from_global_seq,
            to_global_seq = request.to_global_seq,
            aggregate_id = request.aggregate_id.as_deref().unwrap_or(""),
            "replay requested"
        );
        match request.mode {
            ReplayMode::DryRun => self.dry_run(request).await,
            ReplayMode::Full => self.full_replay(request).await,
        }
    }

    pub async fn dry_run(&self, request: ReplayRequest) -> Result<ReplaySummary, CqrsError> {
        let to_global_seq = self.resolve_upper_bound(request.to_global_seq).await?;
        let mut summary = ReplaySummary::default();
        let mut from_global_seq = request.from_global_seq.max(1);

        loop {
            let events = self
                .event_store
                .load_events_by_global_seq(
                    from_global_seq,
                    to_global_seq,
                    request.aggregate_id.as_deref(),
                    DEFAULT_BATCH_SIZE,
                )
                .await?;
            if events.is_empty() {
                break;
            }

            for event in &events {
                summary.record_scanned_event(event);

                if accepts_event(request.target, event)? {
                    summary.applied += 1;
                } else {
                    summary.skipped += 1;
                }
            }

            let Some(last_global_seq) = events.last().and_then(|event| event.global_sequence)
            else {
                break;
            };
            from_global_seq = last_global_seq + 1;
        }

        Ok(summary)
    }

    async fn full_replay(&self, request: ReplayRequest) -> Result<ReplaySummary, CqrsError> {
        let Some(lock) =
            AdvisoryLock::try_acquire(&self.pool, SCHEDULED_ACTION_EXECUTION_LOCK_KEY).await?
        else {
            warn!("replay lock already held by another process");
            return Err(CqrsError::EventStore(
                "replay lock already held by another process".to_string(),
            ));
        };

        let replay_result = self.run_full_replay(request).await;
        lock.release().await?;
        info!("replay lock released");
        replay_result
    }

    async fn run_full_replay(&self, request: ReplayRequest) -> Result<ReplaySummary, CqrsError> {
        let to_global_seq = self.resolve_upper_bound(request.to_global_seq).await?;
        self.reset_projection_target(request.target).await?;
        info!(target = ?request.target, "replay projections reset");

        let village_projector = VillageProjector::new_with_options(self.pool.clone(), false);
        let report_projector = ReportProjector::new(self.pool.clone());

        let mut summary = ReplaySummary::default();
        let mut from_global_seq = request.from_global_seq.max(1);
        loop {
            let events = self
                .event_store
                .load_events_by_global_seq(
                    from_global_seq,
                    to_global_seq,
                    request.aggregate_id.as_deref(),
                    DEFAULT_BATCH_SIZE,
                )
                .await?;
            if events.is_empty() {
                break;
            }

            for event in &events {
                summary.record_scanned_event(event);

                if !accepts_event(request.target, event)? {
                    summary.skipped += 1;
                    continue;
                }

                if matches!(request.target, ReplayTarget::Reports | ReplayTarget::All)
                    && is_report_event(&event.get_payload::<VillageEvent>()?)
                {
                    report_projector.process(event).await?;
                }
                if matches!(request.target, ReplayTarget::Village | ReplayTarget::All) {
                    village_projector.process(event).await?;
                }

                summary.applied += 1;
            }

            let Some(last_global_seq) = events.last().and_then(|event| event.global_sequence)
            else {
                break;
            };
            from_global_seq = last_global_seq + 1;
        }
        Ok(summary)
    }

    async fn resolve_upper_bound(&self, explicit: Option<i64>) -> Result<Option<i64>, CqrsError> {
        if explicit.is_some() {
            return Ok(explicit);
        }

        let upper = sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(global_seq) FROM es_events")
            .fetch_one(&self.pool)
            .await
            .map_err(CqrsError::domain_source)?;
        Ok(upper)
    }

    async fn reset_projection_target(&self, target: ReplayTarget) -> Result<(), CqrsError> {
        let mut tx = self.pool.begin().await.map_err(CqrsError::domain_source)?;

        if matches!(target, ReplayTarget::Reports | ReplayTarget::All) {
            sqlx::query("DELETE FROM rm_report_reads")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
            sqlx::query("DELETE FROM rm_reports")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
        }

        if matches!(target, ReplayTarget::Village | ReplayTarget::All) {
            sqlx::query("DELETE FROM rm_marketplace_offers")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
            sqlx::query("DELETE FROM rm_village_movements")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
            sqlx::query("DELETE FROM rm_armies")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
            sqlx::query("DELETE FROM rm_heroes")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
            sqlx::query("DELETE FROM rm_village")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
            sqlx::query("UPDATE rm_map_fields SET village_id = NULL, player_id = NULL")
                .execute(&mut *tx)
                .await
                .map_err(CqrsError::domain_source)?;
        }

        tx.commit().await.map_err(CqrsError::domain_source)
    }
}
