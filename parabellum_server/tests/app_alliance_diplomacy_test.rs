mod test_utils;

use std::sync::Arc;

use parabellum_app::{
    command_handlers::{
        AcceptAllianceDiplomacyCommandHandler, CreateAllianceDiplomacyCommandHandler,
        DeclineAllianceDiplomacyCommandHandler,
    },
    config::Config,
    cqrs::{
        commands::{AcceptAllianceDiplomacy, CreateAllianceDiplomacy, DeclineAllianceDiplomacy},
        CommandHandler,
    },
    uow::UnitOfWorkProvider,
};
use parabellum_core::{ApplicationError, GameError};
use parabellum_game::models::{
    alliance::{Alliance, AllianceDiplomacy, AlliancePermission},
    player::Player,
};
use parabellum_types::{alliance::DiplomacyType, tribe::Tribe};

use test_utils::tests::*;

/// Test context for alliance diplomacy tests
struct DiplomacyTestContext {
    uow_provider: Arc<dyn UnitOfWorkProvider>,
    alliance1: Alliance,
    alliance2: Alliance,
    player1: Player,
    player2: Player,
}

/// Sets up a complete diplomacy test context with two alliances and their players
async fn setup_diplomacy_context(
    tribe1: Tribe,
    tribe2: Tribe,
    player1_has_perm: bool,
    player2_has_perm: bool,
) -> Result<DiplomacyTestContext, ApplicationError> {
    let uow_provider = setup_test_uow_provider().await;

    // Setup two players with their parties
    let (player1, _, _, _, _) =
        setup_player_party(uow_provider.clone(), None, tribe1, [0; 10], false).await?;
    let (player2, _, _, _, _) =
        setup_player_party(uow_provider.clone(), None, tribe2, [0; 10], false).await?;

    // Create two alliances
    let alliance1 = setup_alliance(uow_provider.clone(), player1.id).await?;
    let alliance2 = setup_alliance_with_name(
        uow_provider.clone(),
        player2.id,
        "Alliance 2".to_string(),
        "AL2".to_string(),
    )
    .await?;

    // Assign players to their alliances
    let player1 = assign_player_to_alliance(uow_provider.clone(), player1, alliance1.id).await?;
    let player2 = assign_player_to_alliance(uow_provider.clone(), player2, alliance2.id).await?;

    // Set diplomacy permissions if needed
    if player1_has_perm || player2_has_perm {
        let uow = uow_provider.begin().await?;

        if player1_has_perm {
            let mut p1 = uow.players().get_by_id(player1.id).await?;
            p1.alliance_role = Some(AlliancePermission::AllianceDiplomacy as i16);
            uow.players().save(&p1).await?;
        }

        if player2_has_perm {
            let mut p2 = uow.players().get_by_id(player2.id).await?;
            p2.alliance_role = Some(AlliancePermission::AllianceDiplomacy as i16);
            uow.players().save(&p2).await?;
        }

        uow.commit().await?;
    }

    Ok(DiplomacyTestContext {
        uow_provider,
        alliance1,
        alliance2,
        player1,
        player2,
    })
}

#[tokio::test]
async fn test_create_alliance_diplomacy_success() -> Result<(), ApplicationError> {
    let ctx = setup_diplomacy_context(Tribe::Roman, Tribe::Gaul, true, false).await?;

    let command = CreateAllianceDiplomacy {
        proposer_player_id: ctx.player1.id,
        target_alliance_id: ctx.alliance2.id,
        diplomacy_type: DiplomacyType::NAP,
    };

    let uow = ctx.uow_provider.begin().await?;
    let handler = CreateAllianceDiplomacyCommandHandler::new();
    let config = Arc::new(Config::from_env());
    handler.handle(command, &uow, &config).await?;
    uow.commit().await?;

    // Verify diplomacy was created
    let uow = ctx.uow_provider.begin().await?;
    let diplomacy = uow
        .alliance_diplomacy()
        .get_between_alliances(ctx.alliance1.id, ctx.alliance2.id)
        .await?;
    uow.commit().await?;

    assert!(diplomacy.is_some());
    let diplomacy = diplomacy.unwrap();
    assert_eq!(diplomacy.alliance1_id, ctx.alliance1.id);
    assert_eq!(diplomacy.alliance2_id, ctx.alliance2.id);
    assert!(diplomacy.is_pending());

    Ok(())
}

