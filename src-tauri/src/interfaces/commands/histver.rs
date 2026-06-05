use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use tauri::State;

use crate::domain::constants::{channel, table_name};
use crate::domain::error::AppResult;
use crate::state::AppState;

pub use crate::domain::entity::HistRelease;
use crate::domain::entity::HistReleasePage;

const PAGE_SIZE: u64 = 50;
const TABLE_STABLE: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_STABLE);
const TABLE_BETA: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_BETA);
const TABLE_NIGHTLY: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_NIGHTLY);

/// Read all releases from db for the given channel filter, sorted by date desc.
/// Returns (total_count, releases_slice) where releases_slice is [offset..offset+limit].
fn read_releases_slice(
    db: &Database,
    channel_filter: Option<&str>,
    offset: u64,
    limit: u64,
) -> AppResult<HistReleasePage> {
    let channels: Vec<&str> = if let Some(ch) = channel_filter {
        vec![ch]
    } else {
        channel::ALL.to_vec()
    };
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
            releases.push(HistRelease {
                version: value.value().to_string(),
                date: key.value().to_string(),
                channel: ch.to_string(),
            })
        }
    }
    releases.sort_by(|a, b| b.date.cmp(&a.date));
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
        channel.as_deref(),
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
    let page = read_releases_slice(
        &state.db,
        channel.as_deref(),
        0,
        u64::MAX,
    )?;
    let lower = keyword.to_lowercase();
    let filtered: Vec<HistRelease> = page
        .items
        .into_iter()
        .filter(|r| r.version.to_lowercase().contains(&lower))
        .collect();
    let total = filtered.len() as u64;
    let off = offset.unwrap_or(0) as usize;
    let lim = limit.unwrap_or(PAGE_SIZE) as usize;
    let items: Vec<HistRelease> = filtered.into_iter().skip(off).take(lim).collect();
    let has_more = (off + lim) < total as usize;
    Ok(HistReleasePage {
        items,
        total,
        has_more,
    })
}

#[tauri::command]
pub fn count_hist_releases(state: State<'_, AppState>, channel: Option<String>) -> AppResult<u64> {
    let page = read_releases_slice(&state.db, channel.as_deref(), 0, u64::MAX)?;
    Ok(page.total)
}