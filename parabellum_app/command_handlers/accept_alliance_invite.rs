use std::sync::Arc;
use chrono::Utc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_types::buildings::BuildingName;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::AcceptAllianceInvite},
    uow::UnitOfWork,
};
use parabellum_game::models::alliance::{AllianceLog, AllianceLogType};

pub struct AcceptAllianceInviteCommandHandler {}

impl Default for AcceptAllianceInviteCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AcceptAllianceInviteCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<AcceptAllianceInvite> for AcceptAllianceInviteCommandHandler {
    async fn handle(
        &self,
        command: AcceptAllianceInvite,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let alliance_repo = uow.alliances();
        let alliance_invite_repo = uow.alliance_invites();
        let player_repo = uow.players();
        let village_repo = uow.villages();
        let alliance_log_repo = uow.alliance_logs();

        // Verify alliance and player exist
        let alliance = alliance_repo.get_by_id(command.alliance_id).await?;
        let player = player_repo.get_by_id(command.player_id).await?;

        // Verify player is not already in an alliance
        if player.alliance_id.is_some() {
            return Err(GameError::PlayerAlreadyInAlliance.into());
        }

        // Verify invitation exists for this player and alliance
        let invites = alliance_invite_repo.get_by_player_id(command.player_id).await?;
        let invite = invites
            .iter()
            .find(|i| i.alliance_id == command.alliance_id)
            .ok_or_else(|| GameError::InvitationNotFound)?;

        // Check alliance has available slots
        let current_members = alliance_repo.count_members(command.alliance_id).await?;
        if current_members >= alliance.max_members as i64 {
            return Err(GameError::AllianceFull.into());
        }

        // Verify player has Embassy level 3+
        let capital = village_repo.get_capital_by_player_id(command.player_id).await?;
        let embassy = capital
            .get_building_by_name(&BuildingName::Embassy)
            .ok_or_else(|| {
                GameError::BuildingNotFound(BuildingName::Embassy)
            })?;

        if embassy.building.level < 3 {
            return Err(GameError::BuildingRequirementsNotMet {
                building: BuildingName::Embassy,
                level: 3,
            }
            .into());
        }

        // Update player's alliance_id and alliance_join_time
        player_repo
            .update_alliance_fields(
                command.player_id,
                Some(command.alliance_id),
                Some(0), // Set alliance_role to 0 (no permissions initially)
                Some(Utc::now()),
            )
            .await?;

        // Delete invitation from alliance_invite
        alliance_invite_repo.delete(invite.id).await?;

        // Log join event to alliance_log
        let log = AllianceLog::new(
            command.alliance_id,
            AllianceLogType::PlayerJoined,
            Some(format!("Player {} joined the alliance", player.username)),
        );
        alliance_log_repo.save(&log).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parabellum_game::test_utils::{
        PlayerFactoryOptions, VillageFactoryOptions, player_factory, valley_factory,
        village_factory,
    };
    use parabellum_types::{buildings::BuildingName, tribe::Tribe};

    use super::*;
    use crate::{config::Config, test_utils::tests::MockUnitOfWork};
    use parabellum_game::models::alliance::{Alliance, AllianceInvite};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_accept_alliance_invite_success() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = AcceptAllianceInviteCommandHandler::new();

        // Create alliance
        let leader_id = Uuid::new_v4();
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader_id,
        );

        // Create player being invited
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

        // Create invitation
        let invite = AllianceInvite::new(leader_id, alliance.id, player.id);

