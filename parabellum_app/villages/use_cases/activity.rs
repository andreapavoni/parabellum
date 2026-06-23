//! Village activity query use cases.
//!
//! This service owns app-facing queue and troop movement reads. It keeps UI
//! activity views out of command-oriented use cases.

use std::{collections::HashSet, sync::Arc};

use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::{
    villages::read_models::{VillageQueues, VillageTroopMovements},
    villages::{
        ports::{Clock, VillageActivityReadPort},
        requests::activity::{
            GetVillageQueuesRequest, GetVillageTroopMovementsRequest,
            ListCancelableOutgoingMovementIdsRequest,
        },
    },
};

/// Application service for village activity reads.
#[derive(Clone)]
pub struct VillageActivityUseCases {
    reads: Arc<dyn VillageActivityReadPort>,
    clock: Arc<dyn Clock>,
}

impl VillageActivityUseCases {
    /// Creates activity use cases from focused read ports.
    pub fn new(reads: Arc<dyn VillageActivityReadPort>, clock: Arc<dyn Clock>) -> Self {
        Self { reads, clock }
    }

    /// Loads village queue summaries.
    pub async fn get_village_queues(
        &self,
        request: GetVillageQueuesRequest,
    ) -> Result<VillageQueues, ApplicationError> {
        self.reads.get_village_queues(request.village_id).await
    }

    /// Loads village troop movement summaries.
    pub async fn get_village_troop_movements(
        &self,
        request: GetVillageTroopMovementsRequest,
    ) -> Result<VillageTroopMovements, ApplicationError> {
        self.reads
            .get_village_troop_movements(request.village_id)
            .await
    }

    /// Lists cancelable outgoing movement ids at the current app clock time.
    pub async fn list_cancelable_outgoing_movement_ids(
        &self,
        request: ListCancelableOutgoingMovementIdsRequest,
    ) -> Result<HashSet<Uuid>, ApplicationError> {
        self.reads
            .list_cancelable_outgoing_movement_ids(request.village_id, self.clock.now())
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use parabellum_types::errors::ApplicationError;
    use uuid::Uuid;

    use crate::{
        villages::read_models::{VillageQueues, VillageTroopMovements},
        villages::{
            ports::{Clock, VillageActivityReadPort},
            requests::activity::{
                GetVillageQueuesRequest, GetVillageTroopMovementsRequest,
                ListCancelableOutgoingMovementIdsRequest,
            },
        },
    };

    use super::VillageActivityUseCases;

    #[derive(Clone)]
    struct FixedClock(chrono::DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            self.0
        }
    }

    #[derive(Default)]
    struct FakeActivityReads {
        queues: Mutex<HashMap<u32, VillageQueues>>,
        movements: Mutex<HashMap<u32, VillageTroopMovements>>,
        cancelable: Mutex<HashMap<u32, HashSet<Uuid>>>,
        cancelable_seen_at: Mutex<Option<chrono::DateTime<Utc>>>,
    }

    #[async_trait]
    impl VillageActivityReadPort for FakeActivityReads {
        async fn get_village_queues(
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

        async fn get_village_troop_movements(
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

        async fn list_cancelable_outgoing_movement_ids(
            &self,
            village_id: u32,
            now: chrono::DateTime<Utc>,
        ) -> Result<HashSet<Uuid>, ApplicationError> {
            *self
                .cancelable_seen_at
                .lock()
                .expect("seen-at lock should not be poisoned") = Some(now);
            Ok(self
                .cancelable
                .lock()
                .expect("cancelable lock should not be poisoned")
                .get(&village_id)
                .cloned()
                .unwrap_or_default())
        }
    }

    #[tokio::test]
    async fn activity_reads_delegate_to_read_port_and_clock() {
        let movement_id = Uuid::new_v4();
        let reads = Arc::new(FakeActivityReads::default());
        reads
            .queues
            .lock()
            .unwrap()
            .insert(1, VillageQueues::default());
        reads
            .movements
            .lock()
            .unwrap()
            .insert(1, VillageTroopMovements::default());
        reads
            .cancelable
            .lock()
            .unwrap()
            .insert(1, HashSet::from([movement_id]));
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let use_cases = VillageActivityUseCases::new(reads.clone(), Arc::new(FixedClock(now)));

        let queues = use_cases
            .get_village_queues(GetVillageQueuesRequest { village_id: 1 })
            .await
            .unwrap();
        let movements = use_cases
            .get_village_troop_movements(GetVillageTroopMovementsRequest { village_id: 1 })
            .await
            .unwrap();
        let cancelable = use_cases
            .list_cancelable_outgoing_movement_ids(ListCancelableOutgoingMovementIdsRequest {
                village_id: 1,
            })
            .await
            .unwrap();

        assert!(queues.building.is_empty());
        assert!(movements.outgoing.is_empty());
        assert_eq!(cancelable, HashSet::from([movement_id]));
        assert_eq!(*reads.cancelable_seen_at.lock().unwrap(), Some(now));
    }
}
