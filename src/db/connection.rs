use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use dotenvy::dotenv;
use std::env;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection_pool() -> DbPool {
    init_connection_pool("DATABASE_URL")
}

#[cfg(test)]
pub fn establish_test_connection_pool() -> DbPool {
    init_connection_pool("TEST_DATABASE_URL")
}

// Wrapper to run tests in a transaction using a clean connection
#[cfg(test)]
pub fn run_test_with_transaction<T>(test: T) -> ()
where
    T: FnOnce(&mut PgConnection) -> Result<(), diesel::result::Error>,
{
    use diesel::Connection;

    let pool = establish_test_connection_pool();
    let mut conn = pool
        .get()
        .expect("Failed to get a connection from the pool");

    // Esegui il test dentro una transazione che verrà sempre annullata (rollback)
    conn.test_transaction(|conn| test(conn));
}

fn init_connection_pool(database_env: &'static str) -> DbPool {
    dotenv().ok();

    let binding = env::var(database_env).expect(format!("{} must be set", database_env).as_str());
    let database_url = binding.as_str();

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect(format!("Failed to create test pool for: {}", database_url).as_str())
}
