use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::projection_repositories::{
    ArmyListFilter, ArmyState, ProjectedReport, ReportKind,
};
use parabellum_game::models::army::Army;
use parabellum_types::reports::ReportPayload;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::warn;
use uuid::Uuid;

use crate::es::{PostgresArmyRepository, PostgresReportRepository, PostgresVillageRepository};
use crate::identity::repositories::PostgresPlayerRepository;

mod battle;
mod marketplace;
mod read_state;
mod reinforcements;

#[derive(Debug, Clone)]
pub struct ReportProjector {
    pool: PgPool,
    event_at: chrono::DateTime<chrono::Utc>,
    villages: PostgresVillageRepository,
    armies: PostgresArmyRepository,
    reports: PostgresReportRepository,
    players: PostgresPlayerRepository,
}

/// Read-model context needed to build source-target report payloads.
pub(super) struct SourceTargetReportContext {
    pub source: VillageModel,
    pub target: VillageModel,
    pub target_home_army: Option<Army>,
    pub target_reinforcements: Vec<Army>,
    pub source_player: String,
    pub target_player: String,
}

/// Fully prepared report projection before persistence.
pub(super) struct ReportProjection {
    pub id: Uuid,
    pub kind: ReportKind,
    pub payload: ReportPayload,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
    pub audience_player_ids: Vec<Uuid>,
}

impl ReportProjection {
    /// Builds a report projection using source/target village context.
    pub fn source_target(
        id: Uuid,
        kind: ReportKind,
        payload: ReportPayload,
        context: &SourceTargetReportContext,
        source_village_id: u32,
        target_village_id: u32,
        audience_player_ids: Vec<Uuid>,
    ) -> Self {
        Self {
            id,
            kind,
            payload,
            actor_player_id: context.source.player_id,
            actor_village_id: Some(source_village_id),
            target_player_id: Some(context.target.player_id),
            target_village_id: Some(target_village_id),
            audience_player_ids,
        }
    }
}

