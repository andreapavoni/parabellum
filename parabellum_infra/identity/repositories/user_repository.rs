use sqlx::PgPool;
use uuid::Uuid;

use parabellum_app::ports::identity::UserRepository;
use parabellum_types::common::User;
use parabellum_types::errors::{ApplicationError, DbError};

use crate::persistence::models as db_models;

#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for PostgresUserRepository {
    async fn save(&self, email: String, password_hash: String) -> Result<(), ApplicationError> {
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
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_email(&self, email: &str) -> Result<User, ApplicationError> {
        let rec = sqlx::query_as!(
            db_models::User,
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE email = $1
            "#,
            email,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserByEmailNotFound(email.to_string())))?;

        Ok(rec.into())
    }

    async fn get_by_username(&self, username: &str) -> Result<User, ApplicationError> {
        let rec = sqlx::query_as::<_, db_models::User>(
            r#"
            SELECT u.id, u.email, u.password_hash
            FROM users u
            JOIN players p ON p.user_id = u.id
            WHERE p.username = $1
            "#,
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserByUsernameNotFound(username.to_string())))?;

        Ok(rec.into())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<User, ApplicationError> {
        let rec = sqlx::query_as!(
            db_models::User,
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE id = $1
            "#,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserByIdNotFound(id)))?;

        Ok(rec.into())
    }
}
