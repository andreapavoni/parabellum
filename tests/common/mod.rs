use parabellum::{
    app::App,
    config::Config,
    db::{establish_test_connection_pool, repository::PostgresRepository, DbPool},
};
use std::sync::Arc;

/// Sets up a test environment with a real database connection pool,
/// a concrete repository, and an `App` instance.
pub async fn setup_test_env() -> (App, Arc<PostgresRepository>, DbPool) {
    let db_pool = establish_test_connection_pool();
    let repo = Arc::new(PostgresRepository::new(db_pool.clone()));
    let config = Arc::new(Config::from_env());

    let app = App::new(
        config,
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
        repo.clone(),
    );

    (app, repo, db_pool)
}
