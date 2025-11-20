use std::sync::Arc;
use parabellum_types::map::Position;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::map_flag::MapFlagType;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::UpdateMapFlag},
    uow::UnitOfWork,
};

// Alliance permission constant for managing map flags
const MANAGE_MARKS: i32 = 128;

pub struct UpdateMapFlagCommandHandler {}

impl Default for UpdateMapFlagCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateMapFlagCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<UpdateMapFlag> for UpdateMapFlagCommandHandler {
    async fn handle(
        &self,
        command: UpdateMapFlag,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        // Get player to verify they exist
        let player = uow.players().get_by_id(command.player_id).await?;

        // Get the existing flag
        let mut flag = uow.map_flags().get_by_id(command.flag_id).await?;

        // Determine ownership of the flag
        let is_alliance_owned = flag.alliance_id.is_some();
        let is_player_owned = flag.player_id.is_some();

        // Verify ownership matches the command
        if command.alliance_id.is_some() != is_alliance_owned {
            return Err(GameError::MapFlagInvalidOwnership.into());
        }

        // Verify the player owns the flag or has permission
        if is_alliance_owned {
            let alliance_id = flag.alliance_id.unwrap();

            // Verify player is in the alliance
            if player.alliance_id != Some(alliance_id) {
                return Err(GameError::NotInAlliance.into());
            }

            // Verify player has MANAGE_MARKS permission
            let alliance_role = player.alliance_role.unwrap_or(0);
            if (alliance_role & MANAGE_MARKS) == 0 {
                return Err(GameError::NoManageMarksPermission.into());
            }

            // Verify alliance ID matches
            if Some(alliance_id) != command.alliance_id {
                return Err(GameError::MapFlagInvalidOwnership.into());
            }
        } else if is_player_owned {
            // Verify the player owns this flag
            if flag.player_id != Some(command.player_id) {
                return Err(GameError::MapFlagInvalidOwnership.into());
            }
        }

        // Determine flag type
        let flag_type = MapFlagType::from_i16(flag.flag_type)?;

        // Validate color range based on flag type and ownership
        match flag_type {
            MapFlagType::PlayerMark | MapFlagType::AllianceMark => {
                // Multi-marks: 0-9
                if command.color < 0 || command.color > 9 {
                    return Err(GameError::InvalidMapFlagColor {
                        color: command.color,
                        min: 0,
                        max: 9,
                    }.into());
                }
            },
            MapFlagType::CustomFlag => {
                if is_alliance_owned {
                    // Alliance custom flags: 10-20
                    if command.color < 10 || command.color > 20 {
                        return Err(GameError::InvalidMapFlagColor {
                            color: command.color,
                            min: 10,
                            max: 20,
                        }.into());
                    }
                } else {
                    // Player custom flags: 0-10
                    if command.color < 0 || command.color > 10 {
                        return Err(GameError::InvalidMapFlagColor {
                            color: command.color,
                            min: 0,
                            max: 10,
                        }.into());
                    }
                }
            },
        }

        // Update the flag
        flag.color = command.color;

        // Update text only for custom flags
        if flag_type == MapFlagType::CustomFlag && command.text.is_some() {
            flag = flag.with_text(command.text.unwrap());
        }

        // Save the updated flag
        uow.map_flags().update(&flag).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;
    use chrono::Utc;

    use parabellum_game::test_utils::{PlayerFactoryOptions, player_factory};
    use parabellum_types::tribe::Tribe;
    use parabellum_game::models::map_flag::MapFlag;

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};

    #[tokio::test]
    async fn test_update_custom_flag_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = UpdateMapFlagCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        // Create a custom flag
        let flag = MapFlag::new_player_flag(
            player.id,
            MapFlagType::CustomFlag,
            5,
            player.id,
        )
        .with_position(Position { x: 100, y: 50 })
        .with_text("Original text".to_string());

        mock_uow.map_flags().save(&flag).await.unwrap();

        let command = UpdateMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id: flag.id,
            color: 8,
            text: Some("Updated text".to_string()),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify flag was updated
        let updated_flag = mock_uow.map_flags().get_by_id(flag.id).await.unwrap();
        assert_eq!(updated_flag.color, 8);
        assert_eq!(updated_flag.text, Some("Updated text".to_string()));
    }

    #[tokio::test]
    async fn test_update_multi_mark_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = UpdateMapFlagCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let target_player = player_factory(PlayerFactoryOptions::default());

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.players().save(&target_player).await.unwrap();

        // Create a player mark
        let flag = MapFlag::new_player_flag(
            player.id,
            MapFlagType::PlayerMark,
            3,
            player.id,
        )
        .with_target(target_player.id);

        mock_uow.map_flags().save(&flag).await.unwrap();

        let command = UpdateMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id: flag.id,
            color: 7,
            text: None,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify flag was updated
        let updated_flag = mock_uow.map_flags().get_by_id(flag.id).await.unwrap();
        assert_eq!(updated_flag.color, 7);
    }

    #[tokio::test]
    async fn test_update_flag_wrong_owner() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = UpdateMapFlagCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let other_player = player_factory(PlayerFactoryOptions::default());

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.players().save(&other_player).await.unwrap();

        // Create a flag owned by other_player
        let flag = MapFlag::new_player_flag(
            other_player.id,
            MapFlagType::CustomFlag,
            5,
            other_player.id,
        )
        .with_position(Position { x: 100, y: 50 })
        .with_text("Test".to_string());

        mock_uow.map_flags().save(&flag).await.unwrap();

        // Try to update with different player
        let command = UpdateMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id: flag.id,
            color: 8,
            text: Some("Hacked".to_string()),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::MapFlagInvalidOwnership) => {},
            e => panic!("Expected MapFlagInvalidOwnership error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_update_alliance_flag_no_permission() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = UpdateMapFlagCommandHandler::new();

        let alliance_id = Uuid::new_v4();
        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        player.alliance_id = Some(alliance_id);
        player.alliance_role = Some(0); // No permissions

        mock_uow.players().save(&player).await.unwrap();

        // Create an alliance flag
        let flag = MapFlag::new_alliance_flag(
            alliance_id,
            MapFlagType::CustomFlag,
            15,
            player.id,
        )
        .with_position(Position { x: 100, y: 50 })
        .with_text("Test".to_string());

        mock_uow.map_flags().save(&flag).await.unwrap();

        let command = UpdateMapFlag {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            flag_id: flag.id,
            color: 18,
            text: Some("Updated".to_string()),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::NoManageMarksPermission) => {},
            e => panic!("Expected NoManageMarksPermission error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_update_flag_invalid_color() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = UpdateMapFlagCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        // Create a custom flag
        let flag = MapFlag::new_player_flag(
            player.id,
            MapFlagType::CustomFlag,
            5,
            player.id,
        )
        .with_position(Position { x: 100, y: 50 })
        .with_text("Test".to_string());

        mock_uow.map_flags().save(&flag).await.unwrap();

        let command = UpdateMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id: flag.id,
            color: 25, // Invalid for player custom flags (0-10)
            text: None,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::InvalidMapFlagColor { color, min, max }) => {
                assert_eq!(color, 25);
                assert_eq!(min, 0);
                assert_eq!(max, 10);
            },
            e => panic!("Expected InvalidMapFlagColor error, got: {:?}", e),
        }
    }
}
