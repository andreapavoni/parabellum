use crate::{
    Result,
    error::ApplicationError,
    game::{
        GameError,
        models::{army::UnitName, smithy::smithy_upgrade_cost_for_unit},
    },
    jobs::{Job, JobPayload, tasks::ResearchSmithyTask},
    repository::{JobRepository, VillageRepository},
};

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ResearchSmithyCommand {
    pub unit: UnitName,
    pub village_id: u32,
}

pub struct ResearchSmithyCommandHandler<'a> {
    village_repo: Arc<dyn VillageRepository + 'a>,
    job_repo: Arc<dyn JobRepository + 'a>,
}

impl<'a> ResearchSmithyCommandHandler<'a> {
    pub fn new(
        village_repo: Arc<dyn VillageRepository + 'a>,
        job_repo: Arc<dyn JobRepository + 'a>,
    ) -> Self {
        Self {
            village_repo,
            job_repo,
        }
    }

    pub async fn handle(&self, command: ResearchSmithyCommand) -> Result<(), ApplicationError> {
        let mut village = self.village_repo.get_by_id(command.village_id).await?;

        let unit_idx = village.tribe.get_unit_idx_by_name(&command.unit).unwrap();
        let tribe_units = village.tribe.get_units();
        let current_level = village.smithy[unit_idx];

        let unit = tribe_units
            .get(unit_idx as usize)
            .ok_or_else(|| ApplicationError::Game(GameError::InvalidUnitIndex(unit_idx as u8)))?;

        for req in unit.get_requirements() {
            if !village
                .buildings
                .iter()
                .any(|b| b.building.name == req.building)
            {
                return Err(ApplicationError::Game(
                    GameError::BuildingRequirementsNotMet {
                        building: req.building.clone(),
                        level: req.level,
                    },
                ));
            }
        }

        if village.academy_research[unit_idx] {
            return Err(ApplicationError::Game(GameError::UnitAlreadyResearched(
                command.unit,
            )));
        }

        // 3. Check resources
        let research_cost = smithy_upgrade_cost_for_unit(&command.unit, current_level)?;

        let _ = village
            .stocks
            .withdraw_resources(&research_cost.resources)?;
        self.village_repo.save(&village).await?;

        // 2. Create Job
        let research_time = research_cost.time;
        let payload = ResearchSmithyTask { unit: command.unit };
        let job_payload = JobPayload::new("ResearchSmithy", serde_json::to_value(&payload)?);
        let new_job = Job::new(
            village.player_id,
            command.village_id as i32,
            research_time as i64,
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
                Player, ResourceGroup, Tribe, army::UnitName, buildings::BuildingName,
                village::Village,
            },
            test_factories::{
                PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
                village_factory,
            },
        },
    };

    use std::sync::Arc;

    fn setup_village_with_buildings() -> Result<(Player, Village)> {
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

        // Add resources
        village
            .stocks
            .store_resources(ResourceGroup(1000, 1000, 1000, 1000));
        village.update_state();

        village.upgrade_building(19)?;
        village.upgrade_building(19)?;
        village.upgrade_building(19)?;

        let _ = village
            .get_building_by_name(BuildingName::MainBuilding)
            .unwrap();

        village.add_building(BuildingName::Warehouse, 20)?;
        village.upgrade_building(20)?;

        village.add_building(BuildingName::Granary, 25)?;
        village.upgrade_building(25)?;

        village.add_building(BuildingName::RallyPoint, 21)?;
        village.upgrade_building(21)?;
        village.upgrade_building(21)?;

        village.add_building(BuildingName::Barracks, 22)?;
        village.upgrade_building(22)?;
        village.upgrade_building(22)?;

        village.add_building(BuildingName::Academy, 23)?;
        village.upgrade_building(23)?;
        village.upgrade_building(23)?;

        village.add_building(BuildingName::Smithy, 24)?;
        village.upgrade_building(24)?;
        village.upgrade_building(24)?;

        village.academy_research[0] = true; // Research Legionnaire

        // Add resources
        village
            .stocks
            .store_resources(ResourceGroup(2000, 2000, 2000, 2000));
        village.update_state();

        Ok((player, village))
    }

    #[tokio::test]
    async fn test_research_academy_handler_success() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());
        let (player, mut village) = setup_village_with_buildings().unwrap();
        let village_id = village.id;

        // Add resources
        village.stocks.lumber = 2000;
        village.stocks.clay = 2000;
        village.stocks.iron = 2000;
        village.stocks.crop = 2000;
        village.update_state();

        mock_village_repo.add_village(village);

        let handler =
            ResearchSmithyCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = ResearchSmithyCommand {
            unit: UnitName::Praetorian,
            village_id: village_id,
        };

        let result = handler.handle(command).await;

        assert!(
            result.is_ok(),
            "Command should execute successfully: {:#?}",
            result.err().unwrap()
        );

        // Check if resources were deducted
        let saved_villages = mock_village_repo.saved_villages();
        assert_eq!(saved_villages.len(), 1, "Village should be saved once");
        let saved_village = &saved_villages[0];

        assert_eq!(
            saved_village.stocks.lumber,
            2000 - 800,
            "Lumber not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.clay,
            2000 - 1010,
            "Clay not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.iron,
            2000 - 1320,
            "Iron not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.crop,
            2000 - 650 as i64,
            "Crop not deducted correctly"
        );

        // Check if job was created
        let added_jobs = mock_job_repo.get_added_jobs();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.player_id, player.id);
        assert_eq!(job.village_id, village_id as i32);

        assert_eq!(
            job.task.task_type, "ResearchSmithy",
            "Job task is not ResearchSmithyTask"
        );

        let task: ResearchSmithyTask = serde_json::from_value(job.task.data.clone())
            .expect("Failed to deserialize job task data");

        assert_eq!(task.unit, UnitName::Praetorian);
    }

    #[tokio::test]
    async fn test_research_academy_handler_not_enough_resources() {
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (_player, mut village) = setup_village_with_buildings().unwrap();
        village.stocks.lumber = 10; // Not enough lumber
        let village_id = village.id;

        // Add resources
        village.stocks.lumber = 200;
        village.stocks.clay = 200;
        village.stocks.iron = 200;
        village.stocks.crop = 200;
        village.update_state();

        mock_village_repo.add_village(village);

        let handler =
            ResearchSmithyCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = ResearchSmithyCommand {
            unit: UnitName::Praetorian,
            village_id: village_id,
        };

        let result = handler.handle(command).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");
        assert_eq!(mock_job_repo.get_added_jobs().len(), 0);
    }

    #[tokio::test]
    async fn test_research_academy_handler_missing_building() {
        let job_repo = Arc::new(MockJobRepository::default());
        let village_repo = Arc::new(MockVillageRepository::default());

        let (_player, mut village) = setup_village_with_buildings().unwrap();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Academy);
        village_repo.add_village(village.clone());

        let handler = ResearchSmithyCommandHandler::new(village_repo.clone(), job_repo.clone());
        let command = ResearchSmithyCommand {
            village_id: village.id,
            unit: UnitName::Praetorian,
        };

        let result = handler.handle(command).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Building requirements not met: requires Academy at level 1"
        );
    }
}
