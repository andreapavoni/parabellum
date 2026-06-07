use parabellum_app::ports::queries::{
    AcademyQueueItem, BuildingQueueItem, SmithyQueueItem, TrainingQueueItem, VillageQueues,
};
use parabellum_app::villages::models::{
    ScheduledAction, ScheduledActionPayload, ScheduledActionStatus, ScheduledActionType,
};
use parabellum_app::villages::queries::ScheduledActionStatusCounts;
use parabellum_app::villages::repositories::ScheduledActionRepository;
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{FromRow, PgPool, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresScheduledActionRepository {
    pool: PgPool,
}

impl PostgresScheduledActionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn requeue_stale_processing(
        &self,
        updated_before_or_equal: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, ApplicationError> {
        let result = sqlx::query(
            r#"
            UPDATE rm_scheduled_actions
            SET status = 'pending', updated_at = NOW()
            WHERE status = 'processing'
              AND updated_at <= $1
            "#,
        )
        .bind(updated_before_or_equal)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(result.rows_affected())
    }

    pub async fn add_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        action: &ScheduledAction,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            INSERT INTO rm_scheduled_actions (id, action_type, execute_at, payload, status)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(action.id)
        .bind(DbScheduledActionType::from(action.action_type))
        .bind(action.execute_at)
        .bind(Json(&action.payload))
        .bind(DbScheduledActionStatus::from(action.status))
        .execute(&mut **tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    pub async fn has_pending_hero_revival_for_player(
        &self,
        player_id: Uuid,
    ) -> Result<bool, ApplicationError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM rm_scheduled_actions
                WHERE action_type = 'HeroRevival'
                  AND status = 'pending'
                  AND payload->'workflow'->>'player_id' = $1
            )
            "#,
        )
        .bind(player_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(exists)
    }

    pub async fn list_village_queues(
        &self,
        village_id: u32,
    ) -> Result<VillageQueues, ApplicationError> {
        let rows: Vec<DbScheduledActionRow> = sqlx::query_as(
            r#"
            SELECT id, action_type, execute_at, payload, status
            FROM rm_scheduled_actions
            WHERE (payload->'workflow'->>'village_id')::int = $1
              AND status IN ('pending', 'processing')
              AND action_type IN (
                'AddBuilding',
                'UpgradeBuilding',
                'DowngradeBuilding',
                'TrainUnit',
                'ResearchAcademy',
                'ResearchSmithy'
              )
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut queues = VillageQueues::default();
        for row in rows {
            let status = row.status.into();
            let payload: ScheduledActionPayload = serde_json::from_value(row.payload)
                .map_err(|e| ApplicationError::Unknown(e.to_string()))?;
            match payload {
                ScheduledActionPayload::Building { workflow } => {
                    queues.building.push(BuildingQueueItem {
                        job_id: row.id,
                        kind: workflow.kind,
                        slot_id: workflow.slot_id,
                        building_name: workflow.building_name,
                        target_level: workflow.level,
                        status,
                        finishes_at: row.execute_at,
                    });
                }
                ScheduledActionPayload::Training { workflow } => {
                    queues.training.push(TrainingQueueItem {
                        job_id: row.id,
                        slot_id: workflow.slot_id,
                        unit: workflow.unit,
                        quantity: workflow.quantity_remaining,
                        time_per_unit: workflow.time_per_unit,
                        status,
                        finishes_at: row.execute_at,
                    });
                }
                ScheduledActionPayload::Research { workflow } => match row.action_type.into() {
                    ScheduledActionType::ResearchAcademy => queues.academy.push(AcademyQueueItem {
                        job_id: row.id,
                        unit: workflow.unit,
                        status,
                        finishes_at: row.execute_at,
                    }),
                    ScheduledActionType::ResearchSmithy => queues.smithy.push(SmithyQueueItem {
                        job_id: row.id,
                        unit: workflow.unit,
                        status,
                        finishes_at: row.execute_at,
                    }),
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(queues)
    }
}

#[derive(Debug, Clone, FromRow)]
struct DbScheduledActionRow {
    id: Uuid,
    action_type: DbScheduledActionType,
    execute_at: chrono::DateTime<chrono::Utc>,
    payload: serde_json::Value,
    status: DbScheduledActionStatus,
}

impl From<DbScheduledActionRow> for ScheduledAction {
    fn from(value: DbScheduledActionRow) -> Self {
        Self {
            id: value.id,
            action_type: value.action_type.into(),
            execute_at: value.execute_at,
            payload: value.payload,
            status: value.status.into(),
        }
    }
}

#[async_trait::async_trait]
impl ScheduledActionRepository for PostgresScheduledActionRepository {
    async fn add(&self, action: &ScheduledAction) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        self.add_in_tx(&mut tx, action).await?;
        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<ScheduledAction, ApplicationError> {
        let row: DbScheduledActionRow = sqlx::query_as(
            r#"
            SELECT id, action_type, execute_at, payload, status
            FROM rm_scheduled_actions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.into())
    }

    async fn take_due_pending(
        &self,
        before_or_equal: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let rows: Vec<DbScheduledActionRow> = sqlx::query_as(
            r#"
            WITH due AS (
                SELECT id
                FROM rm_scheduled_actions
                WHERE status = 'pending' AND execute_at <= $1
                ORDER BY execute_at ASC
                LIMIT $2
                FOR UPDATE SKIP LOCKED
            )
            UPDATE rm_scheduled_actions a
            SET status = 'processing', updated_at = NOW()
            FROM due
            WHERE a.id = due.id
            RETURNING a.id, a.action_type, a.execute_at, a.payload, a.status
            "#,
        )
        .bind(before_or_equal)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: ScheduledActionStatus,
    ) -> Result<(), ApplicationError> {
        sqlx::query(
            r#"
            UPDATE rm_scheduled_actions
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(DbScheduledActionStatus::from(status))
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn list_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        let rows: Vec<DbScheduledActionRow> = sqlx::query_as(
            r#"
            SELECT id, action_type, execute_at, payload, status
            FROM rm_scheduled_actions
            WHERE action_type = $1
              AND (payload->'workflow'->>'village_id')::int = $2
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(DbScheduledActionType::from(action_type))
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        let rows: Vec<DbScheduledActionRow> = sqlx::query_as(
            r#"
            SELECT id, action_type, execute_at, payload, status
            FROM rm_scheduled_actions
            WHERE action_type = $1
              AND (payload->'workflow'->>'target_village_id')::int = $2
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(DbScheduledActionType::from(action_type))
        .bind(target_village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_active_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        let rows: Vec<DbScheduledActionRow> = sqlx::query_as(
            r#"
            SELECT id, action_type, execute_at, payload, status
            FROM rm_scheduled_actions
            WHERE action_type = $1
              AND (payload->'workflow'->>'village_id')::int = $2
              AND status IN ('pending', 'processing')
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(DbScheduledActionType::from(action_type))
        .bind(village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_active_by_target_village_and_type(
        &self,
        target_village_id: u32,
        action_type: ScheduledActionType,
    ) -> Result<Vec<ScheduledAction>, ApplicationError> {
        let rows: Vec<DbScheduledActionRow> = sqlx::query_as(
            r#"
            SELECT id, action_type, execute_at, payload, status
            FROM rm_scheduled_actions
            WHERE action_type = $1
              AND (payload->'workflow'->>'target_village_id')::int = $2
              AND status IN ('pending', 'processing')
            ORDER BY execute_at ASC, created_at ASC
            "#,
        )
        .bind(DbScheduledActionType::from(action_type))
        .bind(target_village_id as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_by_village_and_type(
        &self,
        village_id: u32,
        action_type: ScheduledActionType,
        status_filter: Option<ScheduledActionStatus>,
    ) -> Result<ScheduledActionStatusCounts, ApplicationError> {
        let status_filter = status_filter.map(DbScheduledActionStatus::from);
        let row: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
              COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0)::bigint AS pending_count,
              COALESCE(SUM(CASE WHEN status = 'processing' THEN 1 ELSE 0 END), 0)::bigint AS processing_count,
              COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0)::bigint AS completed_count,
              COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0)::bigint AS failed_count
            FROM rm_scheduled_actions
            WHERE action_type = $1
              AND (payload->'workflow'->>'village_id')::int = $2
              AND ($3::scheduled_action_status IS NULL OR status = $3)
            "#,
        )
        .bind(DbScheduledActionType::from(action_type))
        .bind(village_id as i32)
        .bind(status_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(ScheduledActionStatusCounts {
            pending: row.0 as usize,
            processing: row.1 as usize,
            completed: row.2 as usize,
            failed: row.3 as usize,
        })
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "scheduled_action_status", rename_all = "lowercase")]
enum DbScheduledActionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "scheduled_action_type", rename_all = "PascalCase")]
enum DbScheduledActionType {
    ReinforcementArrival,
    SettlersArrival,
    AttackArrival,
    ArmyReturn,
    ScoutArrival,
    MerchantArrival,
    MerchantReturn,
    AddBuilding,
    UpgradeBuilding,
    DowngradeBuilding,
    TrainUnit,
    ResearchAcademy,
    ResearchSmithy,
    HeroRevival,
}

impl From<DbScheduledActionStatus> for ScheduledActionStatus {
    fn from(value: DbScheduledActionStatus) -> Self {
        match value {
            DbScheduledActionStatus::Pending => Self::Pending,
            DbScheduledActionStatus::Processing => Self::Processing,
            DbScheduledActionStatus::Completed => Self::Completed,
            DbScheduledActionStatus::Failed => Self::Failed,
        }
    }
}

impl From<ScheduledActionStatus> for DbScheduledActionStatus {
    fn from(value: ScheduledActionStatus) -> Self {
        match value {
            ScheduledActionStatus::Pending => Self::Pending,
            ScheduledActionStatus::Processing => Self::Processing,
            ScheduledActionStatus::Completed => Self::Completed,
            ScheduledActionStatus::Failed => Self::Failed,
        }
    }
}

impl From<DbScheduledActionType> for ScheduledActionType {
    fn from(value: DbScheduledActionType) -> Self {
        match value {
            DbScheduledActionType::ReinforcementArrival => Self::ReinforcementArrival,
            DbScheduledActionType::SettlersArrival => Self::SettlersArrival,
            DbScheduledActionType::AttackArrival => Self::AttackArrival,
            DbScheduledActionType::ArmyReturn => Self::ArmyReturn,
            DbScheduledActionType::ScoutArrival => Self::ScoutArrival,
            DbScheduledActionType::MerchantArrival => Self::MerchantsArrival,
            DbScheduledActionType::MerchantReturn => Self::MerchantsReturn,
            DbScheduledActionType::AddBuilding => Self::AddBuilding,
            DbScheduledActionType::UpgradeBuilding => Self::UpgradeBuilding,
            DbScheduledActionType::DowngradeBuilding => Self::DowngradeBuilding,
            DbScheduledActionType::TrainUnit => Self::TrainUnit,
            DbScheduledActionType::ResearchAcademy => Self::ResearchAcademy,
            DbScheduledActionType::ResearchSmithy => Self::ResearchSmithy,
            DbScheduledActionType::HeroRevival => Self::HeroRevival,
        }
    }
}

impl From<ScheduledActionType> for DbScheduledActionType {
    fn from(value: ScheduledActionType) -> Self {
        match value {
            ScheduledActionType::ReinforcementArrival => Self::ReinforcementArrival,
            ScheduledActionType::SettlersArrival => Self::SettlersArrival,
            ScheduledActionType::AttackArrival => Self::AttackArrival,
            ScheduledActionType::ArmyReturn => Self::ArmyReturn,
            ScheduledActionType::ScoutArrival => Self::ScoutArrival,
            ScheduledActionType::MerchantsArrival => Self::MerchantArrival,
            ScheduledActionType::MerchantsReturn => Self::MerchantReturn,
            ScheduledActionType::AddBuilding => Self::AddBuilding,
            ScheduledActionType::UpgradeBuilding => Self::UpgradeBuilding,
            ScheduledActionType::DowngradeBuilding => Self::DowngradeBuilding,
            ScheduledActionType::TrainUnit => Self::TrainUnit,
            ScheduledActionType::ResearchAcademy => Self::ResearchAcademy,
            ScheduledActionType::ResearchSmithy => Self::ResearchSmithy,
            ScheduledActionType::HeroRevival => Self::HeroRevival,
        }
    }
}
