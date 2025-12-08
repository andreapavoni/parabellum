use crate::{components::LayoutData, handlers::CurrentUser};
use chrono::Utc;

/// Helper to create layout data from current user
pub fn create_layout_data(user: &CurrentUser, nav_active: &str) -> LayoutData {
    LayoutData {
        player: Some(user.player.clone()),
        village: Some(user.village.clone()),
        server_time: Utc::now().timestamp(),
        nav_active: nav_active.to_string(),
    }
}
