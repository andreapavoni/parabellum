use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::SignedCookieJar;

/// Form for login.
#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub csrf_token: String,
}

use crate::{
    handlers::{
        CsrfForm, HasCsrfToken, ensure_not_authenticated, generate_csrf, initialize_session,
        render_template,
    },
    http::AppState,
    templates::LoginTemplate,
};
use parabellum_app::{cqrs::queries::AuthenticateUser, queries_handlers::AuthenticateUserHandler};
use parabellum_types::errors::{AppError, ApplicationError, DbError};

impl HasCsrfToken for LoginForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// GET /login – Show the login form.
pub async fn login_page(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }
    let (jar, csrf_token) = generate_csrf(jar);

    let template = LoginTemplate {
        csrf_token,
        nav_active: "login",
        ..Default::default()
    };
    (jar, render_template(template, None)).into_response()
}

/// POST /login – Handle login form submission.
pub async fn login(
    State(state): State<AppState>,
    CsrfForm { jar, inner: form }: CsrfForm<LoginForm>,
) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
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
        Ok(user) => match initialize_session(&state, jar, user.id).await {
            Ok(jar) => {
                return (jar, Redirect::to("/village")).into_response();
            }
            Err(e) => {
                tracing::error!("Login session initialization failed: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
                    .into_response();
            }
        },

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

    let (jar, new_csrf_token) = generate_csrf(jar);
    let template = LoginTemplate {
        csrf_token: new_csrf_token,
        nav_active: "login",
        email_value: form.email.clone(),
        error: err_msg,
        ..Default::default()
    };

    (jar, render_template(template, status)).into_response()
}
