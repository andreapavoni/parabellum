use parabellum_types::buildings::BuildingName;
use parabellum_types::errors::ApplicationError;
use parabellum_types::{common::ResourceGroup, map::Position, tribe::Tribe};
use uuid::Uuid;

use crate::villages::models::{
    ScheduledAction, ScheduledActionStatus, ScheduledActionType, VillageModel, VillageMovement,
};

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
        army: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError>;
    async fn update_player_id(
        &self,
        village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;
    async fn update_army(
        &self,
        village_id: u32,
        army: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError>;
    async fn update_reinforcements(
        &self,
        village_id: u32,
        reinforcements: &parabellum_types::army::TroopSet,
    ) -> Result<(), ApplicationError>;
    async fn update_deployed_armies(
        &self,
        village_id: u32,
        deployed_armies: &parabellum_types::army::TroopSet,
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
    async fn get_by_village_id(&self, village_id: u32) -> Result<VillageModel, ApplicationError>;
    async fn list_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError>;
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
}
