use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use uuid::Uuid;

use parabellum_app::repository::{NewReport, ReportAudience, ReportRecord, ReportRepository};
use parabellum_types::{
    Result,
    errors::{ApplicationError, DbError},
};

use crate::toasty_models::report::{
    ReportDbRow, ReportReadDbRow, chrono_to_jiff, jiff_to_chrono_utc, to_report_record,
};

pub struct ToastyReportRepository<'a> {
    tx: Arc<Mutex<toasty::Transaction<'a>>>,
}

impl<'a> ToastyReportRepository<'a> {
    pub fn new(tx: Arc<Mutex<toasty::Transaction<'a>>>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl<'a> ReportRepository for ToastyReportRepository<'a> {
    async fn add(
        &self,
        report: &NewReport,
        audiences: &[ReportAudience],
    ) -> Result<(), ApplicationError> {
        let report_id = Uuid::new_v4();
        let mut tx_guard = self.tx.lock().await;

        toasty::create!(ReportDbRow {
            id: report_id,
            report_type: report.report_type.clone(),
            payload: report.payload.clone(),
            actor_player_id: report.actor_player_id,
            actor_village_id: report.actor_village_id.map(|id| id as i32),
            target_player_id: report.target_player_id,
            target_village_id: report.target_village_id.map(|id| id as i32),
            created_at: jiff::Timestamp::now(),
        })
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        for audience in audiences {
            toasty::create!(ReportReadDbRow {
                report_id,
                player_id: audience.player_id,
                read_at: audience.read_at.map(chrono_to_jiff).transpose()?,
            })
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        }

        Ok(())
    }

    async fn list_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ReportRecord>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let reads = toasty::query!(ReportReadDbRow filter .player_id == #player_id)
            .exec(&mut *tx_guard)
            .await
            .map_err(map_toasty_error)?;
        if reads.is_empty() || limit <= 0 {
            return Ok(vec![]);
        }

        let mut by_report_id: HashMap<Uuid, ReportAudience> = HashMap::new();
        for read in reads {
            by_report_id.insert(
                read.report_id,
                ReportAudience {
                    player_id,
                    read_at: read.read_at.map(jiff_to_chrono_utc).transpose()?,
                },
            );
        }

        let mut rows = Vec::with_capacity(by_report_id.len());
        for report_id in by_report_id.keys() {
            let row = ReportDbRow::get_by_id(&mut *tx_guard, *report_id)
                .await
                .map_err(map_toasty_error)?;
            rows.push(row);
        }

        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        rows.truncate(usize::try_from(limit).unwrap_or(0));

        rows.into_iter()
            .map(|row| {
                let audience = by_report_id.get(&row.id).ok_or_else(|| {
                    ApplicationError::Db(DbError::Transaction(
                        "missing report audience row".to_string(),
                    ))
                })?;
                to_report_record(row, audience)
            })
            .collect()
    }

    async fn get_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportRecord>, ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut reads = toasty::query!(
            ReportReadDbRow filter .report_id == #report_id and .player_id == #player_id
        )
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        let Some(read) = reads.pop() else {
            return Ok(None);
        };

        let report = ReportDbRow::get_by_id(&mut *tx_guard, report_id)
            .await
            .map_err(map_toasty_error)?;
        let audience = ReportAudience {
            player_id,
            read_at: read.read_at.map(jiff_to_chrono_utc).transpose()?,
        };
        Ok(Some(to_report_record(report, &audience)?))
    }

    async fn mark_as_read(&self, report_id: Uuid, player_id: Uuid) -> Result<(), ApplicationError> {
        let mut tx_guard = self.tx.lock().await;
        let mut reads = toasty::query!(
            ReportReadDbRow filter .report_id == #report_id and .player_id == #player_id
        )
        .exec(&mut *tx_guard)
        .await
        .map_err(map_toasty_error)?;

        if let Some(mut read) = reads.pop() {
            read.update()
                .read_at(Some(jiff::Timestamp::now()))
                .exec(&mut *tx_guard)
                .await
                .map_err(map_toasty_error)?;
        }

        Ok(())
    }
}

fn map_toasty_error(err: toasty::Error) -> ApplicationError {
    ApplicationError::Db(DbError::Transaction(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::toasty_db::establish_test_toasty_db;

    #[tokio::test]
    async fn toasty_report_repo_add_list_get_and_mark_read() -> Result<(), ApplicationError> {
        let pool = crate::establish_test_connection_pool()
            .await
            .map_err(ApplicationError::Db)?;
        let seed: Option<(Uuid, i32)> = sqlx::query_as(
            "SELECT p.id, v.id FROM players p JOIN villages v ON v.player_id = p.id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ApplicationError::Db(DbError::Database(e)))?;
        let Some((player_id, village_id)) = seed else {
            return Ok(());
        };

        let mut toasty_db = establish_test_toasty_db()
            .await
            .map_err(ApplicationError::Db)?;
        let tx = toasty_db.transaction().await.map_err(map_toasty_error)?;
        let tx = Arc::new(Mutex::new(tx));
        let repo = ToastyReportRepository::new(tx.clone());

        let report = NewReport {
            report_type: "Battle".to_string(),
            payload: parabellum_types::reports::ReportPayload::MarketplaceDelivery(
                parabellum_types::reports::MarketplaceDeliveryReportPayload {
                    sender_player: "sender".to_string(),
                    sender_village: "sender-village".to_string(),
                    sender_position: parabellum_types::map::Position { x: 1, y: 1 },
                    receiver_player: "receiver".to_string(),
                    receiver_village: "receiver-village".to_string(),
                    receiver_position: parabellum_types::map::Position { x: 2, y: 2 },
                    resources: parabellum_types::common::ResourceGroup::new(10, 20, 30, 40),
                    merchants_used: 1,
                },
            ),
            actor_player_id: player_id,
            actor_village_id: Some(village_id as u32),
            target_player_id: Some(player_id),
            target_village_id: Some(village_id as u32),
        };
        let audience = ReportAudience {
            player_id,
            read_at: None,
        };

        repo.add(&report, &[audience]).await?;
        let list = repo.list_for_player(player_id, 20).await?;
        assert!(!list.is_empty());

        let found = list
            .iter()
            .find(|r| r.report_type == "Battle")
            .ok_or_else(|| {
                ApplicationError::Db(DbError::Transaction("missing inserted report".to_string()))
            })?;

        let fetched = repo.get_for_player(found.id, player_id).await?;
        assert!(fetched.is_some());
        assert_eq!(fetched.expect("checked above").id, found.id);

        repo.mark_as_read(found.id, player_id).await?;
        let after_read = repo
            .get_for_player(found.id, player_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::Db(DbError::Transaction("report disappeared".to_string()))
            })?;
        assert!(after_read.read_at.is_some());

        drop(repo);
        drop(tx); // rollback on drop
        Ok(())
    }
}
