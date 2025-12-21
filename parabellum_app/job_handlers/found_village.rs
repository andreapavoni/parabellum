use async_trait::async_trait;
use tracing::{info, instrument};

use parabellum_game::models::{map::Valley, village::Village};
use parabellum_types::errors::ApplicationError;

use crate::jobs::{
    Job,
    handler::{JobHandler, JobHandlerContext},
    tasks::FoundVillageTask,
};

pub struct FoundVillageJobHandler {
    payload: FoundVillageTask,
}

impl FoundVillageJobHandler {
    pub fn new(payload: FoundVillageTask) -> Self {
        Self { payload }
    }
}

#[async_trait]
impl JobHandler for FoundVillageJobHandler {
    #[instrument(skip_all, fields(
          task_type = "FoundVillage",
          settler_player_id = %self.payload.settler_player_id,
          origin_village_id = %self.payload.origin_village_id,
          target_position = ?self.payload.target_position
      ))]
    async fn handle<'ctx, 'a>(
        &'ctx self,
        ctx: &'ctx JobHandlerContext<'a>,
        _job: &'ctx Job,
    ) -> Result<(), ApplicationError> {
        info!("Execute FoundVillage Job");

        let army_repo = ctx.uow.armies();
        let village_repo = ctx.uow.villages();
        let map_repo = ctx.uow.map();
        let player_repo = ctx.uow.players();

        // Get the settler army
        let settler_army = army_repo.get_by_id(self.payload.army_id).await?;

        // Get the player who is founding the village
        let player = player_repo
            .get_by_id(self.payload.settler_player_id)
            .await?;

        // Get the map field at target position
        let target_village_id = self
            .payload
            .target_position
            .to_id(ctx.config.world_size as i32);
        let map_field = map_repo.get_field_by_id(target_village_id as i32).await?;
        let valley = Valley::try_from(map_field)?;

        // Create the new village
        let mut new_village = Village::new(
            "New Village".to_string(),
            &valley,
            &player,
            false, // Not capital
            ctx.config.world_size as i32,
            ctx.config.speed,
        );

        // Set parent village (the village from which settlers were sent)
        new_village.parent_village_id = Some(self.payload.origin_village_id);

        // Save the new village
        village_repo.save(&new_village).await?;

        // Remove the settler army (they've settled)
        army_repo.remove(settler_army.id).await?;

        info!(
            new_village_id = %new_village.id,
            new_village_position = ?new_village.position,
            "Village founded successfully"
        );

        Ok(())
    }
}
