pub mod layout;
pub mod resources;

pub use layout::{
    LayoutBody, LayoutData, ResourceProduction, UserInfo, VillageCapacity, VillageHeaderData,
    VillageResources, wrap_in_html_shell,
};
pub use resources::{
    BuildingQueueItem, ProductionInfo, QueueState, ResourceSlot, ResourcesPage, ResourcesPageData,
    TroopInfo, VillageInfo,
};
