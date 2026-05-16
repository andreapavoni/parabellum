//! Application ports (hexagonal boundaries).
//!
//! `parabellum_app` exposes behavior through these traits and stays
//! infrastructure-agnostic. `parabellum_infra` provides concrete adapters.

pub mod identity;
pub mod map;
pub mod queries;
pub mod scheduler;
pub mod villages;
