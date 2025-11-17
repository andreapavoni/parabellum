use crate::jobs::{
    Job, JobPayload,
    handler::{JobHandler, JobHandlerContext},
    tasks::TrainUnitsTask,
};

use async_trait::async_trait;
use parabellum_core::ApplicationError;
use parabellum_game::models::army::Army;
use tracing::{info, instrument};

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
        quantity = self.payload.quantity,
        village_id = job.village_id
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        let village_id = job.village_id as u32;
        let player_id = job.player_id;

        info!("Executing TrainUnits job");
        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();
        let mut village = village_repo.get_by_id(village_id).await?;

        let mut village_army = village
            .army()
            .map_or(Army::new_village_army(&village), |a| a.clone());

        village_army.add_unit(self.payload.unit.clone(), 1)?;
        village.set_army(Some(&village_army))?;

        army_repo.save(&village_army).await?;
        village_repo.save(&village).await?;

        if self.payload.quantity > 1 {
            let next_payload = TrainUnitsTask {
                quantity: self.payload.quantity - 1, // Train one less
                ..self.payload.clone()
            };

            let job_payload = JobPayload::new("TrainUnits", serde_json::to_value(&next_payload)?);
            let next_job = Job::new(
                player_id,
                village_id as i32,
                self.payload.time_per_unit as i64, // Schedule for one unit's time
                job_payload,
            );

            ctx.uow.jobs().add(&next_job).await?;
            info!(next_job_id = %next_job.id, "Queued next unit training job");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_core::Result;
    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
    };
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
        let payload = TrainUnitsTask {
            slot_id: 20,
            unit: UnitName::Legionnaire,
            quantity,
            time_per_unit: 100,
        };
        let job_payload = JobPayload::new("TrainUnits", json!(payload.clone()));
        let job = Job::new(player_id, village_id as i32, 0, job_payload);

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
        assert_eq!(army.units()[0], 1, "Should have trained exactly 1 unit");

        let saved_army = context.uow.armies().get_by_id(army.id).await?;
        assert_eq!(saved_army.units()[0], 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_job_handler_queues_next_job() -> Result<()> {
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
        assert_eq!(jobs.len(), 1, "A new job should be queued");

        let next_job = &jobs[0];
        assert_eq!(next_job.task.task_type, "TrainUnits");

        let next_task: TrainUnitsTask = serde_json::from_value(next_job.task.data.clone())?;
        assert_eq!(next_task.quantity, 4, "Next job should have quantity - 1");
        assert_eq!(next_task.unit, UnitName::Legionnaire);
        Ok(())
    }

    #[tokio::test]
    async fn test_train_units_job_handler_finishes_queue() -> Result<()> {
        // We start with a job for only 1 unit
        let (job, payload, config, uow, village_id, player_id) = setup_test(1).await?;
        let handler = TrainUnitsJobHandler::new(payload);
        let context = JobHandlerContext { uow, config };

        handler.handle(&context, &job).await?;

        // Check that NO new job was created
        let jobs = context.uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(jobs.len(), 0, "No new job should be queued");

        // Check that the unit was still trained
        let final_village = context.uow.villages().get_by_id(village_id).await?;
        let army = final_village.army().expect("Village should have an army");
        assert_eq!(army.units()[0], 1, "Should have trained the last unit");
        Ok(())
    }
}
