// use uuid::Uuid;

// use crate::game::models::{
//     map::{
//         generate_new_map, MapField, MapFieldTopology, MapQuadrant, Oasis, Valley, ValleyTopology,
//     },
//     village::Village,
//     Player, Tribe,
// };

// TODO: everything here could/should be managed through commands/queries, no need for a Repository object

// src/db/repository.rs

// use anyhow::Result;
// use diesel::prelude::*;
// use uuid::Uuid;

// use super::connection::DbPool;
// use super::models::{NewPlayer, Player as DbPlayer, Tribe};
// use super::schema::players::dsl::*;
// use crate::game::models::Player as DomainPlayer; // Rinomina per chiarezza
// use crate::repository::Repository;

// pub struct DbRepository {
//     pool: DbPool,
// }

// impl DbRepository {
//     pub fn new(pool: DbPool) -> Self {
//         Self { pool }
//     }
// }

// #[async_trait::async_trait]
// impl Repository for DbRepository {
//     async fn register_player(&self, uname: String, t: Tribe) -> Result<DomainPlayer> {
//         let mut conn = self.pool.get()?;

//         let new_player = NewPlayer {
//             id: Uuid::new_v4(),
//             username: &uname,
//             tribe: t.into(), // Implementa From<Tribe> for TribeEnum
//         };

//         let db_player = diesel::insert_into(players)
//             .values(&new_player)
//             .get_result::<DbPlayer>(&mut conn)?;

//         Ok(db_player.into()) // Implementa From<DbPlayer> for DomainPlayer
//     }
// }
