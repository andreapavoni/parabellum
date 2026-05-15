use parabellum_types::tribe::Tribe;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLeaderboardEntry {
    pub player_id: Uuid,
    pub username: String,
    pub village_count: i64,
    pub population: i64,
    pub tribe: Tribe,
}
