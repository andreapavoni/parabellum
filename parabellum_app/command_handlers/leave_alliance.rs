use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::{AllianceLog, AllianceLogType};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::LeaveAlliance},
    uow::UnitOfWork,
};

pub struct LeaveAllianceCommandHandler {}

impl Default for LeaveAllianceCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LeaveAllianceCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<LeaveAlliance> for LeaveAllianceCommandHandler {
    async fn handle(
        &self,
        command: LeaveAlliance,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let player = uow.players().get_by_id(command.player_id).await?;

        // Verify player is in an alliance
        let alliance_id = player.alliance_id.ok_or(GameError::PlayerNotInAlliance)?;

        let alliance = uow.alliances().get_by_id(alliance_id).await?;

        // Prevent leader from leaving (must transfer leadership first)
        if alliance.is_leader(command.player_id) {
            return Err(GameError::NotAllianceLeader.into());
        }

        // Check if player is the last member
        let member_count = uow.alliances().count_members(alliance_id).await?;

        if member_count == 1 {
            // Last member leaving - delete the alliance
            uow.alliances().delete(alliance_id).await?;
        } else {
            // Remove player from alliance
            uow.players()
                .update_alliance_fields(
                    command.player_id,
                    None,
                    None,
                    None,
                )
                .await?;

            // Log leave
            let log = AllianceLog::new(
                alliance_id,
                AllianceLogType::PlayerLeft,
                Some(format!("Player {} left the alliance", player.username)),
            );
            uow.alliance_logs().save(&log).await?;
        }

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
    async fn test_leave_alliance_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = LeaveAllianceCommandHandler::new();

        // Create leader player
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

        // Create member player
        let mut member = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Gaul),
            ..Default::default()
        });
        member.alliance_id = Some(alliance.id);
        member.alliance_role = Some(0);
        member.alliance_join_time = Some(Utc::now() - Duration::days(20));
        member.current_alliance_training_contributions = 50000;
        member.current_alliance_armor_contributions = 30000;
        member.total_alliance_training_contributions = 100000;
        member.total_alliance_armor_contributions = 75000;

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.players().save(&member).await.unwrap();
        mock_uow_impl.add_alliance_member(leader.clone());
        mock_uow_impl.add_alliance_member(member.clone());

        let command = LeaveAlliance {
            player_id: member.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify member left alliance
        let updated_member = mock_uow_impl.players().get_by_id(member.id).await.unwrap();
        assert_eq!(updated_member.alliance_id, None);
        assert_eq!(updated_member.alliance_role, None);
        assert_eq!(updated_member.alliance_join_time, None);

        // Verify all contribution fields are reset to 0
        assert_eq!(updated_member.current_alliance_training_contributions, 0);
        assert_eq!(updated_member.current_alliance_armor_contributions, 0);
        assert_eq!(updated_member.current_alliance_cp_contributions, 0);
        assert_eq!(updated_member.current_alliance_trade_contributions, 0);
        assert_eq!(updated_member.total_alliance_training_contributions, 0);
        assert_eq!(updated_member.total_alliance_armor_contributions, 0);
        assert_eq!(updated_member.total_alliance_cp_contributions, 0);
        assert_eq!(updated_member.total_alliance_trade_contributions, 0);

        // Verify alliance log was created
        let logs = mock_uow_impl
            .alliance_logs()
            .get_by_alliance_id(alliance.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].data.as_ref().unwrap().contains(&member.username));
        assert_eq!(logs[0].type_, AllianceLogType::PlayerLeft as i16);
    }

    #[tokio::test]
    async fn test_leave_alliance_last_member_deletes_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = LeaveAllianceCommandHandler::new();

        // Create single member (not leader)
        let mut member = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        // Create alliance with a different leader_id (simulating leader already left)
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(), // Different from member.id
        );

        member.alliance_id = Some(alliance.id);
        member.alliance_role = Some(0);
        member.alliance_join_time = Some(Utc::now() - Duration::days(30));

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&member).await.unwrap();
        mock_uow_impl.add_alliance_member(member.clone());

        let command = LeaveAlliance {
            player_id: member.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify alliance was deleted
        let alliance_result = mock_uow_impl.alliances().get_by_id(alliance.id).await;
        assert!(alliance_result.is_err(), "Alliance should be deleted");
    }

    #[tokio::test]
    async fn test_leave_alliance_leader_cannot_leave() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = LeaveAllianceCommandHandler::new();

        // Create leader player
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

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&leader).await.unwrap();
        mock_uow_impl.add_alliance_member(leader.clone());

        // Leader tries to leave
        let command = LeaveAlliance {
            player_id: leader.id,
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
    async fn test_leave_alliance_player_not_in_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = LeaveAllianceCommandHandler::new();

        // Create player not in any alliance
        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        // player.alliance_id = None

        mock_uow.players().save(&player).await.unwrap();

        let command = LeaveAlliance {
            player_id: player.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::PlayerNotInAlliance) => {},
            e => panic!("Expected PlayerNotInAlliance error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_leave_alliance_player_not_found() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = LeaveAllianceCommandHandler::new();

        // Player not saved

        let command = LeaveAlliance {
            player_id: Uuid::new_v4(), // Non-existent player
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Db(_) => {}, // PlayerNotFound error
            e => panic!("Expected Db error (PlayerNotFound), got: {:?}", e),
        }
    }
}
