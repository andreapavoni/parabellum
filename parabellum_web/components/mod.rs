pub mod common;
pub mod layout;
pub mod map;
pub mod reports;
pub mod resources;
pub mod village;

pub use common::{BuildingQueueItem, VillageInfo};
pub use layout::{LayoutData, PageLayout, wrap_in_html};
pub use map::MapPage;
pub use reports::{
    BattleReportPage, GenericReportData, GenericReportPage, ReportListEntry, ReportsPage,
};
pub use resources::{ResourceSlot, ResourcesPage};
pub use village::{BuildingSlot, VillageListItem, VillagePage};
