//! HTTP delivery layer for Parabellum.
//!
//! This crate is intentionally thin:
//! - it exposes JSON endpoints under `/api/v1/*`
//! - it serves the SPA shell + static assets
//! - it translates HTTP payloads into `parabellum_app` commands/queries
//! - it performs API auth using bearer access tokens
//!
//! Game rules and business behavior must stay in `parabellum_game` / `parabellum_app`.

pub mod api;
pub mod auth_tokens;
pub mod session;
pub mod web;

mod http;

pub use http::*;
