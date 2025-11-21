use std::sync::Arc;
use parabellum_types::map::Position;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::AlliancePermission;
use parabellum_game::models::map_flag::{MapFlag, MapFlagType};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::CreateCustomFlag},
    uow::UnitOfWork,
};

pub struct CreateCustomFlagCommandHandler {}

impl Default for CreateCustomFlagCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateCustomFlagCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<CreateCustomFlag> for CreateCustomFlagCommandHandler {
    async fn handle(
        &self,
        command: CreateCustomFlag,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        // Get player to verify they exist
        let player = uow.players().get_by_id(command.player_id).await?;

        // Determine ownership
        let is_alliance_owned = command.alliance_id.is_some();

        // If alliance-owned, verify permissions
        if is_alliance_owned {
            let alliance_id = command.alliance_id.unwrap();

            // Verify player is in the alliance
            if player.alliance_id != Some(alliance_id) {
                return Err(GameError::NotInAlliance.into());
            }

            // Verify player has MANAGE_MARKS permission
            AlliancePermission::verify_permission(&player, AlliancePermission::ManageMarks)?;
        }

        // Check custom flag limit (5 per owner)
        let custom_flag_count = uow.map_flags()
            .count_by_owner(
                if is_alliance_owned { None } else { Some(command.player_id) },
                if is_alliance_owned { command.alliance_id } else { None },
                Some(MapFlagType::CustomFlag.as_i16()),
            )
            .await?;

        if custom_flag_count >= 5 {
            return Err(GameError::MapFlagLimitExceeded.into());
        }

        // Create the custom flag
        let mut flag = if is_alliance_owned {
            MapFlag::new_alliance_flag(
                command.alliance_id.unwrap(),
                MapFlagType::CustomFlag,
                command.color,
                command.player_id,
            )
        } else {
            MapFlag::new_player_flag(
                command.player_id,
                MapFlagType::CustomFlag,
                command.color,
                command.player_id,
            )
        };

        // Set coordinates and text
        flag = flag
            .with_position(Position { x: command.x, y: command.y })
            .with_text(command.text)?;

        // Validate the flag
        flag.validate(config.world_size)?;

        // Save to database
        uow.map_flags().save(&flag).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_game::test_utils::{PlayerFactoryOptions, player_factory};
    use parabellum_types::tribe::Tribe;

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};

    #[tokio::test]
    async fn test_create_custom_flag_player_owned_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateCustomFlagCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        let command = CreateCustomFlag {
            player_id: player.id,
            alliance_id: None,
            x: 100,
            y: 50,
            color: 5,
            text: "Test flag".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify flag was created
        let flags = mock_uow.map_flags().get_by_player_id(player.id).await.unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].player_id, Some(player.id));
        assert_eq!(flags[0].alliance_id, None);
        assert_eq!(flags[0].position, Some(Position { x: 100, y: 50 }));
        assert_eq!(flags[0].color, 5);
        assert_eq!(flags[0].text, Some("Test flag".to_string()));
        assert_eq!(flags[0].flag_type, 2); // CustomFlag
    }

    #[tokio::test]
    async fn test_create_custom_flag_alliance_owned_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateCustomFlagCommandHandler::new();

        let alliance_id = Uuid::new_v4();
        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        player.alliance_id = Some(alliance_id);
        player.alliance_role = Some(128); // MANAGE_MARKS permission

        mock_uow.players().save(&player).await.unwrap();

        let command = CreateCustomFlag {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            x: 50,
            y: -50,
            color: 15,
            text: "Alliance flag".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify flag was created
        let flags = mock_uow.map_flags().get_by_alliance_id(alliance_id).await.unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].alliance_id, Some(alliance_id));
        assert_eq!(flags[0].player_id, None);
        assert_eq!(flags[0].color, 15);
    }

    #[tokio::test]
    async fn test_create_custom_flag_no_permission() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateCustomFlagCommandHandler::new();

        let alliance_id = Uuid::new_v4();
        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        player.alliance_id = Some(alliance_id);
        player.alliance_role = Some(0); // No permissions

        mock_uow.players().save(&player).await.unwrap();

        let command = CreateCustomFlag {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            x: 100,
            y: 50,
            color: 15,
            text: "Test".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::NoManageMarksPermission) => {},
            e => panic!("Expected NoManageMarksPermission error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_custom_flag_limit_exceeded() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateCustomFlagCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        // Create 5 existing flags (limit)
        for i in 0..5 {
            let flag = MapFlag::new_player_flag(
                player.id,
                MapFlagType::CustomFlag,
                5,
                player.id,
            )
            .with_position(Position { x: i * 10, y: i * 10 })
            .with_text(format!("Flag {}", i)).unwrap();

            mock_uow.map_flags().save(&flag).await.unwrap();
        }

        let command = CreateCustomFlag {
            player_id: player.id,
            alliance_id: None,
            x: 100,
            y: 50,
            color: 5,
            text: "Sixth flag".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::MapFlagLimitExceeded) => {},
            e => panic!("Expected MapFlagLimitExceeded error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_custom_flag_invalid_color_player() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateCustomFlagCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        let command = CreateCustomFlag {
            player_id: player.id,
            alliance_id: None,
            x: 100,
            y: 50,
            color: 15, // Invalid for player (should be 0-10)
            text: "Test".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::InvalidMapFlagColor { color, min, max }) => {
                assert_eq!(color, 15);
                assert_eq!(min, 0);
                assert_eq!(max, 10);
            },
            e => panic!("Expected InvalidMapFlagColor error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_custom_flag_invalid_color_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateCustomFlagCommandHandler::new();

        let alliance_id = Uuid::new_v4();
        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        player.alliance_id = Some(alliance_id);
        player.alliance_role = Some(128);

        mock_uow.players().save(&player).await.unwrap();

        let command = CreateCustomFlag {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            x: 100,
            y: 50,
            color: 5, // Invalid for alliance (should be 10-20)
            text: "Test".to_string(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::InvalidMapFlagColor { color, min, max }) => {
                assert_eq!(color, 5);
                assert_eq!(min, 10);
                assert_eq!(max, 20);
            },
            e => panic!("Expected InvalidMapFlagColor error, got: {:?}", e),
        }
    }
}
