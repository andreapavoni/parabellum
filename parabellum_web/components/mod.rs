pub mod common;
pub mod layout;
pub mod map;
pub mod reports;
pub mod resources;
pub mod village;

pub use common::{BuildingQueueItem, VillageInfo};
pub use layout::{LayoutData, PageLayout, wrap_in_html};
pub use map::{MapPage, MapPageData};
pub use reports::{
    BattleReportData, BattleReportPage, GenericReportData, GenericReportPage, ReportListEntry,
    ReportsPage, ReportsPageData,
};
pub use resources::{ProductionInfo, ResourceSlot, ResourcesPage, ResourcesPageData, TroopInfo};
pub use village::{BuildingSlot, VillageListItem, VillagePage, VillagePageData};
