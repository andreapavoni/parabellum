use std::fmt::{self};
use thiserror::Error;

/// Errors for app logic (use cases, commands).
#[derive(Debug, Error)]
pub enum AppError {
    NoJobHandler(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NoJobHandler(job_task) => write!(f, "No job handler for {}", job_task),
        }
    }
}
