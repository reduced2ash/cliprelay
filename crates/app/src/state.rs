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
    pub bot: String,          // "configured" | "not configured" | "@username"
    pub personal: String,     // "signed in" | "not signed in" | display
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
    SetRandomFolderEnabled(String, bool),
    ClearRandomFolders,
    SelectAllRandomFolders,
    LoadRandomFolderOptions,
    ResetShuffle,
    EnsureThumbnail(i64),
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
    LoadMoreFinished(bool, u64, bool, usize),
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
    MarkBotConfigured(String),
    MarkPersonalConfigured(String),
    SelectedMediaChanged(Option<MediaRow>),
    SelectionCheckingChanged(bool),
    TimelineLoadingChanged(bool),
    LibraryPage(LibraryPage),
    LibraryRefreshed,
    FoldersUpdated(Vec<FolderNode>),
    HistoryPage(HistoryPage),
    HistoryRefreshed,
    WorkspacesChanged(Vec<WorkspaceInfo>, usize),
    ThumbnailReady(i64, Option<PathBuf>),
    PreviewReady(i64, Option<PathBuf>),
    TimelineReady(i64, Option<PathBuf>),
    RandomFoldersChanged(Vec<RandomFolderOption>, String, usize, bool, bool),
    Toast(ToastKind, String),
    NavigationRequested(Page),
    RevealRequested { folder: String, media_index: i64, folder_index: i64 },
    NavigationRestored { folder: String, search: String, folder_index: i64 },
    SelectionNavigationChanged(bool, bool),
    NeighborPreload(i64, i64),
    LoadMoreFinished(bool, u64, bool, usize),
    TimelineFinished(i64),
    SelectionVerified(i64, MediaRow),
    DiagnosticsReady(Diagnostics),
    ShuffleDone,
    DraftRestoreRequested(PrepareDraft),
    CountsOnly,
    Tick,
    HoverCheck(i64),
    PickingChanged(bool),
    ClosedCountChanged(usize),
    DraftsDirty,
    ScanFinished(String, u64),
    RecordNavigationOrigin,
    SelectionVerifyFailed(i64),
    SearchResults(String, Vec<SearchResultItem>),
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
