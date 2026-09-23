use parabellum_types::errors::{AppError, ApplicationError, DbError, GameError};

use crate::api::errors::ApiError;

pub(crate) fn internal_error(context: &'static str, err: impl std::fmt::Display) -> ApiError {
    tracing::error!(context = context, error = %err, "api internal error");
    ApiError::internal("Internal server error")
}

pub(crate) fn map_application_error(context: &'static str, err: ApplicationError) -> ApiError {
    match err {
        ApplicationError::Db(db_err) => match db_err {
            DbError::VillageNotFound(_) => ApiError::not_found("Village not found"),
            DbError::PlayerNotFound(_) => ApiError::not_found("Player not found"),
            DbError::ArmyNotFound(_) => ApiError::not_found("Army not found"),
            DbError::JobNotFound(_) => ApiError::not_found("Troop movement not found"),
            DbError::HeroNotFound(_) => ApiError::not_found("Hero not found"),
            DbError::MarketplaceOfferNotFound(_) => {
                ApiError::not_found("Marketplace offer not found")
            }
            DbError::MapFieldNotFound(_) => ApiError::not_found("Map field not found"),
            DbError::UserByIdNotFound(_) | DbError::UserByEmailNotFound(_) => {
                ApiError::not_found("User not found")
            }
            DbError::UserPlayerNotFound(_) => ApiError::not_found("Player not found"),
            DbError::PlayerDoesNotOwnVillage(_, _) => {
                ApiError::not_found("Village not available for the current player")
            }
            _ => internal_error(context, db_err),
        },
        ApplicationError::Game(game_err) => {
            tracing::warn!(context = context, error = %game_err, "api domain error");
            match game_err {
                GameError::VillageNotOwned { .. } => {
                    ApiError::not_found("Village not available for the current player")
                }
                GameError::InvalidMarketplaceOffer | GameError::MarketplaceOfferNoLongerValid => {
                    ApiError::conflict(game_err.to_string())
                }
                GameError::InvalidValley(_) | GameError::TargetOccupied => {
                    ApiError::unprocessable("Target field is not available")
                }
                _ => ApiError::unprocessable(game_err.to_string()),
            }
        }
        ApplicationError::App(app_err) => {
            tracing::warn!(context = context, error = %app_err, "api application error");
            match app_err {
                AppError::WrongAuthCredentials | AppError::PasswordError => {
                    ApiError::unauthorized("Invalid credentials")
                }
                AppError::OptimisticConflict { .. } => {
                    ApiError::conflict("State changed; retry the operation")
                }
                AppError::QueueLimitReached { .. } | AppError::QueueItemAlreadyQueued { .. } => {
                    ApiError::conflict(app_err.to_string())
                }
                AppError::InvalidAggregateTarget { .. } => {
                    ApiError::bad_request(app_err.to_string())
                }
                _ => internal_error(context, app_err),
            }
        }
        ApplicationError::Unknown(message) => {
            internal_error(context, ApplicationError::Unknown(message))
        }
        other => internal_error(context, other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, response::IntoResponse};
    #[test]
    fn conflicts_absence_and_storage_failures_keep_distinct_http_statuses() {
        for (error, status) in [
            (
                ApplicationError::App(AppError::OptimisticConflict {
                    expected_version: 1,
                    actual_version: 2,
                }),
                StatusCode::CONFLICT,
            ),
            (
                ApplicationError::Db(DbError::VillageNotFound(1)),
                StatusCode::NOT_FOUND,
            ),
            (
                ApplicationError::Db(DbError::Database(sqlx::Error::PoolTimedOut)),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ] {
            assert_eq!(
                map_application_error("test", error)
                    .into_response()
                    .status(),
                status
            );
        }
    }
}
