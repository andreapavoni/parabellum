use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{
        QueryHandler,
        queries::{CulturePointsInfo, GetCulturePointsInfo},
    },
    repository::PlayerRepository,
    uow::UnitOfWork,
};

pub struct GetCulturePointsInfoQueryHandler;

impl GetCulturePointsInfoQueryHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl QueryHandler<GetCulturePointsInfo> for GetCulturePointsInfoQueryHandler {
    async fn handle(
        &self,
        query: GetCulturePointsInfo,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<CulturePointsInfo, ApplicationError> {
        let player_repo: Arc<dyn PlayerRepository + '_> = uow.players();

        let player = player_repo.get_by_id(query.player_id).await?;
        let account_cpp = player_repo
            .get_total_culture_points_production(query.player_id)
            .await?;

        Ok(CulturePointsInfo {
            account_culture_points: player.culture_points,
            account_culture_points_production: account_cpp,
        })
    }
}
