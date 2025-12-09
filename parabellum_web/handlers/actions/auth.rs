use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::SignedCookieJar;

use parabellum_app::{
    command_handlers::RegisterPlayerCommandHandler,
    cqrs::{
        commands::RegisterPlayer,
        queries::{AuthenticateUser, GetUserByEmail},
    },
    queries_handlers::{AuthenticateUserHandler, GetUserByEmailHandler},
};
use parabellum_game::models::map::MapQuadrant;
use parabellum_types::{
    errors::{AppError, ApplicationError, DbError},
    tribe::Tribe,
};

use axum::response::Html;
use chrono::Utc;
use dioxus::prelude::*;

use crate::{
    components::{LayoutData, LoginPage, PageLayout, RegisterPage, wrap_in_html},
    handlers::helpers::{
        CsrfForm, HasCsrfToken, ensure_not_authenticated, generate_csrf, initialize_session,
    },
    http::AppState,
};

/// Form for login.
#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub csrf_token: String,
}

impl HasCsrfToken for LoginForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// Form for registration.
#[derive(Clone, serde::Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub tribe: String,
    pub quadrant: String,
    pub csrf_token: String,
}

impl HasCsrfToken for RegisterForm {
    fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}

/// Render login page with error
pub fn render_login_with_error(
    jar: SignedCookieJar,
    email: String,
    error: Option<String>,
) -> impl IntoResponse {
    let (jar, csrf_token) = generate_csrf(jar);

    let layout_data = LayoutData {
        player: None,
        village: None,
        server_time: Utc::now().timestamp(),
        nav_active: "".to_string(),
    };

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            LoginPage {
                csrf_token: csrf_token,
                email_value: email,
                error: error,
            }
        }
    });

    (jar, Html(wrap_in_html(&body_content))).into_response()
}

/// Render register page with error
pub fn render_register_with_error(
    jar: SignedCookieJar,
    form: RegisterForm,
    error: Option<String>,
) -> impl IntoResponse {
    let (jar, csrf_token) = generate_csrf(jar);

    let layout_data = LayoutData {
        player: None,
        village: None,
        server_time: Utc::now().timestamp(),
        nav_active: "".to_string(),
    };

    let body_content = dioxus_ssr::render_element(rsx! {
        PageLayout {
            data: layout_data,
            RegisterPage {
                csrf_token: csrf_token,
                username_value: form.username,
                email_value: form.email,
                selected_tribe: form.tribe,
                selected_quadrant: form.quadrant,
                error: error,
            }
        }
    });

    (jar, Html(wrap_in_html(&body_content))).into_response()
}

/// POST /login – Handle login form submission.
pub async fn login(
    State(state): State<AppState>,
    CsrfForm { jar, form }: CsrfForm<LoginForm>,
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
        Err(ApplicationError::App(AppError::WrongAuthCredentials))
        | Err(ApplicationError::Db(DbError::UserByEmailNotFound(_))) => (
            StatusCode::UNAUTHORIZED,
            Some("Invalid email or password.".to_string()),
        ),
        Err(e) => {
            tracing::error!("Login error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Internal server error.".to_string()),
            )
        }
    };

    (
        status,
        render_login_with_error(jar, form.email.clone(), err_msg),
    )
        .into_response()
}

/// POST /register – Handle signup form submission.
pub async fn register(
    State(state): State<AppState>,
    CsrfForm { jar, form }: CsrfForm<RegisterForm>,
) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let tribe_enum = match form.tribe.as_str() {
        "Roman" => Tribe::Roman,
        "Gaul" => Tribe::Gaul,
        "Teuton" => Tribe::Teuton,
        _ => Tribe::Roman,
    };

    let quadrant_enum = match form.quadrant.as_str() {
        "NorthEast" => MapQuadrant::NorthEast,
        "NorthWest" => MapQuadrant::NorthWest,
        "SouthEast" => MapQuadrant::SouthEast,
        "SouthWest" => MapQuadrant::SouthWest,
        _ => MapQuadrant::NorthEast,
    };

    let command = RegisterPlayer::new(
        form.username.clone(),
        form.email.clone(),
        form.password.clone(),
        tribe_enum,
        quadrant_enum,
    );
    let err = match state
        .app_bus
        .execute(command, RegisterPlayerCommandHandler::new())
        .await
    {
        Ok(()) => {
            let query = GetUserByEmail {
                email: form.email.clone(),
            };
            match state
                .app_bus
                .query(query, GetUserByEmailHandler::new())
                .await
            {
                Ok(user) => match initialize_session(&state, jar, user.id).await {
                    Ok(jar) => return (jar, Redirect::to("/village")).into_response(),
                    Err(e) => {
                        tracing::error!("Registration session initialization failed: {}", e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
                            .into_response();
                    }
                },
                Err(e) => {
                    tracing::error!("Registration follow-up error: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
                        .into_response();
                }
            };
        }
        Err(ApplicationError::App(AppError::PasswordError)) => {
            Some("Invalid password or internal error.".to_string())
        }
        Err(ApplicationError::Db(db_err)) => {
            let err_msg = format!("{}", db_err);
            let user_message = if err_msg.contains("duplicate key value")
                || err_msg.contains("UNIQUE constraint")
            {
                "An account with this email already exists."
            } else if err_msg.contains("null value in column") {
                "Missing required fields."
            } else {
                "Registration failed due to an internal error."
            };
            tracing::error!("Registration DB error: {}", db_err);
            Some(user_message.to_string())
        }
        Err(e) => {
            tracing::error!("Registration error: {}", e);
            Some("Internal server error.".to_string())
        }
    };

    render_register_with_error(jar, form.clone(), err).into_response()
}

/// GET /logout – Log the user out by clearing the auth cookie.
pub async fn logout(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get("user_id") {
        let updated_jar = jar.remove(cookie);
        return (updated_jar, Redirect::to("/")).into_response();
    }

    Redirect::to("/").into_response()
}
