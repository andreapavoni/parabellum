use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::map_flag::{MapFlag, MapFlagType};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::CreateMultiMark},
    uow::UnitOfWork,
};

// Alliance permission constant for managing map flags
const MANAGE_MARKS: i32 = 128;

pub struct CreateMultiMarkCommandHandler {}

impl Default for CreateMultiMarkCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateMultiMarkCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<CreateMultiMark> for CreateMultiMarkCommandHandler {
    async fn handle(
        &self,
        command: CreateMultiMark,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
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
            let alliance_role = player.alliance_role.unwrap_or(0);
            if (alliance_role & MANAGE_MARKS) == 0 {
                return Err(GameError::NoManageMarksPermission.into());
            }
        }

        // Validate mark type (0 = player mark, 1 = alliance mark)
        let flag_type = MapFlagType::from_i16(command.mark_type)?;
        match flag_type {
            MapFlagType::PlayerMark | MapFlagType::AllianceMark => {},
            _ => return Err(GameError::InvalidMapFlagType(command.mark_type).into()),
        }

        // Check multi-mark limit (10 per owner, combined types 0 and 1)
        let multi_mark_count = uow.map_flags()
            .count_by_owner(
                if is_alliance_owned { None } else { Some(command.player_id) },
                if is_alliance_owned { command.alliance_id } else { None },
                None,  // Count all multi-marks (types 0 and 1)
            )
            .await?;

        // Subtract custom flags from the count since we only count multi-marks
        let custom_flag_count = uow.map_flags()
            .count_by_owner(
                if is_alliance_owned { None } else { Some(command.player_id) },
                if is_alliance_owned { command.alliance_id } else { None },
                Some(MapFlagType::CustomFlag.as_i16()),
            )
            .await?;

        let actual_multi_mark_count = multi_mark_count - custom_flag_count;

        if actual_multi_mark_count >= 10 {
            return Err(GameError::MapFlagLimitExceeded.into());
        }

        // Validate color range (0-9 for multi-marks)
        if command.color < 0 || command.color > 9 {
            return Err(GameError::InvalidMapFlagColor {
                color: command.color,
                min: 0,
                max: 9,
            }.into());
        }

        // Verify target exists
        match flag_type {
            MapFlagType::PlayerMark => {
                // Verify target player exists
                let _target_player = uow.players().get_by_id(command.target_id).await?;
            },
            MapFlagType::AllianceMark => {
                // Verify target alliance exists
                let _target_alliance = uow.alliances().get_by_id(command.target_id).await?;
            },
            _ => unreachable!(),
        }

        // Create the multi-mark
        let mut flag = if is_alliance_owned {
            MapFlag::new_alliance_flag(
                command.alliance_id.unwrap(),
                flag_type,
                command.color,
                command.player_id,
            )
        } else {
            MapFlag::new_player_flag(
                command.player_id,
                flag_type,
                command.color,
                command.player_id,
            )
        };

        // Set target
        flag = flag.with_target(command.target_id);

