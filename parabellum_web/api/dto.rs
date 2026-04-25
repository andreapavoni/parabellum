//! API DTOs and mapping helpers.
//!
//! All payloads in this module are wire-level contracts.
//! They intentionally expose canonical values (ids, enum keys, unix timestamps, numeric durations)
//! and avoid UI-formatted strings.

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use parabellum_game::models::village::Village;
use parabellum_types::reports::ReportPayload;

use crate::session::CurrentUser;

#[derive(Debug, Clone)]
struct BuildingQueueItemView {
    slot_id: u8,
    building_name: String,
    target_level: u8,
    is_processing: bool,
    time_seconds: u32,
}

fn building_queue_to_views(
    items: &[parabellum_app::cqrs::queries::BuildingQueueItem],
) -> Vec<BuildingQueueItemView> {
    let now = Utc::now();
    items
        .iter()
        .map(|item| {
            let remaining = (item.finishes_at - now).num_seconds().max(0) as u32;
            BuildingQueueItemView {
                slot_id: item.slot_id,
                building_name: format!("{:?}", item.building_name),
                target_level: item.target_level,
                is_processing: matches!(item.status, parabellum_app::jobs::JobStatus::Processing),
                time_seconds: remaining,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Public user identity for authenticated session payloads.
pub struct SessionUserDto {
    pub user_id: Uuid,
    pub player_id: Uuid,
    pub username: String,
    pub email: String,
    pub tribe: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Canonical resource amount tuple.
pub struct ResourceAmountsDto {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Per-hour production snapshot.
pub struct ProductionAmountsDto {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Village summary used across multiple endpoints.
pub struct VillageSummaryDto {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub population: i32,
    pub warehouse_capacity: u32,
    pub granary_capacity: u32,
    pub resources: ResourceAmountsDto,
    pub production_per_hour: ProductionAmountsDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Village list item for current player context.
pub struct VillageListItemDto {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Village building slot summary (village center slots).
pub struct BuildingSlotDto {
    pub slot_id: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_name: Option<String>,
    pub level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_queue: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Resource field slot summary (slots 1..=18).
pub struct ResourceSlotDto {
    pub slot_id: u8,
    pub building_name: String,
    pub level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_queue: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Building queue entry with computed remaining time.
pub struct BuildingQueueItemDto {
    pub slot_id: u8,
    pub building_name: String,
    pub target_level: u8,
    pub time_seconds: u32,
    pub is_processing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Lightweight player summary for `/me/context`.
pub struct PlayerSummaryDto {
    pub id: Uuid,
    pub username: String,
    pub tribe: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Response payload for `GET /api/v1/me/context`.
pub struct MeContextResponse {
    pub server_time: i64,
    pub world_size: i32,
    pub server_speed: i8,
    pub player: PlayerSummaryDto,
    pub current_village: VillageSummaryDto,
    pub villages: Vec<VillageListItemDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Response payload for `GET /api/v1/villages/{id}/overview`.
pub struct VillageOverviewResponse {
    pub server_time: i64,
    pub village: VillageSummaryDto,
    pub building_slots: Vec<BuildingSlotDto>,
    pub building_queue: Vec<BuildingQueueItemDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Response payload for `GET /api/v1/villages/{id}/resources`.
pub struct VillageResourcesResponse {
    pub server_time: i64,
    pub village: VillageSummaryDto,
    pub resource_slots: Vec<ResourceSlotDto>,
    pub building_queue: Vec<BuildingQueueItemDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Leaderboard row.
pub struct LeaderboardEntryDto {
    pub player_id: String,
    pub rank: i64,
    pub username: String,
    pub tribe: String,
    pub village_count: i64,
    pub population: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Leaderboard pagination metadata.
pub struct PaginationDto {
    pub page: i64,
    pub per_page: i64,
    pub total_players: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Leaderboard response payload.
pub struct StatsResponse {
    pub server_time: i64,
    pub entries: Vec<LeaderboardEntryDto>,
    pub pagination: PaginationDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Player village summary used in player profile.
pub struct PlayerVillageDto {
    pub village_id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub population: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Player profile response payload.
pub struct PlayerProfileResponse {
    pub server_time: i64,
    pub player_id: Uuid,
    pub username: String,
    pub villages: Vec<PlayerVillageDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Report item returned by reports list endpoint.
pub struct ReportListItemDto {
    pub id: Uuid,
    pub report_type: String,
    pub payload: ReportPayload,
    pub created_at: i64,
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Reports list response payload.
pub struct ReportsResponse {
    pub server_time: i64,
    pub reports: Vec<ReportListItemDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Generic report detail response payload.
pub struct ReportDetailResponse<T>
where
    T: Serialize,
{
    pub server_time: i64,
    pub id: Uuid,
    pub report_type: String,
    pub created_at: i64,
    pub payload: T,
}

/// Maps domain village state into API summary DTO.
pub fn village_summary(village: &Village) -> VillageSummaryDto {
    let resources = village.stored_resources();
    VillageSummaryDto {
        id: village.id,
        name: village.name.clone(),
        x: village.position.x,
        y: village.position.y,
        population: village.population as i32,
        warehouse_capacity: village.warehouse_capacity(),
        granary_capacity: village.granary_capacity(),
        resources: ResourceAmountsDto {
            lumber: resources.lumber(),
            clay: resources.clay(),
            iron: resources.iron(),
            crop: resources.crop(),
        },
        production_per_hour: ProductionAmountsDto {
            lumber: village.production.effective.lumber,
            clay: village.production.effective.clay,
            iron: village.production.effective.iron,
            crop: village.production.effective.crop,
        },
    }
}

/// Maps current user villages into list payload.
pub fn village_list(user: &CurrentUser) -> Vec<VillageListItemDto> {
    user.villages
        .iter()
        .map(|village| VillageListItemDto {
            id: village.id,
            name: village.name.clone(),
            x: village.position.x,
            y: village.position.y,
            is_current: village.id == user.village.id,
        })
        .collect()
}

/// Builds player summary from current user context.
pub fn player_summary(user: &CurrentUser) -> PlayerSummaryDto {
    PlayerSummaryDto {
        id: user.player.id,
        username: user.player.username.clone(),
        tribe: format!("{:?}", user.player.tribe),
    }
}

/// Builds authenticated session user payload.
pub fn session_user(user: &CurrentUser) -> SessionUserDto {
    SessionUserDto {
        user_id: user.account.id,
        player_id: user.player.id,
        username: user.player.username.clone(),
        email: user.account.email.clone(),
        tribe: format!("{:?}", user.player.tribe),
    }
}

fn building_queue_items(queue_views: &[BuildingQueueItemView]) -> Vec<BuildingQueueItemDto> {
    queue_views
        .iter()
        .map(|item| BuildingQueueItemDto {
            slot_id: item.slot_id,
            building_name: item.building_name.clone(),
            target_level: item.target_level,
            time_seconds: item.time_seconds,
            is_processing: item.is_processing,
        })
        .collect()
}

fn building_slots(
    village: &Village,
    queue_views: &[BuildingQueueItemView],
) -> Vec<BuildingSlotDto> {
    (19..=40)
        .map(|slot_id| {
            let building = village.buildings().iter().find(|vb| vb.slot_id == slot_id);

            let in_queue = queue_views
                .iter()
                .find(|q| q.slot_id == slot_id)
                .map(|q| q.is_processing);

            BuildingSlotDto {
                slot_id,
                building_name: building.map(|vb| format!("{:?}", vb.building.name)),
                level: building.map_or(0, |vb| vb.building.level),
                in_queue,
            }
        })
        .collect()
}

fn resource_slots(
    village: &Village,
    queue_views: &[BuildingQueueItemView],
) -> Vec<ResourceSlotDto> {
    village
        .resource_fields()
        .into_iter()
        .map(|slot| {
            let in_queue = queue_views
                .iter()
                .find(|q| q.slot_id == slot.slot_id)
                .map(|q| q.is_processing);

            ResourceSlotDto {
                slot_id: slot.slot_id,
                building_name: format!("{:?}", slot.building.name),
                level: slot.building.level,
                in_queue,
            }
        })
        .collect()
}

pub fn village_overview_response(
    village: &Village,
    queues: &parabellum_app::cqrs::queries::VillageQueues,
) -> VillageOverviewResponse {
    let queue_views = building_queue_to_views(&queues.building);
    VillageOverviewResponse {
        server_time: Utc::now().timestamp(),
        village: village_summary(village),
        building_slots: building_slots(village, &queue_views),
        building_queue: building_queue_items(&queue_views),
    }
}

pub fn village_resources_response(
    village: &Village,
    queues: &parabellum_app::cqrs::queries::VillageQueues,
) -> VillageResourcesResponse {
    let queue_views = building_queue_to_views(&queues.building);
    VillageResourcesResponse {
        server_time: Utc::now().timestamp(),
        village: village_summary(village),
        resource_slots: resource_slots(village, &queue_views),
        building_queue: building_queue_items(&queue_views),
    }
}
