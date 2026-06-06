use parabellum_game::models::army::Army;
use parabellum_types::{army::TroopSet, errors::GameError};
use uuid::Uuid;

/// Validates control over a stationed reinforcement army and builds a returning
/// partial army.
///
/// Recall and release commands differ in which aggregate owns the action, but
/// both use the same unit and hero selection rules once infrastructure has
/// loaded the stationed reinforcement army.
pub struct ReinforcementControl;

impl ReinforcementControl {
    pub fn returning_army(
        reinforcement_army: &Army,
        units: &TroopSet,
        hero_id: Option<Uuid>,
        stationed_village_id: u32,
    ) -> Result<Army, GameError> {
        if units.immensity() == 0 {
            return Err(GameError::NoUnitsSelected);
        }

        let mut reinforcement_army = reinforcement_army.clone();
        reinforcement_army.split_units(units.clone(), hero_id, stationed_village_id)
    }
}
