mod academy;
mod army_training;
mod rally_point;
mod smithy;

pub use academy::research_unit;
pub use army_training::train_units;
pub use rally_point::{confirm_send_troops, recall_troops, release_reinforcements, send_troops};
pub use smithy::research_smithy;
