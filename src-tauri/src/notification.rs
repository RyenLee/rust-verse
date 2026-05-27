//! Notification persistence layer: redb-backed CRUD operations.
//!
//! **DDD refactored**: domain types (`Category`, `Priority`, `Notification`,
//! `NewNotification`, `NotificationsConfig`) now live in
//! `domain::notification`.  This module retains only the redb persistence
//! implementation and re-exports the domain types for backward compatibility.

// Re-export domain types so existing callers (`use crate::notification::*`) keep working.
pub use crate::domain::notification::NewNotification;

use crate::domain::base::time::chrono_now_ms;
use crate::domain::notification::Category as DomainCategory;
use crate::domain::notification::NewNotification as DomainNewNotification;
use crate::domain::notification::Notification as DomainNotification;
use crate::domain::notification::Priority as DomainPriority;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

const NOTIFICATIONS_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("notifications");

// ── Table lifecycle ──

/// Create the notifications table if it does not already exist.
/// Call once at application startup. Idempotent — safe to call repeatedly.
pub fn ensure_table(db: &Database) -> Result<(), String> {
    let write_tx = db
        .begin_write()
        .map_err(|e| format!("DB write error: {e}"))?;
    {
        write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
    }
    write_tx
        .commit()
        .map_err(|e| format!("DB commit error: {e}"))?;
    Ok(())
}

/// Check whether opening the notifications table for reading would succeed.
fn table_exists(read_tx: &redb::ReadTransaction) -> Result<bool, String> {
    match read_tx.open_table(NOTIFICATIONS_TABLE) {
        Ok(_) => Ok(true),
        Err(redb::TableError::TableDoesNotExist(_)) => Ok(false),
        Err(e) => Err(format!("DB open table error: {e}")),
    }
}

/// Insert a new notification. Returns its assigned ID.
pub fn insert_notification(db: &Database, new: &DomainNewNotification) -> Result<u64, String> {
    let write_tx = db
        .begin_write()
        .map_err(|e| format!("DB write error: {e}"))?;
    let next_id = {
        let table = write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
        table
            .last()
            .map_err(|e| format!("DB error: {e}"))?
            .map(|(k, _)| k.value() + 1)
            .unwrap_or(1)
    };
    let notif = DomainNotification {
        id: next_id,
        category: new.category,
        priority: new.priority,
        title: new.title.clone(),
        body: new.body.clone(),
        notif_key: new.notif_key.clone(),
        params_json: new.params_json.clone(),
        action_route: new.action_route.clone(),
        is_read: false,
        created_at: chrono_now_ms(),
    };
    let json = serde_json::to_vec(&notif).map_err(|e| format!("Serialise error: {e}"))?;
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
        table
            .insert(next_id, json.as_slice())
            .map_err(|e| format!("DB insert error: {e}"))?;
    }
    write_tx
        .commit()
        .map_err(|e| format!("DB commit error: {e}"))?;

    crate::infrastructure::logger::logger().info(
        "notification",
        &format!(
            "created id={} category={:?} priority={:?} title=\"{}\"",
            next_id, new.category, new.priority, new.title
        ),
    );

    Ok(next_id)
}

/// Retrieve all notifications, newest first.
pub fn list_notifications(db: &Database) -> Result<Vec<DomainNotification>, String> {
    let read_tx = db.begin_read().map_err(|e| format!("DB read error: {e}"))?;
    if !table_exists(&read_tx)? {
        return Ok(Vec::new());
    }
    let table = read_tx
        .open_table(NOTIFICATIONS_TABLE)
        .map_err(|e| format!("DB open table error: {e}"))?;
    let mut items: Vec<DomainNotification> = Vec::new();
    for entry in table.iter().map_err(|e| format!("DB iter error: {e}"))? {
        let (_, guard) = entry.map_err(|e| format!("DB entry error: {e}"))?;
        let notif: DomainNotification =
            serde_json::from_slice(guard.value()).unwrap_or_else(|_| DomainNotification {
                id: 0,
                category: DomainCategory::Operation,
                priority: DomainPriority::Low,
                title: "Corrupted notification".into(),
                body: String::new(),
                notif_key: None,
                params_json: None,
                action_route: None,
                is_read: true,
                created_at: 0,
            });
        items.push(notif);
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

/// Mark a notification as read.
pub fn mark_read(db: &Database, id: u64) -> Result<(), String> {
    update_notification_field(db, id, |n| n.is_read = true)
}

/// Mark a notification as unread.
pub fn mark_unread(db: &Database, id: u64) -> Result<(), String> {
    update_notification_field(db, id, |n| n.is_read = false)
}

/// Delete a single notification.
pub fn delete_notification(db: &Database, id: u64) -> Result<(), String> {
    let write_tx = db
        .begin_write()
        .map_err(|e| format!("DB write error: {e}"))?;
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
        table
            .remove(id)
            .map_err(|e| format!("DB remove error: {e}"))?;
    }
    write_tx
        .commit()
        .map_err(|e| format!("DB commit error: {e}"))?;
    Ok(())
}

/// Delete all notifications.
pub fn delete_all_notifications(db: &Database) -> Result<(), String> {
    let write_tx = db
        .begin_write()
        .map_err(|e| format!("DB write error: {e}"))?;
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
        let keys: Vec<u64> = table
            .iter()
            .map_err(|e| format!("DB iter error: {e}"))?
            .filter_map(|entry| entry.ok().map(|(k, _)| k.value()))
            .collect();
        for key in keys {
            table.remove(key).ok();
        }
    }
    write_tx
        .commit()
        .map_err(|e| format!("DB commit error: {e}"))?;
    Ok(())
}

