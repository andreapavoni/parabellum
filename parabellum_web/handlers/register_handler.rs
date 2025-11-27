use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};

use parabellum_app::{
    command_handlers::RegisterPlayerCommandHandler,
    cqrs::{commands::RegisterPlayer, queries::GetUserByEmail},
    queries_handlers::GetUserByEmailHandler,
};
use parabellum_game::models::map::MapQuadrant;
use parabellum_types::{
    errors::{AppError, ApplicationError},
    tribe::Tribe,
};

use crate::{
    handlers::{CsrfForm, HasCsrfToken, ensure_not_authenticated, generate_csrf, render_template},
    http::AppState,
    templates::RegisterTemplate,
};

// Form for registration.
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

/// GET /register – Show the signup form.
pub async fn register_page(
    State(_state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }

    let (jar, csrf_token) = generate_csrf(jar);

    let template = RegisterTemplate {
        csrf_token,
        current_user: false,
        selected_tribe: "Roman".to_string(), // default selection
        selected_quadrant: "NorthEast".to_string(),
        ..Default::default()
    };

    (jar, render_template(template, None)).into_response()
}

/// POST /register – Handle signup form submission.
pub async fn register(
    State(state): State<AppState>,
    CsrfForm { jar, inner: form }: CsrfForm<RegisterForm>,
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
    match state
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
                Ok(user) => {
                    let cookie = Cookie::new("user_id", user.id.to_string());
                    let updated_jar = jar.add(cookie);
                    return (updated_jar, Redirect::to("/village")).into_response();
                }
                Err(e) => {
                    tracing::error!("Registration follow-up error: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
                        .into_response();
                }
            }
        }
        Err(ApplicationError::App(AppError::PasswordError)) => {
            // Password hashing error or invalid password (unlikely scenario)
            let (jar, new_csrf_token) = generate_csrf(jar);
            let template = RegisterTemplate {
                csrf_token: new_csrf_token,
                current_user: false,
                username_value: form.username.clone(),
                email_value: form.email.clone(),
                selected_tribe: form.tribe.clone(),
                selected_quadrant: form.quadrant.clone(),
                error: Some("Invalid password or internal error.".to_string()),
            };

            return (
                jar,
                render_template(template, Some(StatusCode::INTERNAL_SERVER_ERROR)),
            )
                .into_response();
        }
        Err(ApplicationError::Db(db_err)) => {
            // Likely a database error (e.g., duplicate email constraint)
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
            let (jar, new_csrf_token) = generate_csrf(jar);
            let template = RegisterTemplate {
                current_user: false,
                csrf_token: new_csrf_token,
                username_value: form.username.clone(),
                email_value: form.email.clone(),
                selected_tribe: form.tribe.clone(),
                selected_quadrant: form.quadrant.clone(),
                error: Some(user_message.to_string()),
            };

            return (
                jar,
                render_template(template, Some(StatusCode::UNPROCESSABLE_ENTITY)),
            )
                .into_response();
        }
        Err(e) => {
            // Other errors (should be rare)
            tracing::error!("Registration error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response();
        }
    }
}
