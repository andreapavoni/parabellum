use std::sync::Arc;
use tracing::info;

use parabellum_types::{Result, errors::ApplicationError};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::ReleaseReinforcements},
    jobs::{Job, JobPayload, tasks::ArmyReturnTask},
    uow::UnitOfWork,
};

pub struct ReleaseReinforcementsCommandHandler {}

impl Default for ReleaseReinforcementsCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ReleaseReinforcementsCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<ReleaseReinforcements> for ReleaseReinforcementsCommandHandler {
    async fn handle(
        &self,
        command: ReleaseReinforcements,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let army_repo = uow.armies();
        let village_repo = uow.villages();
        let job_repo = uow.jobs();

        // Get the current village (where reinforcements are stationed)
        let current_village = village_repo.get_by_id(command.village_id).await?;

        // Verify ownership
        if current_village.player_id != command.player_id {
            return Err(ApplicationError::Unknown(
                "You don't own this village".to_string(),
            ));
        }

        // Find the reinforcement army from the source village
        let reinforcement = current_village
            .reinforcements()
            .iter()
            .find(|army| army.village_id == command.source_village_id)
            .ok_or_else(|| {
                ApplicationError::Unknown("No reinforcements from that village found".to_string())
            })?;

        let army_id = reinforcement.id;
        let reinforcement_player_id = reinforcement.player_id;

        // Get the source village (where the reinforcements will return)
        let source_village = village_repo.get_by_id(command.source_village_id).await?;

        // Remove the reinforcement from current location immediately
        // This removes it from the current village's reinforcements when the village is next loaded
        let mut departing_army = army_repo.get_by_id(army_id).await?;
        departing_army.current_map_field_id = None;
        army_repo.save(&departing_army).await?;

        // Calculate travel time back to source
        let travel_time_secs = current_village.position.calculate_travel_time_secs(
            source_village.position,
            departing_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        // Create return job
        let return_payload = ArmyReturnTask {
            army_id,
            resources: Default::default(),
            destination_village_id: source_village.id as i32,
            destination_player_id: reinforcement_player_id,
            from_village_id: current_village.id as i32,
        };

        let job_payload = JobPayload::new("ArmyReturn", serde_json::to_value(&return_payload)?);
        let new_job = Job::new(
            reinforcement_player_id,
            source_village.id as i32,
            travel_time_secs,
            job_payload,
        );

        job_repo.add(&new_job).await?;

        info!(
            release_job_id = %new_job.id,
            army_id = %army_id,
            from_village_id = current_village.id,
            to_village_id = source_village.id,
            arrival_at = %new_job.completed_at,
            "Release reinforcements job created - army leaving current location immediately."
        );

        Ok(())
    }
}
