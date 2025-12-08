pub mod buildings;
pub mod common;
pub mod layout;
pub mod map;
pub mod reports;
pub mod resources;
pub mod village;

pub use buildings::{
    AcademyPage, AcademyQueueItem, AcademyResearchOption, BuildingOption, EmptySlotPage,
    GenericBuildingPage, MissingRequirements, RallyPointPage, ResourceCost, ResourceFieldPage,
    SmithyPage, SmithyQueueItem, SmithyUpgradeOption, TrainingBuildingPage, TrainingQueueItem,
    UnitTrainingOption, UpgradeBlock,
};
pub use common::{
    BuildingQueueItem, MovementDirection, MovementKind, RallyPointUnit, TroopCount, TroopMovement,
    VillageInfo,
};
pub use layout::{LayoutData, PageLayout, wrap_in_html};
pub use map::MapPage;
pub use reports::{
    BattleReportPage, GenericReportData, GenericReportPage, ReportListEntry, ReportsPage,
};
pub use resources::{ResourceSlot, ResourcesPage};
pub use village::{BuildingSlot, VillageListItem, VillagePage};
