use crate::{
    components::{
        LayoutData, ResourceProduction, UserInfo, VillageCapacity, VillageHeaderData,
        VillageResources,
    },
    handlers::CurrentUser,
};
use chrono::Utc;

/// Helper to create layout data from current user
pub fn create_layout_data(user: &CurrentUser, nav_active: &str) -> LayoutData {
    LayoutData {
        user: Some(UserInfo {
            username: user.player.username.clone(),
        }),
        village: Some(VillageHeaderData {
            resources: VillageResources {
                lumber: user.village.stored_resources().lumber(),
                clay: user.village.stored_resources().clay(),
                iron: user.village.stored_resources().iron(),
                crop: user.village.stored_resources().crop(),
            },
            production: ResourceProduction {
                lumber: user.village.production.effective.lumber,
                clay: user.village.production.effective.clay,
                iron: user.village.production.effective.iron,
                crop: user.village.production.effective.crop as u32,
            },
            capacity: VillageCapacity {
                warehouse: user.village.warehouse_capacity(),
                granary: user.village.granary_capacity(),
            },
            population: user.village.population,
        }),
        server_time: Utc::now().timestamp(),
        nav_active: nav_active.to_string(),
    }
}
