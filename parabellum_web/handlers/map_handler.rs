use axum::response::IntoResponse;

use crate::{
    handlers::{CurrentUser, render_template},
    templates::MapTemplate,
};

pub async fn map(user: CurrentUser) -> impl IntoResponse {
    let template = MapTemplate {
        current_user: Some(user),
        nav_active: "map",
    };
    render_template(template, None).into_response()
}
