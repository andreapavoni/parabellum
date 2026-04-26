use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::{jobs::Job, repository::JobRepository};
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::toasty_models::job::JobRecord;

pub struct ToastyJobRepository<'a> {
    tx: Arc<Mutex<toasty::Transaction<'a>>>,
}

impl<'a> ToastyJobRepository<'a> {
    pub fn new(tx: Arc<Mutex<toasty::Transaction<'a>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> JobRepository for ToastyJobRepository<'a> {
    async fn add(&self, job: &Job) -> Result<(), ApplicationError> {
        let record = JobRecord::try_from(job)?;
        let mut tx_guard = self.tx.lock().await;

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
        let mut tx_guard = self.tx.lock().await;
        let record =
            JobRecord::get_by_id(&mut *tx_guard, job_id).await.map_err(map_toasty_error)?;
        Job::try_from(record)
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut rows = toasty::query!(JobRecord filter .player_id == #player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.retain(|row| row.status == "Pending" || row.status == "Processing");
        rows.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_active_jobs_by_village(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut rows = toasty::query!(JobRecord filter .village_id == #village_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        rows.retain(|row| row.status == "Pending" || row.status == "Processing");
        rows.sort_by(|a, b| a.completed_at.cmp(&b.completed_at));

        rows.into_iter()
            .map(Job::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn list_village_targeting_movements(
        &self,
        _village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        Err(unsupported("list_village_targeting_movements"))
    }

    async fn list_village_building_queue(
        &self,
        village_id: i32,
    ) -> Result<Vec<Job>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
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
        let mut tx_guard = self.tx.lock().await;
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
        let mut tx_guard = self.tx.lock().await;
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
        let mut tx_guard = self.tx.lock().await;
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

    async fn find_and_lock_due_jobs(&self, _limit: i64) -> Result<Vec<Job>, ApplicationError> {
        Err(unsupported("find_and_lock_due_jobs"))
    }

    async fn mark_as_completed(&self, job_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut row =
            JobRecord::get_by_id(&mut *tx_guard, job_id).await.map_err(map_toasty_error)?;
        row.update().status("Completed").exec(&mut *tx_guard).await.map_err(map_toasty_error)?;
        Ok(())
    }

    async fn reschedule(
        &self,
        job_id: Uuid,
        task: &parabellum_app::jobs::JobPayload,
        completed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut row =
            JobRecord::get_by_id(&mut *tx_guard, job_id).await.map_err(map_toasty_error)?;
        row.update()
            .task(task.clone())
            .completed_at(chrono_utc_to_jiff(completed_at)?)
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
        let mut tx_guard = self.tx.lock().await;
        let mut row =
            JobRecord::get_by_id(&mut *tx_guard, job_id).await.map_err(map_toasty_error)?;
        row.update().status("Failed").exec(&mut *tx_guard).await.map_err(map_toasty_error)?;
        Ok(())
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}

fn unsupported(method: &str) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(format!(
        "toasty adapter method not implemented yet: {method}"
    )))
}

fn chrono_utc_to_jiff(value: chrono::DateTime<chrono::Utc>) -> Result<jiff::Timestamp, ApplicationError> {
    jiff::Timestamp::from_second(value.timestamp())
        .and_then(|ts| ts.checked_add(jiff::SignedDuration::new(0, value.timestamp_subsec_nanos() as i32)))
        .map_err(|err| {
            ApplicationError::Db(DbError::Transaction(format!(
                "could not convert chrono datetime to jiff timestamp: {err}"
            )))
        })
}
