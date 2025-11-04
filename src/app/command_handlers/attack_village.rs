use std::sync::Arc;
use tracing::info;

use crate::{
    Result,
    config::Config,
    cqrs::{CommandHandler, commands::AttackVillage},
    error::ApplicationError,
    jobs::{Job, JobPayload, tasks::AttackTask},
    repository::{ArmyRepository, JobRepository, VillageRepository, uow::UnitOfWork},
};

pub struct AttackVillageCommandHandler {}

impl Default for AttackVillageCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AttackVillageCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<AttackVillage> for AttackVillageCommandHandler {
    async fn handle(
        &self,
        command: AttackVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let job_repo: Arc<dyn JobRepository + '_> = uow.jobs();
        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();
        let army_repo: Arc<dyn ArmyRepository + '_> = uow.armies();

        let attacker_village = village_repo.get_by_id(command.village_id).await?;

        let attacker_army = army_repo.get_by_id(command.army_id).await?;

        let defender_village = village_repo.get_by_id(command.target_village_id).await?;

        let travel_time_secs = attacker_village.position.calculate_travel_time_secs(
            defender_village.position,
            attacker_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

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
    use crate::app::test_utils::tests::{MockUnitOfWork, assert_handler_success};
    use crate::game::{
        models::{Tribe, buildings::BuildingName, map::Position},
        test_utils::{
            ArmyFactoryOptions, PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            army_factory, player_factory, valley_factory, village_factory,
        },
    };

    #[tokio::test]
    async fn test_attack_village_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let village_repo = mock_uow.villages();
        let army_repo = mock_uow.armies();
        let job_repo = mock_uow.jobs();
        let config = Arc::new(Config::from_env());

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

        village_repo.create(&attacker_village).await.unwrap();
        village_repo.create(&defender_village).await.unwrap();
        army_repo.create(&attacker_army).await.unwrap();

        let handler = AttackVillageCommandHandler::new();
        let command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert_handler_success(result);

        let added_jobs = job_repo
            .list_by_player_id(attacker_player.id)
            .await
            .unwrap();
        assert_eq!(added_jobs.len(), 1, "One job should be created");

        let job = &added_jobs[0];
        assert_eq!(job.player_id, attacker_player.id);
    }
}
