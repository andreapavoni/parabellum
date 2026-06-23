//! Application service that orchestrates game use cases.
//!
//! `GameApplication` is the only entrypoint used by the HTTP layer.
//! It composes identity, use-case, query, and scheduler ports and delegates
//! gameplay orchestration to application use cases.

use std::{collections::HashMap, sync::Arc};

use parabellum_game::models::hero::Hero;
use parabellum_types::common::{Player, User};
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::identity::{IdentityPort, RegisterPlayerRequest, RegistrationUseCases};
use crate::leaderboards::{GetPlayerPopulationLeaderboardPageRequest, LeaderboardUseCases};
use crate::map::{
    GetMapFieldRequest, GetMapRegionRequest, GetMapRegionTileByFieldIdRequest, MapUseCases,
};
use crate::scheduler::{ProcessDueActionsRequest, SchedulerUseCases};
use crate::villages::{
    BuildingUseCases, DevelopmentUseCases, HeroUseCases, MarketplaceUseCases,
    MovementControlUseCases, MovementUseCases, ReinforcementUseCases, ReportUseCases, TrapUseCases,
    VillageActivityUseCases, VillageArmyUseCases, VillageExpansionUseCases, VillageProfileUseCases,
    VillageReferenceUseCases, VillageStateUseCases,
    requests::activity::{
        GetVillageQueuesRequest, GetVillageTroopMovementsRequest,
        ListCancelableOutgoingMovementIdsRequest,
    },
    requests::buildings::{
        AddBuildingRequest, CancelBuildingConstructionRequest, DowngradeBuildingRequest,
        UpgradeBuildingRequest,
    },
    requests::development::{ResearchAcademyRequest, ResearchSmithyRequest, TrainUnitsRequest},
    requests::expansion::GetExpansionCultureInfoRequest,
    requests::heroes::{
        AssignHeroPointsRequest, CreateHeroRequest, GetHeroByPlayerRequest,
        GetPendingHeroRevivalRequest, ResetHeroPointsRequest, ReviveHeroRequest,
        SetHeroResourceFocusRequest,
    },
    requests::marketplace::{
        AcceptMarketplaceOfferRequest, CancelMarketplaceOfferRequest,
        CreateMarketplaceOfferRequest, GetMarketplaceDataRequest, GetMarketplaceOfferRequest,
        SendResourcesRequest,
    },
    requests::movement_control::CancelTroopMovementRequest,
    requests::movements::{
        SendAttackRequest, SendReinforcementRequest, SendScoutRequest, SendSettlersRequest,
    },
    requests::reinforcements::{
        DisbandTrappedTroopsRequest, RecallReinforcementsRequest, ReleaseReinforcementsRequest,
        ReleaseTrappedTroopsRequest,
    },
    requests::reports::{
        CountUnreadReportsForPlayerRequest, GetReportForPlayerRequest, ListReportsForPlayerRequest,
        MarkReportReadRequest,
    },
    requests::traps::BuildTrapsRequest,
    requests::village_army::GetVillageArmyStateViewRequest,
    requests::village_profile::RenameVillageRequest,
    requests::village_references::GetVillageReferencesRequest,
    requests::village_state::{GetVillageStateRequest, ListPlayerVillageStatesRequest},
};

#[derive(Clone)]
/// High-level application facade for game, identity, and scheduler use cases.
pub struct GameApplication {
    identity: Arc<dyn IdentityPort>,
    registration: RegistrationUseCases,
    leaderboards: LeaderboardUseCases,
    map: MapUseCases,
    village_profile: VillageProfileUseCases,
    buildings: BuildingUseCases,
    development: DevelopmentUseCases,
    heroes: HeroUseCases,
    movements: MovementUseCases,
    movement_control: MovementControlUseCases,
    marketplace: MarketplaceUseCases,
    reinforcements: ReinforcementUseCases,
    reports: ReportUseCases,
    activity: VillageActivityUseCases,
    army: VillageArmyUseCases,
    expansion: VillageExpansionUseCases,
    village_references: VillageReferenceUseCases,
    village_state: VillageStateUseCases,
    traps: TrapUseCases,
    scheduler: SchedulerUseCases,
}

