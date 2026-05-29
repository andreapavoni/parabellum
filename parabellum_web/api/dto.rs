//! API DTOs and mapping helpers.
//!
//! All payloads in this module are wire-level contracts.
//! They intentionally expose canonical values (ids, enum keys, unix timestamps, numeric durations)
//! and avoid UI-formatted strings.

use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use parabellum_app::villages::models::ScheduledActionStatus;
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
    items: &[parabellum_app::ports::queries::BuildingQueueItem],
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
                is_processing: matches!(item.status, ScheduledActionStatus::Processing),
                time_seconds: remaining,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public user identity for authenticated session payloads.
pub struct SessionUserDto {
    pub user_id: Uuid,
    pub player_id: Uuid,
    pub username: String,
    pub email: String,
    pub tribe: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Canonical resource amount tuple.
pub struct ResourceAmountsDto {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Per-hour production snapshot.
pub struct ProductionAmountsDto {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Village summary used across multiple endpoints.
pub struct VillageSummaryDto {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub is_capital: bool,
    pub loyalty: u8,
    pub population: i32,
    pub warehouse_capacity: u32,
    pub granary_capacity: u32,
    pub resources: ResourceAmountsDto,
    pub production_per_hour: ProductionAmountsDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Village list item for current player context.
pub struct VillageListItemDto {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub is_capital: bool,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Resource field slot summary (slots 1..=18).
pub struct ResourceSlotDto {
    pub slot_id: u8,
    pub building_name: String,
    pub level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_queue: Option<bool>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Building queue entry with computed remaining time.
pub struct BuildingQueueItemDto {
    pub slot_id: u8,
    pub building_name: String,
    pub target_level: u8,
    pub time_seconds: u32,
    pub is_processing: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Lightweight player summary for `/me/context`.
pub struct PlayerSummaryDto {
    pub id: Uuid,
    pub username: String,
    pub tribe: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Response payload for `GET /api/v1/villages/{id}/overview`.
pub struct VillageOverviewResponse {
    pub server_time: i64,
    pub village: VillageSummaryDto,
    pub building_slots: Vec<BuildingSlotDto>,
    pub building_queue: Vec<BuildingQueueItemDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Response payload for `GET /api/v1/villages/{id}/resources`.
pub struct VillageResourcesResponse {
    pub server_time: i64,
    pub village: VillageSummaryDto,
    pub resource_slots: Vec<ResourceSlotDto>,
    pub building_queue: Vec<BuildingQueueItemDto>,
    pub current_troops: Vec<CurrentTroopDto>,
    pub troop_movement_summary: TroopMovementSummaryDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Troop entry aggregated for resources page.
pub struct CurrentTroopDto {
    pub unit_name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TroopMovementSummaryDto {
    pub incoming_attacks_raids: usize,
    pub incoming_returns_reinforcements: usize,
    pub outgoing_attacks_raids: usize,
    pub outgoing_reinforcements: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Leaderboard pagination metadata.
pub struct PaginationDto {
    pub page: i64,
    pub per_page: i64,
    pub total_players: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Leaderboard response payload.
pub struct StatsResponse {
    pub server_time: i64,
    pub entries: Vec<LeaderboardEntryDto>,
    pub pagination: PaginationDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Player village summary used in player profile.
pub struct PlayerVillageDto {
    pub village_id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub is_capital: bool,
    pub population: i32,
    pub distance_from_current: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Player profile response payload.
pub struct PlayerProfileResponse {
    pub server_time: i64,
    pub player_id: Uuid,
    pub username: String,
    pub villages: Vec<PlayerVillageDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Report item returned by reports list endpoint.
pub struct ReportListItemDto {
    pub id: Uuid,
    pub report_type: String,
    #[schema(value_type = ReportPayloadDoc)]
    pub payload: ReportPayload,
    pub created_at: i64,
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Reports list response payload.
pub struct ReportsResponse {
    pub server_time: i64,
    pub reports: Vec<ReportListItemDto>,
    pub pagination: ReportsPaginationDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportsPaginationDto {
    pub page: i64,
    pub per_page: i64,
    pub has_more: bool,
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

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportDetailPayloadResponse {
    pub server_time: i64,
    pub id: Uuid,
    pub report_type: String,
    pub created_at: i64,
    #[schema(value_type = ReportPayloadDoc)]
    pub payload: ReportPayload,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PositionDoc {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGroupDoc {
    pub lumber: u32,
    pub clay: u32,
    pub iron: u32,
    pub crop: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BattlePartyPayloadDoc {
    #[schema(value_type = String)]
    pub tribe: parabellum_types::tribe::Tribe,
    pub army_before: Vec<u32>,
    pub survivors: Vec<u32>,
    pub losses: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoutingTargetDefensesDoc {
    pub wall: Option<u8>,
    pub palace: Option<u8>,
    pub residence: Option<u8>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ScoutingTargetReportDoc {
    Resources(ResourceGroupDoc),
    Defenses(ScoutingTargetDefensesDoc),
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoutingBattleReportDoc {
    pub was_detected: bool,
    #[schema(value_type = String)]
    pub target: parabellum_types::battle::ScoutingTarget,
    pub target_report: ScoutingTargetReportDoc,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildingDamageReportDoc {
    #[schema(value_type = String)]
    pub name: parabellum_types::buildings::BuildingName,
    pub level_before: u8,
    pub level_after: u8,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BattleReportPayloadDoc {
    #[schema(value_type = String)]
    pub attack_type: parabellum_types::battle::AttackType,
    pub attacker_player: String,
    pub attacker_village: String,
    pub attacker_position: PositionDoc,
    pub defender_player: String,
    pub defender_village: String,
    pub defender_position: PositionDoc,
    pub success: bool,
    pub bounty: ResourceGroupDoc,
    pub attacker: Option<BattlePartyPayloadDoc>,
    pub defender: Option<BattlePartyPayloadDoc>,
    pub reinforcements: Vec<BattlePartyPayloadDoc>,
    pub scouting: Option<ScoutingBattleReportDoc>,
    pub wall_damage: Option<BuildingDamageReportDoc>,
    pub catapult_damage: Vec<BuildingDamageReportDoc>,
    pub loyalty_before: Option<u8>,
    pub loyalty_after: Option<u8>,
    pub conquered: Option<bool>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReinforcementReportPayloadDoc {
    pub sender_player: String,
    pub sender_village: String,
    pub sender_position: PositionDoc,
    pub receiver_player: String,
    pub receiver_village: String,
    pub receiver_position: PositionDoc,
    #[schema(value_type = String)]
    pub tribe: parabellum_types::tribe::Tribe,
    pub units: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceDeliveryReportPayloadDoc {
    pub sender_player: String,
    pub sender_village: String,
    pub sender_position: PositionDoc,
    pub receiver_player: String,
    pub receiver_village: String,
    pub receiver_position: PositionDoc,
    pub resources: ResourceGroupDoc,
    pub merchants_used: u8,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ReportPayloadDoc {
    Battle(BattleReportPayloadDoc),
    Reinforcement(ReinforcementReportPayloadDoc),
    MarketplaceDelivery(MarketplaceDeliveryReportPayloadDoc),
}

/// Maps domain village state into API summary DTO.
pub fn village_summary(village: &Village) -> VillageSummaryDto {
    let resources = village.stored_resources();
    VillageSummaryDto {
        id: village.id,
        name: village.name.clone(),
        x: village.position.x,
        y: village.position.y,
        is_capital: village.is_capital,
        loyalty: village.loyalty(),
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
            is_capital: village.is_capital,
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

fn current_troops(
    army_state: &parabellum_app::ports::queries::VillageArmyStateView,
) -> Vec<CurrentTroopDto> {
    // Troop counters are derived from rm_armies-backed state view
    // (home + stationed reinforcements), not from rm_village snapshots.
    let mut grouped: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut accumulate = |army: &parabellum_game::models::army::Army| {
        for (idx, count) in army.units().units().iter().enumerate() {
            if *count == 0 {
                continue;
            }

            let unit_name = army
                .tribe
                .units()
                .get(idx)
                .map(|unit| format!("{:?}", unit.name))
                .unwrap_or_else(|| format!("Unit{}", idx + 1));
            *grouped.entry(unit_name).or_insert(0) += *count;
        }
    };

    if let Some(home) = &army_state.home_army {
        accumulate(home);
    }
    for reinforcement in &army_state.reinforcements {
        accumulate(reinforcement);
    }

    grouped
        .into_iter()
        .map(|(unit_name, count)| CurrentTroopDto { unit_name, count })
        .collect()
}

pub fn village_overview_response(
    village: &Village,
    queues: &parabellum_app::ports::queries::VillageQueues,
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
    queues: &parabellum_app::ports::queries::VillageQueues,
    army_state: &parabellum_app::ports::queries::VillageArmyStateView,
    movements: &parabellum_app::ports::queries::VillageTroopMovements,
) -> VillageResourcesResponse {
    use parabellum_app::ports::queries::TroopMovementType;
    let queue_views = building_queue_to_views(&queues.building);
    let incoming_attacks_raids = movements
        .incoming
        .iter()
        .filter(|movement| {
            matches!(
                movement.movement_type,
                TroopMovementType::Attack | TroopMovementType::Raid | TroopMovementType::Scout
            )
        })
        .count();
    let incoming_returns_reinforcements = movements
        .incoming
        .iter()
        .filter(|movement| {
            matches!(
                movement.movement_type,
                TroopMovementType::Return | TroopMovementType::Reinforcement
            )
        })
        .count();
    let outgoing_attacks_raids = movements
        .outgoing
        .iter()
        .filter(|movement| {
            matches!(
                movement.movement_type,
                TroopMovementType::Attack | TroopMovementType::Raid | TroopMovementType::Scout
            )
        })
        .count();
    let outgoing_reinforcements = movements
        .outgoing
        .iter()
        .filter(|movement| movement.movement_type == TroopMovementType::Reinforcement)
        .count();
    VillageResourcesResponse {
        server_time: Utc::now().timestamp(),
        village: village_summary(village),
        resource_slots: resource_slots(village, &queue_views),
        building_queue: building_queue_items(&queue_views),
        current_troops: current_troops(army_state),
        troop_movement_summary: TroopMovementSummaryDto {
            incoming_attacks_raids,
            incoming_returns_reinforcements,
            outgoing_attacks_raids,
            outgoing_reinforcements,
        },
    }
}
