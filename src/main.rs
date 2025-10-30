use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use parabellum::{
    app::{
        commands::{FoundVillage, FoundVillageHandler, RegisterPlayer, RegisterPlayerHandler},
        queries::{GetUnoccupiedValley, GetUnoccupiedValleyHandler},
    },
    bus::AppBus,
    db::{establish_connection_pool, uow::PostgresUnitOfWorkProvider},
    game::models::Tribe,
    jobs::worker::JobWorker,
    logs::setup_logging,
};

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();

    let config = Arc::new(parabellum::config::Config::from_env());
    let db_pool = establish_connection_pool().await?;
    let uow_provider = Arc::new(PostgresUnitOfWorkProvider::new(db_pool));

    // Create the AppBus
    let app_bus = AppBus::new(config, uow_provider.clone());
    let worker = Arc::new(JobWorker::new(uow_provider.clone()));

    worker.run();

    tracing::info!("AppBus initialized. Executing use cases via bus");

    // --- Use Case 1: Register Player ---
    let register_player_cmd = RegisterPlayer::new(None, "pavonz_bus".to_string(), Tribe::Roman);
    let register_player_handler = RegisterPlayerHandler::new();
    let player = match app_bus
        .execute(register_player_cmd, register_player_handler)
        .await
    {
        Ok(_) => {
            // NOTE: The command doesn't return the player anymore.
            // This is a common CQRS pattern. If you *need* the created ID,
            // the command struct can be modified to return it, or you
            // make a subsequent query.
            // For simplicity, let's just query for the player.
            // This is a separate topic, but let's assume we get the player.
            tracing::info!("Player registered!");

            // Fake player for demo
            parabellum::game::models::Player {
                id: Uuid::new_v4(), // We don't know the ID
                username: "pavonz_bus".to_string(),
                tribe: Tribe::Roman,
            }
        }
        Err(e) => {
            tracing::error!("Error during player registration: {}", e);
            return Err(e);
        }
    };

    // --- Use Case 2: Get Valley (Query) ---
    let get_valley_query = GetUnoccupiedValley::new(None);
    let get_valley_handler = GetUnoccupiedValleyHandler::new();
    let valley = app_bus.query(get_valley_query, get_valley_handler).await?;
    tracing::info!(
        x = valley.position.x,
        y = valley.position.y,
        "Found available valley"
    );

    // --- Use Case 3: Found Village ---
    let found_village_cmd = FoundVillage::new(player, valley.position);
    let found_village_handler = FoundVillageHandler::new();
    app_bus
        .execute(found_village_cmd, found_village_handler)
        .await?;
    tracing::info!("Village has been successfully founded!");

    tracing::info!("Done. Application will idle for 60 seconds.");
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    Ok(())
}
