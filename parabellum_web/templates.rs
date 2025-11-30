use askama::Template;
use parabellum_game::models::village::VillageBuilding;
use parabellum_types::{buildings::BuildingName, common::ResourceGroup};

use crate::handlers::CurrentUser;

/// Template for the home page.
#[derive(Debug, Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
}

/// Template for the login page.
#[derive(Debug, Default, Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub csrf_token: String,
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub email_value: String,   // to pre-fill email input
    pub error: Option<String>, // login error message, if any
}

/// Template for the registration page.
#[derive(Debug, Default, Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {
    pub csrf_token: String,
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub username_value: String,    // to pre-fill username on error
    pub email_value: String,       // to pre-fill email on error
    pub selected_tribe: String,    // to retain selected tribe option
    pub selected_quadrant: String, // to retain selected quadrant option
    pub error: Option<String>,     // signup error message, if any
}

/// Template for the village center page.
#[derive(Debug, Default, Template)]
#[template(path = "village.html")]
pub struct VillageTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
}

#[derive(Debug, Clone)]
pub struct ResourceField {
    pub class: &'static str,
    pub name: BuildingName,
    pub level: u8,
}

/// Template for the village center page.
#[derive(Debug, Template)]
#[template(path = "resources.html")]
pub struct ResourcesTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub resource_slots: Vec<ResourceField>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct BuildingOption {
    pub name: BuildingName,
    pub key: String,
    pub cost: ResourceCostView,
    pub upkeep: u32,
    pub time_formatted: String,
}

#[derive(Debug, Clone)]
pub struct BuildingUpgradeInfo {
    pub next_level: u8,
    pub cost: ResourceCostView,
    pub current_upkeep: u32,
    pub upkeep: u32,
    pub time_formatted: String,
}

/// Template for individual building page.
#[derive(Debug, Default, Template)]
#[template(path = "building.html")]
pub struct BuildingTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub slot_id: u8,
    pub slot_building: Option<VillageBuilding>,
    pub available_buildings: Vec<BuildingOption>,
    pub upgrade: Option<BuildingUpgradeInfo>,
    pub current_upkeep: Option<u32>,
    pub csrf_token: String,
    pub flash_error: Option<String>,
}

/// Template for the map page.
#[derive(Debug, Default, Template)]
#[template(path = "map.html")]
pub struct MapTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub world_size: i32,
}
