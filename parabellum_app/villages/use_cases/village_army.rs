//! Village army query use cases.
//!
//! This service owns app-facing army state reads.

use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::{
    villages::read_models::VillageArmyStateView,
    villages::{
        ports::VillageArmyReadPort, requests::village_army::GetVillageArmyStateViewRequest,
    },
};

/// Application service for village army view reads.
#[derive(Clone)]
pub struct VillageArmyUseCases {
    reads: Arc<dyn VillageArmyReadPort>,
}

impl VillageArmyUseCases {
    /// Creates army view use cases from a focused read port.
    pub fn new(reads: Arc<dyn VillageArmyReadPort>) -> Self {
        Self { reads }
    }

    /// Loads the full army state view for a village.
    pub async fn get_village_army_state_view(
        &self,
        request: GetVillageArmyStateViewRequest,
    ) -> Result<VillageArmyStateView, ApplicationError> {
        self.reads
            .get_village_army_state_view(request.village_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use parabellum_types::errors::ApplicationError;

    use crate::{
        villages::read_models::VillageArmyStateView,
        villages::{
            ports::VillageArmyReadPort, requests::village_army::GetVillageArmyStateViewRequest,
        },
    };

    use super::VillageArmyUseCases;

    #[derive(Default)]
    struct FakeArmyReads {
        views: Mutex<HashMap<u32, VillageArmyStateView>>,
    }

    #[async_trait]
    impl VillageArmyReadPort for FakeArmyReads {
        async fn get_village_army_state_view(
            &self,
            village_id: u32,
        ) -> Result<VillageArmyStateView, ApplicationError> {
            self.views
                .lock()
                .expect("views lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing view {village_id}")))
        }
    }

    fn empty_view() -> VillageArmyStateView {
        VillageArmyStateView {
            home_army: None,
            reinforcements: vec![],
            deployed_armies: vec![],
            trapped_here: vec![],
            trapped_away: vec![],
        }
    }

    #[tokio::test]
    async fn army_state_view_delegates_to_read_port() {
        let reads = Arc::new(FakeArmyReads::default());
        reads.views.lock().unwrap().insert(1, empty_view());
        let use_cases = VillageArmyUseCases::new(reads);

        let view = use_cases
            .get_village_army_state_view(GetVillageArmyStateViewRequest { village_id: 1 })
            .await
            .unwrap();

        assert!(view.home_army.is_none());
        assert!(view.reinforcements.is_empty());
    }
}
