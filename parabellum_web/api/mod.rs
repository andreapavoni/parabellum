//! JSON API modules.
//!
//! `auth`, `game`, `actions`, and `buildings` are route handler groups.
//! `errors` and `dto` provide shared response contracts and mapping helpers.

pub mod actions;
pub mod auth;
pub mod buildings;
pub mod dto;
pub mod errors;
pub mod game;
mod helpers;

pub use helpers::{authenticated_user, bearer_token};
