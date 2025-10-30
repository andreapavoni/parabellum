use crate::{
    game::models::army::Army, // Assicurati di avere un modello Army
    jobs::{
        handler::{JobHandler, JobHandlerContext},
        tasks::TrainUnitsTask,
        Job, JobTask,
    },
};
use anyhow::Result;
use async_trait::async_trait;
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
    ) -> Result<()> {
        let village_id = job.village_id as u32;
        let player_id = job.player_id;

        info!("Executing TrainUnits job");
        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();

        // 1. Load village and army
        let mut village = village_repo.get_by_id(village_id).await?.unwrap();
        let mut army = village.army.map_or_else(
            || {
                Army::new(
                    None,
                    village_id,
                    Some(village_id),
                    village.player_id,
                    village.tribe.clone(),
                    [0; 10],
                    Default::default(),
                    None,
                )
            },
            |a| a.clone(),
        );

        // 2. Add 1 unit
        army.add_unit(self.payload.unit.clone(), 1)?;
        army_repo.save(&army).await?;

        // 3. Update village upkeep (village.update_state() dovrebbe farlo)
        village.army = Some(army);
        village.update_state();
        village_repo.save(&village).await?;

        // 4. Check if more units need to be trained
        if self.payload.quantity > 1 {
            // 5. Create the next job in the chain
            let next_payload = TrainUnitsTask {
                quantity: self.payload.quantity - 1, // Train one less
                ..self.payload.clone()
            };

            let next_job = Job::new(
                player_id,
                village_id as i32,
                self.payload.time_per_unit_secs as i64, // Schedule for one unit's time
                JobTask::TrainUnits(next_payload),
            );

            ctx.uow.jobs().add(&next_job).await?;
            info!(next_job_id = %next_job.id, "Queued next unit training job");
        }

        Ok(())
    }
}
