//! Ports required by village application use cases.
//!
//! These traits describe app-layer capabilities. Infrastructure implements
//! them with CQRS/ES, read-model repositories, clocks, and id generators.

pub mod activity_reads;
pub mod building_executor;
pub mod building_reads;
pub mod clock;
pub mod command_executor;
pub mod development_executor;
pub mod development_reads;
pub mod expansion_reads;
pub mod hero_executor;
pub mod hero_reads;
pub mod ids;
pub mod marketplace_executor;
pub mod marketplace_reads;
pub mod movement_control_executor;
pub mod movement_control_reads;
pub mod movement_reads;
pub mod reinforcement_executor;
pub mod reinforcement_reads;
pub mod report_executor;
pub mod report_reads;
pub mod trap_executor;
pub mod trap_reads;
pub mod village_army_reads;
pub mod village_profile_executor;
pub mod village_reference_reads;
pub mod village_state_reads;

pub use activity_reads::VillageActivityReadPort;
pub use building_executor::{BuildingCommandExecutor, BuildingCommandIntent};
pub use building_reads::{BuildingReadPort, CancelBuildingConstructionContext};
pub use clock::{Clock, SystemClock};
pub use command_executor::{VillageCommandExecutor, VillageCommandIntent};
pub use development_executor::{DevelopmentCommandExecutor, DevelopmentCommandIntent};
pub use development_reads::DevelopmentReadPort;
pub use expansion_reads::ExpansionReadPort;
pub use hero_executor::{HeroCommandExecutor, HeroCommandIntent};
pub use hero_reads::HeroReadPort;
pub use ids::{IdGenerator, UuidGenerator};
pub use marketplace_executor::{MarketplaceCommandExecutor, MarketplaceCommandIntent};
pub use marketplace_reads::MarketplaceReadPort;
pub use movement_control_executor::{MovementControlCommandExecutor, MovementControlCommandIntent};
pub use movement_control_reads::{CancelTroopMovementContext, MovementControlReadPort};
pub use movement_reads::MovementReadPort;
pub use reinforcement_executor::{ReinforcementCommandExecutor, ReinforcementCommandIntent};
pub use reinforcement_reads::{
    ReinforcementArmyContext, ReinforcementReadPort, TrappedArmyContext,
};
pub use report_executor::{ReportCommandExecutor, ReportCommandIntent};
pub use report_reads::ReportReadPort;
pub use trap_executor::{TrapCommandExecutor, TrapCommandIntent};
pub use trap_reads::TrapReadPort;
pub use village_army_reads::VillageArmyReadPort;
pub use village_profile_executor::{VillageProfileCommandExecutor, VillageProfileCommandIntent};
pub use village_reference_reads::VillageReferenceReadPort;
pub use village_state_reads::VillageStateReadPort;
