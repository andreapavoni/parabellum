mod academy;
mod army_training;
mod marketplace;
mod rally_point;
mod smithy;

pub use academy::research_unit;
pub use army_training::train_units;
pub use marketplace::{accept_offer, cancel_offer, create_offer, send_resources};
pub use rally_point::{
    confirm_send_troops, recall_confirmation_page, recall_troops, release_confirmation_page,
    release_reinforcements, send_troops,
};
pub use smithy::research_smithy;
