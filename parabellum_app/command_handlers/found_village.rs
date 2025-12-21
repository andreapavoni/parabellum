use std::sync::Arc;
use tracing::info;

use parabellum_types::errors::ApplicationError;

use crate::{
    command_handlers::helpers::deploy_army_from_village,
    config::Config,
    cqrs::{CommandHandler, commands::FoundVillage},
    jobs::{Job, JobPayload, tasks::FoundVillageTask},
    repository::{JobRepository, VillageRepository},
    uow::UnitOfWork,
};

pub struct FoundVillageCommandHandler {}

impl Default for FoundVillageCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FoundVillageCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<FoundVillage> for FoundVillageCommandHandler {
    async fn handle(
        &self,
        command: FoundVillage,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let job_repo: Arc<dyn JobRepository + '_> = uow.jobs();
        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();

        let origin_village = village_repo.get_by_id(command.village_id).await?;
        let (origin_village, deployed_army) = deploy_army_from_village(
            uow,
            origin_village,
            command.army_id,
            command.units,
            None, // No hero
        )
        .await?;

        // Calculate travel time to target position
        let travel_time_secs = origin_village.position.calculate_travel_time_secs(
            command.target_position.clone(),
            deployed_army.speed(),
            config.world_size as i32,
            config.speed as u8,
        ) as i64;

        // Create and enqueue a FoundVillage job for when the settlers arrive
        let target_field_id = command.target_position.to_id(config.world_size as i32);
        let found_village_payload = FoundVillageTask {
            army_id: deployed_army.id,
            settler_player_id: command.player_id,
            origin_village_id: command.village_id,
            target_position: command.target_position.clone(),
            target_field_id,
        };
        let job_payload = JobPayload::new(
            "FoundVillage",
            serde_json::to_value(&found_village_payload)?,
        );
        let new_job = Job::new(
            command.player_id,
            command.village_id as i32,
            travel_time_secs,
            job_payload,
        );
        job_repo.add(&new_job).await?;

        info!(
            found_village_job_id = %new_job.id,
            arrival_at = %new_job.completed_at,
            target_position = ?command.target_position,
            "FoundVillage job planned."
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_game::test_utils::setup_player_party;
    use parabellum_types::army::TroopSet;
    use parabellum_types::errors::ApplicationError;
    use parabellum_types::{map::Position, tribe::Tribe};

    use super::*;
    use crate::{
        config::Config, cqrs::commands::FoundVillage, test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
    };

    #[tokio::test]
    async fn test_found_village_handler_creates_job() -> Result<(), ApplicationError> {
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let config = Arc::new(Config::from_env());
        let handler = FoundVillageCommandHandler::new();

        // Set up a village with settlers
        let (player, village, army, _) = setup_player_party(
            None,
            Tribe::Roman,
            TroopSet::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 3]),
            false,
        )?;

        // Save to mock
        mock_uow.villages().save(&village).await?;
        mock_uow.armies().save(&army).await?;

        let target_position = Position { x: 10, y: 10 };
        let mut settler_troops = TroopSet::default();
        settler_troops.set(9, 3); // 3 settlers

        let command = FoundVillage {
            player_id: player.id,
            village_id: village.id,
            army_id: army.id,
            units: settler_troops,
            target_position,
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Check that a job was created
        let jobs = mock_uow.jobs().list_by_player_id(player.id).await?;
        assert_eq!(
            jobs.len(),
            1,
            "One job should be created for settler movement"
        );

        let job = &jobs[0];
        assert_eq!(job.task.task_type, "FoundVillage");

        Ok(())
    }
}
