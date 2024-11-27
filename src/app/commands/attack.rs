use std::sync::Arc;

use anyhow::Result;

use super::Command;
use crate::{
    app::events::GameEvent,
    app::jobs::{Job, JobTask},
    game::{battle::CataTargets, models::army::Army},
    repository::Repository,
};

pub struct AttackCommand {
    repo: Arc<dyn Repository>,
    village_id: u32,
    army: Army,
    cata_targets: CataTargets,
    defender_village_id: u32,
}

impl AttackCommand {
    pub fn new(
        repo: Arc<dyn Repository>,
        village_id: u32,
        army: Army,
        cata_targets: CataTargets,
        defender_village_id: u32,
    ) -> Self {
        Self {
            repo: repo.clone(),
            village_id,
            army,
            cata_targets,
            defender_village_id,
        }
    }
}

#[async_trait::async_trait]
impl Command for AttackCommand {
    async fn run(&self) -> Result<Vec<GameEvent>> {
        let attacker_village = self.repo.get_village_by_id(self.village_id).await?;
        let defender_village = self
            .repo
            .get_village_by_id(self.defender_village_id)
            .await?;

        let speed = self.army.clone().speed();
        let time_secs =
            attacker_village.calculate_travel_time_secs(defender_village.position, speed) as u64;

        let job = Job::new(
            attacker_village.player_id.clone(),
            self.village_id.clone(),
            time_secs,
            JobTask::Attack {
                army: self.army.clone(),
                cata_targets: self.cata_targets.clone(),
                village_id: self.defender_village_id.clone(),
                player_id: defender_village.player_id.clone(),
            },
        );

        Ok(vec![
            GameEvent::JobEnqueued(job),
            GameEvent::ArmyDeployed {
                army: self.army.clone(),
                village_id: self.village_id.clone(),
            },
        ])
    }
}
