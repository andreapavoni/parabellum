use sqlx::PgPool;
use std::sync::Arc;

use parabellum_app::{application::GameApplication, config::Config};
use parabellum_infra::identity::IdentityService;
use parabellum_infra::{
    adapters::VillageEsAdapter, bootstrap_world_map, es::EsScheduledActionWorker,
    es::VillageEsService, establish_connection_pool,
};
use parabellum_server::logs::setup_logging;
use parabellum_types::{Result, errors::ApplicationError};
use parabellum_web::{AppState, WebRouter};

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    setup_logging();
    let (config, game_app, es_worker, db_pool) = setup_app().await?;
    let state = AppState::new(game_app, db_pool, &config);

    es_worker.run();
    WebRouter::serve(state, 8080).await
}

async fn setup_app() -> Result<
    (
        Arc<Config>,
        Arc<GameApplication>,
        Arc<EsScheduledActionWorker>,
        PgPool,
    ),
    ApplicationError,
> {
    let config = Arc::new(Config::from_env());
    let db_pool = establish_connection_pool().await?;

    sqlx::migrate!("../migrations")
        .run(&db_pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

    setup_world_map(&db_pool, &config).await?;

    let village_service = VillageEsService::new(db_pool.clone());
    let game_app = build_game_application(db_pool.clone(), config.clone(), village_service.clone());
    let es_worker = Arc::new(EsScheduledActionWorker::new(village_service, 1000));

    Ok((config, game_app, es_worker, db_pool))
}

fn build_game_application(
    db_pool: PgPool,
    config: Arc<Config>,
    village_service: VillageEsService,
) -> Arc<GameApplication> {
    let identity = Arc::new(IdentityService::new(db_pool, config.clone()));
    let villages_adapter = Arc::new(VillageEsAdapter::new(village_service, config));
    Arc::new(GameApplication::new(
        identity,
        villages_adapter.clone(),
        villages_adapter.clone(),
        villages_adapter,
    ))
}

async fn setup_world_map(pool: &PgPool, config: &Config) -> Result<(), ApplicationError> {
    match bootstrap_world_map(pool, config.world_size).await {
        Ok(true) => tracing::info!("World Map successfully bootstrapped."),
        Ok(false) => tracing::info!("World Map already set. Skipping bootstrap."),
        Err(e) => {
            tracing::error!("Error during World Map initialization: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}