impl GameApplication {
    /// Creates a new application facade from its required ports.
    pub fn new(
        identity: Arc<dyn IdentityPort>,
        registration: RegistrationUseCases,
        leaderboards: LeaderboardUseCases,
        map: MapUseCases,
        village_profile: VillageProfileUseCases,
        buildings: BuildingUseCases,
        development: DevelopmentUseCases,
        heroes: HeroUseCases,
        movements: MovementUseCases,
        movement_control: MovementControlUseCases,
        marketplace: MarketplaceUseCases,
        reinforcements: ReinforcementUseCases,
        reports: ReportUseCases,
        activity: VillageActivityUseCases,
        army: VillageArmyUseCases,
        expansion: VillageExpansionUseCases,
        village_references: VillageReferenceUseCases,
        village_state: VillageStateUseCases,
        traps: TrapUseCases,
        scheduler: SchedulerUseCases,
    ) -> Self {
        Self {
            identity,
            registration,
            leaderboards,
            map,
            village_profile,
            buildings,
            development,
            heroes,
            movements,
            movement_control,
            marketplace,
            reinforcements,
            reports,
            activity,
            army,
            expansion,
            village_references,
            village_state,
            traps,
            scheduler,
        }
    }

    pub(crate) fn identity_port(&self) -> &Arc<dyn IdentityPort> {
        &self.identity
    }

    pub(crate) fn registration_use_cases(&self) -> &RegistrationUseCases {
        &self.registration
    }

    pub(crate) fn leaderboard_use_cases(&self) -> &LeaderboardUseCases {
        &self.leaderboards
    }

    pub(crate) fn map_use_cases(&self) -> &MapUseCases {
        &self.map
    }

    pub(crate) fn building_use_cases(&self) -> &BuildingUseCases {
        &self.buildings
    }

    pub(crate) fn village_profile_use_cases(&self) -> &VillageProfileUseCases {
        &self.village_profile
    }

    pub(crate) fn development_use_cases(&self) -> &DevelopmentUseCases {
        &self.development
    }

    pub(crate) fn hero_use_cases(&self) -> &HeroUseCases {
        &self.heroes
    }

    pub(crate) fn movement_use_cases(&self) -> &MovementUseCases {
        &self.movements
    }

    pub(crate) fn movement_control_use_cases(&self) -> &MovementControlUseCases {
        &self.movement_control
    }

    pub(crate) fn marketplace_use_cases(&self) -> &MarketplaceUseCases {
        &self.marketplace
    }

    pub(crate) fn reinforcement_use_cases(&self) -> &ReinforcementUseCases {
        &self.reinforcements
    }

    pub(crate) fn report_use_cases(&self) -> &ReportUseCases {
        &self.reports
    }

    pub(crate) fn activity_use_cases(&self) -> &VillageActivityUseCases {
        &self.activity
    }

    pub(crate) fn army_use_cases(&self) -> &VillageArmyUseCases {
        &self.army
    }

    pub(crate) fn expansion_use_cases(&self) -> &VillageExpansionUseCases {
        &self.expansion
    }

    pub(crate) fn village_reference_use_cases(&self) -> &VillageReferenceUseCases {
        &self.village_references
    }

    pub(crate) fn village_state_use_cases(&self) -> &VillageStateUseCases {
        &self.village_state
    }

    pub(crate) fn trap_use_cases(&self) -> &TrapUseCases {
        &self.traps
    }

    pub(crate) fn scheduler_use_cases(&self) -> &SchedulerUseCases {
        &self.scheduler
    }

    pub async fn register_player(
        &self,
        request: RegisterPlayerRequest,
    ) -> Result<(), ApplicationError> {
        self.registration_use_cases().register_player(request).await
    }

    pub async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, ApplicationError> {
        self.identity_port()
            .authenticate_user(username, password)
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
        self.marketplace_use_cases().send_resources(request).await
    }

    pub async fn train_units(&self, request: TrainUnitsRequest) -> Result<(), ApplicationError> {
        self.development_use_cases().train_units(request).await
    }

    pub async fn research_academy(
        &self,
        request: ResearchAcademyRequest,
    ) -> Result<(), ApplicationError> {
        self.development_use_cases().research_academy(request).await
    }

    pub async fn research_smithy(
        &self,
        request: ResearchSmithyRequest,
    ) -> Result<(), ApplicationError> {
        self.development_use_cases().research_smithy(request).await
    }

    pub async fn send_reinforcement(
        &self,
        request: SendReinforcementRequest,
    ) -> Result<(), ApplicationError> {
        self.movement_use_cases().send_reinforcement(request).await
    }

    pub async fn send_attack(&self, request: SendAttackRequest) -> Result<(), ApplicationError> {
        self.movement_use_cases().send_attack(request).await
    }

    pub async fn send_scout(&self, request: SendScoutRequest) -> Result<(), ApplicationError> {
        self.movement_use_cases().send_scout(request).await
    }

