use uuid::Uuid;

use parabellum_app::jobs::JobPayload;

#[derive(Debug, Clone, toasty::Model)]
#[table = "jobs"]
pub struct JobRecord {
    #[key]
    pub id: Uuid,

    #[index]
    pub player_id: Uuid,

    #[index]
    pub village_id: i32,

    #[serialize(json)]
    pub task: JobPayload,

    pub status: String,

    pub completed_at: jiff::Timestamp,
    pub created_at: jiff::Timestamp,
    pub updated_at: jiff::Timestamp,
}
