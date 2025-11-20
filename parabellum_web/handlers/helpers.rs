use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

/// Helper: render a Template to HTML or return 500 on error
pub fn render_template<T: Template>(template: T, status: Option<StatusCode>) -> impl IntoResponse {
    match template.render() {
        Ok(html) => {
            let status = status.unwrap_or(StatusCode::OK);
            (status, Html(html)).into_response()
        }
        Err(err) => {
            tracing::error!("Template render error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response()
        }
    }
}
