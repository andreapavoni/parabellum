use parabellum_types::army::UnitRole;
use std::sync::Arc;
use tracing::info;

use parabellum_core::{ApplicationError, GameError, Result};

use crate::{
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
        // --- FIX 1: Controlla la somma, non il count ---
        if command.units.iter().sum::<u32>() == 0 {
            return Err(ApplicationError::Game(GameError::NotUnitsSelected));
        }

        let job_repo = uow.jobs();
        let village_repo = uow.villages();
        let army_repo = uow.armies();

        let mut attacker_village = village_repo.get_by_id(command.village_id).await?;
        let mut attacker_army = army_repo.get_by_id(command.army_id).await?;
        let defender_village = village_repo.get_by_id(command.target_village_id).await?;

        // --- VALIDAZIONE 2: Controlla che siano solo scout ---
        let tribe_units = attacker_village.tribe.get_units();
        for (idx, &quantity) in command.units.iter().enumerate() {
            if quantity > 0 {
                let unit = tribe_units
                    .get(idx)
                    .ok_or(GameError::InvalidUnitIndex(idx as u8))?;
                if !matches!(unit.role, UnitRole::Scout) {
                    return Err(ApplicationError::Game(GameError::OnlyScoutUnitsAllowed));
                }
            }
        }
        // --- FINE VALIDAZIONE ---

        let deployed_army = attacker_army.deploy(command.units)?;

        if attacker_army.immensity() == 0 {
            army_repo.remove(attacker_army.id).await?;
            attacker_village.army = None;
        } else {
            army_repo.save(&attacker_army).await?;
            attacker_village.army = Some(attacker_army);
        }

        attacker_village.update_state();
        village_repo.save(&attacker_village).await?;
        army_repo.save(&deployed_army).await?;

        let travel_time_secs = attacker_village.position.calculate_travel_time_secs(
            defender_village.position,
            deployed_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        let scout_payload = ScoutTask {
            army_id: deployed_army.id, // <-- ID della nuova armata
            attacker_village_id: attacker_village.id as i32,
            attacker_player_id: command.player_id,
            target_village_id: command.target_village_id as i32,
            target_player_id: defender_village.player_id,
            target: command.target,
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
