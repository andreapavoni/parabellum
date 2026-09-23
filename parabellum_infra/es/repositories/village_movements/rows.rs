//! Typed rows and database enum adapters for village movement projections.

use parabellum_app::villages::models::{MovementDirection, MovementType, VillageMovement};
use sqlx::{FromRow, types::Json};

#[derive(Debug, Clone, FromRow)]
pub(super) struct DbVillageMovementRow {
    village_id: i32,
    payload: Json<VillageMovement>,
}

impl From<DbVillageMovementRow> for VillageMovement {
    fn from(row: DbVillageMovementRow) -> Self {
        let mut movement = row.payload.0;
        movement.viewing_village_id = row.village_id.max(0) as u32;
        movement
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "movement_type", rename_all = "PascalCase")]
pub(super) enum DbMovementType {
    Attack,
    Raid,
    Scout,
    Reinforcement,
    Return,
    FoundVillage,
}

impl From<MovementType> for DbMovementType {
    fn from(value: MovementType) -> Self {
        match value {
            MovementType::Attack => Self::Attack,
            MovementType::Raid => Self::Raid,
            MovementType::Scout => Self::Scout,
            MovementType::Reinforcement => Self::Reinforcement,
            MovementType::Return => Self::Return,
            MovementType::FoundVillage => Self::FoundVillage,
        }
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "movement_direction", rename_all = "PascalCase")]
pub(super) enum DbMovementDirection {
    Incoming,
    Outgoing,
}

impl From<MovementDirection> for DbMovementDirection {
    fn from(value: MovementDirection) -> Self {
        match value {
            MovementDirection::Incoming => Self::Incoming,
            MovementDirection::Outgoing => Self::Outgoing,
        }
    }
}
