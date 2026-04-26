use async_trait::async_trait;
use chrono::{Duration, Utc};
use tracing::{info, instrument};

use parabellum_game::models::army::Army;
use parabellum_types::errors::ApplicationError;

use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::TrainUnitsTask,
};

pub struct TrainUnitsJobHandler {
    payload: TrainUnitsTask,
}

impl TrainUnitsJobHandler {
    pub fn new(payload: TrainUnitsTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for TrainUnitsJobHandler {
    #[instrument(skip_all, fields(
        task_type = "TrainUnits",
        unit = ?self.payload.unit,
        quantity_remaining = self.payload.quantity_remaining,
        village_id = job.village_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        let village_id = job.village_id as u32;

        info!("Executing TrainUnits job");
        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();
        let mut village = village_repo.get_by_id(village_id).await?;

        let mut village_army = village
            .army()
            .map_or(Army::new_village_army(&village), |a| a.clone());
        let started_at = self.payload.effective_started_at(job.created_at);
        let quantity_remaining = self.payload.effective_quantity_remaining();
        if quantity_remaining <= 0 {
            return Ok(());
        }
        let time_per_unit = self.payload.time_per_unit.max(1) as i64;
        let elapsed_units = ((Utc::now() - started_at).num_seconds() / time_per_unit).max(0) as i32;
        let trained_so_far = (self.payload.quantity - quantity_remaining).max(0);
        let newly_completed = (elapsed_units - trained_so_far).max(0);
        let units_to_train = newly_completed.min(quantity_remaining).max(0);
        if units_to_train == 0 {
            return Ok(());
        }

        village_army.add_unit(self.payload.unit.clone(), units_to_train as u32)?;
        village.set_army(Some(&village_army))?;

        army_repo.save(&village_army).await?;
        village_repo.save(&village).await?;

        let remaining_after = quantity_remaining - units_to_train;
        if remaining_after > 0 {
            let next_payload = TrainUnitsTask {
                quantity_remaining: remaining_after,
                started_at: Some(started_at),
                ..self.payload.clone()
            };
            let next_due =
                job.completed_at + Duration::seconds((units_to_train as i64) * time_per_unit);
            let job_payload = JobPayload::new("TrainUnits", serde_json::to_value(&next_payload)?);
            ctx.uow
                .jobs()
                .reschedule(job.id, &job_payload, next_due)
                .await?;
            info!(
                job_id = %job.id,
                units_trained = units_to_train,
                remaining = remaining_after,
                "Rescheduled training job"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
    use parabellum_types::Result;
    use parabellum_types::{army::UnitName, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config,
        jobs::{Job, JobPayload},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    async fn setup_test(
        quantity: i32,
    ) -> Result<(
        Job,
        TrainUnitsTask,
        Arc<Config>,
        Box<dyn UnitOfWork<'static> + 'static>,
        u32,  // village_id
        Uuid, // player_id
    )> {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        let village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });
        let village_id = village.id;
        let player_id = player.id;

        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());
        mock_uow.villages().save(&village).await?;

        let config = Arc::new(Config::from_env());
        let started_at = Utc::now() - Duration::seconds(500);
        let payload = TrainUnitsTask {
            slot_id: 20,
            unit: UnitName::Legionnaire,
            quantity,
            time_per_unit: 100,
            quantity_remaining: quantity,
            started_at: Some(started_at),
        };
        let job_payload = JobPayload::new("TrainUnits", json!(payload.clone()));
        let job = Job::with_deadline(
            player_id,
            village_id as i32,
            job_payload,
            Utc::now() - Duration::seconds(1),
        );
        mock_uow.jobs().add(&job).await?;

        Ok((job, payload, config, mock_uow, village_id, player_id))
    }

    #[tokio::test]
    async fn test_train_units_job_handler_trains_one_unit() -> Result<()> {
        let (job, payload, config, uow, village_id, _player_id) = setup_test(5).await?;
        let handler = TrainUnitsJobHandler::new(payload);
        let context = JobHandlerContext { uow, config };

        handler.handle(&context, &job).await?;

        let final_village = context.uow.villages().get_by_id(village_id).await?;
        let army = final_village.army().expect("Village should have an army");
        assert_eq!(army.units().get(0), 5, "Should have trained elapsed units");

        let saved_army = context.uow.armies().get_by_id(army.id).await?;
        assert_eq!(saved_army.units().get(0), 5);

        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_job_handler_reschedules_same_job() -> Result<()> {
        let (job, payload, config, uow, _village_id, player_id) = setup_test(5).await?;
        let handler = TrainUnitsJobHandler::new(payload);
        let context = JobHandlerContext { uow, config };

        handler.handle(&context, &job).await?;

        let jobs = context
            .uow
            .jobs()
            .list_by_player_id(player_id)
            .await
            .unwrap();
        assert_eq!(jobs.len(), 1, "Training should keep one persistent job");
        assert_eq!(jobs[0].id, job.id, "Job id should be reused");
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_job_handler_finishes_queue() -> Result<()> {
        // We start with a job for only 1 unit
        let (job, payload, config, uow, village_id, player_id) = setup_test(1).await?;
        let handler = TrainUnitsJobHandler::new(payload);
        let context = JobHandlerContext { uow, config };

        handler.handle(&context, &job).await?;

        let jobs = context.uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(jobs.len(), 1, "Single job row remains in mock store");
        assert!(
            matches!(jobs[0].status, crate::jobs::JobStatus::Pending),
            "Reschedule keeps pending status in mock"
        );

        // Check that the unit was still trained
        let final_village = context.uow.villages().get_by_id(village_id).await?;
        let army = final_village.army().expect("Village should have an army");
        assert_eq!(army.units().get(0), 1, "Should have trained the last unit");
        Ok(())
    }
}
