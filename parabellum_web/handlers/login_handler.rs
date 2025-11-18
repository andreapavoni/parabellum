use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};

use parabellum_app::{cqrs::queries::AuthenticateUser, queries_handlers::AuthenticateUserHandler};
use parabellum_core::{AppError, ApplicationError};

use crate::{handlers::render_template, http::AppState, templates::LoginTemplate};

/// Form for login.
#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

/// GET /login – Show the login form.
pub async fn login_page(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    // If already logged in (cookie present), redirect to home instead of showing form
    if let Some(_cookie) = jar.get("user_email") {
        return Redirect::to("/").into_response();
    }
    // Render login form with no error and no pre-filled email
    render_template(LoginTemplate::default()).into_response()
}

/// POST /login – Handle login form submission.
pub async fn login(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    // Attempt to authenticate user via the application layer
    let query = AuthenticateUser {
        email: form.email.clone(),
        password: form.password.clone(),
    };
    match state
        .app_bus
        .query(query, AuthenticateUserHandler::new())
        .await
    {
        Ok(user) => {
            // Authentication successful – set a signed cookie and redirect to home
            let cookie = Cookie::new("user_email", user.email.clone());
            let jar = jar.add(cookie);
            return (jar, Redirect::to("/")).into_response();
        }
        Err(ApplicationError::App(AppError::WrongAuthCredentials)) => {
            // Invalid credentials – re-render login form with error message
            let template = LoginTemplate {
                email_value: form.email.clone(),
                error: Some("Invalid email or password.".to_string()),
                ..Default::default()
            };
            return render_template(template).into_response();
        }
        Err(e) => {
            // Other errors (e.g., database issues)
            tracing::error!("Login error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response();
        }
    }
}
