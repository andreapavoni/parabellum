use askama::Template;

use super::shared::TemplateLayout;

/// Template for the home page.
#[derive(Debug, Template)]
#[template(path = "home/home.html")]
pub struct HomeTemplate {
    pub layout: TemplateLayout,
}
