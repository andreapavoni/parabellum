//! Village ES orchestration service.
//!
//! This module is intentionally split by concern:
//! - `mod.rs`: command dispatch + scheduler tick orchestration
//! - `queries.rs`: read/query helpers consumed by adapters/web layer
//! - `scheduler.rs`: deterministic completion-command execution from scheduled payloads
//!
//! Public API remains centered on `VillageEsService`.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use mini_cqrs_es::anyhow::Result;
use mini_cqrs_es::{CqrsError, QueryRunner};
use parabellum_game::models::army::Army;
use parabellum_game::models::culture_points::required_cp;
use sqlx::PgPool;
use tracing::{info, warn};

use parabellum_app::ports::queries::{
    AcademyQueueItem, BuildingQueueItem, ExpansionCultureInfo, LeaderboardPage, MarketplaceData,
    SmithyQueueItem, TrainingQueueItem, TroopMovement, TroopMovementDirection, TroopMovementType,
    VillageArmyStateView, VillageQueues, VillageTroopMovements,
};
use parabellum_app::ports::{identity::PlayerRepository, map::MapRepository};
use parabellum_app::query_models::{MapRegionTile, VillageInfo};
use parabellum_app::villages::models::{
    MarketplaceOfferSnapshot, MarketplaceOfferStatus, ReportModel, ScheduledAction,
    ScheduledActionPayload, ScheduledActionStatus, ScheduledActionType, VillageModel,
};
use parabellum_app::villages::queries::{
    GetMarketplaceOfferById, GetOpenMarketplaceOffers, GetReportForPlayer, ListReportsForPlayer,
    ScheduledActionStatusCounts,
};
use parabellum_app::villages::repositories::{
    ArmyRepository, HeroRepository, MarketplaceRepository, ReportRepository,
    ScheduledActionRepository, VillageMovementRepository, VillageRepository,
};
use parabellum_app::villages::{
    AcceptMarketplaceOffer, AddBuilding, AttackVillage, CancelMarketplaceOffer,
    CompleteAcademyResearch, CompleteAddBuilding, CompleteArmyReturn, CompleteAttackArrival,
    CompleteDowngradeBuilding, CompleteHeroRevival, CompleteMerchantsArrival,
    CompleteMerchantsReturn, CompleteScoutArrival, CompleteSettlersArrival, CompleteSmithyResearch,
    CompleteTrainUnit, CompleteUpgradeBuilding, ConquerVillage, CreateHero, CreateMarketplaceOffer,
    DowngradeBuilding, FoundVillage, RecallReinforcements, ReinforcementArrived,
    ReleaseReinforcements, ResearchAcademy, ResearchSmithy, ReviveHero, ScoutVillage,
    SendMerchantsTransfer, SendReinforcement, SendSettlers, SetVillageResources, TrainUnits,
    UpgradeBuilding, VillageService,
};
use parabellum_game::models::map::MapField;
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::errors::{DbError, GameError};

use crate::es::lock_keys::SCHEDULED_ACTION_EXECUTION_LOCK_KEY;
use crate::es::{
    PostgresArmyRepository, PostgresHeroRepository, PostgresMarketplaceRepository,
    PostgresReportRepository, PostgresScheduledActionRepository, PostgresVillageMovementRepository,
    PostgresVillageRepository, village_cqrs_runtime,
};
use crate::identity::repositories::PostgresPlayerRepository;
use crate::map::PostgresMapRepository;

mod queries;
mod scheduler;

#[derive(Debug, Clone)]
/// ES orchestration facade for village command, scheduler, and read helper flows.
pub struct VillageEsService {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct ReinforcementContext {
    /// Village where the reinforcement is currently stationed.
    pub stationed_village_id: u32,
    /// Home/origin village of the reinforcement army.
    pub home_village_id: u32,
    /// Full army state for recall/release command construction.
    pub army: Army,
}

impl VillageEsService {
    fn map_troop_movement_type(
        movement_type: parabellum_app::villages::models::MovementType,
    ) -> TroopMovementType {
        match movement_type {
            parabellum_app::villages::models::MovementType::Attack => TroopMovementType::Attack,
            parabellum_app::villages::models::MovementType::Raid => TroopMovementType::Raid,
            parabellum_app::villages::models::MovementType::Scout => TroopMovementType::Attack,
            parabellum_app::villages::models::MovementType::Reinforcement => {
                TroopMovementType::Reinforcement
            }
            parabellum_app::villages::models::MovementType::Return => TroopMovementType::Return,
            parabellum_app::villages::models::MovementType::FoundVillage => {
                TroopMovementType::FoundVillage
            }
        }
    }

