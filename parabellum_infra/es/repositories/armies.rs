use parabellum_app::villages::repositories::ArmyRepository;
use parabellum_game::models::army::Army;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresArmyRepository {
    pool: PgPool,
}

impl PostgresArmyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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

    async fn delete_row(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        sqlx::query("DELETE FROM rm_armies WHERE army_id = $1")
            .bind(army_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn find_stationed_context(
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

    async fn get_moving_by_army_id(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
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

        row.map(|army| army.0)
            .ok_or(ApplicationError::Db(DbError::ArmyNotFound(army_id)))
    }
}

#[async_trait::async_trait]
impl ArmyRepository for PostgresArmyRepository {
    async fn upsert_home(&self, army: &Army, player_id: Uuid) -> Result<(), ApplicationError> {
        // Keep exactly one canonical home army row per village.
        sqlx::query(
            r#"
            DELETE FROM rm_armies
            WHERE village_id = $1
              AND state = 'home'
              AND army_id <> $2
            "#,
        )
        .bind(army.village_id as i32)
        .bind(army.id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        self.upsert(army, army.village_id, player_id, "home").await
    }

    async fn upsert_moving(
        &self,
        army: &Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert(army, current_village_id, player_id, "moving")
            .await
    }

    async fn upsert_stationed(
        &self,
        army: &Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError> {
        self.upsert(army, stationed_village_id, player_id, "stationed")
            .await
    }

    async fn delete(&self, army_id: Uuid) -> Result<(), ApplicationError> {
        self.delete_row(army_id).await
    }

    async fn get_home_army(&self, village_id: u32) -> Result<Option<Army>, ApplicationError> {
        let row: Option<Json<Army>> = sqlx::query_scalar(
            r#"
            SELECT payload
            FROM rm_armies
            WHERE village_id = $1
              AND current_village_id = $1
              AND state = 'home'
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(village_id as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(row.map(|it| it.0))
    }

    async fn list_stationed_armies(&self, village_id: u32) -> Result<Vec<Army>, ApplicationError> {
        let rows: Vec<Json<Army>> = sqlx::query_scalar(
            r#"
            SELECT payload
            FROM rm_armies
            WHERE current_village_id = $1
              AND state = 'stationed'
            ORDER BY updated_at DESC
            "#,
        )
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(rows.into_iter().map(|it| it.0).collect())
    }

    async fn list_deployed_armies(
        &self,
        home_village_id: u32,
    ) -> Result<Vec<Army>, ApplicationError> {
        let rows: Vec<Json<Army>> = sqlx::query_scalar(
            r#"
            SELECT payload
            FROM rm_armies
            WHERE village_id = $1
              AND state = 'stationed'
              AND current_village_id <> $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(home_village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(rows.into_iter().map(|it| it.0).collect())
    }

    async fn get_moving_army(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
        self.get_moving_by_army_id(army_id).await
    }

    async fn find_stationed_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, Army)>, ApplicationError> {
        self.find_stationed_context(army_id).await
    }
}
