pub mod common;
pub mod layout;
pub mod resources;
pub mod village;

pub use common::BuildingQueueItem;
pub use layout::{
    LayoutData, PageLayout, ResourceProduction, UserInfo, VillageCapacity, VillageHeaderData,
    VillageResources, wrap_in_html,
};
pub use resources::{
    ProductionInfo, QueueState, ResourceSlot, ResourcesPage, ResourcesPageData, TroopInfo,
    VillageInfo as ResourceVillageInfo,
};
pub use village::{
    BuildingSlot, QueueState as VillageQueueState, VillageInfo, VillagePage, VillagePageData,
};
