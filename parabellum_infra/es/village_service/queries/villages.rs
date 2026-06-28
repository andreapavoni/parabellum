//! Village state read helpers for `VillageEsService`.
//!
//! These methods expose village read-model state through the service facade.

use mini_cqrs_es::CqrsError;
use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::projection_repositories::VillageRepository;

use crate::es::PostgresVillageRepository;

use super::super::VillageEsService;

impl VillageEsService {
    /// Returns one projected village by id.
    pub async fn get_village(&self, village_id: u32) -> Result<VillageModel, CqrsError> {
        let repo = PostgresVillageRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        repo.get_by_village_id(village_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    /// Returns all projected village states owned by the player.
    pub async fn list_player_village_states(
        &self,
        player_id: uuid::Uuid,
    ) -> Result<Vec<VillageModel>, CqrsError> {
        let repo = PostgresVillageRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        repo.list_by_player_id(player_id)
            .await
            .map_err(CqrsError::domain_source)
    }

    /// Counts child villages founded or conquered from the parent village.
    pub async fn count_child_villages(
        &self,
        player_id: uuid::Uuid,
        parent_village_id: u32,
    ) -> Result<u8, CqrsError> {
        let repo = PostgresVillageRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        repo.count_child_villages(player_id, parent_village_id)
            .await
            .map_err(CqrsError::domain_source)
    }
}
