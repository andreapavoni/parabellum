use std::sync::Arc;
use chrono::Utc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::{
    Alliance, AllianceLog, AllianceLogType, AlliancePermission,
};
use parabellum_types::buildings::BuildingName;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::CreateAlliance},
    uow::UnitOfWork,
};

pub struct CreateAllianceCommandHandler {}

impl Default for CreateAllianceCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateAllianceCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<CreateAlliance> for CreateAllianceCommandHandler {
    async fn handle(
        &self,
        command: CreateAlliance,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let alliance_repo = uow.alliances();
        let alliance_log_repo = uow.alliance_logs();
        let player_repo = uow.players();
        let village_repo = uow.villages();

        // Verify player exists
        let player = player_repo.get_by_id(command.player_id).await?;

        // Verify player is not already in an alliance
        if player.alliance_id.is_some() {
            return Err(GameError::PlayerAlreadyInAlliance.into());
        }

        // Get capital village and verify embassy level 3+
        let capital = village_repo
            .get_capital_by_player_id(command.player_id)
            .await?;

        let embassy = capital
            .get_building_by_name(&BuildingName::Embassy)
            .ok_or_else(|| GameError::BuildingNotFound(BuildingName::Embassy))?;

        if embassy.building.level < 3 {
            return Err(GameError::BuildingRequirementsNotMet {
                building: BuildingName::Embassy,
                level: 3,
            }
            .into());
        }

        // Verify alliance name and tag are unique
        if alliance_repo.get_by_tag(command.tag.clone()).await.is_ok() {
            return Err(GameError::AllianceTagAlreadyExists.into());
        }

        if alliance_repo.get_by_name(command.name.clone()).await.is_ok() {
            return Err(GameError::AllianceNameAlreadyExists.into());
        }

        // Create alliance with max members based on embassy level
        // The creator becomes the initial leader
        let alliance = Alliance::new(
            command.name.clone(),
            command.tag.clone(),
            embassy.building.level as i32,
            command.player_id,
        );

        // Persist alliance and update player
        alliance_repo.save(&alliance).await?;

        let current_time = Utc::now().timestamp() as i32;
        player_repo
            .update_alliance_fields(
                command.player_id,
                Some(alliance.id),
                Some(AlliancePermission::all_permissions()),
                Some(current_time),
            )
            .await?;

        // Log alliance creation
        let log = AllianceLog::new(
            alliance.id,
            AllianceLogType::AllianceCreated,
            Some(format!(
                "Alliance '{}' [{}] created by {}",
                command.name, command.tag, player.username
            )),
            current_time,
        );
        alliance_log_repo.save(&log).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
        village_factory,
    };
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};
    use parabellum_game::models::alliance::Alliance;

    #[tokio::test]
    async fn test_create_alliance_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateAllianceCommandHandler::new();

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

        // Set village as capital
        village.is_capital = true;

        // Add Embassy building at level 3
        let embassy = parabellum_game::models::buildings::Building::new(
            BuildingName::Embassy,
            config.speed,
        )
        .at_level(3, config.speed)
        .unwrap();
        village.add_building_at_slot(embassy, 20).unwrap();

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        let command = CreateAlliance {
            player_id: player.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };

        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify alliance was created
        let alliances = mock_uow.alliances();
        let alliance = alliances.get_by_name("Test Alliance".to_string()).await;
        assert!(alliance.is_ok(), "Alliance should be created");

        let alliance = alliance.unwrap();
        assert_eq!(alliance.name, "Test Alliance");
        assert_eq!(alliance.tag, "TEST");
        assert_eq!(alliance.max_members, 3); // Embassy level 3
        assert_eq!(alliance.leader_id, Some(player.id));

        // Verify player was updated
        let updated_player = mock_uow.players().get_by_id(player.id).await.unwrap();
        assert_eq!(updated_player.alliance_id, Some(alliance.id));
        assert_eq!(updated_player.alliance_role, Some(255)); // All permissions
        assert!(updated_player.alliance_join_time.is_some());

        // Verify alliance log was created
        let logs = mock_uow.alliance_logs()
            .get_by_alliance_id(alliance.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].type_, AllianceLogType::AllianceCreated as i16);
    }

    #[tokio::test]
    async fn test_create_alliance_player_already_in_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateAllianceCommandHandler::new();

        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        // Player already in an alliance
        player.alliance_id = Some(Uuid::new_v4());

        mock_uow.players().save(&player).await.unwrap();

        let command = CreateAlliance {
            player_id: player.id,
            name: "New Alliance".to_string(),
            tag: "NEW".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::PlayerAlreadyInAlliance) => {},
            e => panic!("Expected PlayerAlreadyInAlliance error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_alliance_no_embassy() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateAllianceCommandHandler::new();

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

        village.is_capital = true;
        // No embassy building added

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        let command = CreateAlliance {
            player_id: player.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::BuildingNotFound(BuildingName::Embassy)) => {},
            e => panic!("Expected BuildingNotFound(Embassy) error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_alliance_embassy_level_too_low() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateAllianceCommandHandler::new();

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

        village.is_capital = true;

        // Add Embassy building at level 1 (needs level 3+)
        let embassy = parabellum_game::models::buildings::Building::new(
            BuildingName::Embassy,
            config.speed,
        )
        .at_level(1, config.speed)
        .unwrap();
        village.add_building_at_slot(embassy, 20).unwrap();

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        let command = CreateAlliance {
            player_id: player.id,
            name: "Test Alliance".to_string(),
            tag: "TEST".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::BuildingRequirementsNotMet { building, level }) => {
                assert_eq!(building, BuildingName::Embassy);
                assert_eq!(level, 3);
            },
            e => panic!("Expected BuildingRequirementsNotMet error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_alliance_duplicate_tag() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateAllianceCommandHandler::new();

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

        village.is_capital = true;

        // Add Embassy building at level 3
        let embassy = parabellum_game::models::buildings::Building::new(
            BuildingName::Embassy,
            config.speed,
        )
        .at_level(3, config.speed)
        .unwrap();
        village.add_building_at_slot(embassy, 20).unwrap();

        // Create existing alliance with same tag
        let existing_alliance = Alliance::new(
            "Existing Alliance".to_string(),
            "TEST".to_string(),
            3,
            Uuid::new_v4(),
        );
        mock_uow.alliances().save(&existing_alliance).await.unwrap();

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        let command = CreateAlliance {
            player_id: player.id,
            name: "New Alliance".to_string(),
            tag: "TEST".to_string(), // Duplicate tag
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::AllianceTagAlreadyExists) => {},
            e => panic!("Expected AllianceTagAlreadyExists error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_alliance_duplicate_name() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateAllianceCommandHandler::new();

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

        village.is_capital = true;

        // Add Embassy building at level 3
        let embassy = parabellum_game::models::buildings::Building::new(
            BuildingName::Embassy,
            config.speed,
        )
        .at_level(3, config.speed)
        .unwrap();
        village.add_building_at_slot(embassy, 20).unwrap();

        // Create existing alliance with same name
        let existing_alliance = Alliance::new(
            "Test Alliance".to_string(),
            "OLD".to_string(),
            3,
            Uuid::new_v4(),
        );
        mock_uow.alliances().save(&existing_alliance).await.unwrap();

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();

        let command = CreateAlliance {
            player_id: player.id,
            name: "Test Alliance".to_string(), // Duplicate name
            tag: "NEW".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::AllianceNameAlreadyExists) => {},
            e => panic!("Expected AllianceNameAlreadyExists error, got: {:?}", e),
        }
    }
}
