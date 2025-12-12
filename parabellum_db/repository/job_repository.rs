use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::{jobs::Job, repository::JobRepository};
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::models as db_models;

#[derive(Clone)]
pub struct PostgresJobRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresJobRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> JobRepository for PostgresJobRepository<'a> {
    async fn add(&self, job: &Job) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!(
            r#"
          INSERT INTO jobs (id, player_id, village_id, task, status, completed_at)
          VALUES ($1, $2, $3, $4, 'Pending', $5)
          "#,
            job.id,
            job.player_id,
            job.village_id,
            Json(&job.task) as _,
            job.completed_at
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, job_id: Uuid) -> Result<Job, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let job = sqlx::query_as!(
          db_models::Job,
          r#"SELECT id, player_id, village_id, task, status AS "status: _", completed_at, created_at, updated_at FROM jobs WHERE id = $1"#,
          job_id
      )
      .fetch_one(&mut *tx_guard.as_mut())
      .await.map_err(|_| ApplicationError::Db(DbError::JobNotFound(job_id)))?;

        Ok(job.into())
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let jobs = sqlx::query_as!(
          db_models::Job,
          r#"SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at FROM jobs WHERE player_id = $1 AND status IN ('Pending', 'Processing') ORDER BY completed_at ASC"#,
          player_id
      )
      .fetch_all(&mut *tx_guard.as_mut())
      .await.map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn list_active_jobs_by_village(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let jobs = sqlx::query_as!(
            db_models::Job,
            r#"
            SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at
            FROM jobs
            WHERE village_id = $1
              AND status IN ('Pending', 'Processing')
            ORDER BY completed_at ASC
            "#,
            village_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn list_village_targeting_movements(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let jobs = sqlx::query_as!(
            db_models::Job,
            r#"
            SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at
            FROM jobs
            WHERE status IN ('Pending', 'Processing')
              AND (
                (task ->> 'task_type' = 'Attack' AND (task -> 'data' ->> 'target_village_id')::integer = $1)
                OR (task ->> 'task_type' = 'Scout' AND (task -> 'data' ->> 'target_village_id')::integer = $1)
                OR (task ->> 'task_type' = 'Reinforcement' AND (task -> 'data' ->> 'village_id')::integer = $1)
              )
            ORDER BY completed_at ASC
            "#,
            village_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn list_village_building_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let jobs = sqlx::query_as!(
            db_models::Job,
            r#"
            SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at
            FROM jobs
            WHERE village_id = $1
              AND status IN ('Pending', 'Processing')
              AND task ->> 'task_type' IN ('AddBuilding', 'BuildingUpgrade')
            ORDER BY completed_at ASC
            "#,
            village_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn list_village_training_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let jobs = sqlx::query_as!(
            db_models::Job,
            r#"
            SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at
            FROM jobs
            WHERE village_id = $1
              AND status IN ('Pending', 'Processing')
              AND task ->> 'task_type' = 'TrainUnits'
            ORDER BY completed_at ASC
            "#,
            village_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn list_village_academy_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let jobs = sqlx::query_as!(
            db_models::Job,
            r#"
            SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at
            FROM jobs
            WHERE village_id = $1
              AND status IN ('Pending', 'Processing')
              AND task ->> 'task_type' = 'ResearchAcademy'
            ORDER BY completed_at ASC
            "#,
            village_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn list_village_smithy_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let jobs = sqlx::query_as!(
            db_models::Job,
            r#"
            SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at
            FROM jobs
            WHERE village_id = $1
              AND status IN ('Pending', 'Processing')
              AND task ->> 'task_type' = 'ResearchSmithy'
            ORDER BY completed_at ASC
            "#,
            village_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn find_and_lock_due_jobs(&self, limit: i64) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let due_jobs = sqlx::query_as!(
          db_models::Job,
          r#"
          UPDATE jobs
          SET status = 'Processing', updated_at = NOW()
          WHERE id IN (
              SELECT id
              FROM jobs
              WHERE status = 'Pending' AND completed_at <= NOW()
              ORDER BY completed_at
              LIMIT $1
              FOR UPDATE SKIP LOCKED
          )
          RETURNING id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at;
          "#,
          limit
      )
      .fetch_all(&mut *tx_guard.as_mut())
      .await.map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(due_jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn mark_as_completed(&self, job_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!("UPDATE jobs SET status = 'Completed' WHERE id = $1", job_id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn mark_as_failed(
        &self,
        job_id: Uuid,
        _error_message: &str,
    ) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        sqlx::query!("UPDATE jobs SET status = 'Failed' WHERE id = $1", job_id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }
}
