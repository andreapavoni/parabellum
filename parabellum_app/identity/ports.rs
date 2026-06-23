//! Identity ports.
//!
//! These contracts keep identity persistence, authentication infrastructure,
//! and registration side effects outside the application facade. They remain
//! concern-owned app contracts because identity is shared by web sessions,
//! registration, player lookups, reports, culture-point refresh, and
//! leaderboard reads.

use async_trait::async_trait;
use parabellum_game::models::map::{MapQuadrant, Valley};
use parabellum_types::{
    common::{Player, User},
    errors::ApplicationError,
    tribe::Tribe,
};
use uuid::Uuid;

use crate::villages::{CreateHero, FoundVillage, SetVillageResources};

/// Input required to create identity rows and reserve an initial map valley.
#[derive(Debug, Clone)]
pub struct RegistrationIdentityRecord {
    pub player_id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub tribe: Tribe,
    pub quadrant: MapQuadrant,
}

/// Identity transaction result used by registration orchestration.
#[derive(Debug, Clone)]
pub struct CreatedRegistrationIdentity {
    pub user_id: Uuid,
    pub player: Player,
    pub valley: Valley,
}

/// Identity service boundary used by `GameApplication`.
#[async_trait]
pub trait IdentityPort: Send + Sync {
    /// Authenticates a user by username and password.
    async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, ApplicationError>;

    /// Returns a user by email.
    async fn get_user_by_email(&self, email: &str) -> Result<User, ApplicationError>;

    /// Returns a user by id.
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError>;

    /// Returns the player attached to a user id.
    async fn get_player_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError>;

    /// Returns a player by id.
    async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError>;
}

/// Identity transaction boundary used by registration use cases.
///
/// Implementations must create user and player rows and reserve the selected
/// initial valley atomically. Initial village ES commands are intentionally
/// executed through `InitialVillageCommandExecutor` after this transaction
/// commits so the existing FK and projection behavior stays explicit.
#[async_trait]
pub trait RegistrationIdentityPort: Send + Sync {
    /// Creates identity rows and reserves an initial valley for a new player.
    async fn create_registration_identity(
        &self,
        record: RegistrationIdentityRecord,
    ) -> Result<CreatedRegistrationIdentity, ApplicationError>;

    /// Cleans up identity rows and the soft map reservation after an ES failure.
    async fn cleanup_failed_registration(
        &self,
        user_id: Uuid,
        player_id: Uuid,
        village_id: u32,
    ) -> Result<(), ApplicationError>;
}

/// Initial village command boundary used by registration use cases.
#[async_trait]
pub trait InitialVillageCommandExecutor: Send + Sync {
    /// Appends the initial village founded fact.
    async fn found_initial_village(
        &self,
        village_id: u32,
        command: FoundVillage,
    ) -> Result<(), ApplicationError>;

    /// Appends the initial hero creation fact.
    async fn create_initial_hero(
        &self,
        village_id: u32,
        command: CreateHero,
    ) -> Result<(), ApplicationError>;

    /// Applies optional seed/test resource overrides to the initial village.
    async fn set_initial_village_resources(
        &self,
        village_id: u32,
        command: SetVillageResources,
    ) -> Result<(), ApplicationError>;
}

/// User persistence boundary.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Saves a user.
    async fn save(&self, email: String, password_hash: String) -> Result<(), ApplicationError>;

    /// Returns a user by email.
    async fn get_by_email(&self, email: &str) -> Result<User, ApplicationError>;

    /// Returns a user by username.
    async fn get_by_username(&self, username: &str) -> Result<User, ApplicationError>;

    /// Returns a user by id.
    async fn get_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError>;
}

/// Player persistence and read-model boundary.
///
/// This trait still mixes identity-profile persistence with culture-point read
/// maintenance. Keep the current behavior stable for now; split profile,
/// culture, and player lookup ports when registration/player orchestration is
/// refactored.
#[async_trait]
pub trait PlayerRepository: Send + Sync {
    /// Saves a player, creating or updating the row as needed.
    async fn save(&self, player: &Player) -> Result<(), ApplicationError>;

    /// Returns a player by id.
    async fn get_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError>;

    /// Returns a player by user id.
    async fn get_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError>;

    /// Advances player's total culture points using elapsed time and aggregated village CPP/day.
    async fn update_culture_points(&self, player_id: Uuid) -> Result<(), ApplicationError>;

    /// Returns total culture-points production per day for all player villages.
    async fn get_total_culture_points_production(
        &self,
        player_id: Uuid,
    ) -> Result<u32, ApplicationError>;
}
