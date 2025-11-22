use parabellum_db::bootstrap_world_map;
use sqlx::PgPool;
use std::sync::Arc;

use parabellum_app::{
    app::AppBus, config::Config, job_registry::AppJobRegistry, jobs::worker::JobWorker,
};
use parabellum_core::{ApplicationError, Result};
use parabellum_db::{establish_connection_pool, uow::PostgresUnitOfWorkProvider};
use parabellum_web::AppState;
use parabellum_web::WebRouter;

mod logs;
use logs::setup_logging;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    setup_logging();
    let (config, app_bus, worker) = setup_app().await?;
    let state = AppState::new(app_bus, &config);

    worker.run();
    WebRouter::serve(state, 8080).await
}

async fn setup_app() -> Result<(Arc<Config>, Arc<AppBus>, Arc<JobWorker>), ApplicationError> {
    let config = Arc::new(Config::from_env());
    let db_pool = establish_connection_pool().await?;

    sqlx::migrate!("../migrations")
        .run(&db_pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

    setup_world_map(&db_pool, &config).await?;

    let uow_provider = Arc::new(PostgresUnitOfWorkProvider::new(db_pool));
    let app_bus = Arc::new(AppBus::new(config.clone(), uow_provider.clone()));
    let app_registry = Arc::new(AppJobRegistry::new());
    let worker = Arc::new(JobWorker::new(
        uow_provider.clone(),
        app_registry,
        config.clone(),
    ));

    Ok((config, app_bus, worker))
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
