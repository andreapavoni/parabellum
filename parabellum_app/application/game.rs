//! Application service that orchestrates game use cases.
//!
//! `GameApplication` is the only entrypoint used by the HTTP layer.
//! It composes identity, command, query, and scheduler ports and delegates
//! each use case to the proper adapter implementation.

use std::sync::Arc;

use parabellum_types::common::{Player, User};
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::ports::{
    identity::{IdentityPort, RegisterPlayerRequest},
    queries::VillageQueryPort,
    scheduler::SchedulerPort,
    villages::{
        AcceptMarketplaceOfferRequest, AddBuildingRequest, CancelMarketplaceOfferRequest,
        CreateHeroRequest, CreateMarketplaceOfferRequest, RecallReinforcementsRequest,
        ReleaseReinforcementsRequest, ResearchAcademyRequest, ResearchSmithyRequest,
        ReviveHeroRequest, SendAttackRequest, SendReinforcementRequest, SendResourcesRequest,
        SendScoutRequest, SendSettlersRequest, TrainUnitsRequest, UpgradeBuildingRequest,
        VillageCommandsPort,
    },
};

#[derive(Clone)]
/// High-level application facade for game, identity, and scheduler use cases.
pub struct GameApplication {
    identity: Arc<dyn IdentityPort>,
    villages: Arc<dyn VillageCommandsPort>,
    queries: Arc<dyn VillageQueryPort>,
    scheduler: Arc<dyn SchedulerPort>,
}

impl GameApplication {
    /// Creates a new application facade from its required ports.
    pub fn new(
        identity: Arc<dyn IdentityPort>,
        villages: Arc<dyn VillageCommandsPort>,
        queries: Arc<dyn VillageQueryPort>,
        scheduler: Arc<dyn SchedulerPort>,
    ) -> Self {
        Self {
            identity,
            villages,
            queries,
            scheduler,
        }
    }

    pub(crate) fn identity_port(&self) -> &Arc<dyn IdentityPort> {
        &self.identity
    }

    pub(crate) fn villages_port(&self) -> &Arc<dyn VillageCommandsPort> {
        &self.villages
    }

    pub(crate) fn queries_port(&self) -> &Arc<dyn VillageQueryPort> {
        &self.queries
    }

    pub(crate) fn scheduler_port(&self) -> &Arc<dyn SchedulerPort> {
        &self.scheduler
    }

    pub async fn register_player(
        &self,
        request: RegisterPlayerRequest,
    ) -> Result<(), ApplicationError> {
        self.identity_port().register_player(request).await
    }

    pub async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, ApplicationError> {
        self.identity_port()
            .authenticate_user(email, password)
            .await
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, ApplicationError> {
        self.identity_port().get_user_by_email(email).await
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError> {
        self.identity_port().get_user_by_id(user_id).await
    }

    pub async fn get_player_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        self.identity_port().get_player_by_user_id(user_id).await
    }

    pub async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        self.identity_port().get_player_by_id(player_id).await
    }

    pub async fn send_resources(
        &self,
        request: SendResourcesRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().send_resources(request).await
    }

    pub async fn train_units(&self, request: TrainUnitsRequest) -> Result<(), ApplicationError> {
        self.villages_port().train_units(request).await
    }

    pub async fn research_academy(
        &self,
        request: ResearchAcademyRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().research_academy(request).await
    }

    pub async fn research_smithy(
        &self,
        request: ResearchSmithyRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().research_smithy(request).await
    }

    pub async fn send_reinforcement(
        &self,
        request: SendReinforcementRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().send_reinforcement(request).await
    }

    pub async fn send_attack(&self, request: SendAttackRequest) -> Result<(), ApplicationError> {
        self.villages_port().send_attack(request).await
    }

    pub async fn send_scout(&self, request: SendScoutRequest) -> Result<(), ApplicationError> {
        self.villages_port().send_scout(request).await
    }

    pub async fn send_settlers(
        &self,
        request: SendSettlersRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().send_settlers(request).await
    }

    pub async fn recall_reinforcements(
        &self,
        request: RecallReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().recall_reinforcements(request).await
    }

