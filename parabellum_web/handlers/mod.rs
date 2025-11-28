mod building_handler;
mod helpers;
mod home_handler;
mod login_handler;
mod logout_handler;
mod map_handler;
mod register_handler;
mod village_handler;

pub(crate) use helpers::*;
pub use building_handler::building;
pub use home_handler::home;
pub use login_handler::{login, login_page};
pub use logout_handler::logout;
pub use map_handler::{map, map_region};
pub use register_handler::{RegisterForm, register, register_page};
pub use village_handler::{resources, village};
