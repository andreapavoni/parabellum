use std::sync::Arc;

use parabellum_types::{Result, errors::AppError};

use crate::{
    command_handlers::helpers::{completion_time_for_queue, enforce_queue_capacity},
    config::Config,
    cqrs::{CommandHandler, commands::ResearchAcademy},
    jobs::{Job, JobPayload, tasks::ResearchAcademyTask},
    queries_handlers::queue_converters::academy_queue_item_from_job,
    uow::UnitOfWork,
};

pub struct ResearchAcademyCommandHandler {}

impl Default for ResearchAcademyCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

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
        let job_repo = uow.jobs();

        let mut village = village_repo.get_by_id(command.village_id).await?;
        let active_jobs = job_repo
            .list_active_jobs_by_village(command.village_id as i32)
            .await?;
        let academy_jobs: Vec<Job> = active_jobs
            .into_iter()
            .filter(|job| job.task.task_type == "ResearchAcademy")
            .collect();
        enforce_queue_capacity("academy", &academy_jobs, 2)?;

        if academy_jobs
            .iter()
            .filter_map(|job| academy_queue_item_from_job(job))
            .any(|item| item.unit == command.unit)
        {
            return Err(AppError::QueueItemAlreadyQueued {
                queue: "academy",
                item: format!("{:?}", command.unit),
            }
            .into());
        }

        let research_time_secs = village.init_academy_research(&command.unit, config.speed)? as i64;
        village_repo.save(&village).await?;

        let payload = ResearchAcademyTask { unit: command.unit };
        let job_payload = JobPayload::new("ResearchAcademy", serde_json::to_value(&payload)?);
        let completion_time = completion_time_for_queue(&academy_jobs, research_time_secs);
        let new_job = Job::with_deadline(
            village.player_id,
            command.village_id as i32,
            job_payload,
            completion_time,
        );

        job_repo.add(&new_job).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use parabellum_game::{
        models::{buildings::Building, village::Village},
        test_utils::{
            PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
        },
    };
    use parabellum_types::{
        Result,
        errors::{AppError, ApplicationError, GameError},
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
        jobs::{JobPayload, tasks::ResearchAcademyTask},
        test_utils::tests::{MockUnitOfWork, set_village_resources},
    };
    use std::sync::Arc;

    // Setup helper che crea un villaggio con i requisiti per ricercare Praetorian
    fn setup_village_for_academy() -> Result<(Player, Village, Arc<Config>)> {
        let config = Config::from_env();
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            ..Default::default()
        });

        let academy =
            Building::new(BuildingName::Academy, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(academy, 23)?;

        let smithy = Building::new(BuildingName::Smithy, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(smithy, 24)?;

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(4, config.speed)?;
        village.add_building_at_slot(warehouse, 25)?;

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(4, config.speed)?;
        village.add_building_at_slot(granary, 26)?;

        let config = Arc::new(Config::from_env());
        Ok((player, village, config))
    }

    #[tokio::test]
    async fn test_research_academy_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village_for_academy()?;
        let village_id = village.id;
        let player_id = player.id;
        set_village_resources(&mut village, ResourceGroup(2000, 2000, 2000, 2000));

        mock_uow.villages().save(&village).await?;

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };

        handler.handle(command, &mock_uow, &config).await?;

        let saved_village = mock_uow.villages().get_by_id(village_id).await?;
        // Praetorian research cost: 700, 620, 1480, 580
        assert_eq!(
            saved_village.stored_resources().lumber(),
            2000 - 700,
            "Lumber not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().clay(),
            2000 - 620,
            "Clay not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().iron(),
            2000 - 1480,
            "Iron not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().crop(),
            2000 - 580,
            "Crop not deducted"
        );

        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.task.task_type, "ResearchAcademy");
        let task: ResearchAcademyTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.unit, UnitName::Praetorian);
        Ok(())
    }

    #[tokio::test]
    async fn test_research_academy_handler_not_enough_resources() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village_for_academy()?;
        set_village_resources(&mut village, ResourceGroup::default()); // No resources
        let village_id = village.id;
        let player_id = player.id;

        mock_uow.villages().save(&village).await?;

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

        let added_jobs = mock_uow.jobs().list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 0, "No job should be created");
        Ok(())
    }

    #[tokio::test]
    async fn test_research_academy_handler_requirements_not_met() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_academy()?;
        village.remove_building_at_slot(24, config.speed)?;
        mock_uow.villages().save(&village).await?;

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id: village.id,
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
        Ok(())
    }

    #[tokio::test]
    async fn test_research_academy_handler_already_researched() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_academy()?;

        village.set_academy_research_for_test(&UnitName::Praetorian, true);

        let village_id = village.id;
        mock_uow.villages().save(&village).await?;

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
        Ok(())
    }

    #[tokio::test]
    async fn test_research_academy_handler_queue_limit() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village_for_academy()?;
        set_village_resources(&mut village, ResourceGroup(2000, 2000, 2000, 2000));
        let village_id = village.id;
        mock_uow.villages().save(&village).await?;

        let job_repo = mock_uow.jobs();
        let payload = ResearchAcademyTask {
            unit: UnitName::Praetorian,
        };
        let job_payload = JobPayload::new("ResearchAcademy", serde_json::to_value(&payload)?);
        job_repo
            .add(&Job::new(
                player.id,
                village_id as i32,
                60,
                job_payload.clone(),
            ))
            .await?;
        job_repo
            .add(&Job::new(player.id, village_id as i32, 60, job_payload))
            .await?;

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            matches!(
                result,
                Err(ApplicationError::App(AppError::QueueLimitReached { queue }))
                if queue == "academy"
            ),
            "Expected academy queue limit error"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_research_academy_handler_blocks_duplicate_job() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_academy()?;
        set_village_resources(&mut village, ResourceGroup(2000, 2000, 2000, 2000));
        let village_id = village.id;
        mock_uow.villages().save(&village).await?;

        let payload = ResearchAcademyTask {
            unit: UnitName::Praetorian,
        };
        let job_payload = JobPayload::new("ResearchAcademy", serde_json::to_value(&payload)?);
        mock_uow
            .jobs()
            .add(&Job::new(
                village.player_id,
                village_id as i32,
                60,
                job_payload,
            ))
            .await?;

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            matches!(
                result,
                Err(ApplicationError::App(AppError::QueueItemAlreadyQueued { queue, .. }))
                if queue == "academy"
            ),
            "Expected duplicate queue error"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_research_academy_handler_respects_queue_order() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, mut village, config) = setup_village_for_academy()?;
        set_village_resources(&mut village, ResourceGroup(2000, 2000, 2000, 2000));
        let village_id = village.id;
        mock_uow.villages().save(&village).await?;

        let job_repo = mock_uow.jobs();
        let existing_payload = ResearchAcademyTask {
            unit: UnitName::EquitesLegati,
        };
        let existing_deadline = Utc::now() + Duration::seconds(90);
        job_repo
            .add(&Job::with_deadline(
                player.id,
                village_id as i32,
                JobPayload::new("ResearchAcademy", serde_json::to_value(&existing_payload)?),
                existing_deadline,
            ))
            .await?;

        let handler = ResearchAcademyCommandHandler::new();
        let command = ResearchAcademy {
            unit: UnitName::Praetorian,
            village_id,
        };
        handler.handle(command, &mock_uow, &config).await?;

        let jobs = job_repo.list_by_player_id(player.id).await?;
        let new_job = jobs
            .iter()
            .find(|job| {
                job.task.task_type == "ResearchAcademy"
                    && serde_json::from_value::<ResearchAcademyTask>(job.task.data.clone())
                        .map(|task| task.unit == UnitName::Praetorian)
                        .unwrap_or(false)
            })
            .expect("new job");

        let unit = village
            .tribe
            .units()
            .iter()
            .find(|u| u.name == UnitName::Praetorian)
            .unwrap();
        let expected_duration = ((unit.research_cost.time as f64 / config.speed as f64)
            .floor()
            .max(1.0)) as i64;

        assert_eq!(
            new_job.completed_at,
            existing_deadline + Duration::seconds(expected_duration),
            "Queued research should finish after existing jobs"
        );

        Ok(())
    }
}
