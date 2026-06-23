//! Report use cases.
//!
//! Reports are projected read models, but marking a report as read is still a
//! game event. This service keeps report reads and report commands explicit
//! without routing them through the broad query port.

use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::villages::{
    MarkReportRead,
    models::ReportModel,
    ports::{Clock, ReportCommandExecutor, ReportCommandIntent, ReportReadPort},
    requests::reports::{
        CountUnreadReportsForPlayerRequest, GetReportForPlayerRequest, ListReportsForPlayerRequest,
        MarkReportReadRequest,
    },
};

/// Application service for report reads and report state changes.
#[derive(Clone)]
pub struct ReportUseCases {
    reads: Arc<dyn ReportReadPort>,
    executor: Arc<dyn ReportCommandExecutor>,
    clock: Arc<dyn Clock>,
}

impl ReportUseCases {
    /// Creates report use cases from focused ports.
    pub fn new(
        reads: Arc<dyn ReportReadPort>,
        executor: Arc<dyn ReportCommandExecutor>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            reads,
            executor,
            clock,
        }
    }

    /// Lists reports visible to one player.
    pub async fn list_reports_for_player(
        &self,
        request: ListReportsForPlayerRequest,
    ) -> Result<Vec<ReportModel>, ApplicationError> {
        self.reads
            .list_reports_for_player(request.player_id, request.offset, request.limit)
            .await
    }

    /// Loads one report visible to one player.
    pub async fn get_report_for_player(
        &self,
        request: GetReportForPlayerRequest,
    ) -> Result<Option<ReportModel>, ApplicationError> {
        self.reads
            .get_report_for_player(request.report_id, request.player_id)
            .await
    }

    /// Counts unread reports for one player.
    pub async fn count_unread_reports_for_player(
        &self,
        request: CountUnreadReportsForPlayerRequest,
    ) -> Result<i64, ApplicationError> {
        self.reads
            .count_unread_reports_for_player(request.player_id)
            .await
    }

    /// Marks one report as read.
    pub async fn mark_report_as_read(
        &self,
        request: MarkReportReadRequest,
    ) -> Result<(), ApplicationError> {
        let report = self
            .reads
            .get_report_for_player(request.report_id, request.player_id)
            .await?
            .ok_or_else(|| ApplicationError::Unknown("report not found for player".to_string()))?;
        let village_id = report
            .actor_village_id
            .or(report.target_village_id)
            .ok_or_else(|| {
                ApplicationError::Unknown("report has no village stream anchor".to_string())
            })?;

        self.executor
            .execute_report_command(ReportCommandIntent::MarkReportRead {
                village_id,
                command: MarkReportRead {
                    report_id: request.report_id,
                    player_id: request.player_id,
                    read_at: self.clock.now(),
                },
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, VecDeque},
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use parabellum_types::errors::ApplicationError;
    use uuid::Uuid;

    use crate::villages::{
        models::ReportModel,
        ports::{Clock, ReportCommandExecutor, ReportCommandIntent, ReportReadPort},
        requests::reports::{
            CountUnreadReportsForPlayerRequest, GetReportForPlayerRequest,
            ListReportsForPlayerRequest, MarkReportReadRequest,
        },
    };
    use parabellum_types::{
        common::ResourceGroup,
        map::Position,
        reports::{MarketplaceDeliveryReportPayload, ReportPayload},
    };

    use super::ReportUseCases;

    #[derive(Clone)]
    struct FixedClock(chrono::DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            self.0
        }
    }

    #[derive(Default)]
    struct FakeReportReads {
        reports: Mutex<HashMap<(Uuid, Uuid), ReportModel>>,
        listed: Mutex<Vec<ReportModel>>,
        unread_count: Mutex<i64>,
    }

    #[async_trait]
    impl ReportReadPort for FakeReportReads {
        async fn list_reports_for_player(
            &self,
            _player_id: Uuid,
            _offset: i64,
            _limit: i64,
        ) -> Result<Vec<ReportModel>, ApplicationError> {
            Ok(self
                .listed
                .lock()
                .expect("listed reports lock should not be poisoned")
                .clone())
        }

        async fn get_report_for_player(
            &self,
            report_id: Uuid,
            player_id: Uuid,
        ) -> Result<Option<ReportModel>, ApplicationError> {
            Ok(self
                .reports
                .lock()
                .expect("reports lock should not be poisoned")
                .get(&(report_id, player_id))
                .cloned())
        }

        async fn count_unread_reports_for_player(
            &self,
            _player_id: Uuid,
        ) -> Result<i64, ApplicationError> {
            Ok(*self
                .unread_count
                .lock()
                .expect("unread count lock should not be poisoned"))
        }
    }

    #[derive(Default)]
    struct FakeReportExecutor {
        commands: Mutex<VecDeque<ReportCommandIntent>>,
    }

    #[async_trait]
    impl ReportCommandExecutor for FakeReportExecutor {
        async fn execute_report_command(
            &self,
            command: ReportCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("commands lock should not be poisoned")
                .push_back(command);
            Ok(())
        }
    }

    fn report(report_id: Uuid, player_id: Uuid, village_id: u32) -> ReportModel {
        ReportModel {
            id: report_id,
            report_type: "marketplace_delivery".to_string(),
            actor_player_id: player_id,
            actor_village_id: Some(village_id),
            target_player_id: None,
            target_village_id: None,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            read_at: None,
            payload: ReportPayload::MarketplaceDelivery(MarketplaceDeliveryReportPayload {
                sender_player: "sender".to_string(),
                sender_village: "sender village".to_string(),
                sender_position: Position { x: 0, y: 0 },
                receiver_player: "receiver".to_string(),
                receiver_village: "receiver village".to_string(),
                receiver_position: Position { x: 1, y: 1 },
                resources: ResourceGroup::default(),
                merchants_used: 1,
            }),
        }
    }

    fn use_cases(reads: Arc<FakeReportReads>, executor: Arc<FakeReportExecutor>) -> ReportUseCases {
        ReportUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(Utc.timestamp_opt(1_700_000_500, 0).unwrap())),
        )
    }

    #[tokio::test]
    async fn report_reads_delegate_to_read_port() {
        let player_id = Uuid::from_u128(1);
        let report_id = Uuid::from_u128(2);
        let reads = Arc::new(FakeReportReads::default());
        reads
            .listed
            .lock()
            .expect("listed reports lock should not be poisoned")
            .push(report(report_id, player_id, 10));
        *reads
            .unread_count
            .lock()
            .expect("unread count lock should not be poisoned") = 7;
        reads
            .reports
            .lock()
            .expect("reports lock should not be poisoned")
            .insert((report_id, player_id), report(report_id, player_id, 10));
        let executor = Arc::new(FakeReportExecutor::default());
        let use_cases = use_cases(reads, executor);

        assert_eq!(
            use_cases
                .list_reports_for_player(ListReportsForPlayerRequest {
                    player_id,
                    offset: 0,
                    limit: 10,
                })
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            use_cases
                .get_report_for_player(GetReportForPlayerRequest {
                    report_id,
                    player_id,
                })
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            use_cases
                .count_unread_reports_for_player(CountUnreadReportsForPlayerRequest { player_id })
                .await
                .unwrap(),
            7
        );
    }

    #[tokio::test]
    async fn mark_report_as_read_uses_report_stream_anchor_and_clock() {
        let player_id = Uuid::from_u128(1);
        let report_id = Uuid::from_u128(2);
        let reads = Arc::new(FakeReportReads::default());
        reads
            .reports
            .lock()
            .expect("reports lock should not be poisoned")
            .insert((report_id, player_id), report(report_id, player_id, 44));
        let executor = Arc::new(FakeReportExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .mark_report_as_read(MarkReportReadRequest {
                report_id,
                player_id,
            })
            .await
            .unwrap();

        let command = executor
            .commands
            .lock()
            .expect("commands lock should not be poisoned")
            .pop_front()
            .expect("command should be executed");
        let ReportCommandIntent::MarkReportRead {
            village_id,
            command,
        } = command;
        assert_eq!(village_id, 44);
        assert_eq!(command.report_id, report_id);
        assert_eq!(command.player_id, player_id);
        assert_eq!(
            command.read_at,
            Utc.timestamp_opt(1_700_000_500, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn mark_report_as_read_rejects_missing_report_without_executing() {
        let player_id = Uuid::from_u128(1);
        let report_id = Uuid::from_u128(2);
        let reads = Arc::new(FakeReportReads::default());
        let executor = Arc::new(FakeReportExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .mark_report_as_read(MarkReportReadRequest {
                report_id,
                player_id,
            })
            .await;

        assert!(result.is_err());
        assert!(
            executor
                .commands
                .lock()
                .expect("commands lock should not be poisoned")
                .is_empty()
        );
    }
}
