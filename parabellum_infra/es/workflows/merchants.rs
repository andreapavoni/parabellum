use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    MerchantArrivalWorkflow, MerchantReturnWorkflow, ScheduledAction, ScheduledActionPayload,
    VillageModel,
};
use parabellum_types::common::ResourceGroup;
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub(crate) fn arrival_scheduled_action(
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

pub(crate) fn return_scheduled_action(
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

pub(crate) fn return_fact(action_id: Uuid, workflow: MerchantReturnWorkflow) -> VillageEvent {
    VillageEvent::MerchantsReturned {
        action_id,
        player_id: workflow.player_id,
        source_village_id: workflow.source_village_id,
        merchants_used: workflow.merchants_used,
        returns_at: workflow.returns_at,
    }
}

pub(crate) fn arrival_facts(
    action_id: Uuid,
    workflow: MerchantArrivalWorkflow,
    target: &VillageModel,
) -> (VillageEvent, VillageEvent) {
    let target_stocks = parabellum_game::models::village::VillageStocks {
        warehouse_capacity: target.stocks.warehouse_capacity,
        granary_capacity: target.stocks.granary_capacity,
        lumber: target
            .stocks
            .lumber
            .saturating_add(workflow.resources.lumber()),
        clay: target.stocks.clay.saturating_add(workflow.resources.clay()),
        iron: target.stocks.iron.saturating_add(workflow.resources.iron()),
        crop: target
            .stocks
            .crop
            .saturating_add(workflow.resources.crop() as i64),
    };

    (
        VillageEvent::MerchantsArrived {
            action_id,
            player_id: workflow.player_id,
            source_village_id: workflow.source_village_id,
            target_village_id: workflow.target_village_id,
            resources: workflow.resources.clone(),
            merchants_used: workflow.merchants_used,
            arrives_at: workflow.arrives_at,
        },
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
    )
}
