mod test_utils;

use parabellum_app::{command_handlers::TrainUnitsCommandHandler, cqrs::commands::TrainUnits};
use parabellum_game::models::buildings::Building;
use parabellum_types::{
    Result,
    army::{TroopSet, UnitName},
    buildings::BuildingName,
    common::ResourceGroup,
    errors::{ApplicationError, GameError},
    tribe::Tribe,
};

use crate::test_utils::tests::{setup_app, setup_player_party};

/// Helper to set up a village with palace and necessary buildings for expansion units
async fn setup_village_for_expansion_training(
    uow_provider: &std::sync::Arc<dyn parabellum_app::uow::UnitOfWorkProvider>,
    player_id: uuid::Uuid,
    village: &mut parabellum_game::models::village::Village,
    palace_level: u8,
    with_senator_research: bool,
    config: &std::sync::Arc<parabellum_app::config::Config>,
) -> Result<()> {
    let uow = uow_provider.tx().await?;

    // Add Rally Point (required for senator)
    let rally_point =
        Building::new(BuildingName::RallyPoint, config.speed).at_level(10, config.speed)?;
    village.add_building_at_slot(rally_point, 39)?;

    // Add Academy (required for senator)
    let academy = Building::new(BuildingName::Academy, config.speed).at_level(20, config.speed)?;
    village.add_building_at_slot(academy, 22)?;

    // Add Granary (required for resources)
    let granary = Building::new(BuildingName::Granary, config.speed).at_level(20, config.speed)?;
    village.add_building_at_slot(granary, 30)?;

    // Add Warehouse (required for resources)
    let warehouse =
        Building::new(BuildingName::Warehouse, config.speed).at_level(20, config.speed)?;
    village.add_building_at_slot(warehouse, 31)?;

    // Add Palace
    let palace =
        Building::new(BuildingName::Palace, config.speed).at_level(palace_level, config.speed)?;
    village.add_building_at_slot(palace, 25)?;

    // Add plenty of resources
    village.store_resources(&ResourceGroup(100000, 100000, 100000, 100000));

    // Research senator if requested
    if with_senator_research {
        village.set_academy_research_for_test(&UnitName::Senator, true);
    }

    // Give player enough Culture Points for expansion
    let mut player = uow.players().get_by_id(player_id).await?;
    player.culture_points = 100000; // Enough for many villages
    uow.players().save(&player).await?;

    uow.villages().save(&village).await?;
    uow.commit().await?;

    Ok(())
}

/// Test that settlers can be trained up to 3 per available slot
#[tokio::test]
async fn test_train_settlers_within_limit() -> Result<()> {
    let (app, _worker, uow_provider, config) = setup_app(false).await?;
    let (player, mut village, _, _, _) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    setup_village_for_expansion_training(
        &uow_provider,
        player.id,
        &mut village,
        10,
        false,
        &config,
    )
    .await?;

    let settler_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Settler)
        .unwrap() as u8;

    // Train 3 settlers (max for 1 slot)
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: settler_idx,
        quantity: 3,
        building_name: BuildingName::Palace,
    };

    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(
        result.is_ok(),
        "Should be able to train 3 settlers with 1 available slot: {:?}",
        result.err()
    );

    // Verify job was created
    let uow = uow_provider.tx().await?;
    let jobs = uow.jobs().list_by_player_id(player.id).await?;
    assert_eq!(jobs.len(), 1, "Should have 1 training job");
    uow.rollback().await?;

    Ok(())
}

/// Test that training more than 3 settlers per slot fails
#[tokio::test]
async fn test_train_settlers_exceeds_limit() -> Result<()> {
    let (app, _worker, uow_provider, config) = setup_app(false).await?;
    let (player, mut village, _, _, _) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    setup_village_for_expansion_training(
        &uow_provider,
        player.id,
        &mut village,
        10,
        false,
        &config,
    )
    .await?;

    let settler_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Settler)
        .unwrap() as u8;

    // Try to train 4 settlers (exceeds max of 3 for 1 slot)
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: settler_idx,
        quantity: 4,
        building_name: BuildingName::Palace,
    };

    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(
        result.is_err(),
        "Should fail when training more than 3 settlers per slot"
    );

    match result {
        Err(ApplicationError::Game(GameError::SettlerLimitExceeded {
            max,
            current,
            requested,
        })) => {
            assert_eq!(max, 3, "Max should be 3 for 1 slot");
            assert_eq!(current, 0, "No settlers committed yet");
            assert_eq!(requested, 4, "Requested 4 settlers");
        }
        other => panic!("Expected SettlerLimitExceeded error, got: {:?}", other),
    }

    Ok(())
}

/// Test that chiefs can be trained up to 1 per available slot
#[tokio::test]
async fn test_train_chief_within_limit() -> Result<()> {
    let (app, _worker, uow_provider, config) = setup_app(false).await?;
    let (player, mut village, _, _, _) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    setup_village_for_expansion_training(&uow_provider, player.id, &mut village, 10, true, &config)
        .await?;

    let senator_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Senator)
        .unwrap() as u8;

    // Train 1 senator (max for 1 slot)
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: senator_idx,
        quantity: 1,
        building_name: BuildingName::Palace,
    };

    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(
        result.is_ok(),
        "Should be able to train 1 senator with 1 available slot: {:?}",
        result.err()
    );

    Ok(())
}

