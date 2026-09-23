//! Shared, isolated PostgreSQL fixtures (test-utils feature only).
//! TEST_DATABASE_URL selects a maintenance connection whose role needs CREATEDB.
//! Each owner creates and drops only its UUID-named database; no shared tables reset.
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{str::FromStr, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

static DATABASE_SLOTS: std::sync::LazyLock<Arc<Semaphore>> =
    std::sync::LazyLock::new(|| Arc::new(Semaphore::new(4)));

pub struct IsolatedTestDatabase {
    root: PgConnectOptions,
    name: String,
    _permit: OwnedSemaphorePermit,
}

impl IsolatedTestDatabase {
    /// Applies every migration before returning the isolated pool. Retain the owner
    /// for the entire scenario, including any spawned tasks that use the pool.
    pub async fn create() -> Result<(PgPool, Arc<Self>), ApplicationError> {
        let permit = DATABASE_SLOTS
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        let url = std::env::var("TEST_DATABASE_URL")
            .ok()
            .or_else(|| {
                // Read local defaults without modifying process-global environment.
                dotenvy::dotenv_iter()
                    .ok()?
                    .filter_map(Result::ok)
                    .find_map(|(key, value)| (key == "TEST_DATABASE_URL").then_some(value))
            })
            .ok_or_else(|| {
                ApplicationError::Infrastructure(
                    "TEST_DATABASE_URL must be set (role needs CREATEDB)".into(),
                )
            })?;
        let root = PgConnectOptions::from_str(&url).map_err(DbError::Database)?;
        let name = format!("parabellum_test_{}", uuid::Uuid::new_v4().simple());
        // Install cleanup before the first fallible database operation.
        let owner = Arc::new(Self {
            root: root.clone(),
            name: name.clone(),
            _permit: permit,
        });
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(root.clone())
            .await
            .map_err(DbError::Database)?;
        sqlx::query(&format!("CREATE DATABASE \"{name}\""))
            .execute(&admin)
            .await
            .map_err(DbError::Database)?;
        admin.close().await;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(root.database(&name))
            .await
            .map_err(DbError::Database)?;
        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok((pool, owner))
    }
}

impl Drop for IsolatedTestDatabase {
    fn drop(&mut self) {
        let root = self.root.clone();
        let name = self.name.clone();
        // A separate runtime makes cleanup work on panic and after the scenario's
        // Tokio runtime shuts down. Join so test process exit cannot abandon cleanup.
        let cleanup = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime
                .block_on(async move {
                    let pool = PgPoolOptions::new()
                        .max_connections(1)
                        .acquire_timeout(std::time::Duration::from_secs(10))
                        .connect_with(root)
                        .await?;
                    let result =
                        sqlx::query(&format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"))
                            .execute(&pool)
                            .await;
                    pool.close().await;
                    result.map(|_| ())
                })
                .map_err(std::io::Error::other)
        });
        match cleanup.join() {
            Ok(Ok(())) => {}
            result => eprintln!("isolated test database cleanup failed: {result:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn isolated_databases_do_not_share_data_or_advisory_locks() {
        let ((a, a_owner), (b, b_owner)) = tokio::try_join!(
            IsolatedTestDatabase::create(),
            IsolatedTestDatabase::create()
        )
        .unwrap();
        assert_ne!(a_owner.name, b_owner.name);
        sqlx::query("CREATE TABLE isolation_sentinel (id integer)")
            .execute(&a)
            .await
            .unwrap();
        let absent: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('isolation_sentinel')::text")
                .fetch_one(&b)
                .await
                .unwrap();
        assert!(absent.is_none());
        let mut first = a.begin().await.unwrap();
        let mut second = b.begin().await.unwrap();
        for tx in [&mut first, &mut second] {
            let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(12345)")
                .fetch_one(&mut **tx)
                .await
                .unwrap();
            assert!(acquired);
        }
        first.rollback().await.unwrap();
        second.rollback().await.unwrap();
        a.close().await;
        b.close().await;
    }

    #[tokio::test]
    async fn panicking_scenario_drops_its_database() {
        let (pool, owner) = IsolatedTestDatabase::create().await.unwrap();
        let name = owner.name.clone();
        let root = owner.root.clone();
        let task = tokio::spawn(async move {
            let _owner = owner;
            let _pool = pool;
            panic!("intentional fixture cleanup regression");
        });
        assert!(task.await.unwrap_err().is_panic());
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(root)
            .await
            .unwrap();
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
                .bind(name)
                .fetch_one(&admin)
                .await
                .unwrap();
        assert!(!exists);
        admin.close().await;
    }
}
