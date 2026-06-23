//! Building lifecycle use cases.
//!
//! This service owns app-level orchestration for building construction,
//! upgrades, downgrades, and cancellation. Domain rules remain in the
//! aggregate/domain command handlers; this layer applies app settings, loads
//! cancellation context, validates app-visible ownership/cancelability, and
//! delegates command execution through app ports.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use parabellum_types::errors::{ApplicationError, GameError};

use crate::villages::{
    AddBuilding, CancelBuildingConstruction, DowngradeBuilding, UpgradeBuilding,
    ports::{BuildingCommandExecutor, BuildingCommandIntent, BuildingReadPort, Clock},
    requests::buildings::{
        AddBuildingRequest, CancelBuildingConstructionRequest, DowngradeBuildingRequest,
        UpgradeBuildingRequest,
    },
};

/// Runtime settings used by building lifecycle use cases.
#[derive(Debug, Clone, Copy)]
pub struct BuildingSettings {
    /// Server speed multiplier used by building scheduling commands.
    pub server_speed: i8,
}

/// Application service for building lifecycle operations.
#[derive(Clone)]
pub struct BuildingUseCases {
    reads: Arc<dyn BuildingReadPort>,
    executor: Arc<dyn BuildingCommandExecutor>,
    clock: Arc<dyn Clock>,
    settings: BuildingSettings,
}

impl BuildingUseCases {
    /// Creates building lifecycle use cases from focused ports and settings.
    pub fn new(
        reads: Arc<dyn BuildingReadPort>,
        executor: Arc<dyn BuildingCommandExecutor>,
        clock: Arc<dyn Clock>,
        settings: BuildingSettings,
    ) -> Self {
        Self {
            reads,
            executor,
            clock,
            settings,
        }
    }

