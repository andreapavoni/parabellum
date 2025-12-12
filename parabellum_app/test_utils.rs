#[cfg(any(test, feature = "test-utils"))]
#[cfg(not(tarpaulin_include))]
pub mod tests {
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    use parabellum_game::models::{
        army::Army,
        hero::Hero,
        map::{MapField, MapFieldTopology, MapQuadrant, Valley},
        marketplace::MarketplaceOffer,
        village::Village,
    };
    use parabellum_types::{
        common::{Player, ResourceGroup, User},
        errors::{ApplicationError, DbError},
        map::{Position, ValleyTopology},
    };

    use crate::{
        jobs::{
            Job,
            tasks::{AttackTask, ReinforcementTask},
        },
        repository::{
            ArmyRepository, HeroRepository, JobRepository, MapRegionTile, MapRepository,
            MarketplaceRepository, NewReport, PlayerLeaderboardEntry, PlayerRepository,
            ReportAudience, ReportRecord, ReportRepository, UserRepository, VillageInfo,
            VillageRepository,
        },
        uow::{UnitOfWork, UnitOfWorkProvider},
    };

    #[derive(Default, Clone)]
    pub struct MockJobRepository {
        added_jobs: Arc<Mutex<Vec<Job>>>,
    }

    impl MockJobRepository {
        pub fn new() -> Self {
            Self {
                added_jobs: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl JobRepository for MockJobRepository {
        async fn add(&self, job: &Job) -> Result<(), ApplicationError> {
            self.added_jobs.lock().unwrap().push(job.clone());
            Ok(())
        }

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

        async fn list_active_jobs_by_village(
            &self,
            village_id: i32,
        ) -> Result<Vec<Job>, ApplicationError> {
            Ok(self
                .added_jobs
                .lock()
                .unwrap()
                .iter()
                .filter(|job| job.village_id == village_id)
                .cloned()
                .collect())
        }

        async fn list_village_targeting_movements(
            &self,
            village_id: i32,
        ) -> Result<Vec<Job>, ApplicationError> {
            let jobs = self.added_jobs.lock().unwrap();
            let mut matches = Vec::new();
            for job in jobs.iter() {
                match job.task.task_type.as_str() {
                    "Attack" => {
                        if let Ok(payload) =
                            serde_json::from_value::<AttackTask>(job.task.data.clone())
                        {
                            if payload.target_village_id == village_id {
                                matches.push(job.clone());
                            }
                        }
                    }
                    "Reinforcement" => {
                        if let Ok(payload) =
                            serde_json::from_value::<ReinforcementTask>(job.task.data.clone())
                        {
                            if payload.village_id == village_id {
                                matches.push(job.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(matches)
        }

        async fn find_and_lock_due_jobs(&self, _limit: i64) -> Result<Vec<Job>, ApplicationError> {
            Ok(self.added_jobs.lock().unwrap().clone())
        }

        async fn list_village_building_queue(
            &self,
            village_id: i32,
        ) -> Result<Vec<Job>, ApplicationError> {
            Ok(self
                .added_jobs
                .lock()
                .unwrap()
                .iter()
                .filter(|job| job.village_id == village_id)
                .cloned()
                .collect())
        }

        async fn list_village_training_queue(
            &self,
            village_id: i32,
        ) -> Result<Vec<Job>, ApplicationError> {
            Ok(self
                .added_jobs
                .lock()
                .unwrap()
                .iter()
                .filter(|job| job.village_id == village_id)
                .cloned()
                .collect())
        }

        async fn list_village_academy_queue(
            &self,
            village_id: i32,
        ) -> Result<Vec<Job>, ApplicationError> {
            Ok(self
                .added_jobs
                .lock()
                .unwrap()
                .iter()
                .filter(|job| job.village_id == village_id)
                .cloned()
                .collect())
        }

        async fn list_village_smithy_queue(
            &self,
            village_id: i32,
        ) -> Result<Vec<Job>, ApplicationError> {
            Ok(self
                .added_jobs
                .lock()
                .unwrap()
                .iter()
                .filter(|job| job.village_id == village_id)
                .cloned()
                .collect())
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
    pub struct MockReportRepository {
        reports: Arc<Mutex<Vec<(ReportRecord, Vec<ReportAudience>)>>>,
    }

    #[async_trait]
    impl ReportRepository for MockReportRepository {
        async fn add(
            &self,
            report: &NewReport,
            audiences: &[ReportAudience],
        ) -> Result<(), ApplicationError> {
            let mut store = self.reports.lock().unwrap();
            let record = ReportRecord {
                id: Uuid::new_v4(),
                report_type: report.report_type.clone(),
                payload: report.payload.clone(),
                actor_player_id: report.actor_player_id,
                actor_village_id: report.actor_village_id,
                target_player_id: report.target_player_id,
                target_village_id: report.target_village_id,
                created_at: Utc::now(),
                read_at: None,
            };
            store.push((record, audiences.to_vec()));
            Ok(())
        }

        async fn list_for_player(
            &self,
            player_id: Uuid,
            limit: i64,
        ) -> Result<Vec<ReportRecord>, ApplicationError> {
            let store = self.reports.lock().unwrap();
            let mut results = Vec::new();
            for (record, audiences) in store.iter() {
                if let Some(audience) = audiences.iter().find(|a| a.player_id == player_id) {
                    let mut cloned = record.clone();
                    cloned.read_at = audience.read_at;
                    results.push(cloned);
                }
            }
            results.sort_by_key(|r| r.created_at);
            results.reverse();
            results.truncate(limit as usize);
            Ok(results)
        }

        async fn get_for_player(
            &self,
            report_id: Uuid,
            player_id: Uuid,
        ) -> Result<Option<ReportRecord>, ApplicationError> {
            let store = self.reports.lock().unwrap();
            for (record, audiences) in store.iter() {
                if record.id == report_id {
                    if let Some(audience) = audiences.iter().find(|a| a.player_id == player_id) {
                        let mut cloned = record.clone();
                        cloned.read_at = audience.read_at;
                        return Ok(Some(cloned));
                    }
                }
            }
            Ok(None)
        }

        async fn mark_as_read(
            &self,
            report_id: Uuid,
            player_id: Uuid,
        ) -> Result<(), ApplicationError> {
            let mut store = self.reports.lock().unwrap();
            for (record, audiences) in store.iter_mut() {
                if record.id == report_id {
                    if let Some(audience) = audiences.iter_mut().find(|a| a.player_id == player_id)
                    {
                        if audience.read_at.is_none() {
                            audience.read_at = Some(Utc::now());
                        }
                    }
                }
            }
            Ok(())
        }
    }

    /// Helper to force a village to have the exact resource amounts requested.
    pub fn set_village_resources(village: &mut Village, resources: ResourceGroup) {
        let current = village.stored_resources();
        if current.total() > 0 {
            village
                .deduct_resources(&current)
                .expect("failed to clear village resources");
        }
        if resources.total() > 0 {
            village.store_resources(&resources);
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

            for v in self.villages.lock().unwrap().values() {
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

        async fn get_info_by_ids(
            &self,
            _village_ids: &[u32],
        ) -> Result<HashMap<u32, VillageInfo>, ApplicationError> {
            Ok(HashMap::new())
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

        async fn get_by_hero_id(&self, hero_id: Uuid) -> Result<Army, ApplicationError> {
            let armies = self.armies.lock().unwrap();
            Ok(armies
                .get(&hero_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::ArmyNotFound(hero_id)))?)
        }

        async fn set_hero(
            &self,
            army_id: Uuid,
            hero_id: Option<Uuid>,
        ) -> Result<(), ApplicationError> {
            let armies = self.armies.lock().unwrap();
            let mut army = armies
                .get(&army_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::ArmyNotFound(army_id)))?;

            if let Some(id) = hero_id {
                let hero = Hero::new(
                    Some(id),
                    army.village_id,
                    army.player_id,
                    army.tribe.clone(),
                    None,
                );
                army.set_hero(Some(hero));
            } else {
                army.set_hero(None);
            }

            Ok(())
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

        async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError> {
            if let Some(player) = self
                .players
                .lock()
                .unwrap()
                .values()
                .find(|p| p.user_id == user_id)
            {
                Ok(player.clone())
            } else {
                Err(ApplicationError::Db(DbError::UserPlayerNotFound(user_id)))
            }
        }

        async fn leaderboard_page(
            &self,
            offset: i64,
            limit: i64,
        ) -> Result<(Vec<PlayerLeaderboardEntry>, i64), ApplicationError> {
            // Build a predictable, deterministic ordering for mock data.
            let mut entries: Vec<PlayerLeaderboardEntry> = self
                .players
                .lock()
                .unwrap()
                .values()
                .map(|player| PlayerLeaderboardEntry {
                    player_id: player.id,
                    username: player.username.clone(),
                    village_count: 0,
                    population: 0,
                })
                .collect();

            entries.sort_by(|a, b| {
                b.population
                    .cmp(&a.population)
                    .then_with(|| b.village_count.cmp(&a.village_count))
                    .then_with(|| a.username.cmp(&b.username))
            });

            let total = entries.len() as i64;
            let start = offset.max(0) as usize;
            let end = (start + limit as usize).min(entries.len());
            let page_entries = if start >= entries.len() {
                Vec::new()
            } else {
                entries[start..end].to_vec()
            };

            Ok((page_entries, total))
        }
    }

    #[derive(Default, Clone)]
    pub struct MockMapRepository {
        fields: Arc<Mutex<HashMap<u32, MapField>>>,
    }

    impl MockMapRepository {
        pub fn add_map_field(&mut self, field: MapField) {
            let mut fields = self.fields.lock().unwrap();
            fields.insert(field.id, field);
        }
    }

    #[async_trait]
    impl MapRepository for MockMapRepository {
        async fn find_unoccupied_valley(
            &self,
            _quadrant: &MapQuadrant,
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

        async fn get_region(
            &self,
            center_x: i32,
            center_y: i32,
            radius: i32,
            world_size: i32,
        ) -> Result<Vec<MapRegionTile>, ApplicationError> {
            let fields = self.fields.lock().unwrap();
            let mut region = Vec::new();

            for y in ((center_y - radius)..=(center_y + radius)).rev() {
                let wrapped_y = wrap_coordinate(y, world_size);
                for x in center_x - radius..=center_x + radius {
                    let wrapped_x = wrap_coordinate(x, world_size);
                    let position = Position {
                        x: wrapped_x,
                        y: wrapped_y,
                    };
                    let id = position.to_id(world_size) as u32;
                    if let Some(field) = fields.get(&id) {
                        region.push(MapRegionTile {
                            field: field.clone(),
                            village_name: None,
                            village_population: None,
                            player_name: None,
                        });
                    }
                }
            }

            Ok(region)
        }
    }

    fn wrap_coordinate(value: i32, world_size: i32) -> i32 {
        if world_size <= 0 {
            return value;
        }
        let span = world_size * 2 + 1;
        let mut normalized = (value + world_size) % span;
        if normalized < 0 {
            normalized += span;
        }
        normalized - world_size
    }

    #[derive(Default, Clone)]
    pub struct MockMarketplaceRepository {
        offers: Arc<Mutex<HashMap<Uuid, MarketplaceOffer>>>,
    }

    impl MockMarketplaceRepository {}

    #[async_trait]
    impl MarketplaceRepository for MockMarketplaceRepository {
        async fn get_by_id(&self, offer_id: Uuid) -> Result<MarketplaceOffer, ApplicationError> {
            let offers = self.offers.lock().unwrap();
            Ok(offers
                .get(&offer_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::MarketplaceOfferNotFound(offer_id)))?)
        }

        async fn list_by_village(
            &self,
            village_id: u32,
        ) -> Result<Vec<MarketplaceOffer>, ApplicationError> {
            let offers = self.offers.lock().unwrap();
            let by_village = offers
                .values()
                .filter(|&o| o.village_id == village_id)
                .cloned()
                .collect();

            Ok(by_village)
        }

        async fn create(&self, offer: &MarketplaceOffer) -> Result<(), ApplicationError> {
            let mut offers = self.offers.lock().unwrap();
            offers.insert(offer.id, offer.clone());
            Ok(())
        }

        async fn delete(&self, offer_id: Uuid) -> Result<(), ApplicationError> {
            let mut offers = self.offers.lock().unwrap();
            offers.remove(&offer_id);
            Ok(())
        }
        async fn list_all(&self) -> Result<Vec<MarketplaceOffer>, ApplicationError> {
            let offers = self.offers.lock().unwrap();
            Ok(offers.values().cloned().collect())
        }
    }

    #[derive(Default, Clone)]
    pub struct MockHeroRepository {
        heroes: Arc<Mutex<HashMap<Uuid, Hero>>>,
    }

    impl MockHeroRepository {
        pub fn new() -> Self {
            Self {
                heroes: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl HeroRepository for MockHeroRepository {
        async fn save(&self, hero: &Hero) -> Result<(), ApplicationError> {
            self.heroes.lock().unwrap().insert(hero.id, hero.clone());
            Ok(())
        }

        async fn get_by_id(&self, id: Uuid) -> Result<Hero, ApplicationError> {
            if let Some(h) = self.heroes.lock().unwrap().get(&id) {
                return Ok(h.clone());
            }
            Err(ApplicationError::Db(DbError::HeroNotFound(id)))
        }
    }

    #[derive(Default, Clone)]
    pub struct MockUserRepository {
        users: Arc<Mutex<HashMap<Uuid, User>>>,
    }

    impl MockUserRepository {
        pub fn new() -> Self {
            Self {
                users: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn save(
            &self,
            email: String,
            _password_hash: String,
        ) -> Result<(), ApplicationError> {
            let user = User::new(Uuid::new_v4(), email, "123".to_string());
            self.users.lock().unwrap().insert(user.id, user.clone());
            Ok(())
        }

        async fn get_by_email(&self, email: &str) -> Result<User, ApplicationError> {
            if let Some(user) = self
                .users
                .lock()
                .unwrap()
                .values()
                .into_iter()
                .find(|&u| u.email == email)
            {
                return Ok(user.clone());
            }

            Err(ApplicationError::Db(DbError::UserByEmailNotFound(
                email.to_string(),
            )))
        }

        async fn get_by_id(&self, id: Uuid) -> Result<User, ApplicationError> {
            if let Some(h) = self.users.lock().unwrap().get(&id) {
                return Ok(h.clone());
            }
            Err(ApplicationError::Db(DbError::UserByIdNotFound(id)))
        }
    }

    pub struct MockUnitOfWork {
        players: Arc<MockPlayerRepository>,
        villages: Arc<MockVillageRepository>,
        armies: Arc<MockArmyRepository>,
        jobs: Arc<MockJobRepository>,
        reports: Arc<MockReportRepository>,
        map: Arc<MockMapRepository>,
        marketplace: Arc<MockMarketplaceRepository>,
        heroes: Arc<MockHeroRepository>,
        users: Arc<MockUserRepository>,

        // Flags to check if commit/rollback was called
        committed: Arc<Mutex<bool>>,
        rolled_back: Arc<Mutex<bool>>,
    }

    impl MockUnitOfWork {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn report_repo(&self) -> Arc<MockReportRepository> {
            self.reports.clone()
        }
    }

    impl Default for MockUnitOfWork {
        fn default() -> Self {
            Self {
                players: Arc::new(MockPlayerRepository::default()),
                villages: Arc::new(MockVillageRepository::default()),
                armies: Arc::new(MockArmyRepository::default()),
                jobs: Arc::new(MockJobRepository::default()),
                reports: Arc::new(MockReportRepository::default()),
                map: Arc::new(MockMapRepository::default()),
                marketplace: Arc::new(MockMarketplaceRepository::default()),
                heroes: Arc::new(MockHeroRepository::default()),
                users: Arc::new(MockUserRepository::default()),
                committed: Arc::new(Mutex::new(false)),
                rolled_back: Arc::new(Mutex::new(false)),
            }
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
        fn reports(&self) -> Arc<dyn ReportRepository + 'a> {
            self.reports.clone()
        }
        fn map(&self) -> Arc<dyn MapRepository + 'a> {
            self.map.clone()
        }

        fn marketplace(&self) -> Arc<dyn MarketplaceRepository + 'a> {
            self.marketplace.clone()
        }

        fn heroes(&self) -> Arc<dyn HeroRepository + 'a> {
            self.heroes.clone()
        }

        fn users(&self) -> Arc<dyn UserRepository + 'a> {
            self.users.clone()
        }

        async fn commit(self: Box<Self>) -> Result<(), ApplicationError> {
            *self.committed.lock().unwrap() = true;
            Ok(())
        }

        async fn rollback(self: Box<Self>) -> Result<(), ApplicationError> {
            *self.rolled_back.lock().unwrap() = true;
            Ok(())
        }
    }

    pub struct MockUnitOfWorkProvider {}

    impl MockUnitOfWorkProvider {
        pub fn new() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl UnitOfWorkProvider for MockUnitOfWorkProvider {
        async fn tx<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError> {
            let uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
            Ok(uow)
        }
    }
}
