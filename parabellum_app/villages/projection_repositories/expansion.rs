//! Expansion projection snapshots.

/// Culture-point read snapshot used by expansion read use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionCultureSnapshot {
    pub village_culture_points_production: u32,
    pub player_culture_points_production: u32,
    pub player_village_count: usize,
}

/// Ownership read snapshot used by expansion validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionOwnershipSnapshot {
    pub source_child_villages: u8,
    pub player_village_count: usize,
}
