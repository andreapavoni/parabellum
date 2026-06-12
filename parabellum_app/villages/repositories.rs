use parabellum_types::buildings::BuildingName;
use parabellum_types::errors::ApplicationError;
use parabellum_types::{common::ResourceGroup, map::Position, tribe::Tribe};
use uuid::Uuid;

use crate::ports::queries::MerchantMovement;
use crate::villages::VillageArmyContext;
use crate::villages::models::{
    MarketplaceOfferModel, MarketplaceOfferStatus, ReportModel, ScheduledAction,
    ScheduledActionStatus, ScheduledActionType, VillageModel, VillageMovement,
};
use crate::villages::queries::ScheduledActionStatusCounts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionCultureSnapshot {
    pub village_culture_points_production: u32,
    pub player_culture_points_production: u32,
    pub player_village_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionOwnershipSnapshot {
    pub source_child_villages: u8,
    pub player_village_count: usize,
}

#[async_trait::async_trait]
pub trait VillageRepository: Send + Sync {
    async fn upsert_from_village(
        &self,
        village_id: u32,
        player_id: Uuid,
        village_name: &str,
        position: &Position,
        tribe: Tribe,
        parent_village_id: Option<u32>,
        buildings: &[parabellum_game::models::village::VillageBuilding],
    ) -> Result<(), ApplicationError>;
    async fn update_player_id(
        &self,
        village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;
    async fn update_building(
        &self,
        village_id: u32,
        slot_id: u8,
        building_name: BuildingName,
        level: u8,
        speed: i8,
    ) -> Result<(), ApplicationError>;
    async fn set_stored_resources(
        &self,
        village_id: u32,
        resources: ResourceGroup,
    ) -> Result<(), ApplicationError>;
    async fn set_busy_merchants(
        &self,
        village_id: u32,
        busy_merchants: u8,
    ) -> Result<(), ApplicationError>;
    async fn get_by_village_id(&self, village_id: u32) -> Result<VillageModel, ApplicationError>;
    async fn list_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError>;
    async fn list_by_village_ids(
        &self,
        village_ids: &[u32],
    ) -> Result<Vec<VillageModel>, ApplicationError>;
    async fn get_expansion_culture_snapshot(
        &self,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<ExpansionCultureSnapshot, ApplicationError>;
    async fn count_child_villages(
        &self,
        player_id: Uuid,
        parent_village_id: u32,
    ) -> Result<u8, ApplicationError>;
    async fn get_expansion_ownership_snapshot(
        &self,
        player_id: Uuid,
        source_village_id: u32,
    ) -> Result<ExpansionOwnershipSnapshot, ApplicationError>;
    async fn set_map_occupancy(
        &self,
        field_id: u32,
        village_id: Option<u32>,
        player_id: Option<Uuid>,
    ) -> Result<(), ApplicationError>;
}

#[async_trait::async_trait]
pub trait VillageMovementRepository: Send + Sync {
    async fn upsert(&self, movement: &VillageMovement) -> Result<(), ApplicationError>;
    async fn list_by_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<VillageMovement>, ApplicationError>;
    async fn delete_by_movement_id(&self, movement_id: Uuid) -> Result<(), ApplicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledActionVillageFilter {
    Source(u32),
    Target(u32),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduledActionListFilter {
    pub action_type: Option<ScheduledActionType>,
    pub village: Option<ScheduledActionVillageFilter>,
    pub statuses: Option<Vec<ScheduledActionStatus>>,
}

impl ScheduledActionListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn action_type(mut self, action_type: ScheduledActionType) -> Self {
        self.action_type = Some(action_type);
        self
    }

    pub fn source_village(mut self, village_id: u32) -> Self {
        self.village = Some(ScheduledActionVillageFilter::Source(village_id));
        self
    }

    pub fn target_village(mut self, target_village_id: u32) -> Self {
        self.village = Some(ScheduledActionVillageFilter::Target(target_village_id));
        self
    }

    pub fn statuses(mut self, statuses: Vec<ScheduledActionStatus>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    pub fn active(self) -> Self {
        self.statuses(vec![
            ScheduledActionStatus::Pending,
            ScheduledActionStatus::Processing,
        ])
    }
}

#[async_trait::async_trait]
pub trait ScheduledActionRepository: Send + Sync {
    async fn add(&self, action: &ScheduledAction) -> Result<(), ApplicationError>;
    async fn get_by_id(&self, id: Uuid) -> Result<ScheduledAction, ApplicationError>;
    async fn take_due_pending(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;
    async fn update_status(
        &self,
        id: Uuid,
        status: ScheduledActionStatus,
    ) -> Result<(), ApplicationError>;
    async fn list_actions(
        &self,
        filter: ScheduledActionListFilter,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;
    async fn list_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .source_village(village_id)
                .action_type(action_type),
        )
        .await
    }
    async fn list_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .target_village(target_village_id)
                .action_type(action_type),
        )
        .await
    }
    async fn list_active_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .source_village(village_id)
                .action_type(action_type)
                .active(),
        )
        .await
    }
    async fn list_active_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        self.list_actions(
            ScheduledActionListFilter::new()
                .target_village(target_village_id)
                .action_type(action_type)
                .active(),
        )
        .await
    }
    async fn count_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, ApplicationError>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarketplaceOfferListFilter {
    pub owner_village_id: Option<u32>,
    pub exclude_owner_village_id: Option<u32>,
    pub status: Option<MarketplaceOfferStatus>,
}

impl MarketplaceOfferListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn owner_village(mut self, village_id: u32) -> Self {
        self.owner_village_id = Some(village_id);
        self
    }

    pub fn excluding_owner_village(mut self, village_id: u32) -> Self {
        self.exclude_owner_village_id = Some(village_id);
        self
    }

    pub fn status(mut self, status: MarketplaceOfferStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn open(self) -> Self {
        self.status(MarketplaceOfferStatus::Open)
    }
}

#[async_trait::async_trait]
pub trait MarketplaceRepository: Send + Sync {
    async fn upsert(&self, offer: &MarketplaceOfferModel) -> Result<(), ApplicationError>;
    async fn get_by_offer_id(
        &self,
        offer_id: Uuid,
    ) -> Result<MarketplaceOfferModel, ApplicationError>;
    async fn set_status(
        &self,
        offer_id: Uuid,
        status: MarketplaceOfferStatus,
        accepted_by_player_id: Option<Uuid>,
        accepted_by_village_id: Option<u32>,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError>;
    async fn list_by_owner_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        self.list_offers(MarketplaceOfferListFilter::new().owner_village(village_id))
            .await
    }
    async fn list_offers(
        &self,
        filter: MarketplaceOfferListFilter,
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError>;
    async fn list_open(&self) -> Result<Vec<MarketplaceOfferModel>, ApplicationError> {
        self.list_offers(MarketplaceOfferListFilter::new().open())
            .await
    }
    async fn claim_open_for_accept(
        &self,
        offer_id: Uuid,
        accepted_by_player_id: Uuid,
        accepted_by_village_id: u32,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<MarketplaceOfferModel>, ApplicationError>;
    async fn list_active_outgoing(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError>;
    async fn list_active_incoming(
        &self,
        village_id: u32,
    ) -> Result<Vec<MerchantMovement>, ApplicationError>;
}

#[derive(Debug, Clone)]
pub struct ProjectedReport {
    pub id: Uuid,
    pub report_type: String,
    pub payload: serde_json::Value,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
}

#[async_trait::async_trait]
pub trait ReportRepository: Send + Sync {
    async fn add_projected(
        &self,
        report: &ProjectedReport,
        audience_player_ids: &[Uuid],
    ) -> Result<(), ApplicationError>;
    async fn list_for_player(
        &self,
        player_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ReportModel>, ApplicationError>;

    async fn get_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportModel>, ApplicationError>;

    async fn count_unread_for_player(&self, player_id: Uuid) -> Result<i64, ApplicationError>;

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmyState {
    Home,
    Stationed,
    Moving,
    Trapped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArmyListFilter {
    pub army_id: Option<Uuid>,
    pub home_village_id: Option<u32>,
    pub current_village_id: Option<u32>,
    pub state: Option<ArmyState>,
    pub deployed: Option<bool>,
    pub limit: Option<i64>,
}

impl ArmyListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn army_id(mut self, army_id: Uuid) -> Self {
        self.army_id = Some(army_id);
        self
    }

    pub fn home_village(mut self, village_id: u32) -> Self {
        self.home_village_id = Some(village_id);
        self
    }

    pub fn current_village(mut self, village_id: u32) -> Self {
        self.current_village_id = Some(village_id);
        self
    }

    pub fn state(mut self, state: ArmyState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn deployed(mut self, deployed: bool) -> Self {
        self.deployed = Some(deployed);
        self
    }

    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[async_trait::async_trait]
pub trait ArmyRepository: Send + Sync {
    async fn upsert_home(
        &self,
        army: &parabellum_game::models::army::Army,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;
    async fn upsert_moving(
        &self,
        army: &parabellum_game::models::army::Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;
    async fn upsert_stationed(
        &self,
        army: &parabellum_game::models::army::Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;
    async fn delete(&self, army_id: Uuid) -> Result<(), ApplicationError>;

    async fn list_armies(
        &self,
        filter: ArmyListFilter,
    ) -> Result<Vec<parabellum_game::models::army::Army>, ApplicationError>;

    async fn get_moving_army(
        &self,
        army_id: Uuid,
    ) -> Result<parabellum_game::models::army::Army, ApplicationError> {
        let mut armies = self
            .list_armies(
                ArmyListFilter::new()
                    .army_id(army_id)
                    .state(ArmyState::Moving)
                    .limit(1),
            )
            .await?;
        armies.pop().ok_or(ApplicationError::Db(
            parabellum_types::errors::DbError::ArmyNotFound(army_id),
        ))
    }

    async fn find_stationed_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, parabellum_game::models::army::Army)>, ApplicationError>;

    async fn find_trapped_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, parabellum_game::models::army::Army)>, ApplicationError>;

    async fn army_context_for_village(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let mut home_armies = self
            .list_armies(
                ArmyListFilter::new()
                    .home_village(village_id)
                    .current_village(village_id)
                    .state(ArmyState::Home)
                    .limit(1),
            )
            .await?;
        Ok(VillageArmyContext {
            home: home_armies.pop(),
            stationed: self
                .list_armies(
                    ArmyListFilter::new()
                        .current_village(village_id)
                        .state(ArmyState::Stationed),
                )
                .await?,
            deployed: self
                .list_armies(
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Stationed)
                        .deployed(true),
                )
                .await?,
            moving: self
                .list_armies(
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Moving),
                )
                .await?,
            trapped_here: self
                .list_armies(
                    ArmyListFilter::new()
                        .current_village(village_id)
                        .state(ArmyState::Trapped),
                )
                .await?,
            trapped_away: self
                .list_armies(
                    ArmyListFilter::new()
                        .home_village(village_id)
                        .state(ArmyState::Trapped)
                        .deployed(true),
                )
                .await?,
        })
    }

    async fn delete_by_home_village(&self, village_id: u32) -> Result<(), ApplicationError>;
}

#[async_trait::async_trait]
pub trait HeroRepository: Send + Sync {
    async fn upsert(
        &self,
        hero: &parabellum_game::models::hero::Hero,
        home_village_id: u32,
        current_village_id: u32,
        state: &str,
    ) -> Result<(), ApplicationError>;
    async fn get_by_id(
        &self,
        hero_id: Uuid,
    ) -> Result<parabellum_game::models::hero::Hero, ApplicationError>;
    async fn get_by_player(
        &self,
        player_id: Uuid,
    ) -> Result<Option<parabellum_game::models::hero::Hero>, ApplicationError>;
    async fn has_alive_for_player(&self, player_id: Uuid) -> Result<bool, ApplicationError>;
}
