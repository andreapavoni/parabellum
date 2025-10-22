use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;

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
