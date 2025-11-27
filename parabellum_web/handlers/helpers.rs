use askama::Template;
use axum::{
    extract::{Form, FromRef, FromRequest, FromRequestParts, Request, State},
    http::{StatusCode, request::Parts},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::{
    SignedCookieJar,
    cookie::{Cookie, Key, SameSite},
};
use std::future::Future;
use uuid::Uuid;

use parabellum_app::{cqrs::queries::GetUserById, queries_handlers::GetUserByIdHandler};
use parabellum_types::common::User as UserType;

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

/// Loads the currently authenticated user from the cookie.
/// Returns `Ok(User)` if found, or `Err(Redirect)` to redirect to /login.
pub async fn current_user(state: &AppState, jar: &SignedCookieJar) -> Result<UserType, Redirect> {
    if let Some(cookie) = jar.get("user_id") {
        let user_id = match Uuid::parse_str(cookie.value()) {
            Ok(id) => id,
            Err(_) => return Err(Redirect::to("/login")),
        };
        let query = GetUserById { id: user_id };

        match state.app_bus.query(query, GetUserByIdHandler::new()).await {
            Ok(user) => Ok(user),
            Err(_) => Err(Redirect::to("/login")),
        }
    } else {
        Err(Redirect::to("/login"))
    }
}

/// Ensures that the requester is not already authenticated.
/// If a `user_id` cookie is found, returns a redirect to `/village`.
pub fn ensure_not_authenticated(jar: &SignedCookieJar) -> Result<(), Redirect> {
    if jar.get("user_id").is_some() {
        Err(Redirect::to("/village"))
    } else {
        Ok(())
    }
}

/// Extractor for authenticated users.
/// Automatically loads the user from the cookie.
/// If no user is found or the user doesn't exist, returns a redirect to `/login`.
#[derive(Clone)]
pub struct User(pub UserType);

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
    Key: FromRef<S>,
    AppState: FromRef<S>,
{
    type Rejection = Redirect;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let jar = SignedCookieJar::from_request_parts(parts, state)
                .await
                .map_err(|_| Redirect::to("/login"))?;
            let app_state = State::<AppState>::from_request_parts(parts, state)
                .await
                .map_err(|_| Redirect::to("/login"))?;

            current_user(&app_state, &jar).await.map(User)
        }
    }
}
