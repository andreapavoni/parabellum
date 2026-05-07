use parabellum_game::models::army::Army;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresArmyModelRepository {
    pool: PgPool,
}

impl PostgresArmyModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_home(&self, army: &Army, player_id: Uuid) -> Result<(), ApplicationError> {
        self.upsert(army, army.village_id, player_id, "home").await
    }

    pub async fn upsert_moving(
        &self,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert(army, current_village_id, player_id, "moving")
            .await
    }

    pub async fn upsert_stationed(
        &self,
        army: &Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert(army, stationed_village_id, player_id, "stationed")
            .await
    }

    async fn upsert(
        &self,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
        state: &str,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            INSERT INTO rm_armies (
                army_id, village_id, current_village_id, player_id, state, payload, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (army_id) DO UPDATE SET
                village_id = EXCLUDED.village_id,
                current_village_id = EXCLUDED.current_village_id,
                player_id = EXCLUDED.player_id,
                state = EXCLUDED.state,
                payload = EXCLUDED.payload,
                updated_at = NOW()
            "#,
        )
        .bind(army.id)
        .bind(army.village_id as i32)
        .bind(current_village_id as i32)
        .bind(player_id)
        .bind(state)
        .bind(Json(army))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn delete(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        sqlx::query("DELETE FROM rm_armies WHERE army_id = $1")
            .bind(army_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn find_stationed_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        let row: Option<(i32, Json<Army>)> = sqlx::query_as(
            r#"
            SELECT current_village_id, payload
            FROM rm_armies
            WHERE army_id = $1 AND state = 'stationed'
            "#,
        )
        .bind(army_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.map(|(stationed_village_id, army)| (stationed_village_id as u32, army.0)))
    }

    pub async fn find_moving_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<Army>, ApplicationError> {
        let row: Option<Json<Army>> = sqlx::query_scalar(
            r#"
            SELECT payload
            FROM rm_armies
            WHERE army_id = $1 AND state = 'moving'
            "#,
        )
        .bind(army_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.map(|army| army.0))
    }
}
