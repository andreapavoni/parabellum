use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::{AllianceInvite, AlliancePermission, AllianceLog, AllianceLogType, verify_permission};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::InviteToAlliance},
    uow::UnitOfWork,
};

pub struct InviteToAllianceCommandHandler {}

impl Default for InviteToAllianceCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InviteToAllianceCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<InviteToAlliance> for InviteToAllianceCommandHandler {
    async fn handle(
        &self,
        command: InviteToAlliance,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let inviter = uow.players().get_by_id(command.player_id).await?;
        let target_player = uow.players().get_by_id(command.target_player_id).await?;

        // Verify permissions
        verify_permission(&inviter, AlliancePermission::InvitePlayer)?;

        // Check if invitation already exists
        let existing_invites = uow
            .alliance_invites()
            .get_by_alliance_id(command.alliance_id)
            .await?;
        if existing_invites
            .iter()
            .any(|i| i.to_player_id == command.target_player_id)
        {
            return Err(GameError::InvitationAlreadyExists.into());
        }

        let invite = AllianceInvite::new(
            command.player_id,
            command.alliance_id,
            command.target_player_id,
        );
        uow.alliance_invites().save(&invite).await?;

        let log = AllianceLog::new(
            command.alliance_id,
            AllianceLogType::PlayerJoined,
            Some(format!(
                "Invitation sent to {} by {}",
                target_player.username, inviter.username
            )),
        );
        uow.alliance_logs().save(&log).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, player_factory,
    };
    use parabellum_types::tribe::Tribe;

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};
    use parabellum_game::models::alliance::{Alliance, AllianceInvite, AlliancePermission};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_invite_to_alliance_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = InviteToAllianceCommandHandler::new();

        // Create alliance
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        ).unwrap();

        // Create inviter player with InvitePlayer permission
        let mut inviter = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        inviter.alliance_id = Some(alliance.id);
        inviter.alliance_role = Some(AlliancePermission::InvitePlayer as i32);

        // Create target player
        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        // Save to mock repos
        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&inviter).await.unwrap();
        mock_uow_impl.players().save(&target_player).await.unwrap();

        let command = InviteToAlliance {
            player_id: inviter.id,
            alliance_id: alliance.id,
            target_player_id: target_player.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify invitation was created
        let invites = mock_uow_impl
            .alliance_invites()
            .get_by_alliance_id(alliance.id)
            .await
            .unwrap();
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0].alliance_id, alliance.id);
        assert_eq!(invites[0].to_player_id, target_player.id);
        assert_eq!(invites[0].from_player_id, inviter.id);

        // Verify alliance log was created
        let logs = mock_uow_impl
            .alliance_logs()
            .get_by_alliance_id(alliance.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].data.as_ref().unwrap().contains(&target_player.username));
        assert!(logs[0].data.as_ref().unwrap().contains(&inviter.username));
    }

    #[tokio::test]
    async fn test_invite_to_alliance_no_permission() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = InviteToAllianceCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        ).unwrap();

        // Create inviter without InvitePlayer permission
        let mut inviter = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        inviter.alliance_id = Some(alliance.id);
        inviter.alliance_role = Some(0); // No permissions

        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&inviter).await.unwrap();
        mock_uow.players().save(&target_player).await.unwrap();

        let command = InviteToAlliance {
            player_id: inviter.id,
            alliance_id: alliance.id,
            target_player_id: target_player.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::NoInvitePermission) => {},
            e => panic!("Expected NoInvitePermission error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_invite_to_alliance_already_invited() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = InviteToAllianceCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        ).unwrap();

        let mut inviter = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        inviter.alliance_id = Some(alliance.id);
        inviter.alliance_role = Some(AlliancePermission::InvitePlayer as i32);

        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        // Create existing invitation
        let existing_invite = AllianceInvite::new(
            inviter.id,
            alliance.id,
            target_player.id,
        );

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&inviter).await.unwrap();
        mock_uow_impl.players().save(&target_player).await.unwrap();
        mock_uow_impl.alliance_invites().save(&existing_invite).await.unwrap();

        let command = InviteToAlliance {
            player_id: inviter.id,
            alliance_id: alliance.id,
            target_player_id: target_player.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::InvitationAlreadyExists) => {},
            e => panic!("Expected InvitationAlreadyExists error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_invite_to_alliance_target_player_not_found() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = InviteToAllianceCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        ).unwrap();

        let mut inviter = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        inviter.alliance_id = Some(alliance.id);
        inviter.alliance_role = Some(AlliancePermission::InvitePlayer as i32);

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&inviter).await.unwrap();
        // Note: target_player not saved

        let command = InviteToAlliance {
            player_id: inviter.id,
            alliance_id: alliance.id,
            target_player_id: Uuid::new_v4(), // Non-existent player
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Db(_) => {}, // PlayerNotFound error
            e => panic!("Expected Db error (PlayerNotFound), got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_invite_to_alliance_inviter_not_found() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = InviteToAllianceCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        ).unwrap();

        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&target_player).await.unwrap();
        // Note: inviter not saved

        let command = InviteToAlliance {
            player_id: Uuid::new_v4(), // Non-existent inviter
            alliance_id: alliance.id,
            target_player_id: target_player.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Db(_) => {}, // PlayerNotFound error
            e => panic!("Expected Db error (PlayerNotFound), got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_invite_to_alliance_with_all_permissions() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = InviteToAllianceCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        ).unwrap();

        // Create inviter with all permissions (leader)
        let mut inviter = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        inviter.alliance_id = Some(alliance.id);
        inviter.alliance_role = Some(AlliancePermission::all_permissions()); // All permissions

        let target_player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&inviter).await.unwrap();
        mock_uow_impl.players().save(&target_player).await.unwrap();

        let command = InviteToAlliance {
            player_id: inviter.id,
            alliance_id: alliance.id,
            target_player_id: target_player.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully with all permissions: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify invitation was created
        let invites = mock_uow_impl
            .alliance_invites()
            .get_by_alliance_id(alliance.id)
            .await
            .unwrap();
        assert_eq!(invites.len(), 1);
    }
}
