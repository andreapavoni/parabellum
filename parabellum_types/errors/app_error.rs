use thiserror::Error;

/// Errors for app logic.
#[derive(Debug, Error, Clone)]
pub enum AppError {
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

    #[error("Invalid aggregate target: expected village {expected}, got village {actual}")]
    InvalidAggregateTarget { expected: u32, actual: u32 },
}
