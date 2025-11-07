use parabellum_core::{ApplicationError, GameError};
use parabellum_game::models::smithy::smithy_upgrade_cost_for_unit;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::ResearchSmithy},
    jobs::{Job, JobPayload, tasks::ResearchSmithyTask},
    uow::UnitOfWork,
};

use std::sync::Arc;

pub struct ResearchSmithyCommandHandler {}

impl ResearchSmithyCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<ResearchSmithy> for ResearchSmithyCommandHandler {
    async fn handle(
        &self,
        command: ResearchSmithy,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let village_repo = uow.villages();
        let job_repo = uow.jobs();
        let mut village = village_repo.get_by_id(command.village_id).await?;

        let unit_idx = village.tribe.get_unit_idx_by_name(&command.unit).unwrap();
        let tribe_units = village.tribe.get_units();
        let current_level = village.smithy()[unit_idx];

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

        if !village.academy_research()[unit_idx] && unit.research_cost.time > 0 {
            return Err(ApplicationError::Game(GameError::UnitNotResearched(
                command.unit,
            )));
        }

        let research_cost = smithy_upgrade_cost_for_unit(&command.unit, current_level)?;
        village.deduct_resources(&research_cost.resources)?;
        village_repo.save(&village).await?;

        let research_time = research_cost.time;
        let payload = ResearchSmithyTask { unit: command.unit };
        let job_payload = JobPayload::new("ResearchSmithy", serde_json::to_value(&payload)?);
        let new_job = Job::new(
            village.player_id,
            command.village_id as i32,
            research_time as i64,
            job_payload,
        );
        job_repo.add(&new_job).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::{
            PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
        },
    };
    use parabellum_types::{
        army::UnitName,
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    use super::*;
    use crate::{
        config::Config,
        jobs::tasks::ResearchSmithyTask,
        test_utils::tests::{MockUnitOfWork, assert_handler_success},
        uow::UnitOfWork,
    };
    use std::sync::Arc;

    // Setup helper che crea un villaggio con i requisiti per uppare Praetorian
    fn setup_village_for_smithy() -> (Player, Village, Arc<Config>) {
        let config = Arc::new(Config::from_env());
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        let academy = Building::new(BuildingName::Academy, config.speed)
            .at_level(1, config.speed)
            .unwrap();
        village.add_building_at_slot(academy, 23).unwrap();

        let smithy = Building::new(BuildingName::Smithy, config.speed)
            .at_level(1, config.speed)
            .unwrap();
        village.add_building_at_slot(smithy, 24).unwrap();

        let warehouse = Building::new(BuildingName::Warehouse, config.speed)
            .at_level(4, config.speed)
            .unwrap();
        village.add_building_at_slot(warehouse, 25).unwrap();

        let granary = Building::new(BuildingName::Granary, config.speed)
            .at_level(4, config.speed)
            .unwrap();
        village.add_building_at_slot(granary, 26).unwrap();
        village.update_state();

        village.research_academy(UnitName::Praetorian).unwrap();

        village.store_resources(ResourceGroup(2000, 2000, 2000, 2000));
        village.update_state();

        (player, village, config)
    }

    #[tokio::test]
    async fn test_smithy_handler_success() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village_for_smithy();
        let village_id = village.id;
        let player_id = player.id;

        let village_repo = mock_uow.villages();
        let job_repo = mock_uow.jobs();
        village_repo.save(&village).await.unwrap();

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert_handler_success(result);

        let saved_village = mock_uow.villages().get_by_id(village_id).await.unwrap();
        let cost = smithy_upgrade_cost_for_unit(&command.unit, 0).unwrap();

        // Lvl 1 Praetorian: 800, 1010, 1320, 650
        assert_eq!(
            saved_village.get_stored_resources().lumber(),
            2000 - cost.resources.0,
            "Lumber not deducted"
        );
        assert_eq!(
            saved_village.get_stored_resources().clay(),
            2000 - cost.resources.1,
            "Clay not deducted"
        );
        assert_eq!(
            saved_village.get_stored_resources().iron(),
            2000 - cost.resources.2,
            "Iron not deducted"
        );
        assert_eq!(
            saved_village.get_stored_resources().crop(),
            (2000 - cost.resources.3),
            "Crop not deducted"
        );

        let added_jobs = job_repo.list_by_player_id(player_id).await.unwrap();
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.task.task_type, "ResearchSmithy");
        let task: ResearchSmithyTask = serde_json::from_value(job.task.data.clone()).unwrap();
        assert_eq!(task.unit, UnitName::Praetorian);
    }

    #[tokio::test]
    async fn test_smithy_handler_unit_not_researched() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_smithy();
        let village_repo = mock_uow.villages();

        village.set_academy_research_for_test(&UnitName::Praetorian, false);

        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::UnitNotResearched(UnitName::Praetorian).to_string()
        );
    }

    #[tokio::test]
    async fn test_smithy_handler_requirements_not_met() {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_smithy();

        village
            .buildings
            .retain(|vb| vb.building.name != BuildingName::Smithy);

        let village_repo = mock_uow.villages();

        let village_id = village.id;
        village_repo.save(&village).await.unwrap();

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
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
}
