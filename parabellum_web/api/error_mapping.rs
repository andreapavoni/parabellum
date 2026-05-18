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
        ApplicationError::Game(game_err) => match game_err {
            GameError::VillageNotOwned { .. } => {
                ApiError::not_found("Village not available for the current player")
            }
            GameError::InvalidValley(_) | GameError::TargetOccupied => {
                ApiError::unprocessable("Target field is not available")
            }
            _ => ApiError::unprocessable(game_err.to_string()),
        },
        ApplicationError::App(app_err) => match app_err {
            AppError::WrongAuthCredentials | AppError::PasswordError => {
                ApiError::unauthorized("Invalid credentials")
            }
            AppError::QueueLimitReached { .. } | AppError::QueueItemAlreadyQueued { .. } => {
                ApiError::conflict(app_err.to_string())
            }
            AppError::InvalidAggregateTarget { .. } => ApiError::bad_request(app_err.to_string()),
            _ => internal_error(context, app_err),
        },
        other => internal_error(context, other),
    }
}
