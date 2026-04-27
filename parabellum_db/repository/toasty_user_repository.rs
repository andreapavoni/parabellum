use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::UserRepository;
use parabellum_types::{
    common::User,
    errors::{ApplicationError, DbError},
};

use crate::toasty_models::user::UserRecord;

pub struct ToastyUserRepository {
    db: Arc<Mutex<toasty::Db>>,
}

impl ToastyUserRepository {
    pub fn new(db: Arc<Mutex<toasty::Db>>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl UserRepository for ToastyUserRepository {
    async fn save(&self, email: String, password_hash: String) -> Result<(), ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        toasty::create!(UserRecord {
            id: Uuid::new_v4(),
            email,
            password_hash,
            created_at: jiff::Timestamp::now(),
        })
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        Ok(())
    }

    async fn get_by_email(&self, email: &str) -> Result<User, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let row = UserRecord::get_by_email(&mut *tx_guard, email)
            .await
            .map_err(|_| ApplicationError::Db(DbError::UserByEmailNotFound(email.to_string())))?;
        Ok(row.into())
    }

    async fn get_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError> {
        let mut tx_guard = self.db.lock().await;
        let row = UserRecord::get_by_id(&mut *tx_guard, user_id)
            .await
            .map_err(|_| ApplicationError::Db(DbError::UserByIdNotFound(user_id)))?;
        Ok(row.into())
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        establish_test_connection_pool, repository::PostgresUserRepository,
        toasty_db::establish_test_toasty_db,
    };

    #[tokio::test]
    async fn toasty_user_repo_save_and_get() -> Result<(), ApplicationError> {
        let toasty_db = Arc::new(Mutex::new(
            establish_test_toasty_db()
                .await
                .map_err(ApplicationError::Db)?,
        ));
        let repo = ToastyUserRepository::new(toasty_db.clone());

        let unique = Uuid::new_v4();
        let email = format!("toasty-user-{unique}@example.test");
        let password_hash = format!("hash-{unique}");

        repo.save(email.clone(), password_hash).await?;
        let loaded = repo.get_by_email(&email).await?;

        assert_eq!(loaded.email, email);
        assert_eq!(loaded.password_hash(), &format!("hash-{unique}"));

        drop(repo);
        drop(toasty_db);

        Ok(())
    }

    #[tokio::test]
    async fn toasty_and_sqlx_user_get_by_email_parity() -> Result<(), ApplicationError> {
        let pool = establish_test_connection_pool()
            .await
            .map_err(ApplicationError::Db)?;

        let unique = Uuid::new_v4();
        let email = format!("toasty-sqlx-user-{unique}@example.test");
        let password_hash = format!("hash-{unique}");
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind(&email)
        .bind(&password_hash)
        .fetch_one(&pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let sqlx_tx = pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let sqlx_tx = Arc::new(Mutex::new(sqlx_tx));
        let sqlx_repo = PostgresUserRepository::new(sqlx_tx.clone());

        let toasty_db = Arc::new(Mutex::new(
            establish_test_toasty_db()
                .await
                .map_err(ApplicationError::Db)?,
        ));
        let toasty_repo = ToastyUserRepository::new(toasty_db.clone());

        let from_sqlx = sqlx_repo.get_by_email(&email).await?;
        let from_toasty = toasty_repo.get_by_email(&email).await?;

        assert_eq!(from_sqlx.id, user_id);
        assert_eq!(from_sqlx.id, from_toasty.id);
        assert_eq!(from_sqlx.email, from_toasty.email);
        assert_eq!(from_sqlx.password_hash(), from_toasty.password_hash());

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        drop(toasty_repo);
        drop(toasty_db);
        drop(sqlx_repo);
        drop(sqlx_tx); // rollback on drop

        Ok(())
    }
}
