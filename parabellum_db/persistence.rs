use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;

use parabellum_types::errors::{ApplicationError, DbError};

pub type SharedTx<'a> = Arc<Mutex<Transaction<'a, Postgres>>>;

pub async fn begin_transaction(pool: &PgPool) -> Result<SharedTx<'_>, ApplicationError> {
    let tx = pool.begin().await.map_err(map_sqlx_error)?;
    Ok(Arc::new(Mutex::new(tx)))
}

pub async fn commit_transaction<'a>(tx: SharedTx<'a>) -> Result<(), ApplicationError> {
    let mutex = Arc::try_unwrap(tx).map_err(|_| {
        ApplicationError::Db(DbError::Transaction(
            "transaction still has multiple owners".to_string(),
        ))
    })?;

    mutex.into_inner().commit().await.map_err(map_sqlx_error)
}

pub async fn rollback_transaction<'a>(tx: SharedTx<'a>) -> Result<(), ApplicationError> {
    if let Ok(mutex) = Arc::try_unwrap(tx) {
        mutex
            .into_inner()
            .rollback()
            .await
            .map_err(map_sqlx_error)?;
    }

    Ok(())
}

pub fn map_sqlx_error(err: sqlx::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Database(err))
}
