use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BuildingQueueItemView {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub target_level: u8,
    pub is_processing: bool,
    pub time_remaining: String,
    pub time_seconds: u32,
    pub queue_class: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ResourceCostView {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

impl From<ResourceGroup> for ResourceCostView {
    fn from(resources: ResourceGroup) -> Self {
        Self {
            lumber: resources.lumber(),
            clay: resources.clay(),
            iron: resources.iron(),
            crop: resources.crop(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ServerTimeContext {
    pub formatted: String,
    pub timestamp: i64,
}
