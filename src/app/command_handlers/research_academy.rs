use std::sync::Arc;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::ResearchAcademy},
    error::{ApplicationError, Result},
    game::GameError,
    jobs::{Job, JobPayload, tasks::ResearchAcademyTask},
    repository::uow::UnitOfWork,
};

pub struct ResearchAcademyCommandHandler {}

impl ResearchAcademyCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<ResearchAcademy> for ResearchAcademyCommandHandler {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::test_utils::tests::MockUnitOfWork,
        config::Config,
        game::{
            models::{
                Player, Tribe,
                army::UnitName,
                buildings::{Building, BuildingName},
                common::ResourceGroup,
                village::Village,
            },
            test_factories::{
                PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
            },
        },
        jobs::tasks::ResearchAcademyTask,
    };
    use std::sync::Arc;

    // Setup helper che crea un villaggio con i requisiti per ricercare Praetorian
    fn setup_village_for_academy() -> (Player, Village, Arc<Config>) {
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        let academy = Building::new(BuildingName::Academy).at_level(1).unwrap();
        village.add_building_at_slot(academy, 23).unwrap();

        let smithy = Building::new(BuildingName::Smithy).at_level(1).unwrap();
        village.add_building_at_slot(smithy, 24).unwrap();

        let warehouse = Building::new(BuildingName::Warehouse).at_level(4).unwrap();
        village.add_building_at_slot(warehouse, 25).unwrap();

        let granary = Building::new(BuildingName::Granary).at_level(4).unwrap();
        village.add_building_at_slot(granary, 26).unwrap();
        village.update_state();

        village
            .stocks
            .store_resources(ResourceGroup(2000, 2000, 2000, 2000));
        village.update_state();

        let config = Arc::new(Config::from_env());
        (player, village, config)
    }

    #[tokio::test]
    async fn test_research_academy_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village_for_academy();
        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().create(&village).await.unwrap();

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().unwrap().to_string()
        );

        let saved_village = mock_uow.villages().get_by_id(village_id).await.unwrap();
        // Praetorian research cost: 700, 620, 1480, 580
        assert_eq!(
            saved_village.stocks.lumber,
            2000 - 700,
            "Lumber not deducted"
        );
        assert_eq!(saved_village.stocks.clay, 2000 - 620, "Clay not deducted");
        assert_eq!(saved_village.stocks.iron, 2000 - 1480, "Iron not deducted");
        assert_eq!(
            saved_village.stocks.crop,
            (2000 - 580) as i64,
            "Crop not deducted"
        );

        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await.unwrap();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.task.task_type, "ResearchAcademy");
        let task: ResearchAcademyTask = serde_json::from_value(job.task.data.clone()).unwrap();
        assert_eq!(task.unit, UnitName::Praetorian);
    }

    #[tokio::test]
    async fn test_research_academy_handler_not_enough_resources() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village_for_academy();
        village.stocks = Default::default(); // No resources
        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().create(&village).await.unwrap();

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::NotEnoughResources.to_string()
        );

        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await.unwrap();
        assert_eq!(added_jobs.len(), 0, "No job should be created");
    }

    #[tokio::test]
    async fn test_research_academy_handler_requirements_not_met() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_academy();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Smithy);

        let village_id = village.id;
        mock_uow.villages().create(&village).await.unwrap();

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::BuildingRequirementsNotMet {
                building: BuildingName::Smithy,
                level: 1,
            }
            .to_string()
        );
    }

    #[tokio::test]
    async fn test_research_academy_handler_already_researched() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_academy();

        village.academy_research[1] = true; // Praetorian

        let village_id = village.id;
        mock_uow.villages().create(&village).await.unwrap();

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::UnitAlreadyResearched(UnitName::Praetorian).to_string()
        );
    }
}
