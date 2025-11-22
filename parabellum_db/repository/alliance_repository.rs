use async_trait::async_trait;
use parabellum_app::repository::{
    AllianceDiplomacyRepository, AllianceInviteRepository, AllianceLogRepository,
    AllianceRepository,
};
use parabellum_types::errors::{ApplicationError, DbError};
use parabellum_game::models::alliance::{Alliance, AllianceDiplomacy, AllianceInvite, AllianceLog};
use parabellum_game::models::player::Player;
use sqlx::{Postgres, Row, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{self as db_models};

#[derive(Clone)]
pub struct PostgresAllianceRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresAllianceRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl<'a> AllianceRepository for PostgresAllianceRepository<'a> {
    async fn save(&self, alliance: &Alliance) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            r#"
            INSERT INTO alliance (
                id, name, tag, desc1, desc2, info1, info2, forum_link, max_members, leader_id,
                total_attack_points, total_defense_points, total_robber_points, total_climber_points,
                current_attack_points, current_defense_points, current_robber_points, current_climber_points,
                recruitment_bonus_level, recruitment_bonus_contributions, metallurgy_bonus_level, metallurgy_bonus_contributions,
                philosophy_bonus_level, philosophy_bonus_contributions, commerce_bonus_level, commerce_bonus_contributions
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)
            ON CONFLICT (id) DO UPDATE
            SET
                name = $2,
                tag = $3,
                desc1 = $4,
                desc2 = $5,
                info1 = $6,
                info2 = $7,
                forum_link = $8,
                max_members = $9,
                leader_id = $10,
                total_attack_points = $11,
                total_defense_points = $12,
                total_robber_points = $13,
                total_climber_points = $14,
                current_attack_points = $15,
                current_defense_points = $16,
                current_robber_points = $17,
                current_climber_points = $18,
                recruitment_bonus_level = $19,
                recruitment_bonus_contributions = $20,
                metallurgy_bonus_level = $21,
                metallurgy_bonus_contributions = $22,
                philosophy_bonus_level = $23,
                philosophy_bonus_contributions = $24,
                commerce_bonus_level = $25,
                commerce_bonus_contributions = $26
            "#,
            alliance.id,
            &alliance.name,
            &alliance.tag,
            alliance.desc1.as_deref(),
            alliance.desc2.as_deref(),
            alliance.info1.as_deref(),
            alliance.info2.as_deref(),
            alliance.forum_link.as_deref(),
            alliance.max_members,
            alliance.leader_id,
            alliance.total_attack_points,
            alliance.total_defense_points,
            alliance.total_robber_points,
            alliance.total_climber_points,
            alliance.current_attack_points,
            alliance.current_defense_points,
            alliance.current_robber_points,
            alliance.current_climber_points,
            alliance.recruitment_bonus_level,
            alliance.recruitment_bonus_contributions,
            alliance.metallurgy_bonus_level,
            alliance.metallurgy_bonus_contributions,
            alliance.philosophy_bonus_level,
            alliance.philosophy_bonus_contributions,
            alliance.commerce_bonus_level,
            alliance.commerce_bonus_contributions,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Alliance, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let row = sqlx::query("SELECT * FROM alliance WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(Alliance {
            id: row.get("id"),
            name: row.get("name"),
            tag: row.get("tag"),
            desc1: row.get("desc1"),
            desc2: row.get("desc2"),
            info1: row.get("info1"),
            info2: row.get("info2"),
            forum_link: row.get("forum_link"),
            max_members: row.get("max_members"),
            leader_id: row.get("leader_id"),
            total_attack_points: row.get("total_attack_points"),
            total_defense_points: row.get("total_defense_points"),
            total_robber_points: row.get("total_robber_points"),
            total_climber_points: row.get("total_climber_points"),
            current_attack_points: row.get("current_attack_points"),
            current_defense_points: row.get("current_defense_points"),
            current_robber_points: row.get("current_robber_points"),
            current_climber_points: row.get("current_climber_points"),
            recruitment_bonus_level: row.get("recruitment_bonus_level"),
            recruitment_bonus_contributions: row.get("recruitment_bonus_contributions"),
            metallurgy_bonus_level: row.get("metallurgy_bonus_level"),
            metallurgy_bonus_contributions: row.get("metallurgy_bonus_contributions"),
            philosophy_bonus_level: row.get("philosophy_bonus_level"),
            philosophy_bonus_contributions: row.get("philosophy_bonus_contributions"),
            commerce_bonus_level: row.get("commerce_bonus_level"),
            commerce_bonus_contributions: row.get("commerce_bonus_contributions"),
        })
    }

    async fn get_by_tag(&self, tag: String) -> Result<Alliance, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let row = sqlx::query("SELECT * FROM alliance WHERE tag = $1")
            .bind(&tag)
            .fetch_one(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(Alliance {
            id: row.get("id"),
            name: row.get("name"),
            tag: row.get("tag"),
            desc1: row.get("desc1"),
            desc2: row.get("desc2"),
            info1: row.get("info1"),
            info2: row.get("info2"),
            forum_link: row.get("forum_link"),
            max_members: row.get("max_members"),
            leader_id: row.get("leader_id"),
            total_attack_points: row.get("total_attack_points"),
            total_defense_points: row.get("total_defense_points"),
            total_robber_points: row.get("total_robber_points"),
            total_climber_points: row.get("total_climber_points"),
            current_attack_points: row.get("current_attack_points"),
            current_defense_points: row.get("current_defense_points"),
            current_robber_points: row.get("current_robber_points"),
            current_climber_points: row.get("current_climber_points"),
            recruitment_bonus_level: row.get("recruitment_bonus_level"),
            recruitment_bonus_contributions: row.get("recruitment_bonus_contributions"),
            metallurgy_bonus_level: row.get("metallurgy_bonus_level"),
            metallurgy_bonus_contributions: row.get("metallurgy_bonus_contributions"),
            philosophy_bonus_level: row.get("philosophy_bonus_level"),
            philosophy_bonus_contributions: row.get("philosophy_bonus_contributions"),
            commerce_bonus_level: row.get("commerce_bonus_level"),
            commerce_bonus_contributions: row.get("commerce_bonus_contributions"),
        })
    }

    async fn get_by_name(&self, name: String) -> Result<Alliance, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let row = sqlx::query("SELECT * FROM alliance WHERE name = $1")
            .bind(&name)
            .fetch_one(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(Alliance {
            id: row.get("id"),
            name: row.get("name"),
            tag: row.get("tag"),
            desc1: row.get("desc1"),
            desc2: row.get("desc2"),
            info1: row.get("info1"),
            info2: row.get("info2"),
            forum_link: row.get("forum_link"),
            max_members: row.get("max_members"),
            leader_id: row.get("leader_id"),
            total_attack_points: row.get("total_attack_points"),
            total_defense_points: row.get("total_defense_points"),
            total_robber_points: row.get("total_robber_points"),
            total_climber_points: row.get("total_climber_points"),
            current_attack_points: row.get("current_attack_points"),
            current_defense_points: row.get("current_defense_points"),
            current_robber_points: row.get("current_robber_points"),
            current_climber_points: row.get("current_climber_points"),
            recruitment_bonus_level: row.get("recruitment_bonus_level"),
            recruitment_bonus_contributions: row.get("recruitment_bonus_contributions"),
            metallurgy_bonus_level: row.get("metallurgy_bonus_level"),
            metallurgy_bonus_contributions: row.get("metallurgy_bonus_contributions"),
            philosophy_bonus_level: row.get("philosophy_bonus_level"),
            philosophy_bonus_contributions: row.get("philosophy_bonus_contributions"),
            commerce_bonus_level: row.get("commerce_bonus_level"),
            commerce_bonus_contributions: row.get("commerce_bonus_contributions"),
        })
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query("DELETE FROM alliance WHERE id = $1")
            .bind(id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }

    async fn update(&self, alliance: &Alliance) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            r#"
            UPDATE alliance SET
                name = $1, tag = $2, desc1 = $3, desc2 = $4, info1 = $5, info2 = $6, forum_link = $7, max_members = $8, leader_id = $9,
                total_attack_points = $10, total_defense_points = $11, total_robber_points = $12, total_climber_points = $13,
                current_attack_points = $14, current_defense_points = $15, current_robber_points = $16, current_climber_points = $17,
                recruitment_bonus_level = $18, recruitment_bonus_contributions = $19, metallurgy_bonus_level = $20, metallurgy_bonus_contributions = $21,
                philosophy_bonus_level = $22, philosophy_bonus_contributions = $23, commerce_bonus_level = $24, commerce_bonus_contributions = $25
            WHERE id = $26
            "#,
            &alliance.name,
            &alliance.tag,
            alliance.desc1.as_deref(),
            alliance.desc2.as_deref(),
            alliance.info1.as_deref(),
            alliance.info2.as_deref(),
            alliance.forum_link.as_deref(),
            alliance.max_members,
            alliance.leader_id,
            alliance.total_attack_points,
            alliance.total_defense_points,
            alliance.total_robber_points,
            alliance.total_climber_points,
            alliance.current_attack_points,
            alliance.current_defense_points,
            alliance.current_robber_points,
            alliance.current_climber_points,
            alliance.recruitment_bonus_level,
            alliance.recruitment_bonus_contributions,
            alliance.metallurgy_bonus_level,
            alliance.metallurgy_bonus_contributions,
            alliance.philosophy_bonus_level,
            alliance.philosophy_bonus_contributions,
            alliance.commerce_bonus_level,
            alliance.commerce_bonus_contributions,
            alliance.id,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_leader(&self, alliance_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        // Get alliance to find leader_id
        let alliance_row = sqlx::query("SELECT leader_id FROM alliance WHERE id = $1")
            .bind(alliance_id)
            .fetch_one(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let leader_id: Uuid = alliance_row.get("leader_id");

        // Get the leader player
        let db_player = sqlx::query_as::<_, db_models::Player>(
            r#"SELECT id, username, tribe, user_id, created_at, alliance_id, alliance_role, alliance_join_time, current_alliance_recruitment_contributions, current_alliance_metallurgy_contributions, current_alliance_philosophy_contributions, current_alliance_commerce_contributions, total_alliance_recruitment_contributions, total_alliance_metallurgy_contributions, total_alliance_philosophy_contributions, total_alliance_commerce_contributions FROM players WHERE id = $1"#
        )
        .bind(leader_id)
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(leader_id)))?;

        Ok(db_player.into())
    }

    async fn count_members(&self, alliance_id: Uuid) -> Result<i64, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM players WHERE alliance_id = $1",
            alliance_id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(result.count.unwrap_or(0))
    }

    async fn list_members(&self, alliance_id: Uuid) -> Result<Vec<Player>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let db_players: Vec<db_models::Player> = sqlx::query_as(
            r#"
        SELECT
            id,
            username,
            user_id,
            tribe, -- NOTE: no ::text here
            alliance_id,
            alliance_role,
            alliance_join_time,
            current_alliance_recruitment_contributions,
            current_alliance_metallurgy_contributions,
            current_alliance_philosophy_contributions,
            current_alliance_commerce_contributions,
            total_alliance_recruitment_contributions,
            total_alliance_metallurgy_contributions,
            total_alliance_philosophy_contributions,
            total_alliance_commerce_contributions
        FROM players
        WHERE alliance_id = $1
        "#,
        )
        .bind(alliance_id)
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(db_players.into_iter().map(Into::into).collect())
    }
}

