use axum::response::IntoResponse;

use crate::{
    handlers::{CurrentUser, render_template},
    templates::{ResourcesTemplate, VillageTemplate},
};

pub async fn village(user: CurrentUser) -> impl IntoResponse {
    let template = VillageTemplate {
        current_user: Some(user),
        nav_active: "village",
    };
    render_template(template, None).into_response()
}

pub async fn resources(user: CurrentUser) -> impl IntoResponse {
    let template = ResourcesTemplate {
        current_user: Some(user),
        nav_active: "resources",
    };
    render_template(template, None).into_response()
}