#[tokio::test]
async fn test_create_alliance_diplomacy_no_permission() -> Result<(), ApplicationError> {
    let ctx = setup_diplomacy_context(Tribe::Roman, Tribe::Gaul, false, false).await?;

    let command = CreateAllianceDiplomacy {
        proposer_player_id: ctx.player1.id,
        target_alliance_id: ctx.alliance2.id,
        diplomacy_type: DiplomacyType::NAP,
    };

    let uow = ctx.uow_provider.begin().await?;
    let handler = CreateAllianceDiplomacyCommandHandler::new();
    let config = Arc::new(Config::from_env());
    let result = handler.handle(command, &uow, &config).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ApplicationError::Game(GameError::NoDiplomacyPermission)
    ));

    Ok(())
}

#[tokio::test]
async fn test_accept_alliance_diplomacy_success() -> Result<(), ApplicationError> {
    let ctx = setup_diplomacy_context(Tribe::Roman, Tribe::Gaul, false, true).await?;

    // Create diplomacy proposal
    let diplomacy = AllianceDiplomacy::new(ctx.alliance1.id, ctx.alliance2.id, DiplomacyType::NAP);
    let uow = ctx.uow_provider.begin().await?;
    uow.alliance_diplomacy().save(&diplomacy).await?;
    uow.commit().await?;

    // Accept diplomacy
    let command = AcceptAllianceDiplomacy {
        player_id: ctx.player2.id,
        diplomacy_id: diplomacy.id,
    };

    let uow = ctx.uow_provider.begin().await?;
    let handler = AcceptAllianceDiplomacyCommandHandler::new();
    let config = Arc::new(Config::from_env());
    handler.handle(command, &uow, &config).await?;
    uow.commit().await?;

    // Verify diplomacy was accepted
    let uow = ctx.uow_provider.begin().await?;
    let updated_diplomacy = uow
        .alliance_diplomacy()
        .get_by_id(diplomacy.id)
        .await?
        .unwrap();
    uow.commit().await?;

    assert!(updated_diplomacy.is_accepted());

    Ok(())
}

#[tokio::test]
async fn test_decline_alliance_diplomacy_success() -> Result<(), ApplicationError> {
    let ctx = setup_diplomacy_context(Tribe::Roman, Tribe::Gaul, false, true).await?;

    // Create diplomacy proposal
    let diplomacy = AllianceDiplomacy::new(ctx.alliance1.id, ctx.alliance2.id, DiplomacyType::Alliance);
    let uow = ctx.uow_provider.begin().await?;
    uow.alliance_diplomacy().save(&diplomacy).await?;
    uow.commit().await?;

    // Decline diplomacy
    let command = DeclineAllianceDiplomacy {
        player_id: ctx.player2.id,
        diplomacy_id: diplomacy.id,
    };

    let uow = ctx.uow_provider.begin().await?;
    let handler = DeclineAllianceDiplomacyCommandHandler::new();
    let config = Arc::new(Config::from_env());
    handler.handle(command, &uow, &config).await?;
    uow.commit().await?;

    // Verify diplomacy was declined
    let uow = ctx.uow_provider.begin().await?;
    let updated_diplomacy = uow
        .alliance_diplomacy()
        .get_by_id(diplomacy.id)
        .await?
        .unwrap();
    uow.commit().await?;

    assert!(updated_diplomacy.is_declined());

    Ok(())
}

#[tokio::test]
async fn test_accept_diplomacy_already_processed() -> Result<(), ApplicationError> {
    let ctx = setup_diplomacy_context(Tribe::Roman, Tribe::Gaul, false, true).await?;

    // Create and immediately accept diplomacy
    let mut diplomacy = AllianceDiplomacy::new(ctx.alliance1.id, ctx.alliance2.id, DiplomacyType::NAP);
    diplomacy.accept();
    let uow = ctx.uow_provider.begin().await?;
    uow.alliance_diplomacy().save(&diplomacy).await?;
    uow.commit().await?;

    // Try to accept already-accepted diplomacy
    let command = AcceptAllianceDiplomacy {
        player_id: ctx.player2.id,
        diplomacy_id: diplomacy.id,
    };

    let uow = ctx.uow_provider.begin().await?;
    let handler = AcceptAllianceDiplomacyCommandHandler::new();
    let config = Arc::new(Config::from_env());
    let result = handler.handle(command, &uow, &config).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ApplicationError::Game(GameError::DiplomacyAlreadyProcessed)
    ));

    Ok(())
}
