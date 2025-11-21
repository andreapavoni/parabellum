use std::sync::Arc;

use parabellum_core::ApplicationError;
use parabellum_game::models::alliance::BonusType;

use crate::{
    config::Config,
    cqrs::CommandHandler,
    uow::UnitOfWork,
};

use super::super::cqrs::commands::ContributeToAllianceBonus;

pub struct ContributeToAllianceBonusCommandHandler;

#[async_trait::async_trait]
impl CommandHandler<ContributeToAllianceBonus> for ContributeToAllianceBonusCommandHandler {
    async fn handle(
        &self,
        command: ContributeToAllianceBonus,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let bonus_type = BonusType::from_i16(command.bonus_type)
            .ok_or(parabellum_core::GameError::InvalidBonusType(command.bonus_type))?;

        let mut player = uow.players().get_by_id(command.player_id).await?;
        let mut village = uow.villages().get_by_id(command.village_id).await?;
        let mut alliance = uow.alliances().get_by_id(command.alliance_id).await?;

        // Verify player is in the alliance and owns the village
        if player.alliance_id != Some(command.alliance_id) {
            return Err(ApplicationError::Game(parabellum_core::GameError::NotInAlliance));
        }
        village.verify_ownership(command.player_id)?;

        // Get player's capital to check embassy level
        let capital = uow.villages().get_capital_by_player_id(command.player_id).await?;
        let embassy_level = capital.get_embassy_level().unwrap_or(0);

        // Get current time for cooldown check
        use chrono::Utc;
        let current_time = Utc::now();

        // Process contribution (includes all validations: donation limit, cooldown, etc.)
        let contribution_result = alliance.add_contribution(
            bonus_type,
            &command.resources,
            &mut village,
            &mut player,
            embassy_level,
            config.speed as i32,
            current_time,
        )?;

        uow.villages().save(&village).await?;
        uow.players().save(&player).await?;
        uow.alliances().update(&alliance).await?;

        // Schedule upgrade job if bonus level up is triggered
        if contribution_result.upgrade_triggered {
            if let Some(base_duration) = alliance.get_upgrade_duration_seconds(bonus_type) {
                let duration_seconds = (base_duration as f64 / config.speed as f64) as i64;

                use crate::jobs::{Job, JobPayload, tasks::AllianceBonusUpgradeTask};
                use serde_json::json;

                let task = AllianceBonusUpgradeTask {
                    alliance_id: alliance.id,
                    bonus_type: bonus_type.as_i16(),
                };

                let job_payload = JobPayload::new("AllianceBonusUpgrade", json!(task));
                let job = Job::new(
                    command.player_id,
                    command.village_id as i32,
                    duration_seconds as i64,
                    job_payload,
                );

                uow.jobs().add(&job).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;
    use parabellum_core::{ApplicationError, Result};
    use parabellum_game::{
        models::{alliance::Alliance, player::Player, village::Village},
        test_utils::{
            PlayerFactoryOptions, ValleyFactoryOptions, VillageFactoryOptions, player_factory,
            valley_factory, village_factory,
        },
    };
    use parabellum_types::{
        common::ResourceGroup,
        map::Position,
        tribe::Tribe,
    };
    use crate::{
        config::Config,
        cqrs::{CommandHandler, commands::ContributeToAllianceBonus},
        test_utils::tests::MockUnitOfWork,
        uow::UnitOfWork,
        command_handlers::contribute_alliance_bonus::ContributeToAllianceBonusCommandHandler,
    };

    async fn setup_test_environment(
        _config: &Arc<Config>,
    ) -> Result<(
        Box<dyn UnitOfWork<'static> + 'static>,
        Player,
        Village,
        Alliance,
    )> {
        let mock_uow: Box<dyn UnitOfWork<'static> + 'static> = Box::new(MockUnitOfWork::new());

        let mut alliance = Alliance::new("Test Alliance".to_string(), "TEST".to_string(), 60, Uuid::new_v4()).unwrap();
        alliance.id = Uuid::new_v4();

        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        player.alliance_id = Some(alliance.id);

        let valley = valley_factory(ValleyFactoryOptions {
            position: Some(Position { x: 0, y: 0 }),
            ..Default::default()
        });

        let mut village = village_factory(VillageFactoryOptions {
            player: Some(player.clone()),
            valley: Some(valley),
            ..Default::default()
        });

        // Add Warehouse and Granary if missing and upgrade to increase capacity
        use parabellum_game::models::buildings::Building;
        use parabellum_types::buildings::BuildingName;

        if village.get_building_by_name(&BuildingName::Warehouse).is_none() {
            let warehouse = Building::new(BuildingName::Warehouse, 1);
            village.add_building_at_slot(warehouse, 30).unwrap();
        }
        let warehouse_slot = village.get_building_by_name(&BuildingName::Warehouse).unwrap().slot_id;
        village.set_building_level_at_slot(warehouse_slot, 20, 1).unwrap();

        if village.get_building_by_name(&BuildingName::Granary).is_none() {
            let granary = Building::new(BuildingName::Granary, 1);
            village.add_building_at_slot(granary, 31).unwrap();
        }
        let granary_slot = village.get_building_by_name(&BuildingName::Granary).unwrap().slot_id;
        village.set_building_level_at_slot(granary_slot, 20, 1).unwrap();


        // Ensure village has enough resources for contribution
        // First deduct all existing resources, then add exact amount needed
        let current_resources = village.stored_resources();
        if current_resources.total() > 0 {
            village.deduct_resources(&current_resources).unwrap();
        }
        // 60k total = 15k of each resource
        village.store_resources(&ResourceGroup(15_000, 15_000, 15_000, 15_000));

        // Set production to zero to prevent time-based resource generation during tests
        village.production = Default::default();

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        Ok((mock_uow, player, village, alliance))
    }

    #[tokio::test]
    async fn test_contribute_success() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, player, village, alliance) = setup_test_environment(&config).await?;

        let handler = ContributeToAllianceBonusCommandHandler;
        let command = ContributeToAllianceBonus {
            player_id: player.id,
            village_id: village.id,
            alliance_id: alliance.id,
            bonus_type: 1, // Training
            resources: ResourceGroup(1000, 1000, 1000, 1000), // 4000 total -> 4 points
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Verify resources deducted (started with 60k, deducted 4k = 56k remaining)
        let saved_village = mock_uow.villages().get_by_id(village.id).await?;
        assert_eq!(saved_village.stored_resources().total(), 56000);

        // Verify Player contributions updated
        let saved_player = mock_uow.players().get_by_id(player.id).await?;
        assert_eq!(saved_player.current_alliance_training_contributions, 4);
        assert_eq!(saved_player.total_alliance_training_contributions, 4);

        // Verify Alliance contributions updated
        let saved_alliance = mock_uow.alliances().get_by_id(alliance.id).await?;
        assert_eq!(saved_alliance.training_bonus_contributions, 4);

        Ok(())
    }

    #[tokio::test]
    async fn test_contribute_triggers_upgrade() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, player, mut village, mut alliance) = setup_test_environment(&config).await?;

        // Set initial contribution to near threshold for level 1
        // Need to reach the threshold exactly, so set to threshold - 5
        use parabellum_game::models::alliance::Alliance;
        let level_1_threshold = Alliance::get_bonus_contributions_needed(1, 1);
        alliance.training_bonus_contributions = level_1_threshold - 5;
        mock_uow.alliances().save(&alliance).await?;

        // Give village extra resources for the large contribution
        village.store_resources(&ResourceGroup(10_000, 10_000, 10_000, 10_000));
        mock_uow.villages().save(&village).await?;

        let handler = ContributeToAllianceBonusCommandHandler;

        let command = ContributeToAllianceBonus {
            player_id: player.id,
            alliance_id: alliance.id,
            village_id: village.id,
            bonus_type: 1,
            resources: ResourceGroup(1_250, 1_250, 1_250, 1_250), // 5k total = 5 points (reaches threshold)
        };

        handler.handle(command, &mock_uow, &config).await?;

        // Verify upgrade job was created
        let jobs = mock_uow.jobs().find_and_lock_due_jobs(100).await?;
        let alliance_job = jobs.iter().find(|job| job.task.task_type == "AllianceBonusUpgrade");
        assert!(alliance_job.is_some(), "Alliance bonus upgrade job should be created");
        assert_eq!(alliance_job.unwrap().task.task_type, "AllianceBonusUpgrade");

        Ok(())
    }

