use parabellum_types::army::UnitRole;
use std::sync::Arc;
use tracing::info;

use parabellum_types::{
    Result,
    errors::{ApplicationError, GameError},
};

use crate::{
    command_handlers::helpers::deploy_army_from_village,
    config::Config,
    cqrs::{CommandHandler, commands::ScoutVillage},
    jobs::{Job, JobPayload, tasks::ScoutTask},
    uow::UnitOfWork,
};

pub struct ScoutVillageCommandHandler {}

impl Default for ScoutVillageCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ScoutVillageCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<ScoutVillage> for ScoutVillageCommandHandler {
    async fn handle(
        &self,
        command: ScoutVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let job_repo = uow.jobs();
        let village_repo = uow.villages();

        let defender_village = village_repo.get_by_id(command.target_village_id).await?;
        let attacker_village = village_repo.get_by_id(command.village_id).await?;

        let (attacker_village, deployed_army) = deploy_army_from_village(
            uow,
            attacker_village,
            command.army_id,
            command.units.clone(),
            None,
        )
        .await?;

        // Check only army is only scouts
        let tribe_units = attacker_village.tribe.units();
        for (idx, &quantity) in command.units.units().iter().enumerate() {
            if quantity > 0 {
                let unit = tribe_units
                    .get(idx)
                    .ok_or(GameError::InvalidUnitIndex(idx as u8))?;
                if !matches!(unit.role, UnitRole::Scout) {
                    return Err(ApplicationError::Game(GameError::OnlyScoutUnitsAllowed));
                }
            }
        }

        let travel_time_secs = attacker_village.position.calculate_travel_time_secs(
            defender_village.position,
            deployed_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        let scout_payload = ScoutTask {
            army_id: deployed_army.id,
            attacker_village_id: attacker_village.id as i32,
            attacker_player_id: command.player_id,
            target_village_id: command.target_village_id as i32,
            target_player_id: defender_village.player_id,
            target: command.target,
            attack_type: command.attack_type.clone(),
        };

        let job_payload = JobPayload::new("Scout", serde_json::to_value(&scout_payload)?);
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            travel_time_secs,
            job_payload,
        );
        job_repo.add(&new_job).await?;

        info!(
            scout_job_id = %new_job.id,
            arrival_at = %new_job.completed_at,
            "Scout job planned."
        );

        Ok(())
    }
}
