//! Trap-building use cases.
//!
//! This service owns app-level trap construction orchestration: it loads
//! village and army occupancy context, delegates capacity/cost planning to the
//! domain trapper model, checks resource availability, and sends command intent
//! through app ports.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parabellum_game::models::trapper::{TRAP_BUILD_TIME_SECS, Trapper};
use parabellum_types::errors::{ApplicationError, GameError};

use crate::villages::{
    BuildTraps, VillageArmyContext, hydrate_village,
    ports::{Clock, IdGenerator, TrapCommandExecutor, TrapCommandIntent, TrapReadPort},
    requests::traps::BuildTrapsRequest,
};

/// Application service for trap construction.
#[derive(Clone)]
pub struct TrapUseCases {
    reads: Arc<dyn TrapReadPort>,
    executor: Arc<dyn TrapCommandExecutor>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl TrapUseCases {
    pub fn new(
        reads: Arc<dyn TrapReadPort>,
        executor: Arc<dyn TrapCommandExecutor>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            reads,
            executor,
            clock,
            ids,
        }
    }

    pub async fn build_traps(&self, request: BuildTrapsRequest) -> Result<(), ApplicationError> {
        let village = self.reads.get_trap_village(request.village_id).await?;
        if village.player_id != request.player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: request.village_id,
                player_id: request.player_id,
            }));
        }

        let army_state = self.reads.get_trap_army_state(request.village_id).await?;
        let occupied = army_state
            .trapped_here
            .iter()
            .map(|army| army.units().immensity())
            .sum();
        let mut trapper = Trapper::from_buildings(&village.buildings, village.trapper, occupied);
        let Some(plan) = trapper.start_trap_build(request.quantity) else {
            return Err(ApplicationError::Unknown(
                "Requested trap quantity is not buildable".to_string(),
            ));
        };

        let domain_village = hydrate_village(village, VillageArmyContext::default());
        if !domain_village.has_enough_resources(&plan.cost) {
            return Err(ApplicationError::Game(GameError::NotEnoughResources));
        }

        let execute_at = self.trap_execute_at();
        self.executor
            .execute_trap_command(TrapCommandIntent::BuildTraps {
                village_id: request.village_id,
                command: BuildTraps {
                    action_id: self.ids.next(),
                    player_id: request.player_id,
                    village_id: request.village_id,
                    quantity_remaining: request.quantity as i32,
                    time_per_trap: TRAP_BUILD_TIME_SECS as i32,
                    cost: plan.cost,
                    trapper: trapper.state(),
                    execute_at,
                },
            })
            .await
    }

    fn trap_execute_at(&self) -> DateTime<Utc> {
        self.clock.now() + Duration::seconds(TRAP_BUILD_TIME_SECS as i64)
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
        army::Army,
        buildings::Building,
        trapper::{TRAP_BUILD_TIME_SECS, TrapperState},
        village::{
            AcademyResearch, ProductionBonus, VillageBuilding, VillageEffectiveProduction,
            VillageProduction, VillageStocks,
        },
    };
    use parabellum_types::{
        army::TroopSet,
        buildings::BuildingName,
        errors::{ApplicationError, GameError},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::{
        villages::read_models::VillageArmyStateView,
        villages::{
            models::VillageModel,
            ports::{Clock, IdGenerator, TrapCommandExecutor, TrapCommandIntent, TrapReadPort},
            requests::traps::BuildTrapsRequest,
        },
    };

    use super::TrapUseCases;

    #[derive(Clone)]
    struct FixedClock(chrono::DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            self.0
        }
    }

    struct FixedIds {
        ids: Mutex<VecDeque<Uuid>>,
    }

    impl FixedIds {
        fn new(ids: Vec<Uuid>) -> Self {
            Self {
                ids: Mutex::new(ids.into()),
            }
        }
    }

    impl IdGenerator for FixedIds {
        fn next(&self) -> Uuid {
            self.ids
                .lock()
                .expect("id lock should not be poisoned")
                .pop_front()
                .expect("test should provide enough ids")
        }
    }

    #[derive(Default)]
    struct FakeTrapReads {
        villages: Mutex<HashMap<u32, VillageModel>>,
        army_states: Mutex<HashMap<u32, VillageArmyStateView>>,
    }

    #[async_trait]
    impl TrapReadPort for FakeTrapReads {
        async fn get_trap_village(
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

        async fn get_trap_army_state(
            &self,
            village_id: u32,
        ) -> Result<VillageArmyStateView, ApplicationError> {
            self.army_states
                .lock()
                .expect("army state lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .ok_or_else(|| {
                    ApplicationError::Unknown(format!("missing army state {village_id}"))
                })
        }
    }

    #[derive(Default)]
    struct FakeTrapExecutor {
        commands: Mutex<Vec<TrapCommandIntent>>,
    }

    #[async_trait]
    impl TrapCommandExecutor for FakeTrapExecutor {
        async fn execute_trap_command(
            &self,
            command: TrapCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push(command);
            Ok(())
        }
    }

    fn village(
        village_id: u32,
        player_id: Uuid,
        trapper_level: u8,
        stocks: VillageStocks,
    ) -> VillageModel {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let updated_at = Utc::now();
        let buildings = if trapper_level > 0 {
            vec![VillageBuilding {
                slot_id: 20,
                building: Building::new(BuildingName::Trapper, 1)
                    .at_level(trapper_level, 1)
                    .unwrap(),
            }]
        } else {
            vec![]
        };

        VillageModel {
            village_id,
            player_id,
            village_name: format!("village-{village_id}"),
            position: Position { x: 0, y: 0 },
            tribe: Tribe::Gaul,
            buildings,
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
            stocks,
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
            updated_at,
            parent_village_id: None,
        }
    }

    fn stocks(lumber: u32, clay: u32, iron: u32, crop: u32) -> VillageStocks {
        VillageStocks {
            warehouse_capacity: 800,
            granary_capacity: 800,
            lumber,
            clay,
            iron,
            crop: crop.into(),
        }
    }

    fn army_state(trapped_here: Vec<Army>) -> VillageArmyStateView {
        VillageArmyStateView {
            home_army: None,
            reinforcements: vec![],
            deployed_armies: vec![],
            trapped_here,
            trapped_away: vec![],
        }
    }

    fn occupying_army(village_id: u32, trapped_count: u32) -> Army {
        Army::new(
            Some(Uuid::new_v4()),
            village_id,
            Some(village_id),
            Uuid::new_v4(),
            Tribe::Roman,
            &TroopSet::new([trapped_count, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            &[0; 8],
            None,
        )
    }

    fn use_cases(
        reads: Arc<FakeTrapReads>,
        executor: Arc<FakeTrapExecutor>,
        ids: Vec<Uuid>,
    ) -> TrapUseCases {
        TrapUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            )),
            Arc::new(FixedIds::new(ids)),
        )
    }

    #[tokio::test]
    async fn build_traps_rejects_non_owner_without_executing() {
        let owner_id = Uuid::new_v4();
        let reads = Arc::new(FakeTrapReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, owner_id, 1, stocks(800, 800, 800, 800)));
        reads
            .army_states
            .lock()
            .unwrap()
            .insert(1, army_state(vec![]));
        let executor = Arc::new(FakeTrapExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![Uuid::new_v4()]);

        let result = use_cases
            .build_traps(BuildTrapsRequest {
                player_id: Uuid::new_v4(),
                village_id: 1,
                quantity: 1,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: 1,
                ..
            }))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn build_traps_rejects_unbuildable_quantity_without_executing() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeTrapReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id, 1, stocks(800, 800, 800, 800)));
        reads
            .army_states
            .lock()
            .unwrap()
            .insert(1, army_state(vec![occupying_army(1, 10)]));
        let executor = Arc::new(FakeTrapExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![Uuid::new_v4()]);

        let result = use_cases
            .build_traps(BuildTrapsRequest {
                player_id,
                village_id: 1,
                quantity: 1,
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::Unknown(_))));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn build_traps_rejects_insufficient_resources_without_executing() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeTrapReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id, 1, stocks(0, 0, 0, 0)));
        reads
            .army_states
            .lock()
            .unwrap()
            .insert(1, army_state(vec![]));
        let executor = Arc::new(FakeTrapExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![Uuid::new_v4()]);

        let result = use_cases
            .build_traps(BuildTrapsRequest {
                player_id,
                village_id: 1,
                quantity: 1,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::NotEnoughResources))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn build_traps_builds_command_with_deterministic_id_and_time() {
        let player_id = Uuid::new_v4();
        let action_id = Uuid::new_v4();
        let reads = Arc::new(FakeTrapReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id, 1, stocks(800, 800, 800, 800)));
        reads
            .army_states
            .lock()
            .unwrap()
            .insert(1, army_state(vec![]));
        let executor = Arc::new(FakeTrapExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![action_id]);

        use_cases
            .build_traps(BuildTrapsRequest {
                player_id,
                village_id: 1,
                quantity: 2,
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let TrapCommandIntent::BuildTraps {
            village_id,
            command,
        } = commands.first().expect("command should be executed");
        assert_eq!(*village_id, 1);
        assert_eq!(command.action_id, action_id);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.village_id, 1);
        assert_eq!(command.quantity_remaining, 2);
        assert_eq!(command.time_per_trap, TRAP_BUILD_TIME_SECS as i32);
        assert_eq!(
            command.execute_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()
                + chrono::Duration::seconds(TRAP_BUILD_TIME_SECS as i64)
        );
    }
}
