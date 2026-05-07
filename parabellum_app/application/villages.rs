use parabellum_types::errors::ApplicationError;

use crate::ports::villages::{
    AcceptMarketplaceOfferRequest, AddBuildingRequest, CancelMarketplaceOfferRequest,
    CreateMarketplaceOfferRequest, RecallReinforcementsRequest, ReleaseReinforcementsRequest,
    ResearchAcademyRequest, ResearchSmithyRequest, SendAttackRequest, SendReinforcementRequest,
    SendResourcesRequest, SendScoutRequest, SendSettlersRequest, TrainUnitsRequest,
    UpgradeBuildingRequest,
};

use super::GameApplication;

pub async fn send_resources(
    app: &GameApplication,
    request: SendResourcesRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().send_resources(request).await
}

pub async fn train_units(
    app: &GameApplication,
    request: TrainUnitsRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().train_units(request).await
}

pub async fn add_building(
    app: &GameApplication,
    request: AddBuildingRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().add_building(request).await
}

pub async fn upgrade_building(
    app: &GameApplication,
    request: UpgradeBuildingRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().upgrade_building(request).await
}

pub async fn research_academy(
    app: &GameApplication,
    request: ResearchAcademyRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().research_academy(request).await
}

pub async fn research_smithy(
    app: &GameApplication,
    request: ResearchSmithyRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().research_smithy(request).await
}

pub async fn send_reinforcement(
    app: &GameApplication,
    request: SendReinforcementRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().send_reinforcement(request).await
}

pub async fn send_attack(
    app: &GameApplication,
    request: SendAttackRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().send_attack(request).await
}

pub async fn send_scout(
    app: &GameApplication,
    request: SendScoutRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().send_scout(request).await
}

pub async fn send_settlers(
    app: &GameApplication,
    request: SendSettlersRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().send_settlers(request).await
}

pub async fn recall_reinforcements(
    app: &GameApplication,
    request: RecallReinforcementsRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().recall_reinforcements(request).await
}

pub async fn release_reinforcements(
    app: &GameApplication,
    request: ReleaseReinforcementsRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().release_reinforcements(request).await
}

pub async fn create_marketplace_offer(
    app: &GameApplication,
    request: CreateMarketplaceOfferRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().create_offer(request).await
}

pub async fn accept_marketplace_offer(
    app: &GameApplication,
    request: AcceptMarketplaceOfferRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().accept_offer(request).await
}

pub async fn cancel_marketplace_offer(
    app: &GameApplication,
    request: CancelMarketplaceOfferRequest,
) -> Result<(), ApplicationError> {
    app.villages_port().cancel_offer(request).await
}
