mod academy_handler;
mod army_handler;
mod auth_handler;
mod building_handler;
mod helpers;
mod home_handler;
mod map_handler;
mod village_handler;

pub use academy_handler::research_unit;
pub use army_handler::train_units;
pub use auth_handler::{RegisterForm, login, login_page, logout, register, register_page};
pub use building_handler::{build_action, building};
pub(crate) use helpers::*;
pub use home_handler::home;
pub use map_handler::{map, map_region};
pub use village_handler::{resources, village};
