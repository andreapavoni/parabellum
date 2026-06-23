use async_trait::async_trait;
use uuid::Uuid;

use parabellum_app::identity::{
    CreatedRegistrationIdentity, IdentityPort, RegistrationIdentityPort, RegistrationIdentityRecord,
};
use parabellum_game::models::map::{MapQuadrant, Valley};
use parabellum_types::{
    common::{Player, User},
    errors::{AppError, ApplicationError, DbError},
    map::ValleyTopology,
};
use sqlx::PgPool;

use crate::map::repository::random_unoccupied_4446_valley_for_update_query;
use crate::persistence::models as db_models;

#[derive(Clone)]
/// Core identity service for authentication and identity persistence.
pub struct IdentityService {
    pool: PgPool,
}

impl IdentityService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn create_registration_identity_record(
        &self,
        record: RegistrationIdentityRecord,
    ) -> Result<CreatedRegistrationIdentity, ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let user_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO users (email, password_hash)
            VALUES ($1, $2)
            RETURNING id
            "#,
        )
        .bind(&record.email)
        .bind(&record.password_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let tribe: crate::persistence::models::Tribe = record.tribe.clone().into();
        sqlx::query(
            r#"
            INSERT INTO players (id, username, tribe, user_id, culture_points)
            VALUES ($1, $2, $3, $4, 0)
            "#,
        )
        .bind(record.player_id)
        .bind(&record.username)
        .bind(tribe)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let valley = self
            .select_unoccupied_valley(&mut tx, &record.quadrant)
            .await?;
        if valley.topology != ValleyTopology(4, 4, 4, 6) {
            return Err(ApplicationError::Db(DbError::Transaction(
                "initial village must be founded on a 4-4-4-6 valley".to_string(),
            )));
        }
        let player = Player {
            id: record.player_id,
            username: record.username.clone(),
            tribe: record.tribe.clone(),
            user_id,
            culture_points: 0,
        };
        let village_id = valley.id;

        // Soft-reserve the selected map field with player ownership while
        // keeping village_id NULL until FoundVillage projection writes rm_village.
        let updated = sqlx::query(
            r#"
            UPDATE rm_map_fields
            SET player_id = $2
            WHERE id = $1 AND village_id IS NULL AND player_id IS NULL
            "#,
        )
        .bind(village_id as i32)
        .bind(record.player_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        if updated.rows_affected() != 1 {
            return Err(ApplicationError::Db(DbError::Transaction(
                "selected valley became occupied during registration".to_string(),
            )));
        }

        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(CreatedRegistrationIdentity {
            user_id,
            player,
            valley,
        })
    }

    async fn cleanup_failed_registration_rows(
        &self,
        user_id: Uuid,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<(), ApplicationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        sqlx::query(
            r#"
            UPDATE rm_map_fields
            SET player_id = NULL
            WHERE id = $1 AND village_id IS NULL AND player_id = $2
            "#,
        )
        .bind(village_id as i32)
        .bind(player_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        sqlx::query("DELETE FROM players WHERE id = $1")
            .bind(player_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        tx.commit()
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))
    }

    async fn select_unoccupied_valley(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        quadrant: &MapQuadrant,
    ) -> Result<Valley, ApplicationError> {
        let query = random_unoccupied_4446_valley_for_update_query(quadrant);

        let map_field = sqlx::query_as::<_, crate::persistence::models::MapField>(query)
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| ApplicationError::Db(DbError::WorldMapNotInitialized))?;

        let game_map_field: parabellum_game::models::map::MapField = map_field.into();
        Valley::try_from(game_map_field.clone())
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(game_map_field.id)))
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<User, ApplicationError> {
        let user = self.user_by_username(username).await?;
        parabellum_app::auth::verify_password(user.password_hash(), password)
            .map_err(|_| ApplicationError::App(AppError::WrongAuthCredentials))?;
        Ok(user)
    }

    async fn user_by_email(&self, email: &str) -> Result<User, ApplicationError> {
        let rec = sqlx::query_as!(
            db_models::User,
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserByEmailNotFound(email.to_string())))?;
        Ok(rec.into())
    }

    async fn user_by_username(&self, username: &str) -> Result<User, ApplicationError> {
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

    async fn user_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError> {
        let rec = sqlx::query_as!(
            db_models::User,
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserByIdNotFound(user_id)))?;
        Ok(rec.into())
    }

    async fn player_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        let rec = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::UserPlayerNotFound(user_id)))?;
        Ok(rec.into())
    }

    async fn player_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        let rec = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _", user_id, culture_points FROM players WHERE id = $1"#,
            player_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;
        Ok(rec.into())
    }
}

#[async_trait]
impl IdentityPort for IdentityService {
    async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, ApplicationError> {
        self.authenticate(username, password).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, ApplicationError> {
        self.user_by_email(email).await
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError> {
        self.user_by_id(user_id).await
    }

    async fn get_player_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        self.player_by_user_id(user_id).await
    }

    async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        self.player_by_id(player_id).await
    }
}

#[async_trait]
impl RegistrationIdentityPort for IdentityService {
    async fn create_registration_identity(
        &self,
        record: RegistrationIdentityRecord,
    ) -> Result<CreatedRegistrationIdentity, ApplicationError> {
        self.create_registration_identity_record(record).await
    }

    async fn cleanup_failed_registration(
        &self,
        user_id: Uuid,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<(), ApplicationError> {
        self.cleanup_failed_registration_rows(user_id, player_id, village_id)
            .await
    }
}