    pub async fn send_settlers(
        &self,
        request: SendSettlersRequest,
    ) -> Result<(), ApplicationError> {
        self.movement_use_cases().send_settlers(request).await
    }

    pub async fn recall_reinforcements(
        &self,
        request: RecallReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        self.reinforcement_use_cases()
            .recall_reinforcements(request)
            .await
    }

    pub async fn release_reinforcements(
        &self,
        request: ReleaseReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        self.reinforcement_use_cases()
            .release_reinforcements(request)
            .await
    }

    pub async fn release_trapped_troops(
        &self,
        request: ReleaseTrappedTroopsRequest,
    ) -> Result<(), ApplicationError> {
        self.reinforcement_use_cases()
            .release_trapped_troops(request)
            .await
    }

    pub async fn disband_trapped_troops(
        &self,
        request: DisbandTrappedTroopsRequest,
    ) -> Result<(), ApplicationError> {
        self.reinforcement_use_cases()
            .disband_trapped_troops(request)
            .await
    }

    pub async fn build_traps(&self, request: BuildTrapsRequest) -> Result<(), ApplicationError> {
        self.trap_use_cases().build_traps(request).await
    }

    pub async fn cancel_troop_movement(
        &self,
        request: CancelTroopMovementRequest,
    ) -> Result<(), ApplicationError> {
        self.movement_control_use_cases()
            .cancel_troop_movement(request)
            .await
    }

    pub async fn add_building(&self, request: AddBuildingRequest) -> Result<(), ApplicationError> {
        self.building_use_cases().add_building(request).await
    }

    pub async fn upgrade_building(
        &self,
        request: UpgradeBuildingRequest,
    ) -> Result<(), ApplicationError> {
        self.building_use_cases().upgrade_building(request).await
    }

    pub async fn downgrade_building(
        &self,
        request: DowngradeBuildingRequest,
    ) -> Result<(), ApplicationError> {
        self.building_use_cases().downgrade_building(request).await
    }

    pub async fn cancel_building_construction(
        &self,
        request: CancelBuildingConstructionRequest,
    ) -> Result<(), ApplicationError> {
        self.building_use_cases()
            .cancel_building_construction(request)
            .await
    }

    pub async fn rename_village(
        &self,
        request: RenameVillageRequest,
    ) -> Result<(), ApplicationError> {
        self.village_profile_use_cases()
            .rename_village(request)
            .await
    }

    pub async fn create_marketplace_offer(
        &self,
        request: CreateMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.marketplace_use_cases()
            .create_marketplace_offer(request)
            .await
    }

    pub async fn accept_marketplace_offer(
        &self,
        request: AcceptMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.marketplace_use_cases()
            .accept_marketplace_offer(request)
            .await
    }

