mod application;
mod domain;
mod infrastructure;
mod interfaces;
mod state;

use crate::domain::constants::{
    app as app_const, event_name, file_name, log_module, tray, tray_menu,
};
use crate::infrastructure::app_paths;
use crate::infrastructure::db::{ensure_version_in_db, migrate_from_toml, open_or_create};
use crate::infrastructure::logger;
use interfaces::commands::component::{add_component, list_components, remove_component};
use interfaces::commands::env_check::{check_env, get_versions};
use interfaces::commands::env_var::{
    delete_env_var_meta, get_env_var, list_env_vars, remove_env_var, set_env_var,
    update_env_var_meta,
};
use interfaces::commands::histver::{
    count_hist_releases, list_hist_releases, search_hist_releases,
};
use interfaces::commands::locale::{
    LocaleScanState, get_locale, list_available_locales, set_locale,
};
use interfaces::commands::manifest::{
    download_manifests, startup_sync_manifests, sync_from_manifests,
};
use interfaces::commands::mirror::{
    check_crm_installed, crm_best, crm_current, crm_default, crm_list, crm_test, crm_use,
    crm_version, install_crm,
};
use interfaces::commands::notification::{
    notification_delete_read_before, notify_count, notify_create, notify_delete, notify_delete_all,
    notify_list, notify_mark_read, notify_mark_unread, notify_unread_count,
};
use interfaces::commands::override_cmd::{
    get_override, list_overrides, remove_override, set_override,
};
use interfaces::commands::persist::{
    is_env_var_persisted, list_persisted_env_vars, persist_env_var, remove_persisted_env_var,
};
use interfaces::commands::plugin::{
    install_plugin, list_cargo_plugins, search_plugins, uninstall_plugin,
};
use interfaces::commands::rustup_mirror::{
    add_rustup_mirror_source, delete_rustup_mirror_source, init_rustup_mirror_sources,
    list_rustup_mirror_sources, update_rustup_mirror_source,
};
use interfaces::commands::settings::{get_config, get_settings, save_settings};
use interfaces::commands::system::{
    cancel_background_task, frontend_log, get_log_dir, install_rustup, invalidate_config_cache,
    is_background_task_running, refresh_process_path, reinit_terminal, restart_application,
    uninstall_rustup,
};
use interfaces::commands::target::{add_target, list_targets, remove_target};
use interfaces::commands::toolchain::{
    install_toolchain, list_toolchains, set_default_toolchain, uninstall_toolchain,
};
use interfaces::commands::update::{check_update, diag_network, update_all, update_rustup};
use state::AppState;
use tauri::{
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

macro_rules! dual_log {
    ($log:expr, $level:expr, $module:expr, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        let msg = format!($fmt $(, $arg)*);
        $log.info($module, &msg);
    }};
    ($log:expr, $level:expr, $module:expr, $fmt:expr) => {{
        $log.info($module, $fmt);
    }};
}

fn migrate_db_to_data_dir() {
    let paths = app_paths::app_paths();
    let exe_dir = paths.exe_dir();
    let db_file = format!("{}.{}", paths.db_name(), paths.db_type());

    let old_path = exe_dir.join(&db_file);
    if !old_path.exists() {
        return;
    }

    let data_dir = paths.data_dir();
    let new_path = data_dir.join(&db_file);
    if new_path.exists() {
        return;
    }

    std::fs::create_dir_all(data_dir).ok();
    match std::fs::rename(&old_path, &new_path) {
        Ok(()) => logger::logger().info(
            log_module::STARTUP,
            &format!("Migrated database: {:?} -> {:?}", old_path, new_path),
        ),
        Err(e) => logger::logger().warn(
            log_module::STARTUP,
            &format!("Warning: failed to move database to data/ dir: {e}; will use old location"),
        ),
    }
}

fn try_migrate_from_toml(db: &redb::Database) {
    let exe_dir = app_paths::app_paths().exe_dir().clone();

    let toml_path = exe_dir.join(file_name::CONFIG_TOML);
    if !toml_path.exists() {
        return;
    }

    match migrate_from_toml(db, &toml_path) {
        Ok(true) => {
            let migrated = exe_dir.join(file_name::CONFIG_TOML_MIGRATED);
            let _ = std::fs::remove_file(&migrated);
            let _ = std::fs::rename(&toml_path, &migrated);
            logger::logger().info(
                log_module::STARTUP,
                &format!(
                    "Migrated config.toml -> {}.{}",
                    app_paths::app_paths().db_name(),
                    app_paths::app_paths().db_type()
                ),
            );
        }
        Ok(false) => logger::logger().info(
            log_module::STARTUP,
            "config.toml exists but matches defaults, skipping migration",
        ),
        Err(e) => logger::logger().warn(
            log_module::STARTUP,
            &format!("Warning: config.toml migration failed: {e}"),
        ),
    }
}

