use async_trait::async_trait;
use parabellum_core::{ApplicationError, DbError};
use parabellum_game::models::alliance::{Alliance, AllianceInvite, AllianceLog, AllianceDiplomacy};
use parabellum_app::repository::{AllianceRepository, AllianceInviteRepository, AllianceLogRepository, AllianceDiplomacyRepository};
use parabellum_types::common::Player;
use parabellum_types::tribe::Tribe;
use sqlx::{Postgres, Transaction, Row};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

fn parse_tribe(tribe_str: &str) -> Result<Tribe, ApplicationError> {
    match tribe_str {
        "Roman" => Ok(Tribe::Roman),
        "Gaul" => Ok(Tribe::Gaul),
        "Teuton" => Ok(Tribe::Teuton),
        "Natar" => Ok(Tribe::Natar),
        "Nature" => Ok(Tribe::Nature),
        _ => Err(ApplicationError::Db(DbError::Database(sqlx::Error::ColumnNotFound(format!("Invalid tribe: {}", tribe_str))))),
    }
}

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
                total_attack_points, total_defense_points, current_attack_points, current_defense_points, current_robber_points,
                training_bonus_level, training_bonus_contributions, armor_bonus_level, armor_bonus_contributions,
                cp_bonus_level, cp_bonus_contributions, trade_bonus_level, trade_bonus_contributions, old_pop
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
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
                current_attack_points = $13,
                current_defense_points = $14,
                current_robber_points = $15,
                training_bonus_level = $16,
                training_bonus_contributions = $17,
                armor_bonus_level = $18,
                armor_bonus_contributions = $19,
                cp_bonus_level = $20,
                cp_bonus_contributions = $21,
                trade_bonus_level = $22,
                trade_bonus_contributions = $23,
                old_pop = $24
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
            alliance.current_attack_points,
            alliance.current_defense_points,
            alliance.current_robber_points,
            alliance.training_bonus_level,
            alliance.training_bonus_contributions,
            alliance.armor_bonus_level,
            alliance.armor_bonus_contributions,
            alliance.cp_bonus_level,
            alliance.cp_bonus_contributions,
            alliance.trade_bonus_level,
            alliance.trade_bonus_contributions,
            alliance.old_pop,
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
            current_attack_points: row.get("current_attack_points"),
            current_defense_points: row.get("current_defense_points"),
            current_robber_points: row.get("current_robber_points"),
            training_bonus_level: row.get("training_bonus_level"),
            training_bonus_contributions: row.get("training_bonus_contributions"),
            armor_bonus_level: row.get("armor_bonus_level"),
            armor_bonus_contributions: row.get("armor_bonus_contributions"),
            cp_bonus_level: row.get("cp_bonus_level"),
            cp_bonus_contributions: row.get("cp_bonus_contributions"),
            trade_bonus_level: row.get("trade_bonus_level"),
            trade_bonus_contributions: row.get("trade_bonus_contributions"),
            old_pop: row.get("old_pop"),
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
            current_attack_points: row.get("current_attack_points"),
            current_defense_points: row.get("current_defense_points"),
            current_robber_points: row.get("current_robber_points"),
            training_bonus_level: row.get("training_bonus_level"),
            training_bonus_contributions: row.get("training_bonus_contributions"),
            armor_bonus_level: row.get("armor_bonus_level"),
            armor_bonus_contributions: row.get("armor_bonus_contributions"),
            cp_bonus_level: row.get("cp_bonus_level"),
            cp_bonus_contributions: row.get("cp_bonus_contributions"),
            trade_bonus_level: row.get("trade_bonus_level"),
            trade_bonus_contributions: row.get("trade_bonus_contributions"),
            old_pop: row.get("old_pop"),
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
            current_attack_points: row.get("current_attack_points"),
            current_defense_points: row.get("current_defense_points"),
            current_robber_points: row.get("current_robber_points"),
            training_bonus_level: row.get("training_bonus_level"),
            training_bonus_contributions: row.get("training_bonus_contributions"),
            armor_bonus_level: row.get("armor_bonus_level"),
            armor_bonus_contributions: row.get("armor_bonus_contributions"),
            cp_bonus_level: row.get("cp_bonus_level"),
            cp_bonus_contributions: row.get("cp_bonus_contributions"),
            trade_bonus_level: row.get("trade_bonus_level"),
            trade_bonus_contributions: row.get("trade_bonus_contributions"),
            old_pop: row.get("old_pop"),
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
                total_attack_points = $10, total_defense_points = $11, current_attack_points = $12, current_defense_points = $13, current_robber_points = $14,
                training_bonus_level = $15, training_bonus_contributions = $16, armor_bonus_level = $17, armor_bonus_contributions = $18,
                cp_bonus_level = $19, cp_bonus_contributions = $20, trade_bonus_level = $21, trade_bonus_contributions = $22, old_pop = $23
            WHERE id = $24
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
            alliance.current_attack_points,
            alliance.current_defense_points,
            alliance.current_robber_points,
            alliance.training_bonus_level,
            alliance.training_bonus_contributions,
            alliance.armor_bonus_level,
            alliance.armor_bonus_contributions,
            alliance.cp_bonus_level,
            alliance.cp_bonus_contributions,
            alliance.trade_bonus_level,
            alliance.trade_bonus_contributions,
            alliance.old_pop,
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
        let row = sqlx::query(
            r#"SELECT id, username, user_id, tribe::text as tribe, alliance_id, alliance_role, alliance_join_time,
               current_alliance_training_contributions, current_alliance_armor_contributions,
               current_alliance_cp_contributions, current_alliance_trade_contributions,
               total_alliance_training_contributions, total_alliance_armor_contributions,
               total_alliance_cp_contributions, total_alliance_trade_contributions
               FROM players WHERE id = $1"#
        )
        .bind(leader_id)
        .fetch_one(&mut *tx_guard.as_mut())
        .await
        .map_err(|_| ApplicationError::Db(DbError::PlayerNotFound(leader_id)))?;

        let tribe_str: String = row.get("tribe");
        let tribe = parse_tribe(&tribe_str)?;

        Ok(Player {
            id: row.get("id"),
            username: row.get("username"),
            user_id: row.get("user_id"),
            tribe,
            alliance_id: row.get("alliance_id"),
            alliance_role: row.get("alliance_role"),
            alliance_join_time: row.get("alliance_join_time"),
            current_alliance_training_contributions: row.get("current_alliance_training_contributions"),
            current_alliance_armor_contributions: row.get("current_alliance_armor_contributions"),
            current_alliance_cp_contributions: row.get("current_alliance_cp_contributions"),
            current_alliance_trade_contributions: row.get("current_alliance_trade_contributions"),
            total_alliance_training_contributions: row.get("total_alliance_training_contributions"),
            total_alliance_armor_contributions: row.get("total_alliance_armor_contributions"),
            total_alliance_cp_contributions: row.get("total_alliance_cp_contributions"),
            total_alliance_trade_contributions: row.get("total_alliance_trade_contributions"),
        })
    }

    async fn count_members(&self, alliance_id: Uuid) -> Result<i64, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let result = sqlx::query!("SELECT COUNT(*) as count FROM players WHERE alliance_id = $1", alliance_id)
            .fetch_one(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(result.count.unwrap_or(0))
    }

    async fn list_members(&self, alliance_id: Uuid) -> Result<Vec<Player>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query(
            r#"SELECT id, username, user_id, tribe::text as tribe, alliance_id, alliance_role, alliance_join_time,
               current_alliance_training_contributions, current_alliance_armor_contributions,
               current_alliance_cp_contributions, current_alliance_trade_contributions,
               total_alliance_training_contributions, total_alliance_armor_contributions,
               total_alliance_cp_contributions, total_alliance_trade_contributions
               FROM players WHERE alliance_id = $1"#
        )
        .bind(alliance_id)
        .fetch_all(&mut *tx_guard.as_mut())
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        let mut players: Vec<Player> = Vec::new();

        for row in rows {
            let tribe_str: String = row.get("tribe");
            let tribe = parse_tribe(&tribe_str)?;

            players.push(Player {
                id: row.get("id"),
                username: row.get("username"),
                user_id: row.get("user_id"),
                tribe,
                alliance_id: row.get("alliance_id"),
                alliance_role: row.get("alliance_role"),
                alliance_join_time: row.get("alliance_join_time"),
                current_alliance_training_contributions: row.get("current_alliance_training_contributions"),
                current_alliance_armor_contributions: row.get("current_alliance_armor_contributions"),
                current_alliance_cp_contributions: row.get("current_alliance_cp_contributions"),
                current_alliance_trade_contributions: row.get("current_alliance_trade_contributions"),
                total_alliance_training_contributions: row.get("total_alliance_training_contributions"),
                total_alliance_armor_contributions: row.get("total_alliance_armor_contributions"),
                total_alliance_cp_contributions: row.get("total_alliance_cp_contributions"),
                total_alliance_trade_contributions: row.get("total_alliance_trade_contributions"),
            });
        }

        Ok(players)
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
        
        let row = sqlx::query("SELECT * FROM alliance_invite WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(AllianceInvite {
            id: row.get("id"),
            from_player_id: row.get("from_player_id"),
            alliance_id: row.get("alliance_id"),
            to_player_id: row.get("to_player_id"),
        })
    }

    async fn get_by_player_id(&self, player_id: Uuid) -> Result<Vec<AllianceInvite>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query("SELECT * FROM alliance_invite WHERE to_player_id = $1")
            .bind(player_id)
            .fetch_all(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.iter().map(|row| AllianceInvite {
            id: row.get("id"),
            from_player_id: row.get("from_player_id"),
            alliance_id: row.get("alliance_id"),
            to_player_id: row.get("to_player_id"),
        }).collect())
    }

    async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<AllianceInvite>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        
        let rows = sqlx::query("SELECT * FROM alliance_invite WHERE alliance_id = $1")
            .bind(alliance_id)
            .fetch_all(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.iter().map(|row| AllianceInvite {
            id: row.get("id"),
            from_player_id: row.get("from_player_id"),
            alliance_id: row.get("alliance_id"),
            to_player_id: row.get("to_player_id"),
        }).collect())
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

    async fn get_by_alliance_id(&self, alliance_id: Uuid, limit: i32, offset: i32) -> Result<Vec<AllianceLog>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query("SELECT * FROM alliance_log WHERE alliance_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3")
            .bind(alliance_id)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.iter().map(|row| AllianceLog {
            id: row.get("id"),
            alliance_id: row.get("alliance_id"),
            type_: row.get("type"),
            data: row.get("data"),
            created_at: row.get("created_at"),
        }).collect())
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

    async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<AllianceDiplomacy>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;

        let rows = sqlx::query("SELECT * FROM alliance_diplomacy WHERE alliance1_id = $1 OR alliance2_id = $1")
            .bind(alliance_id)
            .fetch_all(&mut *tx_guard.as_mut())
            .await
            .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;

        Ok(rows.iter().map(|row| AllianceDiplomacy {
            id: row.get("id"),
            alliance1_id: row.get("alliance1_id"),
            alliance2_id: row.get("alliance2_id"),
            type_: row.get("type"),
            accepted: row.get("accepted"),
        }).collect())
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