        // Save to mock repos
        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&player).await.unwrap();
        mock_uow_impl.villages().save(&village).await.unwrap();
        mock_uow_impl.alliance_invites().save(&invite).await.unwrap();

        // Add leader to alliance member count
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        leader.alliance_id = Some(alliance.id);
        mock_uow_impl.add_alliance_member(leader.clone());

        let command = AcceptAllianceInvite {
            player_id: player.id,
            alliance_id: alliance.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command.clone(), &mock_uow, &config).await;
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().map(|e| e.to_string())
        );

        // Verify player was updated
        let updated_player = mock_uow_impl.players().get_by_id(player.id).await.unwrap();
        assert_eq!(updated_player.alliance_id, Some(alliance.id));
        assert_eq!(updated_player.alliance_role, Some(0)); // No permissions initially
        assert!(updated_player.alliance_join_time.is_some());

        // Verify invitation was deleted
        let invites = mock_uow_impl.alliance_invites().get_by_player_id(player.id).await.unwrap();
        assert_eq!(invites.len(), 0);

        // Verify alliance log was created
        let logs = mock_uow_impl.alliance_logs()
            .get_by_alliance_id(alliance.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].type_, AllianceLogType::PlayerJoined as i16);
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_player_already_in_alliance() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = AcceptAllianceInviteCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        );

        let mut player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        // Player already in an alliance
        player.alliance_id = Some(Uuid::new_v4());

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&player).await.unwrap();

        let command = AcceptAllianceInvite {
            player_id: player.id,
            alliance_id: alliance.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::PlayerAlreadyInAlliance) => {},
            e => panic!("Expected PlayerAlreadyInAlliance error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_no_invitation() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = AcceptAllianceInviteCommandHandler::new();

        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            Uuid::new_v4(),
        );

        let player = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&player).await.unwrap();
        // Note: No invitation created

        let command = AcceptAllianceInvite {
            player_id: player.id,
            alliance_id: alliance.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::InvitationNotFound) => {},
            e => panic!("Expected InvitationNotFound error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_alliance_full() {
        let config = Arc::new(Config::from_env());
        let mock_uow_impl = MockUnitOfWork::new();
        let handler = AcceptAllianceInviteCommandHandler::new();

        let leader_id = Uuid::new_v4();
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            1, // max_members = 1 (only leader)
            leader_id,
        );

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

        let embassy = parabellum_game::models::buildings::Building::new(
            BuildingName::Embassy,
            config.speed,
        )
        .at_level(3, config.speed)
        .unwrap();
        village.add_building_at_slot(embassy, 20).unwrap();

        let invite = AllianceInvite::new(leader_id, alliance.id, player.id);

        mock_uow_impl.alliances().save(&alliance).await.unwrap();
        mock_uow_impl.players().save(&player).await.unwrap();
        mock_uow_impl.villages().save(&village).await.unwrap();
        mock_uow_impl.alliance_invites().save(&invite).await.unwrap();

        // Add leader to fill the alliance
        let mut leader = player_factory(PlayerFactoryOptions {
            tribe: Some(Tribe::Roman),
            ..Default::default()
        });
        leader.alliance_id = Some(alliance.id);
        mock_uow_impl.add_alliance_member(leader.clone());

        let command = AcceptAllianceInvite {
            player_id: player.id,
            alliance_id: alliance.id,
        };

        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(mock_uow_impl.clone());
        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::AllianceFull) => {},
            e => panic!("Expected AllianceFull error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_no_embassy() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = AcceptAllianceInviteCommandHandler::new();

        let leader_id = Uuid::new_v4();
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader_id,
        );

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

        let invite = AllianceInvite::new(leader_id, alliance.id, player.id);

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();
        mock_uow.alliance_invites().save(&invite).await.unwrap();

        let command = AcceptAllianceInvite {
            player_id: player.id,
            alliance_id: alliance.id,
        };

        let result = handler.handle(command, &mock_uow, &config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::Game(GameError::BuildingNotFound(BuildingName::Embassy)) => {},
            e => panic!("Expected BuildingNotFound(Embassy) error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_accept_alliance_invite_embassy_level_too_low() {
        let config = Arc::new(Config::from_env());
        let mock_uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
        let handler = AcceptAllianceInviteCommandHandler::new();

        let leader_id = Uuid::new_v4();
        let alliance = Alliance::new(
            "Test Alliance".to_string(),
            "TEST".to_string(),
            5,
            leader_id,
        );

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

        let invite = AllianceInvite::new(leader_id, alliance.id, player.id);

        mock_uow.alliances().save(&alliance).await.unwrap();
        mock_uow.players().save(&player).await.unwrap();
        mock_uow.villages().save(&village).await.unwrap();
        mock_uow.alliance_invites().save(&invite).await.unwrap();

        let command = AcceptAllianceInvite {
            player_id: player.id,
            alliance_id: alliance.id,
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
}
