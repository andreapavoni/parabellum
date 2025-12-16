use parabellum_types::errors::{AppError, ApplicationError};

use crate::{
    command_handlers::helpers::{completion_time_for_queue, enforce_queue_capacity},
    config::Config,
    cqrs::{CommandHandler, commands::ResearchSmithy},
    jobs::{Job, JobPayload, tasks::ResearchSmithyTask},
    queries_handlers::queue_converters::smithy_queue_item_from_job,
    uow::UnitOfWork,
};

use std::sync::Arc;

pub struct ResearchSmithyCommandHandler {}

impl Default for ResearchSmithyCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

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
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let village_repo = uow.villages();
        let job_repo = uow.jobs();
        let mut village = village_repo.get_by_id(command.village_id).await?;

        let active_jobs = job_repo
            .list_active_jobs_by_village(command.village_id as i32)
            .await?;
        let smithy_jobs: Vec<Job> = active_jobs
            .into_iter()
            .filter(|job| job.task.task_type == "ResearchSmithy")
            .collect();
        enforce_queue_capacity("smithy", &smithy_jobs, 2)?;

        if smithy_jobs
            .iter()
            .filter_map(smithy_queue_item_from_job)
            .any(|item| item.unit == command.unit)
        {
            return Err(AppError::QueueItemAlreadyQueued {
                queue: "smithy",
                item: format!("{:?}", command.unit),
            }
            .into());
        }

        let research_time = village.init_smithy_research(&command.unit, config.speed)? as i64;
        village_repo.save(&village).await?;

        let payload = ResearchSmithyTask { unit: command.unit };
        let job_payload = JobPayload::new("ResearchSmithy", serde_json::to_value(&payload)?);
        let completion_time = completion_time_for_queue(&smithy_jobs, research_time);
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
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use parabellum_game::{
        models::{buildings::Building, smithy::smithy_upgrade_cost_for_unit, village::Village},
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
        jobs::{JobPayload, tasks::ResearchSmithyTask},
        test_utils::tests::{MockUnitOfWork, set_village_resources},
        uow::UnitOfWork,
    };

    // Setup helper che crea un villaggio con i requisiti per uppare Praetorian
    fn setup_village_for_smithy() -> Result<(Player, Village, Arc<Config>)> {
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
        village.add_building_at_slot(academy, 23)?;

        let smithy = Building::new(BuildingName::Smithy, config.speed).at_level(1, config.speed)?;
        village.add_building_at_slot(smithy, 24)?;

        let warehouse =
            Building::new(BuildingName::Warehouse, config.speed).at_level(4, config.speed)?;
        village.add_building_at_slot(warehouse, 25)?;

        let granary =
            Building::new(BuildingName::Granary, config.speed).at_level(4, config.speed)?;
        village.add_building_at_slot(granary, 26)?;
        village.research_academy(UnitName::Praetorian)?;
        set_village_resources(&mut village, ResourceGroup(2000, 2000, 2000, 2000));
        Ok((player, village, config))
    }

    #[tokio::test]
    async fn test_smithy_handler_success() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village_for_smithy()?;
        let village_id = village.id;
        let player_id = player.id;

        let village_repo = mock_uow.villages();
        let job_repo = mock_uow.jobs();
        village_repo.save(&village).await?;

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
            unit: UnitName::Praetorian,
            village_id,
        };
        handler.handle(command.clone(), &mock_uow, &config).await?;

        let saved_village = mock_uow.villages().get_by_id(village_id).await?;
        let cost = smithy_upgrade_cost_for_unit(&command.unit, 0)?;

        // Lvl 1 Praetorian: 800, 1010, 1320, 650
        assert_eq!(
            saved_village.stored_resources().lumber(),
            2000 - cost.resources.0,
            "Lumber not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().clay(),
            2000 - cost.resources.1,
            "Clay not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().iron(),
            2000 - cost.resources.2,
            "Iron not deducted"
        );
        assert_eq!(
            saved_village.stored_resources().crop(),
            (2000 - cost.resources.3),
            "Crop not deducted"
        );

        let added_jobs = job_repo.list_by_player_id(player_id).await?;
        assert_eq!(added_jobs.len(), 1, "One job should be created");
        let job = &added_jobs[0];

        assert_eq!(job.task.task_type, "ResearchSmithy");
        let task: ResearchSmithyTask = serde_json::from_value(job.task.data.clone())?;
        assert_eq!(task.unit, UnitName::Praetorian);
        Ok(())
    }

    #[tokio::test]
    async fn test_smithy_handler_unit_not_researched() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_smithy()?;
        let village_repo = mock_uow.villages();

        village.set_academy_research_for_test(&UnitName::Praetorian, false);

        let village_id = village.id;
        village_repo.save(&village).await?;

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
        Ok(())
    }

    #[tokio::test]
    async fn test_smithy_handler_requirements_not_met() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (_player, mut village, config) = setup_village_for_smithy()?;
        let village_repo = mock_uow.villages();

        village.remove_building_at_slot(24, config.speed)?;
        village_repo.save(&village).await?;

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
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
    async fn test_smithy_handler_queue_limit() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village_for_smithy()?;
        let village_id = village.id;
        mock_uow.villages().save(&village).await?;

        let job_repo = mock_uow.jobs();
        let payload = ResearchSmithyTask {
            unit: UnitName::Praetorian,
        };
        let job_payload = JobPayload::new("ResearchSmithy", serde_json::to_value(&payload)?);
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

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            matches!(
                result,
                Err(ApplicationError::App(AppError::QueueLimitReached { queue }))
                if queue == "smithy"
            ),
            "Expected smithy queue limit error"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_smithy_handler_blocks_duplicate_job() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village_for_smithy()?;
        let village_id = village.id;
        mock_uow.villages().save(&village).await?;

        let payload = ResearchSmithyTask {
            unit: UnitName::Praetorian,
        };
        let job_payload = JobPayload::new("ResearchSmithy", serde_json::to_value(&payload)?);
        mock_uow
            .jobs()
            .add(&Job::new(player.id, village_id as i32, 60, job_payload))
            .await?;

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
            unit: UnitName::Praetorian,
            village_id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            matches!(
                result,
                Err(ApplicationError::App(AppError::QueueItemAlreadyQueued { queue, .. }))
                if queue == "smithy"
            ),
            "Expected duplicate smithy job error"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_smithy_handler_respects_queue_order() -> Result<()> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let (player, village, config) = setup_village_for_smithy()?;
        let village_id = village.id;
        mock_uow.villages().save(&village).await?;

        let job_repo = mock_uow.jobs();
        let existing_payload = ResearchSmithyTask {
            unit: UnitName::Legionnaire,
        };
        let existing_deadline = Utc::now() + Duration::seconds(120);
        job_repo
            .add(&Job::with_deadline(
                player.id,
                village_id as i32,
                JobPayload::new("ResearchSmithy", serde_json::to_value(&existing_payload)?),
                existing_deadline,
            ))
            .await?;

        let handler = ResearchSmithyCommandHandler::new();
        let command = ResearchSmithy {
            unit: UnitName::Praetorian,
            village_id,
        };
        handler.handle(command, &mock_uow, &config).await?;

        let jobs = job_repo.list_by_player_id(player.id).await?;
        let new_job = jobs
            .iter()
            .find(|job| {
                job.task.task_type == "ResearchSmithy"
                    && serde_json::from_value::<ResearchSmithyTask>(job.task.data.clone())
                        .map(|task| task.unit == UnitName::Praetorian)
                        .unwrap_or(false)
            })
            .expect("new job");

        let expected_cost = smithy_upgrade_cost_for_unit(&UnitName::Praetorian, 0)?;
        let expected_duration = ((expected_cost.time as f64 / config.speed as f64)
            .floor()
            .max(1.0)) as i64;

        assert_eq!(
            new_job.completed_at,
            existing_deadline + Duration::seconds(expected_duration),
            "Queued smithy job should finish after existing ones"
        );

        Ok(())
    }
}
