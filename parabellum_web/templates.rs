mod auth;
mod home;
mod map;
mod reports;
mod shared;
mod village;

pub use auth::{LoginTemplate, RegisterTemplate};
pub use home::HomeTemplate;
pub use map::MapTemplate;
pub use reports::{ReportListEntry, ReportsTemplate};
pub use shared::{BuildingQueueItemView, ServerTime, TemplateLayout};
pub use village::{
    AcademyResearchOption, AcademyResearchQueueItemView, AcademyTemplate, BarracksTemplate,
    BuildingOption, BuildingPageContext, BuildingRequirementView, BuildingUpgradeInfo,
    EmptySlotTemplate, GenericBuildingTemplate, MovementDirectionView, MovementKindView,
    RallyPointTemplate, RallyPointUnitView, ResourceField, ResourceFieldTemplate,
    ResourcesTemplate, SmithyQueueItemView, SmithyTemplate, SmithyUpgradeOption, StableTemplate,
    TroopCountView, TroopMovementView, UnitTrainingOption, UnitTrainingQueueItemView,
    VillageTemplate, WorkshopTemplate,
};