    fn as_offer_snapshot(
        offer: parabellum_app::villages::models::MarketplaceOfferModel,
    ) -> MarketplaceOfferSnapshot {
        MarketplaceOfferSnapshot {
            offer_id: offer.offer_id,
            owner_player_id: offer.owner_player_id,
            owner_village_id: offer.owner_village_id,
            offer_resources: offer.offer_resources,
            seek_resources: offer.seek_resources,
            merchants_reserved: offer.merchants_reserved,
        }
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn found_village(
        &self,
        village_id: u32,
        command: &FoundVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.found_village(village_id, command).await
    }

    pub async fn send_reinforcement(
        &self,
        village_id: u32,
        command: &SendReinforcement,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_reinforcement(village_id, command).await
    }

    pub async fn send_attack(
        &self,
        village_id: u32,
        command: &AttackVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_attack(village_id, command).await
    }

    pub async fn recall_reinforcements(
        &self,
        village_id: u32,
        command: &RecallReinforcements,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.recall_reinforcements(village_id, command).await
    }

    pub async fn release_reinforcements(
        &self,
        village_id: u32,
        command: &ReleaseReinforcements,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.release_reinforcements(village_id, command).await
    }

    pub async fn send_scout(
        &self,
        village_id: u32,
        command: &ScoutVillage,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_scout(village_id, command).await
    }

    pub async fn send_settlers(
        &self,
        village_id: u32,
        command: &SendSettlers,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_settlers(village_id, command).await
    }

    pub async fn create_hero(
        &self,
        village_id: u32,
        command: &CreateHero,
    ) -> Result<u32, CqrsError> {
        if self.player_has_alive_hero(command.player_id).await? {
            return Err(CqrsError::Domain(
                "player already has an alive hero".to_string(),
            ));
        }
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.create_hero(village_id, command).await
    }

    pub async fn revive_hero(
        &self,
        village_id: u32,
        command: &ReviveHero,
    ) -> Result<u32, CqrsError> {
        if self
            .player_has_pending_hero_revival(command.player_id)
            .await?
        {
            return Err(CqrsError::Domain(
                "hero revival already pending".to_string(),
            ));
        }
        if self.player_has_alive_hero(command.player_id).await? {
            return Err(CqrsError::Domain(
                "cannot revive while an alive hero exists".to_string(),
            ));
        }
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.revive_hero(village_id, command).await
    }

    /// Returns whether a target map field is currently an unoccupied valley.
    pub async fn is_unoccupied_valley(&self, field_id: u32) -> Result<bool, CqrsError> {
        let map_repo = PostgresMapRepository::new(self.pool.clone());
        map_repo
            .is_unoccupied_valley(field_id as i32)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn add_building(
        &self,
        village_id: u32,
        command: &AddBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.add_building(village_id, command).await
    }

    pub async fn upgrade_building(
        &self,
        village_id: u32,
        command: &UpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.upgrade_building(village_id, command).await
    }

    pub async fn downgrade_building(
        &self,
        village_id: u32,
        command: &DowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.downgrade_building(village_id, command).await
    }

    pub async fn train_units(
        &self,
        village_id: u32,
        command: &TrainUnits,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.train_units(village_id, command).await
    }

    pub async fn complete_train_unit(
        &self,
        village_id: u32,
        command: &CompleteTrainUnit,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.complete_train_unit(village_id, command).await
    }

    pub async fn research_academy(
        &self,
        village_id: u32,
        command: &ResearchAcademy,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.research_academy(village_id, command).await
    }

    pub async fn research_smithy(
        &self,
        village_id: u32,
        command: &ResearchSmithy,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.research_smithy(village_id, command).await
    }

    pub async fn complete_add_building(
        &self,
        village_id: u32,
        command: &CompleteAddBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.complete_add_building(village_id, command).await
    }

    pub async fn complete_upgrade_building(
        &self,
        village_id: u32,
        command: &CompleteUpgradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.complete_upgrade_building(village_id, command).await
    }

    pub async fn complete_downgrade_building(
        &self,
        village_id: u32,
        command: &CompleteDowngradeBuilding,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .complete_downgrade_building(village_id, command)
            .await
    }

    pub async fn send_resources(
        &self,
        village_id: u32,
        command: &SendMerchantsTransfer,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.send_resources(village_id, command).await
    }

    pub async fn create_marketplace_offer(
        &self,
        village_id: u32,
        command: &CreateMarketplaceOffer,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.create_marketplace_offer(village_id, command).await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        village_id: u32,
        player_id: uuid::Uuid,
        offer_id: uuid::Uuid,
    ) -> Result<u32, CqrsError> {
        let offer = self.get_marketplace_offer(offer_id).await?;
        if offer.status != MarketplaceOfferStatus::Open
            || offer.owner_village_id != village_id
            || offer.owner_player_id != player_id
        {
            return Err(CqrsError::domain("invalid marketplace offer cancellation"));
        }

        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .cancel_marketplace_offer(
                village_id,
                &CancelMarketplaceOffer {
                    player_id,
                    offer: Self::as_offer_snapshot(offer),
                },
            )
            .await
    }

    pub async fn accept_marketplace_offer(
        &self,
        accepting_village_id: u32,
        accepting_player_id: uuid::Uuid,
        offer_id: uuid::Uuid,
        owner_arrives_at: chrono::DateTime<chrono::Utc>,
        accepting_arrives_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<u32, CqrsError> {
        let offers = PostgresMarketplaceRepository::new(self.pool.clone());
        let Some(offer) = offers
            .claim_open_for_accept(
                offer_id,
                accepting_player_id,
                accepting_village_id,
                chrono::Utc::now(),
            )
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?
        else {
            return Err(CqrsError::domain(GameError::MarketplaceOfferNoLongerValid));
        };

        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service
            .accept_marketplace_offer(
                accepting_village_id,
                &AcceptMarketplaceOffer {
                    player_id: accepting_player_id,
                    offer: Self::as_offer_snapshot(offer),
                    owner_arrives_at,
                    accepting_arrives_at,
                },
            )
            .await
    }

    /// Executes the village resource utility command through the ES runtime.
    pub async fn set_village_resources(
        &self,
        village_id: u32,
        command: &SetVillageResources,
    ) -> Result<u32, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        service.set_village_resources(village_id, command).await
    }

    /// Executes due scheduled actions by dispatching completion commands.
    ///
    /// Status transitions are persisted for each action (`completed` or `failed`).
    pub async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, CqrsError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(SCHEDULED_ACTION_EXECUTION_LOCK_KEY)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        if !acquired {
            info!(
                action = "scheduler_skip_locked",
                "scheduled action execution lock is held; skipping tick"
            );
            return Ok(0);
        }

        let actions = PostgresScheduledActionRepository::new(self.pool.clone())
            .take_due_pending(before_or_equal, limit)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let claimed = actions.len();
        if claimed > 0 {
            info!(
                action = "scheduler_claim_due",
                claimed,
                limit,
                before_or_equal = %before_or_equal,
                "claimed due scheduled actions"
            );
        }

        let result = self.process_actions(&actions).await;
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(SCHEDULED_ACTION_EXECUTION_LOCK_KEY)
            .execute(&mut *conn)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        result
    }

    pub async fn process_actions(
        &self,
        actions: &Vec<ScheduledAction>,
    ) -> Result<usize, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        let mut processed = 0usize;
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());

        for action in actions {
            let result = scheduler::execute_action(self, &service, action).await;
            let next_status = if result.is_ok() {
                ScheduledActionStatus::Completed
            } else {
                ScheduledActionStatus::Failed
            };
            repo.update_status(action.id, next_status)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            if let Err(err) = &result {
                warn!(
                    action = "scheduler_action_failed",
                    action_id = %action.id,
                    action_type = ?action.action_type,
                    error = %err,
                    "scheduled action marked failed"
                );
            } else {
                info!(
                    action = "scheduler_action_completed",
                    action_id = %action.id,
                    action_type = ?action.action_type,
                    "scheduled action completed"
                );
            }
            result?;
            processed += 1;
        }
        Ok(processed)
    }

}
