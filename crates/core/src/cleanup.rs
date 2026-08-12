//! Safe cleanup of generated exports, ported from `cleanup.py`.

use crate::paths::is_within;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum CleanupError {
    #[error("Source videos are protected and cannot be cleaned up by ClipRelay.")]
    SourceProtected,
    #[error("Only files inside the configured exports folder can be moved to Trash.")]
    OutsideExportDir,
    #[error("The generated file is no longer available.")]
    Missing,
}

/// Move a generated export to the OS Trash after verifying:
/// 1. the record says it is generated (sources are protected),
/// 2. the resolved path stays inside the configured export directory,
/// 3. the file still exists.
pub fn move_generated_to_trash(
    path: impl AsRef<Path>,
    export_dir: impl AsRef<Path>,
    is_generated: bool,
) -> Result<()> {
    let target = path.as_ref();
    if !is_generated {
        return Err(CleanupError::SourceProtected.into());
    }
    // Resolve symlinks before the containment check (mirrors the
    // original's .resolve()). Missing files resolve through their parent
    // so the comparison stays symmetric (/var -> /private/var on macOS).
    let resolve = |path: &std::path::Path| -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| {
            match (path.parent(), path.file_name()) {
                (Some(parent), Some(name)) => std::fs::canonicalize(parent)
                    .map(|p| p.join(name))
                    .unwrap_or_else(|_| path.to_path_buf()),
                _ => path.to_path_buf(),
            }
        })
    };
    let target = resolve(target);
    let export = resolve(export_dir.as_ref());
    if !is_within(&target, &export) {
        return Err(CleanupError::OutsideExportDir.into());
    }
    if !target.is_file() {
        return Err(CleanupError::Missing.into());
    }
    let resolved: PathBuf = target.to_path_buf();
    trash::delete(&resolved)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn protects_sources() {
        let dir = export_dir();
        let path = dir.path().join("source.mp4");
        std::fs::write(&path, b"x").unwrap();
        let err = move_generated_to_trash(&path, dir.path(), false).unwrap_err();
        assert!(err
            .to_string()
            .contains("Source videos are protected"));
    }

    #[test]
    fn rejects_outside_paths() {
        let dir = export_dir();
        let outside = std::env::temp_dir().join("cliprelay-outside-test.mp4");
        std::fs::write(&outside, b"x").unwrap();
        let err = move_generated_to_trash(&outside, dir.path(), true).unwrap_err();
        let _ = std::fs::remove_file(&outside);
        assert!(err
            .to_string()
            .contains("configured exports folder"));
    }

    #[test]
    fn rejects_missing_files() {
        let dir = export_dir();
        let path = dir.path().join("gone.mp4");
        let err = move_generated_to_trash(&path, dir.path(), true).unwrap_err();
        assert!(err.to_string().contains("no longer available"));
    }
}