impl ReportProjector {
    fn projected_report_id(event: &StoredEvent) -> Uuid {
        match event.global_sequence {
            Some(seq) => Uuid::from_u128(0xa11ce000000000000000000000000000u128 | seq as u128),
            None => Uuid::new_v4(),
        }
    }

    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            event_at: chrono::DateTime::UNIX_EPOCH,
            villages: PostgresVillageRepository::new(crate::ProjectionDb::new(pool.clone())),
            armies: PostgresArmyRepository::new(crate::ProjectionDb::new(pool.clone())),
            reports: PostgresReportRepository::new(crate::ProjectionDb::new(pool.clone())),
            players: PostgresPlayerRepository::new(pool),
        }
    }

    pub(super) async fn player_username(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        player_id: Uuid,
    ) -> Result<String, CqrsError> {
        self.players
            .username_in_tx(tx, player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    pub(super) async fn try_village_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<Option<VillageModel>, CqrsError> {
        match self.villages.get_by_village_id_in_tx(tx, village_id).await {
            Ok(v) => Ok(Some(v)),
            Err(parabellum_types::errors::ApplicationError::Db(
                parabellum_types::errors::DbError::VillageNotFound(_),
            )) => {
                warn!(
                    village_id,
                    "ReportProjector skipping event because village read model was not found"
                );
                Ok(None)
            }
            Err(error) => Err(CqrsError::domain_source(error)),
        }
    }

    pub(super) async fn source_target_context_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        source_village_id: u32,
        target_village_id: u32,
    ) -> Result<Option<SourceTargetReportContext>, CqrsError> {
        let Some(source) = self.try_village_in_tx(tx, source_village_id).await? else {
            return Ok(None);
        };
        let Some(target) = self.try_village_in_tx(tx, target_village_id).await? else {
            return Ok(None);
        };
        let mut target_home_armies = self
            .armies
            .list_armies_in_tx(
                tx,
                ArmyListFilter::new()
                    .home_village(target_village_id)
                    .current_village(target_village_id)
                    .state(ArmyState::Home)
                    .limit(1),
            )
            .await
            .map_err(CqrsError::domain_source)?;
        let target_home_army = target_home_armies.pop();
        let target_reinforcements = self
            .armies
            .list_armies_in_tx(
                tx,
                ArmyListFilter::new()
                    .current_village(target_village_id)
                    .state(ArmyState::Stationed),
            )
            .await
            .map_err(CqrsError::domain_source)?;
        let source_player = self.player_username(tx, source.player_id).await?;
        let target_player = self.player_username(tx, target.player_id).await?;

        Ok(Some(SourceTargetReportContext {
            source,
            target,
            target_home_army,
            target_reinforcements,
            source_player,
            target_player,
        }))
    }

    pub(super) fn audience_with_target(actor_player_id: Uuid, target_player_id: Uuid) -> Vec<Uuid> {
        let mut audiences = vec![actor_player_id];
        if target_player_id != actor_player_id {
            audiences.push(target_player_id);
        }
        audiences
    }

    pub(super) async fn project_report_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        projection: ReportProjection,
    ) -> Result<(), CqrsError> {
        self.reports
            .add_projected_at_in_tx(
                tx,
                &ProjectedReport {
                    id: projection.id,
                    report_type: projection.kind.as_str().to_string(),
                    payload: serde_json::to_value(projection.payload)
                        .map_err(CqrsError::Serialization)?,
                    actor_player_id: projection.actor_player_id,
                    actor_village_id: projection.actor_village_id,
                    target_player_id: projection.target_player_id,
                    target_village_id: projection.target_village_id,
                },
                &projection.audience_player_ids,
                self.event_at,
            )
            .await
            .map_err(CqrsError::domain_source)?;

        Ok(())
    }

    pub async fn process_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &StoredEvent,
    ) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }
        let mut projector = self.clone();
        projector.event_at = parabellum_app::villages::effective_event_time(event)?;
        projector.villages = projector
            .villages
            .at_event_time(parabellum_app::villages::effective_event_time(event)?);
        let projected_report_id = Self::projected_report_id(event);

        let domain_event = event.get_payload::<VillageEvent>()?;
        if let Some(result) = projector
            .project_reinforcement_report_in_tx(tx, projected_report_id, &domain_event)
            .await
        {
            return result;
        }
        if let Some(result) = projector
            .project_marketplace_report_in_tx(tx, projected_report_id, &domain_event)
            .await
        {
            return result;
        }
        if let Some(result) = projector
            .project_battle_report_in_tx(tx, projected_report_id, &domain_event)
            .await
        {
            return result;
        }
        if let Some(result) = projector.project_read_state_in_tx(tx, &domain_event).await {
            return result;
        }

        Ok(())
    }
}

impl EventConsumer for ReportProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        let mut dbtx = self.pool.begin().await.map_err(CqrsError::domain_source)?;
        self.process_in_tx(&mut dbtx, event).await?;
        dbtx.commit().await.map_err(CqrsError::domain_source)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn optional_village_skips_absence_but_propagates_storage_failure() {
        let (pool, _database) = crate::test_support::IsolatedTestDatabase::create()
            .await
            .unwrap();
        let projector = ReportProjector::new(pool.clone());
        let mut tx = pool.begin().await.unwrap();
        assert!(
            projector
                .try_village_in_tx(&mut tx, 123)
                .await
                .unwrap()
                .is_none()
        );
        sqlx::query("ALTER TABLE rm_village RENAME TO unavailable_village")
            .execute(&mut *tx)
            .await
            .unwrap();
        let error = projector.try_village_in_tx(&mut tx, 123).await.unwrap_err();
        assert!(matches!(error, CqrsError::DomainSource(_)));
        tx.rollback().await.unwrap();
        pool.close().await;
    }
}
