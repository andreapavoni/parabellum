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

use crate::{
    handlers::{
        CsrfForm, HasCsrfToken, ensure_not_authenticated, generate_csrf, initialize_session,
        render_template,
    },
    http::AppState,
    templates::{LoginTemplate, RegisterTemplate, TemplateLayout},
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

/// GET /login – Show the login form.
pub async fn login_page(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Err(redirect) = ensure_not_authenticated(&jar) {
        return redirect.into_response();
    }
    let (jar, csrf_token) = generate_csrf(jar);

    let template = LoginTemplate {
        csrf_token,
        layout: TemplateLayout::new(None, "login"),
        email_value: String::new(),
        error: None,
    };
    (jar, render_template(template, None)).into_response()
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
            Some(StatusCode::UNAUTHORIZED),
            Some("Invalid email or password.".to_string()),
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
        layout: TemplateLayout::new(None, "login"),
        email_value: form.email.clone(),
        error: err_msg,
    };

    (jar, render_template(template, status)).into_response()
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
        layout: TemplateLayout::new(None, "register"),
        username_value: String::new(),
        email_value: String::new(),
        selected_tribe: "Roman".to_string(),
        selected_quadrant: "NorthEast".to_string(),
        error: None,
    };

    (jar, render_template(template, None)).into_response()
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
            }
        }
        Err(ApplicationError::App(AppError::PasswordError)) => {
            let (jar, new_csrf_token) = generate_csrf(jar);
            let template = RegisterTemplate {
                csrf_token: new_csrf_token,
                layout: TemplateLayout::new(None, "register"),
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
                layout: TemplateLayout::new(None, "register"),
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
            tracing::error!("Registration error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.").into_response();
        }
    }
}

/// GET /logout – Log the user out by clearing the auth cookie.
pub async fn logout(State(_state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get("user_id") {
        let updated_jar = jar.remove(cookie);
        return (updated_jar, Redirect::to("/")).into_response();
    }

    Redirect::to("/").into_response()
}
