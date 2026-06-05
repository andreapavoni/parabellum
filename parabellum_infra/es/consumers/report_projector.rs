use mini_cqrs_es::{CqrsError, EventConsumer, StoredEvent};
use parabellum_app::ports::identity::PlayerRepository;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::VillageModel;
use parabellum_game::models::army::Army;
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
    villages: PostgresVillageRepository,
    armies: PostgresArmyRepository,
    reports: PostgresReportRepository,
    players: PostgresPlayerRepository,
}

pub(super) struct SourceTargetReportContext {
    pub source: VillageModel,
    pub target: VillageModel,
    pub target_home_army: Option<Army>,
    pub target_reinforcements: Vec<Army>,
    pub source_player: String,
    pub target_player: String,
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
            villages: PostgresVillageRepository::new(pool.clone()),
            armies: PostgresArmyRepository::new(pool.clone()),
            reports: PostgresReportRepository::new(pool.clone()),
            players: PostgresPlayerRepository::new(pool),
        }
    }

    pub(super) async fn player_username(&self, player_id: Uuid) -> Result<String, CqrsError> {
        let player = self
            .players
            .get_by_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(player.username)
    }

    pub(super) async fn try_village_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<Option<VillageModel>, CqrsError> {
        match self.villages.get_by_village_id_in_tx(tx, village_id).await {
            Ok(v) => Ok(Some(v)),
            Err(_) => {
                warn!(
                    village_id,
                    "ReportProjector skipping event because village read model was not found"
                );
                Ok(None)
            }
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
        let target_home_army = self
            .armies
            .get_home_army_in_tx(tx, target_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let target_reinforcements = self
            .armies
            .list_stationed_armies_in_tx(tx, target_village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let source_player = self.player_username(source.player_id).await?;
        let target_player = self.player_username(target.player_id).await?;

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

    pub async fn process_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: &StoredEvent,
    ) -> Result<(), CqrsError> {
        if !event.aggregate_type.contains("VillageAggregate") {
            return Ok(());
        }
        let projected_report_id = Self::projected_report_id(event);

        let domain_event = event.get_payload::<VillageEvent>()?;
        if let Some(result) = self
            .project_reinforcement_report_in_tx(tx, projected_report_id, &domain_event)
            .await
        {
            return result;
        }
        if let Some(result) = self
            .project_marketplace_report_in_tx(tx, projected_report_id, &domain_event)
            .await
        {
            return result;
        }
        if let Some(result) = self
            .project_battle_report_in_tx(tx, projected_report_id, &domain_event)
            .await
        {
            return result;
        }
        if let Some(result) = self.project_read_state_in_tx(tx, &domain_event).await {
            return result;
        }

        Ok(())
    }
}

impl EventConsumer for ReportProjector {
    async fn process(&self, event: &StoredEvent) -> Result<(), CqrsError> {
        let mut dbtx = self
            .pool
            .begin()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        self.process_in_tx(&mut dbtx, event).await?;
        dbtx.commit()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(())
    }
}
