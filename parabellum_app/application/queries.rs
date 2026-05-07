use parabellum_types::errors::ApplicationError;
use std::collections::HashMap;
use uuid::Uuid;

use super::GameApplication;

pub async fn get_marketplace_offer(
    app: &GameApplication,
    offer_id: Uuid,
) -> Result<crate::villages::models::MarketplaceOfferModel, ApplicationError> {
    app.queries_port().get_marketplace_offer(offer_id).await
}

pub async fn list_reports_for_player(
    app: &GameApplication,
    player_id: Uuid,
    limit: i64,
) -> Result<Vec<crate::villages::models::ReportModel>, ApplicationError> {
    app.queries_port()
        .list_reports_for_player(player_id, limit)
        .await
}

pub async fn get_report_for_player(
    app: &GameApplication,
    report_id: Uuid,
    player_id: Uuid,
) -> Result<Option<crate::villages::models::ReportModel>, ApplicationError> {
    app.queries_port()
        .get_report_for_player(report_id, player_id)
        .await
}

pub async fn mark_report_as_read(
    app: &GameApplication,
    report_id: Uuid,
    player_id: Uuid,
) -> Result<(), ApplicationError> {
    app.queries_port()
        .mark_report_as_read(report_id, player_id)
        .await
}

pub async fn get_village_queues(
    app: &GameApplication,
    village_id: u32,
) -> Result<crate::cqrs::queries::VillageQueues, ApplicationError> {
    app.queries_port().get_village_queues(village_id).await
}

pub async fn get_village_troop_movements(
    app: &GameApplication,
    village_id: u32,
) -> Result<crate::cqrs::queries::VillageTroopMovements, ApplicationError> {
    app.queries_port()
        .get_village_troop_movements(village_id)
        .await
}

pub async fn get_marketplace_data(
    app: &GameApplication,
    village_id: u32,
) -> Result<crate::cqrs::queries::MarketplaceData, ApplicationError> {
    app.queries_port().get_marketplace_data(village_id).await
}

pub async fn get_village_info_by_ids(
    app: &GameApplication,
    village_ids: Vec<u32>,
) -> Result<HashMap<u32, crate::repository::VillageInfo>, ApplicationError> {
    app.queries_port()
        .get_village_info_by_ids(village_ids)
        .await
}

pub async fn get_expansion_culture_info(
    app: &GameApplication,
    player_id: Uuid,
    village_id: u32,
    server_speed: i8,
) -> Result<crate::ports::queries::ExpansionCultureInfo, ApplicationError> {
    app.queries_port()
        .get_expansion_culture_info(player_id, village_id, server_speed)
        .await
}

pub async fn get_leaderboard_page(
    app: &GameApplication,
    page: i64,
    per_page: i64,
) -> Result<crate::ports::queries::LeaderboardPage, ApplicationError> {
    app.queries_port()
        .get_leaderboard_page(page, per_page)
        .await
}

pub async fn list_village_models_by_player_id(
    app: &GameApplication,
    player_id: Uuid,
) -> Result<Vec<crate::villages::models::VillageModel>, ApplicationError> {
    app.queries_port()
        .list_village_models_by_player_id(player_id)
        .await
}

pub async fn get_map_region(
    app: &GameApplication,
    center_x: i32,
    center_y: i32,
    radius: i32,
    world_size: i32,
) -> Result<Vec<crate::repository::MapRegionTile>, ApplicationError> {
    app.queries_port()
        .get_map_region(center_x, center_y, radius, world_size)
        .await
}

pub async fn get_map_field(
    app: &GameApplication,
    field_id: u32,
) -> Result<parabellum_game::models::map::MapField, ApplicationError> {
    app.queries_port().get_map_field(field_id).await
}

pub async fn get_map_region_tile_by_field_id(
    app: &GameApplication,
    field_id: u32,
) -> Result<Option<crate::repository::MapRegionTile>, ApplicationError> {
    app.queries_port()
        .get_map_region_tile_by_field_id(field_id)
        .await
}
