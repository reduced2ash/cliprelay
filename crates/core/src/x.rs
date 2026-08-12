//! X handoff: composer intent URL, clipboard file copy, reveal in Finder.
//! Ported from `x_assist.py`. (The native drag source has no GPUI
//! equivalent, so the handoff is clipboard-copy + composer.)

use anyhow::Result;
use std::path::Path;

pub struct XAssistant;

impl XAssistant {
    /// `https://x.com/intent/tweet?text=<urlencoded caption>`.
    pub fn intent_url(caption: &str) -> String {
        format!("https://x.com/intent/tweet?text={}", urlencoding::encode(caption))
    }

    /// Put the local file URL on the clipboard so it can be pasted/dragged
    /// into the composer. On macOS this uses `osascript` so Finder/X see a
    /// file reference (the same paste-as-file behavior as the Qt original);
    /// elsewhere it falls back to a `file://` text URL.
    pub fn copy_file(path: &Path) -> Result<()> {
        let target = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());
        #[cfg(target_os = "macos")]
        {
            let script = format!(
                "set the clipboard to (POSIX file \"{}\" as alias)",
                target.to_string_lossy().replace('"', "\\\"")
            );
            let status = std::process::Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .status();
            if let Ok(status) = status {
                if status.success() {
                    return Ok(());
                }
            }
        }
        let url = format!("file://{}", target.to_string_lossy());
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(url)?;
        Ok(())
    }

    /// Copy the file to the clipboard and open the composer with the caption.
    pub fn prepare(path: &Path, caption: &str) -> Result<()> {
        Self::copy_file(path)?;
        let url = Self::intent_url(caption);
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Reveal a file in the OS file manager.
    pub fn reveal(path: &Path) -> Result<()> {
        let target = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("-R")
                .arg(&target)
                .spawn()
                .map(|_| ())
                .map_err(Into::into)
        }
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(format!("/select,{}", target.to_string_lossy()))
                .spawn()
                .map(|_| ())
                .map_err(Into::into)
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let parent = target.parent().unwrap_or(&target);
            std::process::Command::new("open")
                .arg(parent)
                .spawn()
                .map(|_| ())
                .map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_url_encodes_caption() {
        assert_eq!(
            XAssistant::intent_url("hello world & more"),
            "https://x.com/intent/tweet?text=hello%20world%20%26%20more"
        );
        assert_eq!(XAssistant::intent_url(""), "https://x.com/intent/tweet?text=");
    }
}