        // Validate the flag
        flag.validate()?;

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
    use parabellum_game::models::alliance::Alliance;

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};

    #[tokio::test]
    async fn test_create_player_mark_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateMultiMarkCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.players().save(&target_player).await.unwrap();

        let command = CreateMultiMark {
            player_id: player.id,
            alliance_id: None,
            target_id: target_player.id,
            mark_type: 0, // Player mark
            color: 3,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify mark was created
        let flags = mock_uow.map_flags().get_by_player_id(player.id).await.unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].player_id, Some(player.id));
        assert_eq!(flags[0].alliance_id, None);
        assert_eq!(flags[0].target_id, Some(target_player.id));
        assert_eq!(flags[0].color, 3);
        assert_eq!(flags[0].flag_type, 0); // PlayerMark
    }

    #[tokio::test]
    async fn test_create_alliance_mark_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateMultiMarkCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let target_alliance = Alliance::new(
            "Enemy Alliance".to_string(),
            "ENEMY".to_string(),
            10,
            Uuid::new_v4(),
        );

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.alliances().save(&target_alliance).await.unwrap();

        let command = CreateMultiMark {
            player_id: player.id,
            alliance_id: None,
            target_id: target_alliance.id,
            mark_type: 1, // Alliance mark
            color: 5,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify mark was created
        let flags = mock_uow.map_flags().get_by_player_id(player.id).await.unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].target_id, Some(target_alliance.id));
        assert_eq!(flags[0].flag_type, 1); // AllianceMark
    }

    #[tokio::test]
    async fn test_create_multi_mark_alliance_owned_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateMultiMarkCommandHandler::new();

        let alliance_id = Uuid::new_v4();
        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        player.alliance_id = Some(alliance_id);
        player.alliance_role = Some(128); // MANAGE_MARKS permission

        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.players().save(&target_player).await.unwrap();

        let command = CreateMultiMark {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            target_id: target_player.id,
            mark_type: 0,
            color: 7,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify mark was created
        let flags = mock_uow.map_flags().get_by_alliance_id(alliance_id).await.unwrap();
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].alliance_id, Some(alliance_id));
        assert_eq!(flags[0].player_id, None);
    }

    #[tokio::test]
    async fn test_create_multi_mark_no_permission() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateMultiMarkCommandHandler::new();

        let alliance_id = Uuid::new_v4();
        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        player.alliance_id = Some(alliance_id);
        player.alliance_role = Some(0); // No permissions

        let target_player = player_factory(PlayerFactoryOptions::default());

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.players().save(&target_player).await.unwrap();

        let command = CreateMultiMark {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            target_id: target_player.id,
            mark_type: 0,
            color: 3,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::NoManageMarksPermission) => {},
            e => panic!("Expected NoManageMarksPermission error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_multi_mark_limit_exceeded() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateMultiMarkCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        // Create 10 existing multi-marks (limit)
        for _ in 0..10 {
            let target = player_factory(PlayerFactoryOptions::default());
            mock_uow.players().save(&target).await.unwrap();

            let flag = MapFlag::new_player_flag(
                player.id,
                MapFlagType::PlayerMark,
                5,
                player.id,
            )
            .with_target(target.id);

            mock_uow.map_flags().save(&flag).await.unwrap();
        }

        let target_player = player_factory(PlayerFactoryOptions::default());
        mock_uow.players().save(&target_player).await.unwrap();

        let command = CreateMultiMark {
            player_id: player.id,
            alliance_id: None,
            target_id: target_player.id,
            mark_type: 0,
            color: 5,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::MapFlagLimitExceeded) => {},
            e => panic!("Expected MapFlagLimitExceeded error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_multi_mark_invalid_color() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateMultiMarkCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let target_player = player_factory(PlayerFactoryOptions::default());

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.players().save(&target_player).await.unwrap();

        let command = CreateMultiMark {
            player_id: player.id,
            alliance_id: None,
            target_id: target_player.id,
            mark_type: 0,
            color: 15, // Invalid for multi-marks (should be 0-9)
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::InvalidMapFlagColor { color, min, max }) => {
                assert_eq!(color, 15);
                assert_eq!(min, 0);
                assert_eq!(max, 9);
            },
            e => panic!("Expected InvalidMapFlagColor error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_multi_mark_invalid_type() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = CreateMultiMarkCommandHandler::new();

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.players().save(&player).await.unwrap();

        let command = CreateMultiMark {
            player_id: player.id,
            alliance_id: None,
            target_id: Uuid::new_v4(),
            mark_type: 2, // Invalid - CustomFlag type not allowed for multi-marks
            color: 5,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::InvalidMapFlagType(2)) => {},
            e => panic!("Expected InvalidMapFlagType error, got: {:?}", e),
        }
    }
}
