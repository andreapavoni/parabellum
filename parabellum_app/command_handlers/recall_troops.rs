use std::sync::Arc;
use tracing::info;

use parabellum_types::{Result, errors::ApplicationError};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::RecallTroops},
    jobs::{Job, JobPayload, tasks::ArmyReturnTask},
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
        use parabellum_game::models::army::Army;

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

        // Validate requested units don't exceed available
        for (idx, &requested) in command.units.units().iter().enumerate() {
            if requested > army.units().get(idx) {
                return Err(ApplicationError::Unknown(format!(
                    "Cannot recall {} units of type {} - only {} available",
                    requested,
                    idx,
                    army.units().get(idx)
                )));
            }
        }

        // Check if at least one unit is being recalled
        if command.units.immensity() == 0 {
            return Err(ApplicationError::Unknown(
                "Must recall at least one unit".to_string(),
            ));
        }

        // Get source and destination villages
        let home_village = village_repo.get_by_id(command.village_id).await?;
        let current_village = village_repo.get_by_id(current_location_id).await?;

        // Determine if this is a full or partial recall
        let is_full_recall = command
            .units
            .units()
            .iter()
            .enumerate()
            .all(|(idx, &qty)| qty == army.units().get(idx));

        let returning_army_id = if is_full_recall {
            // Full recall: move the entire army
            army.current_map_field_id = None;
            army_repo.save(&army).await?;
            army.id
        } else {
            // Partial recall: create new army for returning troops, update original
            let mut remaining_units = army.units().clone();
            for (idx, &recalled) in command.units.units().iter().enumerate() {
                remaining_units.remove(idx, recalled);
            }

            // Create new army for the returning troops
            let returning_army = Army::new(
                None, // New ID
                army.village_id,
                None, // In transit
                army.player_id,
                army.tribe.clone(),
                &command.units,
                army.smithy(),
                None, // Heroes stay with remaining troops for now
            );
            army_repo.save(&returning_army).await?;

            // Update original army with remaining troops
            army.update_units(&remaining_units);
            army_repo.save(&army).await?;

            info!(
                "Partial recall: {} units staying, {} units returning",
                remaining_units.immensity(),
                command.units.immensity()
            );

            returning_army.id
        };

        // Calculate travel time back home
        // For partial recall, calculate speed based on returning troops
        let speed = Army::new(
            None,
            army.village_id,
            None,
            army.player_id,
            army.tribe.clone(),
            &command.units,
            army.smithy(),
            None,
        )
        .speed();

        let travel_time_secs = current_village.position.calculate_travel_time_secs(
            home_village.position,
            speed,
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        // Create return job
        let return_payload = ArmyReturnTask {
            army_id: returning_army_id,
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
            army_id = %returning_army_id,
            from_location = current_location_id,
            arrival_at = %new_job.completed_at,
            is_full_recall = is_full_recall,
            "Recall job created - army leaving current location immediately."
        );

        Ok(())
    }
}
