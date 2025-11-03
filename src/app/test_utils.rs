#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    use crate::{
        Result,
        error::ApplicationError,
        game::models::{Player, army::Army, map::Valley, village::Village},
        jobs::Job,
        repository::{
            ArmyRepository, JobRepository, MapRepository, PlayerRepository, VillageRepository,
            uow::UnitOfWork,
        },
    };

    // --- New Mock Repositories ---
    #[derive(Default, Clone)]
    pub struct MockJobRepository {
        // Use Arc<Mutex<...>> to hold state
        added_jobs: Arc<Mutex<Vec<Job>>>,
    }

    impl MockJobRepository {
        pub fn new() -> Self {
            Self {
                added_jobs: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn get_added_jobs(&self) -> Vec<Job> {
            self.added_jobs.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl JobRepository for MockJobRepository {
        async fn add(&self, job: &Job) -> Result<(), ApplicationError> {
            self.added_jobs.lock().unwrap().push(job.clone());
            Ok(())
        }

        // ... (implement other methods as needed, returning Ok(...) or mock data)
        async fn get_by_id(&self, _id: Uuid) -> Result<Option<Job>, ApplicationError> {
            Ok(None)
        }
        async fn list_by_player_id(&self, _id: Uuid) -> Result<Vec<Job>, ApplicationError> {
            Ok(self.added_jobs.lock().unwrap().clone())
        }
        async fn find_and_lock_due_jobs(&self, _limit: i64) -> Result<Vec<Job>, ApplicationError> {
            Ok(self.added_jobs.lock().unwrap().clone())
        }
        async fn mark_as_completed(&self, _job_id: Uuid) -> Result<(), ApplicationError> {
            Ok(())
        }
        async fn mark_as_failed(
            &self,
            _job_id: Uuid,
            _error_message: &str,
        ) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    pub struct MockVillageRepository {
        villages: Arc<Mutex<HashMap<u32, Village>>>,
    }

    impl MockVillageRepository {
        pub fn add_village(&self, village: Village) {
            self.villages.lock().unwrap().insert(village.id, village);
        }

        pub fn saved_villages(&self) -> Vec<Village> {
            self.villages.lock().unwrap().values().cloned().collect()
        }
    }

    #[async_trait]
    impl VillageRepository for MockVillageRepository {
        async fn get_by_id(&self, village_id: u32) -> Result<Option<Village>, ApplicationError> {
            let villages = self.villages.lock().unwrap();
            Ok(villages.get(&village_id).cloned())
        }

        async fn create(&self, village: &Village) -> Result<(), ApplicationError> {
            self.villages
                .lock()
                .unwrap()
                .insert(village.id, village.clone());

            Ok(())
        }
        async fn list_by_player_id(
            &self,
            player_id: Uuid,
        ) -> Result<Vec<Village>, ApplicationError> {
            let mut villages: Vec<Village> = vec![];

            for v in self.villages.lock().unwrap().values().into_iter() {
                if v.player_id == player_id {
                    villages.push(v.clone());
                }
            }

            Ok(villages)
        }
        async fn save(&self, village: &Village) -> Result<(), ApplicationError> {
            self.villages
                .lock()
                .unwrap()
                .insert(village.id, village.clone());
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    pub struct MockArmyRepository {
        armies: Arc<Mutex<HashMap<Uuid, Army>>>,
    }

    impl MockArmyRepository {
        pub fn add_army(&self, army: Army) {
            self.armies.lock().unwrap().insert(army.id, army);
        }
    }

    #[async_trait]
    impl ArmyRepository for MockArmyRepository {
        async fn get_by_id(&self, army_id: Uuid) -> Result<Option<Army>, ApplicationError> {
            let armies = self.armies.lock().unwrap();
            Ok(armies.get(&army_id).cloned())
        }
        // ... (implement other methods)
        async fn create(&self, _army: &Army) -> Result<(), ApplicationError> {
            Ok(())
        }
        async fn save(&self, _army: &Army) -> Result<(), ApplicationError> {
            Ok(())
        }
        async fn remove(&self, _army_id: Uuid) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    // Mock per i repo non usati in questo test
    #[derive(Default, Clone)]
    pub struct MockPlayerRepository {
        players: Arc<Mutex<HashMap<Uuid, Player>>>,
    }

    #[async_trait]
    impl PlayerRepository for MockPlayerRepository {
        async fn create(&self, _player: &Player) -> Result<(), ApplicationError> {
            Ok(())
        }
        async fn get_by_id(&self, _player_id: Uuid) -> Result<Option<Player>, ApplicationError> {
            Ok(None)
        }
        async fn get_by_username(
            &self,
            _username: &str,
        ) -> Result<Option<Player>, ApplicationError> {
            Ok(None)
        }
    }

    #[derive(Default, Clone)]
    pub struct MockMapRepository;

    #[async_trait]
    impl MapRepository for MockMapRepository {
        async fn find_unoccupied_valley(
            &self,
            _quadrant: &crate::game::models::map::MapQuadrant,
        ) -> Result<Valley, ApplicationError> {
            panic!("Not mocked")
        }
        async fn get_field_by_id(
            &self,
            _id: i32,
        ) -> Result<Option<crate::game::models::map::MapField>, ApplicationError> {
            Ok(None)
        }
    }

    /// A Mock Unit of Work that holds mock repositories.
    #[derive(Default)]
    pub struct MockUnitOfWork {
        players: Arc<MockPlayerRepository>,
        villages: Arc<MockVillageRepository>,
        armies: Arc<MockArmyRepository>,
        jobs: Arc<MockJobRepository>,
        map: Arc<MockMapRepository>,

        // Flags to check if commit/rollback was called
        committed: Arc<Mutex<bool>>,
        rolled_back: Arc<Mutex<bool>>,
    }

    impl MockUnitOfWork {
        pub fn new() -> Self {
            Default::default()
        }

        // Helper methods to get the underlying mocks for setup
        pub fn mock_villages(&self) -> Arc<MockVillageRepository> {
            self.villages.clone()
        }

        pub fn mock_armies(&self) -> Arc<MockArmyRepository> {
            self.armies.clone()
        }

        pub fn mock_jobs(&self) -> Arc<MockJobRepository> {
            self.jobs.clone()
        }
    }

    #[async_trait]
    impl<'a> UnitOfWork<'a> for MockUnitOfWork {
        fn players(&self) -> Arc<dyn PlayerRepository + 'a> {
            self.players.clone()
        }
        fn villages(&self) -> Arc<dyn VillageRepository + 'a> {
            self.villages.clone()
        }
        fn armies(&self) -> Arc<dyn ArmyRepository + 'a> {
            self.armies.clone()
        }
        fn jobs(&self) -> Arc<dyn JobRepository + 'a> {
            self.jobs.clone()
        }
        fn map(&self) -> Arc<dyn MapRepository + 'a> {
            self.map.clone()
        }

        // We consume self (Box<Self>) as per the trait definition
        async fn commit(self: Box<Self>) -> Result<(), ApplicationError> {
            *self.committed.lock().unwrap() = true;
            Ok(())
        }

        async fn rollback(self: Box<Self>) -> Result<(), ApplicationError> {
            *self.rolled_back.lock().unwrap() = true;
            Ok(())
        }
    }
}
