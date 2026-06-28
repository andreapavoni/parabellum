//! Army projection repository contracts.

use parabellum_types::errors::{ApplicationError, DbError};
use uuid::Uuid;

use crate::villages::VillageArmyContext;

/// Projected army placement state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmyState {
    Home,
    Stationed,
    Moving,
    Trapped,
}

/// Filter for projected army queries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArmyListFilter {
    pub army_id: Option<Uuid>,
    pub home_village_id: Option<u32>,
    pub current_village_id: Option<u32>,
    pub state: Option<ArmyState>,
    pub deployed: Option<bool>,
    pub limit: Option<i64>,
}

impl ArmyListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn army_id(mut self, army_id: Uuid) -> Self {
        self.army_id = Some(army_id);
        self
    }

    pub fn home_village(mut self, village_id: u32) -> Self {
        self.home_village_id = Some(village_id);
        self
    }

    pub fn current_village(mut self, village_id: u32) -> Self {
        self.current_village_id = Some(village_id);
        self
    }

    pub fn state(mut self, state: ArmyState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn deployed(mut self, deployed: bool) -> Self {
        self.deployed = Some(deployed);
        self
    }

    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Persistence boundary for projected army rows.
#[async_trait::async_trait]
pub trait ArmyRepository: Send + Sync {
    async fn upsert_home(
        &self,
        army: &parabellum_game::models::army::Army,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;

    async fn upsert_moving(
        &self,
        army: &parabellum_game::models::army::Army,
        current_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;

    async fn upsert_stationed(
        &self,
        army: &parabellum_game::models::army::Army,
        stationed_village_id: u32,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;

    async fn delete(&self, army_id: Uuid) -> Result<(), ApplicationError>;

    async fn list_armies(
        &self,
        filter: ArmyListFilter,
    ) -> Result<Vec<parabellum_game::models::army::Army>, ApplicationError>;

    async fn get_moving_army(
        &self,
        army_id: Uuid,
    ) -> Result<parabellum_game::models::army::Army, ApplicationError> {
        let mut armies = self
            .list_armies(
                ArmyListFilter::new()
                    .army_id(army_id)
                    .state(ArmyState::Moving)
                    .limit(1),
            )
            .await?;
        armies
            .pop()
            .ok_or(ApplicationError::Db(DbError::ArmyNotFound(army_id)))
    }

    async fn find_stationed_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, parabellum_game::models::army::Army)>, ApplicationError>;

    async fn find_trapped_context_by_army_id(
        &self,
        army_id: Uuid,
    ) -> Result<Option<(u32, parabellum_game::models::army::Army)>, ApplicationError>;

    /// Returns the complete army placement context needed to hydrate one village.
    async fn army_context_for_village(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyContext, ApplicationError>;

    async fn delete_by_home_village(&self, village_id: u32) -> Result<(), ApplicationError>;
}