    /// Schedules construction of a new building on an empty village slot.
    pub async fn add_building(&self, request: AddBuildingRequest) -> Result<(), ApplicationError> {
        self.executor
            .execute_building_command(BuildingCommandIntent::AddBuilding {
                village_id: request.village_id,
                command: AddBuilding {
                    player_id: request.player_id,
                    slot_id: request.slot_id,
                    building_name: request.building_name,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }

    /// Schedules an upgrade for an existing building slot.
    pub async fn upgrade_building(
        &self,
        request: UpgradeBuildingRequest,
    ) -> Result<(), ApplicationError> {
        self.executor
            .execute_building_command(BuildingCommandIntent::UpgradeBuilding {
                village_id: request.village_id,
                command: UpgradeBuilding {
                    player_id: request.player_id,
                    slot_id: request.slot_id,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }

    /// Schedules a downgrade for an existing building slot.
    pub async fn downgrade_building(
        &self,
        request: DowngradeBuildingRequest,
    ) -> Result<(), ApplicationError> {
        self.executor
            .execute_building_command(BuildingCommandIntent::DowngradeBuilding {
                village_id: request.village_id,
                command: DowngradeBuilding {
                    player_id: request.player_id,
                    slot_id: request.slot_id,
                    speed: self.settings.server_speed,
                },
            })
            .await
    }

    /// Cancels a queued building construction action before it executes.
    pub async fn cancel_building_construction(
        &self,
        request: CancelBuildingConstructionRequest,
    ) -> Result<(), ApplicationError> {
        let now = self.clock.now();
        let context = self
            .reads
            .get_cancel_building_construction_context(request.village_id, request.action_id, now)
            .await?;

        if context.player_id != request.player_id || context.village_id != request.village_id {
            return Err(ApplicationError::Game(GameError::VillageNotOwned {
                village_id: request.village_id,
                player_id: request.player_id,
            }));
        }

        self.ensure_cancelable(context.execute_at, now)?;

        self.executor
            .execute_building_command(BuildingCommandIntent::CancelBuildingConstruction {
                village_id: context.village_id,
                command: CancelBuildingConstruction {
                    action_ids: context.action_ids,
                    player_id: request.player_id,
                    village_id: context.village_id,
                    refund: context.refund,
                    canceled_at: now,
                },
            })
            .await
    }

    fn ensure_cancelable(
        &self,
        execute_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        if now >= execute_at {
            return Err(ApplicationError::Game(
                GameError::BuildingConstructionNotCancelable,
            ));
        }
        Ok(())
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
    use parabellum_types::{
        buildings::BuildingName,
        common::ResourceGroup,
        errors::{ApplicationError, GameError},
    };
    use uuid::Uuid;

    use crate::villages::{
        ports::{
            BuildingCommandExecutor, BuildingCommandIntent, BuildingReadPort,
            CancelBuildingConstructionContext, Clock,
        },
        requests::buildings::{
            AddBuildingRequest, CancelBuildingConstructionRequest, DowngradeBuildingRequest,
            UpgradeBuildingRequest,
        },
    };

    use super::{BuildingSettings, BuildingUseCases};

    #[derive(Clone)]
    struct FixedClock(chrono::DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            self.0
        }
    }

    #[derive(Default)]
    struct FakeBuildingReads {
        contexts: Mutex<HashMap<Uuid, CancelBuildingConstructionContext>>,
    }

    #[async_trait]
    impl BuildingReadPort for FakeBuildingReads {
        async fn get_cancel_building_construction_context(
            &self,
            _village_id: u32,
            action_id: Uuid,
            _canceled_at: chrono::DateTime<Utc>,
        ) -> Result<CancelBuildingConstructionContext, ApplicationError> {
            self.contexts
                .lock()
                .expect("context lock should not be poisoned")
                .get(&action_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing action {action_id}")))
        }
    }

    #[derive(Default)]
    struct FakeBuildingExecutor {
        commands: Mutex<VecDeque<BuildingCommandIntent>>,
    }

    #[async_trait]
    impl BuildingCommandExecutor for FakeBuildingExecutor {
        async fn execute_building_command(
            &self,
            command: BuildingCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push_back(command);
            Ok(())
        }
    }

    fn use_cases(
        reads: Arc<FakeBuildingReads>,
        executor: Arc<FakeBuildingExecutor>,
    ) -> BuildingUseCases {
        BuildingUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            )),
            BuildingSettings { server_speed: 3 },
        )
    }

    #[tokio::test]
    async fn add_building_builds_command_with_configured_speed() {
        let executor = Arc::new(FakeBuildingExecutor::default());
        let use_cases = use_cases(Arc::new(FakeBuildingReads::default()), executor.clone());
        let player_id = Uuid::new_v4();

        use_cases
            .add_building(AddBuildingRequest {
                player_id,
                village_id: 7,
                slot_id: 22,
                building_name: BuildingName::Barracks,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let BuildingCommandIntent::AddBuilding {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected add building command");
        };
        assert_eq!(village_id, 7);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.slot_id, 22);
        assert_eq!(command.building_name, BuildingName::Barracks);
        assert_eq!(command.speed, 3);
    }

    #[tokio::test]
    async fn upgrade_building_builds_command_with_configured_speed() {
        let executor = Arc::new(FakeBuildingExecutor::default());
        let use_cases = use_cases(Arc::new(FakeBuildingReads::default()), executor.clone());
        let player_id = Uuid::new_v4();

        use_cases
            .upgrade_building(UpgradeBuildingRequest {
                player_id,
                village_id: 8,
                slot_id: 19,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let BuildingCommandIntent::UpgradeBuilding {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected upgrade building command");
        };
        assert_eq!(village_id, 8);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.slot_id, 19);
        assert_eq!(command.speed, 3);
    }

    #[tokio::test]
    async fn downgrade_building_builds_command_with_configured_speed() {
        let executor = Arc::new(FakeBuildingExecutor::default());
        let use_cases = use_cases(Arc::new(FakeBuildingReads::default()), executor.clone());
        let player_id = Uuid::new_v4();

        use_cases
            .downgrade_building(DowngradeBuildingRequest {
                player_id,
                village_id: 9,
                slot_id: 20,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let BuildingCommandIntent::DowngradeBuilding {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected downgrade building command");
        };
        assert_eq!(village_id, 9);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.slot_id, 20);
        assert_eq!(command.speed, 3);
    }

    #[tokio::test]
    async fn cancel_building_construction_rejects_non_owner_without_executing() {
        let owner_id = Uuid::new_v4();
        let requester_id = Uuid::new_v4();
        let action_id = Uuid::new_v4();
        let reads = Arc::new(FakeBuildingReads::default());
        reads.contexts.lock().unwrap().insert(
            action_id,
            CancelBuildingConstructionContext {
                action_ids: vec![action_id],
                player_id: owner_id,
                village_id: 1,
                execute_at: Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap(),
                refund: ResourceGroup::new(10, 20, 30, 40),
            },
        );
        let executor = Arc::new(FakeBuildingExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .cancel_building_construction(CancelBuildingConstructionRequest {
                player_id: requester_id,
                village_id: 1,
                action_id,
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
    async fn cancel_building_construction_rejects_executed_action_without_executing() {
        let player_id = Uuid::new_v4();
        let action_id = Uuid::new_v4();
        let reads = Arc::new(FakeBuildingReads::default());
        reads.contexts.lock().unwrap().insert(
            action_id,
            CancelBuildingConstructionContext {
                action_ids: vec![action_id],
                player_id,
                village_id: 1,
                execute_at: Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
                refund: ResourceGroup::new(10, 20, 30, 40),
            },
        );
        let executor = Arc::new(FakeBuildingExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .cancel_building_construction(CancelBuildingConstructionRequest {
                player_id,
                village_id: 1,
                action_id,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(
                GameError::BuildingConstructionNotCancelable
            ))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn cancel_building_construction_builds_command_with_context_refund_and_time() {
        let player_id = Uuid::new_v4();
        let action_id = Uuid::new_v4();
        let linked_action_id = Uuid::new_v4();
        let reads = Arc::new(FakeBuildingReads::default());
        reads.contexts.lock().unwrap().insert(
            action_id,
            CancelBuildingConstructionContext {
                action_ids: vec![action_id, linked_action_id],
                player_id,
                village_id: 1,
                execute_at: Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap(),
                refund: ResourceGroup::new(10, 20, 30, 40),
            },
        );
        let executor = Arc::new(FakeBuildingExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .cancel_building_construction(CancelBuildingConstructionRequest {
                player_id,
                village_id: 1,
                action_id,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let BuildingCommandIntent::CancelBuildingConstruction {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected cancel construction command");
        };
        assert_eq!(village_id, 1);
        assert_eq!(command.action_ids, vec![action_id, linked_action_id]);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.village_id, 1);
        assert_eq!(command.refund, ResourceGroup::new(10, 20, 30, 40));
        assert_eq!(
            command.canceled_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()
        );
    }
}
