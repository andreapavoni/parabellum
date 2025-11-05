// RegisterPlayer
// Attack
// Raid
// Reinforce
// ReturnArmy
// SendMerchant
// ReturnMerchant
// TrainBarracksUnit
// TrainStableUnit
// TrainWorkshopUnit
// TrainExpansionUnit
// TrainTrapperUnit
// TrainGreatBarracksUnit
// TrainGreatStableUnit
// TrainGreatWorkshopUnit
// ResearchAcademy
// ResearchSmithy
// StartTownHallCelebration
// StartBreweryCelebration

pub mod app_bus;
pub mod command_handlers;
pub mod config;
pub mod cqrs;
pub mod job_handlers;
pub mod job_registry;
pub mod jobs;
pub mod queries_handlers;
pub mod repository;
pub mod uow;

#[cfg(test)]
pub mod test_utils;
