use std::sync::Arc;

use parabellum_core::{ApplicationError, GameError, Result};
use parabellum_game::models::alliance::{
    AllianceDiplomacy, AllianceLog, AllianceLogType, AlliancePermission, verify_permission,
};

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::CreateAllianceDiplomacy},
    uow::UnitOfWork,
};

pub struct CreateAllianceDiplomacyCommandHandler {}

impl Default for CreateAllianceDiplomacyCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateAllianceDiplomacyCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl CommandHandler<CreateAllianceDiplomacy> for CreateAllianceDiplomacyCommandHandler {
    async fn handle(
        &self,
        command: CreateAllianceDiplomacy,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        // Load the proposer player
        let proposer = uow.players().get_by_id(command.proposer_player_id).await?;

        // Verify the player has diplomacy permission
        verify_permission(&proposer, AlliancePermission::AllianceDiplomacy)?;

        // Verify player is in an alliance
        let proposer_alliance_id = proposer
            .alliance_id
            .ok_or(GameError::PlayerNotInAlliance)?;

        // Verify target alliance exists
        let _target_alliance = uow
            .alliances()
            .get_by_id(command.target_alliance_id)
            .await?;

        // Check if diplomacy already exists between these alliances
        let existing_diplomacy = uow
            .alliance_diplomacy()
            .get_between_alliances(proposer_alliance_id, command.target_alliance_id)
            .await?;

        if existing_diplomacy.is_some() {
            return Err(GameError::DiplomacyAlreadyExists.into());
        }

        // Create new diplomacy proposal
        let diplomacy = AllianceDiplomacy::new(
            proposer_alliance_id,
            command.target_alliance_id,
            command.diplomacy_type,
        );

        // Save the diplomacy
        uow.alliance_diplomacy().save(&diplomacy).await?;

        // Log the diplomacy proposal in both alliances
        let log_proposer = AllianceLog::new(
            proposer_alliance_id,
            AllianceLogType::DiplomacyProposed,
            Some(format!(
                "Proposed diplomacy with alliance {}",
                command.target_alliance_id
            )),
        );
        uow.alliance_logs().save(&log_proposer).await?;

        let log_target = AllianceLog::new(
            command.target_alliance_id,
            AllianceLogType::DiplomacyProposed,
            Some(format!(
                "Received diplomacy proposal from alliance {}",
                proposer_alliance_id
            )),
        );
        uow.alliance_logs().save(&log_target).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::tests::MockUnitOfWork;
    use parabellum_game::models::{alliance::Alliance, player::Player};
    use parabellum_types::{alliance::DiplomacyType, tribe::Tribe};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_alliance_diplomacy_requires_permission() {
        let mock_uow = MockUnitOfWork::new();

        let player_id = Uuid::new_v4();
        let alliance1_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

        // Create player WITHOUT diplomacy permission
        let player = Player {
            id: player_id,
            username: "testplayer".to_string(),
            tribe: Tribe::Roman,
            user_id: Uuid::new_v4(),
            alliance_id: Some(alliance1_id),
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

        let command = CreateAllianceDiplomacy {
            proposer_player_id: player_id,
            target_alliance_id: alliance2_id,
            diplomacy_type: DiplomacyType::NAP,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow);
        let handler = CreateAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::NoDiplomacyPermission)
        ));
    }

    #[tokio::test]
    async fn test_create_alliance_diplomacy_requires_player_in_alliance() {
        let mock_uow = MockUnitOfWork::new();

        let player_id = Uuid::new_v4();
        let alliance2_id = Uuid::new_v4();

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

        let command = CreateAllianceDiplomacy {
            proposer_player_id: player_id,
            target_alliance_id: alliance2_id,
            diplomacy_type: DiplomacyType::NAP,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow);
        let handler = CreateAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::PlayerNotInAlliance)
        ));
    }

    #[tokio::test]
    async fn test_create_alliance_diplomacy_rejects_existing_diplomacy() {
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
            alliance_id: Some(alliance1_id),
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

        // Create both alliances with matching IDs
        let mut alliance1 = Alliance::new(
            "Alliance 1".to_string(),
            "AL1".to_string(),
            60,
            player_id,
        ).unwrap();
        alliance1.id = alliance1_id;

        let mut alliance2 = Alliance::new(
            "Alliance 2".to_string(),
            "AL2".to_string(),
            60,
            Uuid::new_v4(),
        ).unwrap();
        alliance2.id = alliance2_id;

        // Create existing diplomacy
        let existing_diplomacy = AllianceDiplomacy::new(alliance1_id, alliance2_id, DiplomacyType::NAP);

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.alliances().save(&alliance1).await.unwrap();
        mock_uow.alliances().save(&alliance2).await.unwrap();
        mock_uow.alliance_diplomacy().save(&existing_diplomacy).await.unwrap();

        let command = CreateAllianceDiplomacy {
            proposer_player_id: player_id,
            target_alliance_id: alliance2_id,
            diplomacy_type: DiplomacyType::Alliance,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow);
        let handler = CreateAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::Game(GameError::DiplomacyAlreadyExists)
        ));
    }

    #[tokio::test]
    async fn test_create_alliance_diplomacy_success() {
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
            alliance_id: Some(alliance1_id),
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

        // Create both alliances with matching IDs
        let mut alliance1 = Alliance::new(
            "Alliance 1".to_string(),
            "AL1".to_string(),
            60,
            player_id,
        ).unwrap();
        alliance1.id = alliance1_id;

        let mut alliance2 = Alliance::new(
            "Alliance 2".to_string(),
            "AL2".to_string(),
            60,
            Uuid::new_v4(),
        ).unwrap();
        alliance2.id = alliance2_id;

        mock_uow.players().save(&player).await.unwrap();
        mock_uow.alliances().save(&alliance1).await.unwrap();
        mock_uow.alliances().save(&alliance2).await.unwrap();

        let command = CreateAllianceDiplomacy {
            proposer_player_id: player_id,
            target_alliance_id: alliance2_id,
            diplomacy_type: DiplomacyType::NAP,
        };

        let uow: Box<dyn UnitOfWork> = Box::new(mock_uow.clone());
        let handler = CreateAllianceDiplomacyCommandHandler::new();
        let config = Arc::new(Config::from_env());
        let result = handler.handle(command, &uow, &config).await;

        assert!(result.is_ok());

        // Verify diplomacy was saved
        let saved_diplomacy = mock_uow
            .alliance_diplomacy()
            .get_between_alliances(alliance1_id, alliance2_id)
            .await
            .unwrap();
        assert!(saved_diplomacy.is_some());
        let diplomacy = saved_diplomacy.unwrap();
        assert!(diplomacy.is_pending());
        assert_eq!(diplomacy.alliance1_id, alliance1_id);
        assert_eq!(diplomacy.alliance2_id, alliance2_id);
    }
}
