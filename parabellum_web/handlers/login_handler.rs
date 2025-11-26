use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};

use parabellum_app::{cqrs::queries::AuthenticateUser, queries_handlers::AuthenticateUserHandler};
use parabellum_types::errors::{AppError, ApplicationError, DbError};

use crate::{handlers::{render_template, generate_csrf_token, validate_csrf_token}, http::AppState, templates::LoginTemplate};

/// Form for login.
#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub csrf_token: String,
}

/// GET /login – Show the login form.
pub async fn login_page(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Some(_cookie) = jar.get("user_email") {
        return Redirect::to("/village").into_response();
    }
    let (updated_jar, csrf_token) = generate_csrf_token(jar);
    let template = LoginTemplate {
        csrf_token,
        ..Default::default()
    };
    (updated_jar, render_template(template, None)).into_response()
}

/// POST /login – Handle login form submission.
pub async fn login(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if let Some(_cookie) = jar.get("user_email") {
        return Redirect::to("/village").into_response();
    }

    // Valida CSRF token
    if !validate_csrf_token(jar.clone(), &form.csrf_token) {
        let (updated_jar, csrf_token) = generate_csrf_token(jar);
        let template = LoginTemplate {
            email_value: form.email.clone(),
            error: Some("Invalid CSRF token. Please try again.".to_string()),
            csrf_token,
            ..Default::default()
        };
        return (updated_jar, render_template(template, Some(StatusCode::FORBIDDEN))).into_response();
    }

    let query = AuthenticateUser {
        email: form.email.clone(),
        password: form.password.clone(),
    };

    let (status, err_msg) = match state
        .app_bus
        .query(query, AuthenticateUserHandler::new())
        .await
    {
        Ok(user) => {
            let cookie = Cookie::new("user_email", user.email.clone());
            let jar = jar.add(cookie);
            return (jar, Redirect::to("/village")).into_response();
        }

        Err(ApplicationError::App(AppError::WrongAuthCredentials)) => (
            Some(StatusCode::UNAUTHORIZED),
            Some("Invalid email or password.".to_string()),
        ),

        Err(ApplicationError::Db(DbError::UserByEmailNotFound(_))) => (
            Some(StatusCode::UNAUTHORIZED),
            Some("User not found.".to_string()),
        ),
        Err(e) => {
            tracing::error!("Login error: {}", e);
            (
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                Some("Internal server error.".to_string()),
            )
        }
    };

    let (updated_jar, csrf_token) = generate_csrf_token(jar);
    let template = LoginTemplate {
        email_value: form.email.clone(),
        error: err_msg,
        csrf_token,
        ..Default::default()
    };
    (updated_jar, render_template(template, status)).into_response()
}
