use async_trait::async_trait;
use parabellum_types::errors::ApplicationError;

use super::{
    AcceptMarketplaceOfferRequest, AddBuildingRequest, CancelMarketplaceOfferRequest,
    CreateHeroRequest, CreateMarketplaceOfferRequest, RecallReinforcementsRequest,
    ReleaseReinforcementsRequest, ResearchAcademyRequest, ResearchSmithyRequest,
    ReviveHeroRequest, SendAttackRequest, SendReinforcementRequest, SendResourcesRequest,
    SendScoutRequest, SendSettlersRequest, TrainUnitsRequest, UpgradeBuildingRequest,
};

#[async_trait]
pub trait VillageCommandsPort: Send + Sync {
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
    async fn create_marketplace_offer(
        &self,
        request: CreateMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError>;
    async fn accept_marketplace_offer(
        &self,
        request: AcceptMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError>;
    async fn cancel_marketplace_offer(
        &self,
        request: CancelMarketplaceOfferRequest,
    ) -> Result<(), ApplicationError>;
    async fn create_hero(&self, request: CreateHeroRequest) -> Result<(), ApplicationError>;
    async fn revive_hero(&self, request: ReviveHeroRequest) -> Result<(), ApplicationError>;
}
