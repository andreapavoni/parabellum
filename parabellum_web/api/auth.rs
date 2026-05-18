//! Authentication and token lifecycle handlers.
//!
//! Endpoints in this module:
//! - `POST /api/v1/auth/token/login`
//! - `POST /api/v1/auth/token/register`
//! - `POST /api/v1/auth/refresh`
//! - `POST /api/v1/auth/token/logout`
//!
//! Contract notes:
//! - Access tokens are short-lived JWTs.
//! - Refresh tokens are opaque, rotated on refresh, and persisted hashed.

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde::{Deserialize, Serialize};

use parabellum_app::ports::identity::RegisterPlayerRequest;
use parabellum_game::models::map::MapQuadrant;
use parabellum_types::{
    errors::{AppError, ApplicationError, DbError},
    tribe::Tribe,
};

use crate::{
    api::{
        dto::{SessionUserDto, session_user},
        error_mapping::internal_error,
        errors::ApiError,
    },
    auth_metrics::{inc_auth_failure, inc_auth_success, inc_refresh_failure, inc_refresh_success},
    auth_tokens::{AuthTokenError, IssuedTokenPair},
    http::AppState,
    session::current_user_by_ids,
};

use super::bearer_token;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Login payload for token-based authentication.
pub struct TokenLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
/// Registration payload.
pub struct TokenRegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub tribe: String,
    pub quadrant: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Refresh request with current refresh token.
pub struct TokenRefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Logout request. Optionally revokes all sessions for the user.
pub struct TokenLogoutRequest {
    pub refresh_token: String,
    #[serde(default)]
    pub all_sessions: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Logout response payload.
pub struct LogoutResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Token response returned by login/register/refresh.
pub struct TokenAuthResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub user: SessionUserDto,
    pub current_village_id: u32,
}

/// Authenticates user credentials and returns access+refresh token pair.
pub async fn token_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TokenLoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.email.trim().is_empty() {
        return Err(ApiError::unprocessable("Missing required login fields.")
            .with_field_error("email", "Email is required"));
    }
    if payload.password.trim().is_empty() {
        return Err(ApiError::unprocessable("Missing required login fields.")
            .with_field_error("password", "Password is required"));
    }

    let account = state
        .game_app
        .authenticate_user(&payload.email, &payload.password)
        .await
        .map_err(|err| {
            inc_auth_failure();
            map_auth_error(err)
        })?;

    let current = current_user_by_ids(&state, account.id, None)
        .await
        .map_err(|_| ApiError::unauthorized("Authentication required"))?;
    let (refresh_session, refresh_token) = state
        .token_service
        .create_refresh_session(
            &state.db_pool,
            &current,
            user_agent(&headers),
            client_ip(&headers),
        )
        .await
        .map_err(map_token_error)?;
    let pair = state
        .token_service
        .issue_token_pair(&current, refresh_session.id, refresh_token)
        .map_err(map_token_error)?;
    inc_auth_success();
    Ok(Json(token_response(pair, &current)))
}

/// Registers a new user/player and immediately returns tokens.
pub async fn token_register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TokenRegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.username.trim().is_empty() {
        return Err(
            ApiError::unprocessable("Missing required registration fields.")
                .with_field_error("username", "Username is required"),
        );
    }
    if payload.email.trim().is_empty() {
        return Err(
            ApiError::unprocessable("Missing required registration fields.")
                .with_field_error("email", "Email is required"),
        );
    }
    if payload.password.trim().is_empty() {
        return Err(
            ApiError::unprocessable("Missing required registration fields.")
                .with_field_error("password", "Password is required"),
        );
    }

    let tribe = parse_tribe(&payload.tribe)?;
    let quadrant = parse_quadrant(&payload.quadrant)?;
    state
        .game_app
        .register_player(RegisterPlayerRequest {
            player_id: uuid::Uuid::new_v4(),
            username: payload.username.clone(),
            email: payload.email.clone(),
            password: payload.password.clone(),
            tribe,
            quadrant,
        })
        .await
        .map_err(map_register_error)?;

    let account = state
        .game_app
        .get_user_by_email(&payload.email)
        .await
        .map_err(|err| internal_error("auth_register_load_account_failed", err))?;
    let current = current_user_by_ids(&state, account.id, None)
        .await
        .map_err(|_| ApiError::unauthorized("Authentication required"))?;
    let (refresh_session, refresh_token) = state
        .token_service
        .create_refresh_session(
            &state.db_pool,
            &current,
            user_agent(&headers),
            client_ip(&headers),
        )
        .await
        .map_err(map_token_error)?;
    let pair = state
        .token_service
        .issue_token_pair(&current, refresh_session.id, refresh_token)
        .map_err(map_token_error)?;
    Ok(Json(token_response(pair, &current)))
}

