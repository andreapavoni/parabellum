use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    MarketplaceOfferModel, MerchantArrivalWorkflow, MerchantReturnWorkflow, ScheduledAction,
    ScheduledActionPayload, VillageModel,
};
use parabellum_game::models::village::VillageStocks;
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

use crate::es::VillageEsService;

pub(crate) struct OfferAcceptanceWorkflowInput<'a> {
    pub offer: &'a MarketplaceOfferModel,
    pub accepting_player_id: Uuid,
    pub accepting_village_id: u32,
    pub accepting_stocks: VillageStocks,
    pub accepting_busy_merchants: u8,
    pub accepting_merchants_used: u8,
    pub accepted_at: chrono::DateTime<chrono::Utc>,
    pub owner_arrives_at: chrono::DateTime<chrono::Utc>,
    pub accepting_arrives_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) struct ScheduledMerchantTrip {
    pub(crate) arrival_action: ScheduledAction,
    pub(crate) return_action: ScheduledAction,
}

#[allow(clippy::too_many_arguments)]
fn arrival_scheduled_action(
    action_id: Uuid,
    village_id: u32,
    source_village_id: u32,
    target_village_id: u32,
    player_id: Uuid,
    resources: ResourceGroup,
    merchants_used: u8,
    arrives_at: chrono::DateTime<chrono::Utc>,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        arrives_at,
        ScheduledActionPayload::MerchantsArrival {
            workflow: MerchantArrivalWorkflow {
                village_id,
                source_village_id,
                target_village_id,
                player_id,
                resources,
                merchants_used,
                arrives_at,
            },
        },
    )
}

fn return_scheduled_action(
    action_id: Uuid,
    village_id: u32,
    source_village_id: u32,
    target_village_id: Option<u32>,
    player_id: Uuid,
    merchants_used: u8,
    returns_at: chrono::DateTime<chrono::Utc>,
) -> Result<ScheduledAction, CqrsError> {
    super::scheduled_action(
        action_id,
        returns_at,
        ScheduledActionPayload::MerchantsReturn {
            workflow: MerchantReturnWorkflow {
                village_id,
                source_village_id,
                target_village_id,
                player_id,
                merchants_used,
                returns_at,
            },
        },
    )
}

pub(crate) fn scheduled_trip_from_event(
    event: &VillageEvent,
) -> Result<ScheduledMerchantTrip, CqrsError> {
    let VillageEvent::MerchantsTripScheduled {
        arrival_action_id,
        return_action_id,
        player_id,
        source_village_id,
        target_village_id,
        resources,
        merchants_used,
        arrives_at,
        returns_at,
        ..
    } = event
    else {
        unreachable!("scheduled_trip_from_event called with non-MerchantsTripScheduled event");
    };

    Ok(ScheduledMerchantTrip {
        arrival_action: arrival_scheduled_action(
            *arrival_action_id,
            *source_village_id,
            *source_village_id,
            *target_village_id,
            *player_id,
            resources.clone(),
            *merchants_used,
            *arrives_at,
        )?,
        return_action: return_scheduled_action(
            *return_action_id,
            *source_village_id,
            *source_village_id,
            Some(*target_village_id),
            *player_id,
            *merchants_used,
            *returns_at,
        )?,
    })
}

pub(crate) fn return_events(
    action_id: Uuid,
    workflow: MerchantReturnWorkflow,
) -> super::WorkflowEvents {
    super::WorkflowEvents::one(
        workflow.source_village_id,
        VillageEvent::MerchantsReturned {
            action_id,
            player_id: workflow.player_id,
            source_village_id: workflow.source_village_id,
            merchants_used: workflow.merchants_used,
            returns_at: workflow.returns_at,
        },
    )
}

