mod academy;
mod army;
mod auth;
mod smithy;

pub use academy::research_unit;
pub use army::{send_troops, train_units};
pub use auth::{login, logout, register};
pub use smithy::research_smithy;
