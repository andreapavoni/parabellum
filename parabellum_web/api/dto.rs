//! API DTOs and mapping helpers.
//!
//! All payloads in this module are wire-level contracts.
//! They intentionally expose canonical values (ids, enum keys, unix timestamps, numeric durations)
//! and avoid UI-formatted strings.

use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use parabellum_app::villages::models::{BuildingWorkflowKind, ScheduledActionStatus};
use parabellum_game::models::village::Village;
use parabellum_types::reports::ReportPayload;

use crate::session::CurrentUser;

#[derive(Debug, Clone)]
struct BuildingQueueItemView {
    kind: String,
    slot_id: u8,
    building_name: String,
    target_level: u8,
    is_processing: bool,
    finishes_at: chrono::DateTime<chrono::Utc>,
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
                kind: building_queue_kind_key(item.kind).to_string(),
                slot_id: item.slot_id,
                building_name: format!("{:?}", item.building_name),
                target_level: item.target_level,
                is_processing: matches!(item.status, ScheduledActionStatus::Processing),
                finishes_at: item.finishes_at,
                time_seconds: remaining,
            }
        })
        .collect()
}

fn building_queue_kind_key(kind: BuildingWorkflowKind) -> &'static str {
    match kind {
        BuildingWorkflowKind::Add => "add",
        BuildingWorkflowKind::Upgrade => "upgrade",
        BuildingWorkflowKind::Downgrade => "downgrade",
    }
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
/// Building queue entry with an absolute completion deadline.
pub struct BuildingQueueItemDto {
    pub kind: String,
    pub slot_id: u8,
    pub building_name: String,
    pub target_level: u8,
    pub finishes_at: chrono::DateTime<chrono::Utc>,
    pub time_seconds: u32,
    pub is_processing: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Lightweight player summary for game context responses.
pub struct PlayerSummaryDto {
    pub id: Uuid,
    pub username: String,
    pub tribe: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
/// Rich hydration payload for `GET /api/v1/game/context`.
pub struct GameContextResponse {
    pub server_time: i64,
    pub world_size: i32,
    pub server_speed: i8,
    pub unread_reports_count: i64,
    pub player: PlayerSummaryDto,
    pub current_village_id: u32,
    pub current_village: VillageSummaryDto,
    pub villages: Vec<VillageListItemDto>,
    pub building_slots: Vec<BuildingSlotDto>,
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
    pub incoming_attacks: usize,
    pub incoming_attacks_next_at: Option<chrono::DateTime<chrono::Utc>>,
    pub incoming_raids: usize,
    pub incoming_raids_next_at: Option<chrono::DateTime<chrono::Utc>>,
    pub incoming_returns_reinforcements: usize,
    pub incoming_returns_reinforcements_next_at: Option<chrono::DateTime<chrono::Utc>>,
    pub outgoing_attacks: usize,
    pub outgoing_attacks_next_at: Option<chrono::DateTime<chrono::Utc>>,
    pub outgoing_raids: usize,
    pub outgoing_raids_next_at: Option<chrono::DateTime<chrono::Utc>>,
    pub outgoing_reinforcements: usize,
    pub outgoing_reinforcements_next_at: Option<chrono::DateTime<chrono::Utc>>,
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
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
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
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
    pub created_at: i64,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportDetailPayloadResponse {
    pub server_time: i64,
    pub id: Uuid,
    pub report_type: String,
    pub actor_player_id: Uuid,
    pub actor_village_id: Option<u32>,
    pub target_player_id: Option<Uuid>,
    pub target_village_id: Option<u32>,
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
            kind: item.kind.clone(),
            slot_id: item.slot_id,
            building_name: item.building_name.clone(),
            target_level: item.target_level,
            finishes_at: item.finishes_at,
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

pub fn game_context_response(
    server_time: i64,
    world_size: i32,
    server_speed: i8,
    unread_reports_count: i64,
    user: &CurrentUser,
    village: &Village,
    queues: &parabellum_app::ports::queries::VillageQueues,
    army_state: &parabellum_app::ports::queries::VillageArmyStateView,
    movements: &parabellum_app::ports::queries::VillageTroopMovements,
) -> GameContextResponse {
    use parabellum_app::ports::queries::TroopMovementType;
    let queue_views = building_queue_to_views(&queues.building);
    let summarize = |items: &[parabellum_app::ports::queries::TroopMovement],
                     predicate: fn(TroopMovementType) -> bool| {
        let mut count = 0usize;
        let mut next_at: Option<chrono::DateTime<chrono::Utc>> = None;
        for movement in items {
            if predicate(movement.movement_type) {
                count += 1;
                next_at = match next_at {
                    Some(current) => Some(std::cmp::min(current, movement.arrives_at)),
                    None => Some(movement.arrives_at),
                };
            }
        }
        (count, next_at)
    };
    let (incoming_attacks, incoming_attacks_next_at) = summarize(&movements.incoming, |kind| {
        matches!(kind, TroopMovementType::Attack | TroopMovementType::Scout)
    });
    let (incoming_raids, incoming_raids_next_at) =
        summarize(&movements.incoming, |kind| kind == TroopMovementType::Raid);
    let (incoming_returns_reinforcements, incoming_returns_reinforcements_next_at) =
        summarize(&movements.incoming, |kind| {
            matches!(
                kind,
                TroopMovementType::Return | TroopMovementType::Reinforcement
            )
        });
    let (outgoing_attacks, outgoing_attacks_next_at) = summarize(&movements.outgoing, |kind| {
        matches!(kind, TroopMovementType::Attack | TroopMovementType::Scout)
    });
    let (outgoing_raids, outgoing_raids_next_at) =
        summarize(&movements.outgoing, |kind| kind == TroopMovementType::Raid);
    let (outgoing_reinforcements, outgoing_reinforcements_next_at) =
        summarize(&movements.outgoing, |kind| {
            kind == TroopMovementType::Reinforcement
        });

    GameContextResponse {
        server_time,
        world_size,
        server_speed,
        unread_reports_count,
        player: player_summary(user),
        current_village_id: village.id,
        current_village: village_summary(village),
        villages: village_list(user),
        building_slots: building_slots(village, &queue_views),
        resource_slots: resource_slots(village, &queue_views),
        building_queue: building_queue_items(&queue_views),
        current_troops: current_troops(army_state),
        troop_movement_summary: TroopMovementSummaryDto {
            incoming_attacks,
            incoming_attacks_next_at,
            incoming_raids,
            incoming_raids_next_at,
            incoming_returns_reinforcements,
            incoming_returns_reinforcements_next_at,
            outgoing_attacks,
            outgoing_attacks_next_at,
            outgoing_raids,
            outgoing_raids_next_at,
            outgoing_reinforcements,
            outgoing_reinforcements_next_at,
        },
    }
}
