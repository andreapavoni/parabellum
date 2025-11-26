use askama::Template;
use axum::{
    extract::{Form, FromRef, FromRequest, Request},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::{
    SignedCookieJar,
    cookie::{Cookie, Key, SameSite},
};
use std::future::Future;
use uuid::Uuid;

use parabellum_app::{cqrs::queries::GetUserByEmail, queries_handlers::GetUserByEmailHandler};
use parabellum_types::common::User;

use crate::http::AppState;

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

/// Trait that exposes a CSRF token field on a form type.
pub trait HasCsrfToken {
    fn csrf_token(&self) -> &str;
}

/// Extractor that wraps a form type and enforces CSRF validation.
/// On success it yields the parsed form and the cookie jar.
/// On failure it returns a 400 response.
pub struct CsrfForm<T> {
    pub inner: T,
    pub jar: SignedCookieJar,
}

impl<S, T> FromRequest<S> for CsrfForm<T>
where
    S: Send + Sync,
    Key: FromRef<S>,
    T: HasCsrfToken + serde::de::DeserializeOwned + Send,
{
    type Rejection = Response;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            use axum::extract::FromRequest;

            // Extract both cookie jar and form in one go
            let (jar, Form(inner)) =
                match <(SignedCookieJar, Form<T>) as FromRequest<S>>::from_request(req, state).await
                {
                    Ok(v) => v,
                    Err(rejection) => return Err(rejection.into_response()),
                };

            if !validate_csrf(&jar, inner.csrf_token()) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid form token. Please try again.",
                )
                    .into_response());
            }

            Ok(CsrfForm { inner, jar })
        }
    }
}

/// Helper: load the currently authenticated user from the `user_email` cookie.
/// Returns `Ok(User)` if found, or `Err(Redirect)` to redirect to /login.
pub async fn current_user(state: &AppState, jar: &SignedCookieJar) -> Result<User, Redirect> {
    if let Some(cookie) = jar.get("user_email") {
        let email = cookie.value().to_string();
        let query = GetUserByEmail {
            email: email.clone(),
        };

        match state
            .app_bus
            .query(query, GetUserByEmailHandler::new())
            .await
        {
            Ok(user) => Ok(user),
            Err(_) => Err(Redirect::to("/login")),
        }
    } else {
        Err(Redirect::to("/login"))
    }
}
