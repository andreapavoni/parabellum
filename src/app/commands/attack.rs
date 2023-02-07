use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use super::Command;
use crate::{
    app::events::GameEvent,
    app::jobs::{Job, JobTask},
    game::{battle::CataTargets, models::army::TroopSet},
};

#[derive(Debug, Clone)]
pub struct AttackCommand {
    village_id: u32,
    units: TroopSet,
    cata_targets: CataTargets,
    defender_village_id: u32,
}

impl AttackCommand {
    pub fn new(
        village_id: u32,
        units: TroopSet,
        cata_targets: CataTargets,
        defender_village_id: u32,
    ) -> Self {
        Self {
            village_id,
            units,
            cata_targets,
            defender_village_id,
        }
    }
}

impl Command for AttackCommand {
    fn run(&self) -> Result<Vec<GameEvent>> {
        // TODO: load attacker and defender villages from db

        let job = Job::new(
            Uuid::new_v4(), // village.player_id.clone(),
            self.village_id.clone(),
            Utc::now(),
            Utc::now(), // FIXME: calculate arrival time
            JobTask::Attack {
                units: self.units.clone(),
                cata_targets: self.cata_targets.clone(),
                village_id: self.defender_village_id.clone(),
                player_id: Uuid::new_v4(), //, defender_village.player_id.clone(),
            },
        );

        Ok(vec![
            GameEvent::JobEnqueued(job),
            GameEvent::ArmyDeployed {
                units: self.units.clone(),
                village_id: self.village_id.clone(),
            },
        ])
    }
}
