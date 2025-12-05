use askama::Template;

use super::shared::TemplateLayout;

/// Template for the map page.
#[derive(Debug, Default, Template)]
#[template(path = "map/map.html")]
pub struct MapTemplate {
    pub layout: TemplateLayout,
    pub world_size: i32,
}
