//! Player registration use cases.
//!
//! Registration is a cross-context workflow: it creates identity rows, reserves
//! an initial map valley, founds the initial village, creates the initial hero,
//! and optionally applies deterministic seed resources. The database adapter
//! still owns the identity/map transaction, while this service owns the
//! application ordering and command planning.

use std::sync::Arc;

use parabellum_game::models::{
    buildings::Building,
    village::{Village, VillageBuilding},
};
use parabellum_types::{
    buildings::BuildingName,
    common::Player,
    errors::{ApplicationError, DbError},
    map::ValleyTopology,
};

use crate::{
    auth::hash_password,
    identity::{
        ports::{
            InitialVillageCommandExecutor, RegistrationIdentityPort, RegistrationIdentityRecord,
        },
        requests::RegisterPlayerRequest,
    },
    villages::{CreateHero, FoundVillage, IdGenerator, SetVillageResources},
};

/// Runtime settings required to plan an initial village.
#[derive(Debug, Clone, Copy)]
pub struct RegistrationSettings {
    /// Square world size used by initial village construction.
    pub world_size: i32,
    /// Server speed used for default initial village buildings.
    pub server_speed: i8,
}

/// Application service for player registration.
#[derive(Clone)]
pub struct RegistrationUseCases {
    identities: Arc<dyn RegistrationIdentityPort>,
    villages: Arc<dyn InitialVillageCommandExecutor>,
    ids: Arc<dyn IdGenerator>,
    settings: RegistrationSettings,
}

impl RegistrationUseCases {
    /// Creates registration use cases from focused app ports.
    pub fn new(
        identities: Arc<dyn RegistrationIdentityPort>,
        villages: Arc<dyn InitialVillageCommandExecutor>,
        ids: Arc<dyn IdGenerator>,
        settings: RegistrationSettings,
    ) -> Self {
        Self {
            identities,
            villages,
            ids,
            settings,
        }
    }

    /// Registers a player and initializes their first village.
    ///
    /// The identity rows and map reservation are committed before ES village
    /// commands run. If initial village foundation or hero creation fails, this
    /// service asks the identity port to clean up the committed rows and map
    /// reservation. Optional resource override failure preserves the historical
    /// behavior and is returned without cleanup.
    pub async fn register_player(
        &self,
        request: RegisterPlayerRequest,
    ) -> Result<(), ApplicationError> {
        let password_hash = hash_password(&request.password)?;
        let created = self
            .identities
            .create_registration_identity(RegistrationIdentityRecord {
                player_id: request.player_id,
                username: request.username.clone(),
                email: request.email.clone(),
                password_hash,
                tribe: request.tribe.clone(),
                quadrant: request.quadrant.clone(),
            })
            .await?;

        if created.valley.topology != ValleyTopology(4, 4, 4, 6) {
            self.cleanup_after_initialization_failure(
                created.user_id,
                request.player_id,
                created.valley.id,
            )
            .await;
            return Err(ApplicationError::Db(DbError::Transaction(
                "initial village must be founded on a 4-4-4-6 valley".to_string(),
            )));
        }

        let village = self.initial_village(&request, &created.player, &created.valley);
        let server_speed = request
            .initial_village
            .as_ref()
            .and_then(|setup| setup.speed)
            .unwrap_or(self.settings.server_speed);
        let (village_name, buildings) =
            village_setup_from_request(&request, &village, server_speed)?;
        let village_id = village.id;

        let found = FoundVillage {
            village_name,
            position: village.position.clone(),
            tribe: village.tribe.clone(),
            player_id: village.player_id,
            parent_village_id: None,
            buildings,
        };

        if let Err(err) = self.villages.found_initial_village(village_id, found).await {
            self.cleanup_after_initialization_failure(
                created.user_id,
                request.player_id,
                village_id,
            )
            .await;
            return Err(err);
        }

        if let Err(err) = self
            .villages
            .create_initial_hero(
                village_id,
                CreateHero {
                    hero_id: self.ids.next(),
                    player_id: request.player_id,
                    village_id,
                    has_existing_hero: false,
                    bypass_hero_mansion_requirement: true,
                },
            )
            .await
        {
            self.cleanup_after_initialization_failure(
                created.user_id,
                request.player_id,
                village_id,
            )
            .await;
            return Err(err);
        }

        if let Some(resources) = request
            .initial_village
            .as_ref()
            .and_then(|setup| setup.resources.clone())
        {
            self.villages
                .set_initial_village_resources(
                    village_id,
                    SetVillageResources {
                        player_id: request.player_id,
                        resources,
                    },
                )
                .await?;
        }

        Ok(())
    }

