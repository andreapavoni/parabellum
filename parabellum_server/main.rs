use std::sync::Arc;
use uuid::Uuid;

use parabellum_app::{
    app_bus::AppBus,
    command_handlers::{FoundVillageCommandHandler, RegisterPlayerCommandHandler},
    config::Config,
    cqrs::{
        commands::{FoundVillage, RegisterPlayer},
        queries::GetUnoccupiedValley,
    },
    job_registry::AppJobRegistry,
    jobs::worker::JobWorker,
    queries_handlers::GetUnoccupiedValleyHandler,
};
use parabellum_core::{ApplicationError, Result};
use parabellum_db::{establish_connection_pool, uow::PostgresUnitOfWorkProvider};
use parabellum_types::{common::Player, tribe::Tribe};

mod logs;
use logs::setup_logging;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    setup_logging();

    let config = Arc::new(Config::from_env());
    let db_pool = establish_connection_pool().await?;
    let uow_provider = Arc::new(PostgresUnitOfWorkProvider::new(db_pool));

    // Create the AppBus
    let app_bus = AppBus::new(config.clone(), uow_provider.clone());
    let app_registry = Arc::new(AppJobRegistry::new());
    let worker = Arc::new(JobWorker::new(
        uow_provider.clone(),
        app_registry,
        config.clone(),
    ));
    worker.run();

    tracing::info!("AppBus initialized. Executing use cases via bus");

    // --- Use Case 1: Register Player ---
    let register_player_cmd = RegisterPlayer::new(None, "pavonz_bus".to_string(), Tribe::Roman);
    let register_player_handler = RegisterPlayerCommandHandler::new();
    let player = match app_bus
        .execute(register_player_cmd, register_player_handler)
        .await
    {
        Ok(_) => {
            tracing::info!("Player registered!");

            // Fake player for demo
            Player {
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
    let found_village_handler = FoundVillageCommandHandler::new();
    app_bus
        .execute(found_village_cmd, found_village_handler)
        .await?;
    tracing::info!("Village has been successfully founded!");

    tracing::info!("Done. Application will idle for 60 seconds.");
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    Ok(())
}
