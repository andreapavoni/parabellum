use crate::{app::AppError, db::DbError, game::GameError};
use thiserror::Error;
pub type Result<T, E = ApplicationError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Game(#[from] GameError),

    #[error(transparent)]
    App(#[from] AppError),

    #[error(transparent)]
    Db(#[from] DbError),

    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    #[error("An unknown error occurred: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for ApplicationError {
    fn from(err: anyhow::Error) -> Self {
        ApplicationError::Unknown(err.to_string())
    }
}
