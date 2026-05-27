//! Interfaces layer — Tauri command adapters.
//!
//! Each command function is a thin adapter that:
//! 1. Deserializes input parameters
//! 2. Delegates to application/ or domain/ services
//! 3. Handles error conditions
//! 4. Returns formatted results
//!
//! No business logic lives here.

pub mod commands;