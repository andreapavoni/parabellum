use crate::{
    cqrs::{Command, CommandHandler},
    game::models::buildings::BuildingName,
    jobs::{tasks::AttackTask, Job, JobTask},
    repository::{uow::UnitOfWork, ArmyRepository, JobRepository, VillageRepository},
};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AttackVillage {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub target_village_id: u32,
    pub catapult_targets: [BuildingName; 2],
}

impl Command for AttackVillage {}

pub struct AttackVillageHandler {}

impl AttackVillageHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<AttackVillage> for AttackVillageHandler {
    async fn handle(
        &self,
        command: AttackVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
    ) -> Result<()> {
        let job_repo: Arc<dyn JobRepository + '_> = uow.jobs();
        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
        let army_repo: Arc<dyn ArmyRepository + '_> = uow.armies();

        let attacker_village = village_repo
            .get_by_id(command.village_id)
            .await?
            .ok_or_else(|| anyhow!("Attacker village not found"))?;

        let attacker_army = army_repo
            .get_by_id(command.army_id)
            .await?
            .ok_or_else(|| anyhow!("Attacker army not found"))?;

        let defender_village = village_repo
            .get_by_id(command.target_village_id)
            .await?
            .ok_or_else(|| anyhow!("Defender village not found"))?;

        let travel_time_secs = attacker_village
            .position
            .calculate_travel_time_secs(defender_village.position, attacker_army.speed())
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
        job_repo.add(&new_job).await?;

        info!(
            attack_job_id = %new_job.id,
            arrival_at = %new_job.completed_at,
            "Attack job planned."
        );

        // TODO: update travelling army status
        // self.army_repo.set_status(command.army_id, ArmyStatus::Travelling).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
