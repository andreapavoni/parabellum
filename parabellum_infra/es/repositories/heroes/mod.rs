//! Postgres-backed hero projection repository.

mod queries;
mod rows;

use parabellum_app::villages::projection_repositories::{HeroPlacementState, HeroRepository};
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::ProjectionDb;

use self::rows::DbHeroRow;

#[derive(Clone)]
pub struct PostgresHeroRepository {
    pool: PgPool,
}

impl PostgresHeroRepository {
    pub fn new(db: ProjectionDb) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn upsert_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        hero: &Hero,
        home_village_id: u32,
        current_village_id: u32,
        state: HeroPlacementState,
    ) -> Result<(), ApplicationError> {
        queries::upsert_hero_query(hero, home_village_id, current_village_id, state)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(database_error)?;
        Ok(())
    }

    pub async fn update_stats_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        hero: &Hero,
    ) -> Result<(), ApplicationError> {
        queries::update_hero_stats_query(hero)
            .build()
            .execute(&mut **tx)
            .await
            .map_err(database_error)?;
        Ok(())
    }
}

fn database_error(error: sqlx::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Database(error))
}

#[async_trait::async_trait]
impl HeroRepository for PostgresHeroRepository {
    async fn upsert(
        &self,
        hero: &Hero,
        home_village_id: u32,
        current_village_id: u32,
        state: HeroPlacementState,
    ) -> Result<(), ApplicationError> {
        let mut tx = self.pool.begin().await.map_err(database_error)?;
        self.upsert_in_tx(&mut tx, hero, home_village_id, current_village_id, state)
            .await?;
        tx.commit().await.map_err(database_error)?;
        Ok(())
    }

    async fn get_by_id(&self, hero_id: Uuid) -> Result<Hero, ApplicationError> {
        let row: DbHeroRow = queries::hero_by_id_query(hero_id)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(|_| ApplicationError::Db(DbError::HeroNotFound(hero_id)))?;

        Ok(row.into())
    }

    async fn get_by_player(&self, player_id: Uuid) -> Result<Option<Hero>, ApplicationError> {
        let row: Option<DbHeroRow> = queries::hero_by_player_query(player_id)
            .build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(database_error)?;

        Ok(row.map(Into::into))
    }

    async fn has_alive_for_player(&self, player_id: Uuid) -> Result<bool, ApplicationError> {
        queries::alive_hero_exists_for_player_query(player_id)
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(database_error)
    }
}
