//! Village-owned read models.
//!
//! These types are app-facing views returned by village read use cases. They
//! are not ports: focused read ports load or assemble them, while use cases
//! decide which view belongs to each application workflow.

pub mod activity;
pub mod marketplace;
pub mod village_army;

pub use activity::{
    AcademyQueueItem, BuildingQueueItem, SmithyQueueItem, TrainingQueueItem, TrapQueueItem,
    TroopMovement, TroopMovementDirection, TroopMovementType, VillageQueues, VillageTroopMovements,
};
pub use marketplace::{MarketplaceData, MerchantMovement, MerchantMovementKind};
pub use village_army::VillageArmyStateView;
