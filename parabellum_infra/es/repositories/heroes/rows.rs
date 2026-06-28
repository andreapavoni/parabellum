//! Typed rows for hero projection reads.

use parabellum_game::models::hero::{Hero, HeroResourceFocus};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub(super) struct DbHeroRow {
    hero_id: Uuid,
    player_id: Uuid,
    home_village_id: i32,
    tribe: crate::persistence::models::Tribe,
    level: i16,
    health: i16,
    experience: i32,
    resource_focus: Json<HeroResourceFocus>,
    strength_points: i16,
    off_bonus_points: i16,
    def_bonus_points: i16,
    regeneration_points: i16,
    resources_points: i16,
    unassigned_points: i16,
}

impl From<DbHeroRow> for Hero {
    fn from(row: DbHeroRow) -> Self {
        Self {
            id: row.hero_id,
            player_id: row.player_id,
            village_id: row.home_village_id as u32,
            tribe: row.tribe.into(),
            level: row.level as u16,
            resource_focus: row.resource_focus.0,
            health: row.health as u16,
            experience: row.experience as u32,
            strength_points: row.strength_points as u16,
            off_bonus_points: row.off_bonus_points as u16,
            def_bonus_points: row.def_bonus_points as u16,
            regeneration_points: row.regeneration_points as u16,
            resources_points: row.resources_points as u16,
            unassigned_points: row.unassigned_points as u16,
        }
    }
}