/// Test that training more than 1 chief per slot fails
#[tokio::test]
async fn test_train_chief_exceeds_limit() -> Result<()> {
    let (app, _worker, uow_provider, config) = setup_app(false).await?;
    let (player, mut village, _, _, _) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    setup_village_for_expansion_training(&uow_provider, player.id, &mut village, 10, true, &config)
        .await?;

    let senator_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Senator)
        .unwrap() as u8;

    // Try to train 2 senators (exceeds max of 1 for 1 slot)
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: senator_idx,
        quantity: 2,
        building_name: BuildingName::Palace,
    };

    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(
        result.is_err(),
        "Should fail when training more than 1 chief per slot"
    );

    match result {
        Err(ApplicationError::Game(GameError::ChiefLimitExceeded {
            max,
            current,
            requested,
        })) => {
            assert_eq!(max, 1, "Max should be 1 for 1 slot");
            assert_eq!(current, 0, "No chiefs committed yet");
            assert_eq!(requested, 2, "Requested 2 chiefs");
        }
        other => panic!("Expected ChiefLimitExceeded error, got: {:?}", other),
    }

    Ok(())
}

/// Test that settlers can't be trained when a chief is using the slot
#[tokio::test]
async fn test_train_settlers_blocked_by_chief() -> Result<()> {
    let (app, _worker, uow_provider, config) = setup_app(false).await?;
    let (player, mut village, _, _, _) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    setup_village_for_expansion_training(&uow_provider, player.id, &mut village, 10, true, &config)
        .await?;

    // First, train a senator
    let senator_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Senator)
        .unwrap() as u8;
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: senator_idx,
        quantity: 1,
        building_name: BuildingName::Palace,
    };
    app.execute(command, TrainUnitsCommandHandler::new())
        .await?;

    // Now try to train settlers - should fail because the 1 slot is used by the chief
    let settler_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Settler)
        .unwrap() as u8;
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: settler_idx,
        quantity: 1,
        building_name: BuildingName::Palace,
    };

    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(
        result.is_err(),
        "Should fail when chief is using the only available slot"
    );

    match result {
        Err(ApplicationError::Game(GameError::SettlerLimitExceeded { max, .. })) => {
            assert_eq!(max, 0, "Max should be 0 when chief uses the slot");
        }
        other => panic!(
            "Expected SettlerLimitExceeded error with max=0, got: {:?}",
            other
        ),
    }

    Ok(())
}

/// Test that chiefs can't be trained when settlers are using the slot
#[tokio::test]
async fn test_train_chief_blocked_by_settlers() -> Result<()> {
    let (app, _worker, uow_provider, config) = setup_app(false).await?;
    let (player, mut village, _, _, _) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    setup_village_for_expansion_training(&uow_provider, player.id, &mut village, 10, true, &config)
        .await?;

    // First, train 3 settlers
    let settler_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Settler)
        .unwrap() as u8;
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: settler_idx,
        quantity: 3,
        building_name: BuildingName::Palace,
    };
    app.execute(command, TrainUnitsCommandHandler::new())
        .await?;

    // Now try to train a chief - should fail because the 1 slot is used by settlers
    let senator_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Senator)
        .unwrap() as u8;
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: senator_idx,
        quantity: 1,
        building_name: BuildingName::Palace,
    };

    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(
        result.is_err(),
        "Should fail when settlers are using the only available slot"
    );

    match result {
        Err(ApplicationError::Game(GameError::ChiefLimitExceeded { max, .. })) => {
            assert_eq!(max, 0, "Max should be 0 when settlers use the slot");
        }
        other => panic!(
            "Expected ChiefLimitExceeded error with max=0, got: {:?}",
            other
        ),
    }

    Ok(())
}

/// Test training with multiple foundation slots
#[tokio::test]
async fn test_train_with_multiple_slots() -> Result<()> {
    let (app, _worker, uow_provider, config) = setup_app(false).await?;
    let (player, mut village, _, _, _) = setup_player_party(
        uow_provider.clone(),
        None,
        Tribe::Roman,
        TroopSet::default(),
        false,
    )
    .await?;

    setup_village_for_expansion_training(&uow_provider, player.id, &mut village, 20, true, &config)
        .await?;

    // Train 1 chief (uses 1 slot, 2 remaining)
    let senator_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Senator)
        .unwrap() as u8;
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: senator_idx,
        quantity: 1,
        building_name: BuildingName::Palace,
    };
    app.execute(command, TrainUnitsCommandHandler::new())
        .await?;

    // Train 6 settlers (uses 2 slots: 3 settlers per slot)
    let settler_idx = village
        .tribe
        .get_unit_idx_by_name(&UnitName::Settler)
        .unwrap() as u8;
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: settler_idx,
        quantity: 6,
        building_name: BuildingName::Palace,
    };
    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(
        result.is_ok(),
        "Should be able to train 6 settlers with 2 remaining slots: {:?}",
        result.err()
    );

    // Try to train more settlers - should fail (all 3 slots used)
    let command = TrainUnits {
        player_id: player.id,
        village_id: village.id,
        unit_idx: settler_idx,
        quantity: 1,
        building_name: BuildingName::Palace,
    };
    let result = app.execute(command, TrainUnitsCommandHandler::new()).await;
    assert!(result.is_err(), "Should fail when all slots are used");

    Ok(())
}
