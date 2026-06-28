//! Hero use cases.
//!
//! This service owns app-level orchestration for hero lifecycle and hero
//! profile updates. Hero domain validation remains in aggregate command
//! handlers; this layer loads current hero context, applies runtime settings,
//! and delegates command execution through app ports.

use std::sync::Arc;

use parabellum_types::errors::{ApplicationError, GameError};

use crate::villages::{
    AssignHeroPoints, CreateHero, ResetHeroPoints, ReviveHero, SetHeroResourceFocus,
    ports::{Clock, HeroCommandExecutor, HeroCommandIntent, HeroReadPort, IdGenerator},
    requests::heroes::{
        AssignHeroPointsRequest, CreateHeroRequest, GetHeroByPlayerRequest,
        GetPendingHeroRevivalRequest, ResetHeroPointsRequest, ReviveHeroRequest,
        SetHeroResourceFocusRequest,
    },
};

/// Runtime settings used by hero use cases.
#[derive(Debug, Clone, Copy)]
pub struct HeroSettings {
    /// Server speed multiplier used by hero revival cost/time.
    pub server_speed: i8,
}

/// Application service for hero operations.
#[derive(Clone)]
pub struct HeroUseCases {
    reads: Arc<dyn HeroReadPort>,
    executor: Arc<dyn HeroCommandExecutor>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    settings: HeroSettings,
}

impl HeroUseCases {
    /// Creates hero use cases from focused ports and settings.
    pub fn new(
        reads: Arc<dyn HeroReadPort>,
        executor: Arc<dyn HeroCommandExecutor>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        settings: HeroSettings,
    ) -> Self {
        Self {
            reads,
            executor,
            clock,
            ids,
            settings,
        }
    }

    /// Loads the current hero for a player, if any.
    pub async fn get_hero_by_player(
        &self,
        request: GetHeroByPlayerRequest,
    ) -> Result<Option<parabellum_game::models::hero::Hero>, ApplicationError> {
        self.reads.get_hero_by_player(request.player_id).await
    }

