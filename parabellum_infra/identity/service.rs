use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;
use uuid::Uuid;

use parabellum_app::{
    auth::hash_password,
    config::Config,
    ports::identity::{IdentityPort, RegisterPlayerRequest},
    villages::FoundVillage,
};
use parabellum_game::models::{
    map::{MapQuadrant, Valley},
    village::Village,
};
use parabellum_types::{
    common::{Player, User},
    errors::{AppError, ApplicationError, DbError},
};
use sqlx::PgPool;

use crate::db_types as db_models;
use crate::es::VillageEsService;

#[derive(Clone)]
/// Core registration service that persists identity/player data and initializes
/// the starting village through the ES village service.
pub struct IdentityService {
    pool: PgPool,
    config: Arc<Config>,
}

impl IdentityService {
    pub fn new(pool: PgPool, config: Arc<Config>) -> Self {
        Self { pool, config }
    }

    async fn register(&self, req: RegisterPlayerRequest) -> Result<(), ApplicationError> {
        let password_hash = hash_password(&req.password)?;
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
        .bind(&req.email)
        .bind(password_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let tribe: crate::db_types::Tribe = req.tribe.clone().into();
        sqlx::query(
            r#"
            INSERT INTO players (id, username, tribe, user_id, culture_points)
            VALUES ($1, $2, $3, $4, 0)
            "#,
        )
        .bind(req.player_id)
        .bind(&req.username)
        .bind(tribe)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let valley = self
            .select_unoccupied_valley(&mut tx, &req.quadrant)
            .await?;
        let player = Player {
            id: req.player_id,
            username: req.username.clone(),
            tribe: req.tribe.clone(),
            user_id,
            culture_points: 0,
        };
        let village = Village::new(
            format!("{}'s Village", req.username),
            &valley,
            &player,
            true,
            self.config.world_size as i32,
            self.config.speed,
        );

        let village_id = village.id;
        let found = FoundVillage {
            village_name: village.name.clone(),
            position: village.position.clone(),
            tribe: village.tribe.clone(),
            player_id: village.player_id,
            buildings: village.buildings().clone(),
        };

        // Reserve the selected map field in the canonical world map before
        // releasing the transaction lock.
        let updated = sqlx::query(
            r#"
            UPDATE rm_map_fields
            SET village_id = $1, player_id = $2
            WHERE id = $3 AND village_id IS NULL
            "#,
        )
        .bind(village_id as i32)
        .bind(req.player_id)
        .bind(village_id as i32)
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

        // ES runtime owns a separate transaction; call it only after the
        // player row is committed so rm_village FK checks can succeed.
        if let Err(e) = VillageEsService::new(self.pool.clone())
            .found_village(village_id, &found)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))
        {
            // Registration must be all-or-nothing. If village foundation fails
            // after identity commit, rollback identity + map reservation.
            if let Err(cleanup_err) = self
                .cleanup_failed_registration(user_id, req.player_id, village_id)
                .await
            {
                warn!(
                    user_id = %user_id,
                    player_id = %req.player_id,
                    village_id,
                    error = %cleanup_err,
                    "failed to cleanup registration after village foundation error"
                );
            }
            return Err(e);
        }

        Ok(())
    }

    async fn cleanup_failed_registration(
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
            SET village_id = NULL, player_id = NULL
            WHERE id = $1 AND player_id = $2
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
        let query = match quadrant {
            MapQuadrant::NorthEast => {
                "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology @> '{\"Valley\":[4,4,4,6]}' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
            }
            MapQuadrant::SouthEast => {
                "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology @> '{\"Valley\":[4,4,4,6]}' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
            }
            MapQuadrant::SouthWest => {
                "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology @> '{\"Valley\":[4,4,4,6]}' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
            }
            MapQuadrant::NorthWest => {
                "SELECT id, village_id, player_id, position, topology FROM rm_map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology @> '{\"Valley\":[4,4,4,6]}' ORDER BY RANDOM() LIMIT 1 FOR UPDATE SKIP LOCKED"
            }
        };

        let map_field = sqlx::query_as::<_, crate::db_types::MapField>(query)
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| ApplicationError::Db(DbError::WorldMapNotInitialized))?;

        let game_map_field: parabellum_game::models::map::MapField = map_field.into();
        Valley::try_from(game_map_field.clone())
            .map_err(|_| ApplicationError::Db(DbError::VillageNotFound(game_map_field.id)))
    }

    async fn authenticate(&self, email: &str, password: &str) -> Result<User, ApplicationError> {
        let user = self.get_user_by_email(email).await?;
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
    async fn register_player(
        &self,
        request: RegisterPlayerRequest,
    ) -> Result<(), ApplicationError> {
        self.register(request).await
    }

    async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, ApplicationError> {
        self.authenticate(email, password).await
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
