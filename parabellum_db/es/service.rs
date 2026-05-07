use std::sync::Arc;

use mini_cqrs_es::anyhow::Result;
use mini_cqrs_es::{CqrsError, QueryRunner};
use parabellum_game::models::army::Army;
use sqlx::PgPool;

use parabellum_app::villages::models::{
    MarketplaceOfferSnapshot, MarketplaceOfferStatus, ReportModel, ScheduledAction,
    ScheduledActionPayload, ScheduledActionStatus, ScheduledActionType, VillageModel,
    VillageTroopMovements,
};
use parabellum_app::villages::queries::{
    GetMarketplaceOfferById, GetOpenMarketplaceOffers, GetReportForPlayer,
    GetScheduledActionStatusCounts, ListReportsForPlayer, ScheduledActionStatusCounts,
};
use parabellum_app::villages::repositories::{
    MarketplaceOfferRepository, ReportReadModelRepository, ScheduledActionRepository,
    VillageModelRepository, VillageMovementRepository,
};
use parabellum_app::villages::{
    AcceptMarketplaceOffer, AddBuilding, AttackVillage, CancelMarketplaceOffer,
    CompleteAcademyResearch, CompleteAddBuilding, CompleteArmyReturn, CompleteAttackArrival,
    CompleteDowngradeBuilding, CompleteMerchantsArrival, CompleteMerchantsReturn,
    CompleteScoutArrival, CompleteSettlersArrival, CompleteSmithyResearch, CompleteTrainUnit,
    CompleteUpgradeBuilding, ConquerVillage, CreateMarketplaceOffer, DowngradeBuilding,
    FoundVillage, RecallReinforcements, ReinforcementArrived, ReleaseReinforcements,
    ResearchAcademy, ResearchSmithy, ScoutVillage, SendMerchantsTransfer, SendReinforcement,
    SendSettlers, SetVillageResources, TrainUnits, UpgradeBuilding, VillageService,
};
use parabellum_types::errors::{DbError, GameError};

use crate::es::{
    PostgresArmyModelRepository, PostgresMarketplaceOfferRepository,
    PostgresReportReadModelRepository, PostgresScheduledActionRepository,
    PostgresVillageModelRepository, PostgresVillageMovementRepository, village_cqrs_runtime,
};

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

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
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
        let offers = PostgresMarketplaceOfferRepository::new(self.pool.clone());
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
        let runtime = village_cqrs_runtime(self.pool.clone());
        let service = VillageService::new(&runtime);
        let actions = PostgresScheduledActionRepository::new(self.pool.clone())
            .take_due_pending(before_or_equal, limit)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        let mut processed = 0usize;
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
            match movement.direction {
                parabellum_app::villages::models::MovementDirection::Outgoing => {
                    outgoing.push(movement)
                }
                parabellum_app::villages::models::MovementDirection::Incoming => {
                    incoming.push(movement)
                }
            }
        }
        outgoing.sort_by_key(|m| m.arrives_at);
        incoming.sort_by_key(|m| m.arrives_at);
        Ok(VillageTroopMovements { outgoing, incoming })
    }

    pub async fn get_village_model(&self, village_id: u32) -> Result<VillageModel, CqrsError> {
        let repo = PostgresVillageModelRepository::new(self.pool.clone());
        repo.get_by_village_id(village_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn list_village_models_by_player_id(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Vec<VillageModel>, CqrsError> {
        let repo = PostgresVillageModelRepository::new(self.pool.clone());
        repo.list_by_player_id(player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn find_reinforcement_context(
        &self,
        army_id: uuid::Uuid,
    ) -> Result<ReinforcementContext, CqrsError> {
        let army_repo = PostgresArmyModelRepository::new(self.pool.clone());
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
        repo.list_by_village_and_type(village_id, ScheduledActionType::TrainUnit)
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
                .list_by_village_and_type(village_id, action_type)
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
        repo.list_by_village_and_type(village_id, ScheduledActionType::ResearchSmithy)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }

    pub async fn get_village_academy_queue(
        &self,
        village_id: u32,
    ) -> Result<Vec<ScheduledAction>, CqrsError> {
        let repo = PostgresScheduledActionRepository::new(self.pool.clone());
        repo.list_by_village_and_type(village_id, ScheduledActionType::ResearchAcademy)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
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
        let runtime = village_cqrs_runtime(self.pool.clone());
        let query = GetScheduledActionStatusCounts {
            repository: Arc::new(PostgresScheduledActionRepository::new(self.pool.clone())),
            village_id,
            action_type,
            status_filter,
        };
        runtime.query(&query).await
    }

    pub async fn get_open_marketplace_offers(
        &self,
    ) -> Result<Vec<parabellum_app::villages::models::MarketplaceOfferModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&GetOpenMarketplaceOffers {
                repository: Arc::new(PostgresMarketplaceOfferRepository::new(self.pool.clone())),
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
                repository: Arc::new(PostgresMarketplaceOfferRepository::new(self.pool.clone())),
                offer_id,
            })
            .await
    }

    pub async fn list_reports_for_player(
        &self,
        player_id: uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<ReportModel>, CqrsError> {
        let runtime = village_cqrs_runtime(self.pool.clone());
        runtime
            .query(&ListReportsForPlayer {
                repository: Arc::new(PostgresReportReadModelRepository::new(self.pool.clone()))
                    as Arc<dyn ReportReadModelRepository>,
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
                repository: Arc::new(PostgresReportReadModelRepository::new(self.pool.clone()))
                    as Arc<dyn ReportReadModelRepository>,
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
        let repo = PostgresReportReadModelRepository::new(self.pool.clone());
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
                } else if let Some(army) = PostgresArmyModelRepository::new(self.pool.clone())
                    .find_moving_by_army_id(army_id)
                    .await
                    .map_err(|e| CqrsError::EventStore(e.to_string()))?
                {
                    let source = self.get_village_model(source_village_id).await?;
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
                    let target = self.get_village_model(target_village_id).await?;
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
        }
        Ok(())
    }
}
