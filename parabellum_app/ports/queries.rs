use crate::read_models::MapRegionTile;
use crate::read_models::PlayerLeaderboardEntry;
use crate::read_models::VillageInfo;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parabellum_game::models::army::Army;
use parabellum_game::models::map::MapField;
use parabellum_game::models::marketplace::MarketplaceOffer;
use parabellum_types::errors::ApplicationError;
use parabellum_types::{
    army::{TroopSet, UnitName},
    buildings::BuildingName,
    common::ResourceGroup,
    map::Position,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::villages::models::{MarketplaceOfferModel, ReportModel};
use crate::villages::models::{ScheduledActionStatus, VillageModel};

#[derive(Debug, Clone)]
pub struct BuildingQueueItem {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub building_name: BuildingName,
    pub target_level: u8,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TrainingQueueItem {
    pub job_id: Uuid,
    pub slot_id: u8,
    pub unit: UnitName,
    pub quantity: i32,
    pub time_per_unit: i32,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AcademyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SmithyQueueItem {
    pub job_id: Uuid,
    pub unit: UnitName,
    pub status: ScheduledActionStatus,
    pub finishes_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct VillageQueues {
    pub building: Vec<BuildingQueueItem>,
    pub training: Vec<TrainingQueueItem>,
    pub academy: Vec<AcademyQueueItem>,
    pub smithy: Vec<SmithyQueueItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementType {
    Attack,
    Raid,
    Reinforcement,
    Return,
    FoundVillage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TroopMovementDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq)]
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
    pub units: TroopSet,
    pub tribe: parabellum_types::tribe::Tribe,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VillageTroopMovements {
    pub outgoing: Vec<TroopMovement>,
    pub incoming: Vec<TroopMovement>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MerchantMovementKind {
    Going,
    Return,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MerchantMovement {
    pub job_id: Uuid,
    pub kind: MerchantMovementKind,
    pub origin_village_id: u32,
    pub destination_village_id: u32,
    pub resources: ResourceGroup,
    pub merchants_used: u8,
    pub arrives_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarketplaceData {
    pub own_offers: Vec<MarketplaceOffer>,
    pub global_offers: Vec<MarketplaceOffer>,
    pub outgoing_merchants: Vec<MerchantMovement>,
    pub incoming_merchants: Vec<MerchantMovement>,
    pub village_info: HashMap<u32, VillageInfo>,
}

#[derive(Debug, Clone)]
pub struct VillageArmyStateView {
    pub home_army: Option<Army>,
    pub reinforcements: Vec<Army>,
    pub deployed_armies: Vec<Army>,
}

#[derive(Debug, Clone)]
pub struct LeaderboardPage {
    pub entries: Vec<PlayerLeaderboardEntry>,
    pub total_players: i64,
}

#[derive(Debug, Clone)]
pub struct ExpansionCultureInfo {
    pub village_culture_points_production: u32,
    pub player_culture_points: u32,
    pub player_culture_points_production: u32,
    pub next_cp_required: u32,
}

#[async_trait]
pub trait VillageQueryPort: Send + Sync {
    async fn get_marketplace_offer(
        &self,
        offer_id: Uuid,
    ) -> Result<MarketplaceOfferModel, ApplicationError>;
    async fn list_reports_for_player(
        &self,
        player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ReportModel>, ApplicationError>;
    async fn get_report_for_player(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<Option<ReportModel>, ApplicationError>;
    async fn mark_report_as_read(
        &self,
        report_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), ApplicationError>;
    async fn get_village_queues(&self, village_id: u32) -> Result<VillageQueues, ApplicationError>;
    async fn get_village_troop_movements(
        &self,
        village_id: u32,
    ) -> Result<VillageTroopMovements, ApplicationError>;
    async fn get_marketplace_data(
        &self,
        village_id: u32,
    ) -> Result<MarketplaceData, ApplicationError>;
    async fn get_village_army_state_view(
        &self,
        village_id: u32,
    ) -> Result<VillageArmyStateView, ApplicationError>;
    async fn get_village_info_by_ids(
        &self,
        village_ids: Vec<u32>,
    ) -> Result<HashMap<u32, VillageInfo>, ApplicationError>;
    async fn get_expansion_culture_info(
        &self,
        player_id: Uuid,
        village_id: u32,
        server_speed: i8,
    ) -> Result<ExpansionCultureInfo, ApplicationError>;
    async fn get_leaderboard_page(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<LeaderboardPage, ApplicationError>;
    async fn list_villages_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError>;
    async fn get_village_model(&self, village_id: u32) -> Result<VillageModel, ApplicationError>;
    async fn get_map_region(
        &self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        world_size: i32,
    ) -> Result<Vec<MapRegionTile>, ApplicationError>;
    async fn get_map_field(&self, field_id: u32) -> Result<MapField, ApplicationError>;
    async fn get_map_region_tile_by_field_id(
        &self,
        field_id: u32,
    ) -> Result<Option<MapRegionTile>, ApplicationError>;
}
