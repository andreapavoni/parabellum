use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

use async_trait::async_trait;

use parabellum_app::{
    config::Config,
    ports::{
        queries::{
            LeaderboardPage, MarketplaceData, VillageArmyStateView, VillageQueryPort,
            VillageQueues, VillageTroopMovements,
        },
        scheduler::SchedulerPort,
        villages::{
            AcceptMarketplaceOfferRequest, AddBuildingRequest, CancelMarketplaceOfferRequest,
            CreateHeroRequest, CreateMarketplaceOfferRequest, RecallReinforcementsRequest,
            ReleaseReinforcementsRequest, ResearchAcademyRequest, ResearchSmithyRequest,
            ReviveHeroRequest, SendAttackRequest, SendReinforcementRequest, SendResourcesRequest,
            SendScoutRequest, SendSettlersRequest, TrainUnitsRequest, UpgradeBuildingRequest,
            VillageCommandPort,
        },
    },
    villages::{
        AddBuilding, AttackVillage, CreateHero, CreateMarketplaceOffer, RecallReinforcements,
        ReleaseReinforcements, ResearchAcademy, ResearchSmithy, ReviveHero, ScoutVillage,
        SendMerchantsTransfer, SendReinforcement, SendSettlers, TrainUnits, UpgradeBuilding,
    },
};
use parabellum_types::{
    errors::{ApplicationError, GameError},
    map::Position,
};

use crate::es::VillageEsService;

#[derive(Clone)]
/// Transport adapter implementing app ports by delegating to ES service/query flows.
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

    fn position_to_field_id(&self, position: &Position) -> u32 {
        position.to_id(self.config.world_size as i32)
    }

    fn map_cqrs_error(err: mini_cqrs_es::CqrsError) -> ApplicationError {
        let msg = err.to_string();
        if msg.contains("player already has an alive hero") {
            return ApplicationError::Game(GameError::HeroAlreadyExists);
        }
        if msg.contains("hero revival already pending") {
            return ApplicationError::Game(GameError::HeroRevivalAlreadyPending);
        }
        if msg.contains("cannot revive while an alive hero exists") {
            return ApplicationError::Game(GameError::HeroAlreadyExists);
        }
        ApplicationError::Unknown(msg)
    }
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
            .get_village(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village(request.target_village_id)
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
            .get_village(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village(request.target_village_id)
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
            .get_village(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village(request.target_village_id)
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
            .get_village(request.source_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let target = self
            .service
            .get_village(request.target_village_id)
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
            .get_village(request.source_village_id)
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
        let target_is_empty_valley = self
            .service
            .is_unoccupied_valley(target_field_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        if !target_is_empty_valley {
            return Err(parabellum_types::errors::GameError::InvalidValley(target_field_id).into());
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
            .get_village(context.stationed_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let home = self
            .service
            .get_village(request.village_id)
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
                    hero_id: request.hero_id,
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
            .get_village(context.stationed_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let home = self
            .service
            .get_village(request.village_id)
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
                    hero_id: request.hero_id,
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
            .get_village(offer.owner_village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
        let accepting = self
            .service
            .get_village(request.village_id)
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

    async fn create_hero(&self, request: CreateHeroRequest) -> Result<(), ApplicationError> {
        self.service
            .create_hero(
                request.village_id,
                &CreateHero {
                    hero_id: request.hero_id,
                    player_id: request.player_id,
                    village_id: request.village_id,
                    has_existing_hero: self
                        .service
                        .player_has_alive_hero(request.player_id)
                        .await
                        .map_err(Self::map_cqrs_error)?,
                },
            )
            .await
            .map_err(Self::map_cqrs_error)?;
        Ok(())
    }

    async fn revive_hero(&self, request: ReviveHeroRequest) -> Result<(), ApplicationError> {
        let hero = self
            .service
            .get_hero(request.hero_id)
            .await
            .map_err(Self::map_cqrs_error)?;
        if self
            .service
            .player_has_pending_hero_revival(request.player_id)
            .await
            .map_err(Self::map_cqrs_error)?
        {
            return Err(ApplicationError::Game(GameError::HeroRevivalAlreadyPending));
        }
        if self
            .service
            .player_has_alive_hero(request.player_id)
            .await
            .map_err(Self::map_cqrs_error)?
        {
            return Err(ApplicationError::Game(GameError::HeroAlreadyExists));
        }
        let revive_at = chrono::Utc::now()
            + chrono::Duration::seconds(hero.resurrection_cost(self.config.speed).time as i64);
        self.service
            .revive_hero(
                request.village_id,
                &ReviveHero {
                    action_id: Uuid::new_v4(),
                    player_id: request.player_id,
                    village_id: request.village_id,
                    hero,
                    reset: request.reset,
                    speed: self.config.speed,
                    revive_at,
                },
            )
            .await
            .map_err(Self::map_cqrs_error)?;
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
        self.service
            .get_village_queues(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, ApplicationError> {
        self.service
            .get_village_troop_movements(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, ApplicationError> {
        self.service
            .get_marketplace_data(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError> {
        self.service
            .get_village_army_state_view(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_village_info_by_ids(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, parabellum_app::read_models::VillageInfo>, ApplicationError> {
        self.service
            .get_village_info_by_ids(village_ids)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_expansion_culture_info(
        &self,
        player_id: Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<parabellum_app::ports::queries::ExpansionCultureInfo, ApplicationError> {
        self.service
            .get_expansion_culture_info(player_id, village_id, server_speed)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<LeaderboardPage, ApplicationError> {
        self.service
            .get_leaderboard_page(page, per_page)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn list_villages_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<parabellum_app::villages::models::VillageModel>, ApplicationError> {
        self.service
            .list_villages_by_player_id(player_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_village_model(
        &self,
        village_id: u32,
    ) -> Result<parabellum_app::villages::models::VillageModel, ApplicationError> {
        self.service
            .get_village(village_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<parabellum_app::read_models::MapRegionTile>, ApplicationError> {
        self.service
            .get_map_region(center_x, center_y, radius, world_size)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_map_field(
        &self,
        field_id: u32,
    ) -> Result<parabellum_game::models::map::MapField, ApplicationError> {
        self.service
            .get_map_field(field_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
    }

    async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<parabellum_app::read_models::MapRegionTile>, ApplicationError> {
        self.service
            .get_map_region_tile_by_field_id(field_id)
            .await
            .map_err(|e| ApplicationError::Unknown(e.to_string()))
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
