use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use tauri::State;

use crate::domain::constants::{channel, table_name};
use crate::domain::error::AppResult;
use crate::state::AppState;

pub use crate::domain::entity::HistRelease;

const TABLE_STABLE: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_STABLE);
const TABLE_BETA: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_BETA);
const TABLE_NIGHTLY: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_NIGHTLY);

fn read_releases_from_db(db: &Database, channel_filter: Option<&str>) -> AppResult<Vec<HistRelease>> {
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
    Ok(releases)
}

#[tauri::command]
pub fn list_hist_releases(
    state: State<'_, AppState>,
    channel: Option<String>,
) -> AppResult<Vec<HistRelease>> {
    read_releases_from_db(&state.db, channel.as_deref())
}

#[tauri::command]
pub fn search_hist_releases(
    state: State<'_, AppState>,
    keyword: String,
    channel: Option<String>,
) -> AppResult<Vec<HistRelease>> {
    let releases = read_releases_from_db(&state.db, channel.as_deref())?;
    let lower = keyword.to_lowercase();
    Ok(releases
        .into_iter()
        .filter(|r| r.version.to_lowercase().contains(&lower))
        .collect())
}

#[tauri::command]
pub fn count_hist_releases(state: State<'_, AppState>, channel: Option<String>) -> AppResult<u64> {
    let releases = read_releases_from_db(&state.db, channel.as_deref())?;
    Ok(releases.len() as u64)
}