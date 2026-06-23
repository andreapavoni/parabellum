/// Request one page from the player population leaderboard.
#[derive(Debug, Clone, Copy)]
pub struct GetPlayerPopulationLeaderboardPageRequest {
    /// Requested 1-based page.
    pub page: i64,
    /// Maximum rows per page.
    pub per_page: i64,
}
