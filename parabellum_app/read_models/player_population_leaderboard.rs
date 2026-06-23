use parabellum_types::tribe::Tribe;
use uuid::Uuid;

/// One player row in the population leaderboard.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPopulationLeaderboardEntry {
    /// Player id.
    pub player_id: Uuid,
    /// Player display name.
    pub username: String,
    /// Number of villages owned by the player.
    pub village_count: i64,
    /// Total population across the player's villages.
    pub population: i64,
    /// Player tribe.
    pub tribe: Tribe,
}
