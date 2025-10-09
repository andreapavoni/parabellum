use anyhow::{anyhow, Error, Result};
use chrono::{DateTime, Utc};
use diesel::{prelude::*, sql_types::BigInt};
use tokio::task;
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
    jobs::{Job, JobTask},
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
    async fn create(&self, _username: String, _tribe: Tribe) -> Result<Player> {
        unimplemented!()
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
        let pool = self.pool.clone();
        let village_id_i32 = village_id_u32 as i32;

        let village = task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let db_village: db_models::Village = match villages::table
                .find(village_id_i32)
                .first::<db_models::Village>(&mut conn)
                .optional()?
            {
                Some(v) => v,
                None => return Ok::<Option<Village>, Error>(None),
            };

            let db_player: db_models::Player =
                players::table.find(db_village.player_id).first(&mut conn)?;

            let all_armies: Vec<db_models::Army> = armies::table
                .filter(armies::village_id.eq(village_id_i32))
                .or_filter(armies::current_map_field_id.eq(village_id_i32))
                .load(&mut conn)?;

            let db_oases: Vec<db_models::MapField> = map_fields::table
                .filter(map_fields::village_id.eq(village_id_i32))
                .load(&mut conn)?;

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

            let domain_village = Village {
                id: db_village.id as u32,
                name: db_village.name,
                player_id: db_village.player_id,
                position: db_village.position.into(),
                tribe,
                buildings: db_village.buildings.into(),
                oases,
                population: db_village.population as u32,
                army: home_army.ok_or_else(|| {
                    anyhow!("Il villaggio {} non ha un'armata di casa", village_id_i32)
                })?,
                reinforcements,
                deployed_armies,
                loyalty: db_village.loyalty as u8,
                production: db_village.production.into(),
                is_capital: db_village.is_capital,
                smithy: db_village.smithy_upgrades.into(),
                stocks: db_village.stocks.into(),
                updated_at: db_village.updated_at,
            };

            Ok(Some(domain_village))
        })
        .await??;

        Ok(village)
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
    async fn find_unoccupied_valley(&self, _quadrant: &MapQuadrant) -> Result<Valley> {
        unimplemented!()
    }
    async fn get_field_by_id(&self, _id: i32) -> Result<Option<MapField>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ArmyRepository for PostgresRepository {
    async fn get_by_id(&self, _army_id: Uuid) -> Result<Option<Army>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl JobRepository for PostgresRepository {
    async fn create(
        &self,
        id: Uuid,
        player_id: Uuid,
        village_id: i32,
        task: JobTask,
        completed_at: DateTime<Utc>,
    ) -> Result<()> {
        let pool = self.pool.clone();
        let new_job = NewJob {
            id,
            player_id,
            village_id,
            task: JsonbWrapper(task),
            status: JobStatus::Pending,
            completed_at,
        };

        task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(jobs::table)
                .values(&new_job)
                .execute(&mut conn)?;
            Ok::<(), Error>(())
        })
        .await??;
        Ok(())
    }

    async fn find_and_lock_due_jobs(&self, limit: i64) -> Result<Vec<Job>> {
        let pool = self.pool.clone();
        let jobs = task::spawn_blocking(move || {
            let mut conn = pool.get()?;

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
                .load::<crate::db::models::Job>(&mut conn)?;

            Ok::<Vec<crate::db::models::Job>, Error>(due_jobs)
        })
        .await??;

        Ok(jobs.into_iter().map(|db_job| db_job.into()).collect())
    }

    async fn mark_as_completed(&self, job_id: Uuid) -> Result<()> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(jobs::table.find(job_id))
                .set(jobs::status.eq(JobStatus::Completed))
                .execute(&mut conn)?;
            Ok::<(), Error>(())
        })
        .await??;
        Ok(())
    }

    async fn mark_as_failed(&self, job_id: Uuid, _error_message: &str) -> Result<()> {
        // Potresti voler aggiungere una colonna `error` alla tabella jobs
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(jobs::table.find(job_id))
                .set(jobs::status.eq(JobStatus::Failed))
                .execute(&mut conn)?;
            Ok::<(), Error>(())
        })
        .await??;
        Ok(())
    }
}
