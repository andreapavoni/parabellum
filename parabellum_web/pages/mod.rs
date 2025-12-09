pub mod buildings;
mod home;
mod login;
mod map;
mod register;
mod reports;
mod resources;
mod village;

pub use home::HomePage;
pub use login::LoginPage;
pub use map::MapPage;
pub use register::RegisterPage;
pub use reports::{BattleReportPage, GenericReportPage, ReportsPage};
pub use resources::ResourcesPage;
pub use village::VillagePage;
