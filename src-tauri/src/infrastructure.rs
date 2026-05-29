//! Infrastructure layer — concrete implementations of I/O and external dependencies.
//!
//! Implements domain repository traits with real infrastructure:
//! redb database, system environment, CLI execution, logging, proxy, installer.

pub mod app_paths;
pub mod config;
pub mod config_cache;
pub mod db;
pub mod exec;
pub mod http_client;
pub mod installer;
#[cfg(feature = "db-json")]
pub mod json_store;
pub mod logger;
pub mod notifier;
pub mod pool;
pub mod proxy;
pub mod system;
