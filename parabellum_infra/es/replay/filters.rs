//! Replay target filtering.

use mini_cqrs_es::{CqrsError, StoredEvent};
use parabellum_app::villages::VillageEvent;

use super::ReplayTarget;

pub(super) fn accepts_event(target: ReplayTarget, event: &StoredEvent) -> Result<bool, CqrsError> {
    if !event.aggregate_type.contains("VillageAggregate") {
        return Ok(false);
    }

    let domain_event = event.get_payload::<VillageEvent>()?;
    Ok(match target {
        ReplayTarget::Village | ReplayTarget::All => true,
        ReplayTarget::Reports => is_report_event(&domain_event),
    })
}

pub(super) fn is_report_event(event: &VillageEvent) -> bool {
    matches!(
        event,
        VillageEvent::ReinforcementArrived { .. }
            | VillageEvent::MerchantsArrived { .. }
            | VillageEvent::ScoutBattleResolved { .. }
            | VillageEvent::AttackBattleResolved { .. }
            | VillageEvent::ReportMarkedAsRead { .. }
    )
}
