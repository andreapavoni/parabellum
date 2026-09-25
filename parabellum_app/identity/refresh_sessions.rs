//! Refresh-session lifecycle. Token generation/signing belongs to the caller;
//! repositories receive only hashes and rotate in a single transaction.
use crate::villages::ports::{Clock, IdGenerator};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use parabellum_types::errors::ApplicationError;
use std::{net::IpAddr, sync::Arc};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RefreshSessionError {
    #[error("refresh token expired")]
    Expired,
    #[error("refresh session revoked")]
    Revoked,
    #[error(transparent)]
    Storage(#[from] ApplicationError),
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

impl RefreshSession {
    /// Rejects revoked sessions before checking expiration, consistently across reads and rotation.
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), RefreshSessionError> {
        if self.revoked_at.is_some() {
            return Err(RefreshSessionError::Revoked);
        }
        if self.expires_at <= now {
            return Err(RefreshSessionError::Expired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionClient {
    pub user_agent: Option<String>,
    pub ip: Option<IpAddr>,
}

/// Replacement values independent of the old session's identity and village.
pub struct SessionReplacement {
    pub id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub client: SessionClient,
}

#[async_trait]
pub trait RefreshSessionRepository: Send + Sync {
    async fn create(
        &self,
        session: &RefreshSession,
        token_hash: &str,
        client: SessionClient,
    ) -> Result<(), RefreshSessionError>;
    /// Lock, validate, revoke and insert atomically. A failed insertion must not revoke the old session.
    async fn rotate(
        &self,
        old_hash: &str,
        replacement: SessionReplacement,
    ) -> Result<RefreshSession, RefreshSessionError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RefreshSession>, RefreshSessionError>;
    async fn find_by_hash(&self, hash: &str)
    -> Result<Option<RefreshSession>, RefreshSessionError>;
    async fn revoke(&self, hash: &str) -> Result<(), RefreshSessionError>;
    async fn revoke_all(&self, user_id: Uuid) -> Result<(), RefreshSessionError>;
    async fn set_village(&self, id: Uuid, village_id: u32) -> Result<(), RefreshSessionError>;
}

/// Application entry point shared by HTTP authentication flows.
#[derive(Clone)]
pub struct RefreshSessionUseCases {
    repository: Arc<dyn RefreshSessionRepository>,
    ttl: Duration,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl RefreshSessionUseCases {
    pub fn new(
        repository: Arc<dyn RefreshSessionRepository>,
        ttl_secs: i64,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            repository,
            clock,
            ids,
            ttl: Duration::seconds(ttl_secs),
        }
    }
    pub async fn create(
        &self,
        user_id: Uuid,
        player_id: Uuid,
        village_id: u32,
        hash: &str,
        client: SessionClient,
    ) -> Result<RefreshSession, RefreshSessionError> {
        let session = RefreshSession {
            id: self.ids.next(),
            user_id,
            player_id,
            current_village_id: village_id,
            expires_at: self.clock.now() + self.ttl,
            revoked_at: None,
        };
        self.repository.create(&session, hash, client).await?;
        Ok(session)
    }
    pub async fn rotate(
        &self,
        old_hash: &str,
        new_hash: String,
        client: SessionClient,
    ) -> Result<RefreshSession, RefreshSessionError> {
        self.repository
            .rotate(
                old_hash,
                SessionReplacement {
                    id: self.ids.next(),
                    token_hash: new_hash,
                    expires_at: self.clock.now() + self.ttl,
                    client,
                },
            )
            .await
    }
    pub async fn validate_id(&self, id: Uuid) -> Result<RefreshSession, RefreshSessionError> {
        let session = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or(RefreshSessionError::Revoked)?;
        session.validate(self.clock.now())?;
        Ok(session)
    }
    pub async fn validate_hash(&self, hash: &str) -> Result<RefreshSession, RefreshSessionError> {
        let session = self
            .repository
            .find_by_hash(hash)
            .await?
            .ok_or(RefreshSessionError::Expired)?;
        session.validate(self.clock.now())?;
        Ok(session)
    }
    pub async fn revoke(&self, hash: &str) -> Result<(), RefreshSessionError> {
        self.repository.revoke(hash).await
    }
    pub async fn revoke_all(&self, user_id: Uuid) -> Result<(), RefreshSessionError> {
        self.repository.revoke_all(user_id).await
    }
    pub async fn set_village(&self, id: Uuid, village_id: u32) -> Result<(), RefreshSessionError> {
        self.repository.set_village(id, village_id).await
    }
}
