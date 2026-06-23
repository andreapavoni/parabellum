//! Movement-control use cases.
//!
//! This service owns app-level orchestration for controlling existing troop
//! movements. It validates source ownership, cancelability windows, deterministic
//! ids/timestamps, and delegates command execution through app ports.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parabellum_types::errors::{ApplicationError, GameError};

use crate::villages::{
    CancelTroopMovement,
    ports::{
        Clock, IdGenerator, MovementControlCommandExecutor, MovementControlCommandIntent,
        MovementControlReadPort,
    },
    requests::movement_control::CancelTroopMovementRequest,
};

/// Application service for controlling already-created troop movements.
#[derive(Clone)]
pub struct MovementControlUseCases {
    reads: Arc<dyn MovementControlReadPort>,
    executor: Arc<dyn MovementControlCommandExecutor>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl MovementControlUseCases {
    /// Creates movement-control use cases from focused read/execution ports.
    pub fn new(
        reads: Arc<dyn MovementControlReadPort>,
        executor: Arc<dyn MovementControlCommandExecutor>,
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

    /// Cancels an outgoing troop movement and schedules the returning army.
    ///
    /// The request village must match the movement source, the requesting
    /// player must own that source village, and cancellation must happen before
    /// arrival within the Travian-style cancel window.
    pub async fn cancel_troop_movement(
        &self,
        request: CancelTroopMovementRequest,
    ) -> Result<(), ApplicationError> {
        let context = self
            .reads
            .get_cancel_troop_movement_context(request.movement_id)
            .await?;
        if context.source_village_id != request.village_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: request.village_id,
                player_id: request.player_id,
            }));
        }

        let source = self
            .reads
            .get_movement_control_village(context.source_village_id)
            .await?;
        if source.player_id != request.player_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: context.source_village_id,
                player_id: request.player_id,
            }));
        }

        let now = self.clock.now();
        let returns_at = self.cancel_return_time(context.sent_at, context.arrives_at, now)?;
        self.executor
            .execute_movement_control_command(MovementControlCommandIntent::CancelTroopMovement {
                source_village_id: context.source_village_id,
                command: CancelTroopMovement {
                    movement_id: context.movement_id,
                    arrival_action_id: context.arrival_action_id,
                    return_action_id: self.ids.next(),
                    army_id: context.army_id,
                    player_id: request.player_id,
                    source_village_id: context.source_village_id,
                    target_village_id: context.target_village_id,
                    army: context.army,
                    returns_at,
                },
            })
            .await
    }

    fn cancel_return_time(
        &self,
        sent_at: DateTime<Utc>,
        arrives_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, ApplicationError> {
        if now >= arrives_at {
            return Err(ApplicationError::Game(
                GameError::TroopMovementNotCancelable,
            ));
        }

        let cancel_deadline = sent_at + Duration::seconds(60);
        if now > cancel_deadline {
            return Err(ApplicationError::Game(
                GameError::TroopMovementCancelWindowExpired,
            ));
        }

        let elapsed = (now - sent_at).num_seconds().max(1);
        Ok(now + Duration::seconds(elapsed))
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

    use crate::villages::{
        models::VillageModel,
        ports::{
            CancelTroopMovementContext, Clock, IdGenerator, MovementControlCommandExecutor,
            MovementControlCommandIntent, MovementControlReadPort,
        },
        requests::movement_control::CancelTroopMovementRequest,
    };

    use super::MovementControlUseCases;

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
    struct FakeMovementControlReads {
        contexts: Mutex<HashMap<Uuid, CancelTroopMovementContext>>,
        villages: Mutex<HashMap<u32, VillageModel>>,
    }

    #[async_trait]
    impl MovementControlReadPort for FakeMovementControlReads {
        async fn get_cancel_troop_movement_context(
            &self,
            movement_id: Uuid,
        ) -> Result<CancelTroopMovementContext, ApplicationError> {
            self.contexts
                .lock()
                .expect("context lock should not be poisoned")
                .get(&movement_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing movement {movement_id}")))
        }

        async fn get_movement_control_village(
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
    }

    #[derive(Default)]
    struct FakeMovementControlExecutor {
        commands: Mutex<Vec<MovementControlCommandIntent>>,
    }

    #[async_trait]
    impl MovementControlCommandExecutor for FakeMovementControlExecutor {
        async fn execute_movement_control_command(
            &self,
            command: MovementControlCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push(command);
            Ok(())
        }
    }

    fn village(village_id: u32, player_id: Uuid) -> VillageModel {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        VillageModel {
            village_id,
            player_id,
            village_name: format!("village-{village_id}"),
            position: Position { x: 0, y: 0 },
            tribe: Tribe::Roman,
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

    fn army(army_id: Uuid, player_id: Uuid) -> Army {
        Army::new(
            Some(army_id),
            1,
            Some(1),
            player_id,
            Tribe::Roman,
            &TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            &[0; 8],
            None,
        )
    }

    fn context(
        movement_id: Uuid,
        player_id: Uuid,
        sent_at: chrono::DateTime<Utc>,
        arrives_at: chrono::DateTime<Utc>,
    ) -> CancelTroopMovementContext {
        let army_id = Uuid::new_v4();
        CancelTroopMovementContext {
            movement_id,
            arrival_action_id: Uuid::new_v4(),
            army_id,
            player_id,
            source_village_id: 1,
            target_village_id: 2,
            army: army(army_id, player_id),
            sent_at,
            arrives_at,
        }
    }

    fn use_cases(
        reads: Arc<FakeMovementControlReads>,
        executor: Arc<FakeMovementControlExecutor>,
        ids: Vec<Uuid>,
    ) -> MovementControlUseCases {
        MovementControlUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 30).unwrap(),
            )),
            Arc::new(FixedIds::new(ids)),
        )
    }

    #[tokio::test]
    async fn cancel_troop_movement_rejects_source_village_mismatch_without_executing() {
        let player_id = Uuid::new_v4();
        let movement_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementControlReads::default());
        reads.contexts.lock().unwrap().insert(
            movement_id,
            context(
                movement_id,
                player_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 10, 0).unwrap(),
            ),
        );
        let executor = Arc::new(FakeMovementControlExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![Uuid::new_v4()]);

        let result = use_cases
            .cancel_troop_movement(CancelTroopMovementRequest {
                player_id,
                village_id: 99,
                movement_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: 99,
                ..
            }))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancel_troop_movement_rejects_non_owner_without_executing() {
        let owner_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let movement_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementControlReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, owner_id));
        reads.contexts.lock().unwrap().insert(
            movement_id,
            context(
                movement_id,
                owner_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 10, 0).unwrap(),
            ),
        );
        let executor = Arc::new(FakeMovementControlExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![Uuid::new_v4()]);

        let result = use_cases
            .cancel_troop_movement(CancelTroopMovementRequest {
                player_id: requester_id,
                village_id: 1,
                movement_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: 1,
                player_id
            })) if player_id == requester_id
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancel_troop_movement_rejects_arrived_movement_without_executing() {
        let player_id = Uuid::new_v4();
        let movement_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementControlReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id));
        reads.contexts.lock().unwrap().insert(
            movement_id,
            context(
                movement_id,
                player_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 30).unwrap(),
            ),
        );
        let executor = Arc::new(FakeMovementControlExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![Uuid::new_v4()]);

        let result = use_cases
            .cancel_troop_movement(CancelTroopMovementRequest {
                player_id,
                village_id: 1,
                movement_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(
                GameError::TroopMovementNotCancelable
            ))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancel_troop_movement_rejects_expired_cancel_window_without_executing() {
        let player_id = Uuid::new_v4();
        let movement_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementControlReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id));
        reads.contexts.lock().unwrap().insert(
            movement_id,
            context(
                movement_id,
                player_id,
                Utc.with_ymd_and_hms(2026, 1, 1, 11, 58, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 10, 0).unwrap(),
            ),
        );
        let executor = Arc::new(FakeMovementControlExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![Uuid::new_v4()]);

        let result = use_cases
            .cancel_troop_movement(CancelTroopMovementRequest {
                player_id,
                village_id: 1,
                movement_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(
                GameError::TroopMovementCancelWindowExpired
            ))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancel_troop_movement_builds_return_command_with_deterministic_id_and_time() {
        let player_id = Uuid::new_v4();
        let movement_id = Uuid::new_v4();
        let return_action_id = Uuid::new_v4();
        let reads = Arc::new(FakeMovementControlReads::default());
        reads
            .villages
            .lock()
            .unwrap()
            .insert(1, village(1, player_id));
        let context = context(
            movement_id,
            player_id,
            Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 1, 12, 10, 0).unwrap(),
        );
        let army_id = context.army_id;
        let arrival_action_id = context.arrival_action_id;
        reads.contexts.lock().unwrap().insert(movement_id, context);
        let executor = Arc::new(FakeMovementControlExecutor::default());
        let use_cases = use_cases(reads, executor.clone(), vec![return_action_id]);

        use_cases
            .cancel_troop_movement(CancelTroopMovementRequest {
                player_id,
                village_id: 1,
                movement_id,
            })
            .await
            .unwrap();

        let commands = executor.commands.lock().unwrap();
        let MovementControlCommandIntent::CancelTroopMovement {
            source_village_id,
            command,
        } = commands.first().expect("command should be executed");
        assert_eq!(*source_village_id, 1);
        assert_eq!(command.movement_id, movement_id);
        assert_eq!(command.arrival_action_id, arrival_action_id);
        assert_eq!(command.return_action_id, return_action_id);
        assert_eq!(command.army_id, army_id);
        assert_eq!(command.player_id, player_id);
        assert_eq!(
            command.returns_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 12, 1, 0).unwrap()
        );
    }
}
