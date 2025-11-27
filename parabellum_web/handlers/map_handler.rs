use axum::response::IntoResponse;

use crate::{
    handlers::{User, render_template},
    templates::MapTemplate,
};

pub async fn map(_user: User) -> impl IntoResponse {
    let template = MapTemplate { current_user: true };
    render_template(template, None).into_response()
}
