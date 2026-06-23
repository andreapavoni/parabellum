//! Outbound movement dispatch use cases.
//!
//! This service owns movement orchestration in the application layer. It loads
//! read-model context through app ports, delegates movement mechanics to the
//! game/domain layer, creates deterministic command ids and timestamps, and
//! sends canonical command intent to the command executor.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parabellum_game::models::{army::Army, hero::Hero};
use parabellum_types::errors::{ApplicationError, GameError};

use crate::villages::{
    AttackVillage, ScoutVillage, SendReinforcement, SendSettlers,
    ports::{Clock, IdGenerator, MovementReadPort, VillageCommandExecutor, VillageCommandIntent},
    requests::movements::{
        SendAttackRequest, SendReinforcementRequest, SendScoutRequest, SendSettlersRequest,
    },
};

/// Runtime settings needed to plan outbound movement travel.
#[derive(Debug, Clone, Copy)]
pub struct MovementSettings {
    /// Square world size used by map distance calculations.
    pub world_size: i32,
    /// Server speed multiplier used by movement timing.
    pub server_speed: u8,
}

/// Read-model context required to dispatch a source-to-target movement.
#[derive(Debug, Clone)]
pub struct MovementDispatchContext {
    /// Source village dispatching the army.
    pub source: crate::villages::models::VillageModel,
    /// Target village receiving the movement.
    pub target: crate::villages::models::VillageModel,
    /// Optional hero selected for dispatch.
    pub hero: Option<Hero>,
}

/// Application service for outbound troop and settler movement dispatch.
#[derive(Clone)]
pub struct MovementUseCases {
    reads: Arc<dyn MovementReadPort>,
    executor: Arc<dyn VillageCommandExecutor>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    settings: MovementSettings,
}

impl MovementUseCases {
    pub fn new(
        reads: Arc<dyn MovementReadPort>,
        executor: Arc<dyn VillageCommandExecutor>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        settings: MovementSettings,
    ) -> Self {
        Self {
            reads,
            executor,
            clock,
            ids,
            settings,
        }
    }

    pub async fn send_reinforcement(
        &self,
        request: SendReinforcementRequest,
    ) -> Result<(), ApplicationError> {
        let context = self
            .load_dispatch_context(
                request.source_village_id,
                request.target_village_id,
                request.hero_id,
            )
            .await?;
        self.ensure_source_owner(&context, request.player_id)?;
        let arrives_at = self.one_way_arrival_at(&context, &request.units);

        self.executor
            .execute_village_command(
                request.source_village_id,
                VillageCommandIntent::SendReinforcement(SendReinforcement {
                    movement_id: self.ids.next(),
                    army_id: self.ids.next(),
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    units: request.units,
                    hero_id: request.hero_id,
                    arrives_at,
                }),
            )
            .await
    }

    pub async fn send_attack(&self, request: SendAttackRequest) -> Result<(), ApplicationError> {
        let context = self
            .load_dispatch_context(
                request.source_village_id,
                request.target_village_id,
                request.hero_id,
            )
            .await?;
        self.ensure_source_owner(&context, request.player_id)?;
        let (arrives_at, returns_at) = self.round_trip_times(&context, &request.units);

        self.executor
            .execute_village_command(
                request.source_village_id,
                VillageCommandIntent::AttackVillage(AttackVillage {
                    movement_id: self.ids.next(),
                    arrival_action_id: self.ids.next(),
                    return_action_id: self.ids.next(),
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    units: request.units,
                    hero_id: request.hero_id,
                    attack_type: request.attack_type,
                    catapult_targets: request.catapult_targets,
                    arrives_at,
                    returns_at,
                }),
            )
            .await
    }

    pub async fn send_scout(&self, request: SendScoutRequest) -> Result<(), ApplicationError> {
        let context = self
            .load_dispatch_context(request.source_village_id, request.target_village_id, None)
            .await?;
        self.ensure_source_owner(&context, request.player_id)?;
        let (arrives_at, returns_at) = self.round_trip_times(&context, &request.units);

        self.executor
            .execute_village_command(
                request.source_village_id,
                VillageCommandIntent::ScoutVillage(ScoutVillage {
                    movement_id: self.ids.next(),
                    arrival_action_id: self.ids.next(),
                    return_action_id: self.ids.next(),
                    player_id: request.player_id,
                    target_village_id: request.target_village_id,
                    units: request.units,
                    target: request.target,
                    attack_type: request.attack_type,
                    arrives_at,
                    returns_at,
                }),
            )
            .await
    }

