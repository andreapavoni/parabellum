use crate::{
    cqrs::{
        QueryHandler,
        queries::{GetReportsForPlayer, ReportView},
    },
    uow::UnitOfWork,
};
use parabellum_types::{Result, errors::ApplicationError};

pub struct GetReportsForPlayerHandler;

impl GetReportsForPlayerHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetReportsForPlayer> for GetReportsForPlayerHandler {
    async fn handle(
        &self,
        query: GetReportsForPlayer,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &std::sync::Arc<crate::config::Config>,
    ) -> Result<Vec<ReportView>, ApplicationError> {
        let repo = uow.reports();
        let records = repo.list_for_player(query.player_id, query.limit).await?;

        Ok(records
            .into_iter()
            .map(|record| ReportView {
                id: record.id,
                report_type: record.report_type,
                payload: record.payload,
                created_at: record.created_at,
                read_at: record.read_at,
            })
            .collect())
    }
}
