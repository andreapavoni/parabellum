use std::sync::Arc;

use async_trait::async_trait;
use parabellum_app::config::Config;
use parabellum_app::cqrs::queries::{
    AcademyQueueItem, BuildingQueueItem, MarketplaceData, MerchantMovement, SmithyQueueItem,
    TrainingQueueItem, TroopMovement, TroopMovementDirection, TroopMovementType, VillageQueues,
    VillageTroopMovements,
};
use parabellum_app::jobs::JobStatus;
use parabellum_app::ports::queries::LeaderboardPage;
use parabellum_app::ports::queries::VillageQueryPort;
use parabellum_app::ports::scheduler::SchedulerPort;
use parabellum_app::ports::villages::{
    AcceptMarketplaceOfferRequest, AddBuildingRequest, CancelMarketplaceOfferRequest,
    CreateMarketplaceOfferRequest, RecallReinforcementsRequest, ReleaseReinforcementsRequest,
    ResearchAcademyRequest, ResearchSmithyRequest, SendAttackRequest, SendReinforcementRequest,
    SendResourcesRequest, SendScoutRequest, SendSettlersRequest, TrainUnitsRequest,
    UpgradeBuildingRequest, VillageCommandPort,
};
use parabellum_app::villages::{
    AddBuilding, AttackVillage, CreateMarketplaceOffer, RecallReinforcements,
    ReleaseReinforcements, ResearchAcademy, ResearchSmithy, ScoutVillage, SendMerchantsTransfer,
    SendReinforcement, SendSettlers, TrainUnits, UpgradeBuilding, models::ScheduledActionPayload,
    models::ScheduledActionStatus,
};
use parabellum_game::models::culture_points::required_cp;
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::errors::ApplicationError;
use parabellum_types::map::Position;
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use uuid::Uuid;

use crate::es::VillageEsService;
use crate::models as db_models;

#[derive(Clone)]
pub struct VillageEsAdapter {
    service: VillageEsService,
    config: Arc<Config>,
}

impl VillageEsAdapter {
    pub fn new(service: VillageEsService, config: Arc<Config>) -> Self {
        Self { service, config }
    }

    fn compute_travel_duration(
        &self,
        source: &parabellum_app::villages::models::VillageModel,
        target: &parabellum_app::villages::models::VillageModel,
        speed: u8,
    ) -> chrono::Duration {
        let secs = source.position.calculate_travel_time_secs(
            target.position.clone(),
            speed,
            self.config.world_size as i32,
            self.config.speed as u8,
        );
        chrono::Duration::seconds(std::cmp::max(1, secs) as i64)
    }

    fn movement_speed(
        tribe: &parabellum_types::tribe::Tribe,
        units: &parabellum_types::army::TroopSet,
    ) -> u8 {
        let mut min_speed: Option<u8> = None;
        for (idx, qty) in units.units().iter().enumerate() {
            if *qty == 0 {
                continue;
            }
            if let Some(unit) = tribe.units().get(idx) {
                min_speed = Some(min_speed.map_or(unit.speed, |current| current.min(unit.speed)));
            }
        }
        min_speed.unwrap_or(1)
    }

    fn map_job_status(status: ScheduledActionStatus) -> JobStatus {
        match status {
            ScheduledActionStatus::Pending => JobStatus::Pending,
            ScheduledActionStatus::Processing => JobStatus::Processing,
            ScheduledActionStatus::Completed => JobStatus::Completed,
            ScheduledActionStatus::Failed => JobStatus::Failed,
        }
    }

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

    fn build_region_ids(center_x: i32, center_y: i32, radius: i32, world_size: i32) -> Vec<i32> {
        let diameter = (radius * 2 + 1).max(0) as usize;
        let mut ids = Vec::with_capacity(diameter * diameter);
        for y in ((center_y - radius)..=(center_y + radius)).rev() {
            let wrapped_y = Self::wrap_coordinate(y, world_size);
            for x in center_x - radius..=center_x + radius {
                let wrapped_x = Self::wrap_coordinate(x, world_size);
                let position = Position {
                    x: wrapped_x,
                    y: wrapped_y,
                };
                ids.push(position.to_id(world_size) as i32);
            }
        }
        ids
    }

