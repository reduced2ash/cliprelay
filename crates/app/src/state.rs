//! Shared state types: commands to the background controller, events back
//! to the UI, and the UI-visible snapshots (workspaces, scan, publish...).

use cliprelay_core::db::{MediaRow, PostRow};
use cliprelay_core::telegram::DialogInfo;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToastKind::Info => "info",
            ToastKind::Success => "success",
            ToastKind::Warning => "warning",
            ToastKind::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Library,
    History,
    Settings,
}

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub id: String,
    pub title: String,
    pub root: String,
    pub folder: String,
    pub search: String,
    pub sort_mode: String,
    pub folder_sort_mode: String,
    pub selected_media_name: String,
    pub active: bool,
    pub index: usize,
    pub can_close: bool,
    pub has_back: bool,
    pub has_forward: bool,
    pub scanning: bool,
    pub scan_cancelling: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ScanState {
    pub active: bool,
    pub cancelling: bool,
    pub progress: f64, // -1.0 = indeterminate
    pub message: String,
    pub root_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct PublishState {
    pub active: bool,
    pub progress: f64,
    pub stage: String,
    pub error: String,
    pub output_path: String,
    pub output_size: String,
    pub output_encoder: String,
    pub hardware_accelerated: bool,
    pub post_id: i64,
}

#[derive(Debug, Clone, Default)]
pub struct TelegramState {
    pub bot: String,      // "configured" | "not configured" | "@username"
    pub personal: String, // "signed in" | "not signed in" | display
    pub message: String,
    pub password_required: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RandomFolderOption {
    pub folder: String,
    pub name: String,
    pub detail: String,
    pub video_count: i64,
    pub direct_video_count: i64,
    pub selection_state: i64, // 0 none, 1 partial, 2 all
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
    pub parent: String,
}

impl RandomFolderOption {
    /// All-folders mode is stored separately from explicit subtree tokens.
    /// Its rows are checked too, so toggling one excludes that subtree.
    pub fn effective_selection_state(&self, all_selected: bool) -> i64 {
        if all_selected {
            2
        } else {
            self.selection_state
        }
    }
}

/// Presentation derived only when sources, filtering, or expansion change.
/// Virtual rows refer back to the authoritative options by index.
#[derive(Default)]
pub struct RandomSourceList {
    pub rows: Vec<usize>,
    pub folder_count: usize,
    pub video_count: i64,
    pub has_branches: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryPage {
    pub rows: Vec<MediaRow>,
    pub has_more: bool,
    pub offset: usize,
    pub generation: u64,
}

#[derive(Debug, Clone, Default)]
pub struct HistoryPage {
    pub rows: Vec<PostRow>,
    pub has_more: bool,
    pub offset: usize,
    pub generation: u64,
}

#[derive(Debug, Clone, Default)]
pub struct FolderNode {
    pub folder: String,
    pub name: String,
    pub count: i64,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
    pub parent: String,
    pub latest_mtime: f64,
    pub latest_indexed: String,
}

/// The immutable item a transient context menu acts on. The menu owns this
/// target so a later library selection cannot redirect an already-open menu.
#[derive(Debug, Clone)]
pub enum ContextMenuTarget {
    Video(Box<MediaRow>),
    Folder {
        relative_path: String,
        path: PathBuf,
        name: String,
    },
}

impl ContextMenuTarget {
    pub fn path(&self) -> &std::path::Path {
        match self {
            Self::Video(row) => std::path::Path::new(&row.path),
            Self::Folder { path, .. } => path,
        }
    }

    pub fn has_path(&self) -> bool {
        !self.path().as_os_str().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    OpenDefault,
    RevealInFileManager,
    OpenContainingFolder,
    OpenInStudio,
    CopyPath,
    UseAsActiveSource,
}

#[derive(Debug, Clone, Copy)]
pub struct ContextMenuItem {
    pub action: ContextMenuAction,
    pub label: &'static str,
    pub glyph: &'static str,
    pub separator_before: bool,
}

pub fn context_menu_items(target: &ContextMenuTarget) -> Vec<ContextMenuItem> {
    match target {
        ContextMenuTarget::Video(_) => vec![
            ContextMenuItem {
                action: ContextMenuAction::OpenDefault,
                label: "Open in default player",
                glyph: "play",
                separator_before: false,
            },
            ContextMenuItem {
                action: ContextMenuAction::RevealInFileManager,
                label: "Show in file manager",
                glyph: "folder",
                separator_before: false,
            },
            ContextMenuItem {
                action: ContextMenuAction::OpenContainingFolder,
                label: "Open containing folder",
                glyph: "external",
                separator_before: false,
            },
            ContextMenuItem {
                action: ContextMenuAction::OpenInStudio,
                label: "Open in Studio",
                glyph: "maximize",
                separator_before: true,
            },
            ContextMenuItem {
                action: ContextMenuAction::CopyPath,
                label: "Copy path",
                glyph: "copy",
                separator_before: false,
            },
        ],
        ContextMenuTarget::Folder { .. } => vec![
            ContextMenuItem {
                action: ContextMenuAction::OpenContainingFolder,
                label: "Open folder",
                glyph: "external",
                separator_before: false,
            },
            ContextMenuItem {
                action: ContextMenuAction::CopyPath,
                label: "Copy folder path",
                glyph: "copy",
                separator_before: false,
            },
            ContextMenuItem {
                action: ContextMenuAction::UseAsActiveSource,
                label: "Use as active source",
                glyph: "folder",
                separator_before: true,
            },
        ],
    }
}

/// Wrap keyboard navigation through menu items without relying on render-local
/// indices. An empty menu intentionally has no valid selection.
pub fn next_context_menu_index(current: usize, count: usize, direction: i8) -> usize {
    if count == 0 {
        return 0;
    }
    if direction < 0 {
        (current + count - 1) % count
    } else {
        (current + 1) % count
    }
}

#[cfg(test)]
mod context_menu_tests {
    use super::*;

    fn video(path: &str) -> ContextMenuTarget {
        ContextMenuTarget::Video(Box::new(MediaRow {
            id: 42,
            name: "Clip.mp4".to_string(),
            path: path.to_string(),
            ..Default::default()
        }))
    }

    #[test]
    fn video_menu_keeps_its_immutable_target_and_action_set() {
        let target = video("/media/Clip.mp4");
        let actions: Vec<_> = context_menu_items(&target)
            .into_iter()
            .map(|item| item.action)
            .collect();
        assert_eq!(target.path(), PathBuf::from("/media/Clip.mp4"));
        assert_eq!(
            actions,
            vec![
                ContextMenuAction::OpenDefault,
                ContextMenuAction::RevealInFileManager,
                ContextMenuAction::OpenContainingFolder,
                ContextMenuAction::OpenInStudio,
                ContextMenuAction::CopyPath,
            ]
        );
    }

    #[test]
    fn folder_menu_has_only_safe_folder_actions() {
        let target = ContextMenuTarget::Folder {
            relative_path: "Exports".to_string(),
            path: PathBuf::from("/media/Exports"),
            name: "Exports".to_string(),
        };
        let actions: Vec<_> = context_menu_items(&target)
            .into_iter()
            .map(|item| item.action)
            .collect();
        assert_eq!(
            actions,
            vec![
                ContextMenuAction::OpenContainingFolder,
                ContextMenuAction::CopyPath,
                ContextMenuAction::UseAsActiveSource,
            ]
        );
    }

    #[test]
    fn unavailable_paths_are_not_actionable_and_navigation_wraps() {
        assert!(!video("").has_path());
        assert_eq!(next_context_menu_index(0, 0, 1), 0);
        assert_eq!(next_context_menu_index(0, 3, -1), 2);
        assert_eq!(next_context_menu_index(2, 3, 1), 0);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublishPayload {
    pub media_id: i64,
    pub telegram_enabled: bool,
    pub x_enabled: bool,
    pub telegram_caption: String,
    pub x_caption: String,
    pub telegram_mode: String,
    pub telegram_destination: String,
    pub cleanup_policy: String,
    pub preset: String,
    pub target_mb: f64,
    pub trim_start: f64,
    pub trim_end: f64,
    pub edits: Value,
}

#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    pub ffmpeg: String,
    pub ffprobe: String,
    pub gstreamer: String,
    pub export_encoder: String,
    pub database: String,
    pub secret_backend: String,
    pub library_inside_exports: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PrepareDraft {
    pub media_id: i64,
    pub trim_start: f64,
    pub trim_end: f64,
    pub same_caption: bool,
    pub studio_tab: i64,
    pub inspector_tab: i64,
    pub telegram_mode_index: i64,
    pub destination: String,
    pub caption: String,
    pub x_caption: String,
    pub compression_index: i64,
    pub target_size: String,
    pub cleanup_index: i64,
    #[serde(default)]
    pub guide_mode: i64,
    pub edit_scroll_y: f64,
    pub publish_scroll_y: f64,
    pub studio_inspector_width: f64,
    pub edits: Value,
}

/// Commands sent from the UI to the background controller.
#[derive(Debug, Clone)]
pub enum Command {
    ChooseLibrary(String),
    SetSetting(String, Value),
    ActivateWorkspace(usize),
    CreateWorkspace(String),
    CloseWorkspace(String),
    DuplicateWorkspace(String),
    CloseOtherWorkspaces(String),
    CloseWorkspacesToRight(String),
    ReopenClosedWorkspace,
    RenameWorkspace(String, String),
    RevealWorkspaceRoot(String),
    SetSearch(String),
    SetFolder(String),
    SetSortMode(String),
    SetFolderSortMode(String),
    LoadMoreLibrary,
    LoadMoreHistory,
    RefreshAll,
    ScanLibrary,
    CancelScan,
    ScanBatchReady(String, u64),
    ScanFinished(String, u64),
    RecordNavigationOrigin,
    SelectionVerifyFailed(i64),
    PickingFinished,
    SelectMedia(i64),
    ClearSelection,
    NavigateSelection(i32),
    NavigateBack,
    NavigateForward,
    RevealSelectedInLibrary,
    RevealMedia(i64),
    ViewHistoryPost(i64),
    PickRandom,
    RandomPicked(i64),
    SetRandomFolderEnabled(String, bool),
    ClearRandomFolders,
    SelectAllRandomFolders,
    LoadRandomFolderOptions,
    ResetShuffle,
    EnsureThumbnail(i64),
    ThumbnailFinished(i64),
    SelectionCheckFinished,
    EnsurePreview(i64),
    PreviewFinished(i64),
    EnsureTimeline(i64),
    Publish(PublishPayload),
    CancelPublish,
    RetryTelegram(i64),
    MarkXPosted(i64, String),
    PrepareXAgain(i64),
    TrashExport(i64),
    SetHistorySearch(String),
    ValidateBotToken(String),
    ValidateBotDestination(String),
    DisconnectBot,
    BeginPersonalLogin(i32, String, String),
    CompletePersonalLogin(String, String),
    LoadTelegramDialogs,
    MarkBotConfigured(String),
    MarkPersonalConfigured(String),
    SignOutPersonal,
    SaveWorkspaceDraft(String, Value),
    FlushDrafts,
    LoadMoreFinished(bool, u64, bool, usize, usize),
    TimelineFinished(i64),
    SelectionVerified(i64, MediaRow),
    SearchSuggestions { query: String, scope: String },
    Diagnostics,
    Shutdown,
}

/// Events emitted by the background controller to the UI.
#[derive(Debug, Clone)]
pub enum Event {
    SettingsChanged(std::collections::HashMap<String, Value>),
    CountsChanged(i64, i64, i64),
    ScanStateChanged(ScanState),
    PublishStateChanged(PublishState),
    TelegramStateChanged(TelegramState),
    TelegramDialogsChanged(Vec<DialogInfo>),
    HistorySearchCommitted(u64, String),
    CommandSearchCommitted {
        generation: u64,
        query: String,
        scope: String,
        library_search: Option<String>,
    },
    MarkBotConfigured(String),
    MarkPersonalConfigured(String),
    SelectedMediaChanged(Option<MediaRow>),
    SelectionCheckingChanged(i64, bool),
    TimelineLoadingChanged(i64, bool),
    LibraryPage(LibraryPage),
    LibraryRefreshed,
    FoldersUpdated(Vec<FolderNode>),
    HistoryPage(HistoryPage),
    HistoryRefreshed,
    WorkspacesChanged(Vec<WorkspaceInfo>, usize),
    ThumbnailReady(i64, Option<PathBuf>),
    PreviewReady(i64, Option<PathBuf>),
    PreviewDeferred(i64),
    TimelineReady(i64, Option<PathBuf>),
    RandomFoldersChanged(Vec<RandomFolderOption>, String, usize, bool, bool),
    Toast(ToastKind, String),
    NavigationRequested(Page),
    RevealRequested {
        folder: String,
        media_index: i64,
        folder_index: i64,
    },
    NavigationRestored {
        folder: String,
        search: String,
        folder_index: i64,
    },
    SelectionNavigationChanged(bool, bool),
    NeighborPreload(i64, i64),
    LoadMoreFinished(bool, u64, bool, usize, usize),
    TimelineFinished(i64),
    SelectionVerified(i64, MediaRow),
    DiagnosticsReady(Diagnostics),
    ShuffleDone,
    DraftRestoreRequested(PrepareDraft),
    CountsOnly,
    Tick,
    HoverCheck(i64, u64),
    PickingChanged(bool),
    RandomPicked(i64),
    ClosedCountChanged(usize),
    DraftsDirty,
    ScanFinished(String, u64),
    RecordNavigationOrigin,
    SelectionVerifyFailed(i64),
    ScanBatchReady(String, u64),
    SearchResults(String, String, Vec<SearchResultItem>),
}

#[derive(Debug, Clone, Default)]
pub struct SearchResultItem {
    pub kind: String, // "media" | "folder"
    pub media_id: i64,
    pub folder_path: String,
    pub title: String,
    pub detail: String,
    pub count: i64,
}