    /// Loads the pending hero revival completion timestamp for a player.
    pub async fn get_pending_hero_revival_at(
        &self,
        request: GetPendingHeroRevivalRequest,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, ApplicationError> {
        self.reads
            .get_pending_hero_revival_at(request.player_id)
            .await
    }

    /// Creates a hero in an owned village.
    pub async fn create_hero(&self, request: CreateHeroRequest) -> Result<(), ApplicationError> {
        let has_existing_hero = self.reads.player_has_alive_hero(request.player_id).await?;

        self.executor
            .execute_hero_command(HeroCommandIntent::CreateHero {
                village_id: request.village_id,
                command: CreateHero {
                    hero_id: request.hero_id,
                    player_id: request.player_id,
                    village_id: request.village_id,
                    has_existing_hero,
                    bypass_hero_mansion_requirement: false,
                },
            })
            .await
    }

    /// Queues revival for a dead hero.
    pub async fn revive_hero(&self, request: ReviveHeroRequest) -> Result<(), ApplicationError> {
        let hero = self.reads.get_hero(request.hero_id).await?;
        if self
            .reads
            .get_pending_hero_revival_at(request.player_id)
            .await?
            .is_some()
        {
            return Err(ApplicationError::Game(GameError::HeroRevivalAlreadyPending));
        }
        if self.reads.player_has_alive_hero(request.player_id).await? {
            return Err(ApplicationError::Game(GameError::HeroAlreadyExists));
        }

        let revive_at = self.clock.now()
            + chrono::Duration::seconds(
                hero.resurrection_cost(self.settings.server_speed).time as i64,
            );
        self.executor
            .execute_hero_command(HeroCommandIntent::ReviveHero {
                village_id: request.village_id,
                command: ReviveHero {
                    action_id: self.ids.next(),
                    player_id: request.player_id,
                    village_id: request.village_id,
                    hero,
                    reset: request.reset,
                    speed: self.settings.server_speed,
                    revive_at,
                },
            })
            .await
    }

    /// Assigns available hero points.
    pub async fn assign_hero_points(
        &self,
        request: AssignHeroPointsRequest,
    ) -> Result<(), ApplicationError> {
        let hero = self.reads.get_hero(request.hero_id).await?;
        self.executor
            .execute_hero_command(HeroCommandIntent::AssignHeroPoints {
                village_id: request.village_id,
                command: AssignHeroPoints {
                    player_id: request.player_id,
                    village_id: request.village_id,
                    hero,
                    strength: request.strength,
                    off_bonus: request.off_bonus,
                    def_bonus: request.def_bonus,
                    regeneration: request.regeneration,
                    resources: request.resources,
                },
            })
            .await
    }

    /// Resets level-zero hero points.
    pub async fn reset_hero_points(
        &self,
        request: ResetHeroPointsRequest,
    ) -> Result<(), ApplicationError> {
        let hero = self.reads.get_hero(request.hero_id).await?;
        self.executor
            .execute_hero_command(HeroCommandIntent::ResetHeroPoints {
                village_id: request.village_id,
                command: ResetHeroPoints {
                    player_id: request.player_id,
                    village_id: request.village_id,
                    hero,
                },
            })
            .await
    }

    /// Changes hero resource production focus.
    pub async fn set_hero_resource_focus(
        &self,
        request: SetHeroResourceFocusRequest,
    ) -> Result<(), ApplicationError> {
        let hero = self.reads.get_hero(request.hero_id).await?;
        self.executor
            .execute_hero_command(HeroCommandIntent::SetHeroResourceFocus {
                village_id: request.village_id,
                command: SetHeroResourceFocus {
                    player_id: request.player_id,
                    village_id: request.village_id,
                    hero,
                    focus: request.focus,
                },
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, VecDeque},
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use parabellum_game::models::hero::{Hero, HeroResourceFocus};
    use parabellum_types::{
        errors::{ApplicationError, GameError},
        tribe::Tribe,
    };
    use uuid::Uuid;

    use crate::villages::{
        ports::{Clock, HeroCommandExecutor, HeroCommandIntent, HeroReadPort, IdGenerator},
        requests::heroes::{
            AssignHeroPointsRequest, CreateHeroRequest, GetHeroByPlayerRequest,
            GetPendingHeroRevivalRequest, ResetHeroPointsRequest, ReviveHeroRequest,
            SetHeroResourceFocusRequest,
        },
    };

    use super::{HeroSettings, HeroUseCases};

    #[derive(Clone)]
    struct FixedClock(chrono::DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            self.0
        }
    }

    struct FixedIds {
        ids: Mutex<VecDeque<Uuid>>,
    }

    impl FixedIds {
        fn new(ids: Vec<Uuid>) -> Self {
            Self {
                ids: Mutex::new(ids.into()),
            }
        }
    }

    impl IdGenerator for FixedIds {
        fn next(&self) -> Uuid {
            self.ids
                .lock()
                .expect("id lock should not be poisoned")
                .pop_front()
                .expect("test should provide enough ids")
        }
    }

    #[derive(Default)]
    struct FakeHeroReads {
        heroes: Mutex<HashMap<Uuid, Hero>>,
        player_hero: Mutex<HashMap<Uuid, Option<Hero>>>,
        alive_hero: Mutex<HashMap<Uuid, bool>>,
        pending_revival_at: Mutex<HashMap<Uuid, Option<chrono::DateTime<Utc>>>>,
    }

    #[async_trait]
    impl HeroReadPort for FakeHeroReads {
        async fn get_hero(&self, hero_id: Uuid) -> Result<Hero, ApplicationError> {
            self.heroes
                .lock()
                .expect("hero lock should not be poisoned")
                .get(&hero_id)
                .cloned()
                .ok_or_else(|| ApplicationError::Unknown(format!("missing hero {hero_id}")))
        }

        async fn get_hero_by_player(
            &self,
            player_id: Uuid,
        ) -> Result<Option<Hero>, ApplicationError> {
            Ok(self
                .player_hero
                .lock()
                .expect("player hero lock should not be poisoned")
                .get(&player_id)
                .cloned()
                .unwrap_or(None))
        }

        async fn player_has_alive_hero(&self, player_id: Uuid) -> Result<bool, ApplicationError> {
            Ok(*self
                .alive_hero
                .lock()
                .expect("alive lock should not be poisoned")
                .get(&player_id)
                .unwrap_or(&false))
        }

        async fn get_pending_hero_revival_at(
            &self,
            player_id: Uuid,
        ) -> Result<Option<chrono::DateTime<Utc>>, ApplicationError> {
            Ok(self
                .pending_revival_at
                .lock()
                .expect("pending at lock should not be poisoned")
                .get(&player_id)
                .cloned()
                .unwrap_or(None))
        }
    }

    #[derive(Default)]
    struct FakeHeroExecutor {
        commands: Mutex<VecDeque<HeroCommandIntent>>,
    }

    #[async_trait]
    impl HeroCommandExecutor for FakeHeroExecutor {
        async fn execute_hero_command(
            &self,
            command: HeroCommandIntent,
        ) -> Result<(), ApplicationError> {
            self.commands
                .lock()
                .expect("command lock should not be poisoned")
                .push_back(command);
            Ok(())
        }
    }

    fn hero(hero_id: Uuid, player_id: Uuid, village_id: u32) -> Hero {
        Hero::new(Some(hero_id), village_id, player_id, Tribe::Roman, Some(5))
    }

    fn dead_hero(hero_id: Uuid, player_id: Uuid, village_id: u32) -> Hero {
        let mut hero = hero(hero_id, player_id, village_id);
        hero.apply_battle_damage(1.0);
        hero
    }

    fn use_cases(reads: Arc<FakeHeroReads>, executor: Arc<FakeHeroExecutor>) -> HeroUseCases {
        HeroUseCases::new(
            reads,
            executor,
            Arc::new(FixedClock(
                Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            )),
            Arc::new(FixedIds::new(vec![Uuid::from_u128(42)])),
            HeroSettings { server_speed: 2 },
        )
    }

    #[tokio::test]
    async fn hero_queries_delegate_to_read_port() {
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let pending_at = Utc.with_ymd_and_hms(2026, 1, 2, 12, 0, 0).unwrap();
        let reads = Arc::new(FakeHeroReads::default());
        reads
            .player_hero
            .lock()
            .unwrap()
            .insert(player_id, Some(hero(hero_id, player_id, 1)));
        reads
            .pending_revival_at
            .lock()
            .unwrap()
            .insert(player_id, Some(pending_at));
        let executor = Arc::new(FakeHeroExecutor::default());
        let use_cases = use_cases(reads, executor);

        let loaded_hero = use_cases
            .get_hero_by_player(GetHeroByPlayerRequest { player_id })
            .await
            .unwrap()
            .expect("hero should be loaded");
        let loaded_pending_at = use_cases
            .get_pending_hero_revival_at(GetPendingHeroRevivalRequest { player_id })
            .await
            .unwrap();

        assert_eq!(loaded_hero.id, hero_id);
        assert_eq!(loaded_pending_at, Some(pending_at));
    }

    #[tokio::test]
    async fn create_hero_builds_command_with_existing_hero_context() {
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let reads = Arc::new(FakeHeroReads::default());
        reads.alive_hero.lock().unwrap().insert(player_id, true);
        let executor = Arc::new(FakeHeroExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .create_hero(CreateHeroRequest {
                hero_id,
                player_id,
                village_id: 7,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let HeroCommandIntent::CreateHero {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected create hero command");
        };
        assert_eq!(village_id, 7);
        assert_eq!(command.hero_id, hero_id);
        assert_eq!(command.player_id, player_id);
        assert!(command.has_existing_hero);
        assert!(!command.bypass_hero_mansion_requirement);
    }

    #[tokio::test]
    async fn revive_hero_rejects_pending_revival_without_executing() {
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let reads = Arc::new(FakeHeroReads::default());
        reads
            .heroes
            .lock()
            .unwrap()
            .insert(hero_id, dead_hero(hero_id, player_id, 1));
        reads
            .pending_revival_at
            .lock()
            .unwrap()
            .insert(player_id, Some(Utc::now()));
        let executor = Arc::new(FakeHeroExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        let result = use_cases
            .revive_hero(ReviveHeroRequest {
                hero_id,
                player_id,
                village_id: 1,
                reset: false,
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Game(GameError::HeroRevivalAlreadyPending))
        ));
        assert!(executor.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn revive_hero_builds_command_with_deterministic_id_and_time() {
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let hero = dead_hero(hero_id, player_id, 1);
        let expected_revive_at = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()
            + chrono::Duration::seconds(hero.resurrection_cost(2).time as i64);
        let reads = Arc::new(FakeHeroReads::default());
        reads.heroes.lock().unwrap().insert(hero_id, hero.clone());
        let executor = Arc::new(FakeHeroExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .revive_hero(ReviveHeroRequest {
                hero_id,
                player_id,
                village_id: 1,
                reset: true,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let HeroCommandIntent::ReviveHero {
            village_id,
            command,
        } = commands.pop_front().expect("command should be executed")
        else {
            panic!("expected revive hero command");
        };
        assert_eq!(village_id, 1);
        assert_eq!(command.action_id, Uuid::from_u128(42));
        assert_eq!(command.player_id, player_id);
        assert_eq!(command.hero.id, hero_id);
        assert!(command.reset);
        assert_eq!(command.speed, 2);
        assert_eq!(command.revive_at, expected_revive_at);
    }

    #[tokio::test]
    async fn assign_hero_points_builds_command_with_loaded_hero() {
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let reads = Arc::new(FakeHeroReads::default());
        reads
            .heroes
            .lock()
            .unwrap()
            .insert(hero_id, hero(hero_id, player_id, 1));
        let executor = Arc::new(FakeHeroExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .assign_hero_points(AssignHeroPointsRequest {
                hero_id,
                player_id,
                village_id: 1,
                strength: 1,
                off_bonus: 2,
                def_bonus: 3,
                regeneration: 4,
                resources: 5,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let HeroCommandIntent::AssignHeroPoints { command, .. } =
            commands.pop_front().expect("command should be executed")
        else {
            panic!("expected assign hero points command");
        };
        assert_eq!(command.hero.id, hero_id);
        assert_eq!(command.strength, 1);
        assert_eq!(command.off_bonus, 2);
        assert_eq!(command.def_bonus, 3);
        assert_eq!(command.regeneration, 4);
        assert_eq!(command.resources, 5);
    }

    #[tokio::test]
    async fn reset_hero_points_builds_command_with_loaded_hero() {
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let reads = Arc::new(FakeHeroReads::default());
        reads
            .heroes
            .lock()
            .unwrap()
            .insert(hero_id, hero(hero_id, player_id, 1));
        let executor = Arc::new(FakeHeroExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .reset_hero_points(ResetHeroPointsRequest {
                hero_id,
                player_id,
                village_id: 1,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let HeroCommandIntent::ResetHeroPoints { command, .. } =
            commands.pop_front().expect("command should be executed")
        else {
            panic!("expected reset hero points command");
        };
        assert_eq!(command.hero.id, hero_id);
        assert_eq!(command.player_id, player_id);
    }

    #[tokio::test]
    async fn set_hero_resource_focus_builds_command_with_loaded_hero() {
        let player_id = Uuid::new_v4();
        let hero_id = Uuid::new_v4();
        let reads = Arc::new(FakeHeroReads::default());
        reads
            .heroes
            .lock()
            .unwrap()
            .insert(hero_id, hero(hero_id, player_id, 1));
        let executor = Arc::new(FakeHeroExecutor::default());
        let use_cases = use_cases(reads, executor.clone());

        use_cases
            .set_hero_resource_focus(SetHeroResourceFocusRequest {
                hero_id,
                player_id,
                village_id: 1,
                focus: HeroResourceFocus::Crop,
            })
            .await
            .unwrap();

        let mut commands = executor.commands.lock().unwrap();
        let HeroCommandIntent::SetHeroResourceFocus { command, .. } =
            commands.pop_front().expect("command should be executed")
        else {
            panic!("expected set hero focus command");
        };
        assert_eq!(command.hero.id, hero_id);
        assert_eq!(command.focus, HeroResourceFocus::Crop);
    }
}
