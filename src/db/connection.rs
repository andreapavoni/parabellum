use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::sync::Arc;
use std::{future::Future, pin::Pin};

use crate::db::repository::PostgresRepository;

pub type DbPool = PgPool;

pub async fn establish_connection_pool() -> Result<DbPool, sqlx::Error> {
    init_connection_pool("DATABASE_URL").await
}

pub async fn establish_test_connection_pool() -> Result<DbPool, sqlx::Error> {
    init_connection_pool("TEST_DATABASE_URL").await
}

async fn init_connection_pool(database_env: &'static str) -> Result<DbPool, sqlx::Error> {
    dotenvy::dotenv().ok();

    let database_url = env::var(database_env).expect(&format!("{} must be set", database_env));

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}

// Wrapper to run tests in a transaction using a clean connection
pub async fn run_test_with_transaction<F>(test: F)
where
    F: for<'a> FnOnce(
        &'a mut sqlx::Transaction<'static, sqlx::Postgres>,
        Arc<PostgresRepository>, // Add this parameter
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>,
{
    let pool = establish_test_connection_pool()
        .await
        .expect("Failed to create test pool");

    // Create the repository with the main pool
    let repo = Arc::new(PostgresRepository::new(pool.clone()));

    let mut tx = pool
        .begin()
        .await
        .expect("Failed to begin test transaction");

    // Pass the transaction AND the repository to the test closure
    let fut = test(&mut tx, repo);
    let result = fut.await;

    if let Err(e) = result {
        // Rollback on failure
        tx.rollback()
            .await
            .expect("Failed to rollback test transaction on error");
        panic!("Test failed with error: {:?}", e);
    }

    // Rollback on success to keep the database clean
    tx.rollback()
        .await
        .expect("Failed to rollback test transaction");
}
