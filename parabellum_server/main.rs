use std::sync::Arc;

use parabellum_app::{
    app::AppBus, config::Config, job_registry::AppJobRegistry, jobs::worker::JobWorker,
};
use parabellum_core::{ApplicationError, Result};
use parabellum_db::{establish_connection_pool, uow::PostgresUnitOfWorkProvider};
use parabellum_web::WebRouter;

mod logs;
use logs::setup_logging;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ApplicationError> {
    use parabellum_app::app::AppState;

    setup_logging();
    let (_config, app_bus, worker) = setup_app().await?;

    worker.run();

    let state = AppState {
        app_bus: app_bus,
        // ...config?
    };

    WebRouter::serve(state, 8080).await
}

async fn setup_app() -> Result<(Arc<Config>, Arc<AppBus>, Arc<JobWorker>), ApplicationError> {
    let config = Arc::new(Config::from_env());
    let db_pool = establish_connection_pool().await?;
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
