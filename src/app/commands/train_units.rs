use crate::{
    game::models::{buildings::BuildingName, ResourceGroup},
    jobs::{tasks::TrainUnitsTask, Job, JobPayload},
    repository::{JobRepository, VillageRepository},
};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TrainUnitsCommand {
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit_idx: u8,
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

        // 2. Check requirements
        if !village.academy_research[command.unit_idx as usize] {
            return Err(anyhow!("Unit not researched in Academy"));
        }

        let tribe_units = village.tribe.get_units();
        let unit_data = tribe_units
            .get(command.unit_idx as usize)
            .ok_or_else(|| anyhow!("Invalid unit index"))?;

        // 3. Check resources
        // TODO: Get cost from unit data
        let cost_per_unit = &unit_data.cost; // Usa `cost`, non `research_cost`
        let total_cost = ResourceGroup::new(
            cost_per_unit.resources.0 * command.quantity as u32,
            cost_per_unit.resources.1 * command.quantity as u32,
            cost_per_unit.resources.2 * command.quantity as u32,
            cost_per_unit.resources.3 * command.quantity as u32,
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
        let time_per_unit_secs = cost_per_unit.build_time;

        let building = village
            .get_building_by_name(BuildingName::Barracks)
            .ok_or_else(|| anyhow!("Required building not found: {:?}", BuildingName::Barracks))?;

        let payload = TrainUnitsTask {
            slot_id: building.slot_id,
            unit: unit_data.clone().name,
            quantity: command.quantity,
            time_per_unit_secs: time_per_unit_secs as i32,
        };

        let job_payload = JobPayload::new("TrainUnits", serde_json::to_value(&payload)?);
        // Schedule the *first* unit to be completed.
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            time_per_unit_secs as i64,
            job_payload,
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
        app::test_utils::tests::{MockJobRepository, MockVillageRepository},
        game::{
            models::{
                army::UnitName,
                buildings::Building,
                village::{Village, VillageBuilding},
                Player, Tribe,
            },
            test_factories::{
                player_factory, valley_factory, village_factory, PlayerFactoryOptions,
                VillageFactoryOptions,
            },
        },
    };

    use std::sync::Arc;

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

        village.academy_research[0] = true; // Research Legionnaire

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
            unit_idx: 0,
            quantity: 5,
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_ok(), "Handler should execute successfully");

        // Check if resources were deducted
        let saved_villages = mock_village_repo.saved_villages();
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
        let added_jobs = mock_job_repo.get_added_jobs();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.player_id, player.id);
        assert_eq!(job.village_id, village_id as i32);

        assert_eq!(
            job.task.task_type, "TrainUnits",
            "Job task is not TrainUnitsTask"
        );

        let task: TrainUnitsTask = serde_json::from_value(job.task.data.clone())
            .expect("Failed to deserialize job task data");

        assert_eq!(task.unit, UnitName::Legionnaire);
        assert_eq!(task.quantity, 5);
    }

    #[tokio::test]
    async fn test_train_units_handler_not_enough_resources() {
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
            unit_idx: 0,
            quantity: 1,
        };

        let result = handler.handle(command).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");
        assert_eq!(mock_job_repo.get_added_jobs().len(), 0);
    }

    #[tokio::test]
    async fn test_train_units_handler_missing_building() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (player, mut village) = setup_village_with_barracks();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Barracks);

        let _ = village.research_academy(UnitName::Praetorian);
        let village_id = village.id;
        mock_village_repo.add_village(village);

        let handler =
            TrainUnitsCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = TrainUnitsCommand {
            player_id: player.id,
            village_id: village_id,
            unit_idx: 1,
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
