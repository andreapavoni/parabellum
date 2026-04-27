use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::{jobs::Job, repository::JobRepository};
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::toasty_models::job::JobRecord;
use crate::toasty_time::chrono_to_jiff_utc;

pub struct ToastyJobRepository {
    db: Arc<Mutex<toasty::Db>>,
}

impl ToastyJobRepository {
    pub fn new(db: Arc<Mutex<toasty::Db>>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl JobRepository for ToastyJobRepository {
    async fn add(&self, job: &Job) -> Result<(), ApplicationError> {
        let record = JobRecord::try_from(job)?;
        let mut tx_guard = self.db.lock().await;

        toasty::create!(JobRecord {
            id: record.id,
            player_id: record.player_id,
            village_id: record.village_id,
            task: record.task,
            status: record.status,
            completed_at: record.completed_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        Ok(())
    }

    async fn get_by_id(&self, job_id: Uuid) -> Result<Job, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let record = JobRecord::get_by_id(&mut *tx_guard, job_id)
            .await
            .map_err(map_toasty_error)?;
        Job::try_from(record)
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let rows = JobRecord::filter_by_player_id(player_id)
            .filter(
                JobRecord::fields()
                    .status()
                    .in_list(["Pending", "Processing"]),
            )
            .order_by(JobRecord::fields().completed_at().asc())
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_active_jobs_by_village(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let rows = JobRecord::filter_by_village_id(village_id)
            .filter(
                JobRecord::fields()
                    .status()
                    .in_list(["Pending", "Processing"]),
            )
            .order_by(JobRecord::fields().completed_at().asc())
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_village_targeting_movements(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let rows = toasty::query!(
            JobRecord filter .status == "Pending" or .status == "Processing"
        )
        .order_by(JobRecord::fields().completed_at().asc())
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        let rows: Vec<_> = rows
            .into_iter()
            .filter(|row| row_targets_village(row, village_id))
            .collect();

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_village_building_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut rows = toasty::query!(JobRecord filter .village_id == #village_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.retain(|row| {
            (row.status == "Pending" || row.status == "Processing")
                && (row.task.task_type == "AddBuilding" || row.task.task_type == "BuildingUpgrade")
        });
        rows.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_village_training_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut rows = toasty::query!(JobRecord filter .village_id == #village_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.retain(|row| {
            (row.status == "Pending" || row.status == "Processing")
                && row.task.task_type == "TrainUnits"
        });
        rows.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_village_academy_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut rows = toasty::query!(JobRecord filter .village_id == #village_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.retain(|row| {
            (row.status == "Pending" || row.status == "Processing")
                && row.task.task_type == "ResearchAcademy"
        });
        rows.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_village_smithy_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut rows = toasty::query!(JobRecord filter .village_id == #village_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.retain(|row| {
            (row.status == "Pending" || row.status == "Processing")
                && row.task.task_type == "ResearchSmithy"
        });
        rows.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn find_and_lock_due_jobs(&self, limit: i64) -> Result<Vec<Job>, ApplicationError> {
        let now = jiff::Timestamp::now();
        let limit = usize::try_from(limit).unwrap_or(0);
        if limit == 0 {
            return Ok(vec![]);
        }

        let mut tx_guard = self.db.lock().await;
        let mut rows = toasty::query!(
            JobRecord filter .status == "Pending" and .completed_at <= #now
        )
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        rows.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));
        rows.truncate(limit);

        for row in &mut rows {
            row.update()
                .status("Processing")
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
            row.status = "Processing".to_string();
        }

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn mark_as_completed(&self, job_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut row = JobRecord::get_by_id(&mut *tx_guard, job_id)
            .await
            .map_err(map_toasty_error)?;
        row.update()
            .status("Completed")
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        Ok(())
    }

    async fn reschedule(
        &self,
        job_id: Uuid,
        task: &parabellum_app::jobs::JobPayload,
        completed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut row = JobRecord::get_by_id(&mut *tx_guard, job_id)
            .await
            .map_err(map_toasty_error)?;
        row.update()
            .task(task.clone())
            .completed_at(chrono_to_jiff_utc(completed_at)?)
            .status("Pending")
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        Ok(())
    }

    async fn mark_as_failed(
        &self,
        job_id: Uuid,
        _error_message: &str,
    ) -> Result<(), ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let mut row = JobRecord::get_by_id(&mut *tx_guard, job_id)
            .await
            .map_err(map_toasty_error)?;
        row.update()
            .status("Failed")
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        Ok(())
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}

fn row_targets_village(row: &JobRecord, village_id: i32) -> bool {
    let task_type = row.task.task_type.as_str();
    let data = &row.task.data;

    match task_type {
        "Attack" | "Scout" => json_i32(data, "target_village_id") == Some(village_id),
        "Reinforcement" => json_i32(data, "village_id") == Some(village_id),
        "MerchantGoing" => json_i32(data, "destination_village_id") == Some(village_id),
        _ => false,
    }
}

fn json_i32(value: &serde_json::Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(|v| v.as_i64())
        .and_then(|n| i32::try_from(n).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    use crate::{establish_test_connection_pool, toasty_db::establish_test_toasty_db};

    #[tokio::test]
    async fn toasty_job_repo_add_get_and_list_by_player_id() -> Result<(), ApplicationError> {
        let pool = establish_test_connection_pool()
            .await
            .map_err(ApplicationError::Db)?;
        let toasty_db = Arc::new(Mutex::new(
            establish_test_toasty_db()
                .await
                .map_err(ApplicationError::Db)?,
        ));

        let seed: Option<(i32, Uuid)> = sqlx::query_as(
            r#"
            SELECT v.id, v.player_id
            FROM villages v
            LIMIT 1
            "#,
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let Some(seed) = seed else {
            return Ok(());
        };

        let repo = ToastyJobRepository::new(toasty_db.clone());

        let payload = parabellum_app::jobs::JobPayload::new(
            "ToastyTestTask",
            serde_json::json!({ "kind": "smoke", "value": 1 }),
        );
        let job = Job::with_deadline(seed.1, seed.0, payload, Utc::now() + Duration::minutes(5));

        repo.add(&job).await?;
        let loaded = repo.get_by_id(job.id).await?;
        let listed = repo.list_by_player_id(seed.1).await?;

        assert_eq!(loaded.id, job.id);
        assert!(listed.iter().any(|j| j.id == job.id));

        drop(repo);
        drop(toasty_db);

        Ok(())
    }
}
