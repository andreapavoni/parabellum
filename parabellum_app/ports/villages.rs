use async_trait::async_trait;
use parabellum_types::army::{TroopSet, UnitName};
use parabellum_types::battle::{AttackType, ScoutingTarget};
use parabellum_types::buildings::BuildingName;
use parabellum_types::common::{ResourceGroup, ResourceQuantity};
use parabellum_types::errors::ApplicationError;
use parabellum_types::map::Position;
use parabellum_types::tribe::Tribe;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AddBuildingRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
    pub building_name: BuildingName,
}

#[derive(Debug, Clone)]
pub struct UpgradeBuildingRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub slot_id: u8,
}

#[derive(Debug, Clone)]
pub struct TrainUnitsRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit_idx: u8,
    pub building_name: BuildingName,
    pub quantity: i32,
}

#[derive(Debug, Clone)]
pub struct ResearchAcademyRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit: UnitName,
}

#[derive(Debug, Clone)]
pub struct ResearchSmithyRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub unit: UnitName,
}

#[derive(Debug, Clone)]
pub struct SendReinforcementRequest {
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct SendAttackRequest {
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
    pub attack_type: AttackType,
    pub catapult_targets: [BuildingName; 2],
}

#[derive(Debug, Clone)]
pub struct SendScoutRequest {
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub units: TroopSet,
    pub target: ScoutingTarget,
    pub attack_type: AttackType,
}

#[derive(Debug, Clone)]
pub struct SendSettlersRequest {
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_position: Position,
    pub village_name: String,
    pub tribe: Tribe,
}

#[derive(Debug, Clone)]
pub struct RecallReinforcementsRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ReleaseReinforcementsRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub army_id: Uuid,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct SendResourcesRequest {
    pub player_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub resources: ResourceGroup,
}

#[derive(Debug, Clone)]
pub struct CreateMarketplaceOfferRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub offer_resources: ResourceQuantity,
    pub seek_resources: ResourceQuantity,
}

#[derive(Debug, Clone)]
pub struct AcceptMarketplaceOfferRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub offer_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CancelMarketplaceOfferRequest {
    pub player_id: Uuid,
    pub village_id: u32,
    pub offer_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CreateHeroRequest {
    pub hero_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
}

#[derive(Debug, Clone)]
pub struct ReviveHeroRequest {
    pub hero_id: Uuid,
    pub player_id: Uuid,
    pub village_id: u32,
    pub reset: bool,
}

#[async_trait]
pub trait VillageCommandPort: Send + Sync {
    async fn add_building(&self, request: AddBuildingRequest) -> Result<(), ApplicationError>;
    async fn upgrade_building(
        &self,
        request: UpgradeBuildingRequest,
    ) -> Result<(), ApplicationError>;
    async fn train_units(&self, request: TrainUnitsRequest) -> Result<(), ApplicationError>;
    async fn research_academy(
        &self,
        request: ResearchAcademyRequest,
    ) -> Result<(), ApplicationError>;
    async fn research_smithy(&self, request: ResearchSmithyRequest)
    -> Result<(), ApplicationError>;
    async fn send_reinforcement(
        &self,
        request: SendReinforcementRequest,
    ) -> Result<(), ApplicationError>;
    async fn send_attack(&self, request: SendAttackRequest) -> Result<(), ApplicationError>;
    async fn send_scout(&self, request: SendScoutRequest) -> Result<(), ApplicationError>;
    async fn send_settlers(&self, request: SendSettlersRequest) -> Result<(), ApplicationError>;
    async fn recall_reinforcements(
        &self,
        request: RecallReinforcementsRequest,
    ) -> Result<(), ApplicationError>;
    async fn release_reinforcements(
        &self,
        request: ReleaseReinforcementsRequest,
    ) -> Result<(), ApplicationError>;
    async fn send_resources(&self, request: SendResourcesRequest) -> Result<(), ApplicationError>;
    async fn create_offer(
        &self,
        request: CreateMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError>;
    async fn accept_offer(
        &self,
        request: AcceptMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError>;
    async fn cancel_offer(
        &self,
        request: CancelMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError>;
    async fn create_hero(&self, request: CreateHeroRequest) -> Result<(), ApplicationError>;
    async fn revive_hero(&self, request: ReviveHeroRequest) -> Result<(), ApplicationError>;
}
