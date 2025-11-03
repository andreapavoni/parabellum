use std::sync::Arc;

use crate::{
    config::Config,
    cqrs::{Command, CommandHandler},
    error::{ApplicationError, Result},
    game::{GameError, models::army::UnitName},
    jobs::{Job, JobPayload, tasks::ResearchAcademyTask},
    repository::uow::UnitOfWork,
};

#[derive(Debug, Clone)]
pub struct ResearchAcademy {
    pub unit: UnitName,
    pub village_id: u32,
}

impl Command for ResearchAcademy {}

pub struct ResearchAcademyHandler {}

impl ResearchAcademyHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<ResearchAcademy> for ResearchAcademyHandler {
    async fn handle(
        &self,
        command: ResearchAcademy,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let village_repo = uow.villages();

        let mut village = village_repo.get_by_id(command.village_id).await?;

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

        if !village.stocks.check_resources(&research_cost.resources) {
            return Err(ApplicationError::Game(GameError::NotEnoughResources));
        }
        village.stocks.remove_resources(&research_cost.resources);
        village_repo.save(&village).await?;

        let time_per_unit_secs = (research_cost.time as f64 / config.speed as f64).floor() as i64;

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

        let job_repo = uow.jobs();
        job_repo.add(&new_job).await?;

        Ok(())
    }
}

// 4. Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::MockUnitOfWork,
        game::{
            models::{
                Player, Tribe,
                army::UnitName,
                buildings::{Building, BuildingName},
                village::Village,
            },
            test_factories::{
                PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
                village_factory,
            },
        },
    };

    use std::sync::Arc;

    fn setup_village_with_buildings() -> Result<(Player, Village), ApplicationError> {
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

        let main_building = Building::new(BuildingName::MainBuilding).at_level(3)?;
        village.upgrade_building_at_slot(main_building, 19)?;

        let warehouse = Building::new(BuildingName::Warehouse).at_level(2)?;
        village.add_building_at_slot(warehouse, 20)?;

        let granary = Building::new(BuildingName::Granary).at_level(2)?;
        village.add_building_at_slot(granary, 25)?;

        let rally_point = Building::new(BuildingName::RallyPoint).at_level(3)?;
        village.add_building_at_slot(rally_point, 21)?;

        let barracks = Building::new(BuildingName::Barracks).at_level(3)?;
        village.add_building_at_slot(barracks, 22)?;

        let academy = Building::new(BuildingName::Academy).at_level(3)?;
        village.add_building_at_slot(academy, 23)?;

        let smithy = Building::new(BuildingName::Smithy).at_level(3)?;
        village.add_building_at_slot(smithy, 24)?;

        village.academy_research[0] = true; // Research Legionnaire

        // Add resources
        village.stocks.lumber = 2000;
        village.stocks.clay = 2000;
        village.stocks.iron = 2000;
        village.stocks.crop = 2000;
        village.update_state();

        Ok((player, village))
    }

    #[tokio::test]
    async fn test_research_academy_handler_success() {
        let mock_uow = MockUnitOfWork::new();
        let mock_village_repo = mock_uow.villages();
        let mock_job_repo = mock_uow.jobs();
        let config = Arc::new(Config::from_env());

        let (player, mut village) = setup_village_with_buildings().unwrap();
        let village_id = village.id;

        // Add resources
        village.stocks.lumber = 2000;
        village.stocks.clay = 2000;
        village.stocks.iron = 2000;
        village.stocks.crop = 2000;
        village.update_state();

        mock_village_repo.create(&village).await.unwrap();

        let handler = ResearchAcademyHandler::new();

        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id: village_id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow);

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(
            result.is_ok(),
            "Handler should execute successfully, got: {:?}",
            result.err().unwrap().to_string()
        );

        // Check if resources were deducted
        let saved_village = mock_village_repo.get_by_id(village.id).await.unwrap();

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
        let added_jobs = mock_job_repo.list_by_player_id(player.id).await.unwrap();
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
        let mock_uow = MockUnitOfWork::new();
        let mock_village_repo = mock_uow.villages();
        let mock_job_repo = mock_uow.jobs();
        let config = Arc::new(Config::from_env());

        let (player, mut village) = setup_village_with_buildings().unwrap();
        village.stocks.lumber = 10; // Not enough lumber
        let village_id = village.id;

        // Add resources
        village.stocks.lumber = 200;
        village.stocks.clay = 200;
        village.stocks.iron = 200;
        village.stocks.crop = 200;
        village.update_state();

        mock_village_repo.create(&village).await.unwrap();

        let handler = ResearchAcademyHandler::new();

        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id: village_id,
        };
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow);
        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(result.err().unwrap().to_string(), "Not enough resources");
        assert_eq!(
            mock_job_repo
                .list_by_player_id(player.id)
                .await
                .unwrap()
                .len(),
            0
        );
    }

    #[tokio::test]
    async fn test_research_academy_handler_missing_building() {
        let mock_uow = MockUnitOfWork::new();
        let mock_village_repo = mock_uow.villages();
        let config = Arc::new(Config::from_env());

        let (_player, mut village) = setup_village_with_buildings().unwrap();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Academy);

        let village_id = village.id;

        mock_village_repo.create(&village.clone()).await.unwrap();

        let handler = ResearchAcademyHandler::new();

        let command = ResearchAcademy {
            village_id: village_id,
            unit: UnitName::Praetorian,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow);
        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should return an error");
        assert_eq!(
            result.err().unwrap().to_string(),
            "Building requirements not met: requires Academy at level 1"
        );
    }
}
