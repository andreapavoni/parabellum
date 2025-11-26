use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};

use parabellum_app::{
    command_handlers::RegisterPlayerCommandHandler, cqrs::commands::RegisterPlayer,
};
use parabellum_game::models::map::MapQuadrant;
use parabellum_types::{
    errors::{AppError, ApplicationError},
    tribe::Tribe,
};

use crate::{handlers::{render_template, generate_csrf_token, validate_csrf_token}, http::AppState, templates::RegisterTemplate};

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

/// GET /register – Show the signup form.
pub async fn register_page(
    State(_state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    if let Some(_) = jar.get("user_email") {
        return Redirect::to("/").into_response();
    }
    let (updated_jar, csrf_token) = generate_csrf_token(jar);
    let template = RegisterTemplate {
        current_user: false,
        username_value: "".to_string(),
        email_value: "".to_string(),
        selected_tribe: "Roman".to_string(), // default selection
        selected_quadrant: "NorthEast".to_string(),
        error: None,
        csrf_token,
    };
    (updated_jar, render_template(template, None)).into_response()
}

/// POST /register – Handle signup form submission.
pub async fn register(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<RegisterForm>,
) -> impl IntoResponse {
    if let Some(_) = jar.get("user_email") {
        return Redirect::to("/").into_response();
    }

    // Valida CSRF token
    if !validate_csrf_token(jar.clone(), &form.csrf_token) {
        let (updated_jar, csrf_token) = generate_csrf_token(jar);
        let template = RegisterTemplate {
            current_user: false,
            username_value: form.username.clone(),
            email_value: form.email.clone(),
            selected_tribe: form.tribe.clone(),
            selected_quadrant: form.quadrant.clone(),
            error: Some("Invalid CSRF token. Please try again.".to_string()),
            csrf_token,
        };
        return (updated_jar, render_template(template, Some(StatusCode::FORBIDDEN))).into_response();
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
            let cookie = Cookie::new("user_email", form.email.clone());
            let updated_jar = jar.add(cookie);
            return (updated_jar, Redirect::to("/village")).into_response();
        }
        Err(ApplicationError::App(AppError::PasswordError)) => {
            // Password hashing error or invalid password (unlikely scenario)
            let (updated_jar, csrf_token) = generate_csrf_token(jar);
            let template = RegisterTemplate {
                current_user: false,
                username_value: form.username.clone(),
                email_value: form.email.clone(),
                selected_tribe: form.tribe.clone(),
                selected_quadrant: form.quadrant.clone(),
                error: Some("Invalid password or internal error.".to_string()),
                csrf_token,
            };
            return (updated_jar, render_template(template, Some(StatusCode::INTERNAL_SERVER_ERROR)))
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
            let (updated_jar, csrf_token) = generate_csrf_token(jar);
            let template = RegisterTemplate {
                current_user: false,
                username_value: form.username.clone(),
                email_value: form.email.clone(),
                selected_tribe: form.tribe.clone(),
                selected_quadrant: form.quadrant.clone(),
                error: Some(user_message.to_string()),
                csrf_token,
            };
            return (updated_jar, render_template(template, Some(StatusCode::UNPROCESSABLE_ENTITY)))
                .into_response();
        }
        Err(e) => {
            // Other errors (should be rare)
            tracing::error!("Registration error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response();
        }
    }
}
