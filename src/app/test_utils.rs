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
        db::DbError,
        error::ApplicationError,
        game::models::{
            Player,
            army::Army,
            map::{MapField, MapFieldTopology, Position, Valley, ValleyTopology},
            village::Village,
        },
        jobs::Job,
        repository::{
            ArmyRepository, JobRepository, MapRepository, PlayerRepository, VillageRepository,
        },
        uow::UnitOfWork,
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
        async fn get_by_id(&self, id: Uuid) -> Result<Job, ApplicationError> {
            let jobs = self.added_jobs.lock().unwrap().clone();

            Ok(jobs
                .into_iter()
                .find(|j| j.id == id)
                .ok_or_else(|| ApplicationError::Db(DbError::JobNotFound(id)))?)
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

    #[async_trait]
    impl VillageRepository for MockVillageRepository {
        async fn get_by_id(&self, village_id: u32) -> Result<Village, ApplicationError> {
            let villages = self.villages.lock().unwrap();
            Ok(villages.get(&village_id).unwrap().clone())
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
        async fn get_by_id(&self, army_id: Uuid) -> Result<Army, ApplicationError> {
            let armies = self.armies.lock().unwrap();
            Ok(armies
                .get(&army_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::ArmyNotFound(army_id)))?)
        }

        async fn save(&self, army: &Army) -> Result<(), ApplicationError> {
            let mut armies = self.armies.lock().unwrap();
            armies.insert(army.id, army.clone());
            Ok(())
        }
        async fn remove(&self, army_id: Uuid) -> Result<(), ApplicationError> {
            let mut armies = self.armies.lock().unwrap();
            armies.remove(&army_id);
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
        async fn save(&self, player: &Player) -> Result<(), ApplicationError> {
            self.players
                .lock()
                .unwrap()
                .insert(player.id, player.clone());
            Ok(())
        }

        async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError> {
            if let Some(player) = self.players.lock().unwrap().get(&player_id) {
                Ok(player.clone())
            } else {
                Err(ApplicationError::Db(DbError::PlayerNotFound(player_id)))
            }
        }
    }

    #[derive(Default, Clone)]
    pub struct MockMapRepository {
        fields: HashMap<u32, MapField>,
    }

    impl MockMapRepository {
        pub fn add_map_field(&mut self, field: MapField) {
            self.fields.insert(field.id, field);
        }
    }

    #[async_trait]
    impl MapRepository for MockMapRepository {
        async fn find_unoccupied_valley(
            &self,
            _quadrant: &crate::game::models::map::MapQuadrant,
        ) -> Result<Valley, ApplicationError> {
            Ok(MapField {
                id: 100,
                position: Position { x: 10, y: 10 },
                village_id: None,
                topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
                player_id: None,
            }
            .try_into()
            .unwrap())
        }
        async fn get_field_by_id(&self, _id: i32) -> Result<MapField, ApplicationError> {
            // if let Some(map_field) = self.fields.get(&(id as u32)) {
            //     Ok(map_field.clone())
            // } else {
            //     Err(ApplicationError::Db(DbError::MapFieldNotFound(id as u32)))
            // }
            Ok(MapField {
                id: 100,
                position: Position { x: 10, y: 10 },
                village_id: None,
                topology: MapFieldTopology::Valley(ValleyTopology(4, 4, 4, 6)),
                player_id: None,
            })
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

    pub fn assert_handler_success(result: Result<(), ApplicationError>) {
        assert!(
            result.is_ok(),
            "Handler should execute successfully: {:?}",
            result.err().unwrap().to_string()
        )
    }
}
