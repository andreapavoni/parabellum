use chrono::{DateTime, Utc};
use parabellum_game::models::village::Village;
use parabellum_types::{
    army::UnitName,
    buildings::BuildingName,
    common::{Player, User},
    map::Position,
};
use uuid::Uuid;

use crate::repository::MapRegionTile;
use crate::{cqrs::Query, jobs::JobStatus};

/// Checks if a user is authenticates with email and password.
pub struct AuthenticateUser {
    pub email: String,
    pub password: String,
}

impl Query for AuthenticateUser {
    type Output = User;
}

/// Fetch a user by email without checking password (for authenticated sessions).
pub struct GetUserByEmail {
    pub email: String,
}

impl Query for GetUserByEmail {
    type Output = User;
}

/// Fetch a user by id (for authenticated sessions).
pub struct GetUserById {
    pub id: Uuid,
}

impl Query for GetUserById {
    type Output = User;
}

/// Fetch the player entity associated to a user id.
pub struct GetPlayerByUserId {
    pub user_id: Uuid,
}

impl Query for GetPlayerByUserId {
    type Output = Player;
}

/// Fetch a village by id.
pub struct GetVillageById {
    pub id: u32,
}

impl Query for GetVillageById {
    type Output = Village;
}

/// List all villages for a player.
pub struct ListVillagesByPlayerId {
    pub player_id: Uuid,
}

impl Query for ListVillagesByPlayerId {
    type Output = Vec<Village>;
}

/// A queued (or processing) construction for a village.
#[derive(Debug, Clone)]
pub struct BuildingQueueItem {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub target_level: u8,
    pub status: JobStatus,
    pub finishes_at: DateTime<Utc>,
}

/// Fetch the building queue for a village.
pub struct GetVillageBuildingQueue {
    pub village_id: u32,
}

impl Query for GetVillageBuildingQueue {
    type Output = Vec<BuildingQueueItem>;
}

#[derive(Debug, Clone)]
pub struct TrainingQueueItem {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit: i32,
    pub status: JobStatus,
    pub finishes_at: DateTime<Utc>,
}

pub struct GetVillageTrainingQueue {
    pub village_id: u32,
}

impl Query for GetVillageTrainingQueue {
    type Output = Vec<TrainingQueueItem>;
}

#[derive(Debug, Clone)]
pub struct AcademyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: JobStatus,
    pub finishes_at: DateTime<Utc>,
}

pub struct GetVillageAcademyQueue {
    pub village_id: u32,
}

impl Query for GetVillageAcademyQueue {
    type Output = Vec<AcademyQueueItem>;
}

#[derive(Debug, Clone)]
pub struct SmithyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: JobStatus,
    pub finishes_at: DateTime<Utc>,
}

pub struct GetVillageSmithyQueue {
    pub village_id: u32,
}

impl Query for GetVillageSmithyQueue {
    type Output = Vec<SmithyQueueItem>;
}

#[derive(Debug, Clone, Default)]
pub struct VillageQueues {
    pub building: Vec<BuildingQueueItem>,
    pub training: Vec<TrainingQueueItem>,
    pub academy: Vec<AcademyQueueItem>,
    pub smithy: Vec<SmithyQueueItem>,
}

pub struct GetVillageQueues {
    pub village_id: u32,
}

impl Query for GetVillageQueues {
    type Output = VillageQueues;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementType {
    Attack,
    Raid,
    Reinforcement,
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone)]
pub struct TroopMovement {
    pub job_id: Uuid,
    pub movement_type: TroopMovementType,
    pub direction: TroopMovementDirection,
    pub origin_village_id: u32,
    pub origin_village_name: Option<String>,
    pub origin_player_id: Uuid,
    pub origin_position: Position,
    pub target_village_id: u32,
    pub target_village_name: Option<String>,
    pub target_player_id: Uuid,
    pub target_position: Position,
    pub arrives_at: DateTime<Utc>,
    pub time_seconds: u32,
}

#[derive(Debug, Clone, Default)]
pub struct VillageTroopMovements {
    pub outgoing: Vec<TroopMovement>,
    pub incoming: Vec<TroopMovement>,
}

pub struct GetVillageTroopMovements {
    pub village_id: u32,
}

impl Query for GetVillageTroopMovements {
    type Output = VillageTroopMovements;
}

/// Fetch a square region of the world map.
pub struct GetMapRegion {
    pub center: Position,
    pub radius: i32,
    pub world_size: i32,
}

impl Query for GetMapRegion {
    type Output = Vec<MapRegionTile>;
}
