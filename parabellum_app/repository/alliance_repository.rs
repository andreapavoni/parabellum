use parabellum_types::errors::ApplicationError;
use parabellum_game::models::alliance::{Alliance, AllianceInvite, AllianceLog, AllianceDiplomacy};
use parabellum_game::models::player::Player;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait AllianceRepository: Send + Sync {
    async fn save(&self, alliance: &Alliance) -> Result<(), ApplicationError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Alliance, ApplicationError>;
    async fn get_by_tag(&self, tag: String) -> Result<Alliance, ApplicationError>;
    async fn get_by_name(&self, name: String) -> Result<Alliance, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn update(&self, alliance: &Alliance) -> Result<(), ApplicationError>;
    async fn get_leader(&self, alliance_id: Uuid) -> Result<Player, ApplicationError>;
    async fn count_members(&self, alliance_id: Uuid) -> Result<i64, ApplicationError>;
    async fn list_members(&self, alliance_id: Uuid) -> Result<Vec<Player>, ApplicationError>;
}

#[async_trait::async_trait]
pub trait AllianceInviteRepository: Send + Sync {
    async fn save(&self, invite: &AllianceInvite) -> Result<(), ApplicationError>;
    async fn get_by_id(&self, id: Uuid) -> Result<AllianceInvite, ApplicationError>;
    async fn get_by_player_id(&self, player_id: Uuid) -> Result<Vec<AllianceInvite>, ApplicationError>;
    async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<AllianceInvite>, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
}

#[async_trait::async_trait]
pub trait AllianceLogRepository: Send + Sync {
    async fn save(&self, log: &AllianceLog) -> Result<(), ApplicationError>;
    async fn get_by_alliance_id(&self, alliance_id: Uuid, limit: i32, offset: i32) -> Result<Vec<AllianceLog>, ApplicationError>;
}

#[async_trait::async_trait]
pub trait AllianceDiplomacyRepository: Send + Sync {
    async fn save(&self, diplomacy: &AllianceDiplomacy) -> Result<(), ApplicationError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<AllianceDiplomacy>, ApplicationError>;
    async fn get_by_alliance_id(&self, alliance_id: Uuid) -> Result<Vec<AllianceDiplomacy>, ApplicationError>;
    async fn get_between_alliances(&self, alliance1_id: Uuid, alliance2_id: Uuid) -> Result<Option<AllianceDiplomacy>, ApplicationError>;
    async fn update(&self, diplomacy: &AllianceDiplomacy) -> Result<(), ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
}
