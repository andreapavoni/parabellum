use askama::Template;

use crate::handlers::CurrentUser;

/// Template for the home page.
#[derive(Debug, Default, Template)]
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

/// Template for the village center page.
#[derive(Debug, Default, Template)]
#[template(path = "resources.html")]
pub struct ResourcesTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
}

/// Template for the map page.
#[derive(Debug, Default, Template)]
#[template(path = "map.html")]
pub struct MapTemplate {
    pub current_user: Option<CurrentUser>,
    pub nav_active: &'static str,
    pub world_size: i32,
}