    fn wrap_coordinate(value: i32, world_size: i32) -> i32 {
        if world_size <= 0 {
            return value;
        }
        let span = world_size * 2 + 1;
        let mut normalized = (value + world_size) % span;
        if normalized < 0 {
            normalized += span;
        }
        normalized - world_size
    }

    fn position_to_field_id(&self, position: &Position) -> u32 {
        position.to_id(self.config.world_size as i32)
    }
}

#[derive(Debug, FromRow)]
struct DbMapFieldWithOwner {
    id: i32,
    village_id: Option<i32>,
    player_id: Option<Uuid>,
    position: serde_json::Value,
    topology: serde_json::Value,
    rm_village_id: Option<i32>,
    rm_player_id: Option<Uuid>,
    village_name: Option<String>,
    village_population: Option<i32>,
    player_name: Option<String>,
    tribe: Option<db_models::Tribe>,
}

#[async_trait]
impl VillageCommandPort for VillageEsAdapter {
    async fn add_building(&self, request: AddBuildingRequest) -> Result<(), ApplicationError> {
        self.service
            .add_building(
                request.village_id,
                &AddBuilding {
                    player_id: request.player_id,
                    slot_id: request.slot_id,
                    building_name: request.building_name,
                    speed: self.config.speed,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn upgrade_building(
        &self,
        request: UpgradeBuildingRequest,
    ) -> Result<(), ApplicationError> {
        self.service
            .upgrade_building(
                request.village_id,
                &UpgradeBuilding {
                    player_id: request.player_id,
                    slot_id: request.slot_id,
                    speed: self.config.speed,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn send_resources(&self, request: SendResourcesRequest) -> Result<(), ApplicationError> {
        let source = self
            .service
            .get_village_model(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village_model(request.target_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let travel_secs = source.position.calculate_travel_time_secs(
            target.position,
            source.tribe.merchant_stats().speed,
            self.config.world_size as i32,
            self.config.speed as u8,
        );
        let arrives_at =
            chrono::Utc::now() + chrono::Duration::seconds(std::cmp::max(1, travel_secs) as i64);

        self.service
            .send_resources(
                request.source_village_id,
                &SendMerchantsTransfer {
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    resources: request.resources,
                    arrives_at,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn train_units(&self, request: TrainUnitsRequest) -> Result<(), ApplicationError> {
        self.service
            .train_units(
                request.village_id,
                &TrainUnits {
                    player_id: request.player_id,
                    unit_idx: request.unit_idx,
                    building_name: request.building_name,
                    quantity: request.quantity,
                    speed: self.config.speed,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn research_academy(
        &self,
        request: ResearchAcademyRequest,
    ) -> Result<(), ApplicationError> {
        self.service
            .research_academy(
                request.village_id,
                &ResearchAcademy {
                    player_id: request.player_id,
                    unit: request.unit,
                    speed: self.config.speed,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn research_smithy(
        &self,
        request: ResearchSmithyRequest,
    ) -> Result<(), ApplicationError> {
        self.service
            .research_smithy(
                request.village_id,
                &ResearchSmithy {
                    player_id: request.player_id,
                    unit: request.unit,
                    speed: self.config.speed,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn send_reinforcement(
        &self,
        request: SendReinforcementRequest,
    ) -> Result<(), ApplicationError> {
        let source = self
            .service
            .get_village_model(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village_model(request.target_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let speed = Self::movement_speed(&source.tribe, &request.units);
        let arrives_at = chrono::Utc::now() + self.compute_travel_duration(&source, &target, speed);

        self.service
            .send_reinforcement(
                request.source_village_id,
                &SendReinforcement {
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    units: request.units,
                    hero_id: request.hero_id,
                    arrives_at,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn send_attack(&self, request: SendAttackRequest) -> Result<(), ApplicationError> {
        let source = self
            .service
            .get_village_model(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village_model(request.target_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let speed = Self::movement_speed(&source.tribe, &request.units);
        let one_way = self.compute_travel_duration(&source, &target, speed);
        let arrives_at = chrono::Utc::now() + one_way;
        let returns_at = arrives_at + one_way;

        self.service
            .send_attack(
                request.source_village_id,
                &AttackVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    units: request.units,
                    hero_id: request.hero_id,
                    attack_type: request.attack_type,
                    catapult_targets: request.catapult_targets,
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn send_scout(&self, request: SendScoutRequest) -> Result<(), ApplicationError> {
        let source = self
            .service
            .get_village_model(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village_model(request.target_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let speed = Self::movement_speed(&source.tribe, &request.units);
        let one_way = self.compute_travel_duration(&source, &target, speed);
        let arrives_at = chrono::Utc::now() + one_way;
        let returns_at = arrives_at + one_way;

        self.service
            .send_scout(
                request.source_village_id,
                &ScoutVillage {
                    movement_id: Uuid::new_v4(),
                    arrival_action_id: Uuid::new_v4(),
                    return_action_id: Uuid::new_v4(),
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    units: request.units,
                    target: request.target,
                    attack_type: request.attack_type,
                    arrives_at,
                    returns_at,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn send_settlers(&self, request: SendSettlersRequest) -> Result<(), ApplicationError> {
        let source = self
            .service
            .get_village_model(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let settlers_speed = source.tribe.units().get(9).map(|u| u.speed).unwrap_or(1);
        let travel_secs = source.position.calculate_travel_time_secs(
            request.target_position.clone(),
            settlers_speed,
            self.config.world_size as i32,
            self.config.speed as u8,
        );
        let arrives_at =
            chrono::Utc::now() + chrono::Duration::seconds(std::cmp::max(1, travel_secs) as i64);
        let target_field_id = self.position_to_field_id(&request.target_position);
        let target_is_empty_valley: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM rm_map_fields
                WHERE id = $1
                  AND village_id IS NULL
                  AND topology @> '{"Valley":[4,4,4,6]}'
            )
            "#,
        )
        .bind(target_field_id as i32)
        .fetch_one(self.service.pool())
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        if !target_is_empty_valley {
            return Err(ApplicationError::Unknown(
                "Target field is not an empty valley".to_string(),
            ));
        }

        self.service
            .send_settlers(
                request.source_village_id,
                &SendSettlers {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    army_id: Uuid::new_v4(),
                    player_id: request.player_id,
                    target_village_id: target_field_id,
                    target_position: request.target_position,
                    village_name: request.village_name,
                    tribe: request.tribe,
                    arrives_at,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn recall_reinforcements(
        &self,
        request: RecallReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        let context = self
            .service
            .find_reinforcement_context(request.army_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        if context.home_village_id != request.village_id {
            return Err(ApplicationError::Unknown(
                "Reinforcement army does not belong to provided home village".to_string(),
            ));
        }

        let stationed = self
            .service
            .get_village_model(context.stationed_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let home = self
            .service
            .get_village_model(request.village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let reinforcement_army = context.army;

        let returns_at = chrono::Utc::now()
            + self.compute_travel_duration(
                &stationed,
                &home,
                Self::movement_speed(&stationed.tribe, reinforcement_army.units()),
            );

        self.service
            .recall_reinforcements(
                request.village_id,
                &RecallReinforcements {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    player_id: request.player_id,
                    home_village_id: request.village_id,
                    stationed_village_id: context.stationed_village_id,
                    reinforcement_army,
                    units: request.units,
                    returns_at,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn release_reinforcements(
        &self,
        request: ReleaseReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        let context = self
            .service
            .find_reinforcement_context(request.army_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        if context.home_village_id != request.village_id {
            return Err(ApplicationError::Unknown(
                "Reinforcement army does not belong to provided home village".to_string(),
            ));
        }

        let stationed = self
            .service
            .get_village_model(context.stationed_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let home = self
            .service
            .get_village_model(request.village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let reinforcement_army = context.army;

        let returns_at = chrono::Utc::now()
            + self.compute_travel_duration(
                &stationed,
                &home,
                Self::movement_speed(&stationed.tribe, reinforcement_army.units()),
            );

        self.service
            .release_reinforcements(
                context.stationed_village_id,
                &ReleaseReinforcements {
                    action_id: Uuid::new_v4(),
                    movement_id: Uuid::new_v4(),
                    player_id: request.player_id,
                    stationed_village_id: context.stationed_village_id,
                    home_village_id: request.village_id,
                    reinforcement_army,
                    units: request.units,
                    returns_at,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn create_offer(
        &self,
        request: CreateMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.service
            .create_marketplace_offer(
                request.village_id,
                &CreateMarketplaceOffer {
                    player_id: request.player_id,
                    offer_resources: request.offer_resources,
                    seek_resources: request.seek_resources,
                },
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn accept_offer(
        &self,
        request: AcceptMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        let offer = self
            .service
            .get_marketplace_offer(request.offer_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let owner = self
            .service
            .get_village_model(offer.owner_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let accepting = self
            .service
            .get_village_model(request.village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let owner_secs = owner.position.calculate_travel_time_secs(
            accepting.position.clone(),
            owner.tribe.merchant_stats().speed,
            self.config.world_size as i32,
            self.config.speed as u8,
        );
        let accepting_secs = accepting.position.calculate_travel_time_secs(
            owner.position,
            accepting.tribe.merchant_stats().speed,
            self.config.world_size as i32,
            self.config.speed as u8,
        );
        let now = chrono::Utc::now();
        let owner_arrives_at = now + chrono::Duration::seconds(std::cmp::max(1, owner_secs) as i64);
        let accepting_arrives_at =
            now + chrono::Duration::seconds(std::cmp::max(1, accepting_secs) as i64);

        self.service
            .accept_marketplace_offer(
                request.village_id,
                request.player_id,
                request.offer_id,
                owner_arrives_at,
                accepting_arrives_at,
            )
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn cancel_offer(
        &self,
        request: CancelMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.service
            .cancel_marketplace_offer(request.village_id, request.player_id, request.offer_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl VillageQueryPort for VillageEsAdapter {
    async fn get_marketplace_offer(
        &self,
        offer_id: uuid::Uuid,
    ) -> Result<parabellum_app::villages::models::MarketplaceOfferModel, ApplicationError> {
        self.service
            .get_marketplace_offer(offer_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn list_reports_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<parabellum_app::villages::models::ReportModel>, ApplicationError> {
        self.service
            .list_reports_for_player(player_id, limit)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_report_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<parabellum_app::villages::models::ReportModel>, ApplicationError> {
        self.service
            .get_report_for_player(report_id, player_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn mark_report_as_read(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.service
            .mark_report_as_read(report_id, player_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, ApplicationError> {
        let mut building = Vec::new();
        let building_actions = self
            .service
            .get_village_building_queue(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        for action in building_actions {
            let Ok(payload) =
                serde_json::from_value::<ScheduledActionPayload>(action.payload.clone())
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
                status: Self::map_job_status(action.status),
                finishes_at: action.execute_at,
            });
        }
        building.sort_by_key(|it| it.finishes_at);

        let training_actions = self
            .service
            .get_village_training_queue(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let mut training = Vec::new();
        for action in training_actions {
            let Ok(ScheduledActionPayload::TrainUnit {
                slot_id,
                unit,
                quantity_remaining,
                time_per_unit,
                ..
            }) = serde_json::from_value::<ScheduledActionPayload>(action.payload.clone())
            else {
                continue;
            };
            training.push(TrainingQueueItem {
                job_id: action.id,
                slot_id,
                unit,
                quantity: quantity_remaining,
                time_per_unit,
                status: Self::map_job_status(action.status),
                finishes_at: action.execute_at,
            });
        }
        training.sort_by_key(|it| it.finishes_at);

        let academy_actions = self
            .service
            .get_village_academy_queue(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let mut academy = Vec::new();
        for action in academy_actions {
            let Ok(ScheduledActionPayload::ResearchAcademy { unit, .. }) =
                serde_json::from_value::<ScheduledActionPayload>(action.payload.clone())
            else {
                continue;
            };
            academy.push(AcademyQueueItem {
                job_id: action.id,
                unit,
                status: Self::map_job_status(action.status),
                finishes_at: action.execute_at,
            });
        }
        academy.sort_by_key(|it| it.finishes_at);

        let smithy_actions = self
            .service
            .get_village_smithy_queue(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let mut smithy = Vec::new();
        for action in smithy_actions {
            let Ok(ScheduledActionPayload::ResearchSmithy { unit, .. }) =
                serde_json::from_value::<ScheduledActionPayload>(action.payload.clone())
            else {
                continue;
            };
            smithy.push(SmithyQueueItem {
                job_id: action.id,
                unit,
                status: Self::map_job_status(action.status),
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

    async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, ApplicationError> {
        let models = self
            .service
            .get_village_troop_movements(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();

        for movement in models
            .outgoing
            .into_iter()
            .chain(models.incoming.into_iter())
        {
            let origin_model = self
                .service
                .get_village_model(movement.origin_village_id)
                .await
                .ok();
            let target_model = self
                .service
                .get_village_model(movement.target_village_id)
                .await
                .ok();

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
                    .unwrap_or(Position { x: 0, y: 0 }),
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
                    .unwrap_or(Position { x: 0, y: 0 }),
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
            }
        }

        outgoing.sort_by_key(|m| m.arrives_at);
        incoming.sort_by_key(|m| m.arrives_at);
        Ok(VillageTroopMovements { outgoing, incoming })
    }

    async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, ApplicationError> {
        let all_open_models = self
            .service
            .get_open_marketplace_offers()
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

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
            outgoing_merchants: Vec::<MerchantMovement>::new(),
            incoming_merchants: Vec::<MerchantMovement>::new(),
            village_info: HashMap::new(),
        })
    }

    async fn get_village_info_by_ids(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, parabellum_app::repository::VillageInfo>, ApplicationError> {
        if village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = sqlx::query(
            "SELECT village_id, village_name, position FROM rm_village WHERE village_id = ANY($1)",
        )
        .bind(village_ids.iter().map(|id| *id as i32).collect::<Vec<_>>())
        .fetch_all(self.service.pool())
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let mut out = HashMap::new();
        for row in rows {
            let village_id = row.get::<i32, _>("village_id") as u32;
            let village_name = row.get::<String, _>("village_name");
            let position: Position =
                serde_json::from_value(row.get::<serde_json::Value, _>("position"))
                    .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
            out.insert(
                village_id,
                parabellum_app::repository::VillageInfo {
                    id: village_id,
                    name: village_name,
                    position,
                },
            );
        }
        Ok(out)
    }

    async fn get_expansion_culture_info(
        &self,
        player_id: Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<parabellum_app::ports::queries::ExpansionCultureInfo, ApplicationError> {
        let player_culture_points: i64 =
            sqlx::query_scalar("SELECT culture_points::bigint FROM players WHERE id = $1")
                .bind(player_id)
                .fetch_one(self.service.pool())
                .await
                .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let player_culture_points_production: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(culture_points_production), 0)::bigint FROM rm_village WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(self.service.pool())
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let village_row = sqlx::query(
            "SELECT culture_points, culture_points_production FROM rm_village WHERE village_id = $1 AND player_id = $2",
        )
        .bind(village_id as i32)
        .bind(player_id)
        .fetch_one(self.service.pool())
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let village_culture_points = village_row.get::<i32, _>("culture_points").max(0) as u32;
        let village_culture_points_production = village_row
            .get::<i32, _>("culture_points_production")
            .max(0) as u32;

        let village_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*)::bigint FROM rm_village WHERE player_id = $1")
                .bind(player_id)
                .fetch_one(self.service.pool())
                .await
                .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let speed = match server_speed {
            1 => parabellum_types::common::Speed::X1,
            2 => parabellum_types::common::Speed::X2,
            3 => parabellum_types::common::Speed::X3,
            5 => parabellum_types::common::Speed::X5,
            10 => parabellum_types::common::Speed::X10,
            _ => parabellum_types::common::Speed::X1,
        };
        let next_cp_required = required_cp(speed, village_count as usize + 1);

        Ok(parabellum_app::ports::queries::ExpansionCultureInfo {
            village_culture_points,
            village_culture_points_production,
            player_culture_points: player_culture_points as u32,
            player_culture_points_production: player_culture_points_production as u32,
            next_cp_required,
        })
    }

    async fn get_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<LeaderboardPage, ApplicationError> {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let offset = (page - 1) * per_page;

        let total_players: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM players")
            .fetch_one(self.service.pool())
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let rows = sqlx::query(
            r#"
            SELECT
                p.id AS player_id,
                p.username AS username,
                p.tribe::text AS tribe,
                COUNT(v.village_id) AS village_count,
                COALESCE(SUM(v.population), 0) AS population
            FROM players p
            LEFT JOIN rm_village v ON v.player_id = p.id
            GROUP BY p.id, p.username, p.tribe
            ORDER BY COALESCE(SUM(v.population), 0) DESC, p.username ASC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(self.service.pool())
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let tribe = match row.get::<String, _>("tribe").as_str() {
                "Roman" => parabellum_types::tribe::Tribe::Roman,
                "Gaul" => parabellum_types::tribe::Tribe::Gaul,
                "Teuton" => parabellum_types::tribe::Tribe::Teuton,
                "Natar" => parabellum_types::tribe::Tribe::Natar,
                _ => parabellum_types::tribe::Tribe::Nature,
            };
            entries.push(parabellum_app::repository::PlayerLeaderboardEntry {
                player_id: row.get("player_id"),
                username: row.get("username"),
                village_count: row.get::<i64, _>("village_count"),
                population: row.get::<i64, _>("population"),
                tribe,
            });
        }

        Ok(LeaderboardPage {
            entries,
            total_players,
        })
    }

    async fn list_village_models_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<parabellum_app::villages::models::VillageModel>, ApplicationError> {
        self.service
            .list_village_models_by_player_id(player_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<parabellum_app::repository::MapRegionTile>, ApplicationError> {
        let tile_ids = Self::build_region_ids(center_x, center_y, radius, world_size);
        if tile_ids.is_empty() {
            return Ok(Vec::new());
        }

        let records = sqlx::query_as::<_, DbMapFieldWithOwner>(
            r#"
            SELECT
                mf.id,
                mf.village_id,
                mf.player_id,
                mf.position,
                mf.topology,
                rv.village_id AS rm_village_id,
                rv.player_id AS rm_player_id,
                rv.village_name AS village_name,
                rv.population AS village_population,
                p.username AS player_name,
                p.tribe as tribe
            FROM rm_map_fields AS mf
            LEFT JOIN rm_village AS rv
                ON rv.village_id = mf.id
            LEFT JOIN players AS p
                ON p.id = COALESCE(mf.player_id, rv.player_id)
            WHERE mf.id = ANY($1)
            ORDER BY array_position($1, mf.id)
            "#,
        )
        .bind(&tile_ids)
        .fetch_all(self.service.pool())
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        let fields = records
            .into_iter()
            .map(|record| {
                let db_field = db_models::MapField {
                    id: record.id,
                    village_id: record.village_id.or(record.rm_village_id),
                    player_id: record.player_id.or(record.rm_player_id),
                    position: record.position,
                    topology: record.topology,
                };
                parabellum_app::repository::MapRegionTile {
                    field: db_field.into(),
                    village_name: record.village_name,
                    village_population: record.village_population,
                    player_name: record.player_name,
                    tribe: record.tribe.map(|t| t.into()),
                }
            })
            .collect();

        Ok(fields)
    }

    async fn get_map_field(
        &self,
        field_id: u32,
    ) -> Result<parabellum_game::models::map::MapField, ApplicationError> {
        let field = sqlx::query_as::<_, db_models::MapField>(
            "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE id = $1",
        )
        .bind(field_id as i32)
        .fetch_one(self.service.pool())
        .await
        .map_err(|_| {
            ApplicationError::Db(parabellum_types::errors::DbError::MapFieldNotFound(
                field_id,
            ))
        })?;
        Ok(field.into())
    }

    async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<parabellum_app::repository::MapRegionTile>, ApplicationError> {
        let record = sqlx::query_as::<_, DbMapFieldWithOwner>(
            r#"
            SELECT
                mf.id,
                mf.village_id,
                mf.player_id,
                mf.position,
                mf.topology,
                rv.village_id AS rm_village_id,
                rv.player_id AS rm_player_id,
                rv.village_name AS village_name,
                rv.population AS village_population,
                p.username AS player_name,
                p.tribe as tribe
            FROM rm_map_fields AS mf
            LEFT JOIN rm_village AS rv
                ON rv.village_id = mf.id
            LEFT JOIN players AS p
                ON p.id = COALESCE(mf.player_id, rv.player_id)
            WHERE mf.id = $1
            "#,
        )
        .bind(field_id as i32)
        .fetch_optional(self.service.pool())
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

        Ok(record.map(|record| {
            let db_field = db_models::MapField {
                id: record.id,
                village_id: record.village_id.or(record.rm_village_id),
                player_id: record.player_id.or(record.rm_player_id),
                position: record.position,
                topology: record.topology,
            };
            parabellum_app::repository::MapRegionTile {
                field: db_field.into(),
                village_name: record.village_name,
                village_population: record.village_population,
                player_name: record.player_name,
                tribe: record.tribe.map(|t| t.into()),
            }
        }))
    }
}

#[async_trait]
impl SchedulerPort for VillageEsAdapter {
    async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, ApplicationError> {
        self.service
            .process_due_actions(before_or_equal, limit)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }
}
