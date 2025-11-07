use thiserror::Error;
use uuid::Uuid;

use parabellum_types::{army::UnitName, buildings::BuildingName, tribe::Tribe};

/// Errors for domain logic (game rules).
#[derive(Debug, Error)]
pub enum GameError {
    #[error("Not enough resources")]
    NotEnoughResources,

    #[error("Not enough merchants available")]
    NotEnoughMerchants,

    #[error("Village slots are full")]
    VillageSlotsFull,

    #[error("Slot {slot_id} is already occupied")]
    SlotOccupied { slot_id: u8 },

    #[error("No buildings found on {slot_id}")]
    EmptySlot { slot_id: u8 },

    #[error("Building requirements not met: requires {building:?} at level {level}")]
    BuildingRequirementsNotMet { building: BuildingName, level: u8 },

    #[error("Building {building:?} not compatible with {tribe:?} tribe")]
    BuildingTribeMismatch {
        building: BuildingName,
        tribe: Tribe,
    },

    #[error("Building {0:?} can only be built in capital")]
    CapitalConstraint(BuildingName),

    #[error("Building {0:?} can't be built in capital")]
    NonCapitalConstraint(BuildingName),

    #[error("Building {0:?} is in conflict with {1:?}")]
    BuildingConflict(BuildingName, BuildingName),

    #[error("Building {0:?} can only be built once")]
    NoMultipleBuildingConstraint(BuildingName),

    #[error("Must complete other {0:?} to max level")]
    MultipleBuildingMaxNotReached(BuildingName),

    #[error("MapField {0} is not a oasis")]
    InvalidOasis(u32),

    #[error("Village {village_id} not owned by player {player_id:?}")]
    VillageNotOwned { village_id: u32, player_id: Uuid },

    #[error("Building has already reached max level")]
    BuildingMaxLevelReached,

    #[error("Cannot merge armies of different tribes")]
    TribeMismatch,

    #[error("Not enough units available to deploy")]
    NotEnoughUnits,

    #[error("No units selected to deploy")]
    NotUnitsSelected,

    #[error("Only scout units can be used for a scout mission")]
    OnlyScoutUnitsAllowed,

    #[error("Unit {0:?} is already researched in Academy")]
    UnitAlreadyResearched(UnitName),

    #[error("Unit {0:?} not yet researched in Academy")]
    UnitNotResearched(UnitName),

    #[error("Unit {0:?} not found for this tribe")]
    UnitNotFound(UnitName),

    #[error("Smithy upgrade level cannot exceed 20")]
    SmithyMaxLevelReached,

    #[error("Invalid smithy level: {0}")]
    InvalidSmithyLevel(u8),

    #[error("{0} is an invalid level for {1:?}")]
    InvalidBuildingLevel(u8, BuildingName),

    #[error("Invalid unit index: {0}")]
    InvalidUnitIndex(u8),

    #[error("Invalid valley with id {0}")]
    InvalidValley(u32),
}
