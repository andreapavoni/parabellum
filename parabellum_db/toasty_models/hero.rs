use uuid::Uuid;

use parabellum_game::models::hero::Hero;
use parabellum_types::errors::{ApplicationError, DbError};

use crate::{mapping::tribe_to_db_code, models as db_models};

#[derive(Debug, Clone, toasty::Model)]
#[table = "heroes"]
pub struct HeroDbRow {
    #[key]
    pub id: Uuid,

    #[index]
    pub player_id: Uuid,

    #[index]
    pub village_id: i32,

    pub tribe: i64,
    pub level: i16,
    pub health: i16,
    pub experience: i32,

    #[serialize(json)]
    pub resource_focus: serde_json::Value,

    pub strength_points: i32,
    pub off_bonus_points: i16,
    pub def_bonus_points: i16,
    pub regeneration_points: i16,
    pub resources_points: i16,
    pub unassigned_points: i16,
}

impl TryFrom<&Hero> for HeroDbRow {
    type Error = ApplicationError;

    fn try_from(hero: &Hero) -> Result<Self, Self::Error> {
        Ok(Self {
            id: hero.id,
            player_id: hero.player_id,
            village_id: hero.village_id as i32,
            tribe: tribe_to_db_code(&hero.tribe),
            level: i16::try_from(hero.level).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero level overflow for {}: {}",
                    hero.id, hero.level
                )))
            })?,
            health: i16::try_from(hero.health).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero health overflow for {}: {}",
                    hero.id, hero.health
                )))
            })?,
            experience: i32::try_from(hero.experience).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero experience overflow for {}: {}",
                    hero.id, hero.experience
                )))
            })?,
            resource_focus: serde_json::to_value(hero.resource_focus).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid hero resource_focus for {}: {}",
                    hero.id, e
                )))
            })?,
            strength_points: i32::from(hero.strength_points),
            off_bonus_points: i16::try_from(hero.off_bonus_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero off_bonus_points overflow for {}: {}",
                    hero.id, hero.off_bonus_points
                )))
            })?,
            def_bonus_points: i16::try_from(hero.def_bonus_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero def_bonus_points overflow for {}: {}",
                    hero.id, hero.def_bonus_points
                )))
            })?,
            regeneration_points: i16::try_from(hero.regeneration_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero regeneration_points overflow for {}: {}",
                    hero.id, hero.regeneration_points
                )))
            })?,
            resources_points: i16::try_from(hero.resources_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero resources_points overflow for {}: {}",
                    hero.id, hero.resources_points
                )))
            })?,
            unassigned_points: i16::try_from(hero.unassigned_points).map_err(|_| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "hero unassigned_points overflow for {}: {}",
                    hero.id, hero.unassigned_points
                )))
            })?,
        })
    }
}

impl From<HeroDbRow> for db_models::Hero {
    fn from(hero: HeroDbRow) -> Self {
        Self {
            id: hero.id,
            player_id: hero.player_id,
            village_id: hero.village_id,
            tribe: hero.tribe,
            level: hero.level,
            health: hero.health,
            experience: hero.experience,
            resource_focus: hero.resource_focus,
            strength_points: hero.strength_points,
            off_bonus_points: hero.off_bonus_points,
            def_bonus_points: hero.def_bonus_points,
            regeneration_points: hero.regeneration_points,
            resources_points: hero.resources_points,
            unassigned_points: hero.unassigned_points,
        }
    }
}
