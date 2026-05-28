//! Proxy configuration resolver for subprocess execution.
//!
//! Resolves proxy settings from the redb database (user settings table),
//! caches the result in memory to avoid repeated DB reads, and applies
//! the appropriate environment variables to terminal commands.
//!
//! # Data flow
//! ```text
//! run_command() → get_proxy_config() → cache hit? → return cached
//!                                      ↘ cache miss → DB read → validate → cache → return
//! ```
//!
//! # Error handling
//! Any failure (DB unreadable, corrupted data, invalid settings) results in
//! `ProxyConfig::Direct` — the subprocess runs without proxy, which is the
//! safest fallback.

use std::sync::{Arc, Mutex, OnceLock};

use crate::domain::repository::DataStore;
use crate::infrastructure::logger;
use crate::domain::settings::UserSettings;

// ── Global instance ──

static PROXY_RESOLVER: OnceLock<ProxyResolver> = OnceLock::new();

/// Initialize the global proxy resolver. Must be called during app startup
/// before any terminal command is executed.
///
/// **Legacy**: accepts a raw `redb::Database`.  Prefer
/// `init_proxy_resolver_with_store` for new code.
#[allow(dead_code)]
pub fn init_proxy_resolver(db: redb::Database) {
    let store: Arc<dyn DataStore> =
        Arc::new(crate::infrastructure::db::RedbDataStore::new(Arc::new(db)));
    PROXY_RESOLVER
        .set(ProxyResolver {
            cache: Mutex::new(CachedEntry::Empty),
            store,
        })
        .expect("ProxyResolver already initialized");
}

/// Initialize the global proxy resolver from a `DataStore` (preferred).
#[allow(dead_code)]
pub fn init_proxy_resolver_with_store(store: Arc<dyn DataStore>) {
    PROXY_RESOLVER
        .set(ProxyResolver {
            cache: Mutex::new(CachedEntry::Empty),
            store,
        })
        .expect("ProxyResolver already initialized");
}

/// Obtain the currently active proxy configuration.
///
/// Checks the in-memory cache first; on cache miss, reads user settings
/// from the database, parses them, and populates the cache.
///
/// Always returns a valid `ProxyConfig` — errors fall back to `Direct`.
pub fn get_proxy_config() -> ProxyConfig {
    if let Some(resolver) = PROXY_RESOLVER.get() {
        return resolver.resolve();
    }
    // Fallback: no resolver available (should only happen in early startup
    // or in tests that don't call init_proxy_resolver_with_store).
    logger::logger().info(
        "proxy",
        "ProxyResolver not initialized – using direct connection",
    );
    ProxyConfig::Direct
}

/// Invalidate the proxy cache so the next `get_proxy_config()` call
/// re-reads from the database.  Call this after the user saves new
/// settings in the Settings page.
#[allow(dead_code)]
pub fn invalidate_cache() {
    if let Some(resolver) = PROXY_RESOLVER.get() {
        let mut guard = resolver.cache.lock().unwrap_or_else(|e| e.into_inner());
        *guard = CachedEntry::Empty;
        logger::logger().info(
            "proxy",
            "cache invalidated – next terminal call will re-read from DB",
        );
    }
}

/// Apply proxy environment variables to the **current process**.
///
/// Called when the user saves proxy settings, so that any process spawned
/// from the current process inherits the correct proxy variables without
/// needing a restart.  Also affects the WebView's network stack on some
/// platforms.
/// # Safety
///
/// `std::env::set_var` / `remove_var` are marked unsafe because they can
/// cause data races in multithreaded programs.  Our call site is the
/// synchronous `save_settings` command handler, which is serialized by
/// Tauri's command queue — no concurrent env mutation occurs.
pub fn apply_to_current_process(config: &ProxyConfig) {
    match config {
        ProxyConfig::Direct => {
            unsafe {
                std::env::remove_var("HTTP_PROXY");
                std::env::remove_var("HTTPS_PROXY");
                std::env::remove_var("http_proxy");
                std::env::remove_var("https_proxy");
                std::env::remove_var("ALL_PROXY");
                std::env::remove_var("all_proxy");
                std::env::remove_var("SOCKS_PROXY");
                std::env::remove_var("socks_proxy");
                std::env::remove_var("SOCKS5_PROXY");
                std::env::remove_var("socks5_proxy");
                std::env::remove_var("NO_PROXY");
                std::env::remove_var("no_proxy");
            }
            logger::logger().info("proxy", "current process proxy env vars cleared (direct)");
        }
        ProxyConfig::System => {
            unsafe {
                std::env::remove_var("HTTP_PROXY");
                std::env::remove_var("HTTPS_PROXY");
                std::env::remove_var("http_proxy");
                std::env::remove_var("https_proxy");
                std::env::remove_var("ALL_PROXY");
                std::env::remove_var("all_proxy");
            }
            logger::logger().info("proxy", "current process proxy env vars cleared (system mode – OS handles proxy)");
        }
        ProxyConfig::Manual { host, port } => {
            let url = format!("http://{}:{}", host, port);
            unsafe {
                std::env::set_var("HTTP_PROXY", &url);
                std::env::set_var("HTTPS_PROXY", &url);
                std::env::set_var("http_proxy", &url);
                std::env::set_var("https_proxy", &url);
            }
            logger::logger().info("proxy", &format!("current process proxy set to {}", url));
        }
    }
}

