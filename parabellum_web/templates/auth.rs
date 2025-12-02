use askama::Template;

use crate::handlers::CurrentUser;

use super::shared::ServerTimeContext;

/// Template for the login page.
#[derive(Debug, Default, Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub csrf_token: String,
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub email_value: String,
    pub error: Option<String>,
    pub server_time: ServerTimeContext,
}

/// Template for the registration page.
#[derive(Debug, Default, Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub csrf_token: String,
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub username_value: String,
    pub email_value: String,
    pub selected_tribe: String,
    pub selected_quadrant: String,
    pub error: Option<String>,
    pub server_time: ServerTimeContext,
}
