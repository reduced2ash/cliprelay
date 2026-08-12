//! Application directory resolution (ported from `paths.py`).
//!
//! On macOS: config/data under `~/Library/Application Support/ClipRelay`,
//! cache under `~/Library/Caches/ClipRelay`, logs under `~/Library/Logs`.
//! On Windows: `%APPDATA%` for config/data, `%LOCALAPPDATA%` for cache.

use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "ClipRelay";

fn base_config() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

fn base_data() -> PathBuf {
    if let Ok(override_dir) = std::env::var("CLIPRELAY_DATA_DIR") {
        return PathBuf::from(override_dir);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

fn base_cache() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

fn base_log() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Logs")
        .join(APP_NAME)
}

fn ensure(dir: &Path) -> PathBuf {
    let _ = std::fs::create_dir_all(dir);
    dir.to_path_buf()
}

pub fn config_dir() -> PathBuf {
    ensure(&base_config())
}

pub fn data_dir() -> PathBuf {
    ensure(&base_data())
}

pub fn cache_dir() -> PathBuf {
    ensure(&base_cache())
}

pub fn log_dir() -> PathBuf {
    ensure(&base_log())
}

pub fn default_export_dir() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let media = if cfg!(target_os = "macos") { "Movies" } else { "Videos" };
    let path = base.join(media).join(APP_NAME).join("Exports");
    ensure(&path)
}

pub fn database_path() -> PathBuf {
    data_dir().join("cliprelay.sqlite3")
}

pub fn thumbnail_dir() -> PathBuf {
    ensure(&cache_dir().join("thumbnails"))
}

pub fn preview_dir() -> PathBuf {
    ensure(&cache_dir().join("previews"))
}

pub fn timeline_dir() -> PathBuf {
    ensure(&cache_dir().join("timelines"))
}

/// Locate an ffmpeg/ffprobe binary, honoring `CLIPRELAY_FFMPEG_DIR`, a
/// `bin/` directory next to the executable, then `PATH`.
pub fn bundled_binary(name: &str) -> Option<PathBuf> {
    let executable = if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(custom) = std::env::var("CLIPRELAY_FFMPEG_DIR") {
        candidates.push(PathBuf::from(custom).join(&executable));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("bin").join(&executable));
        }
    }
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    // Fall back to PATH lookup.
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(&executable);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn ffmpeg_path() -> Option<PathBuf> {
    bundled_binary("ffmpeg")
}

pub fn ffprobe_path() -> Option<PathBuf> {
    bundled_binary("ffprobe")
}

/// True when `path` resolves inside `parent` (both resolved lexically).
pub fn is_within(path: impl AsRef<Path>, parent: impl AsRef<Path>) -> bool {
    let path = normalize(path.as_ref());
    let parent = normalize(parent.as_ref());
    path.starts_with(&parent)
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_override() {
        let original = std::env::var("CLIPRELAY_DATA_DIR").ok();
        std::env::set_var("CLIPRELAY_DATA_DIR", "/tmp/cliprelay-test-override");
        assert_eq!(
            base_data(),
            std::path::PathBuf::from("/tmp/cliprelay-test-override")
        );
        assert_eq!(
            database_path(),
            std::path::PathBuf::from("/tmp/cliprelay-test-override/cliprelay.sqlite3")
        );
        match original {
            Some(value) => std::env::set_var("CLIPRELAY_DATA_DIR", value),
            None => std::env::remove_var("CLIPRELAY_DATA_DIR"),
        }
    }

    #[test]
    fn within_checks() {
        assert!(is_within("/a/b/c.mp4", "/a/b"));
        assert!(is_within("/a/b/c.mp4", "/a/b/"));
        assert!(!is_within("/a/c/c.mp4", "/a/b"));
        assert!(!is_within("/a/bc/x.mp4", "/a/b"));
        assert!(!is_within("/a/b/../c/x.mp4", "/a/b"));
        assert!(is_within("/a/b/../b/c.mp4", "/a/b"));
    }
}
