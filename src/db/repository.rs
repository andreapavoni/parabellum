use anyhow::Result;
use sqlx::{types::Json, PgPool};
use uuid::Uuid;

use crate::{
    db::models as db_models,
    game::models::{
        army::Army,
        map::{MapField, MapQuadrant, Oasis, Valley},
        village::Village,
        Player, Tribe,
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
    async fn create(&self, id: Uuid, username: String, tribe: Tribe) -> Result<Player> {
        let tribe: db_models::Tribe = tribe.into();
        let new_player = sqlx::query_as!(
            db_models::Player,
            r#"
                INSERT INTO players (id, username, tribe)
                VALUES ($1, $2, $3)
                RETURNING id, username, tribe AS "tribe: _"
                "#,
            id,
            username,
            tribe as _
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(new_player.into())
    }

    async fn get_by_id(&self, _player_id: Uuid) -> Result<Option<Player>> {
        unimplemented!()
    }
    async fn get_by_username(&self, _username: &str) -> Result<Option<Player>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl VillageRepository for PostgresRepository {
    async fn create(&self, _village: &Village) -> Result<()> {
        unimplemented!()
    }

    async fn get_by_id(&self, village_id_u32: u32) -> Result<Option<Village>> {
        let village_id_i32 = village_id_u32 as i32;

        let db_village = sqlx::query_as!(
            db_models::Village,
            "SELECT * FROM villages WHERE id = $1",
            village_id_i32
        )
        .fetch_all(&self.pool)
        .await?;

        let db_village = sqlx::query_as!(
            db_models::Village,
            "SELECT * FROM villages WHERE id = $1",
            village_id_i32
        )
        .fetch_one(&self.pool)
        .await?;

        let db_player = sqlx::query_as!(
            db_models::Player,
            r#"SELECT id, username, tribe AS "tribe: _" FROM players WHERE id = $1"#,
            db_village.player_id
        )
        .fetch_one(&self.pool)
        .await?;

        let all_armies: Vec<db_models::Army> = sqlx::query_as!(
            db_models::Army,
            r#"SELECT id, village_id, current_map_field_id, hero_id, units, smithy, player_id, tribe AS "tribe: _" FROM armies WHERE village_id = $1 OR current_map_field_id = $1"#,
            village_id_i32
        )
        .fetch_all(&self.pool)
        .await?;

        let db_oases: Vec<db_models::MapField> = sqlx::query_as!(
            db_models::MapField,
            "SELECT * FROM map_fields WHERE village_id = $1",
            village_id_i32
        )
        .fetch_all(&self.pool)
        .await?;

        let tribe: Tribe = db_player.tribe.into();

        let mut home_army: Option<Army> = None;
        let mut reinforcements = Vec::new();
        let mut deployed_armies = Vec::new();

        for db_army in all_armies {
            let game_army: Army = db_army.into();

            if game_army.village_id == village_id_u32
                && game_army.current_map_field_id == Some(village_id_u32)
            {
                home_army = Some(game_army);
            } else if game_army.village_id != village_id_u32
                && game_army.current_map_field_id == Some(village_id_u32)
            {
                reinforcements.push(game_army);
            } else if game_army.village_id == village_id_u32
                && game_army.current_map_field_id != Some(village_id_u32)
            {
                deployed_armies.push(game_army);
            }
        }
        let oases: Vec<Oasis> = db_oases
            .into_iter()
            .filter_map(|mf| Oasis::try_from(Into::<MapField>::into(mf)).ok())
            .collect();

        let village = Village {
            id: db_village.id as u32,
            name: db_village.name,
            player_id: db_village.player_id,
            position: serde_json::from_value(db_village.position).unwrap(),
            tribe: tribe.clone(),
            buildings: serde_json::from_value(db_village.buildings).unwrap(),
            oases,
            population: db_village.population as u32,
            army: home_army.unwrap_or_else(|| {
                Army::new(
                    db_village.id as u32,
                    Some(db_village.id as u32),
                    db_village.player_id,
                    tribe,
                    [0; 10],
                    serde_json::from_value(db_village.smithy_upgrades.clone()).unwrap(),
                    None,
                )
            }),
            reinforcements,
            deployed_armies,
            loyalty: db_village.loyalty as u8,
            production: serde_json::from_value(db_village.production).unwrap(),
            is_capital: db_village.is_capital,
            smithy: serde_json::from_value(db_village.smithy_upgrades).unwrap(),
            stocks: serde_json::from_value(db_village.stocks).unwrap(),
            updated_at: db_village.updated_at,
        };

        Ok(Some(village))
    }

    async fn list_by_player_id(&self, _player_id: Uuid) -> Result<Vec<Village>> {
        unimplemented!()
    }
    async fn save(&self, _village: &Village) -> Result<()> {
        unimplemented!()
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

    async fn get_field_by_id(&self, _id: i32) -> Result<Option<MapField>> {
        unimplemented!()
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
