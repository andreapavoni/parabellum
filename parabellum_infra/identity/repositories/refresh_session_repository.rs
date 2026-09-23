//! PostgreSQL refresh sessions. Migrations exclusively own the schema.
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_app::identity::refresh_sessions::{
    RefreshSession, RefreshSessionError, RefreshSessionRepository, SessionClient,
    SessionReplacement,
};
use parabellum_types::errors::{ApplicationError, DbError};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct DbRefreshSessionRow {
    id: Uuid,
    user_id: Uuid,
    player_id: Uuid,
    current_village_id: i32,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

pub struct PostgresRefreshSessionRepository {
    pool: PgPool,
}
impl PostgresRefreshSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn storage(error: sqlx::Error) -> RefreshSessionError {
    RefreshSessionError::Storage(DbError::Database(error).into())
}
fn village_id(id: u32) -> Result<i32, RefreshSessionError> {
    i32::try_from(id)
        .map_err(|e| RefreshSessionError::Storage(ApplicationError::Infrastructure(e.to_string())))
}
impl TryFrom<DbRefreshSessionRow> for RefreshSession {
    type Error = RefreshSessionError;
    fn try_from(row: DbRefreshSessionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            player_id: row.player_id,
            current_village_id: u32::try_from(row.current_village_id).map_err(|e| {
                RefreshSessionError::Storage(ApplicationError::Infrastructure(e.to_string()))
            })?,
            expires_at: row.expires_at,
            revoked_at: row.revoked_at,
        })
    }
}
async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    session: &RefreshSession,
    hash: &str,
    client: SessionClient,
) -> Result<(), RefreshSessionError> {
    sqlx::query("INSERT INTO auth_refresh_sessions (id, user_id, player_id, current_village_id, token_hash, expires_at, user_agent, ip) VALUES ($1,$2,$3,$4,$5,$6,$7,$8::inet)")
        .bind(session.id).bind(session.user_id).bind(session.player_id).bind(village_id(session.current_village_id)?)
        .bind(hash).bind(session.expires_at).bind(client.user_agent).bind(client.ip.map(|ip| ip.to_string()))
        .execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}

#[async_trait]
impl RefreshSessionRepository for PostgresRefreshSessionRepository {
    async fn create(
        &self,
        session: &RefreshSession,
        hash: &str,
        client: SessionClient,
    ) -> Result<(), RefreshSessionError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        // Serialize against logout-all, including session insertions.
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(session.user_id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        insert(&mut tx, session, hash, client).await?;
        tx.commit().await.map_err(storage)
    }
    async fn rotate(
        &self,
        old_hash: &str,
        replacement: SessionReplacement,
    ) -> Result<RefreshSession, RefreshSessionError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        // User lock before session locks when creating or rotating: logout-all cannot miss a concurrently inserted successor.
        let user: Option<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM auth_refresh_sessions WHERE token_hash = $1")
                .bind(old_hash)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?;
        let user = user.ok_or(RefreshSessionError::Expired)?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        let row = sqlx::query_as::<_, DbRefreshSessionRow>("SELECT id,user_id,player_id,current_village_id,expires_at,revoked_at FROM auth_refresh_sessions WHERE token_hash = $1 FOR UPDATE")
            .bind(old_hash).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(RefreshSessionError::Expired)?;
        let old = RefreshSession::try_from(row)?;
        old.validate(Utc::now())?;
        let next = RefreshSession {
            id: replacement.id,
            expires_at: replacement.expires_at,
            revoked_at: None,
            ..old.clone()
        };
        sqlx::query("UPDATE auth_refresh_sessions SET revoked_at = NOW() WHERE id = $1")
            .bind(old.id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        insert(&mut tx, &next, &replacement.token_hash, replacement.client).await?;
        tx.commit().await.map_err(storage)?;
        Ok(next)
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RefreshSession>, RefreshSessionError> {
        sqlx::query_as::<_, DbRefreshSessionRow>("SELECT id,user_id,player_id,current_village_id,expires_at,revoked_at FROM auth_refresh_sessions WHERE id = $1")
            .bind(id).fetch_optional(&self.pool).await.map_err(storage)?.map(RefreshSession::try_from).transpose()
    }
    async fn find_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<RefreshSession>, RefreshSessionError> {
        sqlx::query_as::<_, DbRefreshSessionRow>("SELECT id,user_id,player_id,current_village_id,expires_at,revoked_at FROM auth_refresh_sessions WHERE token_hash = $1")
            .bind(hash).fetch_optional(&self.pool).await.map_err(storage)?.map(RefreshSession::try_from).transpose()
    }
    async fn revoke(&self, hash: &str) -> Result<(), RefreshSessionError> {
        // The row update serializes with rotation: if this wins, rotation is rejected.
        // If rotation wins, logout of the consumed token does not revoke its successor.
        sqlx::query("UPDATE auth_refresh_sessions SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL")
            .bind(hash).execute(&self.pool).await.map_err(storage)?;
        Ok(())
    }
    async fn revoke_all(&self, user_id: Uuid) -> Result<(), RefreshSessionError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        sqlx::query("UPDATE auth_refresh_sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
            .bind(user_id).execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)
    }
    async fn set_village(
        &self,
        id: Uuid,
        current_village_id: u32,
    ) -> Result<(), RefreshSessionError> {
        let result = sqlx::query("UPDATE auth_refresh_sessions SET current_village_id = $1, last_used_at = NOW() WHERE id = $2 AND revoked_at IS NULL AND expires_at > NOW()")
            .bind(village_id(current_village_id)?).bind(id).execute(&self.pool).await.map_err(storage)?;
        if result.rows_affected() == 0 {
            return Err(RefreshSessionError::Revoked);
        }
        Ok(())
    }
}
