//! Application settings, ported from `settings.py`.

use crate::db::Database;
use std::sync::Arc;
use crate::paths::default_export_dir;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

pub const LIBRARY_ROOT: &str = "library_root";
pub const WORKSPACE_TABS: &str = "workspace_tabs";
pub const ACTIVE_WORKSPACE_ID: &str = "active_workspace_id";
pub const CLOSED_WORKSPACE_TABS: &str = "closed_workspace_tabs";
pub const EXPORT_DIR: &str = "export_dir";
pub const FAST_RANDOM: &str = "fast_random";
pub const AUTO_INDEX: &str = "auto_index";
pub const VERIFY_DURING_INDEX: &str = "verify_during_index";
pub const THUMBNAILS_DURING_INDEX: &str = "thumbnails_during_index";
pub const HOVER_PREVIEWS: &str = "hover_previews";
pub const DEEP_SCAN: &str = "deep_scan";
pub const AVOID_REPEATS: &str = "avoid_repeats";
pub const RANDOM_FOLDER_MODE: &str = "random_folder_mode";
pub const RANDOM_FOLDERS: &str = "random_folders";
pub const UI_SCALE: &str = "ui_scale";
pub const THEME_MODE: &str = "theme_mode";
/// User-created themes (JSON array of theme objects, see app `CustomTheme`).
pub const CUSTOM_THEMES: &str = "custom_themes";
pub const PERFORMANCE_MODE: &str = "performance_mode";
pub const LIBRARY_DENSITY: &str = "library_density";
pub const FIT_LIBRARY_THUMBNAILS: &str = "fit_library_thumbnails";
pub const EXPORT_ENCODER: &str = "export_encoder";
pub const SIDEBAR_COLLAPSED: &str = "sidebar_collapsed";
pub const PREPARE_EXPANDED: &str = "prepare_expanded";
pub const SORT_MODE: &str = "sort_mode";
pub const FOLDER_SORT_MODE: &str = "folder_sort_mode";
pub const CLEANUP_POLICY: &str = "cleanup_policy";
pub const X_LIMIT_MB: &str = "x_limit_mb";
/// Last window size (`WxH`), restored on launch (a Rust-port addition).
pub const WINDOW_BOUNDS: &str = "window_bounds";
pub const X_DURATION_SECONDS: &str = "x_duration_seconds";
pub const TELEGRAM_MODE: &str = "telegram_mode";
pub const TELEGRAM_DESTINATION: &str = "telegram_destination";
pub const TELEGRAM_API_ID: &str = "telegram_api_id";
pub const TELEGRAM_PHONE: &str = "telegram_phone";
pub const TELEGRAM_BOT_CONFIGURED: &str = "telegram_bot_configured";
pub const TELEGRAM_PERSONAL_CONFIGURED: &str = "telegram_personal_configured";

pub fn defaults() -> HashMap<&'static str, Value> {
    let mut map = HashMap::new();
    map.insert(LIBRARY_ROOT, json_str(""));
    map.insert(WORKSPACE_TABS, Value::Array(vec![]));
    map.insert(ACTIVE_WORKSPACE_ID, json_str(""));
    map.insert(CLOSED_WORKSPACE_TABS, Value::Array(vec![]));
    map.insert(EXPORT_DIR, json_str(&default_export_dir().to_string_lossy()));
    map.insert(FAST_RANDOM, Value::Bool(true));
    map.insert(AUTO_INDEX, Value::Bool(false));
    map.insert(VERIFY_DURING_INDEX, Value::Bool(true));
    map.insert(THUMBNAILS_DURING_INDEX, Value::Bool(false));
    map.insert(HOVER_PREVIEWS, Value::Bool(true));
    map.insert(DEEP_SCAN, Value::Bool(false));
    map.insert(AVOID_REPEATS, Value::Bool(true));
    map.insert(RANDOM_FOLDER_MODE, json_str("all"));
    map.insert(RANDOM_FOLDERS, Value::Array(vec![]));
    map.insert(UI_SCALE, Value::from(1.0_f64));
    map.insert(THEME_MODE, json_str("relay"));
    map.insert(CUSTOM_THEMES, Value::Array(vec![]));
    map.insert(PERFORMANCE_MODE, json_str("automatic"));
    map.insert(LIBRARY_DENSITY, json_str("default"));
    map.insert(FIT_LIBRARY_THUMBNAILS, Value::Bool(false));
    map.insert(EXPORT_ENCODER, json_str("auto"));
    // The redesigned workbench opens on the compact activity rail shown in
    // the approved shell. Users can still expand it when they want labels.
    map.insert(SIDEBAR_COLLAPSED, Value::Bool(true));
    map.insert(PREPARE_EXPANDED, Value::Bool(false));
    map.insert(SORT_MODE, json_str("newest"));
    map.insert(FOLDER_SORT_MODE, json_str("name_asc"));
    map.insert(CLEANUP_POLICY, json_str("keep"));
    map.insert(X_LIMIT_MB, Value::from(512_i64));
    map.insert(X_DURATION_SECONDS, Value::from(140_i64));
    map.insert(TELEGRAM_MODE, json_str("bot"));
    map.insert(TELEGRAM_DESTINATION, json_str(""));
    map.insert(TELEGRAM_API_ID, json_str(""));
    map.insert(TELEGRAM_PHONE, json_str(""));
    map.insert(TELEGRAM_BOT_CONFIGURED, Value::Bool(false));
    map.insert(TELEGRAM_PERSONAL_CONFIGURED, Value::Bool(false));
    map
}

