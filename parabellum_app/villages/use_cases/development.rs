//! Village development use cases.
//!
//! This service owns app-level orchestration for unit training and research.
//! Aggregate commands keep queue, resource, building, and research rules; this
//! layer applies server settings and read-side expansion-unit validation before
//! command execution.

use std::sync::Arc;

use parabellum_types::{
    army::UnitRole,
    errors::{ApplicationError, GameError},
};

use crate::villages::{
    ExpansionSlotUsage, ResearchAcademy, ResearchSmithy, TrainUnits, VillageArmyContext,
    hydrate_village,
    ports::{DevelopmentCommandExecutor, DevelopmentCommandIntent, DevelopmentReadPort},
    requests::development::{ResearchAcademyRequest, ResearchSmithyRequest, TrainUnitsRequest},
};

/// Runtime settings used by village development use cases.
#[derive(Debug, Clone, Copy)]
pub struct DevelopmentSettings {
    /// Server speed multiplier used by training and research commands.
    pub server_speed: i8,
}

/// Application service for village development operations.
#[derive(Clone)]
pub struct DevelopmentUseCases {
    reads: Arc<dyn DevelopmentReadPort>,
    executor: Arc<dyn DevelopmentCommandExecutor>,
    settings: DevelopmentSettings,
}

impl DevelopmentUseCases {
    /// Creates development use cases from focused ports and settings.
    pub fn new(
        reads: Arc<dyn DevelopmentReadPort>,
        executor: Arc<dyn DevelopmentCommandExecutor>,
        settings: DevelopmentSettings,
    ) -> Self {
        Self {
            reads,
            executor,
            settings,
        }
    }

