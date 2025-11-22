use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::UserRepository;
use parabellum_types::common::User;
use parabellum_types::errors::{ApplicationError, DbError};

use crate::models as db_models;

#[derive(Clone)]
pub struct PostgresUserRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresUserRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> UserRepository for PostgresUserRepository<'a> {
    async fn save(&self, email: String, password_hash: String) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            r#"
              INSERT INTO users (email, password_hash)
              VALUES ($1, $2)
              ON CONFLICT (id) DO UPDATE
              SET
                  email = $1,
                  password_hash = $2
              "#,
            email,
            password_hash,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_email(&self, email: &String) -> Result<User, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let rec = sqlx::query_as!(
            db_models::User,
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE email = $1
            "#,
            email,
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserByEmailNotFound(email.clone())))?;

        Ok(rec.into())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<User, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let rec = sqlx::query_as!(
            db_models::User,
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE id = $1
            "#,
            id,
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserByIdNotFound(id)))?;

        Ok(rec.into())
    }
}
