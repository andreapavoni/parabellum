//! Reinforcement and trapped-troop control use cases.
//!
//! This service owns app-level orchestration for returning stationed armies and
//! controlling trapped troops. It loads current read-model context, applies
//! ownership checks, delegates selected-army rules to app/domain policies, plans
//! return timing through domain map helpers, and sends command intent through
//! app ports.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parabellum_game::models::trapper::{Trapper, TrapperState};
use parabellum_types::errors::{ApplicationError, GameError};

use crate::villages::{
    DisbandTrappedTroops, RecallReinforcements, ReinforcementControl, ReleaseReinforcements,
    ReleaseTrappedTroops,
    models::VillageModel,
    ports::{
        Clock, IdGenerator, ReinforcementCommandExecutor, ReinforcementCommandIntent,
        ReinforcementReadPort, TrappedArmyContext,
    },
    requests::reinforcements::{
        DisbandTrappedTroopsRequest, RecallReinforcementsRequest, ReleaseReinforcementsRequest,
        ReleaseTrappedTroopsRequest,
    },
};

/// Runtime settings needed to plan reinforcement return travel.
#[derive(Debug, Clone, Copy)]
pub struct ReinforcementSettings {
    /// Square world size used by map distance calculations.
    pub world_size: i32,
    /// Server speed multiplier used by movement timing.
    pub server_speed: u8,
}

/// Application service for reinforcement and trapped-troop control.
#[derive(Clone)]
pub struct ReinforcementUseCases {
    reads: Arc<dyn ReinforcementReadPort>,
    executor: Arc<dyn ReinforcementCommandExecutor>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    settings: ReinforcementSettings,
}

impl ReinforcementUseCases {
    pub fn new(
        reads: Arc<dyn ReinforcementReadPort>,
        executor: Arc<dyn ReinforcementCommandExecutor>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        settings: ReinforcementSettings,
    ) -> Self {
        Self {
            reads,
            executor,
            clock,
            ids,
            settings,
        }
    }

    pub async fn recall_reinforcements(
        &self,
        request: RecallReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        let context = self
            .reads
            .get_reinforcement_context(request.army_id)
            .await?;
        if context.home_village_id != request.village_id {
            return Err(ApplicationError::Unknown(
                "Reinforcement army does not belong to provided home village".to_string(),
            ));
        }

        let stationed = self
            .reads
            .get_reinforcement_village(context.stationed_village_id)
            .await?;
        let home = self
            .reads
            .get_reinforcement_village(context.home_village_id)
            .await?;
        self.ensure_village_owner(&home, request.player_id)?;

        ReinforcementControl::returning_army(
            &context.army,
            &request.units,
            request.hero_id,
            context.stationed_village_id,
        )
        .map_err(ApplicationError::Game)?;

        let returns_at = self.return_arrival_at(&stationed, &home, context.army.speed());
        self.executor
            .execute_reinforcement_command(ReinforcementCommandIntent::RecallReinforcements {
                home_village_id: context.home_village_id,
                command: RecallReinforcements {
                    action_id: self.ids.next(),
                    movement_id: self.ids.next(),
                    player_id: request.player_id,
                    home_village_id: context.home_village_id,
                    stationed_village_id: context.stationed_village_id,
                    reinforcement_army: context.army,
                    units: request.units,
                    hero_id: request.hero_id,
                    returns_at,
                },
            })
            .await
    }

    pub async fn release_reinforcements(
        &self,
        request: ReleaseReinforcementsRequest,
    ) -> Result<(), ApplicationError> {
        let context = self
            .reads
            .get_reinforcement_context(request.army_id)
            .await?;
        if context.home_village_id != request.village_id {
            return Err(ApplicationError::Unknown(
                "Reinforcement army does not belong to provided home village".to_string(),
            ));
        }

        let stationed = self
            .reads
            .get_reinforcement_village(context.stationed_village_id)
            .await?;
        let home = self
            .reads
            .get_reinforcement_village(context.home_village_id)
            .await?;
        self.ensure_village_owner(&stationed, request.player_id)?;

        ReinforcementControl::returning_army(
            &context.army,
            &request.units,
            request.hero_id,
            context.stationed_village_id,
        )
        .map_err(ApplicationError::Game)?;

        let returns_at = self.return_arrival_at(&stationed, &home, context.army.speed());
        self.executor
            .execute_reinforcement_command(ReinforcementCommandIntent::ReleaseReinforcements {
                stationed_village_id: context.stationed_village_id,
                command: ReleaseReinforcements {
                    action_id: self.ids.next(),
                    movement_id: self.ids.next(),
                    player_id: request.player_id,
                    stationed_village_id: context.stationed_village_id,
                    home_village_id: context.home_village_id,
                    reinforcement_army: context.army,
                    units: request.units,
                    hero_id: request.hero_id,
                    returns_at,
                },
            })
            .await
    }

