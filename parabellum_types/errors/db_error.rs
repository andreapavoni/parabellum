use thiserror::Error;
use uuid::Uuid;

/// Errors for db stuff.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Village with ID {0} not found")]
    VillageNotFound(u32),

    #[error("User with with email '{0}' not found")]
    UserByEmailNotFound(String),

    #[error("World Map hasn't been initialized")]
    WorldMapNotInitialized,

    #[error("User with with ID {0} not found")]
    UserByIdNotFound(Uuid),

    #[error("Army with ID {0} not found")]
    ArmyNotFound(Uuid),

    #[error("Hero with ID {0} doen't have an army")]
    HeroWithoutArmy(Uuid),

    #[error("Hero with ID {0} not found")]
    HeroNotFound(Uuid),

    #[error("Player with ID {0} not found")]
    PlayerNotFound(Uuid),

    #[error("Player with User ID {0} not found")]
    UserPlayerNotFound(Uuid),

    #[error("Job with ID {0} not found")]
    JobNotFound(Uuid),

    #[error("Marketplace Offer with ID {0} not found")]
    MarketplaceOfferNotFound(Uuid),

    #[error("MapField with ID {0} not found")]
    MapFieldNotFound(u32),

    #[error("Player with ID {0} does not own village with ID {1}")]
    PlayerDoesNotOwnVillage(Uuid, u32),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
