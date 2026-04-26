use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::UserRepository;
use parabellum_types::{
    common::User,
    errors::{ApplicationError, DbError},
};

use crate::toasty_models::user::UserRecord;

pub struct ToastyUserRepository<'a> {
    tx: Arc<Mutex<toasty::Transaction<'a>>>,
}

impl<'a> ToastyUserRepository<'a> {
    pub fn new(tx: Arc<Mutex<toasty::Transaction<'a>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> UserRepository for ToastyUserRepository<'a> {
    async fn save(&self, email: String, password_hash: String) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
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
        let mut tx_guard = self.tx.lock().await;
        let mut rows = toasty::query!(UserRecord filter .email == #email)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let row = rows
            .pop()
            .ok_or_else(|| ApplicationError::Db(DbError::UserByEmailNotFound(email.to_string())))?;
        Ok(row.into())
    }

    async fn get_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut rows = toasty::query!(UserRecord filter .id == #user_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let row = rows
            .pop()
            .ok_or_else(|| ApplicationError::Db(DbError::UserByIdNotFound(user_id)))?;
        Ok(row.into())
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::toasty_db::establish_test_toasty_db;

    #[tokio::test]
    async fn toasty_user_repo_save_and_get() -> Result<(), ApplicationError> {
        let mut toasty_db = establish_test_toasty_db()
            .await
            .map_err(ApplicationError::Db)?;

        let tx = toasty_db.transaction().await.map_err(map_toasty_error)?;
        let tx = Arc::new(Mutex::new(tx));
        let repo = ToastyUserRepository::new(tx.clone());

        let unique = Uuid::new_v4();
        let email = format!("toasty-user-{unique}@example.test");
        let password_hash = format!("hash-{unique}");

        repo.save(email.clone(), password_hash).await?;
        let loaded = repo.get_by_email(&email).await?;

        assert_eq!(loaded.email, email);
        assert_eq!(loaded.password_hash(), &format!("hash-{unique}"));

        drop(repo);
        drop(tx); // rollback on drop

        Ok(())
    }
}