    fn initial_village(
        &self,
        request: &RegisterPlayerRequest,
        player: &Player,
        valley: &parabellum_game::models::map::Valley,
    ) -> Village {
        Village::new(
            format!("{}'s Village", request.username),
            valley,
            player,
            true,
            self.settings.world_size,
            self.settings.server_speed,
        )
    }

    async fn cleanup_after_initialization_failure(
        &self,
        user_id: uuid::Uuid,
        player_id: uuid::Uuid,
        village_id: u32,
    ) {
        let _ = self
            .identities
            .cleanup_failed_registration(user_id, player_id, village_id)
            .await;
    }
}

fn village_setup_from_request(
    request: &RegisterPlayerRequest,
    village: &Village,
    speed: i8,
) -> Result<(String, Vec<VillageBuilding>), ApplicationError> {
    let Some(setup) = &request.initial_village else {
        return Ok((village.name.clone(), village.buildings().clone()));
    };

    let village_name = setup
        .village_name
        .clone()
        .unwrap_or_else(|| village.name.clone());
    let mut buildings = village.buildings().clone();

    if setup.resource_fields_target_level > 0 {
        for building in &mut buildings {
            if building.slot_id <= 18 {
                building.building = Building::new(building.building.name.clone(), speed)
                    .at_level(setup.resource_fields_target_level, speed)
                    .map_err(ApplicationError::from)?;
            }
        }
    }

    for override_building in &setup.buildings {
        if override_building.slot_id <= 18 {
            continue;
        }
        let normalized = VillageBuilding {
            slot_id: override_building.slot_id,
            building: Building::new(override_building.building.name.clone(), speed)
                .at_level(override_building.building.level, speed)
                .map_err(ApplicationError::from)?,
        };
        upsert_building(&mut buildings, normalized);
    }

    ensure_rally_point_minimum(&mut buildings, speed)?;
    normalize_buildings_by_slot(&mut buildings);
    Ok((village_name, buildings))
}

fn upsert_building(buildings: &mut Vec<VillageBuilding>, building: VillageBuilding) {
    if let Some(existing) = buildings.iter_mut().find(|b| b.slot_id == building.slot_id) {
        *existing = building;
        return;
    }
    buildings.push(building);
}

fn ensure_rally_point_minimum(
    buildings: &mut Vec<VillageBuilding>,
    speed: i8,
) -> Result<(), ApplicationError> {
    if buildings.iter().any(|b| b.slot_id == 39) {
        return Ok(());
    }
    let rally = Building::new(BuildingName::RallyPoint, speed)
        .at_level(1, speed)
        .map_err(ApplicationError::from)?;
    buildings.push(VillageBuilding {
        slot_id: 39,
        building: rally,
    });
    Ok(())
}

