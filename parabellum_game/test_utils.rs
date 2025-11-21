use rand::Rng;
use uuid::Uuid;

use parabellum_core::Result;
use parabellum_types::{
    common::User,
    map::{Position, ValleyTopology},
    tribe::Tribe,
};

use crate::models::player::Player;

use crate::models::map::{MapField, MapFieldTopology};

use super::models::{
    army::{Army, TroopSet},
    hero::Hero,
    map::Valley,
    smithy::SmithyUpgrades,
    village::Village,
};

#[derive(Default, Clone)]
pub struct PlayerFactoryOptions<'a> {
    pub id: Option<Uuid>,
    pub username: Option<&'a str>,
    pub tribe: Option<Tribe>,
    pub user_id: Option<Uuid>,
}

#[derive(Default, Clone)]
pub struct UserFactoryOptions {
    pub id: Option<Uuid>,
    pub email: Option<String>,
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
    pub world_size: Option<i32>,
    pub server_speed: Option<i8>,
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

#[derive(Default, Clone)]
pub struct HeroFactoryOptions {
    pub id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub village_id: Option<u32>,
    pub tribe: Option<Tribe>,
    pub level: Option<u16>,
    pub health: Option<u16>,
    pub strength: Option<u16>,
    pub off_bonus: Option<u16>,
    pub def_bonus: Option<u16>,
}

#[derive(Default, Clone)]
pub struct MapFieldFactoryOptions {
    pub position: Option<Position>,
    pub topology: Option<MapFieldTopology>,
    pub village_id: Option<u32>,
    pub player_id: Option<Uuid>,
    pub world_size: Option<i32>,
}

pub fn map_field_factory(options: MapFieldFactoryOptions) -> MapField {
    let position = options.position.unwrap_or(Position { x: 0, y: 0 });
    let topology = options
        .topology
        .unwrap_or(MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)));
    let world_size = options.world_size.unwrap_or(100);

    MapField {
        id: position.to_id(world_size),
        position,
        topology,
        village_id: options.village_id,
        player_id: options.player_id,
    }
}

pub fn user_factory(options: UserFactoryOptions) -> User {
    let default_email: String = format!("user_{}@example.com", rand::thread_rng().r#gen::<u32>());
    User::new(
        options.id.unwrap_or_else(Uuid::new_v4),
        options.email.map_or(default_email, |s| s.to_string()),
        Uuid::new_v4().to_string(),
    )
}

pub fn player_factory(options: PlayerFactoryOptions) -> Player {
    let default_username: String = format!("user_{}", rand::thread_rng().r#gen::<u32>());
    Player {
        id: options.id.unwrap_or_else(Uuid::new_v4),
        username: options.username.map_or(default_username, |s| s.to_string()),
        tribe: options.tribe.unwrap_or(Tribe::Roman),
        user_id: options.user_id.unwrap_or_else(Uuid::new_v4),
        alliance_id: None,
        alliance_role: None,
        alliance_join_time: None,
        current_alliance_training_contributions: 0,
        current_alliance_armor_contributions: 0,
        current_alliance_cp_contributions: 0,
        current_alliance_trade_contributions: 0,
        total_alliance_training_contributions: 0,
        total_alliance_armor_contributions: 0,
        total_alliance_cp_contributions: 0,
        total_alliance_trade_contributions: 0,
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
        options.world_size.unwrap_or(100),
        options.server_speed.unwrap_or(1),
    )
}

pub fn hero_factory(options: HeroFactoryOptions) -> Hero {
    Hero::new(
        options.id,
        options
            .village_id
            .unwrap_or(village_factory(Default::default()).id),
        options
            .player_id
            .unwrap_or(player_factory(Default::default()).id),
        options.tribe.unwrap_or(Tribe::Roman),
        None,
    )
}

pub fn army_factory(options: ArmyFactoryOptions) -> Army {
    let village_id = options.village_id.unwrap_or(1);

    Army::new(
        Some(Uuid::new_v4()),
        village_id,
        Some(village_id),
        options.player_id.unwrap_or(Uuid::new_v4()),
        options.tribe.unwrap_or(Tribe::Roman),
        &options.units.unwrap_or([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        &options.smithy.unwrap_or_default(),
        options.hero,
    )
}

pub fn setup_player_party(
    position: Option<Position>,
    tribe: Tribe,
    units: TroopSet,
    with_hero: bool,
) -> Result<(Player, Village, Army, Option<Hero>)> {
    let position = position.unwrap_or_else(|| {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(1..99);
        let y = rng.gen_range(1..99);
        Position { x, y }
    });

    let player = player_factory(PlayerFactoryOptions {
        tribe: Some(tribe.clone()),
        ..Default::default()
    });

    let valley = valley_factory(ValleyFactoryOptions {
        position: Some(position),
        ..Default::default()
    });

    let village = village_factory(VillageFactoryOptions {
        valley: Some(valley),
        player: Some(player.clone()),
        ..Default::default()
    });

    let hero = if with_hero {
        let hero = Hero::new(None, village.id, player.id, player.tribe.clone(), None);
        Some(hero)
    } else {
        None
    };

    let army = army_factory(ArmyFactoryOptions {
        player_id: Some(player.id),
        village_id: Some(village.id),
        units: Some(units),
        tribe: Some(tribe.clone()),
        hero: hero.clone(),
        ..Default::default()
    });

    Ok((player, village, army, hero))
}
