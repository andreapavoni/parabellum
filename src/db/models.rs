use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{models as db_models, schema::jobs},
    game::models as game_models,
    impl_jsonb_for,
    jobs::JobTask,
};

use super::schema::{armies, map_fields, players, villages};
use super::utils::JsonbWrapper;

impl_jsonb_for!(game_models::map::MapFieldTopology);
impl_jsonb_for!(game_models::map::Position);
impl_jsonb_for!(game_models::SmithyUpgrades);
impl_jsonb_for!(game_models::village::StockCapacity);
impl_jsonb_for!(Vec<game_models::village::VillageBuilding>);
impl_jsonb_for!(game_models::village::VillageProduction);
impl_jsonb_for!(JobTask);

#[derive(DbEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[ExistingTypePath = "crate::db::schema::sql_types::Tribe"]
pub enum Tribe {
    #[db_rename = "Roman"]
    Roman,
    #[db_rename = "Gaul"]
    Gaul,
    #[db_rename = "Teuton"]
    Teuton,
    #[db_rename = "Natar"]
    Natar,
    #[db_rename = "Nature"]
    Nature,
}

impl From<Tribe> for game_models::Tribe {
    fn from(db_tribe: Tribe) -> Self {
        match db_tribe {
            Tribe::Roman => game_models::Tribe::Roman,
            Tribe::Gaul => game_models::Tribe::Gaul,
            Tribe::Teuton => game_models::Tribe::Teuton,
            Tribe::Natar => game_models::Tribe::Natar,
            Tribe::Nature => game_models::Tribe::Nature,
        }
    }
}

impl From<game_models::Tribe> for Tribe {
    fn from(game_tribe: game_models::Tribe) -> Self {
        match game_tribe {
            game_models::Tribe::Roman => Tribe::Roman,
            game_models::Tribe::Gaul => Tribe::Gaul,
            game_models::Tribe::Teuton => Tribe::Teuton,
            game_models::Tribe::Natar => Tribe::Natar,
            game_models::Tribe::Nature => Tribe::Nature,
        }
    }
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = players)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub tribe: Tribe,
}
impl From<Player> for game_models::Player {
    fn from(player: Player) -> Self {
        game_models::Player {
            id: player.id,
            username: player.username,
            // The enum conversion is required because they're defined in two different places
            tribe: player.tribe.into(),
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = players)]
pub struct NewPlayer<'a> {
    pub id: Uuid,
    pub username: &'a str,
    pub tribe: Tribe,
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = villages)]
pub struct Village {
    pub id: i32,
    pub player_id: Uuid,
    pub name: String,
    pub position: JsonbWrapper<game_models::map::Position>,
    pub buildings: JsonbWrapper<Vec<game_models::village::VillageBuilding>>,
    pub production: JsonbWrapper<game_models::village::VillageProduction>,
    pub stocks: JsonbWrapper<game_models::village::StockCapacity>,
    pub smithy_upgrades: JsonbWrapper<game_models::SmithyUpgrades>,
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = villages)]
pub struct NewVillage<'a> {
    pub id: i32,
    pub player_id: Uuid,
    pub name: &'a str,
    pub position: JsonbWrapper<game_models::map::Position>,
    pub buildings: JsonbWrapper<Vec<game_models::village::VillageBuilding>>,
    pub production: JsonbWrapper<game_models::village::VillageProduction>,
    pub stocks: JsonbWrapper<game_models::village::StockCapacity>,
    pub smithy_upgrades: JsonbWrapper<game_models::SmithyUpgrades>,
    pub population: i32,
    pub loyalty: i16,
    pub is_capital: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = armies)]
pub struct Army {
    pub id: Uuid,
    pub village_id: i32,
    pub current_map_field_id: i32, // Oasis or village
    pub hero_id: Option<Uuid>,
    pub units: JsonbWrapper<game_models::army::TroopSet>,
    pub smithy: JsonbWrapper<game_models::SmithyUpgrades>,
    pub tribe: Tribe,
    pub player_id: Uuid,
}

impl From<db_models::Army> for game_models::army::Army {
    fn from(army: db_models::Army) -> Self {
        game_models::army::Army {
            village_id: army.village_id as u32,
            current_map_field_id: Some(army.current_map_field_id as u32),
            player_id: army.player_id,
            units: Default::default(),
            smithy: Default::default(),
            // TODO: load hero through join
            hero: None,
            // The enum conversion is required because they're defined in two different places
            tribe: army.tribe.into(),
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = armies)]
pub struct NewArmy<'a> {
    pub id: Uuid,
    pub village_id: i32,
    pub current_map_field_id: i32, // Oasis or village
    pub hero_id: Option<Uuid>,
    pub units: &'a JsonbWrapper<game_models::army::TroopSet>,
    pub smithy: &'a JsonbWrapper<game_models::SmithyUpgrades>,
    pub tribe: Tribe,
    pub player_id: Uuid,
}

#[derive(Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = map_fields)]
pub struct MapField {
    pub id: i32,
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,
    pub position: JsonbWrapper<game_models::map::Position>,
    pub topology: JsonbWrapper<game_models::map::MapFieldTopology>,
}

