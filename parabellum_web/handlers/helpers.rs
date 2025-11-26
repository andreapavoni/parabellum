use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::{
    SignedCookieJar,
    cookie::{Cookie, SameSite},
};
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

/// Generates a new CSRF token, puts it into a signed cookie,
/// and returns updated cookie jar.
pub fn generate_csrf(jar: SignedCookieJar) -> (SignedCookieJar, String) {
    let token = Uuid::new_v4().to_string();
    let cookie = Cookie::build(Cookie::new("csrf_token", token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .build();
    let jar = jar.add(cookie);
    (jar, token)
}

/// Verify CSRF token from form matches the one in the cookie.
pub fn validate_csrf(jar: &SignedCookieJar, form_token: &str) -> bool {
    jar.get("csrf_token")
        .map(|cookie| cookie.value() == form_token)
        .unwrap_or(false)
}
