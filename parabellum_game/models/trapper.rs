use serde::{Deserialize, Serialize};

use parabellum_types::{army::TroopSet, buildings::BuildingName, common::ResourceGroup};

use super::village::VillageBuilding;

pub const TRAP_COST: ResourceGroup = ResourceGroup::new(20, 30, 10, 20);
pub const TRAP_BUILD_TIME_SECS: u32 = 10;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrapperState {
    pub active_traps: u32,
    pub broken_traps: u32,
    pub queued_traps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrapCaptureOutcome {
    pub trapped_units: TroopSet,
    pub traps_used: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrapFreeOutcome {
    pub units_before: TroopSet,
    pub deaths: TroopSet,
    pub survivors: TroopSet,
    pub traps_destroyed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrapBuildPlan {
    pub quantity: u32,
    pub cost: ResourceGroup,
    pub duration_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trapper {
    capacity: u32,
    occupied_traps: u32,
    state: TrapperState,
}

impl Trapper {
    pub fn from_buildings(
        buildings: &[VillageBuilding],
        state: TrapperState,
        occupied_traps: u32,
    ) -> Self {
        let capacity = buildings
            .iter()
            .filter(|building| building.building.name == BuildingName::Trapper)
            .map(|building| building.building.value)
            .sum();
        Self {
            capacity,
            occupied_traps,
            state,
        }
    }

    pub fn state(&self) -> TrapperState {
        self.state
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn occupied_traps(&self) -> u32 {
        self.occupied_traps
    }

    pub fn active_traps(&self) -> u32 {
        self.state.active_traps.min(self.capacity)
    }

    pub fn broken_traps(&self) -> u32 {
        self.state.broken_traps
    }

    pub fn queued_traps(&self) -> u32 {
        self.state.queued_traps
    }

    pub fn unbuilt_traps(&self) -> u32 {
        self.capacity.saturating_sub(
            self.active_traps()
                .saturating_add(self.occupied_traps)
                .saturating_add(self.state.broken_traps)
                .saturating_add(self.state.queued_traps),
        )
    }

    pub fn buildable_traps(&self) -> u32 {
        self.state.broken_traps.saturating_add(self.unbuilt_traps())
    }

    pub fn capture(&mut self, incoming: &TroopSet) -> TrapCaptureOutcome {
        let mut remaining = self.active_traps();
        let mut trapped = TroopSet::default();
        for (idx, quantity) in incoming.units().iter().enumerate() {
            if remaining == 0 {
                break;
            }
            let captured = (*quantity).min(remaining);
            if captured > 0 {
                trapped.set(idx, captured);
                remaining -= captured;
            }
        }
        let traps_used = trapped.immensity();
        self.state.active_traps = self.state.active_traps.saturating_sub(traps_used);
        self.occupied_traps = self.occupied_traps.saturating_add(traps_used);
        TrapCaptureOutcome {
            trapped_units: trapped,
            traps_used,
        }
    }

    pub fn release_by_owner(&mut self, released_units: &TroopSet) {
        let released = released_units.immensity();
        self.occupied_traps = self.occupied_traps.saturating_sub(released);
        self.state.active_traps = self
            .state
            .active_traps
            .saturating_add(released)
            .min(self.capacity.saturating_sub(self.occupied_traps));
    }

    pub fn free_by_attack(&mut self, trapped_units: &TroopSet) -> TrapFreeOutcome {
        let units_before = trapped_units.clone();
        let mut deaths = TroopSet::default();
        let mut survivors = TroopSet::default();
        for (idx, quantity) in trapped_units.units().iter().enumerate() {
            let dead = quantity / 4;
            deaths.set(idx, dead);
            survivors.set(idx, quantity.saturating_sub(dead));
        }
        let traps_destroyed = trapped_units.immensity();
        self.occupied_traps = self.occupied_traps.saturating_sub(traps_destroyed);
        self.state.broken_traps = self.state.broken_traps.saturating_add(traps_destroyed);
        TrapFreeOutcome {
            units_before,
            deaths,
            survivors,
            traps_destroyed,
        }
    }

    pub fn start_trap_build(&mut self, quantity: u32) -> Option<TrapBuildPlan> {
        if quantity == 0 || quantity > self.buildable_traps() {
            return None;
        }
        self.state.queued_traps = self.state.queued_traps.saturating_add(quantity);
        Some(TrapBuildPlan {
            quantity,
            cost: ResourceGroup::new(
                TRAP_COST.lumber().saturating_mul(quantity),
                TRAP_COST.clay().saturating_mul(quantity),
                TRAP_COST.iron().saturating_mul(quantity),
                TRAP_COST.crop().saturating_mul(quantity),
            ),
            duration_secs: TRAP_BUILD_TIME_SECS.saturating_mul(quantity),
        })
    }

    pub fn complete_trap_build(&mut self, quantity: u32) {
        let completed = quantity.min(self.state.queued_traps);
        self.state.queued_traps = self.state.queued_traps.saturating_sub(completed);
        let repaired = completed.min(self.state.broken_traps);
        self.state.broken_traps = self.state.broken_traps.saturating_sub(repaired);
        self.state.active_traps = self
            .state
            .active_traps
            .saturating_add(completed)
            .min(self.capacity.saturating_sub(self.occupied_traps));
    }
}

#[cfg(test)]
mod tests {
    use parabellum_types::army::TroopSet;

    use super::*;
    use crate::models::buildings::Building;

    fn trapper_building(value: u32) -> VillageBuilding {
        let mut building = Building::new(BuildingName::Trapper, 1);
        building.value = value;
        VillageBuilding {
            slot_id: 20,
            building,
        }
    }

    #[test]
    fn capture_uses_active_traps_before_combat() {
        let mut trapper = Trapper::from_buildings(
            &[trapper_building(10)],
            TrapperState {
                active_traps: 6,
                broken_traps: 0,
                queued_traps: 0,
            },
            0,
        );

        let outcome = trapper.capture(&TroopSet::new([4, 4, 0, 0, 0, 0, 0, 0, 0, 0]));

        assert_eq!(outcome.traps_used, 6);
        assert_eq!(
            outcome.trapped_units,
            TroopSet::new([4, 2, 0, 0, 0, 0, 0, 0, 0, 0])
        );
        assert_eq!(trapper.active_traps(), 0);
        assert_eq!(trapper.occupied_traps(), 6);
    }

    #[test]
    fn owner_release_restores_traps_for_free() {
        let mut trapper = Trapper::from_buildings(
            &[trapper_building(10)],
            TrapperState {
                active_traps: 4,
                broken_traps: 0,
                queued_traps: 0,
            },
            6,
        );

        trapper.release_by_owner(&TroopSet::new([6, 0, 0, 0, 0, 0, 0, 0, 0, 0]));

        assert_eq!(trapper.active_traps(), 10);
        assert_eq!(trapper.occupied_traps(), 0);
        assert_eq!(trapper.broken_traps(), 0);
    }

    #[test]
    fn attack_free_kills_quarter_and_breaks_used_traps() {
        let mut trapper = Trapper::from_buildings(
            &[trapper_building(20)],
            TrapperState {
                active_traps: 5,
                broken_traps: 0,
                queued_traps: 0,
            },
            8,
        );

        let outcome = trapper.free_by_attack(&TroopSet::new([5, 3, 0, 0, 0, 0, 0, 0, 0, 0]));

        assert_eq!(
            outcome.deaths,
            TroopSet::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        );
        assert_eq!(
            outcome.survivors,
            TroopSet::new([4, 3, 0, 0, 0, 0, 0, 0, 0, 0])
        );
        assert_eq!(trapper.occupied_traps(), 0);
        assert_eq!(trapper.broken_traps(), 8);
        assert_eq!(trapper.active_traps(), 5);
    }
}
