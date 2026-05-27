//! Domain layer — pure business logic with zero framework dependencies.
//!
//! Contains entities, value objects, domain services, and repository traits.
//! No Tauri, redb, or platform-specific imports allowed.

pub mod base;
pub mod config_keys;
pub mod entity;
pub mod error;
pub mod mirror;
pub mod notification;
pub mod parsing;
pub mod repository;
pub mod settings;
