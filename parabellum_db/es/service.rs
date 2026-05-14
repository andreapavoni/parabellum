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
use parabellum_app::query_models::{MapRegionTile, VillageInfo};
use parabellum_app::repository::MapRepository;
use parabellum_app::repository::PlayerRepository;
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

#[derive(Debug, Clone)]
/// ES orchestration facade for village command, scheduler, and read helper flows.
pub struct VillageEsService {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct ReinforcementContext {
    pub stationed_village_id: u32,
    pub home_village_id: u32,
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
            let result = self.execute_action(&service, &action).await;
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

    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, CqrsError> {
        let repo = PostgresVillageMovementRepository::new(self.pool.clone());
        let movements = repo
            .list_by_village_id(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        for movement in movements {
            let origin_model = self.get_village(movement.origin_village_id).await.ok();
            let target_model = self.get_village(movement.target_village_id).await.ok();

            let mapped = TroopMovement {
                job_id: movement.movement_id,
                movement_type: Self::map_troop_movement_type(movement.movement_type),
                direction: match movement.direction {
                    parabellum_app::villages::models::MovementDirection::Incoming => {
                        TroopMovementDirection::Incoming
                    }
                    parabellum_app::villages::models::MovementDirection::Outgoing => {
                        TroopMovementDirection::Outgoing
                    }
                },
                origin_village_id: movement.origin_village_id,
                origin_village_name: movement
                    .origin_village_name
                    .or_else(|| origin_model.as_ref().map(|v| v.village_name.clone())),
                origin_player_id: movement.origin_player_id,
                origin_position: movement
                    .origin_position
                    .or_else(|| origin_model.as_ref().map(|v| v.position.clone()))
                    .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
                target_village_id: movement.target_village_id,
                target_village_name: movement
                    .target_village_name
                    .or_else(|| target_model.as_ref().map(|v| v.village_name.clone())),
                target_player_id: movement
                    .target_player_id
                    .or_else(|| target_model.as_ref().map(|v| v.player_id))
                    .unwrap_or(movement.origin_player_id),
                target_position: movement
                    .target_position
                    .or_else(|| target_model.as_ref().map(|v| v.position.clone()))
                    .unwrap_or(parabellum_types::map::Position { x: 0, y: 0 }),
                arrives_at: movement.arrives_at,
                time_seconds: movement.time_seconds.unwrap_or(0),
                units: movement.units,
                tribe: movement
                    .tribe
                    .or_else(|| origin_model.as_ref().map(|v| v.tribe.clone()))
                    .unwrap_or(parabellum_types::tribe::Tribe::Nature),
            };
            match mapped.direction {
                TroopMovementDirection::Outgoing => outgoing.push(mapped),
                TroopMovementDirection::Incoming => incoming.push(mapped),
            };
        }
        outgoing.sort_by_key(|m| m.arrives_at);
        incoming.sort_by_key(|m| m.arrives_at);
        Ok(VillageTroopMovements { outgoing, incoming })
    }

    pub async fn get_village(&self, village_id: u32) -> Result<VillageModel, CqrsError> {
        let repo = PostgresVillageRepository::new(self.pool.clone());
        repo.get_by_village_id(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_hero(
        &self,
        hero_id: uuid::Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, CqrsError> {
        let repo = PostgresHeroRepository::new(self.pool.clone());
        repo.get_by_id(hero_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn player_has_alive_hero(&self, player_id: uuid::Uuid) -> Result<bool, CqrsError> {
        let repo: Arc<dyn HeroRepository> =
            Arc::new(PostgresHeroRepository::new(self.pool.clone()));
        repo.has_alive_for_player(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn player_has_pending_hero_revival(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<bool, CqrsError> {
        PostgresScheduledActionRepository::new(self.pool.clone())
            .has_pending_hero_revival_for_player(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn list_villages_by_player_id(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Vec<VillageModel>, CqrsError> {
        let repo = PostgresVillageRepository::new(self.pool.clone());
        repo.list_by_player_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_expansion_culture_info(
        &self,
        player_id: uuid::Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<ExpansionCultureInfo, CqrsError> {
        let player_repo = PostgresPlayerRepository::new(self.pool.clone());
        let village_repo = PostgresVillageRepository::new(self.pool.clone());

        let player = player_repo
            .get_by_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let player_culture_points_production = player_repo
            .get_total_culture_points_production(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let villages = village_repo
            .list_by_player_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let village = villages
            .iter()
            .find(|v| v.village_id == village_id)
            .ok_or_else(|| {
                CqrsError::EventStore(DbError::VillageNotFound(village_id).to_string())
            })?;

        let speed = match server_speed {
            1 => parabellum_types::common::Speed::X1,
            2 => parabellum_types::common::Speed::X2,
            3 => parabellum_types::common::Speed::X3,
            5 => parabellum_types::common::Speed::X5,
            10 => parabellum_types::common::Speed::X10,
            _ => parabellum_types::common::Speed::X1,
        };
        let next_cp_required = required_cp(speed, villages.len() + 1);

        Ok(ExpansionCultureInfo {
            village_culture_points: village.culture_points,
            village_culture_points_production: village.culture_points_production,
            player_culture_points: player.culture_points as u32,
            player_culture_points_production,
            next_cp_required,
        })
    }

    pub async fn find_reinforcement_context(
        &self,
        army_id: uuid::Uuid,
    ) -> Result<ReinforcementContext, CqrsError> {
        let army_repo: Arc<dyn ArmyRepository> =
            Arc::new(PostgresArmyRepository::new(self.pool.clone()));
        if let Some((stationed_village_id, army)) = army_repo
            .find_stationed_context_by_army_id(army_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?
        {
            return Ok(ReinforcementContext {
                stationed_village_id,
                home_village_id: army.village_id,
                army,
            });
        }

        Err(CqrsError::EventStore(
            DbError::ArmyNotFound(army_id).to_string(),
        ))
    }

    pub async fn get_village_training_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.list_active_by_village_and_type(village_id, ScheduledActionType::TrainUnit)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_village_building_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        let mut actions = Vec::new();
        for action_type in [
            ScheduledActionType::AddBuilding,
            ScheduledActionType::UpgradeBuilding,
            ScheduledActionType::DowngradeBuilding,
        ] {
            let mut typed = repo
                .list_active_by_village_and_type(village_id, action_type)
                .await
                .map_err(|e| CqrsError::EventStore(e.to_string()))?;
            actions.append(&mut typed);
        }
        actions.sort_by_key(|it| it.execute_at);
        Ok(actions)
    }

    pub async fn get_village_smithy_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.list_active_by_village_and_type(village_id, ScheduledActionType::ResearchSmithy)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_village_academy_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.list_active_by_village_and_type(village_id, ScheduledActionType::ResearchAcademy)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, CqrsError> {
        let mut building = Vec::new();
        let building_actions = self.get_village_building_queue(village_id).await?;
        for action in building_actions {
            let Ok(payload) = serde_json::from_value::<ScheduledActionPayload>(action.payload)
            else {
                continue;
            };
            let (slot_id, building_name, target_level) = match payload {
                ScheduledActionPayload::AddBuilding {
                    slot_id,
                    building_name,
                    level,
                    ..
                }
                | ScheduledActionPayload::UpgradeBuilding {
                    slot_id,
                    building_name,
                    level,
                    ..
                }
                | ScheduledActionPayload::DowngradeBuilding {
                    slot_id,
                    building_name,
                    level,
                    ..
                } => (slot_id, building_name, level),
                _ => continue,
            };
            building.push(BuildingQueueItem {
                job_id: action.id,
                slot_id,
                building_name,
                target_level,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        building.sort_by_key(|it| it.finishes_at);

        let mut training = Vec::new();
        let training_actions = self.get_village_training_queue(village_id).await?;
        for action in training_actions {
            let Ok(ScheduledActionPayload::TrainUnit {
                slot_id,
                unit,
                quantity_remaining,
                time_per_unit,
                ..
            }) = serde_json::from_value::<ScheduledActionPayload>(action.payload)
            else {
                continue;
            };
            training.push(TrainingQueueItem {
                job_id: action.id,
                slot_id,
                unit,
                quantity: quantity_remaining,
                time_per_unit,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        training.sort_by_key(|it| it.finishes_at);

        let mut academy = Vec::new();
        let academy_actions = self.get_village_academy_queue(village_id).await?;
        for action in academy_actions {
            let Ok(ScheduledActionPayload::ResearchAcademy { unit, .. }) =
                serde_json::from_value::<ScheduledActionPayload>(action.payload)
            else {
                continue;
            };
            academy.push(AcademyQueueItem {
                job_id: action.id,
                unit,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        academy.sort_by_key(|it| it.finishes_at);

        let mut smithy = Vec::new();
        let smithy_actions = self.get_village_smithy_queue(village_id).await?;
        for action in smithy_actions {
            let Ok(ScheduledActionPayload::ResearchSmithy { unit, .. }) =
                serde_json::from_value::<ScheduledActionPayload>(action.payload)
            else {
                continue;
            };
            smithy.push(SmithyQueueItem {
                job_id: action.id,
                unit,
                status: action.status,
                finishes_at: action.execute_at,
            });
        }
        smithy.sort_by_key(|it| it.finishes_at);

        Ok(VillageQueues {
            building,
            training,
            academy,
            smithy,
        })
    }

    /// Returns scheduled-action status counters for a village and action type.
    ///
    /// If `status_filter` is provided, only that status contributes to counters.
    pub async fn get_village_scheduled_action_status_counts(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.count_by_village_and_type(village_id, action_type, status_filter)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_open_marketplace_offers(
        &self,
    ) -> Result<Vec<parabellum_app::villages::models::MarketplaceOfferModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetOpenMarketplaceOffers {
                repository: Arc::new(PostgresMarketplaceRepository::new(self.pool.clone())),
            })
            .await
    }

    pub async fn get_marketplace_offer(
        &self,
        offer_id: uuid::Uuid,
    ) -> Result<parabellum_app::villages::models::MarketplaceOfferModel, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetMarketplaceOfferById {
                repository: Arc::new(PostgresMarketplaceRepository::new(self.pool.clone())),
                offer_id,
            })
            .await
    }

    pub async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, CqrsError> {
        let marketplace_repo = PostgresMarketplaceRepository::new(self.pool.clone());
        let all_open_models = self.get_open_marketplace_offers().await?;

        let to_offer =
            |m: parabellum_app::villages::models::MarketplaceOfferModel| MarketplaceOffer {
                id: m.offer_id,
                player_id: m.owner_player_id,
                village_id: m.owner_village_id,
                offer_resources: m.offer_resources,
                seek_resources: m.seek_resources,
                merchants_required: m.merchants_reserved,
                created_at: m.created_at,
            };

        let outgoing_merchants = marketplace_repo
            .list_active_outgoing(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let incoming_merchants = marketplace_repo
            .list_active_incoming(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let village_ids = all_open_models
            .iter()
            .map(|m| m.owner_village_id)
            .chain(
                outgoing_merchants
                    .iter()
                    .flat_map(|m| [m.origin_village_id, m.destination_village_id]),
            )
            .chain(
                incoming_merchants
                    .iter()
                    .flat_map(|m| [m.origin_village_id, m.destination_village_id]),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let village_info = self.get_village_info_by_ids(village_ids).await?;

        Ok(MarketplaceData {
            own_offers: all_open_models
                .iter()
                .filter(|m| m.owner_village_id == village_id)
                .cloned()
                .map(to_offer)
                .collect(),
            global_offers: all_open_models
                .into_iter()
                .filter(|m| m.owner_village_id != village_id)
                .map(to_offer)
                .collect(),
            outgoing_merchants,
            incoming_merchants,
            village_info,
        })
    }

    pub async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, CqrsError> {
        let repo = PostgresArmyRepository::new(self.pool.clone());
        let home_army = repo
            .get_home_army(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let reinforcements = repo
            .list_stationed_armies(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        let deployed_armies = repo
            .list_deployed_armies(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(VillageArmyStateView {
            home_army,
            reinforcements,
            deployed_armies,
        })
    }

    pub async fn get_village_info_by_ids(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, VillageInfo>, CqrsError> {
        if village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let repo = PostgresVillageRepository::new(self.pool.clone());
        let villages = repo
            .list_by_village_ids(&village_ids)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        Ok(villages
            .into_iter()
            .map(|v| {
                (
                    v.village_id,
                    VillageInfo {
                        id: v.village_id,
                        name: v.village_name,
                        position: v.position,
                    },
                )
            })
            .collect())
    }

    pub async fn get_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<LeaderboardPage, CqrsError> {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let offset = (page - 1) * per_page;
        let repo = PostgresPlayerRepository::new(self.pool.clone());
        let (entries, total_players) = repo
            .leaderboard_page(offset, per_page)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;
        Ok(LeaderboardPage {
            entries,
            total_players,
        })
    }

    pub async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<MapRegionTile>, CqrsError> {
        let repo = PostgresMapRepository::new(self.pool.clone());
        repo.get_region(center_x, center_y, radius, world_size)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_map_field(&self, field_id: u32) -> Result<MapField, CqrsError> {
        let repo = PostgresMapRepository::new(self.pool.clone());
        repo.get_field_by_id(field_id as i32)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<MapRegionTile>, CqrsError> {
        let repo = PostgresMapRepository::new(self.pool.clone());
        repo.get_region_tile_by_field_id(field_id as i32)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn list_reports_for_player(
        &self,
        player_id: uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&ListReportsForPlayer {
                repository: Arc::new(PostgresReportRepository::new(self.pool.clone()))
                    as Arc<dyn ReportRepository>,
                player_id,
                limit,
            })
            .await
    }

    pub async fn get_report_for_player(
        &self,
        report_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<Option<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetReportForPlayer {
                repository: Arc::new(PostgresReportRepository::new(self.pool.clone()))
                    as Arc<dyn ReportRepository>,
                report_id,
                player_id,
            })
            .await
    }

    pub async fn mark_report_as_read(
        &self,
        report_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<(), CqrsError> {
        let repo = PostgresReportRepository::new(self.pool.clone());
        repo.mark_as_read(report_id, player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    /// Returns the number of scheduled actions for one exact status.
    pub async fn get_village_scheduled_action_status_count(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status: ScheduledActionStatus,
    ) -> Result<usize, CqrsError> {
        let counts = self
            .get_village_scheduled_action_status_counts(village_id, action_type, Some(status))
            .await?;
        Ok(match status {
            ScheduledActionStatus::Pending => counts.pending,
            ScheduledActionStatus::Processing => counts.processing,
            ScheduledActionStatus::Completed => counts.completed,
            ScheduledActionStatus::Failed => counts.failed,
        })
    }

    /// Maps one scheduled action payload to its deterministic completion command.
    async fn execute_action(
        &self,
        service: &VillageService<'_, crate::es::VillageCqrsRuntime>,
        action: &parabellum_app::villages::models::ScheduledAction,
    ) -> Result<(), CqrsError> {
        let payload: ScheduledActionPayload =
            serde_json::from_value(action.payload.clone()).map_err(CqrsError::Serialization)?;
        match payload {
            ScheduledActionPayload::ReinforcementArrival {
                movement_id,
                army_id,
                player_id,
                source_village_id,
                target_village_id,
                army,
                arrives_at,
            } => {
                let command = ReinforcementArrived {
                    movement_id,
                    army_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    army,
                    arrives_at,
                };
                service
                    .reinforcement_arrived(source_village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::SettlersArrival {
                action_id,
                movement_id,
                army_id,
                village_id: _,
                source_village_id,
                target_village_id,
                target_position,
                player_id,
                village_name,
                tribe,
                arrives_at,
            } => {
                let field_exists: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rm_map_fields WHERE id = $1)")
                        .bind(target_village_id as i32)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                let can_found = if field_exists {
                    let claim = sqlx::query(
                        r#"
                        UPDATE rm_map_fields
                        SET village_id = $2,
                            player_id = $3,
                            updated_at = NOW()
                        WHERE id = $1
                          AND village_id IS NULL
                        "#,
                    )
                    .bind(target_village_id as i32)
                    .bind(target_village_id as i32)
                    .bind(player_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    claim.rows_affected() > 0
                } else {
                    true
                };

                if can_found {
                    let command = CompleteSettlersArrival {
                        action_id,
                        movement_id,
                        army_id,
                        player_id,
                        source_village_id,
                        target_village_id,
                        target_position: target_position.clone(),
                        village_name: village_name.clone(),
                        tribe: tribe.clone(),
                        arrives_at,
                    };
                    service
                        .complete_settlers_arrival(source_village_id, &command)
                        .await?;

                    let found = FoundVillage {
                        village_name,
                        position: target_position,
                        tribe,
                        player_id,
                        buildings: vec![],
                    };
                    if let Err(err) = service.found_village(target_village_id, &found).await {
                        let is_already_founded = err.to_string().contains("is already founded");
                        if !is_already_founded {
                            return Err(err);
                        }
                    }
                } else {
                    let army_repo: Arc<dyn ArmyRepository> =
                        Arc::new(PostgresArmyRepository::new(self.pool.clone()));
                    let army = army_repo
                        .get_moving_army(army_id)
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                    let source = self.get_village(source_village_id).await?;
                    let cfg = parabellum_app::config::Config::from_env();
                    let travel_secs = source.position.calculate_travel_time_secs(
                        target_position.clone(),
                        army.speed(),
                        cfg.world_size as i32,
                        cfg.speed as u8,
                    ) as i64;
                    let returns_at =
                        arrives_at + chrono::Duration::seconds(std::cmp::max(1, travel_secs));
                    let return_action_id = uuid::Uuid::new_v4();
                    PostgresScheduledActionRepository::new(self.pool.clone())
                        .add(&ScheduledAction {
                            id: return_action_id,
                            action_type: ScheduledActionPayload::ArmyReturn {
                                action_id: return_action_id,
                                movement_id,
                                army_id,
                                village_id: source_village_id,
                                source_village_id,
                                target_village_id,
                                player_id,
                                army: army.clone(),
                                bounty: None,
                                returns_at,
                            }
                            .action_type(),
                            execute_at: returns_at,
                            payload: serde_json::to_value(ScheduledActionPayload::ArmyReturn {
                                action_id: return_action_id,
                                movement_id,
                                army_id,
                                village_id: source_village_id,
                                source_village_id,
                                target_village_id,
                                player_id,
                                army,
                                bounty: None,
                                returns_at,
                            })
                            .map_err(CqrsError::Serialization)?,
                            status: ScheduledActionStatus::Pending,
                        })
                        .await
                        .map_err(|e| CqrsError::EventStore(e.to_string()))?;
                }
            }
            ScheduledActionPayload::AttackArrival {
                action_id,
                movement_id,
                army_id,
                return_action_id,
                village_id: _,
                source_village_id,
                target_village_id,
                player_id,
                army,
                attack_type,
                catapult_targets,
                arrives_at,
                returns_at,
            } => {
                let command = CompleteAttackArrival {
                    movement_id,
                    army_id,
                    action_id,
                    return_action_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    army: army.clone(),
                    attack_type: attack_type.clone(),
                    catapult_targets,
                    arrives_at,
                    returns_at,
                };
                service
                    .complete_attack_arrival(source_village_id, &command)
                    .await?;

                let has_chief = army.units().get(8) > 0;
                if matches!(attack_type, parabellum_types::battle::AttackType::Normal) && has_chief
                {
                    let target = self.get_village(target_village_id).await?;
                    if target.loyalty == 0 {
                        let conquer = ConquerVillage {
                            player_id,
                            village_id: target_village_id,
                        };
                        service.conquer_village(target_village_id, &conquer).await?;
                    }
                }
            }
            ScheduledActionPayload::ArmyReturn {
                action_id,
                movement_id,
                army_id,
                village_id: _,
                source_village_id,
                target_village_id,
                player_id,
                army,
                returns_at,
                bounty,
            } => {
                let command = CompleteArmyReturn {
                    action_id,
                    movement_id,
                    army_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    army,
                    bounty,
                    returns_at,
                };
                service
                    .complete_army_return(source_village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::ScoutArrival {
                action_id,
                movement_id,
                army_id,
                return_action_id,
                village_id: _,
                source_village_id,
                target_village_id,
                player_id,
                army,
                target,
                attack_type,
                arrives_at,
                returns_at,
            } => {
                let command = CompleteScoutArrival {
                    movement_id,
                    army_id,
                    action_id,
                    return_action_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    army,
                    target,
                    attack_type,
                    arrives_at,
                    returns_at,
                };
                service
                    .complete_scout_arrival(source_village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::MerchantsArrival {
                action_id,
                village_id: _,
                source_village_id,
                target_village_id,
                player_id,
                resources,
                merchants_used,
                arrives_at,
            } => {
                let command = CompleteMerchantsArrival {
                    action_id,
                    player_id,
                    source_village_id,
                    target_village_id,
                    resources,
                    merchants_used,
                    arrives_at,
                };
                service
                    .complete_merchant_arrival(source_village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::MerchantsReturn {
                action_id,
                village_id: _,
                source_village_id,
                player_id,
                merchants_used,
                returns_at,
            } => {
                let command = CompleteMerchantsReturn {
                    action_id,
                    player_id,
                    source_village_id,
                    merchants_used,
                    returns_at,
                };
                service
                    .complete_merchant_return(source_village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::AddBuilding {
                village_id,
                player_id,
                slot_id,
                building_name,
                level,
                speed,
            } => {
                let command = CompleteAddBuilding {
                    action_id: action.id,
                    player_id,
                    village_id,
                    slot_id,
                    building_name,
                    level,
                    speed,
                };
                service.complete_add_building(village_id, &command).await?;
            }
            ScheduledActionPayload::UpgradeBuilding {
                village_id,
                player_id,
                slot_id,
                building_name,
                level,
                speed,
            } => {
                let command = CompleteUpgradeBuilding {
                    action_id: action.id,
                    player_id,
                    village_id,
                    slot_id,
                    building_name,
                    level,
                    speed,
                };
                service
                    .complete_upgrade_building(village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::DowngradeBuilding {
                village_id,
                player_id,
                slot_id,
                building_name,
                level,
                speed,
            } => {
                let command = CompleteDowngradeBuilding {
                    action_id: action.id,
                    player_id,
                    village_id,
                    slot_id,
                    building_name,
                    level,
                    speed,
                };
                service
                    .complete_downgrade_building(village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::TrainUnit {
                action_id,
                village_id,
                player_id,
                slot_id,
                unit,
                time_per_unit,
                quantity_remaining,
                execute_at,
            } => {
                let command = CompleteTrainUnit {
                    action_id,
                    player_id,
                    village_id,
                    slot_id,
                    unit,
                    time_per_unit,
                    quantity_remaining,
                    execute_at,
                };
                service.complete_train_unit(village_id, &command).await?;
            }
            ScheduledActionPayload::ResearchAcademy {
                action_id,
                village_id,
                player_id,
                unit,
            } => {
                let command = CompleteAcademyResearch {
                    action_id,
                    player_id,
                    village_id,
                    unit,
                };
                service
                    .complete_academy_research(village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::ResearchSmithy {
                action_id,
                village_id,
                player_id,
                unit,
            } => {
                let command = CompleteSmithyResearch {
                    action_id,
                    player_id,
                    village_id,
                    unit,
                };
                service
                    .complete_smithy_research(village_id, &command)
                    .await?;
            }
            ScheduledActionPayload::HeroRevival {
                action_id,
                village_id,
                player_id,
                hero,
                reset,
                revive_at,
            } => {
                let command = CompleteHeroRevival {
                    action_id,
                    player_id,
                    village_id,
                    hero,
                    reset,
                    revived_at: revive_at,
                };
                service.complete_hero_revival(village_id, &command).await?;
            }
        }
        Ok(())
    }
}
