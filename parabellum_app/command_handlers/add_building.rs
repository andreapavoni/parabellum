use std::sync::Arc;

use parabellum_game::models::buildings::get_building_data;
use parabellum_types::{Result, buildings::BuildingName, errors::GameError, tribe::Tribe};

use crate::{
    command_handlers::helpers::{completion_time_for_slot, enforce_queue_capacity},
    config::Config,
    cqrs::{CommandHandler, commands::AddBuilding},
    jobs::{Job, JobPayload, tasks::AddBuildingTask},
    uow::UnitOfWork,
};

pub struct AddBuildingCommandHandler {}

impl Default for AddBuildingCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AddBuildingCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<AddBuilding> for AddBuildingCommandHandler {
    async fn handle(
        &self,
        cmd: AddBuilding,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<()> {
        let villages_repo = uow.villages();
        let job_repo = uow.jobs();

        let mut village = villages_repo.get_by_id(cmd.village_id).await?;
        let active_jobs = job_repo
            .list_active_jobs_by_village(cmd.village_id as i32)
            .await?;
        let building_jobs: Vec<Job> = active_jobs
            .into_iter()
            .filter(|job| {
                matches!(
                    job.task.task_type.as_str(),
                    "AddBuilding" | "BuildingUpgrade"
                )
            })
            .collect();
        let building_limit = if matches!(village.tribe, Tribe::Roman) {
            3
        } else {
            2
        };
        enforce_queue_capacity("building", &building_jobs, building_limit)?;
        ensure_queue_allows_building(&cmd.name, &building_jobs)?;

        let build_time_secs =
            village.init_building_construction(cmd.slot_id, cmd.name.clone(), config.speed)?;
        villages_repo.save(&village).await?;

        let payload = AddBuildingTask {
            village_id: village.id as i32,
            slot_id: cmd.slot_id,
            name: cmd.name,
        };
        let job_payload = JobPayload::new("AddBuilding", serde_json::to_value(&payload)?);
        let completion_time =
            completion_time_for_slot(&building_jobs, cmd.slot_id, build_time_secs as i64);
        let new_job = Job::with_deadline(
            cmd.player_id,
            cmd.village_id as i32,
            job_payload,
            completion_time,
        );
        job_repo.add(&new_job).await?;

        Ok(())
    }
}

fn ensure_queue_allows_building(candidate: &BuildingName, jobs: &[Job]) -> Result<(), GameError> {
    if jobs.is_empty() {
        return Ok(());
    }

    let candidate_data = get_building_data(candidate)?;

    for job in jobs {
        if job.task.task_type != "AddBuilding" {
            continue;
        }

        let Some(payload) = serde_json::from_value::<AddBuildingTask>(job.task.data.clone()).ok()
        else {
            continue;
        };

        let queued_name = payload.name;

        if queued_name == *candidate && !candidate_data.rules.allow_multiple {
            return Err(GameError::NoMultipleBuildingConstraint(candidate.clone()));
        }

        if candidate_data
            .rules
            .conflicts
            .iter()
            .any(|conflict| conflict.0 == queued_name)
        {
            return Err(GameError::BuildingConflict(candidate.clone(), queued_name));
        }

        if let Ok(queued_data) = get_building_data(&queued_name)
            && queued_data
                .rules
                .conflicts
                .iter()
                .any(|conflict| conflict.0 == *candidate)
            {
                return Err(GameError::BuildingConflict(candidate.clone(), queued_name));
            }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::setup_player_party,
    };
    use parabellum_types::{
        Result,
        errors::{AppError, ApplicationError, GameError},
    };
    use parabellum_types::{
        buildings::BuildingName,
        common::{Player, ResourceGroup},
        tribe::Tribe,
    };

    use super::*;
    use crate::{
        jobs::tasks::AddBuildingTask,
        test_utils::tests::{MockUnitOfWork, set_village_resources},
    };
    use std::sync::Arc;

    fn setup_village(config: &Config) -> Result<(Player, Village, Arc<Config>)> {
        let (player, mut village, _, _) =
            setup_player_party(None, Tribe::Roman, [0; 10], false).unwrap();

        // main building is level 1 in slot 19 by default
        village.set_building_level_at_slot(19, 3, config.speed)?;

        let rally_point =
            Building::new(BuildingName::RallyPoint, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(rally_point, 39)?;

        let config = Arc::new(Config::from_env());
        Ok((player, village, config))
    }

    #[tokio::test]
    async fn test_add_building_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Config::from_env();
        let (player, mut village, config) = setup_village(&config)?;
        let village_id = village.id;
        let player_id = player.id;

        set_village_resources(&mut village, ResourceGroup(800, 800, 800, 800));

        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 22,
            name: BuildingName::Barracks,
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Check if resources were deducted
        let saved_village = mock_uow.villages().get_by_id(village_id).await?;
        let cost = Building::new(BuildingName::Barracks, config.speed).cost();

        assert_eq!(
            saved_village.stored_resources().lumber(),
            800 - cost.resources.0,
            "Lumber not deducted correctly"
        );

        // Check if job was created
        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.task.task_type, "AddBuilding");
        let task: AddBuildingTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.slot_id, 22);
        assert_eq!(task.name, BuildingName::Barracks);

