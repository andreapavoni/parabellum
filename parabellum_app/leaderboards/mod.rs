//! Leaderboard application use cases.
//!
//! Leaderboards are root application reads. They may aggregate player, village,
//! army, or alliance metrics, so they must not live under a single gameplay
//! aggregate module such as `villages`.

pub mod models;
pub mod ports;
pub mod requests;
pub mod use_cases;

pub use models::PlayerPopulationLeaderboardPage;
pub use ports::LeaderboardReadPort;
pub use requests::GetPlayerPopulationLeaderboardPageRequest;
pub use use_cases::LeaderboardUseCases;