    pub async fn cancel_marketplace_offer(
        &self,
        request: CancelMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError> {
        self.marketplace_use_cases()
            .cancel_marketplace_offer(request)
            .await
    }

    pub async fn create_hero(&self, request: CreateHeroRequest) -> Result<(), ApplicationError> {
        self.hero_use_cases().create_hero(request).await
    }

    pub async fn revive_hero(&self, request: ReviveHeroRequest) -> Result<(), ApplicationError> {
        self.hero_use_cases().revive_hero(request).await
    }

    pub async fn assign_hero_points(
        &self,
        request: AssignHeroPointsRequest,
    ) -> Result<(), ApplicationError> {
        self.hero_use_cases().assign_hero_points(request).await
    }

    pub async fn reset_hero_points(
        &self,
        request: ResetHeroPointsRequest,
    ) -> Result<(), ApplicationError> {
        self.hero_use_cases().reset_hero_points(request).await
    }

    pub async fn set_hero_resource_focus(
        &self,
        request: SetHeroResourceFocusRequest,
    ) -> Result<(), ApplicationError> {
        self.hero_use_cases().set_hero_resource_focus(request).await
    }

    pub async fn get_marketplace_offer(
        &self,
        offer_id: Uuid,
    ) -> Result<crate::villages::models::MarketplaceOfferModel, ApplicationError> {
        self.marketplace_use_cases()
            .get_marketplace_offer(GetMarketplaceOfferRequest { offer_id })
            .await
    }

    pub async fn get_hero_by_player(
        &self,
        player_id: Uuid,
    ) -> Result<Option<Hero>, ApplicationError> {
        self.hero_use_cases()
            .get_hero_by_player(GetHeroByPlayerRequest { player_id })
            .await
    }

    pub async fn get_pending_hero_revival_at(
        &self,
        player_id: Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, ApplicationError> {
        self.hero_use_cases()
            .get_pending_hero_revival_at(GetPendingHeroRevivalRequest { player_id })
            .await
    }

    pub async fn list_reports_for_player(
        &self,
        player_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<crate::villages::models::ReportModel>, ApplicationError> {
        self.report_use_cases()
            .list_reports_for_player(ListReportsForPlayerRequest {
                player_id,
                offset,
                limit,
            })
            .await
    }

    pub async fn get_report_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<crate::villages::models::ReportModel>, ApplicationError> {
        self.report_use_cases()
            .get_report_for_player(GetReportForPlayerRequest {
                report_id,
                player_id,
            })
            .await
    }

    pub async fn count_unread_reports_for_player(
        &self,
        player_id: Uuid,
    ) -> Result<i64, ApplicationError> {
        self.report_use_cases()
            .count_unread_reports_for_player(CountUnreadReportsForPlayerRequest { player_id })
            .await
    }

    pub async fn mark_report_as_read(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.report_use_cases()
            .mark_report_as_read(MarkReportReadRequest {
                report_id,
                player_id,
            })
            .await
    }

    pub async fn get_village_queues(
        &self,
        village_id: u32,
    ) -> Result<crate::villages::read_models::VillageQueues, ApplicationError> {
        self.activity_use_cases()
            .get_village_queues(GetVillageQueuesRequest { village_id })
            .await
    }

    pub async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<crate::villages::read_models::VillageTroopMovements, ApplicationError> {
        self.activity_use_cases()
            .get_village_troop_movements(GetVillageTroopMovementsRequest { village_id })
            .await
    }

    pub async fn list_cancelable_outgoing_movement_ids(
        &self,
        village_id: u32,
    ) -> Result<std::collections::HashSet<Uuid>, ApplicationError> {
        self.activity_use_cases()
            .list_cancelable_outgoing_movement_ids(ListCancelableOutgoingMovementIdsRequest {
                village_id,
            })
            .await
    }

    pub async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<crate::villages::read_models::MarketplaceData, ApplicationError> {
        self.marketplace_use_cases()
            .get_marketplace_data(GetMarketplaceDataRequest { village_id })
            .await
    }

    pub async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<crate::villages::read_models::VillageArmyStateView, ApplicationError> {
        self.army_use_cases()
            .get_village_army_state_view(GetVillageArmyStateViewRequest { village_id })
            .await
    }

    pub async fn get_village_references(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, crate::read_models::VillageReference>, ApplicationError> {
        self.village_reference_use_cases()
            .get_village_references(GetVillageReferencesRequest { village_ids })
            .await
    }

    pub async fn get_expansion_culture_info(
        &self,
        player_id: Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<crate::villages::ExpansionCultureInfo, ApplicationError> {
        self.expansion_use_cases()
            .get_expansion_culture_info(GetExpansionCultureInfoRequest {
                player_id,
                village_id,
                server_speed,
            })
            .await
    }

    pub async fn get_player_population_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<crate::leaderboards::PlayerPopulationLeaderboardPage, ApplicationError> {
        self.leaderboard_use_cases()
            .get_player_population_page(GetPlayerPopulationLeaderboardPageRequest {
                page,
                per_page,
            })
            .await
    }

    pub async fn list_player_village_states(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<crate::villages::models::VillageModel>, ApplicationError> {
        self.village_state_use_cases()
            .list_player_village_states(ListPlayerVillageStatesRequest { player_id })
            .await
    }

    pub async fn get_village_state(
        &self,
        village_id: u32,
    ) -> Result<crate::villages::models::VillageModel, ApplicationError> {
        self.village_state_use_cases()
            .get_village_state(GetVillageStateRequest { village_id })
            .await
    }

    pub async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<crate::read_models::MapRegionTile>, ApplicationError> {
        self.map_use_cases()
            .get_map_region(GetMapRegionRequest {
                center_x,
                center_y,
                radius,
                world_size,
            })
            .await
    }

    pub async fn get_map_field(
        &self,
        field_id: u32,
    ) -> Result<parabellum_game::models::map::MapField, ApplicationError> {
        self.map_use_cases()
            .get_map_field(GetMapFieldRequest { field_id })
            .await
    }

    pub async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<crate::read_models::MapRegionTile>, ApplicationError> {
        self.map_use_cases()
            .get_map_region_tile_by_field_id(GetMapRegionTileByFieldIdRequest { field_id })
            .await
    }

    pub async fn process_due_actions(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<usize, ApplicationError> {
        self.scheduler_use_cases()
            .process_due_actions(ProcessDueActionsRequest {
                before_or_equal,
                limit,
            })
            .await
    }
}
