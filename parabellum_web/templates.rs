mod auth;
mod home;
mod map;
mod shared;
mod village;

pub use auth::{LoginTemplate, RegisterTemplate};
pub use home::HomeTemplate;
pub use map::MapTemplate;
pub use shared::{BuildingQueueItemView, ServerTime, TemplateLayout};
pub use village::{
    AcademyResearchOption, AcademyResearchQueueItemView, AcademyTemplate, BarracksTemplate,
    BuildingOption, BuildingPageContext, BuildingRequirementView, BuildingUpgradeInfo,
    EmptySlotTemplate, GenericBuildingTemplate, ResourceField, ResourceFieldTemplate,
    ResourcesTemplate, SmithyQueueItemView, SmithyTemplate, SmithyUpgradeOption, StableTemplate,
    TroopCountView, UnitTrainingOption, UnitTrainingQueueItemView, VillageTemplate,
    WorkshopTemplate,
};
