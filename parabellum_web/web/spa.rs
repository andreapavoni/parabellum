use axum::response::{Html, IntoResponse};

const SPA_SHELL: &str = include_str!("../templates/spa_shell.html");

pub async fn spa_shell() -> impl IntoResponse {
    Html(SPA_SHELL)
}