    /// Queues unit training in a valid village training building.
    pub async fn train_units(&self, request: TrainUnitsRequest) -> Result<(), ApplicationError> {
        let source = self
            .reads
            .get_development_village(request.village_id)
            .await?;
        let unit = source
            .tribe
            .units()
            .get(request.unit_idx as usize)
            .ok_or(GameError::InvalidUnitIndex(request.unit_idx))
            .map_err(ApplicationError::from)?;
        let unit_role = unit.role;

        if matches!(unit_role, UnitRole::Chief | UnitRole::Settler) {
            let source_village = hydrate_village(source.clone(), VillageArmyContext::default());
            let child_villages = self
                .reads
                .count_development_child_villages(request.player_id, request.village_id)
                .await?;
            let queues = self
                .reads
                .get_development_village_queues(request.village_id)
                .await?;
            let movements = self
                .reads
                .get_development_troop_movements(request.village_id)
                .await?;

            ExpansionSlotUsage::from_village_context(
                &source_village,
                child_villages,
                &queues.training,
                &movements,
                request.player_id,
            )
            .validate_training(unit_role, request.quantity)
            .map_err(ApplicationError::Game)?;
        }

        self.executor
            .execute_development_command(DevelopmentCommandIntent::TrainUnits {
                village_id: request.village_id,
                command: TrainUnits {
                    player_id: request.player_id,
                    unit_idx: request.unit_idx,
                    building_name: request.building_name,
                    quantity: request.quantity,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }

    /// Queues academy research for a unit.
    pub async fn research_academy(
        &self,
        request: ResearchAcademyRequest,
    ) -> Result<(), ApplicationError> {
        self.executor
            .execute_development_command(DevelopmentCommandIntent::ResearchAcademy {
                village_id: request.village_id,
                command: ResearchAcademy {
                    player_id: request.player_id,
                    unit: request.unit,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }

    /// Queues smithy research for a unit.
    pub async fn research_smithy(
        &self,
        request: ResearchSmithyRequest,
    ) -> Result<(), ApplicationError> {
        self.executor
            .execute_development_command(DevelopmentCommandIntent::ResearchSmithy {
                village_id: request.village_id,
                command: ResearchSmithy {
                    player_id: request.player_id,
                    unit: request.unit,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, VecDeque},
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use parabellum_game::models::{
        trapper::TrapperState,
        village::{
            AcademyResearch, ProductionBonus, VillageEffectiveProduction, VillageProduction,
            VillageStocks,
        },
    };
    use parabellum_types::{
        army::UnitName,
        buildings::BuildingName,
        errors::{ApplicationError, GameError},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::{
        villages::read_models::{VillageQueues, VillageTroopMovements},
        villages::{
            models::VillageModel,
            ports::{DevelopmentCommandExecutor, DevelopmentCommandIntent, DevelopmentReadPort},
            requests::development::{
                ResearchAcademyRequest, ResearchSmithyRequest, TrainUnitsRequest,
            },
        },
    };

    use super::{DevelopmentSettings, DevelopmentUseCases};

    #[derive(Default)]
    struct FakeDevelopmentReads {
        villages: Mutex<HashMap<u32, VillageModel>>,
        child_villages: Mutex<HashMap<(Uuid, u32), u8>>,
        queues: Mutex<HashMap<u32, VillageQueues>>,
        movements: Mutex<HashMap<u32, VillageTroopMovements>>,
    }

    #[async_trait]
    impl DevelopmentReadPort for FakeDevelopmentReads {
        async fn get_development_village(
            &self,
            village_id: u32,
        ) -> Result<VillageModel, ApplicationError> {
            self.villages
                .lock()
                .expect("village lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing village {village_id}")))
        }

        async fn count_development_child_villages(
            &self,
            player_id: Uuid,
            village_id: u32,
        ) -> Result<u8, ApplicationError> {
            Ok(*self
                .child_villages
                .lock()
                .expect("children lock should not be poisoned")
                .get(&(player_id, village_id))
                .unwrap_or(&0))
        }

        async fn get_development_village_queues(
            &self,
            village_id: u32,
        ) -> Result<VillageQueues, ApplicationError> {
            Ok(self
                .queues
                .lock()
                .expect("queues lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn get_development_troop_movements(
            &self,
            village_id: u32,
        ) -> Result<VillageTroopMovements, ApplicationError> {
            Ok(self
                .movements
                .lock()
                .expect("movements lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .unwrap_or_default())
        }
    }

    #[derive(Default)]
    struct FakeDevelopmentExecutor {
        commands: Mutex<VecDeque<DevelopmentCommandIntent>>,
    }

    #[async_trait]
    impl DevelopmentCommandExecutor for FakeDevelopmentExecutor {
        async fn execute_development_command(
            &self,
            command: DevelopmentCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push_back(command);
            Ok(())
        }
    }

    fn village(village_id: u32, player_id: Uuid, tribe: Tribe) -> VillageModel {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        VillageModel {
            village_id,
            player_id,
            village_name: format!("village-{village_id}"),
            position: Position { x: 0, y: 0 },
            tribe,
            buildings: vec![],
            production: VillageProduction {
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
                upkeep: 0,
                bonus: ProductionBonus {
                    lumber: 0,
                    clay: 0,
                    iron: 0,
                    crop: 0,
                },
                effective: VillageEffectiveProduction {
                    lumber: 0,
                    clay: 0,
                    iron: 0,
                    crop: 0,
                },
            },
            stocks: VillageStocks {
                warehouse_capacity: 800,
                granary_capacity: 800,
                lumber: 800,
                clay: 800,
                iron: 800,
                crop: 800,
            },
            population: 0,
            loyalty: 100,
            loyalty_updated_at: now,
            is_capital: false,
            culture_points_production: 0,
            smithy_upgrades: [0; 8],
            academy_research: AcademyResearch::default(),
            total_merchants: 0,
            busy_merchants: 0,
            trapper: TrapperState {
                active_traps: 0,
                broken_traps: 0,
                queued_traps: 0,
            },
            updated_at: now,
            parent_village_id: None,
        }
    }

    fn use_cases(
        reads: Arc<FakeDevelopmentReads>,
        executor: Arc<FakeDevelopmentExecutor>,
    ) -> DevelopmentUseCases {
        DevelopmentUseCases::new(reads, executor, DevelopmentSettings { server_speed: 3 })
    }

    #[tokio::test]
    async fn train_units_builds_command_with_configured_speed() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeDevelopmentReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id, Tribe::Roman));
        let executor = Arc::new(FakeDevelopmentExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .train_units(TrainUnitsRequest {
                player_id,
                village_id: 1,
                unit_idx: 0,
                building_name: BuildingName::Barracks,
                quantity: 12,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let DevelopmentCommandIntent::TrainUnits {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected train units command");
        };
        assert_eq!(village_id, 1);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.unit_idx, 0);
        assert_eq!(command.building_name, BuildingName::Barracks);
        assert_eq!(command.quantity, 12);
        assert_eq!(command.speed, 3);
    }

    #[tokio::test]
    async fn train_units_rejects_invalid_unit_index_without_executing() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeDevelopmentReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id, Tribe::Roman));
        let executor = Arc::new(FakeDevelopmentExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .train_units(TrainUnitsRequest {
                player_id,
                village_id: 1,
                unit_idx: 99,
                building_name: BuildingName::Barracks,
                quantity: 12,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::InvalidUnitIndex(99)))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn research_academy_builds_command_with_configured_speed() {
        let player_id = Uuid::new_v4();
        let executor = Arc::new(FakeDevelopmentExecutor::default());
        let use_cases = use_cases(Arc::new(FakeDevelopmentReads::default()), executor.clone());

        use_cases
            .research_academy(ResearchAcademyRequest {
                player_id,
                village_id: 2,
                unit: UnitName::EquitesImperatoris,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let DevelopmentCommandIntent::ResearchAcademy {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected academy research command");
        };
        assert_eq!(village_id, 2);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.unit, UnitName::EquitesImperatoris);
        assert_eq!(command.speed, 3);
    }

    #[tokio::test]
    async fn research_smithy_builds_command_with_configured_speed() {
        let player_id = Uuid::new_v4();
        let executor = Arc::new(FakeDevelopmentExecutor::default());
        let use_cases = use_cases(Arc::new(FakeDevelopmentReads::default()), executor.clone());

        use_cases
            .research_smithy(ResearchSmithyRequest {
                player_id,
                village_id: 3,
                unit: UnitName::Legionnaire,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let DevelopmentCommandIntent::ResearchSmithy {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected smithy research command");
        };
        assert_eq!(village_id, 3);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.unit, UnitName::Legionnaire);
        assert_eq!(command.speed, 3);
    }
}
