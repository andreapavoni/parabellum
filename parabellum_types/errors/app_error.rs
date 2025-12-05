use thiserror::Error;

/// Errors for app logic.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("No job handler for {0}")]
    NoJobHandler(String),

    #[error("Wrong authentication credentials")]
    WrongAuthCredentials,

    #[error("Wrong password")]
    PasswordError,

    #[error(transparent)]
    PasswordHash(#[from] argon2::password_hash::Error),

    #[error("{queue} queue is full")]
    QueueLimitReached { queue: &'static str },

    #[error("{queue} queue already contains {item}")]
    QueueItemAlreadyQueued { queue: &'static str, item: String },
}
