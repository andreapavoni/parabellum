use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};

use parabellum_game::models::alliance::AlliancePermission;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::DeleteMapFlag},
    uow::UnitOfWork,
};

pub struct DeleteMapFlagCommandHandler {}

impl Default for DeleteMapFlagCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeleteMapFlagCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<DeleteMapFlag> for DeleteMapFlagCommandHandler {
    async fn handle(
        &self,
        command: DeleteMapFlag,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        // Get player to verify they exist
        let player = uow.players().get_by_id(command.player_id).await?;

        // Get the existing flag
        let flag = uow.map_flags().get_by_id(command.flag_id).await?;

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
            AlliancePermission::verify_permission(&player, AlliancePermission::ManageMarks)?;

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

        // Delete the flag
        uow.map_flags().delete(command.flag_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use parabellum_game::test_utils::{PlayerFactoryOptions, player_factory};
    use parabellum_types::tribe::Tribe;
    use parabellum_types::map::Position;
    use parabellum_game::models::map_flag::{MapFlag, MapFlagType};

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};

    #[tokio::test]
    async fn test_delete_player_flag_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = DeleteMapFlagCommandHandler::new();

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
        .with_text("Test flag".to_string()).unwrap();

        mock_uow.map_flags().save(&flag).await.unwrap();

        let command = DeleteMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id: flag.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify flag was deleted
        let flags = mock_uow.map_flags().get_by_player_id(player.id).await.unwrap();
        assert_eq!(flags.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_alliance_flag_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = DeleteMapFlagCommandHandler::new();

        let alliance_id = Uuid::new_v4();
        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        player.alliance_id = Some(alliance_id);
        player.alliance_role = Some(128); // MANAGE_MARKS permission

        mock_uow.players().save(&player).await.unwrap();

        // Create an alliance flag
        let flag = MapFlag::new_alliance_flag(
            alliance_id,
            MapFlagType::CustomFlag,
            15,
            player.id,
        )
        .with_position(Position { x: 100, y: 50 })
        .with_text("Alliance flag".to_string()).unwrap();

        mock_uow.map_flags().save(&flag).await.unwrap();

        let command = DeleteMapFlag {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            flag_id: flag.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify flag was deleted
        let flags = mock_uow.map_flags().get_by_alliance_id(alliance_id).await.unwrap();
        assert_eq!(flags.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_flag_wrong_owner() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = DeleteMapFlagCommandHandler::new();

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
        .with_text("Test".to_string()).unwrap();

        mock_uow.map_flags().save(&flag).await.unwrap();

        // Try to delete with different player
        let command = DeleteMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id: flag.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::MapFlagInvalidOwnership) => {},
            e => panic!("Expected MapFlagInvalidOwnership error, got: {:?}", e),
        }

        // Verify flag was NOT deleted
        let flags = mock_uow.map_flags().get_by_player_id(other_player.id).await.unwrap();
        assert_eq!(flags.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_alliance_flag_no_permission() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = DeleteMapFlagCommandHandler::new();

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
        .with_text("Test".to_string()).unwrap();

        mock_uow.map_flags().save(&flag).await.unwrap();

        let command = DeleteMapFlag {
            player_id: player.id,
            alliance_id: Some(alliance_id),
            flag_id: flag.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::NoManageMarksPermission) => {},
            e => panic!("Expected NoManageMarksPermission error, got: {:?}", e),
        }

        // Verify flag was NOT deleted
        let flags = mock_uow.map_flags().get_by_alliance_id(alliance_id).await.unwrap();
        assert_eq!(flags.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_multi_mark_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = DeleteMapFlagCommandHandler::new();

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

        let command = DeleteMapFlag {
            player_id: player.id,
            alliance_id: None,
            flag_id: flag.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify flag was deleted
        let flags = mock_uow.map_flags().get_by_player_id(player.id).await.unwrap();
        assert_eq!(flags.len(), 0);
    }
}
