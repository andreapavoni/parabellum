use crate::{
    game::models::buildings::BuildingName,
    jobs::{tasks::AttackTask, Job, JobTask},
    repository::{JobRepository, VillageRepository},
};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AttackCommand {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub target_village_id: u32,
    pub catapult_targets: [BuildingName; 2],
}

pub struct AttackCommandHandler {
    job_repo: Arc<dyn JobRepository>,
    village_repo: Arc<dyn VillageRepository>,
}

impl AttackCommandHandler {
    pub fn new(job_repo: Arc<dyn JobRepository>, village_repo: Arc<dyn VillageRepository>) -> Self {
        Self {
            job_repo,
            village_repo,
        }
    }

    pub async fn handle(&self, command: AttackCommand) -> Result<()> {
        let attacker_village = self
            .village_repo
            .get_by_id(command.village_id)
            .await?
            .ok_or_else(|| anyhow!("Attacker village not found"))?;

        // TODO: validate attacker army (amount, owner, etc...)

        let defender_village = self
            .village_repo
            .get_by_id(command.target_village_id)
            .await?
            .ok_or_else(|| anyhow!("Defender village not found"))?;

        // TODO: fix army speed
        let speed = 10; // placeholder
        let travel_time_secs = attacker_village
            .calculate_travel_time_secs(defender_village.position.clone(), speed)
            as i64;

        let attack_payload = AttackTask {
            army_id: command.army_id,
            attacker_village_id: attacker_village.id as i32,
            attacker_player_id: command.player_id,
            target_village_id: command.target_village_id as i32,
            target_player_id: defender_village.player_id,
            catapult_targets: command.catapult_targets,
        };

        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            travel_time_secs,
            JobTask::Attack(attack_payload),
        );
        self.job_repo.add(&new_job).await?;

        println!(
            "attack job {} will be executed at {}.",
            new_job.id, new_job.completed_at
        );

        // TODO: update travelling army status
        // self.army_repo.set_status(command.army_id, ArmyStatus::Travelling).await?;

        Ok(())
    }
}
