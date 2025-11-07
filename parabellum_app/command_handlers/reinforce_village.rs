use std::sync::Arc;
use tracing::info;

use parabellum_core::{ApplicationError, Result};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::ReinforceVillage},
    helpers::army_helper::deploy_army_from_village,
    jobs::{Job, JobPayload, tasks::ReinforcementTask},
    uow::UnitOfWork,
};

pub struct ReinforceVillageCommandHandler {}

impl Default for ReinforceVillageCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ReinforceVillageCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<ReinforceVillage> for ReinforceVillageCommandHandler {
    async fn handle(
        &self,
        command: ReinforceVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let job_repo = uow.jobs();
        let village_repo = uow.villages();

        let reinforcer_village = village_repo.get_by_id(command.village_id).await?;
        let target_village = village_repo.get_by_id(command.target_village_id).await?;
        let (reinforcer_village, deployed_army) =
            deploy_army_from_village(uow, reinforcer_village, command.army_id, command.units)
                .await?;

        let travel_time_secs = reinforcer_village.position.calculate_travel_time_secs(
            target_village.position,
            deployed_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        let reinforce_payload = ReinforcementTask {
            army_id: deployed_army.id,
            village_id: command.target_village_id as i32,
            player_id: command.player_id,
        };

        let job_payload =
            JobPayload::new("Reinforcement", serde_json::to_value(&reinforce_payload)?);
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            travel_time_secs,
            job_payload,
        );
        job_repo.add(&new_job).await?;

        info!(
            reinforce_job_id = %new_job.id,
            arrival_at = %new_job.completed_at,
            "Reinforcement job planned."
        );

        // TODO: Update army status into "moving"

        Ok(())
    }
}