#[derive(Clone)]
pub struct PostgresAllianceInviteRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresAllianceInviteRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl<'a> AllianceInviteRepository for PostgresAllianceInviteRepository<'a> {
    async fn save(&self, invite: &AllianceInvite) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            "INSERT INTO alliance_invite (id, from_player_id, alliance_id, to_player_id) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET from_player_id = $2, alliance_id = $3, to_player_id = $4",
            invite.id,
            invite.from_player_id,
            invite.alliance_id,
            invite.to_player_id,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<AllianceInvite, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let row = sqlx::query_as!(
            db_models::AllianceInvite,
            "SELECT id, from_player_id, alliance_id, to_player_id FROM alliance_invite WHERE id = $1",
            id
        )
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.into())
    }

    async fn get_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<AllianceInvite>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query_as!(
            db_models::AllianceInvite,
            "SELECT id, from_player_id, alliance_id, to_player_id FROM alliance_invite WHERE to_player_id = $1",
            player_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_alliance_id(
        &self,
        alliance_id: Uuid,
    ) -> Result<Vec<AllianceInvite>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query_as!(
            db_models::AllianceInvite,
            "SELECT id, from_player_id, alliance_id, to_player_id FROM alliance_invite WHERE alliance_id = $1",
            alliance_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query("DELETE FROM alliance_invite WHERE id = $1")
            .bind(id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PostgresAllianceLogRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresAllianceLogRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl<'a> AllianceLogRepository for PostgresAllianceLogRepository<'a> {
    async fn save(&self, log: &AllianceLog) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            "INSERT INTO alliance_log (id, alliance_id, type, data, created_at) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET alliance_id = $2, type = $3, data = $4, created_at = $5",
            log.id,
            log.alliance_id,
            log.type_,
            log.data.as_deref(),
            log.created_at,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_alliance_id(
        &self,
        alliance_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<AllianceLog>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query_as!(
            db_models::AllianceLog,
            "SELECT id, alliance_id, type AS type_, data, created_at FROM alliance_log WHERE alliance_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            alliance_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(Clone)]
pub struct PostgresAllianceDiplomacyRepository<'a> {
    tx: Arc<Mutex<Transaction<'a, Postgres>>>,
}

impl<'a> PostgresAllianceDiplomacyRepository<'a> {
    pub fn new(tx: Arc<Mutex<Transaction<'a, Postgres>>>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl<'a> AllianceDiplomacyRepository for PostgresAllianceDiplomacyRepository<'a> {
    async fn save(&self, diplomacy: &AllianceDiplomacy) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            "INSERT INTO alliance_diplomacy (id, alliance1_id, alliance2_id, type, accepted) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET alliance1_id = $2, alliance2_id = $3, type = $4, accepted = $5",
            diplomacy.id,
            diplomacy.alliance1_id,
            diplomacy.alliance2_id,
            diplomacy.type_,
            diplomacy.accepted,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<AllianceDiplomacy>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let row = sqlx::query_as!(
            db_models::AllianceDiplomacy,
            "SELECT id, alliance1_id, alliance2_id, type AS type_, accepted FROM alliance_diplomacy WHERE id = $1",
            id
        )
        .fetch_optional(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_by_alliance_id(
        &self,
        alliance_id: Uuid,
    ) -> Result<Vec<AllianceDiplomacy>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query_as!(
            db_models::AllianceDiplomacy,
            "SELECT id, alliance1_id, alliance2_id, type AS type_, accepted FROM alliance_diplomacy WHERE alliance1_id = $1 OR alliance2_id = $1",
            alliance_id
        )
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_between_alliances(
        &self,
        alliance1_id: Uuid,
        alliance2_id: Uuid,
    ) -> Result<Option<AllianceDiplomacy>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let row = sqlx::query_as!(
            db_models::AllianceDiplomacy,
            "SELECT id, alliance1_id, alliance2_id, type AS type_, accepted FROM alliance_diplomacy WHERE (alliance1_id = $1 AND alliance2_id = $2) OR (alliance1_id = $2 AND alliance2_id = $1)",
            alliance1_id,
            alliance2_id
        )
        .fetch_optional(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(row.map(|r| r.into()))
    }

    async fn update(&self, diplomacy: &AllianceDiplomacy) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query!(
            "UPDATE alliance_diplomacy SET alliance1_id = $2, alliance2_id = $3, type = $4, accepted = $5 WHERE id = $1",
            diplomacy.id,
            diplomacy.alliance1_id,
            diplomacy.alliance2_id,
            diplomacy.type_,
            diplomacy.accepted,
        )
        .execute(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        sqlx::query("DELETE FROM alliance_diplomacy WHERE id = $1")
            .bind(id)
            .execute(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        Ok(())
    }
}
