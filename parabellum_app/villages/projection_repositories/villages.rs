//! Village projection repository contracts.

use parabellum_types::{
    buildings::BuildingName, common::ResourceGroup, errors::ApplicationError, map::Position,
    tribe::Tribe,
};
use uuid::Uuid;

use crate::villages::{
    models::VillageModel,
    projection_repositories::{ExpansionCultureSnapshot, ExpansionOwnershipSnapshot},
};

/// Persistence boundary for projected village state and map occupancy.
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
