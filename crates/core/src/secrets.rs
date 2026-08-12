//! OS credential store with a permission-restricted file fallback,
//! ported from `secrets.py`.

use crate::paths::{config_dir, APP_NAME};
use anyhow::Result;
use std::path::PathBuf;

pub struct SecretStore {
    fallback_path: PathBuf,
    /// Which backend is in use: "system keychain" or "restricted local file".
    backend: std::cell::Cell<&'static str>,
}

impl SecretStore {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let target = base_dir.unwrap_or_else(config_dir);
        let _ = std::fs::create_dir_all(&target);
        Self {
            fallback_path: target.join("secrets.json"),
            backend: std::cell::Cell::new("system keychain"),
        }
    }

    fn read_fallback(&self) -> std::collections::HashMap<String, String> {
        match std::fs::read_to_string(&self.fallback_path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => std::collections::HashMap::new(),
        }
    }

    fn write_fallback(&self, values: &std::collections::HashMap<String, String>) {
        let text = serde_json::to_string(values).unwrap_or_else(|_| "{}".to_string());
        let _ = std::fs::write(&self.fallback_path, text);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.fallback_path, std::fs::Permissions::from_mode(0o600));
        }
    }

    pub fn get(&self, key: &str, default: &str) -> String {
        if self.backend.get() == "system keychain" {
            match keyring::Entry::new(APP_NAME, key).and_then(|entry| entry.get_password()) {
                Ok(value) => return value,
                Err(_) => {
                    // Once the keychain fails, stay on the file fallback so
                    // every later call does not retry the OS store.
                    self.backend.set("restricted local file");
                }
            }
        }
        self.read_fallback().get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    /// Which backend is in use: "system keychain" or "restricted local file".
    pub fn backend(&self) -> &'static str {
        self.backend.get()
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        if self.backend.get() == "system keychain" {
            if let Ok(entry) = keyring::Entry::new(APP_NAME, key) {
                if entry.set_password(value).is_ok() {
                    return Ok(());
                }
            }
            // Once the keychain fails, stay on the file fallback so every
            // later call does not retry the OS store.
            self.backend.set("restricted local file");
        }
        let mut values = self.read_fallback();
        if value.is_empty() {
            values.remove(key);
        } else {
            values.insert(key.to_string(), value.to_string());
        }
        self.write_fallback(&values);
        Ok(())
    }

    pub fn delete(&self, key: &str) {
        let _ = keyring::Entry::new(APP_NAME, key).and_then(|entry| entry.delete_credential());
        let mut values = self.read_fallback();
        values.remove(key);
        self.write_fallback(&values);
    }

    /// Force the file fallback (tests must not touch the OS keychain,
    /// which is unsafe under concurrent access and can wedge securityd).
    #[cfg(test)]
    pub fn force_fallback_for_tests(&self) {
        self.backend.set("restricted local file");
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    /// macOS keychain access is not safe under concurrent threads; tests
    /// that touch the real keychain serialize on this lock.
    pub(crate) static KEYCHAIN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::test_util::KEYCHAIN_LOCK;

    #[test]
    fn empty_value_removes_key() {
        let _guard = KEYCHAIN_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::new(Some(dir.path().to_path_buf()));
        store.force_fallback_for_tests();
        store.set("telegram_bot_token", "123:abc").unwrap();
        assert_eq!(store.get("telegram_bot_token", ""), "123:abc");
        store.set("telegram_bot_token", "").unwrap();
        assert_eq!(store.get("telegram_bot_token", "dflt"), "dflt");
    }

    #[test]
    fn fallback_roundtrip() {
        let _guard = KEYCHAIN_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::new(Some(dir.path().to_path_buf()));
        store.force_fallback_for_tests();
        store.set("telegram_bot_token", "123:abc").unwrap();
        let store2 = SecretStore::new(Some(dir.path().to_path_buf()));
        assert_eq!(store2.get("telegram_bot_token", ""), "123:abc");
        assert_eq!(store2.get("missing", "dflt"), "dflt");
        store2.delete("telegram_bot_token");
        let store3 = SecretStore::new(Some(dir.path().to_path_buf()));
        assert_eq!(store3.get("telegram_bot_token", ""), "");
        // Fallback file should be 0600 on unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join("secrets.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
