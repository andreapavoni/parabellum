use crate::{
    game::models::army::Army, // Assicurati di avere un modello Army
    jobs::{
        handler::{JobHandler, JobHandlerContext},
        tasks::ArmyReturnTask,
        Job,
    },
};
use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, instrument};

pub struct ArmyReturnJobHandler {
    payload: ArmyReturnTask,
}

impl ArmyReturnJobHandler {
    pub fn new(payload: ArmyReturnTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for ArmyReturnJobHandler {
    #[instrument(skip_all, fields(
        task_type = "ArmyReturn",
        army_id = ?job.task.data.get("army_id"),
        player_id = %job.player_id,
        village_id = job.village_id,
    ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        job: &'ctx Job,
    ) -> Result<()> {
        info!("Executing ArmyReturn job");

        let village_id = job.village_id as u32;
        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();

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

        let returning_army = army_repo
            .get_by_id(self.payload.army_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Returning army not found"))?;

        army.merge(&returning_army)?;
        army_repo.save(&army).await?;

        village.army = Some(army);
        village.update_state();
        village_repo.save(&village).await?;

        Ok(())
    }
}
