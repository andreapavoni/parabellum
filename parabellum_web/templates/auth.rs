use askama::Template;

use super::shared::TemplateLayout;

/// Template for the login page.
#[derive(Debug, Default, Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub csrf_token: String,
    pub layout: TemplateLayout,
    pub email_value: String,
    pub error: Option<String>,
}

/// Template for the registration page.
#[derive(Debug, Default, Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub csrf_token: String,
    pub layout: TemplateLayout,
    pub username_value: String,
    pub email_value: String,
    pub selected_tribe: String,
    pub selected_quadrant: String,
    pub error: Option<String>,
}
