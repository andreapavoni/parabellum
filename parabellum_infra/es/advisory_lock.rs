use mini_cqrs_es::CqrsError;
use sqlx::{PgPool, Postgres, pool::PoolConnection};

/// A session lock whose connection is discarded on errors or cancellation.
#[derive(Debug)]
pub(crate) struct AdvisoryLock {
    key: i64,
    conn: Option<PoolConnection<Postgres>>,
}

impl AdvisoryLock {
    pub(crate) async fn try_acquire(pool: &PgPool, key: i64) -> Result<Option<Self>, CqrsError> {
        // Install the guard before the query: cancellation can occur after the
        // server acquired the lock but before the client received the result.
        let mut guard = Self {
            key,
            conn: Some(pool.acquire().await.map_err(CqrsError::domain_source)?),
        };
        let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(key)
            .fetch_one(&mut **guard.conn.as_mut().unwrap())
            .await
            .map_err(CqrsError::domain_source)?;
        if acquired {
            Ok(Some(guard))
        } else {
            guard.conn.take();
            Ok(None)
        }
    }

    pub(crate) async fn release(mut self) -> Result<(), CqrsError> {
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(self.key)
            .execute(&mut **self.conn.as_mut().unwrap())
            .await
            .map_err(CqrsError::domain_source)?;
        self.conn.take();
        Ok(())
    }
}

impl Drop for AdvisoryLock {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.as_mut() {
            // SQLx closes this session instead of returning a locked connection
            // to the pool. This also handles a cancelled unlock operation.
            conn.close_on_drop();
        }
    }
}
