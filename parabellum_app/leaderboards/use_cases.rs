use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::leaderboards::{
    models::PlayerPopulationLeaderboardPage, ports::LeaderboardReadPort,
    requests::GetPlayerPopulationLeaderboardPageRequest,
};

/// Application service for leaderboard reads.
#[derive(Clone)]
pub struct LeaderboardUseCases {
    reads: Arc<dyn LeaderboardReadPort>,
}

impl LeaderboardUseCases {
    /// Creates leaderboard use cases from the leaderboard read port.
    pub fn new(reads: Arc<dyn LeaderboardReadPort>) -> Self {
        Self { reads }
    }

    /// Loads one player population leaderboard page.
    pub async fn get_player_population_page(
        &self,
        request: GetPlayerPopulationLeaderboardPageRequest,
    ) -> Result<PlayerPopulationLeaderboardPage, ApplicationError> {
        let page = request.page.max(1);
        let per_page = request.per_page.max(1);
        let offset = (page - 1) * per_page;
        let (entries, total_players) = self
            .reads
            .list_player_population_entries(offset, per_page)
            .await?;

        Ok(PlayerPopulationLeaderboardPage {
            entries,
            total_players,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use parabellum_types::{errors::ApplicationError, tribe::Tribe};
    use uuid::Uuid;

    use crate::{
        leaderboards::{
            LeaderboardReadPort, LeaderboardUseCases,
            requests::GetPlayerPopulationLeaderboardPageRequest,
        },
        read_models::PlayerPopulationLeaderboardEntry,
    };

    #[derive(Default)]
    struct FakeLeaderboardReads {
        calls: Mutex<Vec<(i64, i64)>>,
    }

    #[async_trait]
    impl LeaderboardReadPort for FakeLeaderboardReads {
        async fn list_player_population_entries(
            &self,
            offset: i64,
            limit: i64,
        ) -> Result<(Vec<PlayerPopulationLeaderboardEntry>, i64), ApplicationError> {
            self.calls.lock().unwrap().push((offset, limit));
            Ok((
                vec![PlayerPopulationLeaderboardEntry {
                    player_id: Uuid::nil(),
                    username: "leader".to_string(),
                    village_count: 2,
                    population: 120,
                    tribe: Tribe::Roman,
                }],
                7,
            ))
        }
    }

    #[tokio::test]
    async fn player_population_leaderboard_normalizes_pagination_and_delegates_to_read_port() {
        let reads = Arc::new(FakeLeaderboardReads::default());
        let use_cases = LeaderboardUseCases::new(reads.clone());

        let page = use_cases
            .get_player_population_page(GetPlayerPopulationLeaderboardPageRequest {
                page: 3,
                per_page: 20,
            })
            .await
            .unwrap();

        assert_eq!(reads.calls.lock().unwrap().as_slice(), &[(40, 20)]);
        assert_eq!(page.total_players, 7);
        assert_eq!(page.entries[0].username, "leader");
    }

    #[tokio::test]
    async fn player_population_leaderboard_clamps_invalid_pagination() {
        let reads = Arc::new(FakeLeaderboardReads::default());
        let use_cases = LeaderboardUseCases::new(reads.clone());

        let _ = use_cases
            .get_player_population_page(GetPlayerPopulationLeaderboardPageRequest {
                page: 0,
                per_page: 0,
            })
            .await
            .unwrap();

        assert_eq!(reads.calls.lock().unwrap().as_slice(), &[(0, 1)]);
    }
}
