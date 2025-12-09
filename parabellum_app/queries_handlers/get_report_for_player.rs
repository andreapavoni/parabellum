use crate::{
    cqrs::{
        QueryHandler,
        queries::{GetReportForPlayer, ReportView},
    },
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetReportForPlayerHandler;

impl GetReportForPlayerHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetReportForPlayer> for GetReportForPlayerHandler {
    async fn handle(
        &self,
        query: GetReportForPlayer,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<crate::config::Config>,
    ) -> Result<Option<ReportView>, ApplicationError> {
        let repo = uow.reports();
        let record = repo
            .get_for_player(query.report_id, query.player_id)
            .await?;

        Ok(record.map(|record| ReportView {
            id: record.id,
            report_type: record.report_type,
            payload: record.payload,
            created_at: record.created_at,
            read_at: record.read_at,
        }))
    }
}