pub(crate) async fn arrival_events(
    svc: &VillageEsService,
    action_id: Uuid,
    workflow: MerchantArrivalWorkflow,
) -> Result<super::WorkflowEvents, CqrsError> {
    let target = svc.get_village(workflow.target_village_id).await?;
    Ok(arrival_events_from_target(action_id, workflow, &target))
}

fn arrival_events_from_target(
    action_id: Uuid,
    workflow: MerchantArrivalWorkflow,
    target: &VillageModel,
) -> super::WorkflowEvents {
    let mut target_village = parabellum_game::models::village::Village::from(target.clone());
    target_village.store_resources(&workflow.resources);
    let target_stocks = target_village.stocks().clone();

    super::WorkflowEvents::from_events(vec![
        (
            workflow.source_village_id,
            VillageEvent::MerchantsArrived {
                action_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                resources: workflow.resources.clone(),
                merchants_used: workflow.merchants_used,
                arrives_at: workflow.arrives_at,
            },
        ),
        (
            workflow.target_village_id,
            VillageEvent::MerchantTransferAppliedToVillage {
                action_id,
                player_id: workflow.player_id,
                source_village_id: workflow.source_village_id,
                target_village_id: workflow.target_village_id,
                resources: workflow.resources,
                merchants_used: workflow.merchants_used,
                arrives_at: workflow.arrives_at,
                target_stocks,
            },
        ),
    ])
}

pub(crate) fn offer_acceptance_events(
    input: OfferAcceptanceWorkflowInput<'_>,
) -> super::WorkflowEvents {
    let offer = input.offer;
    let owner_trip_duration =
        (input.owner_arrives_at - input.accepted_at).max(chrono::Duration::seconds(1));
    let accepting_trip_duration =
        (input.accepting_arrives_at - input.accepted_at).max(chrono::Duration::seconds(1));

    super::WorkflowEvents::from_events(vec![
        (
            input.accepting_village_id,
            VillageEvent::MarketplaceOfferAcceptanceAppliedToVillage {
                offer_id: offer.offer_id,
                player_id: input.accepting_player_id,
                village_id: input.accepting_village_id,
                stocks: input.accepting_stocks,
                busy_merchants: input.accepting_busy_merchants,
                applied_at: input.accepted_at,
            },
        ),
        (
            input.accepting_village_id,
            VillageEvent::MarketplaceOfferAccepted {
                offer_id: offer.offer_id,
                owner_player_id: offer.owner_player_id,
                owner_village_id: offer.owner_village_id,
                accepting_player_id: input.accepting_player_id,
                accepting_village_id: input.accepting_village_id,
                offer_resources: offer.offer_resources,
                seek_resources: offer.seek_resources,
                owner_merchants_reserved: offer.merchants_reserved,
                accepting_merchants_used: input.accepting_merchants_used,
                accepted_at: input.accepted_at,
            },
        ),
        (
            offer.owner_village_id,
            VillageEvent::MerchantsTripScheduled {
                arrival_action_id: Uuid::new_v4(),
                return_action_id: Uuid::new_v4(),
                player_id: offer.owner_player_id,
                source_village_id: offer.owner_village_id,
                target_village_id: input.accepting_village_id,
                resources: offer.offer_resources.into(),
                merchants_used: offer.merchants_reserved,
                resources_already_reserved: true,
                arrives_at: input.owner_arrives_at,
                returns_at: input.owner_arrives_at + owner_trip_duration,
            },
        ),
        (
            input.accepting_village_id,
            VillageEvent::MerchantsTripScheduled {
                arrival_action_id: Uuid::new_v4(),
                return_action_id: Uuid::new_v4(),
                player_id: input.accepting_player_id,
                source_village_id: input.accepting_village_id,
                target_village_id: offer.owner_village_id,
                resources: offer.seek_resources.into(),
                merchants_used: input.accepting_merchants_used,
                resources_already_reserved: true,
                arrives_at: input.accepting_arrives_at,
                returns_at: input.accepting_arrives_at + accepting_trip_duration,
            },
        ),
    ])
}
