use axum::http::HeaderMap;

use crate::{
    api::errors::ApiError,
    auth_metrics::inc_token_expired,
    auth_tokens::{AuthTokenError, RefreshSession},
    http::AppState,
    session::{CurrentUser, current_user_by_ids},
};

/// Resolve the authenticated user from the bearer token in `Authorization`.
///
/// It validates:
/// - access token signature + expiry
/// - refresh session state (not expired/revoked)
/// - token context coherence against refresh session data
pub async fn authenticated_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<CurrentUser, ApiError> {
    let token =
        bearer_token(headers).ok_or_else(|| ApiError::unauthorized("Authentication required"))?;
    let claims = state
        .token_service
        .verify_access_token(token)
        .map_err(map_token_error)?;
    let refresh_session = state
        .token_service
        .validate_refresh_session_id(&state.db_pool, claims.refresh_session_id)
        .await
        .map_err(map_token_error)?;
    validate_refresh_context(&claims, &refresh_session)?;
    current_user_by_ids(state, claims.user_id, Some(claims.current_village_id))
        .await
        .map_err(|_| ApiError::unauthorized("Authentication required"))
}

/// Parse `Authorization: Bearer <token>` from request headers.
pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let auth = headers.get(axum::http::header::AUTHORIZATION)?;
    let raw = auth.to_str().ok()?;
    let token = raw.strip_prefix("Bearer ")?;
    if token.is_empty() { None } else { Some(token) }
}

fn validate_refresh_context(
    claims: &crate::auth_tokens::AuthenticatedTokenContext,
    refresh_session: &RefreshSession,
) -> Result<(), ApiError> {
    if refresh_session.user_id != claims.user_id
        || refresh_session.player_id != claims.player_id
        || refresh_session.current_village_id != claims.current_village_id
    {
        return Err(ApiError::unauthorized("Invalid token context"));
    }
    Ok(())
}

fn map_token_error(error: AuthTokenError) -> ApiError {
    match error {
        AuthTokenError::TokenExpired => {
            inc_token_expired();
            ApiError::token_expired("Access token expired")
        }
        AuthTokenError::RefreshExpired => ApiError::refresh_expired("Refresh session expired"),
        AuthTokenError::SessionRevoked => ApiError::session_revoked("Session revoked"),
        AuthTokenError::InvalidToken => ApiError::unauthorized("Invalid bearer token"),
        AuthTokenError::Database(msg) | AuthTokenError::Internal(msg) => ApiError::internal(msg),
    }
}