fn json_str(value: &str) -> Value {
    Value::String(value.to_string())
}

pub struct Settings {
    database: Arc<Database>,
}

impl Settings {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Raw JSON value for `key`, falling back to the app default.
    pub fn get(&self, key: &str) -> Result<Value> {
        let fallback = defaults()
            .get(key)
            .cloned()
            .unwrap_or(Value::Null);
        let raw = self.database.get_setting_raw(key)?;
        match raw {
            Some(stored) => match serde_json::from_str::<Value>(&stored) {
                Ok(value) => Ok(value),
                Err(_) => Ok(fallback),
            },
            None => Ok(fallback),
        }
    }

    /// Typed convenience getters.
    pub fn get_string(&self, key: &str) -> Result<String> {
        Ok(self.get(key)?.as_str().unwrap_or("").to_string())
    }

    pub fn get_bool(&self, key: &str) -> Result<bool> {
        Ok(self.get(key)?.as_bool().unwrap_or(false))
    }

    pub fn get_f64(&self, key: &str) -> Result<f64> {
        Ok(self.get(key)?.as_f64().unwrap_or(0.0))
    }

    pub fn get_i64(&self, key: &str) -> Result<i64> {
        Ok(self.get(key)?.as_i64().unwrap_or(0))
    }

    pub fn get_strings(&self, key: &str) -> Result<Vec<String>> {
        Ok(self
            .get(key)?
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn set(&self, key: &str, value: Value) -> Result<()> {
        let mut value = value;
        if key == THEME_MODE {
            let text = value.as_str().unwrap_or("").to_string();
            value = if matches!(
                text.as_str(),
                "relay" | "pitch_black" | "full_white" | "frosted_glass" | "graphite_glass"
            ) {
                Value::String(text)
            } else {
                Value::String("relay".into())
            };
        }
        if key == RANDOM_FOLDER_MODE {
            let text = value.as_str().unwrap_or("").to_string();
            value = if text == "selected" {
                Value::String("selected".into())
            } else {
                Value::String("all".into())
            };
        }
        if key == PERFORMANCE_MODE {
            let text = value.as_str().unwrap_or("").to_string();
            value = if text == "maximum" {
                Value::String("maximum".into())
            } else {
                Value::String("automatic".into())
            };
        }
        if key == LIBRARY_DENSITY {
            let text = value.as_str().unwrap_or("").to_string();
            value = if text == "compact" {
                Value::String("compact".into())
            } else {
                Value::String("default".into())
            };
        }
        if key == FOLDER_SORT_MODE {
            let text = value.as_str().unwrap_or("").to_string();
            let allowed = [
                "name_asc",
                "name_desc",
                "added_recent",
                "added_old",
                "recent",
                "stale",
                "count_desc",
                "count_asc",
            ];
            value = if allowed.contains(&text.as_str()) {
                Value::String(text)
            } else {
                Value::String("name_asc".into())
            };
        }
        if key == EXPORT_ENCODER {
            let text = value.as_str().unwrap_or("").to_string();
            value = if matches!(text.as_str(), "auto" | "hardware" | "software") {
                Value::String(text)
            } else {
                Value::String("auto".into())
            };
        }
        if matches!(key, LIBRARY_ROOT | EXPORT_DIR) {
            if let Some(text) = value.as_str() {
                if !text.is_empty() {
                    let expanded = std::path::Path::new(text).expanduser().to_path_buf();
                    if key == EXPORT_DIR {
                        let _ = std::fs::create_dir_all(&expanded);
                    }
                    value = Value::String(expanded.to_string_lossy().into_owned());
                }
            }
        }
        self.database
            .set_setting(key, &serde_json::to_string(&value)?)
    }

    /// All settings as JSON values (stored, falling back to defaults).
    pub fn as_map(&self) -> Result<HashMap<String, Value>> {
        let stored = self.database.get_settings()?;
        let mut out: HashMap<String, Value> = HashMap::new();
        for (key, default) in defaults() {
            let value = match stored.get(key) {
                Some(raw) => serde_json::from_str::<Value>(raw).unwrap_or(default.clone()),
                None => default.clone(),
            };
            out.insert(key.to_string(), value);
        }
        Ok(out)
    }
}

trait PathExpand {
    fn expanduser(&self) -> std::path::PathBuf;
}

impl PathExpand for std::path::Path {
    fn expanduser(&self) -> std::path::PathBuf {
        let Some(text) = self.to_str() else {
            return self.to_path_buf();
        };
        if text == "~" {
            return dirs::home_dir().unwrap_or_else(|| self.to_path_buf());
        }
        if let Some(rest) = text.strip_prefix("~/") {
            return dirs::home_dir()
                .map(|home| home.join(rest))
                .unwrap_or_else(|| self.to_path_buf());
        }
        if let Some(rest) = text.strip_prefix("~\\") {
            return dirs::home_dir()
                .map(|home| home.join(rest))
                .unwrap_or_else(|| self.to_path_buf());
        }
        self.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_settings() -> Settings {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("test.sqlite3")).unwrap();
        Settings::new(Arc::new(db))
    }

    #[test]
    fn defaults_apply() {
        let settings = open_settings();
        assert_eq!(settings.get_string(THEME_MODE).unwrap(), "relay");
        assert!(settings.get_bool(FAST_RANDOM).unwrap());
        assert_eq!(settings.get_i64(X_LIMIT_MB).unwrap(), 512);
        assert_eq!(settings.get_i64(X_DURATION_SECONDS).unwrap(), 140);
        assert_eq!(settings.get_f64(UI_SCALE).unwrap(), 1.0);
    }

    #[test]
    fn validation_rules() {
        let settings = open_settings();
        settings
            .set(THEME_MODE, Value::String("neon".into()))
            .unwrap();
        assert_eq!(settings.get_string(THEME_MODE).unwrap(), "relay");
        settings
            .set(THEME_MODE, Value::String("full_white".into()))
            .unwrap();
        assert_eq!(settings.get_string(THEME_MODE).unwrap(), "full_white");
        settings
            .set(THEME_MODE, Value::String("frosted_glass".into()))
            .unwrap();
        assert_eq!(settings.get_string(THEME_MODE).unwrap(), "frosted_glass");
        settings
            .set(THEME_MODE, Value::String("graphite_glass".into()))
            .unwrap();
        assert_eq!(settings.get_string(THEME_MODE).unwrap(), "graphite_glass");
        settings
            .set(FOLDER_SORT_MODE, Value::String("bogus".into()))
            .unwrap();
        assert_eq!(settings.get_string(FOLDER_SORT_MODE).unwrap(), "name_asc");
        settings
            .set(LIBRARY_DENSITY, Value::String("compact".into()))
            .unwrap();
        assert_eq!(settings.get_string(LIBRARY_DENSITY).unwrap(), "compact");
    }

    #[test]
    fn roundtrip_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let db = Database::open(&path).unwrap();
        let settings = Settings::new(Arc::new(db));
        settings
            .set(TELEGRAM_DESTINATION, Value::String("@channel".into()))
            .unwrap();
        settings
            .set(RANDOM_FOLDERS, serde_json::json!(["a", "b"]))
            .unwrap();
        let db = Database::open(&path).unwrap();
        let settings = Settings::new(Arc::new(db));
        assert_eq!(
            settings.get_string(TELEGRAM_DESTINATION).unwrap(),
            "@channel"
        );
        assert_eq!(settings.get_strings(RANDOM_FOLDERS).unwrap(), ["a", "b"]);
    }
}
