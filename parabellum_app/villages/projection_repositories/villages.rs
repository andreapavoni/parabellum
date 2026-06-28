//! Village projection read contracts.

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::villages::{
    models::VillageModel,
    projection_repositories::{ExpansionCultureSnapshot, ExpansionOwnershipSnapshot},
};

/// Read boundary for projected village state.
#[async_trait::async_trait]
pub trait VillageRepository: Send + Sync {
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
}
