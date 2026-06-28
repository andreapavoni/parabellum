mod expansion;
mod hero_bonus;
mod queries;
mod reads;
mod refresh;
mod rows;
mod state_changes;
mod writes;

use parabellum_app::villages::VillageArmyContext;
use parabellum_app::villages::models::VillageModel;
use parabellum_app::villages::projection_repositories::{
    ArmyRepository, ExpansionCultureSnapshot, ExpansionOwnershipSnapshot, VillageRepository,
};
use parabellum_types::errors::ApplicationError;
use sqlx::PgPool;
use uuid::Uuid;

use super::PostgresArmyRepository;
use crate::ProjectionDb;

#[derive(Debug, Clone)]
pub struct PostgresVillageRepository {
    pool: PgPool,
}

impl PostgresVillageRepository {
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub(super) fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn load_army_context(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let armies = PostgresArmyRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        armies.army_context_for_village(village_id).await
    }

    async fn load_army_context_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let armies = PostgresArmyRepository::new(crate::ProjectionDb::new(self.pool.clone()));
        armies.army_context_for_village_in_tx(tx, village_id).await
    }
}

#[async_trait::async_trait]
impl VillageRepository for PostgresVillageRepository {
    async fn list_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        self.list_refreshed_by_player_id(player_id).await
    }

    async fn list_by_village_ids(
        &self,
        village_ids: &[u32],
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        self.list_refreshed_by_village_ids(village_ids).await
    }

    async fn get_expansion_culture_snapshot(
        &self,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<ExpansionCultureSnapshot, ApplicationError> {
        expansion::get_expansion_culture_snapshot(&self.pool, player_id, village_id).await
    }

    async fn count_child_villages(
        &self,
        player_id: Uuid,
        parent_village_id: u32,
    ) -> Result<u8, ApplicationError> {
        expansion::count_child_villages(&self.pool, player_id, parent_village_id).await
    }

    async fn get_expansion_ownership_snapshot(
        &self,
        player_id: Uuid,
        source_village_id: u32,
    ) -> Result<ExpansionOwnershipSnapshot, ApplicationError> {
        expansion::get_expansion_ownership_snapshot(&self.pool, player_id, source_village_id).await
    }

    async fn get_by_village_id(&self, village_id: u32) -> Result<VillageModel, ApplicationError> {
        self.get_refreshed_by_village_id(village_id).await
    }
}
