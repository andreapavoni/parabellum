//! Postgres-backed army projection repository.

mod queries;
mod rows;

use parabellum_app::villages::{
    VillageArmyContext,
    projection_repositories::{ArmyListFilter, ArmyRepository, ArmyState},
};
use parabellum_game::models::army::Army;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::ProjectionDb;

use self::rows::{DbArmyRow, army_context_from_rows};

#[derive(Debug, Clone)]
pub struct PostgresArmyRepository {
    pool: PgPool,
}

impl PostgresArmyRepository {
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn upsert_home_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        queries::delete_other_home_armies_query(army.village_id, army.id)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(database_error)?;

        self.upsert_army_in_tx(tx, army, army.village_id, player_id, ArmyState::Home)
            .await
    }

    pub async fn upsert_moving_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_army_in_tx(tx, army, current_village_id, player_id, ArmyState::Moving)
            .await
    }

    pub async fn upsert_stationed_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_army_in_tx(
            tx,
            army,
            stationed_village_id,
            player_id,
            ArmyState::Stationed,
        )
        .await
    }

    pub async fn upsert_trapped_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        trapped_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_army_in_tx(tx, army, trapped_village_id, player_id, ArmyState::Trapped)
            .await
    }

    pub async fn delete_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army_id: Uuid,
    ) -> Result<(), ApplicationError> {
        queries::delete_army_query(army_id)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    pub async fn delete_by_home_village_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<(), ApplicationError> {
        queries::delete_armies_by_home_village_query(village_id)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    pub async fn list_armies_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        filter: ArmyListFilter,
    ) -> Result<Vec<Army>, ApplicationError> {
        let rows = queries::army_query(filter)?
            .build_query_as::<DbArmyRow>()
            .fetch_all(&mut **tx)
            .await
            .map_err(database_error)?;

        rows.into_iter().map(Army::try_from).collect()
    }

    pub async fn army_context_for_village_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let rows = queries::village_army_context_query(village_id)
            .build_query_as::<DbArmyRow>()
            .fetch_all(&mut **tx)
            .await
            .map_err(database_error)?;

        army_context_from_rows(rows, village_id)
    }

    async fn upsert_army(
        &self,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
        state: ArmyState,
    ) -> Result<(), ApplicationError> {
        queries::upsert_army_query(army, current_village_id, player_id, state)
            .build()
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    async fn upsert_army_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
        state: ArmyState,
    ) -> Result<(), ApplicationError> {
        queries::upsert_army_query(army, current_village_id, player_id, state)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    async fn list_armies_by_filter(
        &self,
        filter: ArmyListFilter,
    ) -> Result<Vec<Army>, ApplicationError> {
        let rows = queries::army_query(filter)?
            .build_query_as::<DbArmyRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)?;

        rows.into_iter().map(Army::try_from).collect()
    }

    async fn find_context_by_army_id(
        &self,
        army_id: Uuid,
        state: ArmyState,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        let row =
            queries::army_query(ArmyListFilter::new().army_id(army_id).state(state).limit(1))?
                .build_query_as::<DbArmyRow>()
                .fetch_optional(&self.pool)
                .await
                .map_err(database_error)?;

        row.map(|row| {
            let current_village_id = row.current_village_id();
            Army::try_from(row).map(|army| (current_village_id, army))
        })
        .transpose()
    }
}

fn database_error(error: sqlx::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Database(error))
}

#[async_trait::async_trait]
impl ArmyRepository for PostgresArmyRepository {
    async fn upsert_home(&self, army: &Army, player_id: Uuid) -> Result<(), ApplicationError> {
        queries::delete_other_home_armies_query(army.village_id, army.id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(database_error)?;

        self.upsert_army(army, army.village_id, player_id, ArmyState::Home)
            .await
    }

    async fn upsert_moving(
        &self,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_army(army, current_village_id, player_id, ArmyState::Moving)
            .await
    }

    async fn upsert_stationed(
        &self,
        army: &Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert_army(army, stationed_village_id, player_id, ArmyState::Stationed)
            .await
    }

    async fn delete(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        queries::delete_army_query(army_id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    async fn list_armies(&self, filter: ArmyListFilter) -> Result<Vec<Army>, ApplicationError> {
        self.list_armies_by_filter(filter).await
    }

    async fn get_moving_army(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        let mut armies = self
            .list_armies_by_filter(
                ArmyListFilter::new()
                    .army_id(army_id)
                    .state(ArmyState::Moving)
                    .limit(1),
            )
            .await?;
        armies
            .pop()
            .ok_or(ApplicationError::Db(DbError::ArmyNotFound(army_id)))
    }

    async fn find_stationed_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        self.find_context_by_army_id(army_id, ArmyState::Stationed)
            .await
    }

    async fn find_trapped_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        self.find_context_by_army_id(army_id, ArmyState::Trapped)
            .await
    }

    async fn army_context_for_village(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError> {
        let rows = queries::village_army_context_query(village_id)
            .build_query_as::<DbArmyRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)?;

        army_context_from_rows(rows, village_id)
    }

    async fn delete_by_home_village(&self, village_id: u32) -> Result<(), ApplicationError> {
        queries::delete_armies_by_home_village_query(village_id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(())
    }
}
