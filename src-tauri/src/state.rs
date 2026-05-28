use std::sync::{Arc, Mutex};

use redb::Database;

use crate::domain::repository::DataStore;
use crate::infrastructure::db::RedbDataStore;
use crate::infrastructure::pool::{MultiDbRegistry, RedbPool};

/// Tracks the state of long-running async tasks (e.g. toolchain install/update).
pub struct TaskState {
    /// Whether a long-running task is currently executing.
    pub running: Mutex<bool>,
    /// Shared cancellation flag — set by frontend to request cancellation.
    pub cancel_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl TaskState {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(false),
            cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

/// Global application state shared across all Tauri commands.
///
/// Initialized during app startup and registered via `.manage()`.
/// Uses `Mutex` for interior mutability since `tauri::State` provides shared references.
pub struct AppState {
    /// Path to the `rustup` binary, detected at startup.
    pub rustup_path: Mutex<Option<std::path::PathBuf>>,
    /// Path to the `cargo` binary, detected at startup.
    pub cargo_path: Mutex<Option<std::path::PathBuf>>,
    /// Multi-datasource connection pool registry.
    ///
    /// Supports named datasources (e.g. "config" → redb, "cache" → in-memory).
    /// Repositories look up their pool by name via `db_registry.config_db()`.
    #[allow(dead_code)]
    pub db_registry: Arc<MultiDbRegistry>,
    /// Legacy redb Database handle (kept for migration compatibility).
    ///
    /// `Database` is thread-safe and supports concurrent reads.
    /// No `Mutex` needed for the handle itself.
    #[allow(dead_code)]
    pub db: Arc<Database>,
    /// Aggregated data store implementing `dyn DataStore`.
    ///
    /// Used by the DDD architecture for settings and notification persistence.
    pub store: Arc<dyn DataStore>,
    /// Locale currently in use (e.g. "zh-CN", "en-US").
    pub locale: Mutex<String>,
    /// State tracker for long-running async tasks.
    pub task_state: TaskState,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let db_arc = Arc::new(db);

        // Create connection pool registry
        let db_registry = Arc::new(MultiDbRegistry::new());
        db_registry.register_config_pool(Arc::new(RedbPool::new("config", Arc::clone(&db_arc))));

        let store = Arc::new(RedbDataStore::from_registry(Arc::clone(&db_registry)));
        Self {
            rustup_path: Mutex::new(None),
            cargo_path: Mutex::new(None),
            db_registry,
            db: db_arc,
            store,
            locale: Mutex::new("C".to_string()),
            task_state: TaskState::new(),
        }
    }
}
