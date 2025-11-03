use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{
    Result,
    cqrs::{Command, CommandHandler},
    db::DbError,
    game::models::buildings::BuildingName,
    jobs::{Job, JobPayload, tasks::AttackTask},
    repository::{ArmyRepository, JobRepository, VillageRepository, uow::UnitOfWork},
};

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

impl Default for AttackVillageHandler {
    fn default() -> Self {
        Self::new()
    }
}

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
            .ok_or_else(|| DbError::VillageNotFound(command.village_id))?;

        let attacker_army = army_repo
            .get_by_id(command.army_id)
            .await?
            .ok_or_else(|| DbError::ArmyNotFound(command.army_id))?;

        let defender_village = village_repo
            .get_by_id(command.target_village_id)
            .await?
            .ok_or_else(|| DbError::VillageNotFound(command.target_village_id))?;

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

        let job_payload = JobPayload::new("Attack", serde_json::to_value(&attack_payload)?);
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            travel_time_secs,
            job_payload,
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
mod tests {
    use super::*;
    use crate::app::test_utils::tests::MockUnitOfWork;
    use crate::game::{
        models::{Tribe, buildings::BuildingName, map::Position},
        test_factories::{
            ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            army_factory, player_factory, valley_factory, village_factory,
        },
    };

    #[tokio::test]
    async fn test_attack_village_handler_success() {
        // 1. Setup
        let mock_uow = MockUnitOfWork::new();

        let mock_village_repo = mock_uow.mock_villages();
        let mock_army_repo = mock_uow.mock_armies();
        let mock_job_repo = mock_uow.mock_jobs();

        let attacker_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        let defender_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let attacker_valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 0, y: 0 }),
            ..Default::default()
        });
        let attacker_village = village_factory(VillageFactoryOptions {
            player: Some(attacker_player.clone()),
            valley: Some(attacker_valley),
            ..Default::default()
        });

        let defender_valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 10, y: 10 }),
            ..Default::default()
        });
        let defender_village = village_factory(VillageFactoryOptions {
            player: Some(defender_player.clone()),
            valley: Some(defender_valley),
            ..Default::default()
        });

        let attacker_army = army_factory(ArmyFactoryOptions {
            player_id: Some(attacker_player.id),
            village_id: Some(attacker_village.id),
            tribe: Some(Tribe::Teuton),
            units: Some([10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Default::default()
        });

        mock_village_repo.add_village(attacker_village.clone());
        mock_village_repo.add_village(defender_village.clone());
        mock_army_repo.add_army(attacker_army.clone());

        let handler = AttackVillageHandler::new();

        let command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
        };

        // 2. Act
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow);
        let result = handler.handle(command, &mock_uow).await;

        // 3. Assert
        assert!(result.is_ok(), "Handler should execute successfully");

        // Check if job was created *nel mock repo*
        let added_jobs = mock_job_repo.get_added_jobs();
        assert_eq!(added_jobs.len(), 1, "One job should be created");

        let job = &added_jobs[0];
        assert_eq!(job.player_id, attacker_player.id);

        // ... more job asserts ...
    }
}
