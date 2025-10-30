use crate::{
    game::models::buildings::BuildingName,
    jobs::{tasks::AttackTask, Job, JobTask},
    repository::{ArmyRepository, JobRepository, VillageRepository},
};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AttackCommand {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub target_village_id: u32,
    pub catapult_targets: [BuildingName; 2],
}

pub struct AttackCommandHandler<'a> {
    job_repo: Arc<dyn JobRepository + 'a>,
    village_repo: Arc<dyn VillageRepository + 'a>,
    army_repo: Arc<dyn ArmyRepository + 'a>,
}

impl<'a> AttackCommandHandler<'a> {
    pub fn new(
        job_repo: Arc<dyn JobRepository + 'a>,
        village_repo: Arc<dyn VillageRepository + 'a>,
        army_repo: Arc<dyn ArmyRepository + 'a>,
    ) -> Self {
        Self {
            job_repo,
            village_repo,
            army_repo,
        }
    }

    pub async fn handle(&self, command: AttackCommand) -> Result<()> {
        let attacker_village = self
            .village_repo
            .get_by_id(command.village_id)
            .await?
            .ok_or_else(|| anyhow!("Attacker village not found"))?;

        let attacker_army = self
            .army_repo
            .get_by_id(command.army_id)
            .await?
            .ok_or_else(|| anyhow!("Attacker army not found"))?;

        let defender_village = self
            .village_repo
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
        self.job_repo.add(&new_job).await?;

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
    use crate::{
        game::{
            models::{army::Army, map::Position, village::Village, Tribe},
            test_factories::{
                army_factory, player_factory, valley_factory, village_factory, ArmyFactoryOptions,
                PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            },
        },
        repository::{ArmyRepository, JobRepository, VillageRepository},
    };
    use async_trait::async_trait;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    // --- Mocks ---

    #[derive(Default)]
    struct MockJobRepository {
        added_jobs: Mutex<Vec<Job>>,
    }

    #[async_trait]
    impl JobRepository for MockJobRepository {
        async fn add(&self, job: &Job) -> Result<()> {
            self.added_jobs.lock().unwrap().push(job.clone());
            Ok(())
        }
        // ... altri metodi non usati in questo test
        async fn get_by_id(&self, _id: Uuid) -> Result<Option<Job>> {
            Ok(None)
        }
        async fn list_by_player_id(&self, _id: Uuid) -> Result<Vec<Job>> {
            Ok(vec![])
        }
        async fn find_and_lock_due_jobs(&self, _limit: i64) -> Result<Vec<Job>> {
            Ok(vec![])
        }
        async fn mark_as_completed(&self, _job_id: Uuid) -> Result<()> {
            Ok(())
        }
        async fn mark_as_failed(&self, _job_id: Uuid, _error_message: &str) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockVillageRepository {
        villages: Mutex<HashMap<u32, Village>>,
    }

    impl MockVillageRepository {
        fn add_village(&self, village: Village) {
            self.villages.lock().unwrap().insert(village.id, village);
        }
    }

    #[async_trait]
    impl VillageRepository for MockVillageRepository {
        async fn create(&self, _village: &Village) -> Result<()> {
            Ok(())
        }
        async fn get_by_id(&self, village_id: u32) -> Result<Option<Village>> {
            let villages = self.villages.lock().unwrap();
            Ok(villages.get(&village_id).cloned())
        }
        async fn list_by_player_id(&self, _player_id: Uuid) -> Result<Vec<Village>> {
            Ok(vec![])
        }
        async fn save(&self, _village: &Village) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct MockArmyRepository {
        armies: Mutex<HashMap<Uuid, Army>>,
    }

    impl MockArmyRepository {
        fn add_army(&self, army: Army) {
            self.armies.lock().unwrap().insert(army.id, army);
        }
    }

    #[async_trait]
    impl ArmyRepository for MockArmyRepository {
        async fn create(&self, _army: &Army) -> Result<()> {
            Ok(())
        }
        async fn get_by_id(&self, army_id: Uuid) -> Result<Option<Army>> {
            let armies = self.armies.lock().unwrap();
            Ok(armies.get(&army_id).cloned())
        }
        async fn save(&self, _army: &Army) -> Result<()> {
            Ok(())
        }
        async fn remove(&self, _army_id: Uuid) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_attack_command_handler_success() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());
        let mock_army_repo = Arc::new(MockArmyRepository::default());

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

        // Teuton Maceman (speed 14)
        let attacker_army = army_factory(ArmyFactoryOptions {
            player_id: Some(attacker_player.id),
            village_id: Some(attacker_village.id),
            tribe: Some(Tribe::Teuton),
            units: Some([10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Default::default()
        });

        // Add entities to mocks
        mock_village_repo.add_village(attacker_village.clone());
        mock_village_repo.add_village(defender_village.clone());
        mock_army_repo.add_army(attacker_army.clone());

        let handler = AttackCommandHandler::new(
            mock_job_repo.clone(),
            mock_village_repo.clone(),
            mock_army_repo.clone(),
        );

        let command = AttackCommand {
            player_id: attacker_player.id,
            village_id: attacker_village.id,
            army_id: attacker_army.id,
            target_village_id: defender_village.id,
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_ok(), "Handler should execute successfully");

        let added_jobs = mock_job_repo.added_jobs.lock().unwrap();
        assert_eq!(added_jobs.len(), 1, "One job should be created");

        let job = &added_jobs[0];
        assert_eq!(job.player_id, attacker_player.id);
        assert_eq!(job.village_id, attacker_village.id as i32);

        if let JobTask::Attack(attack_payload) = &job.task {
            assert_eq!(attack_payload.army_id, attacker_army.id);
            assert_eq!(attack_payload.attacker_player_id, attacker_player.id);
            assert_eq!(attack_payload.target_village_id, defender_village.id as i32);
            assert_eq!(attack_payload.target_player_id, defender_player.id);
        } else {
            panic!("Job task is not an AttackTask");
        }

        // Check travel time. Pos (0,0) to (10,10) -> distance 14. Speed 14.
        // Time = (14 / 14) * 3600 = 3600 seconds (1 hour)
        let expected_travel_time = 3600;
        let duration = (job.completed_at - job.created_at).num_seconds();
        assert_eq!(
            duration, expected_travel_time,
            "Travel time calculation is incorrect"
        );
    }
}
