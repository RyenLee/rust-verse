use crate::domain::config_keys::keys;
use crate::domain::entity::RustupMirrorSource;
use crate::domain::rustup_mirror;
use crate::state::AppState;

fn load_builtin_sources(state: &AppState) -> Vec<RustupMirrorSource> {
    let json = (&*state.store).get_config(keys::RUSTUP_MIRROR_BUILTIN);
    let sources: Vec<RustupMirrorSource> = match json {
        Some(s) => serde_json::from_str(&s).unwrap_or_default(),
        None => Vec::new(),
    };
    if !sources.is_empty() && sources.iter().any(|s| s.id.is_empty()) {
        let regenerated = rustup_mirror::get_builtin_sources();
        let _ = save_builtin_sources(state, &regenerated);
        return regenerated;
    }
    sources
}

fn load_custom_sources(state: &AppState) -> Vec<RustupMirrorSource> {
    let json = (&*state.store).get_config(keys::RUSTUP_MIRROR_CUSTOM);
    match json {
        Some(s) => serde_json::from_str(&s).unwrap_or_default(),
        None => Vec::new(),
    }
}

fn save_builtin_sources(state: &AppState, sources: &[RustupMirrorSource]) -> Result<(), String> {
    let json = serde_json::to_string(sources)
        .map_err(|e| format!("Failed to serialize builtin sources: {e}"))?;
    (&*state.store)
        .set_config(keys::RUSTUP_MIRROR_BUILTIN, &json)
        .map_err(|e| format!("Failed to save builtin sources: {e}"))
}

fn save_custom_sources(state: &AppState, sources: &[RustupMirrorSource]) -> Result<(), String> {
    let json = serde_json::to_string(sources)
        .map_err(|e| format!("Failed to serialize custom sources: {e}"))?;
    (&*state.store)
        .set_config(keys::RUSTUP_MIRROR_CUSTOM, &json)
        .map_err(|e| format!("Failed to save custom sources: {e}"))
}

pub fn init_rustup_mirror_sources(state: &AppState) {
    if (&*state.store)
        .get_config(keys::RUSTUP_MIRROR_BUILTIN)
        .is_some()
    {
        return;
    }
    let sources = rustup_mirror::get_builtin_sources();
    if let Ok(json) = serde_json::to_string(&sources) {
        let _ = (&*state.store).set_config(keys::RUSTUP_MIRROR_BUILTIN, &json);
    }
}

#[tauri::command]
pub fn list_rustup_mirror_sources(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RustupMirrorSource>, String> {
    let builtin = load_builtin_sources(&state);
    let mut custom = load_custom_sources(&state);
    let mut all = builtin;
    all.append(&mut custom);
    Ok(all)
}

#[tauri::command]
pub fn add_rustup_mirror_source(
    state: tauri::State<'_, AppState>,
    name: String,
    dist_server: String,
    update_root: String,
) -> Result<RustupMirrorSource, String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    rustup_mirror::validate_url(&dist_server)?;
    rustup_mirror::validate_url(&update_root)?;

    let mut custom = load_custom_sources(&state);
    let id = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let source = RustupMirrorSource {
        id: id.clone(),
        name: name.trim().to_string(),
        dist_server: dist_server.trim().to_string(),
        update_root: update_root.trim().to_string(),
        is_builtin: false,
    };
    custom.push(source.clone());
    save_custom_sources(&state, &custom)?;
    Ok(source)
}

#[tauri::command]
pub fn update_rustup_mirror_source(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
    dist_server: String,
    update_root: String,
) -> Result<RustupMirrorSource, String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    rustup_mirror::validate_url(&dist_server)?;
    rustup_mirror::validate_url(&update_root)?;

    let mut custom = load_custom_sources(&state);
    if let Some(pos) = custom.iter().position(|s| s.id == id) {
        let updated = RustupMirrorSource {
            id: id.clone(),
            name: name.trim().to_string(),
            dist_server: dist_server.trim().to_string(),
            update_root: update_root.trim().to_string(),
            is_builtin: false,
        };
        custom[pos] = updated.clone();
        save_custom_sources(&state, &custom)?;
        return Ok(updated);
    }

    let mut builtin = load_builtin_sources(&state);
    let pos = builtin
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| format!("Source with id '{id}' not found"))?;

    let updated = RustupMirrorSource {
        id,
        name: name.trim().to_string(),
        dist_server: dist_server.trim().to_string(),
        update_root: update_root.trim().to_string(),
        is_builtin: false,
    };
    builtin.remove(pos);
    save_builtin_sources(&state, &builtin)?;

    custom.push(updated.clone());
    save_custom_sources(&state, &custom)?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_rustup_mirror_source(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut custom = load_custom_sources(&state);
    if let Some(pos) = custom.iter().position(|s| s.id == id) {
        custom.remove(pos);
        return save_custom_sources(&state, &custom);
    }

    let mut builtin = load_builtin_sources(&state);
    let pos = builtin
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| format!("Source with id '{id}' not found"))?;
    builtin.remove(pos);
    save_builtin_sources(&state, &builtin)
}