/// Get the count of unread notifications.
pub fn unread_count(db: &Database) -> Result<u64, String> {
    let read_tx = db.begin_read().map_err(|e| format!("DB read error: {e}"))?;
    if !table_exists(&read_tx)? {
        return Ok(0);
    }
    let table = read_tx
        .open_table(NOTIFICATIONS_TABLE)
        .map_err(|e| format!("DB open table error: {e}"))?;
    let mut count: u64 = 0;
    for entry in table.iter().map_err(|e| format!("DB iter error: {e}"))? {
        let (_, guard) = entry.map_err(|e| format!("DB entry error: {e}"))?;
        if let Ok(notif) = serde_json::from_slice::<DomainNotification>(guard.value()) {
            if !notif.is_read {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Delete all **read** notifications whose `created_at` is older than
/// `cutoff_ms` (Unix timestamp in milliseconds).
///
/// Returns the number of deleted notifications.
pub fn delete_read_before(db: &Database, cutoff_ms: i64) -> Result<u64, String> {
    let write_tx = db
        .begin_write()
        .map_err(|e| format!("DB write error: {e}"))?;
    let deleted = {
        let mut table = write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
        let keys_to_delete: Vec<u64> = table
            .iter()
            .map_err(|e| format!("DB iter error: {e}"))?
            .filter_map(|entry| {
                let (k, v) = entry.ok()?;
                let notif: DomainNotification = serde_json::from_slice(v.value()).ok()?;
                if notif.is_read && notif.created_at < cutoff_ms {
                    Some(k.value())
                } else {
                    None
                }
            })
            .collect();
        let count = keys_to_delete.len() as u64;
        for key in keys_to_delete {
            table.remove(key).ok();
        }
        count
    };
    write_tx
        .commit()
        .map_err(|e| format!("DB commit error: {e}"))?;

    if deleted > 0 {
        crate::infrastructure::logger::logger().info(
            "notification",
            &format!("auto-cleanup: deleted {deleted} read notifications older than {cutoff_ms}"),
        );
    }

    Ok(deleted)
}

fn update_notification_field(
    db: &Database,
    id: u64,
    updater: impl FnOnce(&mut DomainNotification),
) -> Result<(), String> {
    let write_tx = db
        .begin_write()
        .map_err(|e| format!("DB write error: {e}"))?;
    // ── Scope the read so the table handle is dropped before we reopen for write ──
    // redb disallows opening the same table twice in a single transaction.
    let json = {
        let table = write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
        let raw = table
            .get(id)
            .map_err(|e| format!("DB get error: {e}"))?
            .ok_or_else(|| format!("Notification {id} not found"))?;
        let mut notif: DomainNotification =
            serde_json::from_slice(raw.value()).map_err(|e| format!("Deserialise error: {e}"))?;
        updater(&mut notif);
        serde_json::to_vec(&notif).map_err(|e| format!("Serialise error: {e}"))?
    }; // table handle dropped here — safe to reopen now
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS_TABLE)
            .map_err(|e| format!("DB open table error: {e}"))?;
        table
            .insert(id, json.as_slice())
            .map_err(|e| format!("DB insert error: {e}"))?;
    }
    write_tx
        .commit()
        .map_err(|e| format!("DB commit error: {e}"))?;
    Ok(())
}
