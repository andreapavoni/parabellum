use parabellum_app::villages::models::{MovementDirection, VillageMovement};
use parabellum_app::villages::repositories::VillageMovementRepository;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, Postgres, Row, Transaction, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresVillageMovementRepository {
    pool: PgPool,
}

impl PostgresVillageMovementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        movement: &VillageMovement,
    ) -> Result<(), ApplicationError> {
        let village_id = match movement.direction {
            MovementDirection::Incoming => movement.target_village_id,
            MovementDirection::Outgoing => movement.origin_village_id,
        };
        sqlx::query(
            r#"
            INSERT INTO rm_village_movements (
                village_id, movement_id, direction, movement_type, source_village_id, target_village_id, eta, payload
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (village_id, movement_id, direction)
            DO UPDATE SET
                movement_type = EXCLUDED.movement_type,
                source_village_id = EXCLUDED.source_village_id,
                target_village_id = EXCLUDED.target_village_id,
                eta = EXCLUDED.eta,
                payload = EXCLUDED.payload,
                updated_at = NOW()
            "#,
        )
        .bind(village_id as i32)
        .bind(movement.movement_id)
        .bind(DbMovementDirection::from(movement.direction))
        .bind(DbMovementType::from(movement.movement_type))
        .bind(movement.origin_village_id as i32)
        .bind(movement.target_village_id as i32)
        .bind(movement.arrives_at)
        .bind(Json(movement))
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn delete_by_movement_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        movement_id: Uuid,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            DELETE FROM rm_village_movements
            WHERE movement_id = $1
            "#,
        )
        .bind(movement_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl VillageMovementRepository for PostgresVillageMovementRepository {
    async fn upsert(&self, movement: &VillageMovement) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.upsert_in_tx(&mut tx, movement).await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn list_by_village_id(
        &self,
        village_id: u32,
    ) -> Result<Vec<VillageMovement>, ApplicationError> {
        let rows = sqlx::query(
            r#"
            SELECT payload
            FROM rm_village_movements
            WHERE village_id = $1
            ORDER BY eta ASC
            "#,
        )
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        rows.into_iter()
            .map(|row| {
                let payload: serde_json::Value = row
                    .try_get("payload")
                    .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
                serde_json::from_value(payload).map_err(ApplicationError::from)
            })
            .collect()
    }

    async fn delete_by_movement_id(&self, movement_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.delete_by_movement_id_in_tx(&mut tx, movement_id)
            .await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "movement_type", rename_all = "PascalCase")]
enum DbMovementType {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

impl From<parabellum_app::villages::models::MovementType> for DbMovementType {
    fn from(value: parabellum_app::villages::models::MovementType) -> Self {
        match value {
            parabellum_app::villages::models::MovementType::Attack => Self::Attack,
            parabellum_app::villages::models::MovementType::Raid => Self::Raid,
            parabellum_app::villages::models::MovementType::Scout => Self::Scout,
            parabellum_app::villages::models::MovementType::Reinforcement => Self::Reinforcement,
            parabellum_app::villages::models::MovementType::Return => Self::Return,
            parabellum_app::villages::models::MovementType::FoundVillage => Self::FoundVillage,
        }
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "movement_direction", rename_all = "PascalCase")]
enum DbMovementDirection {
    Incoming,
    Outgoing,
}

impl From<parabellum_app::villages::models::MovementDirection> for DbMovementDirection {
    fn from(value: parabellum_app::villages::models::MovementDirection) -> Self {
        match value {
            parabellum_app::villages::models::MovementDirection::Incoming => Self::Incoming,
            parabellum_app::villages::models::MovementDirection::Outgoing => Self::Outgoing,
        }
    }
}
