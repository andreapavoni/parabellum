use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::{AllianceLog, AllianceLogType, AlliancePermission};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::KickFromAlliance},
    uow::UnitOfWork,
};

pub struct KickFromAllianceCommandHandler {}

impl Default for KickFromAllianceCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl KickFromAllianceCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<KickFromAlliance> for KickFromAllianceCommandHandler {
    async fn handle(
        &self,
        command: KickFromAlliance,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let kicker = uow.players().get_by_id(command.player_id).await?;
        let target_player = uow.players().get_by_id(command.target_player_id).await?;

        // Verify kicker is in the alliance
        if kicker.alliance_id != Some(command.alliance_id) {
            return Err(GameError::PlayerNotInAlliance.into());
        }

        // Verify kicker has permission to kick players
        AlliancePermission::verify_permission(&kicker, AlliancePermission::KickPlayer)?;

        // Verify target player is in the same alliance
        if target_player.alliance_id != Some(command.alliance_id) {
            return Err(GameError::PlayerNotInAlliance.into());
        }

        // Verify target is not the alliance leader
        let leader = uow.alliances().get_leader(command.alliance_id).await?;
        if target_player.id == leader.id {
            return Err(GameError::CannotKickLeader.into());
        }

        // Remove player from alliance
        uow.players()
            .update_alliance_fields(
                command.target_player_id,
                None,
                None,
                None,
            )
            .await?;

        // Log kick
        let log = AllianceLog::new(
            command.alliance_id,
            AllianceLogType::PlayerKicked,
            Some(format!(
                "Player {} kicked by {}",
                target_player.username, kicker.username
            )),
        );
        uow.alliance_logs().save(&log).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Utc, Duration};
    use parabellum_game::test_utils::{
        PlayerFactoryOptions, player_factory,
    };
    use parabellum_types::tribe::Tribe;

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};
    use parabellum_game::models::alliance::{Alliance, AlliancePermission};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_kick_from_alliance_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = KickFromAllianceCommandHandler::new();

        // Create leader player first
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        // Create alliance with leader's actual ID
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        );

        // Set leader's alliance fields
        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30)); // Earliest

        // Create kicker player with KickPlayer permission
        let mut kicker = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        kicker.alliance_id = Some(alliance.id);
        kicker.alliance_role = Some(AlliancePermission::KickPlayer as i32);
        kicker.alliance_join_time = Some(Utc::now() - Duration::days(20));

        // Create target player
        let mut target = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        target.alliance_id = Some(alliance.id);
        target.alliance_role = Some(0);
        target.alliance_join_time = Some(Utc::now() - Duration::days(10));
        target.current_alliance_training_contributions = 60000;
        target.current_alliance_cp_contributions = 40000;
        target.total_alliance_training_contributions = 150000;
        target.total_alliance_trade_contributions = 200000;

        // Save to mock repos
        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&kicker).await.unwrap();
        mock_uow_impl.players().save(&target).await.unwrap();

        // Set leader in alliance
        mock_uow_impl.add_alliance_member(leader.clone());

        let command = KickFromAlliance {
            player_id: kicker.id,
            alliance_id: alliance.id,
            target_player_id: target.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify target player was kicked
        let updated_target = mock_uow_impl.players().get_by_id(target.id).await.unwrap();
        assert_eq!(updated_target.alliance_id, None);
        assert_eq!(updated_target.alliance_role, None);
        assert_eq!(updated_target.alliance_join_time, None);

        // Verify all contribution fields are reset to 0
        assert_eq!(updated_target.current_alliance_training_contributions, 0);
        assert_eq!(updated_target.current_alliance_armor_contributions, 0);
        assert_eq!(updated_target.current_alliance_cp_contributions, 0);
        assert_eq!(updated_target.current_alliance_trade_contributions, 0);
        assert_eq!(updated_target.total_alliance_training_contributions, 0);
        assert_eq!(updated_target.total_alliance_armor_contributions, 0);
        assert_eq!(updated_target.total_alliance_cp_contributions, 0);
        assert_eq!(updated_target.total_alliance_trade_contributions, 0);

        // Verify alliance log was created
        let logs = mock_uow_impl
            .alliance_logs()
            .get_by_alliance_id(alliance.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].data.as_ref().unwrap().contains(&target.username));
        assert!(logs[0].data.as_ref().unwrap().contains(&kicker.username));
        assert_eq!(logs[0].type_, AllianceLogType::PlayerKicked as i16);
    }

    #[tokio::test]
    async fn test_kick_from_alliance_no_permission() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = KickFromAllianceCommandHandler::new();

        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        );

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        // Create kicker without KickPlayer permission
        let mut kicker = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        kicker.alliance_id = Some(alliance.id);
        kicker.alliance_role = Some(0); // No permissions
        kicker.alliance_join_time = Some(Utc::now() - Duration::days(20));

        let mut target = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        target.alliance_id = Some(alliance.id);
        target.alliance_role = Some(0);
        target.alliance_join_time = Some(Utc::now() - Duration::days(10));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&kicker).await.unwrap();
        mock_uow_impl.players().save(&target).await.unwrap();
        mock_uow_impl.add_alliance_member(leader.clone());

        let command = KickFromAlliance {
            player_id: kicker.id,
            alliance_id: alliance.id,
            target_player_id: target.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::NoKickPermission) => {},
            e => panic!("Expected NoKickPermission error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_kick_from_alliance_kicker_not_in_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = KickFromAllianceCommandHandler::new();

        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        );

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        // Kicker is not in the alliance
        let kicker = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        // kicker.alliance_id = None (not in alliance)

        let mut target = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        target.alliance_id = Some(alliance.id);
        target.alliance_role = Some(0);
        target.alliance_join_time = Some(Utc::now() - Duration::days(10));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&kicker).await.unwrap();
        mock_uow_impl.players().save(&target).await.unwrap();
        mock_uow_impl.add_alliance_member(leader.clone());

        let command = KickFromAlliance {
            player_id: kicker.id,
            alliance_id: alliance.id,
            target_player_id: target.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::PlayerNotInAlliance) => {},
            e => panic!("Expected PlayerNotInAlliance error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_kick_from_alliance_target_not_in_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = KickFromAllianceCommandHandler::new();

        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        );

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        let mut kicker = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        kicker.alliance_id = Some(alliance.id);
        kicker.alliance_role = Some(AlliancePermission::KickPlayer as i32);
        kicker.alliance_join_time = Some(Utc::now() - Duration::days(20));

        // Target is not in the alliance
        let target = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        // target.alliance_id = None

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&kicker).await.unwrap();
        mock_uow_impl.players().save(&target).await.unwrap();
        mock_uow_impl.add_alliance_member(leader.clone());

        let command = KickFromAlliance {
            player_id: kicker.id,
            alliance_id: alliance.id,
            target_player_id: target.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::PlayerNotInAlliance) => {},
            e => panic!("Expected PlayerNotInAlliance error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_kick_from_alliance_cannot_kick_leader() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = KickFromAllianceCommandHandler::new();

        // Create leader player first
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        // Create alliance with leader's actual ID
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        );

        // Set leader's alliance fields
        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30)); // Earliest join time = leader

        // Create kicker with kick permission
        let mut kicker = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        kicker.alliance_id = Some(alliance.id);
        kicker.alliance_role = Some(AlliancePermission::KickPlayer as i32);
        kicker.alliance_join_time = Some(Utc::now() - Duration::days(20));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&kicker).await.unwrap();
        mock_uow_impl.add_alliance_member(leader.clone());

        // Try to kick the leader
        let command = KickFromAlliance {
            player_id: kicker.id,
            alliance_id: alliance.id,
            target_player_id: leader.id, // Trying to kick leader
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::CannotKickLeader) => {},
            e => panic!("Expected CannotKickLeader error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_kick_from_alliance_target_not_found() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = KickFromAllianceCommandHandler::new();

        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        );

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        let mut kicker = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        kicker.alliance_id = Some(alliance.id);
        kicker.alliance_role = Some(AlliancePermission::KickPlayer as i32);
        kicker.alliance_join_time = Some(Utc::now() - Duration::days(20));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&kicker).await.unwrap();
        mock_uow_impl.add_alliance_member(leader.clone());
        // Note: target player not saved

        let command = KickFromAlliance {
            player_id: kicker.id,
            alliance_id: alliance.id,
            target_player_id: Uuid::new_v4(), // Non-existent player
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Db(_) => {}, // PlayerNotFound error
            e => panic!("Expected Db error (PlayerNotFound), got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_kick_from_alliance_kicker_not_found() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = KickFromAllianceCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        );

        let target = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&target).await.unwrap();
        // Note: kicker not saved

        let command = KickFromAlliance {
            player_id: Uuid::new_v4(), // Non-existent kicker
            alliance_id: alliance.id,
            target_player_id: target.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Db(_) => {}, // PlayerNotFound error
            e => panic!("Expected Db error (PlayerNotFound), got: {:?}", e),
        }
    }
}
