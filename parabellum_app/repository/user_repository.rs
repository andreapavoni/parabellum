use uuid::Uuid;

use parabellum_types::common::User;
use parabellum_types::errors::ApplicationError;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    /// Saves a user.
    async fn save(&self, email: String, password_hash: String) -> Result<(), ApplicationError>;

    /// Find user by email.
    async fn get_by_email(&self, email: &String) -> Result<User, ApplicationError>;

    /// Find user by id.
    async fn get_by_id(&self, user_id: Uuid) -> Result<User, ApplicationError>;
}
