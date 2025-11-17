use thiserror::Error;

/// Errors for app logic.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("No job handler for {0}")]
    NoJobHandler(String),

    #[error("Wrong password")]
    PasswordError,

    #[error(transparent)]
    PasswordHash(#[from] argon2::password_hash::Error),
}
