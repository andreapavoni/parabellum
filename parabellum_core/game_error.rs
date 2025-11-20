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

    #[error("Not enough hero points for next level")]
    NotEnoughHeroPoints,

    #[error("Hero attribute over limit 100")]
    HeroAttributeOverflow,

    #[error("Hero is not dead")]
    HeroNotDead,

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
    #[error("Building {0:?} not found")]
    BuildingNotFound(BuildingName),

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

    #[error("Hero {hero_id:?} not owned by player {player_id:?}")]
    HeroNotOwned { hero_id: Uuid, player_id: Uuid },

    #[error("Building has already reached max level")]
    BuildingMaxLevelReached,

    #[error("Cannot merge armies of different tribes")]
    TribeMismatch,

    #[error("Not enough units available to deploy")]
    NotEnoughUnits,

    #[error("No units selected to deploy")]
    NoUnitsSelected,

    #[error("Only scout units can be used for a scout mission")]
    OnlyScoutUnitsAllowed,

    #[error("Can't use {0:?} to train {1:?}")]
    InvalidTrainingBuilding(BuildingName, UnitName),

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

    #[error("Hero {hero_id:?} not in the village {village_id}")]
    HeroNotAtHome { hero_id: Uuid, village_id: u32 },

    #[error("Invalid valley with id {0}")]
    InvalidValley(u32),

    // Alliance errors
    #[error("Player is already in an alliance")]
    PlayerAlreadyInAlliance,

    #[error("Player is not in an alliance")]
    PlayerNotInAlliance,

    #[error("No invitation found for this player and alliance")]
    InvitationNotFound,

    #[error("Player already has a pending invitation from this alliance")]
    InvitationAlreadyExists,

    #[error("Alliance is full")]
    AllianceFull,

    #[error("Alliance tag already exists")]
    AllianceTagAlreadyExists,

    #[error("Alliance name already exists")]
    AllianceNameAlreadyExists,

    #[error("Player does not have permission to invite")]
    NoInvitePermission,

    #[error("Player does not have permission to kick")]
    NoKickPermission,

    #[error("Cannot kick the alliance leader")]
    CannotKickLeader,

    #[error("Player is not the alliance leader")]
    NotAllianceLeader,

    #[error("Player is already the alliance leader")]
    PlayerAlreadyLeader,
}
