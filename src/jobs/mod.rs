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

        let completed_at = now.checked_add_signed(Duration::seconds(duration)).unwrap();

        Self {
            id,
            player_id,
            village_id,
            task,
            status: JobStatus::Pending,
            completed_at,
            created_at: now,
            updated_at: now,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::models::buildings::BuildingName;
    use chrono::Duration;
    use uuid::Uuid;

    // Helper function to create a dummy AttackTask
    fn create_dummy_attack_task() -> JobTask {
        JobTask::Attack(AttackTask {
            army_id: Uuid::new_v4(),
            attacker_village_id: 1,
            attacker_player_id: Uuid::new_v4(),
            target_village_id: 2,
            target_player_id: Uuid::new_v4(),
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
        })
    }

    #[test]
    fn test_job_new() {
        let player_id = Uuid::new_v4();
        let village_id = 123;
        let duration_secs: i64 = 3600; // 1 hour
        let task = create_dummy_attack_task();

        let before_creation = Utc::now();
        let job = Job::new(player_id, village_id, duration_secs, task);
        let after_creation = Utc::now();

        // Check basic properties
        assert_eq!(job.player_id, player_id);
        assert_eq!(job.village_id, village_id);
        assert_eq!(job.status, JobStatus::Pending);
        assert!(matches!(job.task, JobTask::Attack(_)));

        // Check timestamps
        assert!(job.created_at >= before_creation && job.created_at <= after_creation);
        assert_eq!(job.created_at, job.updated_at);

        // Check completed_at
        let expected_completed_at = job.created_at + Duration::seconds(duration_secs);
        assert_eq!(job.completed_at, expected_completed_at);

        // Check duration calculation is close
        let duration_diff = (job.completed_at - job.created_at).num_seconds();
        assert_eq!(duration_diff, duration_secs);
    }

    #[test]
    fn test_job_new_zero_duration() {
        let player_id = Uuid::new_v4();
        let village_id = 456;
        let duration_secs: i64 = 0; // Instant job
        let task = create_dummy_attack_task();

        let job = Job::new(player_id, village_id, duration_secs, task);

        // completed_at should be the same as created_at
        assert_eq!(job.completed_at, job.created_at);
    }
}
