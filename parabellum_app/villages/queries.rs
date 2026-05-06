use std::sync::Arc;

use mini_cqrs_es::{CqrsError, Query};

use crate::villages::models::{
    MarketplaceOfferModel, ReportModel, ScheduledActionStatus, ScheduledActionType,
};
use crate::villages::repositories::{
    MarketplaceOfferRepository, ReportReadModelRepository, ScheduledActionRepository,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScheduledActionStatusCounts {
    /// Number of actions currently pending.
    pub pending: usize,
    /// Number of actions currently locked/processing.
    pub processing: usize,
    /// Number of actions completed successfully.
    pub completed: usize,
    /// Number of actions failed.
    pub failed: usize,
}

/// Query that computes scheduled-action status counters for one village and action type.
pub struct GetScheduledActionStatusCounts {
    pub repository: Arc<dyn ScheduledActionRepository>,
    pub village_id: u32,
    pub action_type: ScheduledActionType,
    /// Optional status filter. When set, only actions with this status are counted.
    pub status_filter: Option<ScheduledActionStatus>,
}

impl Query for GetScheduledActionStatusCounts {
    type Output = Result<ScheduledActionStatusCounts, CqrsError>;

    async fn apply(&self) -> Self::Output {
        let actions = self
            .repository
            .list_by_village_and_type(self.village_id, self.action_type)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))?;

        let mut counts = ScheduledActionStatusCounts::default();
        for action in actions {
            if let Some(status_filter) = self.status_filter {
                if action.status != status_filter {
                    continue;
                }
            }
            match action.status {
                ScheduledActionStatus::Pending => counts.pending += 1,
                ScheduledActionStatus::Processing => counts.processing += 1,
                ScheduledActionStatus::Completed => counts.completed += 1,
                ScheduledActionStatus::Failed => counts.failed += 1,
            }
        }
        Ok(counts)
    }
}

/// Query that returns all open marketplace offers from ES read models.
pub struct GetOpenMarketplaceOffers {
    pub repository: Arc<dyn MarketplaceOfferRepository>,
}

impl Query for GetOpenMarketplaceOffers {
    type Output = Result<Vec<MarketplaceOfferModel>, CqrsError>;

    async fn apply(&self) -> Self::Output {
        self.repository
            .list_open()
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

/// Query that returns a marketplace offer by id from ES read models.
pub struct GetMarketplaceOfferById {
    pub repository: Arc<dyn MarketplaceOfferRepository>,
    pub offer_id: uuid::Uuid,
}

impl Query for GetMarketplaceOfferById {
    type Output = Result<MarketplaceOfferModel, CqrsError>;

    async fn apply(&self) -> Self::Output {
        self.repository
            .get_by_offer_id(self.offer_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

/// Query that lists projected reports for one player from ES read models.
pub struct ListReportsForPlayer {
    pub repository: Arc<dyn ReportReadModelRepository>,
    pub player_id: uuid::Uuid,
    pub limit: i64,
}

impl Query for ListReportsForPlayer {
    type Output = Result<Vec<ReportModel>, CqrsError>;

    async fn apply(&self) -> Self::Output {
        self.repository
            .list_for_player(self.player_id, self.limit)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

/// Query that loads one projected report for one player from ES read models.
pub struct GetReportForPlayer {
    pub repository: Arc<dyn ReportReadModelRepository>,
    pub report_id: uuid::Uuid,
    pub player_id: uuid::Uuid,
}

impl Query for GetReportForPlayer {
    type Output = Result<Option<ReportModel>, CqrsError>;

    async fn apply(&self) -> Self::Output {
        self.repository
            .get_for_player(self.report_id, self.player_id)
            .await
            .map_err(|e| CqrsError::EventStore(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use parabellum_types::{
        army::TroopSet,
        map::Position,
        reports::{ReinforcementReportPayload, ReportPayload},
        tribe::Tribe,
    };
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    struct MockReportRepo {
        rows: Mutex<HashMap<Uuid, ReportModel>>,
        reads: Mutex<HashMap<(Uuid, Uuid), Option<chrono::DateTime<Utc>>>>,
    }

    #[async_trait::async_trait]
    impl ReportReadModelRepository for MockReportRepo {
        async fn list_for_player(
            &self,
            player_id: Uuid,
            limit: i64,
        ) -> Result<Vec<ReportModel>, parabellum_types::errors::ApplicationError> {
            let reads = self.reads.lock().await;
            let rows = self.rows.lock().await;
            let mut out = Vec::new();
            for ((report_id, pid), read_at) in reads.iter() {
                if *pid != player_id {
                    continue;
                }
                if let Some(row) = rows.get(report_id) {
                    let mut projected = row.clone();
                    projected.read_at = *read_at;
                    out.push(projected);
                }
            }
            out.sort_by_key(|r| r.created_at);
            out.reverse();
            out.truncate(limit as usize);
            Ok(out)
        }

        async fn get_for_player(
            &self,
            report_id: Uuid,
            player_id: Uuid,
        ) -> Result<Option<ReportModel>, parabellum_types::errors::ApplicationError> {
            let reads = self.reads.lock().await;
            if !reads.contains_key(&(report_id, player_id)) {
                return Ok(None);
            }
            let rows = self.rows.lock().await;
            Ok(rows.get(&report_id).cloned())
        }

        async fn mark_as_read(
            &self,
            report_id: Uuid,
            player_id: Uuid,
        ) -> Result<(), parabellum_types::errors::ApplicationError> {
            let mut reads = self.reads.lock().await;
            reads.insert((report_id, player_id), Some(Utc::now()));
            Ok(())
        }
    }

    fn sample_report(id: Uuid, player_id: Uuid) -> ReportModel {
        ReportModel {
            id,
            report_type: "reinforcement".to_string(),
            payload: ReportPayload::Reinforcement(ReinforcementReportPayload {
                sender_player: "a".to_string(),
                sender_village: "A".to_string(),
                sender_position: Position { x: 0, y: 0 },
                receiver_player: "b".to_string(),
                receiver_village: "B".to_string(),
                receiver_position: Position { x: 1, y: 1 },
                tribe: Tribe::Roman,
                units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            }),
            actor_player_id: player_id,
            actor_village_id: Some(100),
            target_player_id: Some(player_id),
            target_village_id: Some(101),
            created_at: Utc::now(),
            read_at: None,
        }
    }

    #[tokio::test]
    async fn list_reports_for_player_query_reads_repository() {
        let repo = Arc::new(MockReportRepo::default());
        let player_id = Uuid::new_v4();
        let report_id = Uuid::new_v4();
        repo.rows
            .lock()
            .await
            .insert(report_id, sample_report(report_id, player_id));
        repo.reads.lock().await.insert((report_id, player_id), None);

        let query = ListReportsForPlayer {
            repository: repo,
            player_id,
            limit: 10,
        };
        let out = query.apply().await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, report_id);
    }

    #[tokio::test]
    async fn get_report_for_player_query_reads_repository() {
        let repo = Arc::new(MockReportRepo::default());
        let player_id = Uuid::new_v4();
        let report_id = Uuid::new_v4();
        repo.rows
            .lock()
            .await
            .insert(report_id, sample_report(report_id, player_id));
        repo.reads.lock().await.insert((report_id, player_id), None);

        let query = GetReportForPlayer {
            repository: repo,
            report_id,
            player_id,
        };
        let out = query.apply().await.unwrap();
        assert!(out.is_some());
        assert_eq!(out.unwrap().id, report_id);
    }
}
