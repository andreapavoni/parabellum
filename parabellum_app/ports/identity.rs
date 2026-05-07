use async_trait::async_trait;
use parabellum_game::models::map::MapQuadrant;
use parabellum_types::common::{Player, User};
use parabellum_types::errors::ApplicationError;
use parabellum_types::tribe::Tribe;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RegisterPlayerRequest {
    pub player_id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub tribe: Tribe,
    pub quadrant: MapQuadrant,
}

#[async_trait]
pub trait IdentityPort: Send + Sync {
    async fn register_player(&self, request: RegisterPlayerRequest)
    -> Result<(), ApplicationError>;
    async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, ApplicationError>;
    async fn get_user_by_email(&self, email: &str) -> Result<User, ApplicationError>;
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError>;
    async fn get_player_by_user_id(&self, user_id: Uuid) -> Result<Player, ApplicationError>;
    async fn get_player_by_id(&self, player_id: Uuid) -> Result<Player, ApplicationError>;
}
