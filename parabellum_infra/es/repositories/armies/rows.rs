//! Typed rows for army projection reads.

use parabellum_app::villages::{VillageArmyContext, projection_repositories::ArmyState};
use parabellum_game::models::{
    army::Army,
    hero::{Hero, HeroResourceFocus},
    smithy::SmithyUpgrades,
};
use parabellum_types::{
    army::TroopSet,
    errors::{ApplicationError, DbError},
};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub(super) struct DbArmyRow {
    army_id: Uuid,
    village_id: i32,
    current_village_id: i32,
    current_map_field_id: Option<i32>,
    player_id: Uuid,
    tribe: crate::persistence::models::Tribe,
    state: String,
    units: Vec<i32>,
    smithy_upgrades: Vec<i16>,
    hero_id: Option<Uuid>,
    hero_player_id: Option<Uuid>,
    hero_home_village_id: Option<i32>,
    hero_tribe: Option<crate::persistence::models::Tribe>,
    hero_level: Option<i16>,
    hero_health: Option<i16>,
    hero_experience: Option<i32>,
    hero_resource_focus: Option<Json<HeroResourceFocus>>,
    hero_strength_points: Option<i16>,
    hero_off_bonus_points: Option<i16>,
    hero_def_bonus_points: Option<i16>,
    hero_regeneration_points: Option<i16>,
    hero_resources_points: Option<i16>,
    hero_unassigned_points: Option<i16>,
}

impl DbArmyRow {
    pub(super) fn current_village_id(&self) -> u32 {
        self.current_village_id as u32
    }

    fn home_village_id(&self) -> u32 {
        self.village_id as u32
    }

    fn state(&self) -> Result<ArmyState, ApplicationError> {
        match self.state.as_str() {
            "home" => Ok(ArmyState::Home),
            "stationed" => Ok(ArmyState::Stationed),
            "moving" => Ok(ArmyState::Moving),
            "trapped" => Ok(ArmyState::Trapped),
            unknown => Err(ApplicationError::Db(DbError::Database(
                sqlx::Error::Protocol(format!("unknown army state '{unknown}'").into()),
            ))),
        }
    }
}

pub(super) fn army_context_from_rows(
    rows: Vec<DbArmyRow>,
    village_id: u32,
) -> Result<VillageArmyContext, ApplicationError> {
    let mut context = VillageArmyContext::default();

    for row in rows {
        let state = row.state()?;
        let home_village_id = row.home_village_id();
        let current_village_id = row.current_village_id();
        let is_home_village = home_village_id == village_id;
        let is_current_village = current_village_id == village_id;
        let is_deployed = current_village_id != home_village_id;
        let army = Army::try_from(row)?;

        match state {
            ArmyState::Home if is_home_village && is_current_village => {
                context.home.get_or_insert(army);
            }
            ArmyState::Stationed if is_current_village => {
                context.stationed.push(army);
            }
            ArmyState::Stationed if is_home_village && is_deployed => {
                context.deployed.push(army);
            }
            ArmyState::Moving if is_home_village => {
                context.moving.push(army);
            }
            ArmyState::Trapped if is_current_village => {
                context.trapped_here.push(army);
            }
            ArmyState::Trapped if is_home_village && is_deployed => {
                context.trapped_away.push(army);
            }
            _ => {}
        }
    }

    Ok(context)
}

impl TryFrom<DbArmyRow> for Army {
    type Error = ApplicationError;

    fn try_from(row: DbArmyRow) -> Result<Self, Self::Error> {
        let units = troop_set(row.units.clone());
        let smithy = smithy_upgrades(row.smithy_upgrades.clone());
        let hero = hero_from_row(&row)?;

        Ok(Army::new(
            Some(row.army_id),
            row.village_id as u32,
            row.current_map_field_id.map(|id| id as u32),
            row.player_id,
            row.tribe.into(),
            &units,
            &smithy,
            hero,
        ))
    }
}

fn hero_from_row(row: &DbArmyRow) -> Result<Option<Hero>, ApplicationError> {
    let Some(hero_id) = row.hero_id else {
        return Ok(None);
    };
    let health = required_hero_field(row.hero_health, "hero_health")?;
    if health <= 0 {
        return Ok(None);
    }

    Ok(Some(Hero {
        id: hero_id,
        player_id: required_hero_field(row.hero_player_id, "hero_player_id")?,
        village_id: required_hero_field(row.hero_home_village_id, "hero_home_village_id")? as u32,
        tribe: required_hero_field(row.hero_tribe, "hero_tribe")?.into(),
        level: required_hero_field(row.hero_level, "hero_level")? as u16,
        resource_focus: required_hero_field(
            row.hero_resource_focus.clone(),
            "hero_resource_focus",
        )?
        .0,
        health: health as u16,
        experience: required_hero_field(row.hero_experience, "hero_experience")? as u32,
        strength_points: required_hero_field(row.hero_strength_points, "hero_strength_points")?
            as u16,
        off_bonus_points: required_hero_field(row.hero_off_bonus_points, "hero_off_bonus_points")?
            as u16,
        def_bonus_points: required_hero_field(row.hero_def_bonus_points, "hero_def_bonus_points")?
            as u16,
        regeneration_points: required_hero_field(
            row.hero_regeneration_points,
            "hero_regeneration_points",
        )? as u16,
        resources_points: required_hero_field(row.hero_resources_points, "hero_resources_points")?
            as u16,
        unassigned_points: required_hero_field(
            row.hero_unassigned_points,
            "hero_unassigned_points",
        )? as u16,
    }))
}

fn required_hero_field<T>(value: Option<T>, field: &'static str) -> Result<T, ApplicationError> {
    value.ok_or_else(|| {
        ApplicationError::Db(DbError::Database(sqlx::Error::Protocol(
            format!("army row references a hero without {field}").into(),
        )))
    })
}

fn troop_set(values: Vec<i32>) -> TroopSet {
    let mut units = [0_u32; 10];
    for (idx, value) in values.into_iter().take(10).enumerate() {
        units[idx] = value.max(0) as u32;
    }
    TroopSet::new(units)
}

fn smithy_upgrades(values: Vec<i16>) -> SmithyUpgrades {
    let mut upgrades = [0_u8; 8];
    for (idx, value) in values.into_iter().take(8).enumerate() {
        upgrades[idx] = value.max(0) as u8;
    }
    upgrades
}
