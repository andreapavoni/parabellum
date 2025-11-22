#[cfg(any(test, feature = "test-utils"))]
#[cfg(not(tarpaulin_include))]
pub mod tests {
    use async_trait::async_trait;
    use parabellum_game::models::{
        alliance::{Alliance, AllianceInvite, AllianceLog, AllianceDiplomacy},
        army::Army,
        hero::Hero,
        map::{MapField, MapFieldTopology, MapQuadrant, Valley},
        map_flag::MapFlag,
        marketplace::MarketplaceOffer,
        player::Player,
        village::Village,
    };
    use parabellum_types::{
        common::User,
        map::{Position, ValleyTopology},
        map_flag::MapFlagType,
    };
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    use parabellum_core::{ApplicationError, DbError};

    use crate::{
        jobs::Job,
        repository::{
            AllianceRepository, AllianceInviteRepository, AllianceLogRepository, AllianceDiplomacyRepository,
            ArmyRepository, HeroRepository, JobRepository, MapRepository, MapFlagRepository, MarketplaceRepository,
            PlayerRepository, UserRepository, VillageRepository,
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

            for v in self.villages.lock().unwrap().values() {
                if v.player_id == player_id {
                    villages.push(v.clone());
                }
            }

            Ok(villages)
        }

        async fn get_capital_by_player_id(&self, player_id: Uuid) -> Result<Village, ApplicationError> {
            let villages = self.villages.lock().unwrap();
            villages
                .values()
                .find(|v| v.player_id == player_id && v.is_capital)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::CapitalVillageNotFound(player_id)))
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

        async fn get_by_email(&self, email: &String) -> Result<User, ApplicationError> {
            if let Some(user) = self
                .users
                .lock()
                .unwrap()
                .values()
                .into_iter()
                .find(|&u| &u.email == email)
            {
                return Ok(user.clone());
            }

            Err(ApplicationError::Db(DbError::UserByEmailNotFound(email.clone())))
        }

