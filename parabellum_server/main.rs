use sqlx::PgPool;
use std::sync::Arc;

use parabellum_app::{
    app::AppBus, config::Config, job_registry::AppJobRegistry, jobs::worker::JobWorker,
};
use parabellum_db::{
    bootstrap_world_map, establish_connection_pool, toasty_db::establish_toasty_db,
    uow::ToastyUnitOfWorkProvider,
};
use parabellum_types::{Result, errors::ApplicationError};
use parabellum_web::{AppState, WebRouter};

mod logs;
use logs::setup_logging;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    setup_logging();
    let (config, app_bus, worker, db_pool) = setup_app().await?;
    let state = AppState::new(app_bus, db_pool, &config);

    worker.run();
    WebRouter::serve(state, 8080).await
}

async fn setup_app() -> Result<(Arc<Config>, Arc<AppBus>, Arc<JobWorker>, PgPool), ApplicationError>
{
    let config = Arc::new(Config::from_env());
    let db_pool = establish_connection_pool().await?;

    sqlx::migrate!("../migrations")
        .run(&db_pool)
        .await
        .map_err(|e| ApplicationError::Unknown(e.to_string()))?;

    setup_world_map(&db_pool, &config).await?;

    let toasty_db = establish_toasty_db().await?;
    let uow_provider = Arc::new(ToastyUnitOfWorkProvider::new(toasty_db, db_pool.clone()));
    let app_bus = Arc::new(AppBus::new(config.clone(), uow_provider.clone()));
    let app_registry = Arc::new(AppJobRegistry::new());
    let worker = Arc::new(JobWorker::new(
        uow_provider.clone(),
        app_registry,
        config.clone(),
    ));

    Ok((config, app_bus, worker, db_pool))
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
