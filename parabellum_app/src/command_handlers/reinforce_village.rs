use std::sync::Arc;
use tracing::info;

use parabellum_core::{ApplicationError, Result};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::ReinforceVillage},
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
        let army_repo = uow.armies();

        let attacker_village = village_repo.get_by_id(command.village_id).await?;
        let attacker_army = army_repo.get_by_id(command.army_id).await?;
        let target_village = village_repo.get_by_id(command.target_village_id).await?;

        let travel_time_secs = attacker_village.position.calculate_travel_time_secs(
            target_village.position,
            attacker_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        let reinforce_payload = ReinforcementTask {
            army_id: command.army_id,
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
