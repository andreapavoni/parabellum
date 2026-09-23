//! Access/refresh token primitives for API authentication.
//!
//! Design choices:
//! - short-lived signed access token (JWT HS256)
//! - opaque refresh token, persisted hashed in DB
//! - refresh rotation on every refresh call
//! - refresh session id embedded in access token for revocation checks

use std::net::IpAddr;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use parabellum_app::identity::refresh_sessions::{
    RefreshSession, RefreshSessionError, SessionClient,
};
use parabellum_types::errors::ApplicationError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use parabellum_app::config::Config;

use crate::session::CurrentUser;

const ACCESS_TOKEN_CLOCK_SKEW_SECS: i64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum AuthTokenError {
    #[error("invalid token")]
    InvalidToken,
    #[error("token expired")]
    TokenExpired,
    #[error("refresh token expired")]
    RefreshExpired,
    #[error("refresh session revoked")]
    SessionRevoked,
    #[error("database error: {0}")]
    Database(#[source] ApplicationError),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct AuthenticatedTokenContext {
    pub user_id: Uuid,
    pub player_id: Uuid,
    pub current_village_id: u32,
    pub refresh_session_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct IssuedTokenPair {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub refresh_session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    player_id: String,
    current_village_id: u32,
    refresh_session_id: String,
    iat: i64,
    exp: i64,
}

pub struct AuthTokenService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    access_ttl_secs: i64,
}

impl AuthTokenService {
    pub fn new(config: &Config) -> Self {
        let key = config.token_signing_key.as_bytes().to_vec();
        Self {
            encoding: EncodingKey::from_secret(&key),
            decoding: DecodingKey::from_secret(&key),
            access_ttl_secs: config.access_token_ttl_secs,
        }
    }

    pub fn issue_access_token(
        &self,
        user: &CurrentUser,
        refresh_session_id: Uuid,
    ) -> Result<(String, i64), AuthTokenError> {
        self.issue_access_token_with_context(
            user.account.id,
            user.player.id,
            user.village.id,
            refresh_session_id,
        )
    }

    pub fn issue_access_token_with_context(
        &self,
        user_id: Uuid,
        player_id: Uuid,
        current_village_id: u32,
        refresh_session_id: Uuid,
    ) -> Result<(String, i64), AuthTokenError> {
        self.issue_access_token_for(
            user_id,
            player_id,
            current_village_id,
            refresh_session_id,
            Utc::now(),
        )
    }

    fn issue_access_token_for(
        &self,
        user_id: Uuid,
        player_id: Uuid,
        current_village_id: u32,
        refresh_session_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(String, i64), AuthTokenError> {
        let expires_at = now + Duration::seconds(self.access_ttl_secs);
        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            player_id: player_id.to_string(),
            current_village_id,
            refresh_session_id: refresh_session_id.to_string(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
        };

        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| AuthTokenError::Internal(e.to_string()))?;
        Ok((token, self.access_ttl_secs))
    }

    pub fn verify_access_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedTokenContext, AuthTokenError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = ACCESS_TOKEN_CLOCK_SKEW_SECS as u64;

        let decoded =
            decode::<AccessTokenClaims>(token, &self.decoding, &validation).map_err(|e| {
                if e.kind() == &jsonwebtoken::errors::ErrorKind::ExpiredSignature {
                    AuthTokenError::TokenExpired
                } else {
                    AuthTokenError::InvalidToken
                }
            })?;

        let claims = decoded.claims;
        Ok(AuthenticatedTokenContext {
            user_id: Uuid::parse_str(&claims.sub).map_err(|_| AuthTokenError::InvalidToken)?,
            player_id: Uuid::parse_str(&claims.player_id)
                .map_err(|_| AuthTokenError::InvalidToken)?,
            current_village_id: claims.current_village_id,
            refresh_session_id: Uuid::parse_str(&claims.refresh_session_id)
                .map_err(|_| AuthTokenError::InvalidToken)?,
        })
    }

    pub async fn create_refresh_session(
        &self,
        sessions: &parabellum_app::application::GameApplication,
        user: &CurrentUser,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
    ) -> Result<(RefreshSession, String), AuthTokenError> {
        let token = generate_refresh_token();
        let session = sessions
            .create_refresh_session(
                user.account.id,
                user.player.id,
                user.village.id,
                &hash_refresh_token(&token),
                SessionClient {
                    user_agent: user_agent.map(str::to_owned),
                    ip,
                },
            )
            .await?;
        Ok((session, token))
    }
    pub async fn rotate_refresh_session(
        &self,
        sessions: &parabellum_app::application::GameApplication,
        refresh_token: &str,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
    ) -> Result<(RefreshSession, String), AuthTokenError> {
        let token = generate_refresh_token();
        let session = sessions
            .rotate_refresh_session(
                &hash_refresh_token(refresh_token),
                hash_refresh_token(&token),
                SessionClient {
                    user_agent: user_agent.map(str::to_owned),
                    ip,
                },
            )
            .await?;
        Ok((session, token))
    }
    pub async fn revoke_refresh_session(
        &self,
        sessions: &parabellum_app::application::GameApplication,
        token: &str,
    ) -> Result<(), AuthTokenError> {
        sessions
            .revoke_refresh_session(&hash_refresh_token(token))
            .await
            .map_err(Into::into)
    }
    pub async fn revoke_all_user_sessions(
        &self,
        sessions: &parabellum_app::application::GameApplication,
        user_id: Uuid,
    ) -> Result<(), AuthTokenError> {
        sessions
            .revoke_all_refresh_sessions(user_id)
            .await
            .map_err(Into::into)
    }
    pub async fn update_refresh_session_village(
        &self,
        sessions: &parabellum_app::application::GameApplication,
        session_id: Uuid,
        village_id: u32,
    ) -> Result<(), AuthTokenError> {
        sessions
            .set_refresh_session_village(session_id, village_id)
            .await
            .map_err(Into::into)
    }
    pub async fn validate_refresh_session(
        &self,
        sessions: &parabellum_app::application::GameApplication,
        token: &str,
    ) -> Result<RefreshSession, AuthTokenError> {
        sessions
            .validate_refresh_session_hash(&hash_refresh_token(token))
            .await
            .map_err(Into::into)
    }
    pub async fn validate_refresh_session_id(
        &self,
        sessions: &parabellum_app::application::GameApplication,
        id: Uuid,
    ) -> Result<RefreshSession, AuthTokenError> {
        sessions
            .validate_refresh_session_id(id)
            .await
            .map_err(Into::into)
    }

    pub fn issue_token_pair(
        &self,
        user: &CurrentUser,
        refresh_session_id: Uuid,
        refresh_token: String,
    ) -> Result<IssuedTokenPair, AuthTokenError> {
        let (access_token, expires_in) = self.issue_access_token(user, refresh_session_id)?;
        Ok(IssuedTokenPair {
            access_token,
            expires_in,
            refresh_token,
            refresh_session_id,
        })
    }
}

