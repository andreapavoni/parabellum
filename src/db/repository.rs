use anyhow::Result;
use sqlx::{types::Json, PgPool};
use uuid::Uuid;

use crate::{
    db::{
        mapping::VillageAggregate,
        models::{self as db_models, Tribe},
    },
    game::models::{
        army::Army,
        map::{MapField, MapQuadrant, Valley},
        village::Village,
        Player,
    },
    jobs::Job,
    repository::*,
};

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl PlayerRepository for PostgresRepository {
    async fn create(&self, player: &Player) -> Result<()> {
        let tribe: db_models::Tribe = player.tribe.clone().into();
        sqlx::query_as!(
            db_models::Player,
            r#"
                INSERT INTO players (id, username, tribe)
                VALUES ($1, $2, $3)
                RETURNING id, username, tribe AS "tribe: _"
                "#,
            player.id,
            player.username,
            tribe as _
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_by_id(&self, player_id: Uuid) -> Result<Option<Player>> {
        let player = sqlx::query_as!(
            db_models::Player,
            r#"
                SELECT id, username, tribe AS "tribe: _"
                FROM players WHERE id = $1
                "#,
            player_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(player.map(Into::into))
    }

    async fn get_by_username(&self, username: &str) -> Result<Option<Player>> {
        let player = sqlx::query_as!(
            db_models::Player,
            r#"
                SELECT id, username, tribe AS "tribe: _"
                FROM players WHERE username = $1
                "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(player.map(Into::into))
    }
}

#[async_trait::async_trait]
impl VillageRepository for PostgresRepository {
    async fn create(&self, village: &Village) -> Result<()> {
        sqlx::query!(
                r#"
                INSERT INTO villages (id, player_id, name, position, buildings, production, stocks, smithy_upgrades, population, loyalty, is_capital)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
                village.id as i32,
                village.player_id,
                village.name,
                Json(&village.position) as _,
                Json(&village.buildings) as _,
                Json(&village.production) as _,
                Json(&village.stocks) as _,
                Json(&village.smithy) as _,
                village.population as i32,
                village.loyalty as i16,
                village.is_capital
            )
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_by_id(&self, village_id_u32: u32) -> Result<Option<Village>> {
        let village_id_i32 = village_id_u32 as i32;

        let db_village = match sqlx::query_as!(
            db_models::Village,
            "SELECT * FROM villages WHERE id = $1",
            village_id_i32
        )
        .fetch_optional(&self.pool)
        .await?
        {
            Some(v) => v,
            None => return Ok(None),
        };

        let db_player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _" FROM players WHERE id = $1"#,
            db_village.player_id
        )
        .fetch_one(&self.pool)
        .await?;

        let all_armies = sqlx::query_as!(
                    db_models::Army,
                    r#"SELECT id, village_id, current_map_field_id, hero_id, units, smithy, player_id, tribe AS "tribe: _" FROM armies WHERE village_id = $1 OR current_map_field_id = $1"#,
                    village_id_i32
                )
                .fetch_all(&self.pool)
                .await?;

        let db_oases = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE village_id = $1",
            village_id_i32
        )
        .fetch_all(&self.pool)
        .await?;

        let aggregate = VillageAggregate {
            village: db_village,
            player: db_player,
            armies: all_armies,
            oases: db_oases,
        };

        let game_village = Village::try_from(aggregate)?;
        Ok(Some(game_village))
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Village>> {
        let villages_ids = sqlx::query!("SELECT id FROM villages WHERE player_id = $1", player_id)
            .fetch_all(&self.pool)
            .await?;

        let mut result = Vec::new();
        for record in villages_ids {
            if let Some(village) = VillageRepository::get_by_id(self, record.id as u32).await? {
                result.push(village);
            }
        }

        Ok(result)
    }

    async fn save(&self, village: &Village) -> Result<()> {
        sqlx::query!(
            r#"
                UPDATE villages
                SET
                    name = $2, buildings = $3, production = $4,
                    stocks = $5, smithy_upgrades = $6, population = $7,
                    loyalty = $8, updated_at = NOW()
                WHERE id = $1
                "#,
            village.id as i32,
            village.name,
            Json(&village.buildings) as _,
            Json(&village.production) as _,
            Json(&village.stocks) as _,
            Json(&village.smithy) as _,
            village.population as i32,
            village.loyalty as i16,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl MapRepository for PostgresRepository {
    async fn find_unoccupied_valley(&self, quadrant: &MapQuadrant) -> Result<Valley> {
        let query = match quadrant {
            MapQuadrant::NorthEast => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
            MapQuadrant::SouthEast => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int > 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
            MapQuadrant::SouthWest => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int < 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
            MapQuadrant::NorthWest => "SELECT * FROM map_fields WHERE village_id IS NULL AND (position->>'x')::int < 0 AND (position->>'y')::int > 0 AND topology ? 'Valley' ORDER BY RANDOM() LIMIT 1",
        };

        let random_unoccupied_field: db_models::MapField =
            sqlx::query_as(query).fetch_one(&self.pool).await?;

        let game_map_field: MapField = random_unoccupied_field.into();
        let valley = Valley::try_from(game_map_field)?;

        Ok(valley)
    }

    async fn get_field_by_id(&self, id: i32) -> Result<Option<MapField>> {
        let field = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(field.map(Into::into))
    }
}

#[async_trait::async_trait]
impl ArmyRepository for PostgresRepository {
    async fn get_by_id(&self, army_id: Uuid) -> Result<Option<Army>> {
        let army = sqlx::query_as!(
            db_models::Army,
            r#"SELECT id, village_id, current_map_field_id, hero_id, units, smithy, player_id, tribe AS "tribe: _" FROM armies WHERE id = $1"#,
            army_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(army.into()))
    }

    async fn create(&self, army: &Army) -> Result<()> {
        let db_tribe: Tribe = army.tribe.clone().into();
        let current_map_field_id = army.current_map_field_id.unwrap_or(army.village_id);
        let hero_id = match army.clone().hero {
            Some(hero) => Some(hero.id),
            _ => None,
        };

        sqlx::query!(
                r#"
                INSERT INTO armies (id, village_id, current_map_field_id, hero_id, units, smithy, tribe, player_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                army.id, army.village_id as i32, current_map_field_id as i32, hero_id, Json(&army.units) as _, Json(&army.smithy) as _, db_tribe as _, army.player_id
            )
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl JobRepository for PostgresRepository {
    async fn add(&self, job: &Job) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO jobs (id, player_id, village_id, task, status, completed_at)
            VALUES ($1, $2, $3, $4, 'Pending', $5)
            "#,
            job.id,
            job.player_id,
            job.village_id,
            Json(&job.task) as _,
            job.completed_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_by_id(&self, job_id: Uuid) -> Result<Option<Job>> {
        let job = sqlx::query_as!(
            db_models::Job,
            r#"SELECT id, player_id, village_id, task, status AS "status: _", completed_at, created_at, updated_at FROM jobs WHERE id = $1"#,
            job_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(job.into()))
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Job>> {
        let jobs = sqlx::query_as!(
            db_models::Job,
            r#"SELECT id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at FROM jobs WHERE player_id = $1 AND status IN ('Pending', 'Processing') ORDER BY completed_at ASC"#,
            player_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn find_and_lock_due_jobs(&self, limit: i64) -> Result<Vec<Job>> {
        let due_jobs = sqlx::query_as!(
            db_models::Job,
            r#"
            UPDATE jobs
            SET status = 'Processing', updated_at = NOW()
            WHERE id IN (
                SELECT id
                FROM jobs
                WHERE status = 'Pending' AND completed_at <= NOW()
                ORDER BY completed_at
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, player_id, village_id, task, status as "status: _", completed_at, created_at, updated_at;
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(due_jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn mark_as_completed(&self, job_id: Uuid) -> Result<()> {
        sqlx::query!("UPDATE jobs SET status = 'Completed' WHERE id = $1", job_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn mark_as_failed(&self, job_id: Uuid, _error_message: &str) -> Result<()> {
        sqlx::query!("UPDATE jobs SET status = 'Failed' WHERE id = $1", job_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
