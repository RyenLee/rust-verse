
//! JSON 文件后端 —— 作为 redb 的轻量级替代方案。
//!
//! 所有数据存储在单个 JSON 文件中，启动时全量加载到内存，
//! 每次变更后立即写回磁盘。适合开发调试和小规模数据场景。
//!
//! 通过 `db-json` Cargo feature 启用。

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::domain::repository::{
    ConfigRepository, DataStore, EnvVarRepository, NotificationRepository, PluginRepository,
    RepositoryError, SettingsRepository,
};
use crate::infrastructure::config::EnvVarEntryConfig;

// ── 序列化结构 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredNotification {
    id: u64,
    json: String,
    read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonDbData {
    config: HashMap<String, String>,
    plugins: Vec<String>,
    env_vars: HashMap<String, HashMap<String, EnvVarEntryConfig>>,
    notifications: Vec<StoredNotification>,
    next_notif_id: u64,
    settings: Option<String>,
}

impl Default for JsonDbData {
    fn default() -> Self {
        Self {
            config: HashMap::new(),
            plugins: Vec::new(),
            env_vars: HashMap::new(),
            notifications: Vec::new(),
            next_notif_id: 1,
            settings: None,
        }
    }
}

// ── 内部状态 ─────────────────────────────────────────────────────────────

struct Inner {
    data: JsonDbData,
    path: PathBuf,
}

// ── JsonDataStore ────────────────────────────────────────────────────────

/// JSON 文件后端的数据仓库实现。
#[derive(Clone)]
pub struct JsonDataStore {
    inner: Arc<Mutex<Inner>>,
}

impl JsonDataStore {
    /// 打开或创建 JSON 数据文件。
    pub fn open(path: &Path) -> io::Result<Self> {
        let data = if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            JsonDbData::default()
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                data,
                path: path.to_path_buf(),
            })),
        })
    }

    /// 创建一个纯内存存储（不持久化到文件）。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                data: JsonDbData::default(),
                path: PathBuf::new(), // 空路径 = 不写文件
            })),
        }
    }

    /// 将当前数据写回文件。
    fn flush(inner: &Inner) -> Result<(), RepositoryError> {
        if inner.path.as_os_str().is_empty() {
            return Ok(()); // 内存模式，不写文件
        }
        let json = serde_json::to_string_pretty(&inner.data)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        if let Some(parent) = inner.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| RepositoryError::Database(e.to_string()))?;
        }
        fs::write(&inner.path, json)
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        Ok(())
    }
}

// ── ConfigRepository ─────────────────────────────────────────────────────

impl ConfigRepository for JsonDataStore {
    fn get_config(&self, key: &str) -> Option<String> {
        self.inner.lock().unwrap().data.config.get(key).cloned()
    }

    fn set_config(&self, key: &str, value: &str) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        inner.data.config.insert(key.to_string(), value.to_string());
        Self::flush(&inner)
    }

    fn get_config_batch(&self, keys: &[&str]) -> HashMap<String, String> {
        let inner = self.inner.lock().unwrap();
        keys.iter()
            .filter_map(|&k| {
                inner
                    .data
                    .config
                    .get(k)
                    .cloned()
                    .map(|v| (k.to_string(), v))
            })
            .collect()
    }
}

// ── EnvVarRepository ─────────────────────────────────────────────────────

impl EnvVarRepository for JsonDataStore {
    fn get_env_var_metas(&self) -> HashMap<String, HashMap<String, EnvVarEntryConfig>> {
        self.inner.lock().unwrap().data.env_vars.clone()
    }

    fn set_env_var_meta(
        &self,
        category: &str,
        name: &str,
        entry: &EnvVarEntryConfig,
    ) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .data
            .env_vars
            .entry(category.to_string())
            .or_default()
            .insert(name.to_string(), entry.clone());
        Self::flush(&inner)
    }

    fn delete_env_var_meta(&self, category: &str, name: &str) -> Result<bool, RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        let removed = inner
            .data
            .env_vars
            .get_mut(category)
            .map(|vars| vars.remove(name).is_some())
            .unwrap_or(false);
        Self::flush(&inner)?;
        Ok(removed)
    }
}

// ── PluginRepository ─────────────────────────────────────────────────────

impl PluginRepository for JsonDataStore {
    fn get_plugin_names(&self) -> Vec<String> {
        self.inner.lock().unwrap().data.plugins.clone()
    }

    fn set_plugin_names(&self, names: &[String]) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        inner.data.plugins = names.to_vec();
        Self::flush(&inner)
    }
}

// ── NotificationRepository ───────────────────────────────────────────────

impl NotificationRepository for JsonDataStore {
    fn notification_ensure_table(&self) -> Result<(), RepositoryError> {
        Ok(()) // JSON 无需建表
    }

    fn notification_insert(&self, json: &str) -> Result<u64, RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.data.next_notif_id;
        inner.data.next_notif_id += 1;
        inner.data.notifications.push(StoredNotification {
            id,
            json: json.to_string(),
            read: false,
        });
        Self::flush(&inner)?;
        Ok(id)
    }

    fn notification_list(&self) -> Result<Vec<(u64, String)>, RepositoryError> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .data
            .notifications
            .iter()
            .map(|n| (n.id, n.json.clone()))
            .collect())
    }

    fn notification_mark_read(&self, id: u64) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(n) = inner.data.notifications.iter_mut().find(|n| n.id == id) {
            n.read = true;
        }
        Self::flush(&inner)
    }

    fn notification_mark_unread(&self, id: u64) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(n) = inner.data.notifications.iter_mut().find(|n| n.id == id) {
            n.read = false;
        }
        Self::flush(&inner)
    }

    fn notification_delete(&self, id: u64) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        inner.data.notifications.retain(|n| n.id != id);
        Self::flush(&inner)
    }

    fn notification_delete_all(&self) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        inner.data.notifications.clear();
        Self::flush(&inner)
    }

    fn notification_unread_count(&self) -> Result<u64, RepositoryError> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .data
            .notifications
            .iter()
            .filter(|n| !n.read)
            .count() as u64)
    }
}

// ── SettingsRepository ───────────────────────────────────────────────────

impl SettingsRepository for JsonDataStore {
    fn get_settings(&self) -> Option<String> {
        self.inner.lock().unwrap().data.settings.clone()
    }

    fn set_settings(&self, json: &str) -> Result<(), RepositoryError> {
        let mut inner = self.inner.lock().unwrap();
        inner.data.settings = Some(json.to_string());
        Self::flush(&inner)
    }
}

// ── DataStore ────────────────────────────────────────────────────────────

impl DataStore for JsonDataStore {}