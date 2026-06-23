//! Village army read models.
//!
//! Army views summarize home, deployed, reinforced, and trapped armies for
//! app-facing reads. Domain combat and army calculations remain in
//! `parabellum_game`.

use parabellum_game::models::army::Army;

/// Full army state visible from a village perspective.
#[derive(Debug, Clone)]
pub struct VillageArmyStateView {
    pub home_army: Option<Army>,
    pub reinforcements: Vec<Army>,
    pub deployed_armies: Vec<Army>,
    pub trapped_here: Vec<Army>,
    pub trapped_away: Vec<Army>,
}
