use axum::response::IntoResponse;

use crate::{
    handlers::{User, render_template},
    templates::{ResourcesTemplate, VillageTemplate},
};

pub async fn village(_user: User) -> impl IntoResponse {
    let template = VillageTemplate { current_user: true };
    render_template(template, None).into_response()
}

pub async fn resources(_user: User) -> impl IntoResponse {
    let template = ResourcesTemplate { current_user: true };
    render_template(template, None).into_response()
}
