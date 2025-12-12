use async_trait::async_trait;
use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    config::Config,
    cqrs::{
        Query, QueryHandler,
        queries::{GetLeaderboard, Leaderboard},
    },
    uow::UnitOfWork,
};

pub struct GetLeaderboardHandler {}

impl GetLeaderboardHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryHandler<GetLeaderboard> for GetLeaderboardHandler {
    async fn handle(
        &self,
        query: GetLeaderboard,
        uow: &Box<dyn UnitOfWork<'_> + '_>,
        _config: &Arc<Config>,
    ) -> Result<<GetLeaderboard as Query>::Output, ApplicationError> {
        let repo = uow.players();

        // Clamp to sensible defaults to avoid invalid offsets.
        let page = if query.page < 1 { 1 } else { query.page };
        let per_page = if query.per_page < 1 {
            1
        } else {
            query.per_page
        };
        let offset = (page - 1) * per_page;

        let (entries, total_players) = repo.leaderboard_page(offset, per_page).await?;

        Ok(Leaderboard {
            entries,
            total_players,
            page,
            per_page,
        })
    }
}