    pub async fn release_trapped_troops(
        &self,
        request: ReleaseTrappedTroopsRequest,
    ) -> Result<(), ApplicationError> {
        let context = self.reads.get_trapped_army_context(request.army_id).await?;
        if context.trapped_village_id != request.village_id {
            return Err(ApplicationError::Unknown(
                "Trapped army is not held in provided village".to_string(),
            ));
        }

        let trapped_village = self
            .reads
            .get_reinforcement_village(context.trapped_village_id)
            .await?;
        self.ensure_village_owner(&trapped_village, request.player_id)?;
        let home = self
            .reads
            .get_reinforcement_village(context.home_village_id)
            .await?;
        let trapper = self
            .released_trapper_state(&trapped_village, &context)
            .await?;
        let returns_at = self.return_arrival_at(&trapped_village, &home, context.army.speed());

        self.executor
            .execute_reinforcement_command(ReinforcementCommandIntent::ReleaseTrappedTroops {
                trapped_village_id: context.trapped_village_id,
                command: ReleaseTrappedTroops {
                    action_id: self.ids.next(),
                    movement_id: self.ids.next(),
                    player_id: request.player_id,
                    home_village_id: context.home_village_id,
                    trapped_village_id: context.trapped_village_id,
                    army: context.army,
                    trapper,
                    returns_at,
                },
            })
            .await
    }

    pub async fn disband_trapped_troops(
        &self,
        request: DisbandTrappedTroopsRequest,
    ) -> Result<(), ApplicationError> {
        let context = self.reads.get_trapped_army_context(request.army_id).await?;
        if context.home_village_id != request.village_id {
            return Err(ApplicationError::Unknown(
                "Trapped army does not belong to provided home village".to_string(),
            ));
        }

        let home = self
            .reads
            .get_reinforcement_village(context.home_village_id)
            .await?;
        self.ensure_village_owner(&home, request.player_id)?;
        let trapped_village = self
            .reads
            .get_reinforcement_village(context.trapped_village_id)
            .await?;
        let trapper = self
            .released_trapper_state(&trapped_village, &context)
            .await?;

        self.executor
            .execute_reinforcement_command(ReinforcementCommandIntent::DisbandTrappedTroops {
                trapped_village_id: context.trapped_village_id,
                command: DisbandTrappedTroops {
                    army_id: context.army.id,
                    player_id: request.player_id,
                    home_village_id: context.home_village_id,
                    trapped_village_id: context.trapped_village_id,
                    trapper,
                },
            })
            .await
    }

