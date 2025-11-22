use std::sync::Arc;

use parabellum_types::errors::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::{AllianceLog, AllianceLogType, AlliancePermission};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::SetAllianceLeader},
    uow::UnitOfWork,
};

pub struct SetAllianceLeaderCommandHandler {}

impl Default for SetAllianceLeaderCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SetAllianceLeaderCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<SetAllianceLeader> for SetAllianceLeaderCommandHandler {
    async fn handle(
        &self,
        command: SetAllianceLeader,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let mut executor = uow.players().get_by_id(command.player_id).await?;
        let mut new_leader = uow.players().get_by_id(command.new_leader_id).await?;
        let mut alliance = uow.alliances().get_by_id(command.alliance_id).await?;

        // Verify executor is in the alliance
        if executor.alliance_id != Some(command.alliance_id) {
            return Err(GameError::PlayerNotInAlliance.into());
        }

        // Verify new leader is in the same alliance
        if new_leader.alliance_id != Some(command.alliance_id) {
            return Err(GameError::PlayerNotInAlliance.into());
        }

        // Transfer leadership
        alliance.transfer_leadership(command.player_id, command.new_leader_id)?;
        uow.alliances().update(&alliance).await?;

        // Grant new leader full permissions
        // Grant new leader full permissions
        new_leader.update_alliance_role(AlliancePermission::all_permissions());
        uow.players().save(&new_leader).await?;

        // Demote old leader to officer role (can still invite, manage marks, send messages)
        executor.update_alliance_role(AlliancePermission::officer_permissions());
        uow.players().save(&executor).await?;

        // Log leadership transfer
        let log = AllianceLog::new(
            command.alliance_id,
            AllianceLogType::RoleChanged,
            Some(format!(
                "Leadership transferred from {} to {}",
                executor.username, new_leader.username
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
    async fn test_set_alliance_leader_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = SetAllianceLeaderCommandHandler::new();

        // Create current leader
        let mut current_leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            current_leader.id,
        ).unwrap();

        current_leader.alliance_id = Some(alliance.id);
        current_leader.alliance_role = Some(AlliancePermission::all_permissions());
        current_leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        // Create new leader member
        let mut new_leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        new_leader.alliance_id = Some(alliance.id);
        new_leader.alliance_role = Some(0);
        new_leader.alliance_join_time = Some(Utc::now() - Duration::days(20));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&current_leader).await.unwrap();
        mock_uow_impl.players().save(&new_leader).await.unwrap();
        mock_uow_impl.add_alliance_member(current_leader.clone());
        mock_uow_impl.add_alliance_member(new_leader.clone());

        let command = SetAllianceLeader {
            player_id: current_leader.id,
            alliance_id: alliance.id,
            new_leader_id: new_leader.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify alliance leader was updated
        let updated_alliance = mock_uow_impl.alliances().get_by_id(alliance.id).await.unwrap();
        assert_eq!(updated_alliance.leader_id, Some(new_leader.id));

        // Verify new leader has all permissions
        let updated_new_leader = mock_uow_impl.players().get_by_id(new_leader.id).await.unwrap();
        assert_eq!(updated_new_leader.alliance_role, Some(AlliancePermission::all_permissions()));

        // Verify old leader has officer permissions (invite, manage marks, send messages)
        let updated_old_leader = mock_uow_impl.players().get_by_id(current_leader.id).await.unwrap();
        assert_eq!(updated_old_leader.alliance_role, Some(AlliancePermission::officer_permissions()));

        // Verify alliance log was created
        let logs = mock_uow_impl
            .alliance_logs()
            .get_by_alliance_id(alliance.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].data.as_ref().unwrap().contains(&current_leader.username));
        assert!(logs[0].data.as_ref().unwrap().contains(&new_leader.username));
        assert_eq!(logs[0].type_, AllianceLogType::RoleChanged as i16);
    }

    #[tokio::test]
    async fn test_set_alliance_leader_executor_not_in_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = SetAllianceLeaderCommandHandler::new();

        // Create executor not in alliance
        let executor = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        // executor.alliance_id = None

        // Create leader
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        ).unwrap();

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&executor).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();

        let command = SetAllianceLeader {
            player_id: executor.id,
            alliance_id: alliance.id,
            new_leader_id: leader.id,
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
    async fn test_set_alliance_leader_executor_not_leader() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = SetAllianceLeaderCommandHandler::new();

        // Create actual leader
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        ).unwrap();

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        // Create member trying to transfer leadership (not leader)
        let mut member = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        member.alliance_id = Some(alliance.id);
        member.alliance_role = Some(0);
        member.alliance_join_time = Some(Utc::now() - Duration::days(20));

        // Create another member to be new leader
        let mut new_leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Teuton),
            ..Default::default()
        });
        new_leader.alliance_id = Some(alliance.id);
        new_leader.alliance_role = Some(0);
        new_leader.alliance_join_time = Some(Utc::now() - Duration::days(10));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&member).await.unwrap();
        mock_uow_impl.players().save(&new_leader).await.unwrap();

        // Member tries to transfer leadership
        let command = SetAllianceLeader {
            player_id: member.id,
            alliance_id: alliance.id,
            new_leader_id: new_leader.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::NotAllianceLeader) => {},
            e => panic!("Expected NotAllianceLeader error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_set_alliance_leader_new_leader_not_in_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = SetAllianceLeaderCommandHandler::new();

        // Create leader
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        ).unwrap();

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        // Create player not in alliance
        let new_leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        // new_leader.alliance_id = None

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&new_leader).await.unwrap();

        let command = SetAllianceLeader {
            player_id: leader.id,
            alliance_id: alliance.id,
            new_leader_id: new_leader.id,
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
    async fn test_set_alliance_leader_already_leader() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = SetAllianceLeaderCommandHandler::new();

        // Create leader
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        ).unwrap();

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());
        leader.alliance_join_time = Some(Utc::now() - Duration::days(30));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();

        // Leader tries to set themselves as leader
        let command = SetAllianceLeader {
            player_id: leader.id,
            alliance_id: alliance.id,
            new_leader_id: leader.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::PlayerAlreadyLeader) => {},
            e => panic!("Expected PlayerAlreadyLeader error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_set_alliance_leader_executor_not_found() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = SetAllianceLeaderCommandHandler::new();

        // Create leader
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        ).unwrap();

        leader.alliance_id = Some(alliance.id);

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&leader).await.unwrap();

        // Non-existent executor
        let command = SetAllianceLeader {
            player_id: Uuid::new_v4(),
            alliance_id: alliance.id,
            new_leader_id: leader.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Db(_) => {}, // PlayerNotFound error
            e => panic!("Expected Db error (PlayerNotFound), got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_set_alliance_leader_new_leader_not_found() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = SetAllianceLeaderCommandHandler::new();

        // Create leader
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader.id,
        ).unwrap();

        leader.alliance_id = Some(alliance.id);
        leader.alliance_role = Some(AlliancePermission::all_permissions());

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&leader).await.unwrap();

        // Non-existent new leader
        let command = SetAllianceLeader {
            player_id: leader.id,
            alliance_id: alliance.id,
            new_leader_id: Uuid::new_v4(),
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Db(_) => {}, // PlayerNotFound error
            e => panic!("Expected Db error (PlayerNotFound), got: {:?}", e),
        }
    }
}
