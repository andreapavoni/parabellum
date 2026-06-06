use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    HeroRevivalWorkflow, ScheduledAction, ScheduledActionPayload,
};
use parabellum_game::models::hero::Hero;
use parabellum_types::errors::GameError;
use uuid::Uuid;

use crate::es::VillageEsService;

fn revival_scheduled_action(
    action_id: Uuid,
    workflow: HeroRevivalWorkflow,
) -> Result<ScheduledAction, CqrsError> {
    let execute_at = workflow.revive_at;
    super::scheduled_action(
        action_id,
        execute_at,
        ScheduledActionPayload::HeroRevival { workflow },
    )
}

fn revival_workflow(
    village_id: u32,
    player_id: Uuid,
    hero: Hero,
    reset: bool,
    revive_at: chrono::DateTime<chrono::Utc>,
) -> HeroRevivalWorkflow {
    HeroRevivalWorkflow {
        village_id,
        player_id,
        hero,
        reset,
        revive_at,
    }
}

pub(crate) fn revival_scheduled_action_from_event(
    event: &VillageEvent,
) -> Result<ScheduledAction, CqrsError> {
    let VillageEvent::HeroRevivalScheduled {
        action_id,
        player_id,
        village_id,
        hero,
        reset,
        revive_at,
        ..
    } = event
    else {
        unreachable!(
            "revival_scheduled_action_from_event called with non-HeroRevivalScheduled event"
        );
    };

    revival_scheduled_action(
        *action_id,
        revival_workflow(*village_id, *player_id, hero.clone(), *reset, *revive_at),
    )
}

pub(crate) async fn revival_events(
    svc: &VillageEsService,
    action_id: Uuid,
    workflow: HeroRevivalWorkflow,
) -> Result<super::WorkflowEvents, CqrsError> {
    validate_revival(svc, &workflow).await?;
    Ok(revived_events(action_id, workflow))
}

async fn validate_revival(
    svc: &VillageEsService,
    workflow: &HeroRevivalWorkflow,
) -> Result<(), CqrsError> {
    let village = svc.get_village(workflow.village_id).await?;
    if village.player_id != workflow.player_id {
        return Err(CqrsError::domain_source(GameError::VillageNotOwned {
            village_id: workflow.village_id,
            player_id: workflow.player_id,
        }));
    }
    if workflow.hero.player_id != workflow.player_id {
        return Err(CqrsError::domain_source(GameError::HeroNotOwned {
            hero_id: workflow.hero.id,
            player_id: workflow.player_id,
        }));
    }

    Ok(())
}

fn revived_events(action_id: Uuid, workflow: HeroRevivalWorkflow) -> super::WorkflowEvents {
    let HeroRevivalWorkflow {
        village_id,
        player_id,
        mut hero,
        reset,
        revive_at,
    } = workflow;

    hero.resurrect(village_id, reset);
    super::WorkflowEvents::one(
        village_id,
        VillageEvent::HeroRevived {
            action_id,
            player_id,
            village_id,
            hero,
            reset,
            revived_at: revive_at,
        },
    )
}
