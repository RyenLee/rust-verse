use std::collections::BTreeMap;
use std::sync::Arc;

use redb::{Database, TableDefinition};

use crate::domain::constants::{channel, file_name, log_module, manifest_parse, table_name, url};
use crate::domain::error::{AppError, AppResult};
use crate::infrastructure::http_client;

const TABLE_STABLE: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_STABLE);
const TABLE_BETA: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_BETA);
const TABLE_NIGHTLY: TableDefinition<&str, &str> = TableDefinition::new(table_name::HISTVER_NIGHTLY);

fn is_stable_version(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

fn is_beta_version(s: &str) -> bool {
    if let Some((ver, beta_part)) = s.split_once("-beta.") {
        is_stable_version(ver) && beta_part.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

fn parse_manifests_body(body: &str) -> BTreeMap<&str, Vec<(String, String)>> {
    let mut grouped: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::from([
        (channel::STABLE, Vec::new()),
        (channel::BETA, Vec::new()),
        (channel::NIGHTLY, Vec::new()),
    ]);

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some(path_part) = line.strip_prefix(manifest_parse::DIST_PREFIX) else {
            continue;
        };

        let Some((date, rest)) = path_part.split_once('/') else {
            continue;
        };

        let Some(inner) = rest
            .strip_prefix(manifest_parse::CHANNEL_RUST_PREFIX)
            .and_then(|s| s.strip_suffix(manifest_parse::TOML_SUFFIX))
        else {
            continue;
        };

        if inner == channel::NIGHTLY {
            grouped
                .get_mut(channel::NIGHTLY)
                .expect("nightly key must exist in grouped map")
                .push((date.to_string(), inner.to_string()));
        } else if is_beta_version(inner) {
            grouped
                .get_mut(channel::BETA)
                .expect("beta key must exist in grouped map")
                .push((date.to_string(), inner.to_string()));
        } else if is_stable_version(inner) {
            grouped
                .get_mut(channel::STABLE)
                .expect("stable key must exist in grouped map")
                .push((date.to_string(), inner.to_string()));
        }
    }

    grouped
}

async fn fetch_manifests() -> AppResult<String> {
    let client = http_client::http_client();

    let resp = client
        .get(url::MANIFESTS)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Failed to download manifests.txt: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::Network(format!(
            "HTTP {} from {}",
            resp.status().as_u16(),
            url::MANIFESTS
        )));
    }

    resp.text()
        .await
        .map_err(|e| AppError::Network(format!("Failed to read manifests.txt body: {}", e)))
}

fn save_manifest_files(
    body: &str,
    grouped: &BTreeMap<&str, Vec<(String, String)>>,
) -> AppResult<usize> {
    let paths = crate::infrastructure::app_paths::app_paths();
    let temp_dir = paths.temp_dir();

    let raw_path = temp_dir.join(file_name::MANIFESTS_TXT);
    std::fs::write(&raw_path, body)
        .map_err(|e| AppError::Command(format!("Failed to save manifests.txt: {}", e)))?;

    let prefix = file_name::SCRUBBING_PREFIX;

    let mut total = 0usize;

    for (channel_key, entries) in grouped {
        let file_name_str = format!("{}-{}", prefix, channel_key);
        let file_path = temp_dir.join(&file_name_str);

        let content: Vec<String> = entries
            .iter()
            .map(|(date, version)| format!("{}{}{}", date, manifest_parse::DATE_VERSION_SEP, version))
            .collect();
        let file_content = content.join(manifest_parse::FILE_ENTRY_SEP);

        std::fs::write(&file_path, &file_content).map_err(|e| {
            AppError::Command(format!("Failed to write {}: {}", file_name_str, e))
        })?;

        total += entries.len();
    }

    Ok(total)
}

#[tauri::command]
pub async fn download_manifests() -> AppResult<String> {
    let paths = crate::infrastructure::app_paths::app_paths();
    let temp_dir = paths.temp_dir();

    let body = fetch_manifests().await?;
    let grouped = parse_manifests_body(&body);
    let total = save_manifest_files(&body, &grouped)?;

    let result = format!(
        "Parsed {} entries: stable={}, beta={}, nightly={} → saved to {:?}",
        total,
        grouped[channel::STABLE].len(),
        grouped[channel::BETA].len(),
        grouped[channel::NIGHTLY].len(),
        temp_dir
    );

    Ok(result)
}

pub async fn do_sync_all_from_manifests(db: &Database) -> AppResult<u64> {
    let body = fetch_manifests().await?;
    let grouped = parse_manifests_body(&body);

    let _ = save_manifest_files(&body, &grouped);

    let mut total = 0u64;

    for (channel_key, entries) in &grouped {
        if entries.is_empty() {
            continue;
        }

        let table_def = match *channel_key {
            channel::STABLE => TABLE_STABLE,
            channel::BETA => TABLE_BETA,
            channel::NIGHTLY => TABLE_NIGHTLY,
            _ => continue,
        };

        let write_tx = db
            .begin_write()
            .map_err(|e| AppError::Command(format!("Failed to begin write transaction: {}", e)))?;
        {
            let mut table = write_tx.open_table(table_def).map_err(|e| {
                AppError::Command(format!(
                    "Failed to open histver table for {}: {}",
                    channel_key, e
                ))
            })?;
            for (date, version) in entries {
                table.insert(date.as_str(), version.as_str()).map_err(|e| {
                    AppError::Command(format!("Failed to store release {}: {}", version, e))
                })?;
            }
        }
        write_tx.commit().map_err(|e| {
            AppError::Command(format!("Failed to commit {} release data: {}", channel_key, e))
        })?;

        total += entries.len() as u64;
    }

    Ok(total)
}

#[tauri::command]
pub async fn sync_from_manifests(
    state: tauri::State<'_, crate::state::AppState>,
) -> AppResult<u64> {
    do_sync_all_from_manifests(&state.db).await
}

pub fn startup_sync_manifests(db: Arc<Database>) {
    tauri::async_runtime::spawn(async move {
        match do_sync_all_from_manifests(&db).await {
            Ok(count) => eprintln!(
                "[{}] startup sync: {} releases from manifests.txt",
                log_module::MANIFEST,
                count
            ),
            Err(e) => eprintln!("[{}] startup sync failed: {}", log_module::MANIFEST, e),
        }
    });
}
