use std::{collections::HashMap, sync::Arc};

use parabellum_types::errors::ApplicationError;

use crate::{
    read_models::VillageReference,
    villages::{
        ports::VillageReferenceReadPort, requests::village_references::GetVillageReferencesRequest,
    },
};

/// Application service for compact village reference reads.
#[derive(Clone)]
pub struct VillageReferenceUseCases {
    reads: Arc<dyn VillageReferenceReadPort>,
}

impl VillageReferenceUseCases {
    /// Creates village reference use cases from the read port.
    pub fn new(reads: Arc<dyn VillageReferenceReadPort>) -> Self {
        Self { reads }
    }

    /// Resolves compact village references for display labels.
    pub async fn get_village_references(
        &self,
        request: GetVillageReferencesRequest,
    ) -> Result<HashMap<u32, VillageReference>, ApplicationError> {
        if request.village_ids.is_empty() {
            return Ok(HashMap::new());
        }

        self.reads.get_village_references(request.village_ids).await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use parabellum_types::{errors::ApplicationError, map::Position};

    use crate::{
        read_models::VillageReference,
        villages::{
            ports::VillageReferenceReadPort,
            requests::village_references::GetVillageReferencesRequest,
            use_cases::village_references::VillageReferenceUseCases,
        },
    };

    #[derive(Default)]
    struct FakeVillageReferenceReads {
        calls: Mutex<Vec<Vec<u32>>>,
    }

    #[async_trait]
    impl VillageReferenceReadPort for FakeVillageReferenceReads {
        async fn get_village_references(
            &self,
            village_ids: Vec<u32>,
        ) -> Result<HashMap<u32, VillageReference>, ApplicationError> {
            self.calls.lock().unwrap().push(village_ids);
            Ok(HashMap::from([(
                10,
                VillageReference {
                    id: 10,
                    name: "Target".to_string(),
                    position: Position { x: 1, y: 2 },
                },
            )]))
        }
    }

    #[tokio::test]
    async fn village_references_delegate_to_read_port() {
        let reads = Arc::new(FakeVillageReferenceReads::default());
        let use_cases = VillageReferenceUseCases::new(reads.clone());

        let references = use_cases
            .get_village_references(GetVillageReferencesRequest {
                village_ids: vec![10],
            })
            .await
            .unwrap();

        assert_eq!(reads.calls.lock().unwrap().as_slice(), &[vec![10]]);
        assert_eq!(references[&10].name, "Target");
    }

    #[tokio::test]
    async fn village_references_return_empty_without_read_port_call_for_empty_request() {
        let reads = Arc::new(FakeVillageReferenceReads::default());
        let use_cases = VillageReferenceUseCases::new(reads.clone());

        let references = use_cases
            .get_village_references(GetVillageReferencesRequest {
                village_ids: Vec::new(),
            })
            .await
            .unwrap();

        assert!(references.is_empty());
        assert!(reads.calls.lock().unwrap().is_empty());
    }
}
