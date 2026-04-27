use uuid::Uuid;

use parabellum_game::models::army::Army;
use parabellum_types::errors::{ApplicationError, DbError};

use crate::{
    mapping::tribe_to_db_code,
    models as db_models,
    toasty_models::hero::HeroDbRow,
};

#[derive(Debug, Clone, toasty::Model)]
#[table = "armies"]
pub struct ArmyDbRow {
    #[key]
    pub id: Uuid,

    #[index]
    pub village_id: i32,

    #[index]
    pub player_id: Uuid,

    #[index]
    pub current_map_field_id: Option<i32>,

    pub tribe: i64,

    #[serialize(json)]
    pub units: serde_json::Value,

    #[serialize(json)]
    pub smithy: serde_json::Value,

    #[index]
    pub hero_id: Option<Uuid>,
}

impl TryFrom<&Army> for ArmyDbRow {
    type Error = ApplicationError;

    fn try_from(army: &Army) -> Result<Self, Self::Error> {
        Ok(Self {
            id: army.id,
            village_id: army.village_id as i32,
            player_id: army.player_id,
            current_map_field_id: army.current_map_field_id.map(|id| id as i32),
            tribe: tribe_to_db_code(&army.tribe),
            units: serde_json::to_value(army.units()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid army units payload for {}: {}",
                    army.id, e
                )))
            })?,
            smithy: serde_json::to_value(army.smithy()).map_err(|e| {
                ApplicationError::Db(DbError::Transaction(format!(
                    "invalid army smithy payload for {}: {}",
                    army.id, e
                )))
            })?,
            hero_id: army.hero().map(|hero| hero.id),
        })
    }
}

pub fn into_db_army(army: ArmyDbRow, hero: Option<HeroDbRow>) -> Result<db_models::Army, ApplicationError> {
    let hero_resource_focus = hero.as_ref().map(|row| row.resource_focus.clone());
    let hero_level = hero.as_ref().map(|row| row.level);
    let hero_health = hero.as_ref().map(|row| row.health);
    let hero_experience = hero.as_ref().map(|row| row.experience);
    let hero_strength_points = hero
        .as_ref()
        .map(|row| i16::try_from(row.strength_points))
        .transpose()
        .map_err(|_| {
            ApplicationError::Db(DbError::Transaction(format!(
                "hero strength_points overflow for army {}",
                army.id
            )))
        })?;
    let hero_off_bonus_points = hero.as_ref().map(|row| row.off_bonus_points);
    let hero_def_bonus_points = hero.as_ref().map(|row| row.def_bonus_points);
    let hero_resources_points = hero.as_ref().map(|row| row.resources_points);
    let hero_regeneration_points = hero.as_ref().map(|row| row.regeneration_points);
    let hero_unassigned_points = hero.as_ref().map(|row| row.unassigned_points);

    Ok(db_models::Army {
        id: army.id,
        village_id: army.village_id,
        player_id: army.player_id,
        current_map_field_id: army.current_map_field_id,
        tribe: army.tribe,
        units: army.units,
        smithy: army.smithy,
        hero_id: army.hero_id,
        hero_level,
        hero_resource_focus,
        hero_health,
        hero_experience,
        hero_strength_points,
        hero_off_bonus_points,
        hero_def_bonus_points,
        hero_resources_points,
        hero_regeneration_points,
        hero_unassigned_points,
    })
}