// ── Resolver ──

/// Internal cache state.
#[derive(Debug)]
enum CachedEntry {
    Empty,
    Resolved(ProxyConfig),
}

/// Global proxy resolver with in-memory cache bound to the data store.
pub struct ProxyResolver {
    cache: Mutex<CachedEntry>,
    store: Arc<dyn DataStore>,
}

impl std::fmt::Debug for ProxyResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyResolver")
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

impl ProxyResolver {
    /// Resolve the proxy configuration: cache → DB → fallback.
    ///
    /// Holds the cache lock through the entire resolve (including DB read)
    /// to eliminate a TOCTOU window where concurrent callers could trigger
    /// duplicate DB reads.
    fn resolve(&self) -> ProxyConfig {
        let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());

        // 1. Check cache (under lock)
        if let CachedEntry::Resolved(ref config) = *guard {
            logger::logger().info("proxy", &format!("cache hit → {:?}", config));
            return config.clone();
        }

        // 2. Cache miss — read from database (lock still held)
        logger::logger().info("proxy", "cache miss – reading user settings from database");
        let config = match self.store.get_settings() {
            Some(json) => match serde_json::from_str::<UserSettings>(&json) {
                Ok(settings) => Self::parse_settings(&settings),
                Err(e) => {
                    logger::logger().error(
                        "proxy",
                        &format!("failed to parse settings JSON: {e} – falling back to direct"),
                    );
                    ProxyConfig::Direct
                }
            },
            None => {
                logger::logger().info(
                    "proxy",
                    "no settings found in database – using defaults (direct)",
                );
                ProxyConfig::Direct
            }
        };

        // 3. Store in cache (lock still held)
        *guard = CachedEntry::Resolved(config.clone());

        logger::logger().info("proxy", &format!("resolved → {:?} (cached)", config));
        config
    }
}

// ── Parsing ──

impl ProxyResolver {
    /// Convert `UserSettings` into a concrete `ProxyConfig`.
    fn parse_settings(s: &UserSettings) -> ProxyConfig {
        match s.proxy_type.as_str() {
            "none" => {
                logger::logger().info("proxy", "proxy_type=none → direct connection");
                ProxyConfig::Direct
            }
            "system" => {
                logger::logger().info(
                    "proxy",
                    "proxy_type=system → pass-through (OS handles proxy)",
                );
                ProxyConfig::System
            }
            "manual" => {
                let host = s.proxy_host.trim().to_string();
                let port = s.proxy_port;

                if host.is_empty() || port == 0 {
                    logger::logger().warn(
                        "proxy",
                        &format!(
                            "proxy_type=manual but host='{host}' / port={port} are incomplete – falling back to direct"
                        ),
                    );
                    return ProxyConfig::Direct;
                }

                let config = ProxyConfig::Manual {
                    host: host.clone(),
                    port,
                };
                logger::logger().info("proxy", &format!("proxy_type=manual → {}:{}", host, port));
                config
            }
            other => {
                logger::logger().warn(
                    "proxy",
                    &format!("unknown proxy_type='{other}' – falling back to direct"),
                );
                ProxyConfig::Direct
            }
        }
    }
}

// ── Proxy configuration enum ──

/// The resolved proxy configuration to apply to subprocesses.
#[derive(Debug, Clone)]
pub enum ProxyConfig {
    /// No proxy — do not set any proxy environment variables.
    Direct,
    /// Pass-through: let the operating system handle proxy settings.
    /// No additional environment variables are injected.
    System,
    /// Manual proxy (HTTP/HTTPS): `http://host:port`.
    /// Applied to both `HTTP_PROXY` and `HTTPS_PROXY`.
    Manual { host: String, port: u16 },
}

// ── Environment variable injection ──

