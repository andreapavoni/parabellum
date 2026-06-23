use std::sync::Arc;

use parabellum_types::errors::ApplicationError;

use crate::villages::{
    models::VillageModel,
    ports::VillageStateReadPort,
    requests::village_state::{GetVillageStateRequest, ListPlayerVillageStatesRequest},
};

/// Application service for full village projection state reads.
#[derive(Clone)]
pub struct VillageStateUseCases {
    reads: Arc<dyn VillageStateReadPort>,
}

impl VillageStateUseCases {
    /// Creates village state use cases from the read port.
    pub fn new(reads: Arc<dyn VillageStateReadPort>) -> Self {
        Self { reads }
    }

    /// Loads one full village projection state.
    pub async fn get_village_state(
        &self,
        request: GetVillageStateRequest,
    ) -> Result<VillageModel, ApplicationError> {
        self.reads.get_village_state(request.village_id).await
    }

    /// Lists full village projection states owned by one player.
    pub async fn list_player_village_states(
        &self,
        request: ListPlayerVillageStatesRequest,
    ) -> Result<Vec<VillageModel>, ApplicationError> {
        self.reads
            .list_player_village_states(request.player_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use parabellum_types::errors::ApplicationError;
    use uuid::Uuid;

    use crate::villages::{
        models::VillageModel,
        ports::VillageStateReadPort,
        requests::village_state::{GetVillageStateRequest, ListPlayerVillageStatesRequest},
        use_cases::village_state::VillageStateUseCases,
    };

    #[derive(Default)]
    struct FakeVillageStateReads {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl VillageStateReadPort for FakeVillageStateReads {
        async fn get_village_state(
            &self,
            village_id: u32,
        ) -> Result<VillageModel, ApplicationError> {
            self.calls.lock().unwrap().push(format!("get:{village_id}"));
            Err(ApplicationError::Unknown("get called".to_string()))
        }

        async fn list_player_village_states(
            &self,
            player_id: Uuid,
        ) -> Result<Vec<VillageModel>, ApplicationError> {
            self.calls.lock().unwrap().push(format!("list:{player_id}"));
            Err(ApplicationError::Unknown("list called".to_string()))
        }
    }

    #[tokio::test]
    async fn village_state_reads_delegate_to_read_port() {
        let reads = Arc::new(FakeVillageStateReads::default());
        let use_cases = VillageStateUseCases::new(reads.clone());
        let player_id = Uuid::new_v4();

        let village = use_cases
            .get_village_state(GetVillageStateRequest { village_id: 10 })
            .await;
        let villages = use_cases
            .list_player_village_states(ListPlayerVillageStatesRequest { player_id })
            .await;

        assert!(village.is_err());
        assert!(villages.is_err());
        assert_eq!(
            reads.calls.lock().unwrap().as_slice(),
            &[format!("get:10"), format!("list:{player_id}")]
        );
    }
}
