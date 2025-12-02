use askama::Template;

use crate::handlers::CurrentUser;

use super::shared::ServerTimeContext;

/// Template for the home page.
#[derive(Debug, Template)]
#[template(path = "home/home.html")]
pub struct HomeTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub server_time: ServerTimeContext,
}