/// Apply proxy environment variables to a `tokio::process::Command`.
///
/// Sets `HTTP_PROXY`, `HTTPS_PROXY`, and `http_proxy` / `https_proxy`
/// (lowercase variants for broader tool compatibility).
/// Clears `NO_PROXY` when a proxy is active to avoid conflicts.
pub fn apply_proxy_env(cmd: &mut tokio::process::Command, config: &ProxyConfig) {
    match config {
        ProxyConfig::Direct => {
            logger::logger().info(
                "proxy",
                "applying Direct mode - clearing all proxy environment variables",
            );
            // Ensure NO proxy env vars leak through - clear ALL known proxy-related variables
            // This includes:
            // - Standard HTTP/HTTPS proxy vars (uppercase and lowercase)
            // - SOCKS proxy vars
            // - NO_PROXY exclusion list (can interfere with direct connections)
            // - ALL_PROXY catch-all variable
            cmd.env_remove("HTTP_PROXY")
                .env_remove("HTTPS_PROXY")
                .env_remove("http_proxy")
                .env_remove("https_proxy")
                .env_remove("ALL_PROXY")
                .env_remove("all_proxy")
                .env_remove("SOCKS_PROXY")
                .env_remove("socks_proxy")
                .env_remove("SOCKS5_PROXY")
                .env_remove("socks5_proxy")
                .env_remove("NO_PROXY")
                .env_remove("no_proxy");
            logger::logger().info("proxy", "cleared all proxy env vars (direct connection)");
        }
        ProxyConfig::System => {
            logger::logger().info(
                "proxy",
                "applying System mode - passing through OS proxy settings",
            );
            // System mode: don't touch proxy env vars, let the OS handle it
            // But still clear any explicitly set vars that might override system settings
            cmd.env_remove("HTTP_PROXY")
                .env_remove("HTTPS_PROXY")
                .env_remove("http_proxy")
                .env_remove("https_proxy")
                .env_remove("ALL_PROXY")
                .env_remove("all_proxy");
            logger::logger().info(
                "proxy",
                "cleared user-set proxy env vars, OS proxy will be used",
            );
        }
        ProxyConfig::Manual { host, port } => {
            let url = format!("http://{}:{}", host, port);
            logger::logger().info(
                "proxy",
                &format!("applying Manual mode - setting proxy to {}", url),
            );
            cmd.env("HTTP_PROXY", &url)
                .env("HTTPS_PROXY", &url)
                .env("http_proxy", &url)
                .env("https_proxy", &url);
            logger::logger().info(
                "proxy",
                &format!(
                    "applied proxy env vars: HTTP_PROXY={} HTTPS_PROXY={}",
                    url, url
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct() {
        let s = UserSettings {
            proxy_type: "none".into(),
            ..Default::default()
        };
        let config = ProxyResolver::parse_settings(&s);
        assert!(matches!(config, ProxyConfig::Direct));
    }

    #[test]
    fn test_parse_system() {
        let s = UserSettings {
            proxy_type: "system".into(),
            ..Default::default()
        };
        let config = ProxyResolver::parse_settings(&s);
        assert!(matches!(config, ProxyConfig::System));
    }

    #[test]
    fn test_parse_manual_valid() {
        let s = UserSettings {
            proxy_type: "manual".into(),
            proxy_host: "127.0.0.1".into(),
            proxy_port: 7890,
            ..Default::default()
        };
        let config = ProxyResolver::parse_settings(&s);
        assert!(matches!(
            config,
            ProxyConfig::Manual {
                host,
                port,
            } if host == "127.0.0.1" && port == 7890
        ));
    }

    #[test]
    fn test_parse_manual_empty_host_falls_back_to_direct() {
        let s = UserSettings {
            proxy_type: "manual".into(),
            proxy_host: "".into(),
            proxy_port: 8080,
            ..Default::default()
        };
        let config = ProxyResolver::parse_settings(&s);
        assert!(matches!(config, ProxyConfig::Direct));
    }

    #[test]
    fn test_parse_manual_zero_port_falls_back_to_direct() {
        let s = UserSettings {
            proxy_type: "manual".into(),
            proxy_host: "proxy.example.com".into(),
            proxy_port: 0,
            ..Default::default()
        };
        let config = ProxyResolver::parse_settings(&s);
        assert!(matches!(config, ProxyConfig::Direct));
    }

    #[test]
    fn test_parse_unknown_type_falls_back_to_direct() {
        let s = UserSettings {
            proxy_type: "socks5".into(),
            ..Default::default()
        };
        let config = ProxyResolver::parse_settings(&s);
        assert!(matches!(config, ProxyConfig::Direct));
    }
}
