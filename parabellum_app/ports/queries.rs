use crate::cqrs::queries::{MarketplaceData, VillageQueues, VillageTroopMovements};
use crate::repository::MapRegionTile;
use crate::repository::PlayerLeaderboardEntry;
use crate::repository::VillageInfo;
use async_trait::async_trait;
use parabellum_game::models::map::MapField;
use parabellum_types::errors::ApplicationError;
use std::collections::HashMap;
use uuid::Uuid;

use crate::villages::models::VillageModel;
use crate::villages::models::{MarketplaceOfferModel, ReportModel};

#[derive(Debug, Clone)]
pub struct LeaderboardPage {
    pub entries: Vec<PlayerLeaderboardEntry>,
    pub total_players: i64,
}

#[derive(Debug, Clone)]
pub struct ExpansionCultureInfo {
    pub village_culture_points: u32,
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
    async fn list_village_models_by_player_id(
        &self,
        player_id: Uuid,
    ) -> Result<Vec<VillageModel>, ApplicationError>;
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
