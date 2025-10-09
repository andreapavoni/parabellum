use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};

#[allow(unused_imports)]
use diesel_async::AsyncConnection;
use dotenvy::dotenv;
use std::env;
#[cfg(test)]
use std::{future::Future, pin::Pin};

pub type DbPool = Pool<AsyncPgConnection>;

pub fn establish_connection_pool() -> DbPool {
    init_connection_pool("DATABASE_URL")
}

pub fn establish_test_connection_pool() -> DbPool {
    init_connection_pool("TEST_DATABASE_URL")
}

fn init_connection_pool(database_env: &'static str) -> DbPool {
    dotenv().ok();

    let database_url = env::var(database_env).expect(&format!("{} must be set", database_env));
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

    Pool::builder(manager)
        .build()
        .expect("Failed to create pool.")
}

// Wrapper to run tests in a transaction using a clean connection
#[cfg(test)]
pub async fn run_test_with_transaction<F>(test: F)
where
    F: for<'a> FnOnce(
        &'a mut AsyncPgConnection,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>,
{
    let pool = establish_test_connection_pool();
    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool");

    conn.begin_test_transaction()
        .await
        .expect("Failed to begin test transaction");

    let fut = test(&mut conn);
    let result = fut.await;

    if let Err(e) = result {
        panic!("Test failed with error: {:?}", e);
    }
}
