use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};
use uuid::Uuid;

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

/// Genera un nuovo CSRF token e lo salva in un cookie firmato
pub fn generate_csrf_token(jar: SignedCookieJar) -> (SignedCookieJar, String) {
    let token = Uuid::new_v4().to_string();
    let cookie = Cookie::build(("csrf_token", token.clone()))
        .path("/")
        .http_only(false) // Deve essere accessibile da JavaScript se necessario
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .build();
    let updated_jar = jar.add(cookie);
    (updated_jar, token)
}

/// Valida il CSRF token confrontandolo con quello nel cookie
pub fn validate_csrf_token(jar: SignedCookieJar, form_token: &str) -> bool {
    if let Some(cookie) = jar.get("csrf_token") {
        cookie.value() == form_token
    } else {
        false
    }
}

/// Ottiene il CSRF token dal cookie se presente
pub fn get_csrf_token(jar: &SignedCookieJar) -> Option<String> {
    jar.get("csrf_token").map(|cookie| cookie.value().to_string())
}
