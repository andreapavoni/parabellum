pub mod common;
pub mod layout;
pub mod resources;
pub mod village;

pub use common::{BuildingQueueItem, VillageInfo};
pub use layout::{LayoutData, PageLayout, wrap_in_html};
pub use resources::{ProductionInfo, ResourceSlot, ResourcesPage, ResourcesPageData, TroopInfo};
pub use village::{BuildingSlot, VillageListItem, VillagePage, VillagePageData};
