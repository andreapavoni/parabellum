use std::sync::Arc;

use crate::{
    db::DbError,
    error::Result,
    game::{GameError, models::army::UnitName},
    jobs::{Job, JobPayload, tasks::ResearchAcademyTask},
    repository::{JobRepository, VillageRepository},
};

#[derive(Debug, Clone)]
pub struct ResearchAcademyCommand {
    pub unit: UnitName,
    pub village_id: u32,
}

pub struct ResearchAcademyCommandHandler<'a> {
    village_repo: Arc<dyn VillageRepository + 'a>,
    job_repo: Arc<dyn JobRepository + 'a>,
}

impl<'a> ResearchAcademyCommandHandler<'a> {
    pub fn new(
        village_repo: Arc<dyn VillageRepository + 'a>,
        job_repo: Arc<dyn JobRepository + 'a>,
    ) -> Self {
        Self {
            village_repo,
            job_repo,
        }
    }

    pub async fn handle(&self, command: ResearchAcademyCommand) -> Result<()> {
        let mut village = self
            .village_repo
            .get_by_id(command.village_id)
            .await?
            .ok_or_else(|| DbError::VillageNotFound(command.village_id))?;

        let unit_idx = village.tribe.get_unit_idx_by_name(&command.unit).unwrap();

        // Check requirements
        if village.academy_research[unit_idx as usize] {
            return Err(GameError::UnitAlreadyResearched(command.unit).into());
        }

        let tribe_units = village.tribe.get_units();
        let unit_data = tribe_units
            .get(unit_idx as usize)
            .ok_or_else(|| GameError::InvalidUnitIndex(unit_idx as u8))?;

        for req in unit_data.requirements.iter() {
            match village
                .buildings
                .iter()
                .find(|&vb| vb.building.name == req.building && vb.building.level >= req.level)
            {
                Some(_) => (),
                None => {
                    return Err(GameError::BuildingRequirementsNotMet {
                        building: req.building.clone(),
                        level: req.level,
                    }
                    .into());
                }
            }
        }

        let research_cost = &unit_data.research_cost;

        village
            .stocks
            .withdraw_resources(&research_cost.resources)?;
        self.village_repo.save(&village).await?;

        // 2. Create Job
        let time_per_unit_secs = research_cost.time;

        let payload = ResearchAcademyTask {
            unit: unit_data.clone().name,
        };

        let job_payload = JobPayload::new("ResearchAcademy", serde_json::to_value(&payload)?);
        // Schedule the *first* unit to be completed.
        let new_job = Job::new(
            village.player_id,
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
            models::{Player, Tribe, army::UnitName, buildings::BuildingName, village::Village},
            test_factories::{
                PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
                village_factory,
            },
        },
    };

    use std::sync::Arc;

    fn setup_village_with_buildings() -> (Player, Village) {
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
        village.stocks.lumber = 1000;
        village.stocks.clay = 1000;
        village.stocks.iron = 1000;
        village.stocks.crop = 1000;
        village.update_state();

        let _ = village.upgrade_building(19);
        let _ = village.upgrade_building(19);
        let _ = village.upgrade_building(19);

        let _ = village
            .get_building_by_name(BuildingName::MainBuilding)
            .unwrap();

        let _ = village.add_building(BuildingName::Warehouse, 20).unwrap();
        let _ = village.upgrade_building(20);

        let _ = village.add_building(BuildingName::Granary, 25).unwrap();
        let _ = village.upgrade_building(25);

        let _ = village.add_building(BuildingName::RallyPoint, 21).unwrap();
        let _ = village.upgrade_building(21);
        let _ = village.upgrade_building(21);

        let _ = village.add_building(BuildingName::Barracks, 22).unwrap();
        let _ = village.upgrade_building(22);
        let _ = village.upgrade_building(22);

        let _ = village.add_building(BuildingName::Academy, 23).unwrap();
        let _ = village.upgrade_building(23);
        let _ = village.upgrade_building(23);

        let _ = village.add_building(BuildingName::Smithy, 24).unwrap();
        let _ = village.upgrade_building(24);
        let _ = village.upgrade_building(24);

        village.academy_research[0] = true; // Research Legionnaire

        // Add resources
        village.stocks.lumber = 2000;
        village.stocks.clay = 2000;
        village.stocks.iron = 2000;
        village.stocks.crop = 2000;
        village.update_state();

        (player, village)
    }

    #[tokio::test]
    async fn test_research_academy_handler_success() {
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());
        let (player, mut village) = setup_village_with_buildings();
        let village_id = village.id;

        // Add resources
        village.stocks.lumber = 2000;
        village.stocks.clay = 2000;
        village.stocks.iron = 2000;
        village.stocks.crop = 2000;
        village.update_state();

        mock_village_repo.add_village(village);

        let handler =
            ResearchAcademyCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = ResearchAcademyCommand {
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
            2000 - 700,
            "Lumber not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.clay,
            2000 - 620,
            "Clay not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.iron,
            2000 - 1480,
            "Iron not deducted correctly"
        );
        assert_eq!(
            saved_village.stocks.crop,
            2000 - 580 as i64,
            "Crop not deducted correctly"
        );

        // Check if job was created
        let added_jobs = mock_job_repo.get_added_jobs();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.player_id, player.id);
        assert_eq!(job.village_id, village_id as i32);

        assert_eq!(
            job.task.task_type, "ResearchAcademy",
            "Job task is not ResearchAcademyTask"
        );

        let task: ResearchAcademyTask = serde_json::from_value(job.task.data.clone())
            .expect("Failed to deserialize job task data");

        assert_eq!(task.unit, UnitName::Praetorian);
    }

    #[tokio::test]
    async fn test_research_academy_handler_not_enough_resources() {
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (_player, mut village) = setup_village_with_buildings();
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
            ResearchAcademyCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = ResearchAcademyCommand {
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
        // 1. Setup
        let mock_job_repo = Arc::new(MockJobRepository::default());
        let mock_village_repo = Arc::new(MockVillageRepository::default());

        let (_player, mut village) = setup_village_with_buildings();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Academy);

        let village_id = village.id;

        mock_village_repo.add_village(village.clone());

        let handler =
            ResearchAcademyCommandHandler::new(mock_village_repo.clone(), mock_job_repo.clone());

        let command = ResearchAcademyCommand {
            village_id: village_id,
            unit: UnitName::Praetorian,
        };

        // 2. Act
        let result = handler.handle(command).await;

        // 3. Assert
        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Building requirements not met: requires Academy at level 1"
        );
    }
}
