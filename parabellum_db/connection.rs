use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;

use parabellum_core::DbError;

pub type DbPool = PgPool;

pub async fn establish_connection_pool() -> Result<DbPool, DbError> {
    Ok(init_connection_pool("DATABASE_URL").await?)
}

pub async fn establish_test_connection_pool() -> Result<DbPool, DbError> {
    Ok(init_connection_pool("TEST_DATABASE_URL").await?)
}

async fn init_connection_pool(database_env: &'static str) -> Result<DbPool, DbError> {
    dotenvy::dotenv().ok();

    let database_url =
        env::var(database_env).unwrap_or_else(|_| panic!("{} must be set", database_env));

    Ok(PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?)
}
