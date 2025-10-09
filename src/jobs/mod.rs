pub mod handler;
pub mod tasks;
pub mod worker;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::jobs::tasks::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobTask {
    Attack(AttackTask),
    // Raid(RaidPayload), // similar to AttackPayload
    // Reinforcement(ReinforcementPayload), // similar to AttackPayload
    ArmyReturn(ArmyReturnTask),
    TrainUnits(TrainUnitsTask),
}

impl Job {
    pub fn new(player_id: Uuid, village_id: i32, duration: i64, task: JobTask) -> Self {
        let id = Uuid::new_v4();
        let now = Utc::now();

        Self {
            id,
            player_id,
            village_id,
            task,
            status: JobStatus::Pending,
            completed_at: now + Duration::new(duration, 0).unwrap(),
            created_at: now,
            updated_at: now,
        }
    }
}
#[derive(Debug, Clone)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: i32,
    pub task: JobTask,
    pub status: JobStatus,
    pub completed_at: DateTime<Utc>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
