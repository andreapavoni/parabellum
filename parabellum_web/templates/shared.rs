use parabellum_types::{buildings::BuildingName, common::ResourceGroup};
use uuid::Uuid;

use crate::{handlers::CurrentUser, view_helpers::server_time};

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
pub struct ServerTime {
    pub formatted: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct TemplateLayout {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub server_time: ServerTime,
}

impl TemplateLayout {
    pub fn new(current_user: Option<CurrentUser>, nav_active: &'static str) -> Self {
        Self {
            current_user,
            nav_active,
            server_time: server_time(),
        }
    }
}

impl Default for TemplateLayout {
    fn default() -> Self {
        Self::new(None, "home")
    }
}
