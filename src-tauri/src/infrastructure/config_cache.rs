//! Lightweight in-memory cache for DB-sourced config values.
//!
//! Config values like events, parsing markers, and locale are read
//! on every command invocation but rarely change at runtime. This
//! cache avoids repeated redb read transactions for static config.

use std::sync::RwLock;

use crate::domain::config_keys::keys;
use crate::domain::repository::DataStore;
use crate::infrastructure::db::{
    EventsConfigValues, ParsingConfigValues, get_events_config, get_parsing_config,
};

pub struct AppConfigCache {
    events: RwLock<Option<EventsConfigValues>>,
    parsing: RwLock<Option<ParsingConfigValues>>,
    locale: RwLock<Option<String>>,
    timeout_rustup_check: RwLock<Option<u64>>,
}

impl AppConfigCache {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(None),
            parsing: RwLock::new(None),
            locale: RwLock::new(None),
            timeout_rustup_check: RwLock::new(None),
        }
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
        *self.locale.write().unwrap() = Some(locale);
        *self.timeout_rustup_check.write().unwrap() = Some(timeout);
    }

    pub fn get_events(&self, store: &dyn DataStore) -> EventsConfigValues {
        if let Some(cached) = self.events.read().unwrap().as_ref() {
            return cached.clone();
        }
        let events = get_events_config(store);
        *self.events.write().unwrap() = Some(events.clone());
        events
    }

    pub fn get_parsing(&self, store: &dyn DataStore) -> ParsingConfigValues {
        if let Some(cached) = self.parsing.read().unwrap().as_ref() {
            return cached.clone();
        }
        let parsing = get_parsing_config(store);
        *self.parsing.write().unwrap() = Some(parsing.clone());
        parsing
    }

    pub fn get_locale(&self, store: &dyn DataStore) -> String {
        if let Some(cached) = self.locale.read().unwrap().as_ref() {
            return cached.clone();
        }
        self.warm_locale_and_timeout(store);
        self.locale.read().unwrap().as_ref().unwrap().clone()
    }

    pub fn get_timeout_rustup_check(&self, store: &dyn DataStore) -> u64 {
        if let Some(cached) = self.timeout_rustup_check.read().unwrap().as_ref() {
            return *cached;
        }
        self.warm_locale_and_timeout(store);
        *self.timeout_rustup_check.read().unwrap().as_ref().unwrap()
    }

    pub fn invalidate(&self) {
        *self.events.write().unwrap() = None;
        *self.parsing.write().unwrap() = None;
        *self.locale.write().unwrap() = None;
        *self.timeout_rustup_check.write().unwrap() = None;
    }
}