    pub async fn release_reinforcements(
        &self,
        request: ReleaseReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().release_reinforcements(request).await
    }

    pub async fn add_building(&self, request: AddBuildingRequest) -> Result<(), ApplicationError> {
        self.villages_port().add_building(request).await
    }

    pub async fn upgrade_building(
        &self,
        request: UpgradeBuildingRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().upgrade_building(request).await
    }

    pub async fn create_marketplace_offer(
        &self,
        request: CreateMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().create_marketplace_offer(request).await
    }

    pub async fn accept_marketplace_offer(
        &self,
        request: AcceptMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().accept_marketplace_offer(request).await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        request: CancelMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.villages_port().cancel_marketplace_offer(request).await
    }

    pub async fn create_hero(&self, request: CreateHeroRequest) -> Result<(), ApplicationError> {
        self.villages_port().create_hero(request).await
    }

    pub async fn revive_hero(&self, request: ReviveHeroRequest) -> Result<(), ApplicationError> {
        self.villages_port().revive_hero(request).await
    }

    pub async fn get_marketplace_offer(
        &self,
        offer_id: Uuid,
    ) -> Result<crate::villages::models::MarketplaceOfferModel, ApplicationError> {
        self.queries_port().get_marketplace_offer(offer_id).await
    }

    pub async fn list_reports_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<crate::villages::models::ReportModel>, ApplicationError> {
        self.queries_port()
            .list_reports_for_player(player_id, limit)
            .await
    }

    pub async fn get_report_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<crate::villages::models::ReportModel>, ApplicationError> {
        self.queries_port()
            .get_report_for_player(report_id, player_id)
            .await
    }

    pub async fn mark_report_as_read(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.queries_port()
            .mark_report_as_read(report_id, player_id)
            .await
    }

    pub async fn get_village_queues(
        &self,
        village_id: u32,
    ) -> Result<crate::ports::queries::VillageQueues, ApplicationError> {
        self.queries_port().get_village_queues(village_id).await
    }

    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<crate::ports::queries::VillageTroopMovements, ApplicationError> {
        self.queries_port()
            .get_village_troop_movements(village_id)
            .await
    }

    pub async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<crate::ports::queries::MarketplaceData, ApplicationError> {
        self.queries_port().get_marketplace_data(village_id).await
    }

    pub async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<crate::ports::queries::VillageArmyStateView, ApplicationError> {
        self.queries_port()
            .get_village_army_state_view(village_id)
            .await
    }

    pub async fn get_village_info_by_ids(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<std::collections::HashMap<u32, crate::read_models::VillageInfo>, ApplicationError>
    {
        self.queries_port()
            .get_village_info_by_ids(village_ids)
            .await
    }

    pub async fn get_expansion_culture_info(
        &self,
        player_id: Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<crate::ports::queries::ExpansionCultureInfo, ApplicationError> {
        self.queries_port()
            .get_expansion_culture_info(player_id, village_id, server_speed)
            .await
    }

    pub async fn get_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<crate::ports::queries::LeaderboardPage, ApplicationError> {
        self.queries_port()
            .get_leaderboard_page(page, per_page)
            .await
    }

    pub async fn list_villages_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<crate::villages::models::VillageModel>, ApplicationError> {
        self.queries_port()
            .list_villages_by_player_id(player_id)
            .await
    }

    pub async fn get_village_model(
        &self,
        village_id: u32,
    ) -> Result<crate::villages::models::VillageModel, ApplicationError> {
        self.queries_port().get_village_model(village_id).await
    }

    pub async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<crate::read_models::MapRegionTile>, ApplicationError> {
        self.queries_port()
            .get_map_region(center_x, center_y, radius, world_size)
            .await
    }

    pub async fn get_map_field(
        &self,
        field_id: u32,
    ) -> Result<parabellum_game::models::map::MapField, ApplicationError> {
        self.queries_port().get_map_field(field_id).await
    }

    pub async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<crate::read_models::MapRegionTile>, ApplicationError> {
        self.queries_port()
            .get_map_region_tile_by_field_id(field_id)
            .await
    }

    pub async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, ApplicationError> {
        self.scheduler_port()
            .process_due_actions(before_or_equal, limit)
            .await
    }
}
