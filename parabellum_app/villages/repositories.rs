use parabellum_types::buildings::BuildingName;
use parabellum_types::errors::ApplicationError;
use parabellum_types::{common::ResourceGroup, map::Position, tribe::Tribe};
use uuid::Uuid;

use crate::ports::queries::MerchantMovement;
use crate::villages::models::{
    MarketplaceOfferModel, MarketplaceOfferStatus, ReportModel, ScheduledAction,
    ScheduledActionStatus, ScheduledActionType, VillageModel, VillageMovement,
};
use crate::villages::queries::ScheduledActionStatusCounts;

#[async_trait::async_trait]
pub trait VillageModelRepository: Send + Sync {
    async fn upsert_from_village(
        &self,
        village_id: u32,
        player_id: Uuid,
        village_name: &str,
        position: &Position,
        tribe: Tribe,
        buildings: &[parabellum_game::models::village::VillageBuilding],
        army: &Option<parabellum_game::models::army::Army>,
    ) -> Result<(), ApplicationError>;
    async fn update_player_id(
        &self,
        village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;
    async fn update_army(
        &self,
        village_id: u32,
        army: &Option<parabellum_game::models::army::Army>,
    ) -> Result<(), ApplicationError>;
    async fn update_reinforcements(
        &self,
        village_id: u32,
        reinforcements: &[parabellum_game::models::army::Army],
    ) -> Result<(), ApplicationError>;
    async fn update_deployed_armies(
        &self,
        village_id: u32,
        deployed_armies: &[parabellum_game::models::army::Army],
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
    async fn list_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;
    async fn list_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;
    async fn list_active_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;
    async fn list_active_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError>;
    async fn count_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, ApplicationError>;
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
    ) -> Result<Vec<MarketplaceOfferModel>, ApplicationError>;
    async fn list_open(&self) -> Result<Vec<MarketplaceOfferModel>, ApplicationError>;
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
        limit: i64,
    ) -> Result<Vec<ReportModel>, ApplicationError>;

    async fn get_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportModel>, ApplicationError>;

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError>;
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

    async fn get_home_army(
        &self,
        village_id: u32,
    ) -> Result<Option<parabellum_game::models::army::Army>, ApplicationError>;

    async fn list_stationed_armies(
        &self,
        village_id: u32,
    ) -> Result<Vec<parabellum_game::models::army::Army>, ApplicationError>;

    async fn list_deployed_armies(
        &self,
        home_village_id: u32,
    ) -> Result<Vec<parabellum_game::models::army::Army>, ApplicationError>;

    async fn get_moving_army(
        &self,
        army_id: Uuid,
    ) -> Result<parabellum_game::models::army::Army, ApplicationError>;

    async fn find_stationed_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, parabellum_game::models::army::Army)>, ApplicationError>;
}
