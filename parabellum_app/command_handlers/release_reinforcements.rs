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
        use parabellum_game::models::army::Army;

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

        // Validate requested units don't exceed available
        for (idx, &requested) in command.units.units().iter().enumerate() {
            if requested > reinforcement.units().get(idx) {
                return Err(ApplicationError::Unknown(format!(
                    "Cannot release {} units of type {} - only {} available",
                    requested,
                    idx,
                    reinforcement.units().get(idx)
                )));
            }
        }

        // Check if at least one unit is being released
        if command.units.immensity() == 0 {
            return Err(ApplicationError::Unknown(
                "Must release at least one unit".to_string(),
            ));
        }

        // Get the source village (where the reinforcements will return)
        let source_village = village_repo.get_by_id(command.source_village_id).await?;

        // Determine if this is a full or partial release
        let is_full_release = command
            .units
            .units()
            .iter()
            .enumerate()
            .all(|(idx, &qty)| qty == reinforcement.units().get(idx));

        let mut departing_army = army_repo.get_by_id(army_id).await?;

        let returning_army_id = if is_full_release {
            // Full release: move the entire reinforcement army
            departing_army.current_map_field_id = None;
            army_repo.save(&departing_army).await?;
            army_id
        } else {
            // Partial release: create new army for returning troops, update original
            let mut remaining_units = departing_army.units().clone();
            for (idx, &released) in command.units.units().iter().enumerate() {
                remaining_units.remove(idx, released);
            }

            // Create new army for the returning troops
            let returning_army = Army::new(
                None, // New ID
                departing_army.village_id,
                None, // In transit
                departing_army.player_id,
                departing_army.tribe.clone(),
                &command.units,
                departing_army.smithy(),
                None, // Heroes stay with remaining troops for now
            );
            army_repo.save(&returning_army).await?;

            // Update original army with remaining troops (still reinforcing)
            departing_army.update_units(&remaining_units);
            army_repo.save(&departing_army).await?;

            info!(
                "Partial release: {} units staying as reinforcements, {} units returning",
                remaining_units.immensity(),
                command.units.immensity()
            );

            returning_army.id
        };

        // Calculate travel time back to source
        // For partial release, calculate speed based on returning troops
        let speed = Army::new(
            None,
            departing_army.village_id,
            None,
            departing_army.player_id,
            departing_army.tribe.clone(),
            &command.units,
            departing_army.smithy(),
            None,
        )
        .speed();

        let travel_time_secs = current_village.position.calculate_travel_time_secs(
            source_village.position,
            speed,
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        // Create return job
        let return_payload = ArmyReturnTask {
            army_id: returning_army_id,
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
            army_id = %returning_army_id,
            from_village_id = current_village.id,
            to_village_id = source_village.id,
            arrival_at = %new_job.completed_at,
            is_full_release = is_full_release,
            "Release reinforcements job created - army leaving current location immediately."
        );

        Ok(())
    }
}