fn run_notification_cleanup(
    store: &dyn crate::domain::repository::DataStore,
    app: &tauri::AppHandle,
) {
    let minutes = match store.get_settings() {
        Some(json) => serde_json::from_str::<crate::domain::settings::UserSettings>(&json)
            .map(|s| s.notifications.auto_cleanup_minutes)
            .unwrap_or(0),
        None => 0,
    };

    if minutes == 0 {
        return;
    }

    let cutoff_ms = crate::domain::base::time::chrono_now_ms() - (minutes as i64) * 60 * 1000;

    match store.notification_delete_read_before(cutoff_ms) {
        Ok(deleted) if deleted > 0 => {
            logger::logger().info(
                log_module::CLEANUP,
                &format!(
                    "Auto-deleted {deleted} expired read notifications (threshold: {minutes} min)"
                ),
            );
            let _ = app.emit(event_name::NOTIFICATION_CLEANUP, deleted);
        }
        Ok(_) => {}
        Err(e) => {
            logger::logger().error(
                log_module::CLEANUP,
                &format!("Failed to auto-delete expired notifications: {e}"),
            );
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = app_paths::init_global();

    let log = logger::logger();
    log.info(
        log_module::STARTUP,
        &format!("=== RustVerse v{} startup ===", env!("CARGO_PKG_VERSION")),
    );

    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Exe dir: {:?}",
        paths.exe_dir()
    );
    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Data dir: {:?}",
        paths.data_dir()
    );
    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Log dir: {:?}",
        paths.log_dir()
    );
    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Webview dir: {:?}",
        paths.webview_dir()
    );
    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Logger initialized at {:?}",
        log.log_dir()
    );

    log.info(log_module::STARTUP, "Running DB migration check...");
    migrate_db_to_data_dir();

    let db_path = paths.db_path().clone();
    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Database path: {:?}",
        db_path
    );
    let db = open_or_create(&db_path).unwrap_or_else(|e| {
        dual_log!(
            log,
            "ERROR",
            log_module::STARTUP,
            "Failed to open database: {e}, falling back to in-memory"
        );
        redb::Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .expect("in-memory database should always succeed")
    });
    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Database opened successfully"
    );

    try_migrate_from_toml(&db);
    ensure_version_in_db(&db);

    let app_state = AppState::new(db);

    crate::infrastructure::proxy::init_proxy_resolver_with_store(app_state.store.clone());

    let locale_scan_state = LocaleScanState::new();
    let webview_data_dir = paths.webview_dir().clone();

    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Webview data dir: {:?}",
        webview_data_dir
    );
    dual_log!(
        log,
        "INFO",
        log_module::STARTUP,
        "Log directory: {:?}",
        log.log_dir()
    );

    let log_for_setup = log;
    log.info(log_module::STARTUP, "Building Tauri application...");

    tauri::Builder::default()
        .setup(move |app| {
            log_for_setup.info(log_module::SETUP, "Tauri setup started");

            let main_window = WebviewWindowBuilder::new(
                app,
                app_const::WINDOW_MAIN,
                WebviewUrl::App(app_const::FRONTEND_URL.into()),
            )
            .title(app_const::TITLE)
            .inner_size(975.0, 975.0)
            .min_inner_size(768.0, 890.0)
            .resizable(true)
            .data_directory(webview_data_dir)
            .build();

            match main_window {
                Ok(_) => log_for_setup.info(log_module::SETUP, "Main window created successfully"),
                Err(e) => log_for_setup.error(
                    log_module::SETUP,
                    &format!("Failed to create main window: {e}"),
                ),
            }

            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
            }

            // ── System Tray ──
            let quit = MenuItem::with_id(
                app,
                tray::MENU_QUIT,
                tray_menu::LABEL_QUIT,
                true,
                None::<&str>,
            )?;
            let show = MenuItem::with_id(
                app,
                tray::MENU_SHOW,
                tray_menu::LABEL_SHOW,
                true,
                None::<&str>,
            )?;
            let tray_menu = Menu::with_items(app, &[&show, &quit])?;

            let tray_icon = app.default_window_icon().cloned();

            let mut builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .tooltip(app_const::TITLE);

            if let Some(icon) = tray_icon {
                builder = builder.icon(icon);
            }

            let _tray = builder
                .on_menu_event(move |app, event| match event.id().0.as_str() {
                    tray::MENU_QUIT => {
                        app.exit(0);
                    }
                    tray::MENU_SHOW => {
                        if let Some(window) = app.get_webview_window(app_const::WINDOW_MAIN) {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            // ── Close-to-tray handler ──
            if let Some(window) = app.get_webview_window(app_const::WINDOW_MAIN) {
                let app_handle = app.handle().clone();
                let window_for_event = window.clone();
                let _ = window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let state = app_handle.state::<AppState>();
                        let should_minimize = match (&*state.store).get_settings() {
                            Some(json) => {
                                serde_json::from_str::<crate::domain::settings::UserSettings>(&json)
                                    .map(|s| s.minimize_to_tray)
                                    .unwrap_or(false)
                            }
                            None => false,
                        };
                        if should_minimize {
                            api.prevent_close();
                            let _ = window_for_event.hide();
                        }
                    }
                });
            }

            #[cfg(debug_assertions)]
            {
                // DevTools auto-open disabled — use pnpm tauri dev for debugging
                // let window = tauri::Manager::get_webview_window(app, "main").unwrap();
                // window.open_devtools();
            }
            log_for_setup.info(log_module::SETUP, "Tauri setup completed");

            // ── Start periodic notification auto-cleanup task ──
            {
                use std::sync::atomic::AtomicBool;
                use std::sync::Arc;
                let app_handle = app.handle().clone();
                let store = app_handle.state::<AppState>().store.clone();
                let running = Arc::new(AtomicBool::new(true));
                let running_clone = running.clone();
                // Store the flag in AppState for graceful shutdown
                let app_handle_for_state = app_handle.clone();
                std::thread::spawn(move || {
                    run_notification_cleanup(&*store, &app_handle);
                    while running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_secs(5 * 60));
                        if !running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        run_notification_cleanup(&*store, &app_handle);
                    }
                });
                // P2: Register cleanup on app exit — use Tauri 2.11 RunEvent via the builder
                // The cleanup thread will naturally terminate when the process exits.
                // Store the flag in AppState for potential future use.
                let _ = app_handle_for_state;
            }

            // ── Start background cache cleanup task ──
            {
                let query_cache = app.handle().state::<AppState>().query_cache.clone();
                query_cache.spawn_cleanup_task();
            }

            // ── Start background histver sync (all 3 channels) via manifests.txt ──
            {
                let db = app.handle().state::<AppState>().db.clone();
                startup_sync_manifests(db);
            }

            // ── Init rustup mirror sources (seed built-in on first run) ──
            {
                let state = app.handle().state::<AppState>();
                init_rustup_mirror_sources(&state);
            }

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_prevent_default::init())
        .invoke_handler(tauri::generate_handler![
            check_env,
            refresh_process_path,
            get_log_dir,
            frontend_log,
            uninstall_rustup,
            install_rustup,
            cancel_background_task,
            is_background_task_running,
            invalidate_config_cache,
            reinit_terminal,
            restart_application,
            get_versions,
            get_config,
            list_toolchains,
            install_toolchain,
            uninstall_toolchain,
            set_default_toolchain,
            get_override,
            set_override,
            remove_override,
            list_overrides,
            list_components,
            add_component,
            remove_component,
            list_targets,
            add_target,
            remove_target,
            check_update,
            update_all,
            update_rustup,
            diag_network,
            download_manifests,
            list_cargo_plugins,
            search_plugins,
            install_plugin,
            uninstall_plugin,
            list_env_vars,
            get_env_var,
            set_env_var,
            remove_env_var,
            update_env_var_meta,
            delete_env_var_meta,
            persist_env_var,
            remove_persisted_env_var,
            is_env_var_persisted,
            list_persisted_env_vars,
            get_locale,
            set_locale,
            list_available_locales,
            check_crm_installed,
            install_crm,
            crm_list,
            crm_current,
            crm_version,
            crm_use,
            crm_best,
            crm_default,
            crm_test,
            sync_from_manifests,
            list_hist_releases,
            search_hist_releases,
            count_hist_releases,
            // Settings
            get_settings,
            save_settings,
            // Notifications
            notify_list,
            notify_count,
            notify_create,
            notify_delete,
            notify_delete_all,
            notify_mark_read,
            notify_mark_unread,
            notify_unread_count,
            notification_delete_read_before,
            // Rustup Mirror
            list_rustup_mirror_sources,
            add_rustup_mirror_source,
            update_rustup_mirror_source,
            delete_rustup_mirror_source,
        ])
        .manage(app_state)
        .manage(locale_scan_state)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                let should_minimize = (&*state.store)
                    .get_settings()
                    .and_then(|json| {
                        serde_json::from_str::<crate::domain::settings::UserSettings>(&json).ok()
                    })
                    .map(|s| s.minimize_to_tray)
                    .unwrap_or(false);

                if should_minimize {
                    api.prevent_close();
                    if let Some(w) = window.get_webview_window(app_const::WINDOW_MAIN) {
                        w.hide().ok();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
