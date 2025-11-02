//! Factories for creating domain model instances for testing.
//! These do not interact with the database.

use super::models::{
    Tribe,
    army::{Army, TroopSet},
    common::Player,
    hero::Hero,
    map::{Position, Valley, ValleyTopology},
    smithy::SmithyUpgrades,
    village::Village,
};
use rand::Rng;
use uuid::Uuid;

// --- Options Structs (Builders) ---

#[derive(Default, Clone)]
pub struct PlayerFactoryOptions<'a> {
    pub id: Option<Uuid>,
    pub username: Option<&'a str>,
    pub tribe: Option<Tribe>,
}

#[derive(Default, Clone)]
pub struct ValleyFactoryOptions {
    pub position: Option<Position>,
    pub topology: Option<ValleyTopology>,
    pub player_id: Option<Uuid>,
    pub village_id: Option<u32>,
}

#[derive(Default, Clone)]
pub struct VillageFactoryOptions {
    pub name: Option<String>,
    pub player: Option<Player>,
    pub valley: Option<Valley>,
    pub is_capital: Option<bool>,
}

#[derive(Default, Clone)]
pub struct ArmyFactoryOptions {
    pub village_id: Option<u32>,
    pub player_id: Option<Uuid>,
    pub tribe: Option<Tribe>,
    pub units: Option<TroopSet>,
    pub smithy: Option<SmithyUpgrades>,
    pub hero: Option<Hero>,
}

// --- Factory Functions ---

pub fn player_factory(options: PlayerFactoryOptions) -> Player {
    let default_username: String = format!("user_{}", rand::thread_rng().r#gen::<u32>());
    Player {
        id: options.id.unwrap_or_else(Uuid::new_v4),
        username: options.username.map_or(default_username, |s| s.to_string()),
        tribe: options.tribe.unwrap_or(Tribe::Roman),
    }
}

pub fn valley_factory(options: ValleyFactoryOptions) -> Valley {
    let position = options.position.unwrap_or(Position { x: 0, y: 0 });

    Valley {
        id: position.to_id(100),
        position,
        topology: options.topology.unwrap_or(ValleyTopology(4, 4, 4, 6)),
        player_id: None,
        village_id: None,
    }
}

pub fn village_factory(options: VillageFactoryOptions) -> Village {
    Village::new(
        options.name.unwrap_or("Factory Village".to_string()),
        &options.valley.unwrap_or(valley_factory(Default::default())),
        &options.player.unwrap_or(player_factory(Default::default())),
        options.is_capital.unwrap_or(true),
    )
}

pub fn army_factory(options: ArmyFactoryOptions) -> Army {
    let village_id = options.village_id.unwrap_or(1);

    Army::new(
        Some(Uuid::new_v4()),
        village_id,
        Some(village_id),
        options.player_id.unwrap_or(Uuid::new_v4()),
        options.tribe.unwrap_or(Tribe::Teuton),
        options.units.unwrap_or([10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        options.smithy.unwrap_or_default(),
        options.hero,
    )
}
