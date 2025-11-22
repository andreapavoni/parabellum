use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::{
    AllianceLog, AllianceLogType, AlliancePermission, verify_permission,
};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::AcceptAllianceDiplomacy},
    uow::UnitOfWork,
};

pub struct AcceptAllianceDiplomacyCommandHandler {}

impl Default for AcceptAllianceDiplomacyCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AcceptAllianceDiplomacyCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<AcceptAllianceDiplomacy> for AcceptAllianceDiplomacyCommandHandler {
    async fn handle(
        &self,
        command: AcceptAllianceDiplomacy,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        // Load the player
        let player = uow.players().get_by_id(command.player_id).await?;

        // Verify the player has diplomacy permission
        verify_permission(&player, AlliancePermission::AllianceDiplomacy)?;

        // Verify player is in an alliance
        let player_alliance_id = player
            .alliance_id
            .ok_or(GameError::PlayerNotInAlliance)?;

        // Load the diplomacy
        let mut diplomacy = uow
            .alliance_diplomacy()
            .get_by_id(command.diplomacy_id)
            .await?
            .ok_or(GameError::DiplomacyNotFound)?;

        // Verify the diplomacy involves the player's alliance
        if diplomacy.alliance1_id != player_alliance_id
            && diplomacy.alliance2_id != player_alliance_id
        {
            return Err(GameError::DiplomacyNotFound.into());
        }

        // Verify the diplomacy is pending
        if !diplomacy.is_pending() {
            return Err(GameError::DiplomacyAlreadyProcessed.into());
        }

        // Accept the diplomacy
        diplomacy.accept();
        uow.alliance_diplomacy().update(&diplomacy).await?;

        // Log the acceptance in both alliances
        let other_alliance_id = if diplomacy.alliance1_id == player_alliance_id {
            diplomacy.alliance2_id
        } else {
            diplomacy.alliance1_id
        };

        let log_acceptor = AllianceLog::new(
            player_alliance_id,
            AllianceLogType::DiplomacyAccepted,
            Some(format!(
                "Accepted diplomacy with alliance {}",
                other_alliance_id
            )),
        );
        uow.alliance_logs().save(&log_acceptor).await?;

        let log_proposer = AllianceLog::new(
            other_alliance_id,
            AllianceLogType::DiplomacyAccepted,
            Some(format!(
                "Alliance {} accepted diplomacy",
                player_alliance_id
            )),
        );
        uow.alliance_logs().save(&log_proposer).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::tests::MockUnitOfWork;
    use parabellum_game::models::{alliance::AllianceDiplomacy, player::Player};
    use parabellum_types::{alliance::DiplomacyType, tribe::Tribe};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_accept_alliance_diplomacy_requires_permission() {
        let mock_uow = MockUnitOfWork::new();

        let player_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();
        let diplomacy_id = Uuid::new_v4();

        // Create player WITHOUT diplomacy permission
        let player = Player {
            id: player_id,
            username: "testplayer".to_string(),
            tribe: Tribe::Roman,
            user_id: Uuid::new_v4(),
            alliance_id: Some(alliance2_id),
            alliance_role: None, // No permission
            alliance_join_time: None,
            current_alliance_recruitment_contributions: 0,
            current_alliance_metallurgy_contributions: 0,
            current_alliance_philosophy_contributions: 0,
            current_alliance_commerce_contributions: 0,
            total_alliance_recruitment_contributions: 0,
            total_alliance_metallurgy_contributions: 0,
            total_alliance_philosophy_contributions: 0,
            total_alliance_commerce_contributions: 0,
        };

        mock_uow.players().save(&player).await.unwrap();

        let command = AcceptAllianceDiplomacy {
            player_id,
            diplomacy_id,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow);
        let handler = AcceptAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::NoDiplomacyPermission)
        ));
    }

    #[tokio::test]
    async fn test_accept_alliance_diplomacy_requires_player_in_alliance() {
        let mock_uow = MockUnitOfWork::new();

        let player_id = Uuid::new_v4();
        let diplomacy_id = Uuid::new_v4();

        // Create player WITH permission but NOT in alliance
        let player = Player {
            id: player_id,
            username: "testplayer".to_string(),
            tribe: Tribe::Roman,
            user_id: Uuid::new_v4(),
            alliance_id: None, // Not in alliance
            alliance_role: Some(AlliancePermission::AllianceDiplomacy as i16),
            alliance_join_time: None,
            current_alliance_recruitment_contributions: 0,
            current_alliance_metallurgy_contributions: 0,
            current_alliance_philosophy_contributions: 0,
            current_alliance_commerce_contributions: 0,
            total_alliance_recruitment_contributions: 0,
            total_alliance_metallurgy_contributions: 0,
            total_alliance_philosophy_contributions: 0,
            total_alliance_commerce_contributions: 0,
        };

        mock_uow.players().save(&player).await.unwrap();

        let command = AcceptAllianceDiplomacy {
            player_id,
            diplomacy_id,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow);
        let handler = AcceptAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::PlayerNotInAlliance)
        ));
    }

    #[tokio::test]
    async fn test_accept_alliance_diplomacy_rejects_already_processed() {
        let mock_uow = MockUnitOfWork::new();

        let player_id = Uuid::new_v4();
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        // Create player with permission and in alliance
        let player = Player {
            id: player_id,
            username: "testplayer".to_string(),
            tribe: Tribe::Roman,
            user_id: Uuid::new_v4(),
            alliance_id: Some(alliance2_id),
            alliance_role: Some(AlliancePermission::AllianceDiplomacy as i16),
            alliance_join_time: None,
            current_alliance_recruitment_contributions: 0,
            current_alliance_metallurgy_contributions: 0,
            current_alliance_philosophy_contributions: 0,
            current_alliance_commerce_contributions: 0,
            total_alliance_recruitment_contributions: 0,
            total_alliance_metallurgy_contributions: 0,
            total_alliance_philosophy_contributions: 0,
            total_alliance_commerce_contributions: 0,
        };

        // Create already-accepted diplomacy
        let mut diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);
        diplomacy.accept(); // Already accepted

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.alliance_diplomacy().save(&diplomacy).await.unwrap();

        let command = AcceptAllianceDiplomacy {
            player_id,
            diplomacy_id: diplomacy.id,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow);
        let handler = AcceptAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::DiplomacyAlreadyProcessed)
        ));
    }

    #[tokio::test]
    async fn test_accept_alliance_diplomacy_success() {
        let mock_uow = MockUnitOfWork::new();

        let player_id = Uuid::new_v4();
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        // Create player with permission and in alliance
        let player = Player {
            id: player_id,
            username: "testplayer".to_string(),
            tribe: Tribe::Roman,
            user_id: Uuid::new_v4(),
            alliance_id: Some(alliance2_id),
            alliance_role: Some(AlliancePermission::AllianceDiplomacy as i16),
            alliance_join_time: None,
            current_alliance_recruitment_contributions: 0,
            current_alliance_metallurgy_contributions: 0,
            current_alliance_philosophy_contributions: 0,
            current_alliance_commerce_contributions: 0,
            total_alliance_recruitment_contributions: 0,
            total_alliance_metallurgy_contributions: 0,
            total_alliance_philosophy_contributions: 0,
            total_alliance_commerce_contributions: 0,
        };

        // Create pending diplomacy
        let diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.alliance_diplomacy().save(&diplomacy).await.unwrap();

        let command = AcceptAllianceDiplomacy {
            player_id,
            diplomacy_id: diplomacy.id,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow.clone());
        let handler = AcceptAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_ok());

        // Verify diplomacy was accepted
        let updated_diplomacy = mock_uow
            .alliance_diplomacy()
            .get_by_id(diplomacy.id)
            .await
            .unwrap()
            .unwrap();
        assert!(updated_diplomacy.is_accepted());
    }
}
