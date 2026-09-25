//! One typed error boundary for both commands and queries.
use mini_cqrs_es::CqrsError;
use parabellum_types::errors::{AppError, ApplicationError, DbError, GameError};

pub(super) fn map_cqrs_error(error: CqrsError) -> ApplicationError {
    match error {
        CqrsError::Conflict {
            expected_version,
            actual_version,
        } => AppError::OptimisticConflict {
            expected_version,
            actual_version,
        }
        .into(),
        CqrsError::DomainSource(source) | CqrsError::CommandInvariantSource(source) => {
            let source = match source.downcast::<ApplicationError>() {
                Ok(error) => return *error,
                Err(source) => source,
            };
            let source = match source.downcast::<DbError>() {
                Ok(error) => return (*error).into(),
                Err(source) => source,
            };
            let source = match source.downcast::<sqlx::Error>() {
                Ok(error) => return DbError::Database(*error).into(),
                Err(source) => source,
            };
            let source = match source.downcast::<GameError>() {
                Ok(error) => return (*error).into(),
                Err(source) => source,
            };
            let source = match source.downcast::<AppError>() {
                Ok(error) => return (*error).into(),
                Err(source) => source,
            };
            ApplicationError::Unknown(source.to_string())
        }
        CqrsError::Other(error) => {
            let error = match error.downcast::<ApplicationError>() {
                Ok(error) => return error,
                Err(error) => error,
            };
            let error = match error.downcast::<DbError>() {
                Ok(error) => return error.into(),
                Err(error) => error,
            };
            let error = match error.downcast::<sqlx::Error>() {
                Ok(error) => return DbError::Database(error).into(),
                Err(error) => error,
            };
            let error = match error.downcast::<GameError>() {
                Ok(error) => return error.into(),
                Err(error) => error,
            };
            let error = match error.downcast::<AppError>() {
                Ok(error) => return error.into(),
                Err(error) => error,
            };
            ApplicationError::Unknown(error.to_string())
        }
        CqrsError::Serialization(error) => ApplicationError::Json(error),
        other => ApplicationError::Unknown(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_absence_storage_failures_and_conflicts() {
        assert!(matches!(
            map_cqrs_error(CqrsError::domain_source(DbError::VillageNotFound(7))),
            ApplicationError::Db(DbError::VillageNotFound(7))
        ));
        for error in [
            CqrsError::domain_source(sqlx::Error::PoolTimedOut),
            CqrsError::invariant_source(ApplicationError::Db(DbError::Database(
                sqlx::Error::PoolTimedOut,
            ))),
        ] {
            assert!(matches!(
                map_cqrs_error(error),
                ApplicationError::Db(DbError::Database(sqlx::Error::PoolTimedOut))
            ));
        }
        assert!(matches!(
            map_cqrs_error(CqrsError::Conflict {
                expected_version: 1,
                actual_version: 2
            }),
            ApplicationError::App(AppError::OptimisticConflict {
                expected_version: 1,
                actual_version: 2
            })
        ));
    }
}
