use askama::Template;

use crate::handlers::CurrentUser;

use super::shared::ServerTimeContext;

/// Template for the map page.
#[derive(Debug, Default, Template)]
#[template(path = "map/map.html")]
pub struct MapTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub world_size: i32,
    pub server_time: ServerTimeContext,
}
