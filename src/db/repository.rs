use anyhow::Result;
use diesel::{
    dsl::sql,
    prelude::*,
    sql_types::{BigInt, Bool, Text},
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    db::{
        models::{self as db_models, JobStatus, NewJob},
        schema::{
            armies::{self},
            jobs, map_fields, players, villages,
        },
        utils::JsonbWrapper,
        DbPool,
    },
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
    pool: DbPool, // Il tuo pool di connessioni Diesel
}

impl PostgresRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl PlayerRepository for PostgresRepository {
    async fn create(&self, username: String, tribe: Tribe) -> Result<Player> {
        let mut conn = self.pool.get().await?;

        let new_db_player = db_models::NewPlayer {
            id: Uuid::new_v4(),
            username: &username,
            tribe: tribe.into(),
        };

        let created_player: db_models::Player = diesel::insert_into(players::table)
            .values(&new_db_player)
            .get_result(&mut conn)
            .await?; // Aggiungiamo .await qui!

        Ok(created_player.into())
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
        let mut conn = self.pool.get().await?;

        let db_village: db_models::Village = villages::table
            .find(village_id_i32)
            .first::<db_models::Village>(&mut conn)
            .await?;

        let db_player: db_models::Player = players::table
            .find(db_village.player_id)
            .first(&mut conn)
            .await?;

        let all_armies: Vec<db_models::Army> = armies::table
            .filter(armies::village_id.eq(village_id_i32))
            .or_filter(armies::current_map_field_id.eq(village_id_i32))
            .load(&mut conn)
            .await?;

        let db_oases: Vec<db_models::MapField> = map_fields::table
            .filter(map_fields::village_id.eq(village_id_i32))
            .load(&mut conn)
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
            .filter_map(|mf| {
                Oasis::try_from(<db_models::MapField as Into<MapField>>::into(mf)).ok()
            })
            .collect();

        let village = Village {
            id: db_village.id as u32,
            name: db_village.name,
            player_id: db_village.player_id,
            position: db_village.position.into(),
            tribe: tribe.clone(),
            buildings: db_village.buildings.into(),
            oases,
            population: db_village.population as u32,
            army: home_army.unwrap_or(Army::new(
                db_village.id as u32,
                Some(db_village.id as u32),
                db_village.player_id,
                tribe,
                [0; 10],
                db_village.smithy_upgrades.clone().into(),
                None,
            )),
            reinforcements,
            deployed_armies,
            loyalty: db_village.loyalty as u8,
            production: db_village.production.into(),
            is_capital: db_village.is_capital,
            smithy: db_village.smithy_upgrades.into(),
            stocks: db_village.stocks.into(),
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
        let mut conn = self.pool.get().await?;

        // Start building the query
        let mut query = map_fields::table
            .filter(map_fields::village_id.is_null())
            .filter(sql::<Bool>(r#"topology ? 'Valley'"#))
            .order_by(sql::<Text>("RANDOM()"))
            .limit(1)
            .into_boxed(); // Use into_boxed() to allow modifying the query

        // Add filters based on the quadrant
        // We query the JSONB field, cast the value to integer, and then compare.
        query = match quadrant {
            MapQuadrant::NorthEast => query
                .filter(sql::<Bool>("(position->>'x')::int > 0"))
                .filter(sql::<Bool>("(position->>'y')::int > 0")),
            MapQuadrant::SouthEast => query
                .filter(sql::<Bool>("(position->>'x')::int > 0"))
                .filter(sql::<Bool>("(position->>'y')::int < 0")),
            MapQuadrant::SouthWest => query
                .filter(sql::<Bool>("(position->>'x')::int < 0"))
                .filter(sql::<Bool>("(position->>'y')::int < 0")),
            MapQuadrant::NorthWest => query
                .filter(sql::<Bool>("(position->>'x')::int < 0"))
                .filter(sql::<Bool>("(position->>'y')::int > 0")),
        };

        let random_unoccupied_field: db_models::MapField = query.get_result(&mut conn).await?;
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
        let mut conn = self.pool.get().await?;

        let army = armies::table
            .find(army_id)
            .first::<db_models::Army>(&mut conn)
            .await?;

        Ok(Some(army.into()))
    }
}

#[async_trait::async_trait]
impl JobRepository for PostgresRepository {
    async fn add(&self, job: &Job) -> Result<()> {
        let pool = self.pool.clone();
        let new_job = NewJob {
            id: job.id,
            player_id: job.player_id,
            village_id: job.village_id,
            task: JsonbWrapper(job.task.clone()),
            status: JobStatus::Pending,
            completed_at: job.completed_at,
        };

        let mut conn = pool.get().await?;

        diesel::insert_into(jobs::table)
            .values(&new_job)
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    async fn get_by_id(&self, job_id: Uuid) -> Result<Option<Job>> {
        let mut conn = self.pool.get().await?;

        let job = jobs::table
            .find(job_id)
            .first::<db_models::Job>(&mut conn)
            .await?;

        Ok(Some(job.into()))
    }

    async fn list_by_player_id(&self, player_id: Uuid) -> Result<Vec<Job>> {
        let mut conn = self.pool.get().await?;

        let jobs = jobs::table
            .filter(jobs::player_id.eq(player_id))
            .filter(jobs::status.eq_any(vec![JobStatus::Pending, JobStatus::Processing]))
            .order(jobs::completed_at.asc())
            .load::<db_models::Job>(&mut conn)
            .await?;

        println!("--------------> JOBS BY PLAYER ID: {:?}", jobs);

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn find_and_lock_due_jobs(&self, limit: i64) -> Result<Vec<Job>> {
        let mut conn = self.pool.get().await?;

        let raw_sql = r#"
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
                RETURNING *;
            "#;

        let due_jobs = diesel::sql_query(raw_sql)
            .bind::<BigInt, _>(limit)
            .load::<db_models::Job>(&mut conn)
            .await?;

        Ok(due_jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn mark_as_completed(&self, job_id: Uuid) -> Result<()> {
        let mut conn = self.pool.get().await?;

        diesel::update(jobs::table.find(job_id))
            .set(jobs::status.eq(JobStatus::Completed))
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    async fn mark_as_failed(&self, job_id: Uuid, _error_message: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;

        diesel::update(jobs::table.find(job_id))
            .set(jobs::status.eq(JobStatus::Failed))
            .execute(&mut conn)
            .await?;

        Ok(())
    }
}
