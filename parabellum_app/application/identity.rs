use parabellum_types::common::{Player, User};
use parabellum_types::errors::ApplicationError;
use uuid::Uuid;

use crate::ports::identity::RegisterPlayerRequest;

use super::GameApplication;

pub async fn register_player(
    app: &GameApplication,
    request: RegisterPlayerRequest,
) -> Result<(), ApplicationError> {
    app.identity_port().register_player(request).await
}

pub async fn authenticate_user(
    app: &GameApplication,
    email: &str,
    password: &str,
) -> Result<User, ApplicationError> {
    app.identity_port().authenticate_user(email, password).await
}

pub async fn get_user_by_email(
    app: &GameApplication,
    email: &str,
) -> Result<User, ApplicationError> {
    app.identity_port().get_user_by_email(email).await
}

pub async fn get_user_by_id(
    app: &GameApplication,
    user_id: Uuid,
) -> Result<User, ApplicationError> {
    app.identity_port().get_user_by_id(user_id).await
}

pub async fn get_player_by_user_id(
    app: &GameApplication,
    user_id: Uuid,
) -> Result<Player, ApplicationError> {
    app.identity_port().get_player_by_user_id(user_id).await
}

pub async fn get_player_by_id(
    app: &GameApplication,
    player_id: Uuid,
) -> Result<Player, ApplicationError> {
    app.identity_port().get_player_by_id(player_id).await
}