    #[tokio::test]
    async fn test_contribute_exceeds_donation_limit() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, mut player, village, alliance) = setup_test_environment(&config).await?;

        // Set player contributions to near the donation limit
        // For embassy level 20, limit is 1,000,000 * speed
        // Set current contributions to 999,996, try to add 5 (would exceed limit)
        use parabellum_game::models::alliance::Alliance;
        let donation_limit = Alliance::get_donation_limit(20, config.speed as i32);
        player.current_alliance_training_contributions = donation_limit - 4;
        mock_uow.players().save(&player).await?;

        let handler = ContributeToAllianceBonusCommandHandler;
        let command = ContributeToAllianceBonus {
            player_id: player.id,
            village_id: village.id,
            alliance_id: alliance.id,
            bonus_type: 1, // Training
            resources: ResourceGroup(1_250, 1_250, 1_250, 1_250), // 5k total = 5 points (would exceed limit)
        };

        // Should fail with donation limit exceeded
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        match result {
            Err(ApplicationError::Game(parabellum_core::GameError::AllianceDonationLimitExceeded)) => {
                // Expected error
            }
            _ => panic!("Expected AllianceDonationLimitExceeded error"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_contribute_fails_new_player_cooldown() -> Result<()> {
        let config = Arc::new(Config::from_env());
        let (mock_uow, mut player, village, mut alliance) = setup_test_environment(&config).await?;

        // Set alliance to level 3 (triggers cooldown for new players)
        alliance.training_bonus_level = 3;
        mock_uow.alliances().save(&alliance).await?;

        // Set player as having just joined (1 second ago)
        use chrono::{Utc, Duration};
        let join_time = Utc::now() - Duration::seconds(1);
        player.alliance_join_time = Some(join_time);
        mock_uow.players().save(&player).await?;

        let handler = ContributeToAllianceBonusCommandHandler;
        let command = ContributeToAllianceBonus {
            player_id: player.id,
            village_id: village.id,
            alliance_id: alliance.id,
            bonus_type: 1, // Training
            resources: ResourceGroup(1_000, 1_000, 1_000, 1_000),
        };

        // Should fail with new player cooldown
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());
        match result {
            Err(ApplicationError::Game(parabellum_core::GameError::AllianceNewPlayerCooldown)) => {
                // Expected error
            }
            _ => panic!("Expected AllianceNewPlayerCooldown error"),
        }

        Ok(())
    }
}
