use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{SignedCookieJar, cookie::Cookie};

use parabellum_app::command_handlers::RegisterPlayerCommandHandler;
use parabellum_app::cqrs::commands::RegisterPlayer;
use parabellum_core::{AppError, ApplicationError};
use parabellum_game::models::map::MapQuadrant;
use parabellum_types::tribe::Tribe;

use crate::{handlers::render_template, http::AppState, templates::RegisterTemplate};

// Form for registration.
#[derive(Clone, serde::Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub tribe: String,
    pub quadrant: String,
}

/// GET /register – Show the signup form.
pub async fn register_page(
    State(_state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    // If already logged in, redirect home (no need to register a new account)
    if let Some(_) = jar.get("user_email") {
        return Redirect::to("/").into_response();
    }
    let template = RegisterTemplate {
        current_user: false,
        current_user_email: None,
        username_value: "".to_string(),
        email_value: "".to_string(),
        selected_tribe: "Roman".to_string(), // default selection
        selected_quadrant: "NorthEast".to_string(),
        error: None,
    };
    render_template(template).into_response()
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
            return (updated_jar, Redirect::to("/")).into_response();
        }
        Err(ApplicationError::App(AppError::PasswordError)) => {
            // Password hashing error or invalid password (unlikely scenario)
            let template = RegisterTemplate {
                current_user: false,
                current_user_email: None,
                username_value: form.username.clone(),
                email_value: form.email.clone(),
                selected_tribe: form.tribe.clone(),
                selected_quadrant: form.quadrant.clone(),
                error: Some("Invalid password or internal error.".to_string()),
            };
            return render_template(template).into_response();
        }
        Err(ApplicationError::Db(db_err)) => {
            // Likely a database error (e.g., duplicate email constraint)
            let err_msg = format!("{}", db_err);
            let user_message = if err_msg.contains("duplicate key value")
                || err_msg.contains("UNIQUE constraint")
            {
                "An account with this email already exists."
            } else if err_msg.contains("null value in column \"user_id\"") {
                // If the player->user link was not set (internal error due to missing user_id)
                "Internal error creating account (user link failed)."
            } else {
                "Registration failed due to an internal error."
            };
            tracing::error!("Registration DB error: {}", db_err);
            let template = RegisterTemplate {
                current_user: false,
                current_user_email: None,
                username_value: form.username.clone(),
                email_value: form.email.clone(),
                selected_tribe: form.tribe.clone(),
                selected_quadrant: form.quadrant.clone(),
                error: Some(user_message.to_string()),
            };
            return render_template(template).into_response();
        }
        Err(e) => {
            // Other errors (should be rare)
            tracing::error!("Registration error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response();
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{Form, extract::State, http::StatusCode, response::IntoResponse};
    use axum_extra::extract::SignedCookieJar;
    use std::sync::Arc;

    use parabellum_app::{app::AppBus, config::Config, test_utils::tests::MockUnitOfWorkProvider};

    use crate::{
        AppState,
        handlers::{register, register_handler::RegisterForm},
    };

    #[tokio::test]
    async fn test_register_success() {
        let config = Arc::new(Config::from_env());

        let uow_provider = Arc::new(MockUnitOfWorkProvider::new());
        let app_bus = Arc::new(AppBus::new(config.clone(), uow_provider));
        let state = AppState::new(app_bus.clone(), &config);

        let form = RegisterForm {
            username: "TestUser".into(),
            email: "test@example.com".into(),
            password: "P@ssw0rd!".into(),
            tribe: "Roman".into(),
            quadrant: "NorthEast".into(),
        };

        let jar = SignedCookieJar::new(state.cookie_key.clone());
        let response = register(State(state.clone()), jar, Form(form.clone())).await;

        // Response should be a redirect (303) with cookie
        let (jar_response, _redirect) = response.into_response().into_parts();
        assert_eq!(jar_response.status, StatusCode::SEE_OTHER);

        assert!(jar_response.headers.get("set-cookie").is_some());
        let cookie = jar_response
            .headers
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.starts_with("user_email"));
    }
}
