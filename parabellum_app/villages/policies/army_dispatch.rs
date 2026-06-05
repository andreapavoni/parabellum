use parabellum_game::models::army::Army;
use parabellum_types::{
    army::{TroopSet, UnitRole},
    buildings::BuildingName,
    errors::GameError,
};
use uuid::Uuid;

use crate::villages::VillageState;

/// Validates and builds an army detached from a village home army.
///
/// This is the app policy for outbound troop dispatch. It owns common command
/// preconditions shared by attacks, reinforcements, and scouting while leaving
/// each command responsible for its workflow-specific events.
pub struct ArmyDispatch;

#[derive(Debug, Clone)]
pub struct ArmyDispatchRequest {
    pub army_id: Uuid,
    pub source_village_id: u32,
    pub target_village_id: u32,
    pub player_id: Uuid,
    pub units: TroopSet,
    pub hero_id: Option<Uuid>,
    pub allow_hero: bool,
    pub scouts_only: bool,
}

impl ArmyDispatch {
    pub fn detach_from_home(
        village: &VillageState,
        request: ArmyDispatchRequest,
    ) -> Result<Army, GameError> {
        if village.player_id() != request.player_id {
            return Err(GameError::VillageNotOwned {
                village_id: request.source_village_id,
                player_id: request.player_id,
            });
        }
        if request.source_village_id == request.target_village_id {
            return Err(GameError::VillageCannotTargetItself {
                village_id: request.source_village_id,
            });
        }
        if village.building_level(BuildingName::RallyPoint) == 0 {
            return Err(GameError::BuildingRequirementsNotMet {
                building: BuildingName::RallyPoint,
                level: 1,
            });
        }
        if request.units.immensity() == 0 && request.hero_id.is_none() {
            return Err(GameError::NoUnitsSelected);
        }
        if !request.allow_hero && request.hero_id.is_some() {
            return Err(GameError::NoUnitsSelected);
        }
        if request.scouts_only {
            validate_scout_units(village, &request.units)?;
        }

        let Some(home_army) = village.village.army() else {
            return Err(if request.hero_id.is_some() {
                GameError::NoArmyInVillage
            } else {
                GameError::NotEnoughUnits
            });
        };
        let mut home_army = home_army.clone();
        let mut detached_army =
            home_army.split_units(request.units, request.hero_id, request.source_village_id)?;
        detached_army.id = request.army_id;
        detached_army.current_map_field_id = Some(request.target_village_id);

        Ok(detached_army)
    }
}

fn validate_scout_units(village: &VillageState, units: &TroopSet) -> Result<(), GameError> {
    if !village
        .village
        .tribe
        .troop_set_contains_only_role(units, UnitRole::Scout)?
    {
        return Err(GameError::OnlyScoutUnitsAllowed);
    }

    Ok(())
}
