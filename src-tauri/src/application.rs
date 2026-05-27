//! Application layer — use case orchestration.
//!
//! Coordinates domain services and infrastructure to fulfill business use cases.
//! Depends on domain/ and infrastructure/, invoked by interfaces/commands/.

pub mod env_check;
pub mod env_var;
pub mod locale;
pub mod persist;
pub mod rustup;
