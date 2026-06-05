use mini_cqrs_es::CqrsError;
use sqlx::{PgPool, Postgres, pool::PoolConnection};

#[derive(Debug)]
pub(crate) struct AdvisoryLock {
    key: i64,
    conn: PoolConnection<Postgres>,
}

impl AdvisoryLock {
    pub(crate) async fn try_acquire(pool: &PgPool, key: i64) -> Result<Option<Self>, CqrsError> {
        let mut conn = pool.acquire().await.map_err(CqrsError::domain_source)?;
        let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(key)
            .fetch_one(&mut *conn)
            .await
            .map_err(CqrsError::domain_source)?;

        if acquired {
            Ok(Some(Self { key, conn }))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn release(mut self) -> Result<(), CqrsError> {
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(self.key)
            .execute(&mut *self.conn)
            .await
            .map_err(CqrsError::domain_source)?;
        Ok(())
    }
}
