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
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::OnceCell;
use uuid::Uuid;

use parabellum_app::config::Config;

use crate::session::CurrentUser;

const ACCESS_TOKEN_CLOCK_SKEW_SECS: i64 = 30;
static REFRESH_SESSION_SCHEMA_READY: OnceCell<()> = OnceCell::const_new();

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
    Database(String),
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
pub struct RefreshSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub player_id: Uuid,
    pub current_village_id: u32,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
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
    refresh_ttl_secs: i64,
}

impl AuthTokenService {
    pub fn new(config: &Config) -> Self {
        let key = config.token_signing_key.as_bytes().to_vec();
        Self {
            encoding: EncodingKey::from_secret(&key),
            decoding: DecodingKey::from_secret(&key),
            access_ttl_secs: config.access_token_ttl_secs,
            refresh_ttl_secs: config.refresh_token_ttl_secs,
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
        pool: &PgPool,
        user: &CurrentUser,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
    ) -> Result<(RefreshSession, String), AuthTokenError> {
        ensure_refresh_session_schema_once(pool).await?;
        let token = generate_refresh_token();
        let token_hash = hash_refresh_token(&token);
        let expires_at = Utc::now() + Duration::seconds(self.refresh_ttl_secs);
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO auth_refresh_sessions
                (id, user_id, player_id, current_village_id, token_hash, expires_at, user_agent, ip)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8::inet)
            "#,
        )
        .bind(id)
        .bind(user.account.id)
        .bind(user.player.id)
        .bind(i32::try_from(user.village.id).map_err(|e| AuthTokenError::Internal(e.to_string()))?)
        .bind(token_hash)
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip.map(|x| x.to_string()))
        .execute(pool)
        .await
        .map_err(|e| AuthTokenError::Database(e.to_string()))?;

        Ok((
            RefreshSession {
                id,
                user_id: user.account.id,
                player_id: user.player.id,
                current_village_id: user.village.id,
                expires_at,
                revoked_at: None,
            },
            token,
        ))
    }

    pub async fn rotate_refresh_session(
        &self,
        pool: &PgPool,
        refresh_token: &str,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
    ) -> Result<(RefreshSession, String), AuthTokenError> {
        ensure_refresh_session_schema_once(pool).await?;
        let old_hash = hash_refresh_token(refresh_token);
        let row =
            sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, DateTime<Utc>, Option<DateTime<Utc>>)>(
                r#"
            SELECT id, user_id, player_id, current_village_id, expires_at, revoked_at
            FROM auth_refresh_sessions
            WHERE token_hash = $1
            "#,
            )
            .bind(old_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| AuthTokenError::Database(e.to_string()))?
            .ok_or(AuthTokenError::RefreshExpired)?;

        let (old_id, user_id, player_id, current_village_id_i32, expires_at, revoked_at) = row;
        if revoked_at.is_some() {
            return Err(AuthTokenError::SessionRevoked);
        }
        if expires_at <= Utc::now() {
            return Err(AuthTokenError::RefreshExpired);
        }

        let new_token = generate_refresh_token();
        let new_hash = hash_refresh_token(&new_token);
        let new_id = Uuid::new_v4();
        let new_expires_at = Utc::now() + Duration::seconds(self.refresh_ttl_secs);

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| AuthTokenError::Database(e.to_string()))?;

        sqlx::query("UPDATE auth_refresh_sessions SET revoked_at = NOW() WHERE id = $1")
            .bind(old_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AuthTokenError::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO auth_refresh_sessions
                (id, user_id, player_id, current_village_id, token_hash, expires_at, user_agent, ip)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8::inet)
            "#,
        )
        .bind(new_id)
        .bind(user_id)
        .bind(player_id)
        .bind(current_village_id_i32)
        .bind(new_hash)
        .bind(new_expires_at)
        .bind(user_agent)
        .bind(ip.map(|x| x.to_string()))
        .execute(&mut *tx)
        .await
        .map_err(|e| AuthTokenError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| AuthTokenError::Database(e.to_string()))?;

        let current_village_id = u32::try_from(current_village_id_i32)
            .map_err(|e| AuthTokenError::Internal(e.to_string()))?;
        Ok((
            RefreshSession {
                id: new_id,
                user_id,
                player_id,
                current_village_id,
                expires_at: new_expires_at,
                revoked_at: None,
            },
            new_token,
        ))
    }

    pub async fn revoke_refresh_session(
        &self,
        pool: &PgPool,
        refresh_token: &str,
    ) -> Result<(), AuthTokenError> {
        let token_hash = hash_refresh_token(refresh_token);
        sqlx::query(
            "UPDATE auth_refresh_sessions SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|e| AuthTokenError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn revoke_all_user_sessions(
        &self,
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<(), AuthTokenError> {
        sqlx::query(
            "UPDATE auth_refresh_sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AuthTokenError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn update_refresh_session_village(
        &self,
        pool: &PgPool,
        session_id: Uuid,
        current_village_id: u32,
    ) -> Result<(), AuthTokenError> {
        let village_id_i32 = i32::try_from(current_village_id)
            .map_err(|e| AuthTokenError::Internal(e.to_string()))?;
        sqlx::query(
            "UPDATE auth_refresh_sessions SET current_village_id = $1, last_used_at = NOW() WHERE id = $2 AND revoked_at IS NULL",
        )
        .bind(village_id_i32)
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AuthTokenError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn validate_refresh_session(
        &self,
        pool: &PgPool,
        refresh_token: &str,
    ) -> Result<RefreshSession, AuthTokenError> {
        let token_hash = hash_refresh_token(refresh_token);
        let row =
            sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, DateTime<Utc>, Option<DateTime<Utc>>)>(
                r#"
            SELECT id, user_id, player_id, current_village_id, expires_at, revoked_at
            FROM auth_refresh_sessions
            WHERE token_hash = $1
            "#,
            )
            .bind(token_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| AuthTokenError::Database(e.to_string()))?
            .ok_or(AuthTokenError::RefreshExpired)?;

        if row.5.is_some() {
            return Err(AuthTokenError::SessionRevoked);
        }
        if row.4 <= Utc::now() {
            return Err(AuthTokenError::RefreshExpired);
        }

        let current_village_id =
            u32::try_from(row.3).map_err(|e| AuthTokenError::Internal(e.to_string()))?;
        Ok(RefreshSession {
            id: row.0,
            user_id: row.1,
            player_id: row.2,
            current_village_id,
            expires_at: row.4,
            revoked_at: row.5,
        })
    }

    pub async fn validate_refresh_session_id(
        &self,
        pool: &PgPool,
        session_id: Uuid,
    ) -> Result<RefreshSession, AuthTokenError> {
        let row =
            sqlx::query_as::<_, (Uuid, Uuid, Uuid, i32, DateTime<Utc>, Option<DateTime<Utc>>)>(
                r#"
            SELECT id, user_id, player_id, current_village_id, expires_at, revoked_at
            FROM auth_refresh_sessions
            WHERE id = $1
            "#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AuthTokenError::Database(e.to_string()))?
            .ok_or(AuthTokenError::SessionRevoked)?;

        if row.5.is_some() {
            return Err(AuthTokenError::SessionRevoked);
        }
        if row.4 <= Utc::now() {
            return Err(AuthTokenError::RefreshExpired);
        }

        let current_village_id =
            u32::try_from(row.3).map_err(|e| AuthTokenError::Internal(e.to_string()))?;
        Ok(RefreshSession {
            id: row.0,
            user_id: row.1,
            player_id: row.2,
            current_village_id,
            expires_at: row.4,
            revoked_at: row.5,
        })
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

    pub async fn ensure_refresh_schema(&self, pool: &PgPool) -> Result<(), AuthTokenError> {
        ensure_refresh_session_schema_once(pool).await
    }
}

async fn ensure_refresh_session_schema(pool: &PgPool) -> Result<(), AuthTokenError> {
    // Defensive bootstrap for integration tests that can run against DBs with
    // partial migration history. In normal environments the SQL migration owns
    // this schema.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS auth_refresh_sessions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
            current_village_id INTEGER NOT NULL REFERENCES villages(id) ON DELETE CASCADE,
            token_hash TEXT NOT NULL UNIQUE,
            expires_at TIMESTAMPTZ NOT NULL,
            revoked_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            user_agent TEXT,
            ip INET
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| AuthTokenError::Database(e.to_string()))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_user_id ON auth_refresh_sessions (user_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthTokenError::Database(e.to_string()))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_player_id ON auth_refresh_sessions (player_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthTokenError::Database(e.to_string()))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_expires_at ON auth_refresh_sessions (expires_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthTokenError::Database(e.to_string()))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_revoked_at ON auth_refresh_sessions (revoked_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthTokenError::Database(e.to_string()))?;

    Ok(())
}

async fn ensure_refresh_session_schema_once(pool: &PgPool) -> Result<(), AuthTokenError> {
    REFRESH_SESSION_SCHEMA_READY
        .get_or_try_init(|| async {
            ensure_refresh_session_schema(pool).await?;
            Ok(())
        })
        .await?;
    Ok(())
}

pub fn hash_refresh_token(refresh_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_refresh_token() -> String {
    let mut bytes = [0_u8; 48];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use parabellum_app::config::Config;

    fn test_config() -> Config {
        Config {
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
