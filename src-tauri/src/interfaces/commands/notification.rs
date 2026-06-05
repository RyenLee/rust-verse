//! Notification Tauri commands — thin adapters that delegate to the
//! persistence layer.
//!
//! DDD interface layer: only parameter extraction and delegation.
//! No business logic lives here.

use crate::domain::notification::{NewNotification, Notification};
use crate::state::AppState;

/// Create a new notification.
#[tauri::command]
pub fn notify_create(
    state: tauri::State<'_, AppState>,
    new: NewNotification,
) -> Result<u64, String> {
    let json = serde_json::to_string(&new).map_err(|e| e.to_string())?;
    (&*state.store).notification_insert(&json).map_err(|e| e.to_string())
}

/// List notifications, newest first.
/// Supports optional `limit` / `offset` for pagination.
#[tauri::command]
pub fn notify_list(
    state: tauri::State<'_, AppState>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<Vec<Notification>, String> {
    let limit = limit.unwrap_or(u64::MAX) as usize;
    let offset = offset.unwrap_or(0) as usize;
    // P1: Use paginated query to avoid loading all notifications into memory
    let pairs = (&*state.store)
        .notification_list_paginated(limit, offset)
        .map_err(|e| e.to_string())?;
    let page: Vec<Notification> = pairs
        .into_iter()
        .map(|(id, json)| {
            let mut n: Notification = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            n.id = id;
            Ok(n)
        })
        .collect::<Result<_, String>>()?;
    Ok(page)
}

/// Get total count of notifications (O(1) counter read, no full table scan).
#[tauri::command]
pub fn notify_count(
    state: tauri::State<'_, AppState>,
) -> Result<u64, String> {
    (&*state.store)
        .notification_count()
        .map_err(|e| e.to_string())
}

/// Mark a notification as read.
#[tauri::command]
pub fn notify_mark_read(
    state: tauri::State<'_, AppState>,
    id: u64,
) -> Result<(), String> {
    (&*state.store).notification_mark_read(id).map_err(|e| e.to_string())
}

/// Mark a notification as unread.
#[tauri::command]
pub fn notify_mark_unread(
    state: tauri::State<'_, AppState>,
    id: u64,
) -> Result<(), String> {
    (&*state.store).notification_mark_unread(id).map_err(|e| e.to_string())
}

/// Delete a single notification.
#[tauri::command]
pub fn notify_delete(
    state: tauri::State<'_, AppState>,
    id: u64,
) -> Result<(), String> {
    (&*state.store).notification_delete(id).map_err(|e| e.to_string())
}

/// Delete all notifications.
#[tauri::command]
pub fn notify_delete_all(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    (&*state.store).notification_delete_all().map_err(|e| e.to_string())
}

/// Get the count of unread notifications.
#[tauri::command]
pub fn notify_unread_count(
    state: tauri::State<'_, AppState>,
) -> Result<u64, String> {
    (&*state.store).notification_unread_count().map_err(|e| e.to_string())
}

/// Delete all read notifications older than `before_ms` (Unix timestamp in milliseconds).
#[tauri::command]
pub fn notification_delete_read_before(
    state: tauri::State<'_, AppState>,
    before_ms: i64,
) -> Result<u64, String> {
    (&*state.store)
        .notification_delete_read_before(before_ms)
        .map_err(|e| e.to_string())
}