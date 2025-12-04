mod auth;
mod home;
mod map;
mod shared;
mod village;

pub use auth::{LoginTemplate, RegisterTemplate};
pub use home::HomeTemplate;
pub use map::MapTemplate;
pub use shared::{BuildingQueueItemView, ServerTime};
pub use village::{
    BuildingOption, BuildingTemplate, BuildingUpgradeInfo, ResourceField, ResourcesTemplate,
    UnitTrainingOption, UnitTrainingQueueItemView, VillageTemplate,
};