impl From<db_models::MapField> for game_models::map::MapField {
    fn from(map_field: db_models::MapField) -> Self {
        let village_id = match map_field.village_id {
            Some(id) => Some(id as u32),
            None => None,
        };
        game_models::map::MapField {
            id: map_field.id as u32,
            village_id: village_id,
            player_id: map_field.player_id,
            position: map_field.position.into(),
            topology: map_field.topology.into(),
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = map_fields)]
pub struct NewMapField<'a> {
    pub id: i32,
    pub village_id: Option<i32>,
    pub player_id: Option<Uuid>,
    pub position: &'a JsonbWrapper<game_models::map::Position>,
    pub topology: &'a JsonbWrapper<game_models::map::MapFieldTopology>,
}

#[derive(DbEnum, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[ExistingTypePath = "crate::db::schema::sql_types::JobStatus"]
pub enum JobStatus {
    #[db_rename = "Pending"]
    Pending,
    #[db_rename = "Processing"]
    Processing,
    #[db_rename = "Completed"]
    Completed,
    #[db_rename = "Failed"]
    Failed,
}

#[derive(Queryable, QueryableByName, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = jobs)]
pub struct Job {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: i32,
    pub task: JsonbWrapper<JobTask>,
    pub status: JobStatus,
    pub completed_at: chrono::DateTime<Utc>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<Job> for crate::jobs::Job {
    fn from(job: Job) -> Self {
        crate::jobs::Job {
            id: job.id,
            player_id: job.player_id,
            village_id: job.village_id,
            task: job.task.into(),
            status: match job.status {
                JobStatus::Pending => crate::jobs::JobStatus::Pending,
                JobStatus::Processing => crate::jobs::JobStatus::Processing,
                JobStatus::Completed => crate::jobs::JobStatus::Completed,
                JobStatus::Failed => crate::jobs::JobStatus::Failed,
            },
            completed_at: job.completed_at,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = jobs)]
pub struct NewJob {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: i32,
    pub task: JsonbWrapper<JobTask>,
    pub status: JobStatus,
    pub completed_at: chrono::DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;
    use crate::{
        db::{connection::run_test_with_transaction, test_factories::*},
        game::models::{
            army::TroopSet,
            map::{MapFieldTopology, OasisTopology, Position},
        },
    };

    #[tokio::test]
    async fn test_factories_with_defaults() {
        run_test_with_transaction(|conn| {
            // Wrap the async block in Box::pin()
            Box::pin(async move {
                let player = player_factory(conn, Default::default()).await;
                assert!(player.username.starts_with("user_"));
                assert_eq!(player.tribe, Tribe::Roman);

                let village = village_factory(conn, Default::default()).await;
                assert_eq!(village.name, "Factory Village");

                let army = army_factory(conn, Default::default()).await;
                assert_eq!(army.units.0[0], 10);

                let field_default = map_field_factory(conn, Default::default()).await;
                assert!(field_default.id != 0);

                Ok(())
            })
        })
        .await;
    }

    #[test]
    fn test_factories_with_overrides() {
        let _ = run_test_with_transaction(|conn| {
            Box::pin(async move {
                let player_id = Uuid::new_v4();

                let player = player_factory(
                    conn,
                    PlayerFactoryOptions {
                        id: Some(player_id),
                        username: Some("Dino"),
                        tribe: Some(Tribe::Gaul),
                    },
                )
                .await;
                assert_eq!(player.id, player_id);
                assert_eq!(player.username, "Dino");
                assert_eq!(player.tribe, Tribe::Gaul);

                let world_size = 100;
                let position = &Position {
                    x: rand::thread_rng().gen_range(-world_size..world_size),
                    y: rand::thread_rng().gen_range(-world_size..world_size),
                };

                let village = village_factory(
                    conn,
                    VillageFactoryOptions {
                        player_id: Some(player.id),
                        name: Some("Dino's Village"),
                        position: Some(position),
                        buildings: Some(vec![]),
                        production: Some(Default::default()),
                        stocks: Some(Default::default()),
                        smithy_upgrades: Some(Default::default()),
                        population: 2,
                        loyalty: 100,
                        is_capital: true,
                    },
                )
                .await;
                assert_eq!(village.player_id, player.id);
                assert_eq!(village.name, "Dino's Village");

                let units: TroopSet = [100, 100, 0, 0, 0, 0, 0, 0, 0, 0];
                let army = army_factory(
                    conn,
                    ArmyFactoryOptions {
                        id: Some(Uuid::new_v4()),
                        player_id: Some(player.id),
                        village_id: Some(village.id),
                        current_map_field_id: Some(village.id),
                        units: Some(units),
                        hero_id: None,
                        smithy: Some(Default::default()),
                        tribe: Some(player.tribe),
                    },
                )
                .await;
                assert_eq!(army.player_id, player.id);
                assert_eq!(army.village_id, village.id);
                assert_eq!(army.units.0, units);

                let topology = MapFieldTopology::Oasis(OasisTopology::Crop50);

                let field_custom = map_field_factory(
                    conn,
                    MapFieldFactoryOptions {
                        position: Some(position.clone()),
                        topology: Some(topology.clone()),
                        village_id: Some(village.id),
                        player_id: Some(player.id),
                    },
                )
                .await;
                assert_eq!(field_custom.position.0, *position);
                assert_eq!(field_custom.topology.0, topology);

                Ok(())
            })
        });
    }
}
