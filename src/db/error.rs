use thiserror::Error;
use uuid::Uuid;

/// Errors for db stuff.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Village with ID {0} not found")]
    VillageNotFound(u32),

    #[error("Army with ID {0} not found")]
    ArmyNotFound(Uuid),

    #[error("Player with ID {0} does not own village with ID {1}")]
    PlayerDoesNotOwnVillage(Uuid, u32),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
