use anyhow::Result;
use parabellum::{
    app::{
        commands::{FoundVillage, RegisterPlayer},
        queries::GetUnoccupiedValley,
        App,
    },
    db::{establish_connection_pool, uow::PostgresUnitOfWorkProvider},
    game::models::Tribe,
    jobs::worker::JobWorker,
    logs::setup_logging,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();

    let config = Arc::new(parabellum::config::Config::from_env());
    let db_pool = establish_connection_pool().await?;
    let uow_provider = Arc::new(PostgresUnitOfWorkProvider::new(db_pool));
    let app = App::new(config, uow_provider.clone());
    let worker = Arc::new(JobWorker::new(uow_provider.clone())); // Anche il worker usa il provider

    worker.run();

    tracing::info!("App initialized. Executing a use case");

    let register_player_cmd = RegisterPlayer::new(None, "pavonz".to_string(), Tribe::Roman);
    let player = match app.register_player(register_player_cmd).await {
        Ok(p) => {
            tracing::info!("Player  '{}' successfully registered!", p.username);
            p
        }
        Err(e) => {
            tracing::error!("Error during player registration: {}", e);
            return Err(e);
        }
    };

    let get_valley_query = GetUnoccupiedValley::new(None);
    let valley = app.get_unoccupied_valley(get_valley_query).await?;
    tracing::info!(
        x = valley.position.x,
        y = valley.position.y,
        "Found available valley"
    );

    let found_village_cmd = FoundVillage::new(player, valley.position);
    let village = app.found_village(found_village_cmd).await?;
    tracing::info!(
        village_id = %village.id,
        village_name = %village.name,
        "Village has been successfully founded!"
    );
    tracing::info!("Done. Application will idle for 60 seconds.");
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    Ok(())
}
