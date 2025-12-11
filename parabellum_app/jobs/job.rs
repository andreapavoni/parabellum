use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub player_id: Uuid,
    pub village_id: i32,
    pub task: JobPayload,
    pub status: JobStatus,
    pub completed_at: DateTime<Utc>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl Job {
    pub fn new(player_id: Uuid, village_id: i32, duration: i64, task: JobPayload) -> Self {
        let now = Utc::now();
        let completed_at = now.checked_add_signed(Duration::seconds(duration)).unwrap();
        Self::with_deadline_internal(player_id, village_id, task, completed_at, now)
    }

    pub fn with_deadline(
        player_id: Uuid,
        village_id: i32,
        task: JobPayload,
        completed_at: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self::with_deadline_internal(player_id, village_id, task, completed_at, now)
    }

    fn with_deadline_internal(
        player_id: Uuid,
        village_id: i32,
        task: JobPayload,
        completed_at: DateTime<Utc>,
        baseline: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            player_id,
            village_id,
            task,
            status: JobStatus::Pending,
            completed_at,
            created_at: baseline,
            updated_at: baseline,
        }
    }
}

/// Represents the data payload for any job, it holds data for the task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPayload {
    /// A string key used to find the correct handler registry.
    /// e.g., "Attack", "TrainUnits", "ArmyReturn"
    pub task_type: String,

    /// The full JSON data for the task payload (e.g., the serialized AttackTask).
    pub data: Value,
}

impl JobPayload {
    pub fn new(task_type: &str, data: Value) -> Self {
        Self {
            task_type: task_type.to_string(),
            data,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::tasks::*;
    use chrono::Duration;
    use serde_json::json;
    use uuid::Uuid;

    use parabellum_types::battle::AttackType;
    use parabellum_types::buildings::BuildingName;

    // Helper function to create a dummy AttackTask payload
    fn create_dummy_attack_payload() -> JobPayload {
        let task_data = AttackTask {
            army_id: Uuid::new_v4(),
            attacker_village_id: 1,
            attacker_player_id: Uuid::new_v4(),
            target_village_id: 2,
            target_player_id: Uuid::new_v4(),
            catapult_targets: [BuildingName::MainBuilding, BuildingName::Warehouse],
            attack_type: AttackType::Normal,
        };

        JobPayload {
            task_type: "Attack".to_string(),
            data: json!(task_data),
        }
    }

    #[test]
    fn test_job_new() {
        let player_id = Uuid::new_v4();
        let village_id = 123;
        let duration_secs: i64 = 3600; // 1 hour
        let task = create_dummy_attack_payload();

        let before_creation = Utc::now();
        let job = Job::new(player_id, village_id, duration_secs, task);
        let after_creation = Utc::now();

        // Check basic properties
        assert_eq!(job.player_id, player_id);
        assert_eq!(job.village_id, village_id);
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.task.task_type, "Attack");
        assert!(job.task.data.is_object());

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
        let task = create_dummy_attack_payload();

        let job = Job::new(player_id, village_id, duration_secs, task);

        // completed_at should be the same as created_at
        assert_eq!(job.completed_at, job.created_at);
    }
}
