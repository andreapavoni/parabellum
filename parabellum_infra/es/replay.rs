use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::VillageEvent;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
use crate::es::{PostgresEventStore, ReportProjector, VillageProjector};

const DEFAULT_BATCH_SIZE: i64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayTarget {
    Village,
    Reports,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
    DryRun,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRequest {
    pub target: ReplayTarget,
    pub mode: ReplayMode,
    pub from_global_seq: i64,
    pub to_global_seq: Option<i64>,
    pub aggregate_id: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReplaySummary {
    pub scanned: usize,
    pub applied: usize,
    pub skipped: usize,
    pub first_global_seq: Option<i64>,
    pub last_global_seq: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ReplayService {
    pool: PgPool,
    event_store: PostgresEventStore,
}

impl ReplayService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            event_store: PostgresEventStore::new(pool.clone()),
            pool,
        }
    }

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
                self.update_sequence_bounds(&mut summary, event);
                summary.scanned += 1;

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
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(CqrsError::domain_source)?;
        let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(SCHEDULED_ACTION_EXECUTION_LOCK_KEY)
            .fetch_one(&mut *conn)
            .await
            .map_err(CqrsError::domain_source)?;
        if !acquired {
            warn!("replay lock already held by another process");
            return Err(CqrsError::EventStore(
                "replay lock already held by another process".to_string(),
            ));
        }

        let replay_result = self.run_full_replay(request).await;
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(SCHEDULED_ACTION_EXECUTION_LOCK_KEY)
            .execute(&mut *conn)
            .await
            .map_err(CqrsError::domain_source)?;
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
                self.update_sequence_bounds(&mut summary, event);
                summary.scanned += 1;

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
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(CqrsError::domain_source)?;

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

        tx.commit()
            .await
            .map_err(CqrsError::domain_source)
    }

    fn update_sequence_bounds(&self, summary: &mut ReplaySummary, event: &StoredEvent) {
        let Some(global_seq) = event.global_sequence else {
            return;
        };
        summary.first_global_seq = Some(
            summary
                .first_global_seq
                .map_or(global_seq, |current| current.min(global_seq)),
        );
        summary.last_global_seq = Some(
            summary
                .last_global_seq
                .map_or(global_seq, |current| current.max(global_seq)),
        );
    }
}

fn accepts_event(target: ReplayTarget, event: &StoredEvent) -> Result<bool, CqrsError> {
    if !event.aggregate_type.contains("VillageAggregate") {
        return Ok(false);
    }

    let domain_event = event.get_payload::<VillageEvent>()?;
    Ok(match target {
        ReplayTarget::Village | ReplayTarget::All => true,
        ReplayTarget::Reports => is_report_event(&domain_event),
    })
}

fn is_report_event(event: &VillageEvent) -> bool {
    matches!(
        event,
        VillageEvent::ReinforcementArrived { .. }
            | VillageEvent::MerchantsArrived { .. }
            | VillageEvent::ScoutBattleResolved { .. }
            | VillageEvent::AttackBattleResolved { .. }
    )
}
