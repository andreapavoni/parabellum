use askama::Template;

/// Template for the home page
#[derive(Debug, Default, Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub current_user: bool,
}

/// Template for the login page
#[derive(Debug, Default, Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub current_user: bool,
    pub email_value: String,   // to pre-fill email input
    pub error: Option<String>, // login error message, if any
}

/// Template for the registration page
#[derive(Debug, Default, Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {
    pub current_user: bool,
    pub username_value: String,    // to pre-fill username on error
    pub email_value: String,       // to pre-fill email on error
    pub selected_tribe: String,    // to retain selected tribe option
    pub selected_quadrant: String, // to retain selected quadrant option
    pub error: Option<String>,     // signup error message, if any
}

/// Template for the village center page
#[derive(Debug, Default, Template)]
#[template(path = "village.html")]
pub struct VillageTemplate {
    pub current_user: bool,
}

/// Template for the village center page
#[derive(Debug, Default, Template)]
#[template(path = "resources.html")]
pub struct ResourcesTemplate {
    pub current_user: bool,
}
