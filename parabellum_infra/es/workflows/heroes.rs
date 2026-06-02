use mini_cqrs_es::CqrsError;
use parabellum_app::villages::VillageEvent;
use parabellum_app::villages::models::{
    HeroRevivalWorkflow, ScheduledAction, ScheduledActionPayload,
};
use parabellum_game::models::hero::Hero;
use uuid::Uuid;

pub(crate) fn revival_scheduled_action(
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

pub(crate) fn revival_workflow(
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

pub(crate) fn revived_fact(action_id: Uuid, workflow: HeroRevivalWorkflow) -> (u32, VillageEvent) {
    let HeroRevivalWorkflow {
        village_id,
        player_id,
        mut hero,
        reset,
        revive_at,
    } = workflow;

    hero.resurrect(village_id, reset);
    (
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
