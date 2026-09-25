//! Queue rules shared by building commands and read previews.
use parabellum_game::models::buildings::get_building_data;
use parabellum_types::{buildings::BuildingName, errors::GameError, tribe::Tribe};

/// Maximum committed building actions for a tribe, including downgrades.
pub fn building_queue_capacity(tribe: &Tribe) -> usize {
    if matches!(tribe, Tribe::Roman) { 3 } else { 2 }
}

/// Validates a construction candidate against one already committed building.
pub fn ensure_queued_building_allows(
    candidate: &BuildingName,
    queued: &BuildingName,
) -> Result<(), GameError> {
    let candidate_data = get_building_data(candidate)?;
    if candidate == queued && !candidate_data.rules.allow_multiple {
        return Err(GameError::NoMultipleBuildingConstraint(candidate.clone()));
    }
    let conflicts = candidate_data
        .rules
        .conflicts
        .iter()
        .any(|conflict| conflict.0 == *queued)
        || get_building_data(queued).is_ok_and(|data| {
            data.rules
                .conflicts
                .iter()
                .any(|conflict| conflict.0 == *candidate)
        });
    if conflicts {
        return Err(GameError::BuildingConflict(
            candidate.clone(),
            queued.clone(),
        ));
    }
    Ok(())
}

/// Whether committed construction makes an empty-slot option unavailable.
pub fn queued_building_conflicts(candidate: &BuildingName, queued: &BuildingName) -> bool {
    ensure_queued_building_allows(candidate, queued).is_err()
}
