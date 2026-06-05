use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use tauri::State;

use crate::domain::constants::{channel, table_name};
use crate::domain::error::AppResult;
use crate::state::AppState;

pub use crate::domain::entity::HistRelease;
use crate::domain::entity::HistReleasePage;

const PAGE_SIZE: u64 = 50;
const TABLE_STABLE: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_STABLE);
const TABLE_BETA: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_BETA);
const TABLE_NIGHTLY: TableDefinition<&str, &str> =
    TableDefinition::new(table_name::HISTVER_NIGHTLY);

/// Read releases from db for the given channel filter, sorted by date desc.
/// If `keyword` is provided, only releases whose version contains the keyword are collected.
/// Returns (total_count, releases_slice) where releases_slice is [offset..offset+limit].
///
/// P1: Uses QueryCache to avoid re-reading the database on every page turn.
/// The full sorted list is cached per (channel, keyword) combination.
fn read_releases_slice(
    db: &Database,
    cache: &crate::infrastructure::query_cache::QueryCache,
    channel_filter: Option<&str>,
    keyword: Option<&str>,
    offset: u64,
    limit: u64,
) -> AppResult<HistReleasePage> {
    // Build cache key
    let cache_key = format!(
        "hist_releases:{}:{}",
        channel_filter.unwrap_or("all"),
        keyword.unwrap_or("")
    );

    // Try cache first
    let releases: Vec<HistRelease> = if let Some(cached_json) = cache.get(&cache_key) {
        serde_json::from_str(&cached_json).unwrap_or_else(|_| Vec::new())
    } else {
        let channels: Vec<&str> = if let Some(ch) = channel_filter {
            vec![ch]
        } else {
            channel::ALL.to_vec()
        };
        let lower = keyword.map(|k| k.to_lowercase());
        let read_tx = db.begin_read().map_err(|e| {
            crate::domain::error::AppError::Command(format!("Failed to begin read transaction: {}", e))
        })?;
        let mut releases = Vec::new();
        for ch in channels {
            let table_def = match ch {
                channel::STABLE => TABLE_STABLE,
                channel::BETA => TABLE_BETA,
                channel::NIGHTLY => TABLE_NIGHTLY,
                _ => continue,
            };
            let table = match read_tx.open_table(table_def) {
                Ok(t) => t,
                Err(e) => {
                    if e.to_string().contains("does not exist") {
                        continue;
                    }
                    return Err(crate::domain::error::AppError::Command(format!(
                        "Failed to open histver table for {}: {}",
                        ch, e
                    )));
                }
            };
            for result in table.iter().map_err(|e| {
                crate::domain::error::AppError::Command(format!("Failed to iterate releases: {}", e))
            })? {
                let (key, value) = result.map_err(|e| {
                    crate::domain::error::AppError::Command(format!(
                        "Failed to read release entry: {}",
                        e
                    ))
                })?;
                let version = value.value().to_string();
                if let Some(ref kw) = lower {
                    if !version.to_lowercase().contains(kw) {
                        continue;
                    }
                }
                releases.push(HistRelease {
                    version,
                    date: key.value().to_string(),
                    channel: ch.to_string(),
                })
            }
        }
        releases.sort_by(|a, b| b.date.cmp(&a.date));
        // Cache the full sorted list
        if let Ok(json) = serde_json::to_string(&releases) {
            cache.set(cache_key, json);
        }
        releases
    };

    let total = releases.len() as u64;
    let items = releases
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect::<Vec<_>>();
    let has_more = offset + limit < total;
    Ok(HistReleasePage {
        items,
        total,
        has_more,
    })
}

#[tauri::command]
pub fn list_hist_releases(
    state: State<'_, AppState>,
    channel: Option<String>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> AppResult<HistReleasePage> {
    read_releases_slice(
        &state.db,
        &state.query_cache,
        channel.as_deref(),
        None,
        offset.unwrap_or(0),
        limit.unwrap_or(PAGE_SIZE),
    )
}

#[tauri::command]
pub fn search_hist_releases(
    state: State<'_, AppState>,
    keyword: String,
    channel: Option<String>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> AppResult<HistReleasePage> {
    read_releases_slice(
        &state.db,
        &state.query_cache,
        channel.as_deref(),
        Some(&keyword),
        offset.unwrap_or(0),
        limit.unwrap_or(PAGE_SIZE),
    )
}

#[tauri::command]
pub fn count_hist_releases(state: State<'_, AppState>, channel: Option<String>) -> AppResult<u64> {
    let channels: Vec<&str> = if let Some(ch) = channel.as_deref() {
        vec![ch]
    } else {
        channel::ALL.to_vec()
    };
    let read_tx = db_begin_read(&state.db)?;
    let mut count = 0u64;
    for ch in channels {
        let table_def = match ch {
            channel::STABLE => TABLE_STABLE,
            channel::BETA => TABLE_BETA,
            channel::NIGHTLY => TABLE_NIGHTLY,
            _ => continue,
        };
        let table = match read_tx.open_table(table_def) {
            Ok(t) => t,
            Err(e) => {
                if e.to_string().contains("does not exist") {
                    continue;
                }
                return Err(crate::domain::error::AppError::Command(format!(
                    "Failed to open histver table for {}: {}",
                    ch, e
                )));
            }
        };
        count += table.len().map_err(|e| {
            crate::domain::error::AppError::Command(format!("Failed to count releases: {}", e))
        })?;
    }
    Ok(count)
}

fn db_begin_read(db: &Database) -> AppResult<redb::ReadTransaction> {
    db.begin_read().map_err(|e| {
        crate::domain::error::AppError::Command(format!("Failed to begin read transaction: {}", e))
    })
}
