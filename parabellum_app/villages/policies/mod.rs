//! Village application policies.
//!
//! Policies live here when a rule combines village/domain state with
//! application-level context such as queued actions, read-model commitments, or
//! command workflow choices. Pure mechanics still belong in `parabellum_game`;
//! SQL and read-model loading stay in infrastructure.
//!
//! Do not add a policy that only delegates to a domain method. In that case,
//! call the domain model directly from the aggregate state or command handler.

pub mod army_dispatch;
pub mod expansion;
pub mod marketplace;
pub mod reinforcement_control;
