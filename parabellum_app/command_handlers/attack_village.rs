use std::sync::Arc;
use tracing::info;

use parabellum_core::ApplicationError;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::AttackVillage},
    helpers::army_helper::deploy_army_from_village,
    jobs::{Job, JobPayload, tasks::AttackTask},
    repository::{JobRepository, VillageRepository},
    uow::UnitOfWork,
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

        let attacker_village = village_repo.get_by_id(command.village_id).await?;
        let (attacker_village, deployed_army) = deploy_army_from_village(
            uow,
            attacker_village,
            command.army_id,
            command.units,
            command.hero_id,
        )
        .await?;

        // Fetch target village to calculate travel time
        let defender_village = village_repo.get_by_id(command.target_village_id).await?;
        let travel_time_secs = attacker_village.position.calculate_travel_time_secs(
            defender_village.position,
            deployed_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        // Create and enqueue an Attack job for the deployed army
        let attack_payload = AttackTask {
            army_id: deployed_army.id,
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_core::Result;
    use parabellum_game::test_utils::setup_player_party;
    use parabellum_types::{buildings::BuildingName, map::Position, tribe::Tribe};

    use super::*;
    use crate::test_utils::tests::MockUnitOfWork;

    #[tokio::test]
    async fn test_attack_village_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let village_repo = mock_uow.villages();
        let army_repo = mock_uow.armies();
        let job_repo = mock_uow.jobs();
        let config = Arc::new(Config::from_env());

        let (attacker_player, attacker_village, attacker_army, _) = setup_player_party(
            Some(Position { x: 0, y: 0 }),
            Tribe::Teuton,
            [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            false,
        )?;
        let (_, defender_village, _, _) = setup_player_party(
            Some(Position { x: 10, y: 10 }),
            Tribe::Roman,
            [0; 10],
            false,
        )?;

        village_repo.save(&attacker_village).await?;
        village_repo.save(&defender_village).await?;
        army_repo.save(&attacker_army).await?;

        let handler = AttackVillageCommandHandler::new();
        let command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: None,
        };

        handler.handle(command, &mock_uow, &config).await?;

        let added_jobs = job_repo.list_by_player_id(attacker_player.id).await?;
        assert_eq!(added_jobs.len(), 1, "One job should be created");

        let job = &added_jobs[0];
        assert_eq!(job.player_id, attacker_player.id);

        Ok(())
    }

    #[tokio::test]
    async fn test_attack_village_handler_with_hero_id() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let village_repo = mock_uow.villages();
        let army_repo = mock_uow.armies();
        let job_repo = mock_uow.jobs();
        let hero_repo = mock_uow.heroes();
        let config = Arc::new(Config::from_env());

        let (attacker_player, attacker_village, attacker_army, some_hero) = setup_player_party(
            Some(Position { x: 0, y: 0 }),
            Tribe::Teuton,
            [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            true,
        )?;
        let hero = some_hero.unwrap();

        let (_, defender_village, _, _) = setup_player_party(
            Some(Position { x: 10, y: 10 }),
            Tribe::Roman,
            [0; 10],
            false,
        )?;

        hero_repo.save(&hero).await?;
        village_repo.save(&attacker_village).await?;
        village_repo.save(&defender_village).await?;
        army_repo.save(&attacker_army).await?;

        let handler = AttackVillageCommandHandler::new();
        let command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: Some(attacker_army.hero().unwrap().id),
        };
        handler.handle(command, &mock_uow, &config).await?;

        let jobs = job_repo.list_by_player_id(attacker_player.id).await?;
        assert_eq!(jobs.len(), 1, "One job should be created");
        let job = &jobs[0];
        assert_eq!(job.task.task_type, "Attack");
        let attack_task: AttackTask = serde_json::from_value(job.task.data.clone())?;
        let deployed_army_id = attack_task.army_id;
        assert_ne!(
            deployed_army_id, attacker_army.id,
            "Deployed army should have a new ID"
        );

        let home_army_res = army_repo.get_by_id(attacker_army.id).await;
        assert!(
            home_army_res.is_err(),
            "Home army should be removed after hero and all troops depart, got {:#?}",
            home_army_res
        );
        let updated_attacker_village = village_repo.get_by_id(attacker_village.id).await?;
        assert!(
            updated_attacker_village.army().is_none(),
            "Attacker village should have no army after sending hero with all troops"
        );

        let deployed_army = army_repo.get_by_id(deployed_army_id).await?;
        assert!(
            deployed_army.hero().is_some(),
            "Deployed army should include the hero"
        );
        assert_eq!(
            deployed_army.hero().unwrap().id,
            attacker_army.hero().unwrap().id,
            "Hero ID should match the one sent with the army"
        );
        assert_eq!(
            deployed_army.hero().unwrap().player_id,
            attacker_player.id,
            "Hero should remain under the attacker's ownership"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_attack_village_handler_hero_not_present() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let village_repo = mock_uow.villages();
        let army_repo = mock_uow.armies();
        let job_repo = mock_uow.jobs();
        let config = Arc::new(Config::from_env());

        let (attacker_player, attacker_village, attacker_army, _) =
            setup_player_party(None, Tribe::Teuton, [10, 0, 0, 0, 0, 0, 0, 0, 0, 0], false)?;

        let (_, defender_village, _, _) =
            setup_player_party(Some(Position { x: 5, y: 5 }), Tribe::Roman, [0; 10], false)?;

        village_repo.save(&attacker_village).await?;
        village_repo.save(&defender_village).await?;
        army_repo.save(&attacker_army).await?;

        let handler = AttackVillageCommandHandler::new();
        let command = AttackVillage {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            units: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            hero_id: None,
        };
        handler.handle(command, &mock_uow, &config).await?;

        let jobs = job_repo.list_by_player_id(attacker_player.id).await?;
        assert_eq!(jobs.len(), 1, "One job should be created");
        let job = &jobs[0];
        assert_eq!(job.task.task_type, "Attack");
        let attack_task: AttackTask = serde_json::from_value(job.task.data.clone())?;
        let deployed_army_id = attack_task.army_id;
        assert_ne!(
            deployed_army_id, attacker_army.id,
            "Deployed army should have a new ID"
        );

        let home_army_res = army_repo.get_by_id(attacker_army.id).await;
        assert!(
            home_army_res.is_err(),
            "Home army should be removed after deploying all troops"
        );
        let updated_attacker_village = village_repo.get_by_id(attacker_village.id).await?;
        assert!(
            updated_attacker_village.army().is_none(),
            "Attacker village should have no army after attack"
        );

        let deployed_army = army_repo.get_by_id(deployed_army_id).await?;
        assert!(
            deployed_army.hero().is_none(),
            "Hero ID was not present in the village, so no hero should accompany the army"
        );

        Ok(())
    }
}
