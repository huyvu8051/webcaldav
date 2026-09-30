use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "webcaldav.connection";
#[cfg(target_arch = "wasm32")]
const PREFS_KEY: &str = "webcaldav.calendar_prefs";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ConnectionConfig {
    pub server_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarPrefs {
    pub colors: HashMap<String, String>,
    pub hidden: HashMap<String, bool>,
}

#[cfg(target_arch = "wasm32")]
pub fn load() -> Option<ConnectionConfig> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let raw = storage.get_item(STORAGE_KEY).ok()??;
    serde_json::from_str(&raw).ok()
}

#[cfg(target_arch = "wasm32")]
pub fn save(config: &ConnectionConfig) {
    if let Ok(raw) = serde_json::to_string(config) {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
            let _ = storage.set_item(STORAGE_KEY, &raw);
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn clear() {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
        let _ = storage.remove_item(STORAGE_KEY);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_calendar_prefs() -> CalendarPrefs {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item(PREFS_KEY).ok())
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub fn save_calendar_prefs(prefs: &CalendarPrefs) {
    if let Ok(raw) = serde_json::to_string(prefs) {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
            let _ = storage.set_item(PREFS_KEY, &raw);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Option<ConnectionConfig> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(_config: &ConnectionConfig) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_calendar_prefs() -> CalendarPrefs {
    CalendarPrefs::default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_calendar_prefs(_prefs: &CalendarPrefs) {}
