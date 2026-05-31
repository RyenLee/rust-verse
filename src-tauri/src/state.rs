use std::sync::{Arc, Mutex};

use redb::Database;
use tokio::sync::Notify;

use crate::domain::constants::locale;
use crate::domain::repository::DataStore;
use crate::infrastructure::config_cache::AppConfigCache;
use crate::infrastructure::db::RedbDataStore;
use crate::infrastructure::query_cache::QueryCache;

/// Tracks the state of long-running async tasks (e.g. toolchain install/update).
pub struct TaskState {
    pub running: Mutex<bool>,
    pub cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    pub cancel_notify: Arc<Notify>,
}

impl TaskState {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(false),
            cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cancel_notify: Arc::new(Notify::new()),
        }
    }
}

/// Global application state shared across all Tauri commands.
///
/// Initialized during app startup and registered via `.manage()`.
pub struct AppState {
    pub db: Arc<Database>,
    pub rustup_path: Mutex<Option<std::path::PathBuf>>,
    pub cargo_path: Mutex<Option<std::path::PathBuf>>,
    pub store: Arc<dyn DataStore>,
    pub config_cache: AppConfigCache,
    pub locale: Mutex<String>,
    pub task_state: TaskState,
    pub query_cache: Arc<QueryCache>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let db_arc = Arc::new(db);
        let store = Arc::new(RedbDataStore::new(db_arc.clone()));
        Self {
            db: db_arc,
            rustup_path: Mutex::new(None),
            cargo_path: Mutex::new(None),
            store,
            config_cache: AppConfigCache::new(),
            locale: Mutex::new(locale::LC_C.to_string()),
            task_state: TaskState::new(),
            query_cache: Arc::new(QueryCache::new(60)),
        }
    }
}