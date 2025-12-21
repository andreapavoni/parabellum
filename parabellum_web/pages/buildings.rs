mod academy;
mod empty_slot;
mod expansion;
mod generic;
mod rally_point;
mod recall_release;
mod resource_field;
mod send_troops_confirmation;
mod smithy;
mod static_building;
mod training;

pub use academy::{AcademyPage, AcademyQueueItem, AcademyResearchOption};
pub use empty_slot::{BuildingOption, BuildingOptionCard, EmptySlotPage};
pub use expansion::ExpansionBuildingPage;
pub use generic::GenericBuildingPage;
pub use rally_point::RallyPointPage;
pub use recall_release::{RecallConfirmationPage, ReleaseConfirmationPage};
pub use resource_field::ResourceFieldPage;
pub use send_troops_confirmation::{ConfirmationType, SendTroopsConfirmationPage};
pub use smithy::{SmithyPage, SmithyQueueItem, SmithyUpgradeOption};
pub use static_building::{BuildingValueType, StaticBuildingPage};
pub use training::{
    TrainingBuildingPage, TrainingQueue, TrainingQueueItem, TrainingUnitCard, UnitTrainingOption,
};