fn normalize_buildings_by_slot(buildings: &mut Vec<VillageBuilding>) {
    let mut normalized = Vec::with_capacity(buildings.len());
    for building in buildings.drain(..) {
        if let Some(existing) = normalized
            .iter_mut()
            .find(|b: &&mut VillageBuilding| b.slot_id == building.slot_id)
        {
            *existing = building;
        } else {
            normalized.push(building);
        }
    }
    *buildings = normalized;
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use parabellum_game::models::map::{MapQuadrant, Valley};
    use parabellum_types::{
        common::{Player, ResourceGroup},
        errors::ApplicationError,
        map::{Position, ValleyTopology},
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::{
        identity::{
            ports::{
                CreatedRegistrationIdentity, InitialVillageCommandExecutor,
                RegistrationIdentityPort, RegistrationIdentityRecord,
            },
            requests::{InitialVillageSetup, RegisterPlayerRequest},
        },
        villages::{CreateHero, FoundVillage, IdGenerator, SetVillageResources},
    };

    use super::{RegistrationSettings, RegistrationUseCases};

    #[derive(Default)]
    struct FakeRegistrationIdentities {
        created: Mutex<Option<CreatedRegistrationIdentity>>,
        records: Mutex<Vec<RegistrationIdentityRecord>>,
        cleanups: Mutex<Vec<(Uuid, Uuid, u32)>>,
    }

    #[async_trait]
    impl RegistrationIdentityPort for FakeRegistrationIdentities {
        async fn create_registration_identity(
            &self,
            record: RegistrationIdentityRecord,
        ) -> Result<CreatedRegistrationIdentity, ApplicationError> {
            self.records
                .lock()
                .expect("records lock should not be poisoned")
                .push(record);
            self.created
                .lock()
                .expect("created lock should not be poisoned")
                .clone()
                .ok_or_else(|| ApplicationError::Unknown("missing created identity".to_string()))
        }

        async fn cleanup_failed_registration(
            &self,
            user_id: Uuid,
            player_id: Uuid,
            village_id: u32,
        ) -> Result<(), ApplicationError> {
            self.cleanups
                .lock()
                .expect("cleanups lock should not be poisoned")
                .push((user_id, player_id, village_id));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeInitialVillageCommands {
        found: Mutex<Vec<(u32, FoundVillage)>>,
        heroes: Mutex<Vec<(u32, CreateHero)>>,
        resources: Mutex<Vec<(u32, SetVillageResources)>>,
        fail_hero: Mutex<bool>,
    }

    #[async_trait]
    impl InitialVillageCommandExecutor for FakeInitialVillageCommands {
        async fn found_initial_village(
            &self,
            village_id: u32,
            command: FoundVillage,
        ) -> Result<(), ApplicationError> {
            self.found
                .lock()
                .expect("found lock should not be poisoned")
                .push((village_id, command));
            Ok(())
        }

        async fn create_initial_hero(
            &self,
            village_id: u32,
            command: CreateHero,
        ) -> Result<(), ApplicationError> {
            if *self
                .fail_hero
                .lock()
                .expect("fail_hero lock should not be poisoned")
            {
                return Err(ApplicationError::Unknown("hero failed".to_string()));
            }
            self.heroes
                .lock()
                .expect("heroes lock should not be poisoned")
                .push((village_id, command));
            Ok(())
        }

        async fn set_initial_village_resources(
            &self,
            village_id: u32,
            command: SetVillageResources,
        ) -> Result<(), ApplicationError> {
            self.resources
                .lock()
                .expect("resources lock should not be poisoned")
                .push((village_id, command));
            Ok(())
        }
    }

    struct FixedIds(Uuid);

    impl IdGenerator for FixedIds {
        fn next(&self) -> Uuid {
            self.0
        }
    }

    fn player(player_id: Uuid, user_id: Uuid) -> Player {
        Player {
            id: player_id,
            username: "andrea".to_string(),
            tribe: Tribe::Roman,
            user_id,
            culture_points: 0,
        }
    }

    fn starting_village_id() -> u32 {
        Position { x: 0, y: 0 }.to_id(100)
    }

    fn valley(topology: ValleyTopology) -> Valley {
        let position = Position { x: 0, y: 0 };
        Valley {
            id: starting_village_id(),
            position,
            topology,
            player_id: None,
            village_id: None,
        }
    }

    fn request(player_id: Uuid) -> RegisterPlayerRequest {
        RegisterPlayerRequest {
            player_id,
            username: "andrea".to_string(),
            email: "andrea@example.com".to_string(),
            password: "secret123".to_string(),
            tribe: Tribe::Roman,
            quadrant: MapQuadrant::NorthEast,
            initial_village: None,
        }
    }

    fn use_cases(
        identities: Arc<FakeRegistrationIdentities>,
        villages: Arc<FakeInitialVillageCommands>,
        hero_id: Uuid,
    ) -> RegistrationUseCases {
        RegistrationUseCases::new(
            identities,
            villages,
            Arc::new(FixedIds(hero_id)),
            RegistrationSettings {
                world_size: 100,
                server_speed: 1,
            },
        )
    }

    #[tokio::test]
    async fn register_player_creates_identity_then_initial_village_and_hero() {
        let user_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let identities = Arc::new(FakeRegistrationIdentities::default());
        *identities.created.lock().unwrap() = Some(CreatedRegistrationIdentity {
            user_id,
            player: player(player_id, user_id),
            valley: valley(ValleyTopology(4, 4, 4, 6)),
        });
        let villages = Arc::new(FakeInitialVillageCommands::default());

        use_cases(identities.clone(), villages.clone(), hero_id)
            .register_player(request(player_id))
            .await
            .unwrap();

        let records = identities.records.lock().unwrap();
        assert_eq!(records[0].player_id, player_id);
        assert_eq!(records[0].username, "andrea");
        assert_ne!(records[0].password_hash, "secret123");

        let found = villages.found.lock().unwrap();
        assert_eq!(found[0].0, starting_village_id());
        assert_eq!(found[0].1.village_name, "andrea's Village");
        assert_eq!(found[0].1.player_id, player_id);
        assert_eq!(found[0].1.parent_village_id, None);

        let heroes = villages.heroes.lock().unwrap();
        assert_eq!(heroes[0].0, starting_village_id());
        assert_eq!(heroes[0].1.hero_id, hero_id);
        assert!(heroes[0].1.bypass_hero_mansion_requirement);
        assert!(identities.cleanups.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn register_player_applies_optional_initial_resources() {
        let user_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let identities = Arc::new(FakeRegistrationIdentities::default());
        *identities.created.lock().unwrap() = Some(CreatedRegistrationIdentity {
            user_id,
            player: player(player_id, user_id),
            valley: valley(ValleyTopology(4, 4, 4, 6)),
        });
        let villages = Arc::new(FakeInitialVillageCommands::default());
        let mut req = request(player_id);
        req.initial_village = Some(InitialVillageSetup {
            village_name: Some("Seed Village".to_string()),
            resource_fields_target_level: 0,
            buildings: vec![],
            resources: Some(ResourceGroup::new(100, 200, 300, 400)),
            speed: None,
        });

        use_cases(identities, villages.clone(), Uuid::new_v4())
            .register_player(req)
            .await
            .unwrap();

        let found = villages.found.lock().unwrap();
        assert_eq!(found[0].1.village_name, "Seed Village");
        let resources = villages.resources.lock().unwrap();
        assert_eq!(resources[0].0, starting_village_id());
        assert_eq!(resources[0].1.player_id, player_id);
        assert_eq!(
            resources[0].1.resources,
            ResourceGroup::new(100, 200, 300, 400)
        );
    }

    #[tokio::test]
    async fn register_player_cleans_up_identity_when_initial_hero_creation_fails() {
        let user_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let identities = Arc::new(FakeRegistrationIdentities::default());
        *identities.created.lock().unwrap() = Some(CreatedRegistrationIdentity {
            user_id,
            player: player(player_id, user_id),
            valley: valley(ValleyTopology(4, 4, 4, 6)),
        });
        let villages = Arc::new(FakeInitialVillageCommands::default());
        *villages.fail_hero.lock().unwrap() = true;

        let result = use_cases(identities.clone(), villages, Uuid::new_v4())
            .register_player(request(player_id))
            .await;

        assert!(result.is_err());
        assert_eq!(
            identities.cleanups.lock().unwrap().as_slice(),
            &[(user_id, player_id, starting_village_id())]
        );
    }
}
