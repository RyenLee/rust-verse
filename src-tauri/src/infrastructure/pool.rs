//! Database connection pool abstraction with multi-datasource support.
//!
//! ## Design
//! - `DbConnection` trait: generic connection interface for any backend
//! - `DbPool` trait: pool that returns connections by name
//! - `MultiDbRegistry`: registry of named pools, supports runtime lookup
//! - `RedbPool`: redb-specific implementation — redb's `Database` is already
//!   designed for concurrent access via `Arc<Database>`, so the pool wraps
//!   it and returns cloned Arcs as "connections"

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use redb::Database;

// ---------------------------------------------------------------------------
// Connection trait
// ---------------------------------------------------------------------------

/// A generic database connection handle.
///
/// Each backend (redb, SQLite, etc.) implements this trait to provide
/// a unified interface for repository implementations.
#[allow(dead_code)]
pub trait DbConnection: Send + Sync {
    /// Returns the database name (e.g. "config", "cache").
    fn name(&self) -> &str;

    /// Returns the underlying redb Database handle.
    /// Returns `None` if this is not a redb connection.
    fn as_redb(&self) -> Option<&Database>;
}

// ---------------------------------------------------------------------------
// Redb-specific connection
// ---------------------------------------------------------------------------

/// Redb-backed connection — wraps an `Arc<Database>`.
#[allow(dead_code)]
pub struct RedbConnection {
    name: String,
    db: Arc<Database>,
}

#[allow(dead_code)]
impl RedbConnection {
    pub fn new(name: &str, db: Arc<Database>) -> Self {
        Self {
            name: name.to_string(),
            db,
        }
    }
}

impl DbConnection for RedbConnection {
    fn name(&self) -> &str {
        &self.name
    }

    fn as_redb(&self) -> Option<&Database> {
        Some(&self.db)
    }
}

// ---------------------------------------------------------------------------
// Pool trait
// ---------------------------------------------------------------------------

/// A connection pool that can hand out connections.
#[allow(dead_code)]
pub trait DbPool: Send + Sync {
    /// Get a connection from the pool.
    fn get_connection(&self) -> Option<Arc<dyn DbConnection>>;
}

// ---------------------------------------------------------------------------
// Redb pool implementation
// ---------------------------------------------------------------------------

/// Pool wrapping a single redb Database.
pub struct RedbPool {
    db: Arc<Database>,
}

#[allow(dead_code)]
impl RedbPool {
    pub fn new(_name: &str, db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> Arc<Database> {
        Arc::clone(&self.db)
    }
}

impl DbPool for RedbPool {
    fn get_connection(&self) -> Option<Arc<dyn DbConnection>> {
        Some(Arc::new(RedbConnection::new(
            "config",
            Arc::clone(&self.db),
        )))
    }
}

// ---------------------------------------------------------------------------
// Multi-datasource registry
// ---------------------------------------------------------------------------

/// Registry of named connection pools.
///
/// Supports multiple datasources (e.g. "config" → redb, "cache" → in-memory).
/// Repositories look up their pool by name.
#[allow(dead_code)]
pub struct MultiDbRegistry {
    pools: RwLock<HashMap<String, Arc<dyn DbPool>>>,
    /// Direct reference to the config RedbPool (avoids downcast).
    config_pool: RwLock<Option<Arc<RedbPool>>>,
}

#[allow(dead_code)]
impl MultiDbRegistry {
    pub fn new() -> Self {
        Self {
            pools: RwLock::new(HashMap::new()),
            config_pool: RwLock::new(None),
        }
    }

    /// Register a named pool.
    pub fn register(&self, name: &str, pool: Arc<dyn DbPool>) {
        let mut pools = self.pools.write().unwrap();
        pools.insert(name.to_string(), pool);
    }

    /// Register the config pool (redb-specific).
    pub fn register_config_pool(&self, pool: Arc<RedbPool>) {
        self.register("config", pool.clone());
        let mut config = self.config_pool.write().unwrap();
        *config = Some(pool);
    }

    /// Get a pool by name.
    #[allow(dead_code)]
    pub fn get_pool(&self, name: &str) -> Option<Arc<dyn DbPool>> {
        let pools = self.pools.read().unwrap();
        pools.get(name).cloned()
    }

    /// Get the config RedbPool directly.
    pub fn config_pool(&self) -> Option<Arc<RedbPool>> {
        self.config_pool.read().unwrap().clone()
    }

    /// Get a redb Database handle from the config pool.
    pub fn config_db(&self) -> Option<Arc<redb::Database>> {
        self.config_pool().map(|p| p.db())
    }
}

impl Default for MultiDbRegistry {
    fn default() -> Self {
        Self::new()
    }
}