    fn ensure_village_owner(
        &self,
        village: &VillageModel,
        player_id: uuid::Uuid,
    ) -> Result<(), ApplicationError> {
        if village.player_id != player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: village.village_id,
                player_id,
            }));
        }
        Ok(())
    }

    async fn released_trapper_state(
        &self,
        trapped_village: &VillageModel,
        context: &TrappedArmyContext,
    ) -> Result<TrapperState, ApplicationError> {
        let army_state = self
            .reads
            .get_reinforcement_army_state(context.trapped_village_id)
            .await?;
        let occupied = army_state
            .trapped_here
            .iter()
            .map(|army| army.units().immensity())
            .sum();
        let mut trapper = Trapper::from_buildings(
            &trapped_village.buildings,
            trapped_village.trapper,
            occupied,
        );
        trapper.release_by_owner(context.army.units());
        Ok(trapper.state())
    }

    fn return_arrival_at(
        &self,
        source: &VillageModel,
        target: &VillageModel,
        speed: u8,
    ) -> DateTime<Utc> {
        self.clock.now() + self.travel_duration(source, target, speed)
    }

    fn travel_duration(&self, source: &VillageModel, target: &VillageModel, speed: u8) -> Duration {
        let secs = source.position.calculate_travel_time_secs(
            target.position.clone(),
            speed.max(1),
            self.settings.world_size,
            self.settings.server_speed,
        );
        Duration::seconds(std::cmp::max(1, secs) as i64)
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
        trapper::TrapperState,
        village::{
            AcademyResearch, ProductionBonus, VillageEffectiveProduction, VillageProduction,
            VillageStocks,
        },
    };
    use parabellum_types::{
        army::TroopSet,
        errors::{ApplicationError, GameError},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::{
        villages::read_models::VillageArmyStateView,
        villages::{
            models::VillageModel,
            ports::{
                Clock, IdGenerator, ReinforcementArmyContext, ReinforcementCommandExecutor,
                ReinforcementCommandIntent, ReinforcementReadPort, TrappedArmyContext,
            },
            requests::reinforcements::{
                DisbandTrappedTroopsRequest, RecallReinforcementsRequest,
                ReleaseReinforcementsRequest, ReleaseTrappedTroopsRequest,
            },
        },
    };

    use super::{ReinforcementSettings, ReinforcementUseCases};

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
    struct FakeReinforcementReads {
        villages: Mutex<HashMap<u32, VillageModel>>,
        reinforcements: Mutex<HashMap<Uuid, ReinforcementArmyContext>>,
        trapped: Mutex<HashMap<Uuid, TrappedArmyContext>>,
        army_states: Mutex<HashMap<u32, VillageArmyStateView>>,
    }

    #[async_trait]
    impl ReinforcementReadPort for FakeReinforcementReads {
        async fn get_reinforcement_context(
            &self,
            army_id: Uuid,
        ) -> Result<ReinforcementArmyContext, ApplicationError> {
            self.reinforcements
                .lock()
                .expect("reinforcement lock should not be poisoned")
                .get(&army_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing army {army_id}")))
        }

        async fn get_trapped_army_context(
            &self,
            army_id: Uuid,
        ) -> Result<TrappedArmyContext, ApplicationError> {
            self.trapped
                .lock()
                .expect("trapped lock should not be poisoned")
                .get(&army_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing trapped army {army_id}")))
        }

        async fn get_reinforcement_village(
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

        async fn get_reinforcement_army_state(
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
    struct FakeReinforcementExecutor {
        commands: Mutex<Vec<ReinforcementCommandIntent>>,
    }

    #[async_trait]
    impl ReinforcementCommandExecutor for FakeReinforcementExecutor {
        async fn execute_reinforcement_command(
            &self,
            command: ReinforcementCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push(command);
            Ok(())
        }
    }

    fn village(village_id: u32, player_id: Uuid, position: Position, tribe: Tribe) -> VillageModel {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        VillageModel {
            village_id,
            player_id,
            village_name: format!("village-{village_id}"),
            position,
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
                lumber: 0,
                clay: 0,
                iron: 0,
                crop: 0,
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

    fn army(
        army_id: Uuid,
        home_village_id: u32,
        current_map_field_id: u32,
        player_id: Uuid,
        tribe: Tribe,
        units: TroopSet,
    ) -> Army {
        Army::new(
            Some(army_id),
            home_village_id,
            Some(current_map_field_id),
            player_id,
            tribe,
            &units,
            &[0; 8],
            None,
        )
    }

    fn empty_army_state(trapped_here: Vec<Army>) -> VillageArmyStateView {
        VillageArmyStateView {
            home_army: None,
            reinforcements: vec![],
            deployed_armies: vec![],
            trapped_here,
            trapped_away: vec![],
        }
    }

    fn use_cases(
        reads: Arc<FakeReinforcementReads>,
        executor: Arc<FakeReinforcementExecutor>,
        ids: Vec<Uuid>,
    ) -> ReinforcementUseCases {
        ReinforcementUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            )),
            Arc::new(FixedIds::new(ids)),
            ReinforcementSettings {
                world_size: 100,
                server_speed: 1,
            },
        )
    }

    #[tokio::test]
    async fn recall_reinforcements_builds_return_command_with_deterministic_ids() {
        let owner_id = Uuid::new_v4();
        let stationed_owner_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        let reinforcement_army = army(
            army_id,
            1,
            2,
            owner_id,
            Tribe::Roman,
            TroopSet::new([10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        );
        let reads = Arc::new(FakeReinforcementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, owner_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        reads.villages.lock().unwrap().insert(
            2,
            village(2, stationed_owner_id, Position { x: 10, y: 0 }, Tribe::Gaul),
        );
        reads.reinforcements.lock().unwrap().insert(
            army_id,
            ReinforcementArmyContext {
                stationed_village_id: 2,
                home_village_id: 1,
                army: reinforcement_army,
            },
        );
        let executor = Arc::new(FakeReinforcementExecutor::default());
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let use_cases = use_cases(reads, executor.clone(), ids.clone());

        use_cases
            .recall_reinforcements(RecallReinforcementsRequest {
                player_id: owner_id,
                village_id: 1,
                army_id,
                units: TroopSet::new([4, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                hero_id: None,
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let ReinforcementCommandIntent::RecallReinforcements {
            home_village_id,
            command,
        } = commands.first().expect("command should be executed")
        else {
            panic!("expected recall reinforcements command");
        };
        assert_eq!(*home_village_id, 1);
        assert_eq!(command.action_id, ids[0]);
        assert_eq!(command.movement_id, ids[1]);
        assert_eq!(command.player_id, owner_id);
        assert_eq!(command.home_village_id, 1);
        assert_eq!(command.stationed_village_id, 2);
        assert!(command.returns_at > Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap());
    }

    #[tokio::test]
    async fn release_reinforcements_requires_stationed_village_owner() {
        let home_owner_id = Uuid::new_v4();
        let stationed_owner_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        let reads = Arc::new(FakeReinforcementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, home_owner_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        reads.villages.lock().unwrap().insert(
            2,
            village(2, stationed_owner_id, Position { x: 10, y: 0 }, Tribe::Gaul),
        );
        reads.reinforcements.lock().unwrap().insert(
            army_id,
            ReinforcementArmyContext {
                stationed_village_id: 2,
                home_village_id: 1,
                army: army(
                    army_id,
                    1,
                    2,
                    home_owner_id,
                    Tribe::Roman,
                    TroopSet::new([10, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                ),
            },
        );
        let executor = Arc::new(FakeReinforcementExecutor::default());
        let use_cases = use_cases(
            reads,
            executor.clone(),
            vec![Uuid::new_v4(), Uuid::new_v4()],
        );

        let result = use_cases
            .release_reinforcements(ReleaseReinforcementsRequest {
                player_id: Uuid::new_v4(),
                village_id: 1,
                army_id,
                units: TroopSet::new([4, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                hero_id: None,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: 2,
                ..
            }))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn release_trapped_troops_requires_trapping_village_owner() {
        let home_owner_id = Uuid::new_v4();
        let trapper_owner_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        let trapped_army = army(
            army_id,
            1,
            2,
            home_owner_id,
            Tribe::Roman,
            TroopSet::new([5, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        );
        let reads = Arc::new(FakeReinforcementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, home_owner_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        reads.villages.lock().unwrap().insert(
            2,
            village(2, trapper_owner_id, Position { x: 10, y: 0 }, Tribe::Gaul),
        );
        reads.trapped.lock().unwrap().insert(
            army_id,
            TrappedArmyContext {
                trapped_village_id: 2,
                home_village_id: 1,
                army: trapped_army.clone(),
            },
        );
        reads
            .army_states
            .lock()
            .unwrap()
            .insert(2, empty_army_state(vec![trapped_army]));
        let executor = Arc::new(FakeReinforcementExecutor::default());
        let use_cases = use_cases(
            reads,
            executor.clone(),
            vec![Uuid::new_v4(), Uuid::new_v4()],
        );

        let result = use_cases
            .release_trapped_troops(ReleaseTrappedTroopsRequest {
                player_id: Uuid::new_v4(),
                village_id: 2,
                army_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: 2,
                ..
            }))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn disband_trapped_troops_builds_command_for_home_owner() {
        let home_owner_id = Uuid::new_v4();
        let trapper_owner_id = Uuid::new_v4();
        let army_id = Uuid::new_v4();
        let trapped_army = army(
            army_id,
            1,
            2,
            home_owner_id,
            Tribe::Roman,
            TroopSet::new([5, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        );
        let reads = Arc::new(FakeReinforcementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, home_owner_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        reads.villages.lock().unwrap().insert(
            2,
            village(2, trapper_owner_id, Position { x: 10, y: 0 }, Tribe::Gaul),
        );
        reads.trapped.lock().unwrap().insert(
            army_id,
            TrappedArmyContext {
                trapped_village_id: 2,
                home_village_id: 1,
                army: trapped_army.clone(),
            },
        );
        reads
            .army_states
            .lock()
            .unwrap()
            .insert(2, empty_army_state(vec![trapped_army]));
        let executor = Arc::new(FakeReinforcementExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![]);

        use_cases
            .disband_trapped_troops(DisbandTrappedTroopsRequest {
                player_id: home_owner_id,
                village_id: 1,
                army_id,
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let ReinforcementCommandIntent::DisbandTrappedTroops {
            trapped_village_id,
            command,
        } = commands.first().expect("command should be executed")
        else {
            panic!("expected disband trapped troops command");
        };
        assert_eq!(*trapped_village_id, 2);
        assert_eq!(command.army_id, army_id);
        assert_eq!(command.player_id, home_owner_id);
        assert_eq!(command.home_village_id, 1);
        assert_eq!(command.trapped_village_id, 2);
    }
}
