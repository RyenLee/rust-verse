//! Lightweight in-memory cache for DB-sourced config values.
//!
//! Config values like events, parsing markers, and locale are read
//! on every command invocation but rarely change at runtime. This
//! cache avoids repeated redb read transactions for static config.
//!
//! Each cached entry carries a TTL (default 300s). When the TTL
//! expires, the next read triggers a DB refresh automatically.
//! Explicit `invalidate()` is still supported for immediate refresh
//! after configuration changes.

use std::sync::RwLock;
use std::time::Instant;

use crate::domain::config_keys::keys;
use crate::domain::repository::DataStore;
use crate::infrastructure::db::{
    EventsConfigValues, ParsingConfigValues, get_events_config, get_parsing_config,
};

const DEFAULT_TTL_SECS: u64 = 300;

type Cached<T> = RwLock<Option<(Instant, T)>>;

pub struct AppConfigCache {
    events: Cached<EventsConfigValues>,
    parsing: Cached<ParsingConfigValues>,
    locale: Cached<String>,
    timeout_rustup_check: Cached<u64>,
    ttl_secs: u64,
}

impl AppConfigCache {
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_TTL_SECS)
    }

    pub fn with_ttl(ttl_secs: u64) -> Self {
        Self {
            events: RwLock::new(None),
            parsing: RwLock::new(None),
            locale: RwLock::new(None),
            timeout_rustup_check: RwLock::new(None),
            ttl_secs,
        }
    }

    fn is_fresh<T>(entry: &Option<(Instant, T)>, ttl_secs: u64) -> bool {
        entry.as_ref().map_or(false, |(ts, _)| ts.elapsed().as_secs() < ttl_secs)
    }

    fn warm_locale_and_timeout(&self, store: &dyn DataStore) {
        let batch = store.get_config_batch(&[keys::LOCALE_FORCE, keys::TIMEOUT_RUSTUP_CHECK]);
        let locale = batch
            .get(keys::LOCALE_FORCE)
            .cloned()
            .unwrap_or_else(crate::infrastructure::config::defaults::force_locale);
        let timeout = batch
            .get(keys::TIMEOUT_RUSTUP_CHECK)
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        let now = Instant::now();
        *self.locale.write().unwrap() = Some((now, locale));
        *self.timeout_rustup_check.write().unwrap() = Some((now, timeout));
    }

    pub fn get_events(&self, store: &dyn DataStore) -> EventsConfigValues {
        {
            let guard = self.events.read().unwrap();
            if Self::is_fresh(&guard, self.ttl_secs) {
                return guard.as_ref().unwrap().1.clone();
            }
        }
        let events = get_events_config(store);
        *self.events.write().unwrap() = Some((Instant::now(), events.clone()));
        events
    }

    pub fn get_parsing(&self, store: &dyn DataStore) -> ParsingConfigValues {
        {
            let guard = self.parsing.read().unwrap();
            if Self::is_fresh(&guard, self.ttl_secs) {
                return guard.as_ref().unwrap().1.clone();
            }
        }
        let parsing = get_parsing_config(store);
        *self.parsing.write().unwrap() = Some((Instant::now(), parsing.clone()));
        parsing
    }

    pub fn get_locale(&self, store: &dyn DataStore) -> String {
        {
            let guard = self.locale.read().unwrap();
            if Self::is_fresh(&guard, self.ttl_secs) {
                return guard.as_ref().unwrap().1.clone();
            }
        }
        self.warm_locale_and_timeout(store);
        self.locale.read().unwrap().as_ref().unwrap().1.clone()
    }

    pub fn get_timeout_rustup_check(&self, store: &dyn DataStore) -> u64 {
        {
            let guard = self.timeout_rustup_check.read().unwrap();
            if Self::is_fresh(&guard, self.ttl_secs) {
                return guard.as_ref().unwrap().1;
            }
        }
        self.warm_locale_and_timeout(store);
        self.timeout_rustup_check.read().unwrap().as_ref().unwrap().1
    }

    pub fn invalidate(&self) {
        *self.events.write().unwrap() = None;
        *self.parsing.write().unwrap() = None;
        *self.locale.write().unwrap() = None;
        *self.timeout_rustup_check.write().unwrap() = None;
    }
}