use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use tokio::task;
use uuid::Uuid;

use crate::{
    db::{
        models as db_models,
        schema::{
            armies::{self},
            map_fields, players, villages,
        },
        DbPool,
    },
    game::models::{
        army::Army,
        map::{MapField, MapQuadrant, Oasis, Valley},
        village::Village,
        Player, Tribe,
    },
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

        // Eseguiamo il codice sincrono di Diesel in un thread separato
        let village = task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // --- 1. Carica il Villaggio Base ---
            let db_village: db_models::Village = match villages::table
                .find(village_id_i32)
                .first::<db_models::Village>(&mut conn)
                .optional()?
            {
                Some(v) => v,
                None => return Ok::<Option<Village>, Error>(None),
            };

            // --- 2. Carica le Relazioni ---
            let db_player: db_models::Player =
                players::table.find(db_village.player_id).first(&mut conn)?;

            let all_armies: Vec<db_models::Army> = armies::table
                .filter(armies::village_id.eq(village_id_i32))
                .or_filter(armies::current_map_field_id.eq(village_id_i32))
                .load(&mut conn)?;

            let db_oases: Vec<db_models::MapField> = map_fields::table
                .filter(map_fields::village_id.eq(village_id_i32))
                .load(&mut conn)?;

            // --- 3. Assembla l'Oggetto di Dominio ---
            let tribe: Tribe = db_player.tribe.into();

            let mut home_army: Option<Army> = None;
            let mut reinforcements = Vec::new();
            let mut deployed_armies = Vec::new();

            for db_army in all_armies {
                let game_army: Army = db_army.into(); // Assumendo impl From<db_models::Army>

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
        .await??; // Il doppio '?' gestisce l'errore di `spawn_blocking` e quello interno di Diesel

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
impl ArmyRepository for PostgresRepository {}

#[async_trait::async_trait]
impl JobRepository for PostgresRepository {}
