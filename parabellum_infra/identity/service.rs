use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info, warn};
use uuid::Uuid;

use parabellum_app::{
    auth::hash_password,
    config::Config,
    ports::identity::{IdentityPort, RegisterPlayerRequest},
    villages::{FoundVillage, SetVillageResources},
};
use parabellum_game::models::{
    buildings::Building,
    map::{MapQuadrant, Valley},
    village::{Village, VillageBuilding},
};
use parabellum_types::{
    buildings::BuildingName,
    common::{Player, User},
    errors::{AppError, ApplicationError, DbError},
};
use sqlx::PgPool;

use crate::es::VillageEsService;
use crate::map::repository::random_unoccupied_4446_valley_for_update_query;
use crate::persistence::models as db_models;

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
        info!(
            player_id = %req.player_id,
            username = %req.username,
            email = %req.email,
            quadrant = ?req.quadrant,
            "starting player registration"
        );
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

        let tribe: crate::persistence::models::Tribe = req.tribe.clone().into();
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
        debug!(
            player_id = %req.player_id,
            village_id = valley.id,
            x = valley.position.x,
            y = valley.position.y,
            "selected initial unoccupied valley"
        );
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

        let server_speed = req.initial_village.as_ref().and_then(|s| s.speed).unwrap_or(self.config.speed);
        let (village_name, buildings) = village_setup_from_request(&req, &village, server_speed)?;
        let village_id = village.id;
        let found = FoundVillage {
            village_name,
            position: village.position.clone(),
            tribe: village.tribe.clone(),
            player_id: village.player_id,
            parent_village_id: None,
            buildings,
        };

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
        .bind(req.player_id)
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
            warn!(
                user_id = %user_id,
                player_id = %req.player_id,
                village_id,
                error = %e,
                "registration failed during initial village foundation; starting cleanup"
            );
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

        if let Some(resources) = req.initial_village.as_ref().and_then(|s| s.resources.clone()) {
            VillageEsService::new(self.pool.clone())
                .set_village_resources(
                    village_id,
                    &SetVillageResources {
                        player_id: req.player_id,
                        resources,
                    },
                )
                .await
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        }

        info!(
            user_id = %user_id,
            player_id = %req.player_id,
            village_id,
            "player registration completed"
        );

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

    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, ApplicationError> {
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

fn village_setup_from_request(
    req: &RegisterPlayerRequest,
    village: &Village,
    speed: i8,
) -> Result<(String, Vec<VillageBuilding>), ApplicationError> {
    let Some(setup) = &req.initial_village else {
        return Ok((village.name.clone(), village.buildings().clone()));
    };

    let village_name = setup
        .village_name
        .clone()
        .unwrap_or_else(|| village.name.clone());
    let mut buildings = village.buildings().clone();

    if setup.resource_fields_target_level > 0 {
        for vb in &mut buildings {
            if vb.slot_id <= 18 {
                vb.building = Building::new(vb.building.name.clone(), speed)
                    .at_level(setup.resource_fields_target_level, speed)
                    .map_err(ApplicationError::from)?;
            }
        }
    }

    for override_building in &setup.buildings {
        if override_building.slot_id <= 18 {
            continue;
        }
        let normalized = VillageBuilding {
            slot_id: override_building.slot_id,
            building: Building::new(override_building.building.name.clone(), speed)
                .at_level(override_building.building.level, speed)
                .map_err(ApplicationError::from)?,
        };
        upsert_building(&mut buildings, normalized);
    }

    ensure_rally_point_minimum(&mut buildings, speed)?;
    normalize_buildings_by_slot(&mut buildings);
    Ok((village_name, buildings))
}

fn upsert_building(buildings: &mut Vec<VillageBuilding>, building: VillageBuilding) {
    if let Some(existing) = buildings.iter_mut().find(|b| b.slot_id == building.slot_id) {
        *existing = building;
        return;
    }
    buildings.push(building);
}

fn ensure_rally_point_minimum(buildings: &mut Vec<VillageBuilding>, speed: i8) -> Result<(), ApplicationError> {
    if buildings.iter().any(|b| b.slot_id == 39) {
        return Ok(());
    }
    let rally = Building::new(BuildingName::RallyPoint, speed)
        .at_level(1, speed)
        .map_err(ApplicationError::from)?;
    buildings.push(VillageBuilding {
        slot_id: 39,
        building: rally,
    });
    Ok(())
}

fn normalize_buildings_by_slot(buildings: &mut Vec<VillageBuilding>) {
    let mut normalized = Vec::with_capacity(buildings.len());
    for building in buildings.drain(..) {
        if let Some(existing) = normalized
            .iter_mut()
            .find(|b: &&mut VillageBuilding| b.slot_id == building.slot_id)
        {
            *existing = building;
        } else {
            normalized.push(building);
        }
    }
    *buildings = normalized;
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
