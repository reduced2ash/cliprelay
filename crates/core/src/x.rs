//! X handoff: composer intent URL, clipboard file copy, reveal in Finder.
//! Ported from `x_assist.py`. (The native drag source has no GPUI
//! equivalent, so the handoff is clipboard-copy + composer.)

use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerAction {
    OpenDefault,
    Reveal,
    OpenFolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerCommand {
    pub program: OsString,
    pub args: Vec<OsString>,
}

/// Build the platform command without spawning it. Keeping this pure makes
/// path quoting and platform routing testable without launching applications.
pub fn file_manager_command(action: FileManagerAction, path: &Path) -> FileManagerCommand {
    let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    #[cfg(target_os = "macos")]
    {
        let args = match action {
            FileManagerAction::OpenDefault | FileManagerAction::OpenFolder => {
                vec![target.into_os_string()]
            }
            FileManagerAction::Reveal => vec![OsString::from("-R"), target.into_os_string()],
        };
        FileManagerCommand {
            program: OsString::from("open"),
            args,
        }
    }
    #[cfg(target_os = "windows")]
    {
        let args = match action {
            FileManagerAction::Reveal => {
                vec![OsString::from(format!("/select,{}", target.display()))]
            }
            FileManagerAction::OpenDefault => vec![
                OsString::from("/C"),
                OsString::from("start"),
                OsString::new(),
                target.into_os_string(),
            ],
            FileManagerAction::OpenFolder => vec![target.into_os_string()],
        };
        let program = if action == FileManagerAction::OpenDefault {
            "cmd"
        } else {
            "explorer"
        };
        FileManagerCommand {
            program: OsString::from(program),
            args,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let open_target: PathBuf = match action {
            FileManagerAction::Reveal => target.parent().unwrap_or(&target).to_path_buf(),
            FileManagerAction::OpenDefault | FileManagerAction::OpenFolder => target,
        };
        FileManagerCommand {
            program: OsString::from("xdg-open"),
            args: vec![open_target.into_os_string()],
        }
    }
}

pub struct XAssistant;

impl XAssistant {
    pub fn launch_file_manager(action: FileManagerAction, path: &Path) -> Result<()> {
        let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let valid = match action {
            FileManagerAction::OpenFolder => target.is_dir(),
            FileManagerAction::OpenDefault | FileManagerAction::Reveal => target.is_file(),
        };
        if !valid {
            anyhow::bail!("That item is no longer available.");
        }
        let command = file_manager_command(action, &target);
        std::process::Command::new(command.program)
            .args(command.args)
            .spawn()
            .map(|_| ())
            .map_err(Into::into)
    }

    /// `https://x.com/intent/tweet?text=<urlencoded caption>`.
    pub fn intent_url(caption: &str) -> String {
        format!(
            "https://x.com/intent/tweet?text={}",
            urlencoding::encode(caption)
        )
    }

    /// Put the local file URL on the clipboard so it can be pasted/dragged
    /// into the composer. On macOS this uses `osascript` so Finder/X see a
    /// file reference (the same paste-as-file behavior as the Qt original);
    /// elsewhere it falls back to a `file://` text URL.
    pub fn copy_file(path: &Path) -> Result<()> {
        let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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
        Self::launch_file_manager(FileManagerAction::Reveal, path)
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
        assert_eq!(
            XAssistant::intent_url(""),
            "https://x.com/intent/tweet?text="
        );
    }

    #[test]
    fn platform_file_manager_commands_keep_unicode_paths_as_one_argument() {
        let path = Path::new("/tmp/with spaces/épisode.mp4");
        let command = file_manager_command(FileManagerAction::Reveal, path);
        #[cfg(target_os = "macos")]
        assert_eq!(command.program, "open");
        #[cfg(target_os = "windows")]
        assert_eq!(command.program, "explorer");
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        assert_eq!(command.program, "xdg-open");
        assert!(!command.args.is_empty());
        assert!(command
            .args
            .iter()
            .any(|arg| arg.to_string_lossy().contains("spaces")
                || arg.to_string_lossy().contains("tmp")));
    }
}
