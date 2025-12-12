pub mod buildings;
mod home;
mod login;
mod map;
mod register;
mod reports;
mod resources;
pub mod stats;
mod village;

pub use home::HomePage;
pub use login::LoginPage;
pub use map::MapPage;
pub use register::RegisterPage;
pub use reports::{BattleReportPage, GenericReportPage, ReinforcementReportPage, ReportsPage};
pub use resources::ResourcesPage;
pub use stats::StatsPage;
pub use village::VillagePage;
