use std::sync::Arc;
use tracing::info;

use parabellum_types::{Result, errors::ApplicationError};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::RecallTroops},
    jobs::{
        Job, JobPayload,
        tasks::{ArmyReturnTask, ReinforcementTask},
    },
    uow::UnitOfWork,
};

pub struct RecallTroopsCommandHandler {}

impl Default for RecallTroopsCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl RecallTroopsCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<RecallTroops> for RecallTroopsCommandHandler {
    async fn handle(
        &self,
        command: RecallTroops,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let army_repo = uow.armies();
        let village_repo = uow.villages();
        let job_repo = uow.jobs();

        // Get the deployed army
        let mut army = army_repo.get_by_id(command.army_id).await?;

        // Verify ownership
        if army.player_id != command.player_id {
            return Err(ApplicationError::Unknown(
                "You don't own this army".to_string(),
            ));
        }

        // Verify the army is deployed (not at home)
        let current_location_id = army
            .current_map_field_id
            .ok_or_else(|| ApplicationError::Unknown("Army is not deployed".to_string()))?;

        // Get source and destination villages
        let home_village = village_repo.get_by_id(command.village_id).await?;
        let current_village = village_repo.get_by_id(current_location_id).await?;

        // Army leaves the current location immediately - set current_map_field_id to None
        // This removes it from the current village's deployed_armies when the village is next loaded
        army.current_map_field_id = None;
        army_repo.save(&army).await?;

        // Calculate travel time back home
        let travel_time_secs = current_village.position.calculate_travel_time_secs(
            home_village.position,
            army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        // Create return job
        let return_payload = ArmyReturnTask {
            army_id: army.id,
            resources: Default::default(), // No resources looted when recalling
            destination_village_id: home_village.id as i32,
            destination_player_id: command.player_id,
            from_village_id: current_location_id as i32,
        };

        let job_payload = JobPayload::new("ArmyReturn", serde_json::to_value(&return_payload)?);
        let new_job = Job::new(
            command.player_id,
            home_village.id as i32,
            travel_time_secs,
            job_payload,
        );

        job_repo.add(&new_job).await?;

        info!(
            recall_job_id = %new_job.id,
            army_id = %army.id,
            from_location = current_location_id,
            arrival_at = %new_job.completed_at,
            "Recall job created - army leaving current location immediately."
        );

        Ok(())
    }
}
