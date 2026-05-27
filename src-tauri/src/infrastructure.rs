//! Infrastructure layer — concrete implementations of I/O and external dependencies.
//!
//! Implements domain repository traits with real infrastructure:
//! redb database, system environment, CLI execution, logging, proxy, installer.

pub mod config;
pub mod db;
pub mod exec;
pub mod installer;
#[cfg(feature = "db-json")]
pub mod json_store;
pub mod logger;
pub mod notifier;
pub mod proxy;
pub mod system;
