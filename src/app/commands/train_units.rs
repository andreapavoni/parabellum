// In src/app/commands/train_units.rs

use crate::{
    game::models::{
        army::{Unit, UnitGroup},
        buildings::BuildingName,
        Cost, ResourceGroup,
    },
    jobs::{tasks::TrainUnitsTask, Job, JobTask},
    repository::{JobRepository, VillageRepository},
};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TrainUnitsCommand {
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit: Unit,
    pub quantity: i32,
}

pub struct TrainUnitsCommandHandler<'a> {
    village_repo: Arc<dyn VillageRepository + 'a>,
    job_repo: Arc<dyn JobRepository + 'a>,
}

impl<'a> TrainUnitsCommandHandler<'a> {
    pub fn new(
        village_repo: Arc<dyn VillageRepository + 'a>,
        job_repo: Arc<dyn JobRepository + 'a>,
    ) -> Self {
        Self {
            village_repo,
            job_repo,
        }
    }

    pub async fn handle(&self, command: TrainUnitsCommand) -> Result<()> {
        let mut village = self
            .village_repo
            .get_by_id(command.village_id)
            .await?
            .ok_or_else(|| anyhow!("Village not found"))?;

        // --- Validation ---

        // 1. Check ownership
        if village.player_id != command.player_id {
            return Err(anyhow!("Player does not own this village"));
        }

        // 2. Check building requirements
        // TODO: This logic should be more robust, maybe defined
        // alongside the Unit definitions in army.rs.
        let required_building = match command.unit.group {
            UnitGroup::Infantry => BuildingName::Barracks,
            UnitGroup::Cavalry => BuildingName::Stable,
            UnitGroup::Siege => BuildingName::Workshop,
            _ => return Err(anyhow!("Unit type not trainable yet")),
        };

        let building = village
            .get_building_by_name(required_building.clone())
            .ok_or_else(|| anyhow!("Required building not found: {:?}", required_building))?;

        if building.building.level == 0 {
            return Err(anyhow!("Required building level is 0"));
        }

        // 3. Check resources
        // TODO: Get cost from unit data
        let placeholder_cost = Cost {
            resources: ResourceGroup::new(120, 100, 150, 30), // Legionnaire cost
            upkeep: 1,
            build_time: 533, // TODO: Calculate this based on building level
        };

        let total_cost = ResourceGroup::new(
            placeholder_cost.resources.0 * command.quantity as u32,
            placeholder_cost.resources.1 * command.quantity as u32,
            placeholder_cost.resources.2 * command.quantity as u32,
            placeholder_cost.resources.3 * command.quantity as u32,
        );

        // This checks current stocks (which are updated on load)
        let stocks = village.stocks.stored_resources();
        if stocks.0 < total_cost.0
            || stocks.1 < total_cost.1
            || stocks.2 < total_cost.2
            || stocks.3 < total_cost.3
        {
            return Err(anyhow!("Not enough resources"));
        }

        // --- Logic & Side Effects ---

        // 1. Deduct resources
        village.stocks.remove_resources(&total_cost);
        self.village_repo.save(&village).await?;

        // 2. Create Job
        // For simplicity, we create one job for the *entire batch*.
        // A more complex (and correct) approach would be to create one job
        // per unit, or one job for the batch that, when complete,
        // queues the next job if the queue is not empty.

        // Let's use the 1-job-per-batch-which-queues-the-next logic.
        // The first job will be for the first unit.

        // TODO: Calculate real time per unit based on building level
        let time_per_unit_secs = placeholder_cost.build_time; // This is in seconds

        let payload = TrainUnitsTask {
            slot_id: building.slot_id, // We need the building slot
            unit: command.unit,
            quantity: command.quantity, // The *total* quantity to train
            time_per_unit_secs: time_per_unit_secs as i32,
        };

        // Schedule the *first* unit to be completed.
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            time_per_unit_secs as i64, // Duration for the first unit
            JobTask::TrainUnits(payload),
        );

        self.job_repo.add(&new_job).await?;

        Ok(())
    }
}

// 4. Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::{
            models::{
                army::Army,
                buildings::Building,
                map::Position,
                village::{Village, VillageBuilding},
                Tribe,
            },
            test_factories::{
                army_factory, player_factory, valley_factory, village_factory, ArmyFactoryOptions,
                PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions,
            },
        },
        jobs::JobStatus,
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
        saved_villages: Mutex<Vec<Village>>,
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
        async fn save(&self, village: &Village) -> Result<()> {
            self.saved_villages.lock().unwrap().push(village.clone());
            // Also update the in-memory village for subsequent gets
            self.villages
                .lock()
                .unwrap()
                .insert(village.id, village.clone());
            Ok(())
        }
    }

    fn setup_village_with_barracks() -> (Player, Village) {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let valley = valley_factory(Default::default());
        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        // Add barracks
        let barracks = VillageBuilding {
            slot_id: 20, // Example slot
            building: Building::new(BuildingName::Barracks),
        };
        village.buildings.push(barracks);

        // Add resources
        village.stocks.lumber = 1000;
        village.stocks.clay = 1000;
        village.stocks.iron = 1000;
        village.stocks.crop = 1000;
        village.update_state(); // Recalculate

        (player, village)
    }

    #[tokio::test]
    async fn test_train_units_handler_success() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, village) = setup_village_with_barracks();
        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit: UnitName::Legionnaire,
            quantity: 5,
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_ok(), "Handler should execute successfully");

        // Check if resources were deducted
        let saved_villages = mock_village_repo.saved_villages.lock().unwrap();
        assert_eq!(saved_villages.len(), 1, "Village should be saved once");
        let saved_village = &saved_villages[0];

        // Cost for 5 Legionnaires (5 * 120 = 600)
        assert_eq!(
            saved_village.stocks.lumber,
            1000 - (120 * 5),
            "Lumber not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.clay,
            1000 - (100 * 5),
            "Clay not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.iron,
            1000 - (150 * 5),
            "Iron not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.crop,
            1000 - (30 * 5) as i64,
            "Crop not deducted correctly"
        );

        // Check if job was created
        let added_jobs = mock_job_repo.added_jobs.lock().unwrap();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.player_id, player.id);
        assert_eq!(job.village_id, village_id as i32);

        if let JobTask::TrainUnits(task) = &job.task {
            assert_eq!(task.unit, UnitName::Legionnaire);
            assert_eq!(task.quantity, 5);
        } else {
            panic!("Job task is not TrainUnitsTask");
        }
    }

    #[tokio::test]
    async fn test_train_units_handler_not_enough_resources() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, mut village) = setup_village_with_barracks();
        village.stocks.lumber = 10; // Not enough lumber
        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit: UnitName::Legionnaire,
            quantity: 1, // Even for 1 (needs 120)
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");

        // Check that nothing was saved
        assert_eq!(mock_village_repo.saved_villages.lock().unwrap().len(), 0);
        assert_eq!(mock_job_repo.added_jobs.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_building() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, mut village) = setup_village_with_barracks();
        // Remove barracks
        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Barracks);

        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit: UnitName::Legionnaire,
            quantity: 1,
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Required building not found: Barracks"
        );
    }
}