impl From<RefreshSessionError> for AuthTokenError {
    fn from(error: RefreshSessionError) -> Self {
        match error {
            RefreshSessionError::Expired => Self::RefreshExpired,
            RefreshSessionError::Revoked => Self::SessionRevoked,
            RefreshSessionError::Storage(error) => Self::Database(error),
        }
    }
}

pub fn hash_refresh_token(refresh_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_refresh_token() -> String {
    let mut bytes = [0_u8; 48];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use parabellum_app::config::Config;

    fn test_config() -> Config {
        Config {
            port: 8000,
            world_size: 100,
            speed: 1,
            access_token_ttl_secs: 600,
            refresh_token_ttl_secs: 86_400,
            token_signing_key: "signing-secret".to_string(),
        }
    }

    #[test]
    fn issue_and_verify_access_token() {
        let service = AuthTokenService::new(&test_config());
        let user_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let village_id = 123;
        let session_id = Uuid::new_v4();
        let (token, _) = service
            .issue_access_token_for(user_id, player_id, village_id, session_id, Utc::now())
            .expect("token");
        let ctx = service.verify_access_token(&token).expect("valid token");
        assert_eq!(ctx.user_id, user_id);
        assert_eq!(ctx.player_id, player_id);
        assert_eq!(ctx.current_village_id, village_id);
        assert_eq!(ctx.refresh_session_id, session_id);
    }

    #[test]
    fn refresh_token_hash_is_stable() {
        let raw = "sample_refresh_token";
        let h1 = hash_refresh_token(raw);
        let h2 = hash_refresh_token(raw);
        assert_eq!(h1, h2);
    }
}
