use crate::read_models::PlayerPopulationLeaderboardEntry;

/// One page of the player population leaderboard.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPopulationLeaderboardPage {
    /// Entries ordered by descending total population.
    pub entries: Vec<PlayerPopulationLeaderboardEntry>,
    /// Total number of players available for pagination.
    pub total_players: i64,
}