        // Check that building is NOT added to village yet
        assert!(
            saved_village.get_building_by_slot_id(22).is_none(),
            "Building should not be added by the command handler"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_not_enough_resources() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;
        set_village_resources(&mut village, ResourceGroup::default());

        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id: player.id,
            village_id: village.id,
            slot_id: 22,
            name: BuildingName::Barracks,
        };

        let uow_box = Box::new(mock_uow);
        let result = handler.handle(command, &uow_box, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::NotEnoughResources.to_string()
        );

        // Check that no job was created
        let added_jobs = uow_box.jobs().list_by_player_id(player.id).await?;
        assert_eq!(added_jobs.len(), 0, "No job should be created");
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_blocks_duplicate_queue() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;
        let village_id = village.id;
        let player_id = player.id;

        set_village_resources(&mut village, ResourceGroup(20000, 20000, 20000, 20000));
        mock_uow.villages().save(&village).await?;

        let queued_payload = AddBuildingTask {
            village_id: village_id as i32,
            slot_id: 22,
            name: BuildingName::Palace,
        };
        let queued_job_payload =
            JobPayload::new("AddBuilding", serde_json::to_value(&queued_payload)?);
        let queued_job = Job::new(player_id, village_id as i32, 0, queued_job_payload);
        mock_uow.jobs().add(&queued_job).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 23,
            name: BuildingName::Palace,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(matches!(
            result,
            Err(ApplicationError::Game(
                GameError::NoMultipleBuildingConstraint(_)
            ))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_blocks_conflicting_queue() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;
        let village_id = village.id;
        let player_id = player.id;

        set_village_resources(&mut village, ResourceGroup(20000, 20000, 20000, 20000));
        mock_uow.villages().save(&village).await?;

        let queued_payload = AddBuildingTask {
            village_id: village_id as i32,
            slot_id: 22,
            name: BuildingName::Palace,
        };
        let queued_job_payload =
            JobPayload::new("AddBuilding", serde_json::to_value(&queued_payload)?);
        let queued_job = Job::new(player_id, village_id as i32, 0, queued_job_payload);
        mock_uow.jobs().add(&queued_job).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 24,
            name: BuildingName::Residence,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::BuildingConflict(
                BuildingName::Residence,
                BuildingName::Palace
            )))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_slot_occupied() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;
        let village_id = village.id;
        let player_id = player.id;

        // Manually occupy slot 22
        let cranny = Building::new(BuildingName::Cranny, config.speed);
        village.add_building_at_slot(cranny, 22)?;
        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 22, // Try to build on same slot
            name: BuildingName::Barracks,
        };

        let uow_box = Box::new(mock_uow);
        let result = handler.handle(command, &uow_box, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::SlotOccupied { slot_id: 22 }.to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_requirements_not_met() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;

        // Remove Rally Point at slot 39 (requirement for Barracks)
        village.remove_building_at_slot(39, config.speed)?;

        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().save(&village).await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 22,
            name: BuildingName::Barracks,
        };

        let uow_box = Box::new(mock_uow);
        let result = handler.handle(command, &uow_box, &config).await;

        assert!(result.is_err(), "Handler should fail");
        assert_eq!(
            result.err().unwrap().to_string(),
            GameError::BuildingRequirementsNotMet {
                building: BuildingName::RallyPoint,
                level: 1,
            }
            .to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_queue_limit_non_roman() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village(&config)?;
        village.tribe = Tribe::Gaul;
        let village_id = village.id;
        let player_id = player.id;
        mock_uow.villages().save(&village).await?;

        let payload = AddBuildingTask {
            village_id: village_id as i32,
            slot_id: 22,
            name: BuildingName::Barracks,
        };
        let job_payload = JobPayload::new("AddBuilding", serde_json::to_value(&payload)?);
        mock_uow
            .jobs()
            .add(&Job::new(
                player_id,
                village_id as i32,
                60,
                job_payload.clone(),
            ))
            .await?;
        mock_uow
            .jobs()
            .add(&Job::new(player_id, village_id as i32, 60, job_payload))
            .await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 23,
            name: BuildingName::Stable,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            matches!(
                result,
                Err(ApplicationError::App(AppError::QueueLimitReached { queue }))
                if queue == "building"
            ),
            "Expected building queue limit error"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_add_building_handler_roman_allows_third_job() -> Result<()> {
        let config = Config::from_env();
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village(&config)?;
        let village_id = village.id;
        let player_id = player.id;
        mock_uow.villages().save(&village).await?;

        let payload = AddBuildingTask {
            village_id: village_id as i32,
            slot_id: 22,
            name: BuildingName::Barracks,
        };
        let job_payload = JobPayload::new("AddBuilding", serde_json::to_value(&payload)?);
        mock_uow
            .jobs()
            .add(&Job::new(
                player_id,
                village_id as i32,
                60,
                job_payload.clone(),
            ))
            .await?;
        mock_uow
            .jobs()
            .add(&Job::new(player_id, village_id as i32, 60, job_payload))
            .await?;

        let handler = AddBuildingCommandHandler::new();
        let command = AddBuilding {
            player_id,
            village_id,
            slot_id: 24,
            name: BuildingName::Cranny,
        };

        handler.handle(command, &mock_uow, &config).await?;
        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 3, "Romans should allow a third job");

        Ok(())
    }
}
