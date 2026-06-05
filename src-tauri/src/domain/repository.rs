//! 数据仓库 trait 定义 —— 领域层的数据访问契约。
//!
//! 所有持久化操作都通过这些 trait 进行，基础设施层提供具体实现（redb / SQLite / JSON 等）。
//! 依赖方向：domain 定义 trait，infrastructure 实现 trait。

use std::collections::HashMap;

use crate::domain::error::AppError;
use crate::infrastructure::config::EnvVarEntryConfig;

// ── RepositoryError ──────────────────────────────────────────────────────

/// 统一的数据仓库错误类型。
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum RepositoryError {
    #[error("database error: {0}")]
    Database(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<RepositoryError> for AppError {
    fn from(e: RepositoryError) -> Self {
        AppError::Config(e.to_string())
    }
}

// ── ConfigRepository ─────────────────────────────────────────────────────

/// 核心配置仓库 —— 管理简单 key-value 配置。
#[allow(dead_code)]
pub trait ConfigRepository: Send + Sync {
    /// 读取单个配置值。
    fn get_config(&self, key: &str) -> Option<String>;

    /// 设置单个配置值。
    fn set_config(&self, key: &str, value: &str) -> Result<(), RepositoryError>;

    /// 批量读取配置值（一次事务）。
    fn get_config_batch(&self, keys: &[&str]) -> HashMap<String, String>;

    /// 批量写入配置值（一次事务）。
    fn set_config_batch(&self, entries: &[(&str, &str)]) -> Result<(), RepositoryError>;
}

// ── EnvVarRepository ─────────────────────────────────────────────────────

/// 环境变量元数据仓库。
pub trait EnvVarRepository: Send + Sync {
    /// 获取所有环境变量元数据，按 category 分组。
    fn get_env_var_metas(&self) -> HashMap<String, HashMap<String, EnvVarEntryConfig>>;

    /// 设置单个环境变量元数据。
    fn set_env_var_meta(
        &self,
        category: &str,
        name: &str,
        entry: &EnvVarEntryConfig,
    ) -> Result<(), RepositoryError>;

    /// 删除单个环境变量元数据。
    fn delete_env_var_meta(&self, category: &str, name: &str) -> Result<bool, RepositoryError>;
}

// ── PluginRepository ─────────────────────────────────────────────────────

/// Cargo 插件仓库。
#[allow(dead_code)]
pub trait PluginRepository: Send + Sync {
    /// 获取官方推荐插件名称列表。
    fn get_plugin_names(&self) -> Vec<String>;

    /// 设置官方推荐插件名称列表。
    fn set_plugin_names(&self, names: &[String]) -> Result<(), RepositoryError>;
}

// ── NotificationRepository ───────────────────────────────────────────────

/// 通知仓库。
pub trait NotificationRepository: Send + Sync {
    /// 确保通知表存在。
    #[allow(dead_code)]
    fn notification_ensure_table(&self) -> Result<(), RepositoryError>;

    /// 插入通知（兼容旧接口，sound_enabled 和 default_priority 使用默认值）。
    fn notification_insert(&self, json: &str) -> Result<u64, RepositoryError>;

    /// 插入通知并同时设置 sound_enabled 和 default_priority 字段。
    ///
    /// 新代码应优先使用此方法，确保存储的通知与事件通知数据一致。
    fn notification_insert_with_settings(
        &self,
        json: &str,
        sound_enabled: bool,
        default_priority: &str,
    ) -> Result<u64, RepositoryError>;

    /// 列出所有通知（保留用于向后兼容，新代码请使用 notification_list_paginated）。
    #[allow(dead_code)]
    fn notification_list(&self) -> Result<Vec<(u64, String)>, RepositoryError>;

    /// 分页列出通知，按 ID 降序（最新在前）。
    fn notification_list_paginated(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<(u64, String)>, RepositoryError>;

    /// 获取通知总数（O(1) 计数器查询，不遍历全表）。
    fn notification_count(&self) -> Result<u64, RepositoryError>;

    /// 标记为已读。
    fn notification_mark_read(&self, id: u64) -> Result<(), RepositoryError>;

    /// 标记为未读。
    fn notification_mark_unread(&self, id: u64) -> Result<(), RepositoryError>;

    /// 删除通知。
    fn notification_delete(&self, id: u64) -> Result<(), RepositoryError>;

    /// 删除所有通知。
    fn notification_delete_all(&self) -> Result<(), RepositoryError>;

    /// 获取未读通知数量。
    fn notification_unread_count(&self) -> Result<u64, RepositoryError>;

    /// 删除所有已读且创建时间早于 `cutoff_ms`（Unix 毫秒时间戳）的通知。
    /// 返回已删除的数量。
    fn notification_delete_read_before(&self, cutoff_ms: i64) -> Result<u64, RepositoryError>;
}

// ── SettingsRepository ───────────────────────────────────────────────────

/// 用户设置仓库。
pub trait SettingsRepository: Send + Sync {
    /// 获取用户设置 JSON 字符串。
    fn get_settings(&self) -> Option<String>;

    /// 保存用户设置 JSON 字符串。
    fn set_settings(&self, json: &str) -> Result<(), RepositoryError>;
}

// ── DataStore ────────────────────────────────────────────────────────────

/// 聚合数据访问 trait —— 组合所有子仓库。
///
/// 应用层通过 `Arc<dyn DataStore>` 使用，无需关心具体后端。
pub trait DataStore:
    ConfigRepository
    + EnvVarRepository
    + PluginRepository
    + NotificationRepository
    + SettingsRepository
    + Send
    + Sync
{
}
