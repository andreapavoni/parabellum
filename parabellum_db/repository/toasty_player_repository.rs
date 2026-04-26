use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::{PlayerLeaderboardEntry, PlayerRepository};
use parabellum_types::{
    Result,
    common::Player,
    errors::{ApplicationError, DbError},
};

use crate::toasty_models::{player::PlayerRecord, village_stats::VillageStatsRecord};

pub struct ToastyPlayerRepository<'a> {
    tx: Arc<Mutex<toasty::Transaction<'a>>>,
}

impl<'a> ToastyPlayerRepository<'a> {
    pub fn new(tx: Arc<Mutex<toasty::Transaction<'a>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> PlayerRepository for ToastyPlayerRepository<'a> {
    async fn save(&self, player: &Player) -> Result<(), ApplicationError> {
        let record = PlayerRecord::try_from(player)?;
        let player_id = record.id;
        let mut tx_guard = self.tx.lock().await;

        let mut rows = toasty::query!(PlayerRecord filter .id == #player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        if let Some(mut existing) = rows.pop() {
            existing
                .update()
                .username(record.username)
                .tribe(record.tribe)
                .user_id(record.user_id)
                .culture_points(record.culture_points)
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
        } else {
            toasty::create!(PlayerRecord {
                id: record.id,
                username: record.username,
                tribe: record.tribe,
                user_id: record.user_id,
                culture_points: record.culture_points,
            })
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        }

        Ok(())
    }

    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut rows = toasty::query!(PlayerRecord filter .id == #player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let row = rows
            .pop()
            .ok_or_else(|| ApplicationError::Db(DbError::PlayerNotFound(player_id)))?;
        Player::try_from(row)
    }

    async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut rows = toasty::query!(PlayerRecord filter .user_id == #user_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let row = rows
            .pop()
            .ok_or_else(|| ApplicationError::Db(DbError::UserPlayerNotFound(user_id)))?;
        Player::try_from(row)
    }

    async fn leaderboard_page(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<PlayerLeaderboardEntry>, i64), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let players = PlayerRecord::all()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let villages = VillageStatsRecord::all()
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        let mut per_player: HashMap<Uuid, (i64, i64)> = HashMap::new();
        for village in villages {
            let entry = per_player.entry(village.player_id).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += i64::from(village.population);
        }

        let mut entries: Vec<PlayerLeaderboardEntry> = players
            .into_iter()
            .map(|row| {
                let (village_count, population) =
                    per_player.get(&row.id).copied().unwrap_or((0, 0));
                let player = Player::try_from(row)?;
                Ok(PlayerLeaderboardEntry {
                    player_id: player.id,
                    username: player.username,
                    village_count,
                    population,
                    tribe: player.tribe,
                })
            })
            .collect::<Result<_, ApplicationError>>()?;

        entries.sort_by(|a, b| {
            b.population
                .cmp(&a.population)
                .then_with(|| a.username.cmp(&b.username))
        });

        let total_players = i64::try_from(entries.len()).unwrap_or(i64::MAX);
        let start = usize::try_from(offset.max(0))
            .unwrap_or(usize::MAX)
            .min(entries.len());
        let max_len = usize::try_from(limit.max(0)).unwrap_or(0);
        let end = start.saturating_add(max_len).min(entries.len());
        let page = entries[start..end].to_vec();

        Ok((page, total_players))
    }

    async fn update_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let villages = toasty::query!(VillageStatsRecord filter .player_id == #player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        let total: i64 = villages.iter().map(|v| i64::from(v.culture_points)).sum();
        let total = i32::try_from(total).map_err(|_| {
            ApplicationError::Db(DbError::Transaction(format!(
                "culture_points overflow while aggregating player {player_id}",
            )))
        })?;

        let mut rows = toasty::query!(PlayerRecord filter .id == #player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        if let Some(mut player) = rows.pop() {
            player
                .update()
                .culture_points(total)
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
        }

        Ok(())
    }

    async fn get_total_culture_points_production(
        &self,
        player_id: Uuid,
    ) -> Result<u32, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let villages = toasty::query!(VillageStatsRecord filter .player_id == #player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;

        let total: i64 = villages
            .iter()
            .map(|v| i64::from(v.culture_points_production))
            .sum();

        u32::try_from(total).map_err(|_| {
            ApplicationError::Db(DbError::Transaction(format!(
                "culture_points_production overflow while aggregating player {player_id}",
            )))
        })
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use parabellum_app::repository::UserRepository;
    use parabellum_types::tribe::Tribe;

    use crate::{repository::ToastyUserRepository, toasty_db::establish_test_toasty_db};

    #[tokio::test]
    async fn toasty_player_repo_save_get_and_leaderboard() -> Result<(), ApplicationError> {
        let mut toasty_db = establish_test_toasty_db()
            .await
            .map_err(ApplicationError::Db)?;

        let tx = toasty_db.transaction().await.map_err(map_toasty_error)?;
        let tx = Arc::new(Mutex::new(tx));
        let user_repo = ToastyUserRepository::new(tx.clone());
        let player_repo = ToastyPlayerRepository::new(tx.clone());

        let unique = Uuid::new_v4();
        let email = format!("toasty-player-{unique}@example.test");
        user_repo
            .save(email.clone(), format!("hash-{unique}"))
            .await?;
        let user = user_repo.get_by_email(&email).await?;

        let player = Player {
            id: Uuid::new_v4(),
            username: format!("toasty-player-{unique}"),
            tribe: Tribe::Roman,
            user_id: user.id,
            culture_points: 0,
        };

        player_repo.save(&player).await?;

        let loaded = player_repo.get_by_id(player.id).await?;
        assert_eq!(loaded.id, player.id);

        let by_user = player_repo.get_by_user_id(user.id).await?;
        assert_eq!(by_user.id, player.id);

        let (entries, total) = player_repo.leaderboard_page(0, 50).await?;
        assert!(total >= 1);
        assert!(entries.iter().any(|entry| entry.player_id == player.id));

        let cpp = player_repo
            .get_total_culture_points_production(player.id)
            .await?;
        assert_eq!(cpp, 0);

        drop(player_repo);
        drop(user_repo);
        drop(tx); // rollback on drop

        Ok(())
    }
}
