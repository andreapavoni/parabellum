use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{
        QueryHandler,
        queries::{CulturePointsInfo, GetCulturePointsInfo},
    },
    repository::{PlayerRepository, VillageRepository},
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
        let village_repo: Arc<dyn VillageRepository + '_> = uow.villages();

        // Compute a live account CP snapshot from villages, so elapsed-time accumulation
        // is reflected even when player.culture_points hasn't been persisted yet.
        let villages = village_repo.list_by_player_id(query.player_id).await?;
        let account_culture_points = villages
            .iter()
            .map(|village| village.culture_points)
            .sum::<u32>();
        let account_cpp = player_repo
            .get_total_culture_points_production(query.player_id)
            .await?;

        Ok(CulturePointsInfo {
            account_culture_points,
            account_culture_points_production: account_cpp,
        })
    }
}
