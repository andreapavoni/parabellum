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
        CreateMarketplaceOfferRequest, RecallReinforcementsRequest, ReleaseReinforcementsRequest,
        ResearchAcademyRequest, ResearchSmithyRequest, SendAttackRequest, SendReinforcementRequest,
        SendResourcesRequest, SendScoutRequest, SendSettlersRequest, TrainUnitsRequest,
        UpgradeBuildingRequest, VillageCommandPort,
    },
};

#[derive(Clone)]
pub struct GameApplication {
    identity: Arc<dyn IdentityPort>,
    villages: Arc<dyn VillageCommandPort>,
    queries: Arc<dyn VillageQueryPort>,
    scheduler: Arc<dyn SchedulerPort>,
}

impl GameApplication {
    pub fn new(
        identity: Arc<dyn IdentityPort>,
        villages: Arc<dyn VillageCommandPort>,
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

    pub(crate) fn villages_port(&self) -> &Arc<dyn VillageCommandPort> {
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
        super::identity::register_player(self, request).await
    }

    pub async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, ApplicationError> {
        super::identity::authenticate_user(self, email, password).await
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, ApplicationError> {
        super::identity::get_user_by_email(self, email).await
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError> {
        super::identity::get_user_by_id(self, user_id).await
    }

    pub async fn get_player_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        super::identity::get_player_by_user_id(self, user_id).await
    }

    pub async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        super::identity::get_player_by_id(self, player_id).await
    }

    pub async fn send_resources(
        &self,
        request: SendResourcesRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::send_resources(self, request).await
    }

    pub async fn train_units(&self, request: TrainUnitsRequest) -> Result<(), ApplicationError> {
        super::villages::train_units(self, request).await
    }

    pub async fn research_academy(
        &self,
        request: ResearchAcademyRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::research_academy(self, request).await
    }

    pub async fn research_smithy(
        &self,
        request: ResearchSmithyRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::research_smithy(self, request).await
    }

    pub async fn send_reinforcement(
        &self,
        request: SendReinforcementRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::send_reinforcement(self, request).await
    }

    pub async fn send_attack(&self, request: SendAttackRequest) -> Result<(), ApplicationError> {
        super::villages::send_attack(self, request).await
    }

    pub async fn send_scout(&self, request: SendScoutRequest) -> Result<(), ApplicationError> {
        super::villages::send_scout(self, request).await
    }

    pub async fn send_settlers(
        &self,
        request: SendSettlersRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::send_settlers(self, request).await
    }

    pub async fn recall_reinforcements(
        &self,
        request: RecallReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::recall_reinforcements(self, request).await
    }

    pub async fn release_reinforcements(
        &self,
        request: ReleaseReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::release_reinforcements(self, request).await
    }

    pub async fn add_building(&self, request: AddBuildingRequest) -> Result<(), ApplicationError> {
        super::villages::add_building(self, request).await
    }

    pub async fn upgrade_building(
        &self,
        request: UpgradeBuildingRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::upgrade_building(self, request).await
    }

    pub async fn create_marketplace_offer(
        &self,
        request: CreateMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::create_marketplace_offer(self, request).await
    }

    pub async fn accept_marketplace_offer(
        &self,
        request: AcceptMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::accept_marketplace_offer(self, request).await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        request: CancelMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        super::villages::cancel_marketplace_offer(self, request).await
    }

    pub async fn get_marketplace_offer(
        &self,
        offer_id: Uuid,
    ) -> Result<crate::villages::models::MarketplaceOfferModel, ApplicationError> {
        super::queries::get_marketplace_offer(self, offer_id).await
    }

    pub async fn list_reports_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<crate::villages::models::ReportModel>, ApplicationError> {
        super::queries::list_reports_for_player(self, player_id, limit).await
    }

    pub async fn get_report_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<crate::villages::models::ReportModel>, ApplicationError> {
        super::queries::get_report_for_player(self, report_id, player_id).await
    }

    pub async fn mark_report_as_read(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        super::queries::mark_report_as_read(self, report_id, player_id).await
    }

    pub async fn get_village_queues(
        &self,
        village_id: u32,
    ) -> Result<crate::cqrs::queries::VillageQueues, ApplicationError> {
        super::queries::get_village_queues(self, village_id).await
    }

    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<crate::cqrs::queries::VillageTroopMovements, ApplicationError> {
        super::queries::get_village_troop_movements(self, village_id).await
    }

    pub async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<crate::cqrs::queries::MarketplaceData, ApplicationError> {
        super::queries::get_marketplace_data(self, village_id).await
    }

    pub async fn get_village_info_by_ids(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<std::collections::HashMap<u32, crate::repository::VillageInfo>, ApplicationError>
    {
        super::queries::get_village_info_by_ids(self, village_ids).await
    }

    pub async fn get_expansion_culture_info(
        &self,
        player_id: Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<crate::ports::queries::ExpansionCultureInfo, ApplicationError> {
        super::queries::get_expansion_culture_info(self, player_id, village_id, server_speed).await
    }

    pub async fn get_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<crate::ports::queries::LeaderboardPage, ApplicationError> {
        super::queries::get_leaderboard_page(self, page, per_page).await
    }

    pub async fn list_village_models_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<crate::villages::models::VillageModel>, ApplicationError> {
        super::queries::list_village_models_by_player_id(self, player_id).await
    }

    pub async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<crate::repository::MapRegionTile>, ApplicationError> {
        super::queries::get_map_region(self, center_x, center_y, radius, world_size).await
    }

    pub async fn get_map_field(
        &self,
        field_id: u32,
    ) -> Result<parabellum_game::models::map::MapField, ApplicationError> {
        super::queries::get_map_field(self, field_id).await
    }

    pub async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<crate::repository::MapRegionTile>, ApplicationError> {
        super::queries::get_map_region_tile_by_field_id(self, field_id).await
    }

    pub async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, ApplicationError> {
        super::scheduler::process_due_actions(self, before_or_equal, limit).await
    }
}
