use std::sync::Arc;

use parabellum_types::Result;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::ResearchAcademy},
    jobs::{Job, JobPayload, tasks::ResearchAcademyTask},
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
        let research_time_secs = village.init_academy_research(&command.unit, config.speed)? as i64;
        village_repo.save(&village).await?;

        let payload = ResearchAcademyTask { unit: command.unit };
        let job_payload = JobPayload::new("ResearchAcademy", serde_json::to_value(&payload)?);
        let new_job = Job::new(
            village.player_id,
            command.village_id as i32,
            research_time_secs as i64,
            job_payload,
        );

        job_repo.add(&new_job).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use parabellum_types::{errors::GameError, Result};
    use parabellum_game::{
        models::{buildings::Building, village::Village, player::Player},
        test_utils::{
            PlayerFactoryOptions, VillageFactoryOptions, player_factory, village_factory,
        },
    };
    use parabellum_types::{
        army::UnitName,
        buildings::BuildingName,
        common::ResourceGroup,
        tribe::Tribe,
    };

    use super::*;
    use crate::{
        config::Config, jobs::tasks::ResearchAcademyTask, test_utils::tests::MockUnitOfWork,
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
        village.store_resources(&ResourceGroup(2000, 2000, 2000, 2000));

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
            (2000 - 580),
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
        village.store_resources(&ResourceGroup::default()); // No resources
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
}
