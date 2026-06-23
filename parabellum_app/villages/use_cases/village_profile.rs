//! Village profile use cases.
//!
//! This service owns app-level orchestration for village metadata changes.
//! Domain validation, such as name normalization and allowed length, remains in
//! the aggregate command handler.

use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::villages::{
    RenameVillage,
    ports::{VillageProfileCommandExecutor, VillageProfileCommandIntent},
    requests::village_profile::RenameVillageRequest,
};

/// Application service for village profile operations.
#[derive(Clone)]
pub struct VillageProfileUseCases {
    executor: Arc<dyn VillageProfileCommandExecutor>,
}

impl VillageProfileUseCases {
    /// Creates village profile use cases from a focused command executor.
    pub fn new(executor: Arc<dyn VillageProfileCommandExecutor>) -> Self {
        Self { executor }
    }

    /// Renames an owned village.
    pub async fn rename_village(
        &self,
        request: RenameVillageRequest,
    ) -> Result<(), ApplicationError> {
        self.executor
            .execute_village_profile_command(VillageProfileCommandIntent::RenameVillage {
                village_id: request.village_id,
                command: RenameVillage {
                    player_id: request.player_id,
                    village_name: request.village_name,
                },
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use parabellum_types::errors::ApplicationError;
    use uuid::Uuid;

    use crate::villages::{
        ports::{VillageProfileCommandExecutor, VillageProfileCommandIntent},
        requests::village_profile::RenameVillageRequest,
    };

    use super::VillageProfileUseCases;

    #[derive(Default)]
    struct FakeVillageProfileExecutor {
        commands: Mutex<VecDeque<VillageProfileCommandIntent>>,
    }

    #[async_trait]
    impl VillageProfileCommandExecutor for FakeVillageProfileExecutor {
        async fn execute_village_profile_command(
            &self,
            command: VillageProfileCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push_back(command);
            Ok(())
        }
    }

    #[tokio::test]
    async fn rename_village_builds_command_intent() {
        let executor = Arc::new(FakeVillageProfileExecutor::default());
        let use_cases = VillageProfileUseCases::new(executor.clone());
        let player_id = Uuid::new_v4();

        use_cases
            .rename_village(RenameVillageRequest {
                player_id,
                village_id: 12,
                village_name: "New Home".to_string(),
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let VillageProfileCommandIntent::RenameVillage {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed");
        assert_eq!(village_id, 12);
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.village_name, "New Home");
    }
}