    pub async fn send_settlers(
        &self,
        request: SendSettlersRequest,
    ) -> Result<(), ApplicationError> {
        let source = self
            .reads
            .get_movement_village(request.source_village_id)
            .await?;
        if source.player_id != request.player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: request.source_village_id,
                player_id: request.player_id,
            }));
        }

        let target_field_id = request.target_position.to_id(self.settings.world_size);
        if !self.reads.is_unoccupied_valley(target_field_id).await? {
            return Err(ApplicationError::Game(GameError::InvalidValley(
                target_field_id,
            )));
        }

        let settlers_speed = source.tribe.units().get(9).map(|u| u.speed).unwrap_or(1);
        let arrives_at = self.clock.now()
            + self.travel_duration(
                source.position,
                request.target_position.clone(),
                settlers_speed,
            );

        self.executor
            .execute_village_command(
                request.source_village_id,
                VillageCommandIntent::SendSettlers(SendSettlers {
                    action_id: self.ids.next(),
                    movement_id: self.ids.next(),
                    army_id: self.ids.next(),
                    player_id: request.player_id,
                    target_village_id: target_field_id,
                    target_position: request.target_position,
                    village_name: request.village_name,
                    tribe: request.tribe,
                    arrives_at,
                }),
            )
            .await
    }

    async fn load_dispatch_context(
        &self,
        source_village_id: u32,
        target_village_id: u32,
        hero_id: Option<uuid::Uuid>,
    ) -> Result<MovementDispatchContext, ApplicationError> {
        let source = self.reads.get_movement_village(source_village_id).await?;
        let target = self.reads.get_movement_village(target_village_id).await?;
        let hero = match hero_id {
            Some(hero_id) => Some(self.reads.get_movement_hero(hero_id).await?),
            None => None,
        };

        Ok(MovementDispatchContext {
            source,
            target,
            hero,
        })
    }

    fn ensure_source_owner(
        &self,
        context: &MovementDispatchContext,
        player_id: uuid::Uuid,
    ) -> Result<(), ApplicationError> {
        if context.source.player_id != player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: context.source.village_id,
                player_id,
            }));
        }
        Ok(())
    }

    fn one_way_arrival_at(
        &self,
        context: &MovementDispatchContext,
        units: &parabellum_types::army::TroopSet,
    ) -> DateTime<Utc> {
        self.clock.now() + self.one_way_duration(context, units)
    }

    fn round_trip_times(
        &self,
        context: &MovementDispatchContext,
        units: &parabellum_types::army::TroopSet,
    ) -> (DateTime<Utc>, DateTime<Utc>) {
        let one_way = self.one_way_duration(context, units);
        let arrives_at = self.clock.now() + one_way;
        let returns_at = arrives_at + one_way;
        (arrives_at, returns_at)
    }

    fn one_way_duration(
        &self,
        context: &MovementDispatchContext,
        units: &parabellum_types::army::TroopSet,
    ) -> Duration {
        let speed =
            Army::speed_for_units(&context.source.tribe, units, context.hero.as_ref()).max(1);
        self.travel_duration(
            context.source.position.clone(),
            context.target.position.clone(),
            speed,
        )
    }

    fn travel_duration(
        &self,
        source: parabellum_types::map::Position,
        target: parabellum_types::map::Position,
        speed: u8,
    ) -> Duration {
        let secs = source.calculate_travel_time_secs(
            target,
            speed,
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
        hero::Hero,
        trapper::TrapperState,
        village::{
            AcademyResearch, ProductionBonus, VillageEffectiveProduction, VillageProduction,
            VillageStocks,
        },
    };
    use parabellum_types::{
        army::TroopSet,
        battle::{AttackType, ScoutingTarget},
        errors::{ApplicationError, GameError},
        map::Position,
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{
        models::VillageModel,
        ports::{
            Clock, IdGenerator, MovementReadPort, VillageCommandExecutor, VillageCommandIntent,
        },
        requests::movements::{
            SendAttackRequest, SendReinforcementRequest, SendScoutRequest, SendSettlersRequest,
        },
    };

    use super::{MovementSettings, MovementUseCases};

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
    struct FakeMovementReads {
        villages: Mutex<HashMap<u32, VillageModel>>,
        heroes: Mutex<HashMap<Uuid, Hero>>,
        unoccupied_valleys: Mutex<HashMap<u32, bool>>,
    }

    #[async_trait]
    impl MovementReadPort for FakeMovementReads {
        async fn get_movement_village(
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

        async fn get_movement_hero(&self, hero_id: Uuid) -> Result<Hero, ApplicationError> {
            self.heroes
                .lock()
                .expect("hero lock should not be poisoned")
                .get(&hero_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing hero {hero_id}")))
        }

        async fn is_unoccupied_valley(&self, field_id: u32) -> Result<bool, ApplicationError> {
            Ok(*self
                .unoccupied_valleys
                .lock()
                .expect("valley lock should not be poisoned")
                .get(&field_id)
                .unwrap_or(&false))
        }
    }

    #[derive(Default)]
    struct FakeExecutor {
        commands: Mutex<Vec<(u32, VillageCommandIntent)>>,
    }

    #[async_trait]
    impl VillageCommandExecutor for FakeExecutor {
        async fn execute_village_command(
            &self,
            village_id: u32,
            command: VillageCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push((village_id, command));
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

    fn use_cases(
        reads: Arc<FakeMovementReads>,
        executor: Arc<FakeExecutor>,
        ids: Vec<Uuid>,
    ) -> MovementUseCases {
        MovementUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            )),
            Arc::new(FixedIds::new(ids)),
            MovementSettings {
                world_size: 100,
                server_speed: 1,
            },
        )
    }

    #[tokio::test]
    async fn attack_dispatch_builds_round_trip_command_with_deterministic_ids() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, player_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        reads.villages.lock().unwrap().insert(
            2,
            village(2, Uuid::new_v4(), Position { x: 10, y: 0 }, Tribe::Gaul),
        );
        let executor = Arc::new(FakeExecutor::default());
        let ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let use_cases = use_cases(reads, executor.clone(), ids.clone());

        use_cases
            .send_attack(SendAttackRequest {
                player_id,
                source_village_id: 1,
                target_village_id: 2,
                units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                hero_id: None,
                attack_type: AttackType::Normal,
                catapult_targets: [None, None],
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let (village_id, command) = commands.first().expect("command should be executed");
        assert_eq!(*village_id, 1);
        let VillageCommandIntent::AttackVillage(command) = command else {
            panic!("expected attack command");
        };
        assert_eq!(command.movement_id, ids[0]);
        assert_eq!(command.arrival_action_id, ids[1]);
        assert_eq!(command.return_action_id, ids[2]);
        assert_eq!(command.player_id, player_id);
        assert!(command.returns_at > command.arrives_at);
    }

    #[tokio::test]
    async fn settlers_dispatch_rejects_occupied_target_before_execution() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, player_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        let target_position = Position { x: 2, y: 2 };
        reads
            .unoccupied_valleys
            .lock()
            .unwrap()
            .insert(target_position.to_id(100), false);
        let executor = Arc::new(FakeExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![]);

        let result = use_cases
            .send_settlers(SendSettlersRequest {
                player_id,
                source_village_id: 1,
                target_position,
                village_name: "new village".to_string(),
                tribe: Tribe::Roman,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::InvalidValley(_)))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn reinforcement_dispatch_rejects_non_owner_before_execution() {
        let owner_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, owner_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        reads.villages.lock().unwrap().insert(
            2,
            village(2, Uuid::new_v4(), Position { x: 1, y: 0 }, Tribe::Roman),
        );
        let executor = Arc::new(FakeExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![]);

        let result = use_cases
            .send_reinforcement(SendReinforcementRequest {
                player_id,
                source_village_id: 1,
                target_village_id: 2,
                units: TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                hero_id: None,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: 1,
                player_id: rejected_player_id,
            })) if rejected_player_id == player_id
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn scout_dispatch_builds_scout_command() {
        let player_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementReads::default());
        reads.villages.lock().unwrap().insert(
            1,
            village(1, player_id, Position { x: 0, y: 0 }, Tribe::Roman),
        );
        reads.villages.lock().unwrap().insert(
            2,
            village(2, Uuid::new_v4(), Position { x: 3, y: 0 }, Tribe::Roman),
        );
        let executor = Arc::new(FakeExecutor::default());
        let ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let use_cases = use_cases(reads, executor.clone(), ids.clone());

        use_cases
            .send_scout(SendScoutRequest {
                player_id,
                source_village_id: 1,
                target_village_id: 2,
                units: TroopSet::new([0, 0, 0, 1, 0, 0, 0, 0, 0, 0]),
                target: ScoutingTarget::Resources,
                attack_type: AttackType::Raid,
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let (_, command) = commands.first().expect("command should be executed");
        let VillageCommandIntent::ScoutVillage(command) = command else {
            panic!("expected scout command");
        };
        assert_eq!(command.movement_id, ids[0]);
        assert_eq!(command.arrival_action_id, ids[1]);
        assert_eq!(command.return_action_id, ids[2]);
        assert_eq!(command.player_id, player_id);
    }
}