        async fn get_by_id(&self, id: Uuid) -> Result<User, ApplicationError> {
            if let Some(h) = self.users.lock().unwrap().get(&id) {
                return Ok(h.clone());
            }
            Err(ApplicationError::Db(DbError::UserByIdNotFound(id)))
        }
    }

    #[derive(Default, Clone)]
    pub struct MockAllianceRepository {
        alliances: Arc<Mutex<HashMap<Uuid, Alliance>>>,
        players: Arc<Mutex<HashMap<Uuid, Player>>>, // Store player data for tests
    }

    impl MockAllianceRepository {
        pub fn new() -> Self {
            Self {
                alliances: Arc::new(Mutex::new(HashMap::new())),
                players: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        pub fn add_player(&self, player: Player) {
            self.players.lock().unwrap().insert(player.id, player);
        }
    }

    #[async_trait]
    impl AllianceRepository for MockAllianceRepository {
        async fn save(&self, alliance: &Alliance) -> Result<(), ApplicationError> {
            self.alliances.lock().unwrap().insert(alliance.id, alliance.clone());
            Ok(())
        }

        async fn get_by_id(&self, id: Uuid) -> Result<Alliance, ApplicationError> {
            self.alliances
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::AllianceNotFound(id)))
        }

        async fn get_by_tag(&self, tag: String) -> Result<Alliance, ApplicationError> {
            self.alliances
                .lock()
                .unwrap()
                .values()
                .find(|a| a.tag == tag)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::AllianceByTagNotFound(tag)))
        }

        async fn get_by_name(&self, name: String) -> Result<Alliance, ApplicationError> {
            self.alliances
                .lock()
                .unwrap()
                .values()
                .find(|a| a.name == name)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::AllianceByNameNotFound(name)))
        }

        async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
            self.alliances.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn update(&self, alliance: &Alliance) -> Result<(), ApplicationError> {
            self.alliances.lock().unwrap().insert(alliance.id, alliance.clone());
            Ok(())
        }

        async fn get_leader(&self, alliance_id: Uuid) -> Result<Player, ApplicationError> {
            let alliance = self.get_by_id(alliance_id).await?;
            let leader_id = alliance.leader_id
                .ok_or_else(|| ApplicationError::Db(DbError::AllianceLeaderNotFound(alliance_id)))?;

            self.players
                .lock()
                .unwrap()
                .get(&leader_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::PlayerNotFound(leader_id)))
        }

        async fn count_members(&self, alliance_id: Uuid) -> Result<i64, ApplicationError> {
            let count = self.players
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.alliance_id == Some(alliance_id))
                .count() as i64;
            Ok(count)
        }

        async fn list_members(&self, alliance_id: Uuid) -> Result<Vec<Player>, ApplicationError> {
            let members = self.players
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.alliance_id == Some(alliance_id))
                .cloned()
                .collect();
            Ok(members)
        }
    }

    #[derive(Default, Clone)]
    pub struct MockAllianceInviteRepository {
        invites: Arc<Mutex<HashMap<Uuid, AllianceInvite>>>,
    }

    impl MockAllianceInviteRepository {
        pub fn new() -> Self {
            Self {
                invites: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl AllianceInviteRepository for MockAllianceInviteRepository {
        async fn save(&self, invite: &AllianceInvite) -> Result<(), ApplicationError> {
            self.invites.lock().unwrap().insert(invite.id, invite.clone());
            Ok(())
        }

        async fn get_by_id(&self, id: Uuid) -> Result<AllianceInvite, ApplicationError> {
            self.invites
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::AllianceInviteNotFound(id)))
        }

        async fn get_by_player_id(&self, player_id: Uuid) -> Result<Vec<AllianceInvite>, ApplicationError> {
            let invites = self.invites
                .lock()
                .unwrap()
                .values()
                .filter(|i| i.to_player_id == player_id)
                .cloned()
                .collect();
            Ok(invites)
        }

        async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<AllianceInvite>, ApplicationError> {
            let invites = self.invites
                .lock()
                .unwrap()
                .values()
                .filter(|i| i.alliance_id == alliance_id)
                .cloned()
                .collect();
            Ok(invites)
        }

        async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
            self.invites.lock().unwrap().remove(&id);
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    pub struct MockAllianceLogRepository {
        logs: Arc<Mutex<Vec<AllianceLog>>>,
    }

    impl MockAllianceLogRepository {
        pub fn new() -> Self {
            Self {
                logs: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl AllianceLogRepository for MockAllianceLogRepository {
        async fn save(&self, log: &AllianceLog) -> Result<(), ApplicationError> {
            self.logs.lock().unwrap().push(log.clone());
            Ok(())
        }

        async fn get_by_alliance_id(&self, alliance_id: Uuid, _limit: i32, _offset: i32) -> Result<Vec<AllianceLog>, ApplicationError> {
            let logs = self.logs
                .lock()
                .unwrap()
                .iter()
                .filter(|l| l.alliance_id == alliance_id)
                .cloned()
                .collect();
            Ok(logs)
        }
    }

    #[derive(Default, Clone)]
    pub struct MockAllianceDiplomacyRepository {
        diplomacy: Arc<Mutex<HashMap<Uuid, AllianceDiplomacy>>>,
    }

    impl MockAllianceDiplomacyRepository {
        pub fn new() -> Self {
            Self {
                diplomacy: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl AllianceDiplomacyRepository for MockAllianceDiplomacyRepository {
        async fn save(&self, diplomacy: &AllianceDiplomacy) -> Result<(), ApplicationError> {
            self.diplomacy.lock().unwrap().insert(diplomacy.id, diplomacy.clone());
            Ok(())
        }

        async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<AllianceDiplomacy>, ApplicationError> {
            let result = self.diplomacy
                .lock()
                .unwrap()
                .values()
                .filter(|d| d.alliance1_id == alliance_id || d.alliance2_id == alliance_id)
                .cloned()
                .collect();
            Ok(result)
        }

        async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
            self.diplomacy.lock().unwrap().remove(&id);
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    pub struct MockMapFlagRepository {
        flags: Arc<Mutex<HashMap<Uuid, MapFlag>>>,
    }

    impl MockMapFlagRepository {
        pub fn new() -> Self {
            Self {
                flags: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl MapFlagRepository for MockMapFlagRepository {
        async fn save(&self, flag: &MapFlag) -> Result<(), ApplicationError> {
            self.flags.lock().unwrap().insert(flag.id, flag.clone());
            Ok(())
        }

        async fn get_by_id(&self, id: Uuid) -> Result<MapFlag, ApplicationError> {
            self.flags
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| ApplicationError::Db(DbError::MapFlagNotFound(id)))
        }

        async fn get_by_player_id(&self, player_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError> {
            let result = self.flags
                .lock()
                .unwrap()
                .values()
                .filter(|f| f.player_id == Some(player_id))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError> {
            let result = self.flags
                .lock()
                .unwrap()
                .values()
                .filter(|f| f.alliance_id == Some(alliance_id))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_coordinates(&self, x: i32, y: i32) -> Result<Vec<MapFlag>, ApplicationError> {
            let result = self.flags
                .lock()
                .unwrap()
                .values()
                .filter(|f| f.position.as_ref().map_or(false, |p| p.x == x && p.y == y))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn get_by_target_id(&self, target_id: Uuid) -> Result<Vec<MapFlag>, ApplicationError> {
            let result = self.flags
                .lock()
                .unwrap()
                .values()
                .filter(|f| f.target_id == Some(target_id))
                .cloned()
                .collect();
            Ok(result)
        }

        async fn count_by_owner(
            &self,
            player_id: Option<Uuid>,
            alliance_id: Option<Uuid>,
            flag_type: Option<MapFlagType>,
        ) -> Result<i64, ApplicationError> {
            let count = self.flags
                .lock()
                .unwrap()
                .values()
                .filter(|f| {
                    let owner_match = if let Some(pid) = player_id {
                        f.player_id == Some(pid)
                    } else if let Some(aid) = alliance_id {
                        f.alliance_id == Some(aid)
                    } else {
                        false
                    };

                    let type_match = if let Some(ftype) = flag_type {
                        f.flag_type == ftype
                    } else {
                        true
                    };

                    owner_match && type_match
                })
                .count() as i64;
            Ok(count)
        }

        async fn update(&self, flag: &MapFlag) -> Result<(), ApplicationError> {
            let mut flags = self.flags.lock().unwrap();
            if flags.contains_key(&flag.id) {
                flags.insert(flag.id, flag.clone());
                Ok(())
            } else {
                Err(ApplicationError::Db(DbError::MapFlagNotFound(flag.id)))
            }
        }

        async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
            self.flags.lock().unwrap().remove(&id);
            Ok(())
        }
    }

    #[derive(Default, Clone)]
    pub struct MockUnitOfWork {
        players: Arc<MockPlayerRepository>,
        villages: Arc<MockVillageRepository>,
        armies: Arc<MockArmyRepository>,
        jobs: Arc<MockJobRepository>,
        map: Arc<MockMapRepository>,
        marketplace: Arc<MockMarketplaceRepository>,
        heroes: Arc<MockHeroRepository>,
        users: Arc<MockUserRepository>,
        alliances: Arc<MockAllianceRepository>,
        alliance_invites: Arc<MockAllianceInviteRepository>,
        alliance_logs: Arc<MockAllianceLogRepository>,
        alliance_diplomacy: Arc<MockAllianceDiplomacyRepository>,
        map_flags: Arc<MockMapFlagRepository>,

        // Flags to check if commit/rollback was called
        committed: Arc<Mutex<bool>>,
        rolled_back: Arc<Mutex<bool>>,
    }

    impl MockUnitOfWork {
        pub fn new() -> Self {
            Default::default()
        }

        // Helper method for tests to add a player to alliance member tracking
        pub fn add_alliance_member(&self, player: Player) {
            self.alliances.add_player(player);
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

        fn marketplace(&self) -> Arc<dyn MarketplaceRepository + 'a> {
            self.marketplace.clone()
        }

        fn heroes(&self) -> Arc<dyn HeroRepository + 'a> {
            self.heroes.clone()
        }

        fn users(&self) -> Arc<dyn UserRepository + 'a> {
            self.users.clone()
        }

        fn alliances(&self) -> Arc<dyn AllianceRepository + 'a> {
            self.alliances.clone()
        }

        fn alliance_invites(&self) -> Arc<dyn AllianceInviteRepository + 'a> {
            self.alliance_invites.clone()
        }

        fn alliance_logs(&self) -> Arc<dyn AllianceLogRepository + 'a> {
            self.alliance_logs.clone()
        }

        fn alliance_diplomacy(&self) -> Arc<dyn AllianceDiplomacyRepository + 'a> {
            self.alliance_diplomacy.clone()
        }

        fn map_flags(&self) -> Arc<dyn MapFlagRepository + 'a> {
            self.map_flags.clone()
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
        async fn begin<'p>(&'p self) -> Result<Box<dyn UnitOfWork<'p> + 'p>, ApplicationError> {
            let uow: Box<dyn UnitOfWork<'_> + '_> = Box::new(MockUnitOfWork::new());
            Ok(uow)
        }
    }
}
