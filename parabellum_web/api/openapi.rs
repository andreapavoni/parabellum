use axum::Json;
use serde_json::{Map, Value, json};
use utoipa::OpenApi;

use crate::api::{
    actions::{
        ActionResponse, AddBuildingRequest, CreateOfferRequest, FoundVillageRequest, MovementKind,
        MovementPreviewResponse, OfferActionRequest, PreviewDetectedKind, PreviewFoundVillageRequest,
        PreviewTroopsRequest, RecallTroopsRequest, ReleaseReinforcementsRequest,
        ResearchAcademyRequest, ResearchSmithyRequest, ScoutingTargetKind, SendResourcesRequest,
        SendTroopsRequest, TrainUnitsRequest, UpgradeBuildingRequest,
    },
    auth::{
        LogoutResponse, TokenAuthResponse, TokenLoginRequest, TokenLogoutRequest,
        TokenRefreshRequest, TokenRegisterRequest,
    },
    dto::{
        BattlePartyPayloadDoc, BattleReportPayloadDoc, BuildingDamageReportDoc, LeaderboardEntryDto,
        MarketplaceDeliveryReportPayloadDoc, MeContextResponse, PaginationDto, PlayerProfileResponse,
        PlayerSummaryDto, PlayerVillageDto, PositionDoc, ProductionAmountsDto, ReportDetailPayloadResponse,
        ReportListItemDto, ReportPayloadDoc, ReportsResponse, ReinforcementReportPayloadDoc,
        ResourceAmountsDto, ResourceGroupDoc, ScoutingBattleReportDoc, ScoutingTargetDefensesDoc,
        ScoutingTargetReportDoc, SessionUserDto, StatsResponse, VillageListItemDto, VillageSummaryDto,
    },
    game::{
        MapFieldDetailResponse, MapPoint, MapRegionQuery, MapRegionResponse, MapTileResponse,
        MeSessionResponse, StatsQuery, SwitchVillageRequest, SwitchVillageResponse, TileType,
        ValleyDistribution,
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::auth::token_login,
        crate::api::auth::token_register,
        crate::api::auth::token_refresh,
        crate::api::auth::token_logout,
        crate::api::game::me_session,
        crate::api::game::me_context,
        crate::api::game::stats,
        crate::api::game::village_overview,
        crate::api::game::village_resources,
        crate::api::buildings::building_detail,
        crate::api::actions::add_building,
        crate::api::actions::upgrade_building,
        crate::api::actions::train_units,
        crate::api::actions::send_resources,
        crate::api::actions::create_marketplace_offer,
        crate::api::actions::accept_marketplace_offer,
        crate::api::actions::cancel_marketplace_offer,
        crate::api::actions::send_troops,
        crate::api::actions::preview_troops,
        crate::api::actions::recall_troops,
        crate::api::actions::release_reinforcements,
        crate::api::actions::research_academy,
        crate::api::actions::research_smithy,
        crate::api::actions::found_village,
        crate::api::actions::preview_found_village,
        crate::api::game::switch_village,
        crate::api::game::player_profile,
        crate::api::game::reports,
        crate::api::game::report_detail,
        crate::api::game::map_region,
        crate::api::game::map_field,
        crate::api::openapi::openapi_spec
    ),
    components(
        schemas(
            TokenLoginRequest,
            TokenRegisterRequest,
            TokenRefreshRequest,
            TokenLogoutRequest,
            TokenAuthResponse,
            LogoutResponse,
            MeSessionResponse,
            MeContextResponse,
            StatsQuery,
            StatsResponse,
            SessionUserDto,
            ResourceAmountsDto,
            ProductionAmountsDto,
            VillageSummaryDto,
            VillageListItemDto,
            PlayerSummaryDto,
            PlayerVillageDto,
            PlayerProfileResponse,
            LeaderboardEntryDto,
            PaginationDto,
            ReportListItemDto,
            ReportsResponse,
            ReportDetailPayloadResponse,
            ReportPayloadDoc,
            BattleReportPayloadDoc,
            BattlePartyPayloadDoc,
            ReinforcementReportPayloadDoc,
            MarketplaceDeliveryReportPayloadDoc,
            ScoutingBattleReportDoc,
            ScoutingTargetReportDoc,
            ScoutingTargetDefensesDoc,
            BuildingDamageReportDoc,
            PositionDoc,
            ResourceGroupDoc,
            ActionResponse,
            AddBuildingRequest,
            UpgradeBuildingRequest,
            TrainUnitsRequest,
            SendResourcesRequest,
            CreateOfferRequest,
            OfferActionRequest,
            MovementKind,
            ScoutingTargetKind,
            SendTroopsRequest,
            PreviewTroopsRequest,
            MovementPreviewResponse,
            PreviewDetectedKind,
            RecallTroopsRequest,
            ReleaseReinforcementsRequest,
            ResearchAcademyRequest,
            ResearchSmithyRequest,
            FoundVillageRequest,
            PreviewFoundVillageRequest,
            SwitchVillageRequest,
            SwitchVillageResponse,
            MapRegionQuery,
            MapRegionResponse,
            MapPoint,
            MapTileResponse,
            TileType,
            MapFieldDetailResponse,
            ValleyDistribution
        )
    )
)]
pub struct ApiDoc;

fn fallback_paths() -> Map<String, Value> {
    Map::new()
}

pub fn merged_openapi_value(mut value: Value) -> Value {
    value["servers"] = json!([{ "url": "/api/v1" }]);

    let paths = value["paths"].as_object_mut();
    if let Some(paths) = paths {
        for (path, entry) in fallback_paths() {
            paths.entry(path).or_insert(entry);
        }
    }

    value
}

#[utoipa::path(
    get,
    path = "/openapi.json",
    responses((status = 200, body = serde_json::Value))
)]
pub async fn openapi_spec() -> Json<Value> {
    let value = serde_json::to_value(ApiDoc::openapi())
        .unwrap_or_else(|_| json!({"openapi": "3.1.0", "paths": {}}));
    Json(merged_openapi_value(value))
}
