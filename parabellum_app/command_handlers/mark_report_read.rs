use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{CommandHandler, commands::MarkReportRead},
    uow::UnitOfWork,
};

pub struct MarkReportReadCommandHandler;

impl MarkReportReadCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CommandHandler<MarkReportRead> for MarkReportReadCommandHandler {
    async fn handle(
        &self,
        command: MarkReportRead,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<(), ApplicationError> {
        let repo = uow.reports();
        repo.mark_as_read(command.report_id, command.player_id)
            .await
    }
}