/// Rotates refresh token and returns a fresh token pair.
pub async fn token_refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TokenRefreshRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(ApiError::unprocessable("Refresh token is required.")
            .with_field_error("refresh_token", "Refresh token is required"));
    }

    let (session, rotated_refresh_token) = state
        .token_service
        .rotate_refresh_session(
            &state.db_pool,
            &payload.refresh_token,
            user_agent(&headers),
            client_ip(&headers),
        )
        .await
        .map_err(|err| {
            inc_refresh_failure();
            map_token_error(err)
        })?;
    let current = current_user_by_ids(&state, session.user_id, Some(session.current_village_id))
        .await
        .map_err(|_| ApiError::unauthorized("Authentication required"))?;
    let pair = state
        .token_service
        .issue_token_pair(&current, session.id, rotated_refresh_token)
        .map_err(map_token_error)?;
    inc_refresh_success();
    Ok(Json(token_response(pair, &current)))
}

/// Revokes the provided refresh token (and optionally all user sessions).
pub async fn token_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TokenLogoutRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(ApiError::unprocessable("Refresh token is required.")
            .with_field_error("refresh_token", "Refresh token is required"));
    }

    state
        .token_service
        .revoke_refresh_session(&state.db_pool, &payload.refresh_token)
        .await
        .map_err(map_token_error)?;

    if payload.all_sessions {
        let token = bearer_token(&headers)
            .ok_or_else(|| ApiError::unauthorized("Authentication required"))?;
        let claims = state
            .token_service
            .verify_access_token(token)
            .map_err(map_token_error)?;
        state
            .token_service
            .revoke_all_user_sessions(&state.db_pool, claims.user_id)
            .await
            .map_err(map_token_error)?;
    }

    Ok(Json(LogoutResponse { success: true }))
}

fn map_auth_error(error: ApplicationError) -> ApiError {
    match error {
        ApplicationError::App(AppError::WrongAuthCredentials)
        | ApplicationError::Db(DbError::UserByEmailNotFound(_)) => {
            ApiError::unauthorized("Invalid email or password.")
        }
        _ => internal_error("auth_login_failed", error),
    }
}

fn map_register_error(error: ApplicationError) -> ApiError {
    match error {
        ApplicationError::App(AppError::PasswordError) => {
            ApiError::unprocessable("Invalid password or internal password error.")
                .with_field_error("password", "The password does not satisfy the server rules")
        }
        ApplicationError::Db(DbError::Database(sqlx::Error::Database(db))) => match db.code() {
            Some(code) if code == "23505" => {
                ApiError::conflict("An account with this email already exists.")
                    .with_field_error("email", "Email already exists")
            }
            Some(code) if code == "23502" => {
                ApiError::unprocessable("Missing required registration fields.")
            }
            _ => internal_error("auth_register_failed", db),
        },
        ApplicationError::Db(db_err) => internal_error("auth_register_failed", db_err),
        _ => internal_error("auth_register_failed", error),
    }
}

fn parse_tribe(value: &str) -> Result<Tribe, ApiError> {
    match value {
        "Roman" => Ok(Tribe::Roman),
        "Gaul" => Ok(Tribe::Gaul),
        "Teuton" => Ok(Tribe::Teuton),
        other => Err(
            ApiError::unprocessable(format!("Unsupported tribe '{other}'"))
                .with_field_error("tribe", "Expected Roman, Gaul, or Teuton"),
        ),
    }
}

fn parse_quadrant(value: &str) -> Result<MapQuadrant, ApiError> {
    match value {
        "NorthEast" => Ok(MapQuadrant::NorthEast),
        "NorthWest" => Ok(MapQuadrant::NorthWest),
        "SouthEast" => Ok(MapQuadrant::SouthEast),
        "SouthWest" => Ok(MapQuadrant::SouthWest),
        other => Err(
            ApiError::unprocessable(format!("Unsupported quadrant '{other}'")).with_field_error(
                "quadrant",
                "Expected NorthEast, NorthWest, SouthEast, or SouthWest",
            ),
        ),
    }
}

fn map_token_error(error: AuthTokenError) -> ApiError {
    match error {
        AuthTokenError::TokenExpired => ApiError::token_expired("Access token expired"),
        AuthTokenError::RefreshExpired => ApiError::refresh_expired("Refresh token expired"),
        AuthTokenError::SessionRevoked => ApiError::session_revoked("Refresh session revoked"),
        AuthTokenError::InvalidToken => ApiError::unauthorized("Invalid token"),
        AuthTokenError::Database(err) | AuthTokenError::Internal(err) => {
            internal_error("auth_token_validation_failed", err)
        }
    }
}

fn token_response(
    pair: IssuedTokenPair,
    current: &crate::session::CurrentUser,
) -> TokenAuthResponse {
    TokenAuthResponse {
        access_token: pair.access_token,
        expires_in: pair.expires_in,
        refresh_token: pair.refresh_token,
        user: session_user(current),
        current_village_id: current.village.id,
    }
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
}

fn client_ip(headers: &HeaderMap) -> Option<std::net::IpAddr> {
    let raw = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)?;
    raw.parse::<std::net::IpAddr>().ok()
}
