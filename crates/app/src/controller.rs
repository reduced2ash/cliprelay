//! Background controller: the app's orchestration layer (ported from
//! `controller.py`). Runs on its own thread, owns the database, media
//! pipeline, Telegram services, workspaces, and delivery state machine,
//! and streams events to the UI.

use crate::state::*;
use anyhow::Result;
use cliprelay_core::cleanup::move_generated_to_trash;
use cliprelay_core::db::{Database, MediaRow, PostUpdate, PostValues, RandomFolder};
use cliprelay_core::media::{
    normalize_edit_spec, EncoderMode, MediaIndexer, MediaProcessor, ScanResult,
};
use cliprelay_core::paths::{database_path, ffmpeg_path, ffprobe_path, is_within};
use cliprelay_core::secrets::SecretStore;
use cliprelay_core::settings::*;
use cliprelay_core::telegram::{
    DialogInfo, PersonalTelegram, ProgressCb, TelegramBotService, TelegramDelivery, TelegramError,
};
use cliprelay_core::utils::{format_bytes, format_duration};
use cliprelay_core::x::XAssistant;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const PAGE_SIZE_LIBRARY: i64 = 240;
pub const PAGE_SIZE_HISTORY: i64 = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct NavState {
    pub folder: String,
    pub search: String,
    pub media_id: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Workspace {
    pub id: String,
    pub title: String,
    pub custom_title: bool,
    pub root: String,
    pub folder: String,
    pub search: String,
    pub sort_mode: String,
    pub folder_sort_mode: String,
    pub selected_media_id: i64,
    pub selected_media_name: String,
    pub nav_back: Vec<NavState>,
    pub nav_forward: Vec<NavState>,
    pub random_mode: String,
    pub random_folders: Vec<String>,
    pub draft: Value,
}

impl Workspace {
    pub fn new(id: String, root: String, title: Option<String>) -> Self {
        let title = title.unwrap_or_else(|| workspace_title(&root));
        Self {
            id,
            title,
            custom_title: false,
            root,
            folder: String::new(),
            search: String::new(),
            sort_mode: "newest".into(),
            folder_sort_mode: "name_asc".into(),
            selected_media_id: 0,
            selected_media_name: String::new(),
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
            random_mode: "all".into(),
            random_folders: Vec::new(),
            draft: Value::Null,
        }
    }
}

fn workspace_title(root: &str) -> String {
    if root.is_empty() {
        return "New workspace".into();
    }
    let path = Path::new(root);
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| root.to_string())
}

#[derive(Clone)]
struct ScanJob {
    cancel: Arc<AtomicBool>,
    workspace_id: String,
    root: String,
    include_index: bool,
    generation: u64,
}

fn scan_job_matches(scan: Option<&ScanJob>, workspace_id: &str, generation: u64) -> bool {
    scan.is_some_and(|job| job.workspace_id == workspace_id && job.generation == generation)
}

fn scan_generation_matches(current: &AtomicU64, generation: u64) -> bool {
    current.load(Ordering::Relaxed) == generation
}

#[allow(dead_code)]
pub struct Controller {
    db: Arc<Database>,
    settings: Settings,
    secrets: SecretStore,
    indexer: Arc<MediaIndexer>,
    processor: Arc<MediaProcessor>,
    export_dir: PathBuf,
    personal: Arc<Mutex<PersonalTelegram>>,
    events: flume::Sender<Event>,
    thumb_tx: flume::Sender<i64>,
    workspaces: Vec<Workspace>,
    active_id: String,
    closed: Vec<Value>,
    scan: Option<ScanJob>,
    scan_state: ScanState,
    publish_state: PublishState,
    telegram_state: TelegramState,
    dialogs: Vec<DialogInfo>,
    counts: (i64, i64, i64),
    thumb_queue: HashSet<i64>,
    selected: Option<MediaRow>,
    checking: bool,
    timeline_loading: bool,
    timeline_pending: Option<(i64, u64)>,
    timeline_generation: u64,
    nav_available: (bool, bool),
    random_folders_cache: Vec<RandomFolderOption>,
    random_folders_selected: usize,
    all_random_folders_selected: bool,
    has_random_folder_selection: bool,
    random_folders_loading: bool,
    draft_save_pending: bool,
    nav_restoring: bool,
    random_picking: bool,
    random_retry_media_id: Option<i64>,
    preview_pending: Arc<Mutex<HashSet<i64>>>,
    preview_slots: Arc<std::sync::atomic::AtomicUsize>,
    library_generation: u64,
    library_offset: usize,
    library_has_more: bool,
    library_loading_more: bool,
    history_generation: u64,
    history_offset: usize,
    history_has_more: bool,
    history_loading_more: bool,
    history_search: String,
    next_scan_generation: Arc<AtomicU64>,
    shutting_down: bool,
    bot_configured: bool,
    personal_configured: bool,
}

/// Start the controller thread; returns the command channel.
pub fn spawn_controller(
    db_path: Option<PathBuf>,
    events: flume::Sender<Event>,
) -> flume::Sender<Command> {
    let (tx, rx) = flume::unbounded::<Command>();
    std::thread::Builder::new()
        .name("cliprelay-controller".into())
        .spawn(move || {
            let path = db_path.unwrap_or_else(database_path);
            let db = match Database::open(&path) {
                Ok(db) => db,
                Err(e) => {
                    let _ = events.send(Event::Toast(
                        ToastKind::Error,
                        format!("The library database could not be updated. ({e})"),
                    ));
                    return;
                }
            };
            let db = Arc::new(db);
            let settings = Settings::new(Arc::clone(&db));
            let export_dir = settings
                .get_string(EXPORT_DIR)
                .map(PathBuf::from)
                .unwrap_or_else(|_| cliprelay_core::paths::default_export_dir());
            let secrets = SecretStore::new(None);
            let indexer = Arc::new(MediaIndexer {
                database: Arc::clone(&db),
                export_dir: export_dir.clone(),
            });
            let processor = Arc::new(MediaProcessor::new(
                Arc::clone(&db),
                export_dir.clone(),
                EncoderMode::Auto,
            ));
            let (thumb_tx, thumb_rx) = flume::unbounded::<i64>();
            let mut controller = Controller {
                db,
                settings,
                secrets,
                indexer,
                processor,
                export_dir,
                personal: Arc::new(Mutex::new(PersonalTelegram::new(SecretStore::new(None)))),
                events: events.clone(),
                thumb_tx,
                workspaces: Vec::new(),
                active_id: String::new(),
                closed: Vec::new(),
                scan: None,
                scan_state: ScanState::default(),
                publish_state: PublishState::default(),
                telegram_state: TelegramState::default(),
                dialogs: Vec::new(),
                counts: (0, 0, 0),
                thumb_queue: HashSet::new(),
                selected: None,
                checking: false,
                timeline_loading: false,
                timeline_pending: None,
                timeline_generation: 0,
                nav_available: (false, false),
                random_folders_cache: Vec::new(),
                random_folders_selected: 0,
                all_random_folders_selected: true,
                has_random_folder_selection: false,
                random_folders_loading: false,
                draft_save_pending: false,
                nav_restoring: false,
                random_picking: false,
                random_retry_media_id: None,
                preview_pending: Arc::new(Mutex::new(HashSet::new())),
                preview_slots: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                library_generation: 0,
                library_offset: 0,
                library_has_more: false,
                library_loading_more: false,
                history_generation: 0,
                history_offset: 0,
                history_has_more: false,
                history_loading_more: false,
                history_search: String::new(),
                next_scan_generation: Arc::new(AtomicU64::new(0)),
                shutting_down: false,
                bot_configured: false,
                personal_configured: false,
            };
            controller.init();
            // Thumbnail worker thread.
            let thumb_indexer = Arc::clone(&controller.indexer);
            let thumb_events = events.clone();
            std::thread::Builder::new()
                .name("cliprelay-thumbs".into())
                .spawn(move || {
                    let mut pending = Vec::new();
                    loop {
                        if pending.is_empty() {
                            let Ok(media_id) = thumb_rx.recv() else {
                                break;
                            };
                            pending.push(media_id);
                        }
                        // Visible tiles enqueue as the viewport changes. Drain
                        // new arrivals and take the newest first so work from
                        // a viewport the user already left cannot starve the
                        // current one. Older requests remain in the backlog.
                        pending.extend(thumb_rx.try_iter());
                        let Some(media_id) = pending.pop() else {
                            continue;
                        };
                        let result = thumb_indexer.ensure_thumbnail(media_id, None);
                        let _ = thumb_events.send(Event::ThumbnailReady(media_id, result));
                    }
                })
                .ok();
            while let Ok(command) = rx.recv() {
                if controller.shutting_down {
                    break;
                }
                controller.handle(command);
            }
            controller.shutdown_now();
        })
        .ok();
    tx
}

impl Controller {
    fn init(&mut self) {
        self.sync_configured_flags();
        self.settings_changed();
        self.restore_workspaces();
        self.refresh_counts();
        self.refresh_library();
        self.refresh_folders();
        self.refresh_history();
        // Startup refresh (mirrors the 4s startup timer). When auto_index is
        // on, this becomes the full verify + thumbnail scan.
        let auto_index = self.settings.get_bool(AUTO_INDEX).unwrap_or(false);
        self.request_scan("startup", auto_index);
    }

    fn sync_configured_flags(&mut self) {
        let bot_stored = self
            .settings
            .get_bool(TELEGRAM_BOT_CONFIGURED)
            .unwrap_or(false);
        let mode = self
            .settings
            .get_string(TELEGRAM_MODE)
            .unwrap_or_else(|_| "bot".into());
        let destination = self
            .settings
            .get_string(TELEGRAM_DESTINATION)
            .unwrap_or_default();
        self.bot_configured = bot_stored || (mode == "bot" && !destination.trim().is_empty());
        let personal_stored = self
            .settings
            .get_bool(TELEGRAM_PERSONAL_CONFIGURED)
            .unwrap_or(false);
        let api_id = self
            .settings
            .get_string(TELEGRAM_API_ID)
            .unwrap_or_default();
        let phone = self.settings.get_string(TELEGRAM_PHONE).unwrap_or_default();
        self.personal_configured = personal_stored
            || (mode == "personal" && !api_id.trim().is_empty() && !phone.trim().is_empty());
        self.telegram_state.bot = if self.bot_configured {
            "configured".into()
        } else {
            "not configured".into()
        };
        self.telegram_state.personal = if self.personal_configured {
            "signed in".into()
        } else {
            "not signed in".into()
        };
    }

    fn emit(&self, event: Event) {
        let _ = self.events.send(event);
    }

    fn toast(&self, kind: ToastKind, message: impl Into<String>) {
        self.emit(Event::Toast(kind, message.into()));
    }

    fn settings_changed(&mut self) {
        let settings = self.settings.as_map().unwrap_or_default();
        self.emit(Event::SettingsChanged(settings));
    }

    // ---- workspaces -----------------------------------------------------

    fn restore_workspaces(&mut self) {
        let tabs: Vec<Value> = self
            .settings
            .get(WORKSPACE_TABS)
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();
        let library_root = self.settings.get_string(LIBRARY_ROOT).unwrap_or_default();
        let mut workspaces: Vec<Workspace> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for tab in tabs {
            if let Some(workspace) = self.normalize_workspace(tab, &mut seen) {
                workspaces.push(workspace);
            }
        }
        if workspaces.is_empty() {
            // No saved tabs: seed the initial workspace from the legacy
            // top-level settings (mirrors the original).
            let mut initial = Workspace::new(new_id(), library_root.clone(), None);
            let sort = self.settings.get_string(SORT_MODE).unwrap_or_default();
            if matches!(
                sort.as_str(),
                "newest" | "oldest" | "name" | "duration" | "size"
            ) {
                initial.sort_mode = sort;
            }
            let folder_sort = self
                .settings
                .get_string(FOLDER_SORT_MODE)
                .unwrap_or_default();
            if !folder_sort.is_empty() {
                initial.folder_sort_mode = folder_sort;
            }
            initial.random_mode = self
                .settings
                .get_string(RANDOM_FOLDER_MODE)
                .unwrap_or_default();
            if initial.random_mode != "selected" {
                initial.random_mode = "all".into();
            }
            if let Ok(folders) = self.settings.get(RANDOM_FOLDERS) {
                if let Some(items) = folders.as_array() {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            if !initial.random_folders.contains(&s.to_string()) {
                                initial.random_folders.push(s.to_string());
                            }
                        }
                    }
                }
            }
            workspaces.push(initial);
        }
        let saved_active = self
            .settings
            .get_string(ACTIVE_WORKSPACE_ID)
            .unwrap_or_default();
        let activate_id = if workspaces.iter().any(|w| w.id == saved_active) {
            saved_active
        } else {
            // Fall back to the first restored tab (mirrors the original).
            workspaces[0].id.clone()
        };
        self.closed = self
            .settings
            .get(CLOSED_WORKSPACE_TABS)
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();
        self.emit(Event::ClosedCountChanged(self.closed.len()));
        self.workspaces = workspaces;
        // Clear the active id so the activation guard below does not skip
        // the initial restore (it would otherwise match immediately).
        self.active_id = String::new();
        self.activate_workspace_by_id(&activate_id, false);
        self.persist_workspaces();
    }
    fn normalize_workspace(&self, value: Value, seen: &mut HashSet<String>) -> Option<Workspace> {
        let obj = value.as_object()?;
        let mut id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() || !seen.insert(id.clone()) {
            id = new_id();
        }
        let root = obj
            .get("root")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| workspace_title(&root));
        let mut workspace = Workspace::new(id, root, Some(title));
        workspace.custom_title = obj
            .get("customTitle")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        workspace.folder = obj
            .get("folder")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        workspace.search = obj
            .get("search")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sort = obj
            .get("sortMode")
            .and_then(|v| v.as_str())
            .unwrap_or("newest");
        workspace.sort_mode = if matches!(sort, "newest" | "oldest" | "name" | "duration" | "size")
        {
            sort.to_string()
        } else {
            "newest".into()
        };
        workspace.folder_sort_mode = obj
            .get("folderSortMode")
            .and_then(|v| v.as_str())
            .unwrap_or("name_asc")
            .to_string();
        workspace.random_mode = match obj.get("randomFolderMode").and_then(|v| v.as_str()) {
            Some("selected") => "selected",
            _ => "all",
        }
        .to_string();
        workspace.random_folders = obj
            .get("randomFolders")
            .and_then(|v| v.as_array())
            .map(|items| {
                let mut out = Vec::new();
                for item in items {
                    if let Some(s) = item.as_str() {
                        if !out.contains(&s.to_string()) {
                            out.push(s.to_string());
                        }
                    }
                }
                out
            })
            .unwrap_or_default();
        workspace.selected_media_id = obj
            .get("selectedMediaId")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        workspace.selected_media_name = obj
            .get("selectedMediaName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        workspace.nav_back = nav_from_value(obj.get("navigationBack"));
        workspace.nav_forward = nav_from_value(obj.get("navigationForward"));
        workspace.draft = obj.get("draft").cloned().unwrap_or(Value::Null);
        Some(workspace)
    }

    fn workspace_snapshot(&self, workspace: &Workspace) -> Value {
        json!({
            "id": workspace.id,
            "title": workspace.title,
            "customTitle": workspace.custom_title,
            "root": workspace.root,
            "folder": workspace.folder,
            "search": workspace.search,
            "sortMode": workspace.sort_mode,
            "folderSortMode": workspace.folder_sort_mode,
            "selectedMediaId": workspace.selected_media_id,
            "selectedMediaName": workspace.selected_media_name,
            "navigationBack": nav_to_value(&workspace.nav_back),
            "navigationForward": nav_to_value(&workspace.nav_forward),
            "randomFolderMode": workspace.random_mode,
            "randomFolders": workspace.random_folders,
            "draft": workspace.draft,
        })
    }

    fn persist_workspaces(&mut self) {
        let snapshots: Vec<Value> = self
            .workspaces
            .iter()
            .map(|w| self.workspace_snapshot(w))
            .collect();
        let _ = self.settings.set(WORKSPACE_TABS, Value::Array(snapshots));
        let _ = self
            .settings
            .set(ACTIVE_WORKSPACE_ID, json!(self.active_id));
        // Persist in append order so the most recently closed reopens first
        // after a restart (pop from the end).
        let closed = self.closed.iter().take(10).cloned().collect::<Vec<_>>();
        let _ = self
            .settings
            .set(CLOSED_WORKSPACE_TABS, Value::Array(closed));
        self.emit_workspaces();
    }

    fn emit_workspaces(&mut self) {
        let mut infos = Vec::new();
        for (index, workspace) in self.workspaces.iter().enumerate() {
            let scanning = self
                .scan
                .as_ref()
                .is_some_and(|s| s.workspace_id == workspace.id);
            infos.push(WorkspaceInfo {
                id: workspace.id.clone(),
                title: workspace.title.clone(),
                root: workspace.root.clone(),
                folder: workspace.folder.clone(),
                search: workspace.search.clone(),
                sort_mode: workspace.sort_mode.clone(),
                folder_sort_mode: workspace.folder_sort_mode.clone(),
                selected_media_name: workspace.selected_media_name.clone(),
                active: workspace.id == self.active_id,
                index,
                can_close: self.workspaces.len() > 1,
                has_back: !workspace.nav_back.is_empty(),
                has_forward: !workspace.nav_forward.is_empty(),
                scanning,
                scan_cancelling: scanning
                    && self
                        .scan
                        .as_ref()
                        .is_some_and(|s| s.cancel.load(Ordering::Relaxed)),
            });
        }
        self.emit(Event::WorkspacesChanged(infos, self.workspaces.len()));
    }

    fn active(&self) -> &Workspace {
        self.workspaces
            .iter()
            .find(|w| w.id == self.active_id)
            .unwrap_or(&self.workspaces[0])
    }

    fn active_mut(&mut self) -> &mut Workspace {
        let index = self
            .workspaces
            .iter()
            .position(|w| w.id == self.active_id)
            .unwrap_or(0);
        &mut self.workspaces[index]
    }

    fn activate_workspace_by_id(&mut self, id: &str, scan_new_root: bool) {
        if id == self.active_id && !self.workspaces.is_empty() {
            return;
        }
        if !self.workspaces.iter().any(|w| w.id == id) {
            return;
        }
        self.active_id = id.to_string();
        let (
            root,
            folder,
            search,
            sort_mode,
            folder_sort_mode,
            random_mode,
            random_folders,
            media_id,
        ) = {
            let workspace = self.active();
            (
                workspace.root.clone(),
                workspace.folder.clone(),
                workspace.search.clone(),
                workspace.sort_mode.clone(),
                workspace.folder_sort_mode.clone(),
                workspace.random_mode.clone(),
                workspace.random_folders.clone(),
                workspace.selected_media_id,
            )
        };
        let _ = self.settings.set(LIBRARY_ROOT, json!(root));
        let _ = self.settings.set(SORT_MODE, json!(sort_mode));
        let _ = self.settings.set(FOLDER_SORT_MODE, json!(folder_sort_mode));
        let _ = self.settings.set(RANDOM_FOLDER_MODE, json!(random_mode));
        let _ = self.settings.set(RANDOM_FOLDERS, json!(random_folders));
        self.activate_root(&root);
        self.thumb_queue.clear();
        self.library_generation += 1;
        self.library_offset = 0;
        self.library_has_more = false;
        self.refresh_library();
        self.refresh_folders();
        self.select_media_row(media_id);
        // Restore the workspace's Prepare draft once its selection is in place.
        let draft = self.active().draft.clone();
        if !draft.is_null() {
            if let Ok(draft) = serde_json::from_value::<PrepareDraft>(draft) {
                self.emit(Event::DraftRestoreRequested(draft));
            }
        }
        if scan_new_root && !root.is_empty() {
            self.request_scan("new_root", true);
        }
        self.persist_workspaces();
        self.emit(Event::NavigationRestored {
            folder,
            search,
            folder_index: -1,
        });
    }

    fn activate_root(&mut self, root: &str) {
        let result = if root.is_empty() {
            self.db.activate_root(None)
        } else {
            self.db.activate_root(Some(root))
        };
        if let Err(e) = result {
            self.toast(
                ToastKind::Error,
                format!("The library database could not be updated. ({e})"),
            );
        }
    }

    fn handle_create_workspace(&mut self, root: String) {
        if root.is_empty() {
            return;
        }
        let path = PathBuf::from(&root);
        if !path.is_dir() {
            self.toast(ToastKind::Error, "Choose an existing folder.");
            return;
        }
        let workspace = Workspace::new(new_id(), root.clone(), None);
        self.workspaces.push(workspace);
        let id = self.workspaces.last().unwrap().id.clone();
        self.active_id = id;
        let _ = self.settings.set(LIBRARY_ROOT, json!(root));
        self.activate_root(&root);
        self.library_generation += 1;
        self.library_offset = 0;
        self.refresh_library();
        self.refresh_folders();
        self.request_scan("new_root", true);
        self.persist_workspaces();
    }

    fn handle_close_workspace(&mut self, id: &str) {
        let Some(index) = self.workspaces.iter().position(|w| w.id == id) else {
            return;
        };
        let removed = self.workspaces.remove(index);
        self.closed.push(self.workspace_snapshot(&removed));
        self.emit(Event::ClosedCountChanged(self.closed.len()));
        if self.workspaces.is_empty() {
            self.workspaces
                .push(Workspace::new(new_id(), String::new(), None));
        }
        if self.active_id == id {
            let new_index = index.min(self.workspaces.len() - 1);
            let new_id = self.workspaces[new_index].id.clone();
            self.active_id = String::new();
            self.activate_workspace_by_id(&new_id, false);
        }
        if self.scan.as_ref().is_some_and(|s| s.workspace_id == id) {
            if let Some(scan) = self.scan.take() {
                scan.cancel.store(true, Ordering::Relaxed);
                self.scan_state.active = false;
                self.emit(Event::ScanStateChanged(self.scan_state.clone()));
            }
        }
        self.persist_workspaces();
    }

    fn handle_duplicate_workspace(&mut self, id: &str) {
        let Some(index) = self.workspaces.iter().position(|w| w.id == id) else {
            return;
        };
        let source = self.workspaces[index].clone();
        let mut copy = source.clone();
        copy.id = new_id();
        copy.title = format!("{} copy", source.title);
        copy.custom_title = true;
        copy.nav_back.clear();
        copy.nav_forward.clear();
        self.workspaces.insert(index + 1, copy);
        self.active_id = self.workspaces[index + 1].id.clone();
        let (root, folder, search) = {
            let workspace = self.active();
            (
                workspace.root.clone(),
                workspace.folder.clone(),
                workspace.search.clone(),
            )
        };
        self.activate_root(&root);
        self.library_generation += 1;
        self.library_offset = 0;
        self.refresh_library();
        self.refresh_folders();
        self.emit(Event::NavigationRestored {
            folder,
            search,
            folder_index: -1,
        });
        self.persist_workspaces();
    }

    fn handle_reopen_closed(&mut self) {
        let Some(snapshot) = self.closed.pop() else {
            return;
        };
        self.emit(Event::ClosedCountChanged(self.closed.len()));
        let mut seen = HashSet::new();
        for workspace in &self.workspaces {
            seen.insert(workspace.id.clone());
        }
        if let Some(workspace) = self.normalize_workspace(snapshot, &mut seen) {
            let index = self
                .workspaces
                .iter()
                .position(|w| w.id == self.active_id)
                .unwrap_or(0);
            self.workspaces.insert(index + 1, workspace.clone());
            self.active_id = workspace.id.clone();
            let (root, folder, search) = (
                workspace.root.clone(),
                workspace.folder.clone(),
                workspace.search.clone(),
            );
            self.activate_root(&root);
            self.library_generation += 1;
            self.library_offset = 0;
            self.refresh_library();
            self.refresh_folders();
            self.emit(Event::NavigationRestored {
                folder,
                search,
                folder_index: -1,
            });
            self.persist_workspaces();
        }
    }

    fn handle_rename_workspace(&mut self, id: &str, title: String) {
        let Some(workspace) = self.workspaces.iter_mut().find(|w| w.id == id) else {
            return;
        };
        let collapsed: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
        let collapsed = collapsed.chars().take(80).collect::<String>();
        if collapsed.is_empty() {
            workspace.title = workspace_title(&workspace.root);
            workspace.custom_title = false;
        } else {
            workspace.title = collapsed;
            workspace.custom_title = true;
        }
        self.persist_workspaces();
    }

    // ---- settings -------------------------------------------------------

    fn handle_set_setting(&mut self, key: &str, value: Value) {
        let old_library_root = if key == LIBRARY_ROOT {
            self.settings.get_string(LIBRARY_ROOT).unwrap_or_default()
        } else {
            String::new()
        };
        if let Err(e) = self.settings.set(key, value.clone()) {
            self.toast(ToastKind::Error, e.to_string());
            return;
        }
        if key == EXPORT_DIR {
            let new_dir = value
                .as_str()
                .map(PathBuf::from)
                .unwrap_or_else(cliprelay_core::paths::default_export_dir);
            self.export_dir = new_dir.clone();
            self.indexer = Arc::new(MediaIndexer {
                database: Arc::clone(&self.db),
                export_dir: new_dir.clone(),
            });
            self.processor = Arc::new(MediaProcessor::new(
                Arc::clone(&self.db),
                new_dir,
                self.effective_encoder(),
            ));
        }
        if key == LIBRARY_ROOT {
            let new_root = value.as_str().unwrap_or("").to_string();
            if new_root != old_library_root {
                self.handle_library_root_change(new_root.clone());
            }
        }
        if key == SORT_MODE {
            self.library_generation += 1;
            self.library_offset = 0;
            self.library_has_more = false;
        }
        if key == EXPORT_ENCODER || key == PERFORMANCE_MODE {
            let encoder = self.effective_encoder();
            self.processor.set_encoder_mode(encoder);
        }
        if key == AUTO_INDEX && self.settings.get_bool(AUTO_INDEX).unwrap_or(false) {
            let root = self.settings.get_string(LIBRARY_ROOT).unwrap_or_default();
            if !root.is_empty() {
                self.request_scan("automatic", true);
            }
        }
        self.settings_changed();
    }

    fn effective_encoder(&self) -> EncoderMode {
        let encoder = self
            .settings
            .get_string(EXPORT_ENCODER)
            .unwrap_or_else(|_| "auto".into());
        let performance = self
            .settings
            .get_string(PERFORMANCE_MODE)
            .unwrap_or_else(|_| "automatic".into());
        match encoder.as_str() {
            "hardware" => EncoderMode::Hardware,
            "software" => EncoderMode::Software,
            _ => {
                if performance == "maximum" {
                    EncoderMode::Hardware
                } else {
                    EncoderMode::Software
                }
            }
        }
    }

    fn handle_library_root_change(&mut self, new_root: String) {
        let _ = self.settings.set(RANDOM_FOLDER_MODE, json!("all"));
        let _ = self.settings.set(RANDOM_FOLDERS, json!([]));
        let workspace = self.active_mut();
        workspace.root = new_root.clone();
        workspace.folder.clear();
        workspace.search.clear();
        workspace.selected_media_id = 0;
        workspace.selected_media_name.clear();
        workspace.nav_back.clear();
        workspace.nav_forward.clear();
        workspace.random_mode = "all".into();
        workspace.random_folders.clear();
        workspace.draft = Value::Null;
        if !workspace.custom_title {
            workspace.title = workspace_title(&new_root);
        }
        self.selected = None;
        self.emit(Event::SelectedMediaChanged(None));
        self.activate_root(&new_root);
        self.library_generation += 1;
        self.library_offset = 0;
        self.library_has_more = false;
        self.refresh_library();
        self.refresh_folders();
        if !new_root.is_empty() && Path::new(&new_root).is_dir() {
            let auto_index = self.settings.get_bool(AUTO_INDEX).unwrap_or(false);
            self.request_scan("new_root", auto_index);
            if !auto_index {
                self.toast(
                    ToastKind::Success,
                    "Folder selected. Building the fast filename list now.",
                );
            }
        }
        self.persist_workspaces();
    }

    // ---- library browsing ----------------------------------------------

    fn active_library_filter(&self) -> (String, String, String) {
        let workspace = self.active();
        (
            workspace.search.clone(),
            workspace.folder.clone(),
            workspace.sort_mode.clone(),
        )
    }

    fn refresh_library(&mut self) {
        self.refresh_library_page(PAGE_SIZE_LIBRARY as usize);
    }

    fn refresh_library_preserving_loaded(&mut self) {
        self.refresh_library_page(self.library_offset.max(PAGE_SIZE_LIBRARY as usize));
    }

    fn refresh_library_page(&mut self, page_size: usize) {
        let (search, folder, sort_mode) = self.active_library_filter();
        let generation = self.library_generation;
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        std::thread::spawn(move || {
            let rows = db
                .list_media(&search, &folder, &sort_mode, page_size as i64 + 1, 0)
                .unwrap_or_default();
            let has_more = rows.len() > page_size;
            let rows: Vec<_> = rows.into_iter().take(page_size).collect();
            let loaded = rows.len();
            let _ = events.send(Event::LibraryPage(LibraryPage {
                rows,
                has_more,
                offset: 0,
                generation,
            }));
            let _ = events.send(Event::LoadMoreFinished(
                true, generation, has_more, 0, loaded,
            ));
        });
    }

    fn load_more_library(&mut self) {
        if !self.library_has_more || self.library_loading_more {
            return;
        }
        // One load-more at a time (mirrors the original's in-flight
        // guard); each command reserves a distinct offset.
        self.library_loading_more = true;
        let (search, folder, sort_mode) = self.active_library_filter();
        let offset = self.library_offset;
        self.library_offset += PAGE_SIZE_LIBRARY as usize;
        let generation = self.library_generation;
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        std::thread::spawn(move || {
            let rows = db
                .list_media(
                    &search,
                    &folder,
                    &sort_mode,
                    PAGE_SIZE_LIBRARY + 1,
                    offset as i64,
                )
                .unwrap_or_default();
            let has_more = rows.len() as i64 > PAGE_SIZE_LIBRARY;
            let rows: Vec<_> = rows.into_iter().take(PAGE_SIZE_LIBRARY as usize).collect();
            let loaded = rows.len();
            let _ = events.send(Event::LibraryPage(LibraryPage {
                rows,
                has_more,
                offset,
                generation,
            }));
            let _ = events.send(Event::LoadMoreFinished(
                true, generation, has_more, offset, loaded,
            ));
        });
    }

    fn refresh_folders(&mut self) {
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        let sort_mode = {
            let workspace = self.active();
            workspace.folder_sort_mode.clone()
        };
        std::thread::spawn(move || {
            let rows = db.list_explorer_folders().unwrap_or_default();
            // Build a compact tree with auto-expanded top-level branches.
            let row_by_folder: HashMap<&str, _> =
                rows.iter().map(|row| (row.folder.as_str(), row)).collect();
            let mut nodes: HashMap<String, FolderNode> = HashMap::new();
            let mut children_of: HashMap<String, HashSet<String>> = HashMap::new();
            for row in &rows {
                let parts: Vec<&str> = row.folder.split('/').collect();
                for depth in 0..parts.len() {
                    let folder = parts[..=depth].join("/");
                    let parent = if depth == 0 {
                        String::new()
                    } else {
                        parts[..depth].join("/")
                    };
                    children_of
                        .entry(parent.clone())
                        .or_default()
                        .insert(folder.clone());
                    nodes.entry(folder.clone()).or_insert_with(|| {
                        let exact = row_by_folder.get(folder.as_str()).copied();
                        FolderNode {
                            folder: folder.clone(),
                            name: parts[depth].to_string(),
                            count: exact.map_or(0, |value| value.count),
                            depth,
                            has_children: false,
                            expanded: depth < 1,
                            parent,
                            latest_mtime: exact.map_or(0.0, |value| value.latest_mtime),
                            latest_indexed: exact
                                .map_or_else(String::new, |value| value.latest_indexed.clone()),
                        }
                    });
                }
            }
            for node in nodes.values_mut() {
                node.has_children = children_of
                    .get(&node.folder)
                    .is_some_and(|children| !children.is_empty());
            }
            // Sort siblings by the workspace's folder sort mode (level-order
            // walk with per-level sorting).
            let sort_siblings = |siblings: &mut Vec<FolderNode>| {
                siblings.sort_by(|a, b| {
                    let a_name = a.name.to_lowercase();
                    let b_name = b.name.to_lowercase();
                    let order =
                        match sort_mode.as_str() {
                            "name_desc" => b_name.cmp(&a_name),
                            "added_recent" => rank_indexed(&b.latest_indexed)
                                .cmp(&rank_indexed(&a.latest_indexed)),
                            "added_old" => rank_indexed(&a.latest_indexed)
                                .cmp(&rank_indexed(&b.latest_indexed)),
                            "recent" => b.latest_mtime.total_cmp(&a.latest_mtime),
                            "stale" => a.latest_mtime.total_cmp(&b.latest_mtime),
                            "count_desc" => b.count.cmp(&a.count),
                            "count_asc" => a.count.cmp(&b.count),
                            _ => a_name.cmp(&b_name),
                        };
                    order.then_with(|| a_name.cmp(&b_name))
                });
            };
            let mut by_parent: HashMap<String, Vec<FolderNode>> = HashMap::new();
            for node in nodes.into_values() {
                by_parent.entry(node.parent.clone()).or_default().push(node);
            }
            for siblings in by_parent.values_mut() {
                sort_siblings(siblings);
            }
            let mut sorted = Vec::new();
            let mut stack = by_parent.remove("").unwrap_or_default();
            stack.reverse();
            while let Some(node) = stack.pop() {
                sorted.push(node.clone());
                if let Some(mut children) = by_parent.remove(&node.folder) {
                    children.reverse();
                    stack.extend(children);
                }
            }
            let _ = events.send(Event::FoldersUpdated(sorted));
        });
    }

    fn refresh_counts(&mut self) {
        let counts = self.db.counts().unwrap_or((0, 0, 0));
        self.counts = counts;
        self.emit(Event::CountsChanged(counts.0, counts.1, counts.2));
    }

    fn refresh_history(&mut self) {
        let search = self.history_search.clone();
        let generation = self.history_generation;
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        std::thread::spawn(move || {
            let rows = db
                .list_history(&search, PAGE_SIZE_HISTORY + 1, 0)
                .unwrap_or_default();
            let has_more = rows.len() as i64 > PAGE_SIZE_HISTORY;
            let rows: Vec<_> = rows.into_iter().take(PAGE_SIZE_HISTORY as usize).collect();
            let loaded = rows.len();
            let _ = events.send(Event::HistoryPage(HistoryPage {
                rows,
                has_more,
                offset: 0,
                generation,
            }));
            let _ = events.send(Event::LoadMoreFinished(
                false, generation, has_more, 0, loaded,
            ));
        });
    }

    fn load_more_history(&mut self) {
        if !self.history_has_more || self.history_loading_more {
            return;
        }
        // One load-more at a time (mirrors the original's in-flight
        // guard); each command reserves a distinct offset.
        self.history_loading_more = true;
        let search = self.history_search.clone();
        let offset = self.history_offset;
        self.history_offset += PAGE_SIZE_HISTORY as usize;
        let generation = self.history_generation;
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        std::thread::spawn(move || {
            let rows = db
                .list_history(&search, PAGE_SIZE_HISTORY + 1, offset as i64)
                .unwrap_or_default();
            let has_more = rows.len() as i64 > PAGE_SIZE_HISTORY;
            let rows: Vec<_> = rows.into_iter().take(PAGE_SIZE_HISTORY as usize).collect();
            let loaded = rows.len();
            let _ = events.send(Event::HistoryPage(HistoryPage {
                rows,
                has_more,
                offset,
                generation,
            }));
            let _ = events.send(Event::LoadMoreFinished(
                false, generation, has_more, offset, loaded,
            ));
        });
    }

    // ---- selection ------------------------------------------------------

    fn select_media_row(&mut self, media_id: i64) {
        if media_id <= 0 {
            self.selected = None;
            self.emit(Event::SelectedMediaChanged(None));
            return;
        }
        match self.db.get_media(media_id) {
            Ok(Some(row)) => {
                let changed = self
                    .selected
                    .as_ref()
                    .map(|current| current.id != row.id)
                    .unwrap_or(true);
                if changed && !self.nav_restoring {
                    self.record_navigation_origin();
                }
                self.selected = Some(row.clone());
                let workspace = self.active_mut();
                workspace.selected_media_id = row.id;
                workspace.selected_media_name = row.name.clone();
                self.emit(Event::SelectedMediaChanged(Some(row.clone())));
                if changed {
                    self.persist_workspaces();
                }
                if row.duration <= 0.0 {
                    self.verify_selection(row.id);
                } else {
                    self.ensure_thumbnail(row.id);
                    self.ensure_timeline(row.id);
                    // Maximum-performance mode preloads the preview on
                    // selection (mirrors the original).
                    if self
                        .settings
                        .get_string(PERFORMANCE_MODE)
                        .unwrap_or_default()
                        == "maximum"
                        && self.settings.get_bool(HOVER_PREVIEWS).unwrap_or(true)
                    {
                        self.ensure_preview(row.id);
                    }
                }
                self.queue_selection_neighbors();
            }
            _ => {
                self.selected = None;
                self.emit(Event::SelectedMediaChanged(None));
            }
        }
    }

    fn verify_selection(&mut self, media_id: i64) {
        self.checking = true;
        self.emit(Event::SelectionCheckingChanged(media_id, true));
        let indexer = Arc::clone(&self.indexer);
        let events = self.events.clone();
        let db = Arc::clone(&self.db);
        std::thread::spawn(move || {
            let result = indexer.ensure_metadata(media_id);
            let _ = events.send(Event::SelectionCheckingChanged(media_id, false));
            match result {
                Some(row) => {
                    // Route through the controller so a newer selection
                    // wins over a stale in-flight check.
                    let _ = events.send(Event::SelectionVerified(media_id, row));
                }
                None => {
                    // The controller clears the selection only if it still
                    // matches (a newer selection must win).
                    let _ = events.send(Event::SelectionVerifyFailed(media_id));
                    let _ = db.set_media_valid(media_id, false);
                    let _ = events.send(Event::Toast(
                        ToastKind::Warning,
                        "That file is not a readable video.".to_string(),
                    ));
                }
            }
        });
    }

    fn queue_selection_neighbors(&mut self) {
        let Some(media_id) = self.selected.as_ref().map(|m| m.id) else {
            self.nav_available = (false, false);
            self.emit(Event::SelectionNavigationChanged(false, false));
            return;
        };
        let (search, folder, sort_mode) = self.active_library_filter();
        let maximum = self
            .settings
            .get_string(PERFORMANCE_MODE)
            .unwrap_or_default()
            == "maximum";
        let hover_previews = self.settings.get_bool(HOVER_PREVIEWS).unwrap_or(true);
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        let indexer = Arc::clone(&self.indexer);
        std::thread::spawn(move || {
            let result = db
                .navigation_neighbors(media_id, &search, &folder, &sort_mode)
                .unwrap_or((false, 0, 0));
            let _ = events.send(Event::SelectionNavigationChanged(
                result.1 != 0,
                result.2 != 0,
            ));
            // Maximum-performance mode preloads the neighbors' assets
            // (mirrors the original's _preload_selection_neighbors).
            // Previews route through the slotted ensure_preview so the
            // concurrency cap is respected.
            if maximum {
                for neighbor in [result.1, result.2] {
                    if neighbor > 0 {
                        let _ = indexer.ensure_thumbnail(neighbor, None);
                    }
                }
                if hover_previews {
                    let _ = events.send(Event::NeighborPreload(result.1, result.2));
                }
            }
        });
    }

    // ---- scanning -------------------------------------------------------

    fn request_scan(&mut self, reason: &str, include_index: bool) {
        // A new scan supersedes any running one (mirrors the original's
        // pending-request preemption).
        if let Some(existing) = self.scan.as_ref() {
            existing.cancel.store(true, Ordering::Relaxed);
        }
        let workspace = self.active();
        let root = workspace.root.clone();
        let workspace_id = workspace.id.clone();
        if root.is_empty() {
            if include_index || matches!(reason, "manual" | "new_root") {
                self.toast(
                    ToastKind::Info,
                    "Choose a library folder to begin indexing videos.",
                );
            }
            return;
        }
        if !Path::new(&root).is_dir() {
            if matches!(reason, "manual" | "new_root") {
                self.toast(
                    ToastKind::Error,
                    "The selected library folder is unavailable.",
                );
            }
            return;
        }
        let generation = self.next_scan_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let cancel = Arc::new(AtomicBool::new(false));
        self.scan_state.active = true;
        self.scan_state.cancelling = false;
        self.scan_state.progress = -1.0;
        let title = workspace_title(&root);
        self.scan_state.message = format!("Scanning {title} for video filenames");
        self.scan_state.root_name = title.clone();
        self.emit(Event::ScanStateChanged(self.scan_state.clone()));
        let job = ScanJob {
            cancel: Arc::clone(&cancel),
            workspace_id,
            root: root.clone(),
            include_index,
            generation,
        };
        self.scan = Some(job.clone());
        self.emit_workspaces();
        let indexer = Arc::clone(&self.indexer);
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        let current_scan_generation = Arc::clone(&self.next_scan_generation);
        let settings_values = self.settings.as_map().unwrap_or_default();
        std::thread::spawn(move || {
            let verify_during_index = settings_values
                .get(VERIFY_DURING_INDEX)
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let thumbnails_during_index = settings_values
                .get(THUMBNAILS_DURING_INDEX)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let deep_scan = settings_values
                .get(DEEP_SCAN)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let scan = run_scan_job(
                &job,
                &indexer,
                &db,
                &events,
                &current_scan_generation,
                verify_during_index,
                thumbnails_during_index,
                deep_scan,
            );
            if let Some(result) = scan {
                if result.failed > 0
                    && scan_generation_matches(&current_scan_generation, job.generation)
                {
                    let _ = events.send(Event::Toast(
                        ToastKind::Warning,
                        format!(
                            "Indexed {} videos; skipped {} unreadable files.",
                            result.indexed, result.failed
                        ),
                    ));
                }
            }
        });
    }

    fn cancel_scan(&mut self, reason: &str) {
        let Some(scan) = self.scan.take() else {
            return;
        };
        scan.cancel.store(true, Ordering::Relaxed);
        self.scan_state.cancelling = true;
        self.scan_state.message = format!("Stopping scan for {}", self.scan_state.root_name);
        self.emit(Event::ScanStateChanged(self.scan_state.clone()));
        let events = self.events.clone();
        let _ = reason;
        let _ = events;
    }

    // ---- random ---------------------------------------------------------

    fn pick_random(&mut self) {
        if self.random_picking {
            return;
        }
        self.random_picking = true;
        let workspace = self.active();
        if workspace.root.is_empty() {
            self.random_picking = false;
            self.toast(
                ToastKind::Info,
                "Choose a library folder before picking a video.",
            );
            return;
        }
        if workspace.random_mode == "selected" && workspace.random_folders.is_empty() {
            self.random_picking = false;
            self.toast(ToastKind::Info, "Select at least one random source folder.");
            return;
        }
        let avoid_repeats = self.settings.get_bool(AVOID_REPEATS).unwrap_or(true);
        let folders = workspace.random_folders.clone();
        let wait_for_manifest = self.scan.is_some();
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        let _ = events.send(Event::PickingChanged(true));
        std::thread::spawn(move || {
            let mut attempts = 0usize;
            let result = loop {
                match db.random_media(avoid_repeats, false, &folders) {
                    Ok(Some(row)) => {
                        break Ok(Some(row.id));
                    }
                    Ok(None) if wait_for_manifest && attempts < 60 => {
                        attempts += 1;
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Ok(None) => break Ok(None),
                    Err(error) => break Err(error),
                }
            };
            match result {
                Ok(Some(media_id)) => {
                    let _ = events.send(Event::RandomPicked(media_id));
                }
                Ok(None) => {
                    let message = if folders.is_empty() {
                        "No video filenames were found in this library."
                    } else {
                        "No video filenames were found in the selected folders."
                    };
                    let _ = events.send(Event::Toast(ToastKind::Warning, message.to_string()));
                }
                Err(error) => {
                    let _ = events.send(Event::Toast(ToastKind::Error, error.to_string()));
                }
            }
            let _ = events.send(Event::PickingChanged(false));
        });
    }

    fn load_random_folder_options(&mut self) {
        if self.random_folders_loading {
            return;
        }
        self.random_folders_loading = true;
        let workspace = self.active();
        let selected_direct: HashSet<String> = workspace
            .random_folders
            .iter()
            .filter_map(|token| token.strip_prefix('\u{1e}'))
            .map(|s| s.to_string())
            .collect();
        let random_mode = workspace.random_mode.clone();
        // The controller already runs off the GPUI application thread. Keep
        // this small indexed query serialized here so repeated selection or
        // scan refreshes cannot race stale snapshots back into the popup.
        let rows = self.db.list_random_folders().unwrap_or_default();
        let (options, summary, selected_count, all_selected, has_selection) =
            build_random_options(&rows, &selected_direct, &random_mode);
        self.random_folders_cache = options.clone();
        self.random_folders_selected = selected_count;
        self.all_random_folders_selected = all_selected;
        self.has_random_folder_selection = has_selection;
        self.random_folders_loading = false;
        self.emit(Event::RandomFoldersChanged(
            options,
            summary,
            selected_count,
            all_selected,
            has_selection,
        ));
    }

    fn set_random_folder_enabled(&mut self, folder: &str, enabled: bool) {
        // Subtree semantics (mirrors the original): toggling a folder
        // affects every direct-video folder at or below it.
        let all = self.db.list_random_folders().unwrap_or_default();
        let direct_all: Vec<String> = all
            .iter()
            .filter(|r| r.direct_count > 0)
            .map(|r| r.folder.clone())
            .collect();
        let subtree: Vec<String> = direct_all
            .iter()
            .filter(|path| path.as_str() == folder || path.starts_with(&format!("{folder}/")))
            .cloned()
            .collect();
        let mode_all = self.active().random_mode == "all";
        let mut selected: std::collections::HashSet<String> = if mode_all && enabled {
            std::collections::HashSet::new()
        } else if mode_all {
            direct_all.iter().cloned().collect()
        } else {
            self.active()
                .random_folders
                .iter()
                .filter_map(|t| t.strip_prefix('\u{1e}'))
                .map(String::from)
                .collect()
        };
        if enabled {
            for path in &subtree {
                selected.insert(path.clone());
            }
        } else {
            for path in &subtree {
                selected.remove(path);
            }
        }
        let all_selected =
            !direct_all.is_empty() && direct_all.iter().all(|path| selected.contains(path));
        let tokens: Vec<String> = direct_all
            .iter()
            .filter(|path| selected.contains(*path))
            .map(|path| format!("\u{1e}{path}"))
            .collect();
        let (random_mode, random_folders) = {
            let workspace = self.active_mut();
            if all_selected {
                workspace.random_mode = "all".into();
                workspace.random_folders.clear();
            } else {
                workspace.random_mode = "selected".into();
                workspace.random_folders = tokens;
            }
            (
                workspace.random_mode.clone(),
                workspace.random_folders.clone(),
            )
        };
        let _ = self.settings.set(RANDOM_FOLDER_MODE, json!(random_mode));
        let _ = self.settings.set(RANDOM_FOLDERS, json!(random_folders));
        self.persist_workspaces();
        self.settings_changed();
        self.load_random_folder_options();
    }

    fn reset_shuffle(&mut self) {
        let workspace = self.active();
        if workspace.random_mode == "selected" && workspace.random_folders.is_empty() {
            self.toast(ToastKind::Info, "Select at least one random source folder.");
            return;
        }
        let root = workspace.root.clone();
        let folders = workspace.random_folders.clone();
        let random_mode = workspace.random_mode.clone();
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        std::thread::spawn(move || {
            let result = if root.is_empty() {
                db.reset_shuffle(None, &folders)
            } else {
                db.reset_shuffle(Some(&root), &folders)
            };
            match result {
                Ok(()) => {
                    let message = if folders.is_empty() {
                        "Random selection history was reset."
                    } else {
                        "Random history was reset for the selected folders."
                    };
                    let _ = events.send(Event::Toast(ToastKind::Success, message.to_string()));
                    // Refresh the random-source counts (mirrors the
                    // original's option refresh after a reset).
                    if let Ok(rows) = db.list_random_folders() {
                        let selected_direct: std::collections::HashSet<String> = folders
                            .iter()
                            .filter_map(|t| t.strip_prefix('\u{1e}'))
                            .map(|s| s.to_string())
                            .collect();
                        let (options, summary, selected_count, all_selected, has_selection) =
                            build_random_options(&rows, &selected_direct, &random_mode);
                        let _ = events.send(Event::RandomFoldersChanged(
                            options,
                            summary,
                            selected_count,
                            all_selected,
                            has_selection,
                        ));
                    }
                }
                Err(e) => {
                    let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
                }
            }
        });
    }

    // ---- thumbnails / previews / timeline -------------------------------

    fn ensure_thumbnail(&mut self, media_id: i64) {
        if media_id <= 0 || self.shutting_down {
            return;
        }
        if self.thumb_queue.contains(&media_id) {
            return;
        }
        self.thumb_queue.insert(media_id);
        let _ = self.thumb_tx.send(media_id);
    }

    fn ensure_preview(&mut self, media_id: i64) {
        if !self.settings.get_bool(HOVER_PREVIEWS).unwrap_or(true) {
            return;
        }
        if !self.preview_pending.lock().insert(media_id) {
            return;
        }
        // Bound concurrent preview encodes like the original's semaphore
        // (1 automatic, 2 maximum performance).
        let slots = Arc::clone(&self.preview_slots);
        let pending = Arc::clone(&self.preview_pending);
        let cap = if self
            .settings
            .get_string(PERFORMANCE_MODE)
            .unwrap_or_default()
            == "maximum"
        {
            2
        } else {
            1
        };
        let indexer = Arc::clone(&self.indexer);
        let events = self.events.clone();
        std::thread::spawn(move || {
            if slots.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= cap {
                slots.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                // Release the pending mark so a later hover can retry.
                pending.lock().remove(&media_id);
                std::thread::sleep(Duration::from_millis(80));
                let _ = events.send(Event::PreviewDeferred(media_id));
                return;
            }
            let result = indexer.ensure_preview(media_id);
            slots.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            // Release the pending mark so later hovers can re-extract.
            pending.lock().remove(&media_id);
            let _ = events.send(Event::PreviewReady(media_id, result));
        });
    }

    fn ensure_timeline(&mut self, media_id: i64) {
        if media_id <= 0 {
            return;
        }
        // Dedupe in-flight requests (the UI and selection paths both call).
        if self.timeline_pending.as_ref().map(|(id, _)| *id) == Some(media_id) {
            return;
        }
        if media_id <= 0 {
            return;
        }
        self.timeline_generation += 1;
        let generation = self.timeline_generation;
        self.timeline_pending = Some((media_id, generation));
        self.timeline_loading = true;
        self.emit(Event::TimelineLoadingChanged(media_id, true));
        let indexer = Arc::clone(&self.indexer);
        let events = self.events.clone();
        let db = Arc::clone(&self.db);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(160));
            let result = indexer.ensure_timeline(media_id, 12);
            let _ = db.get_media(media_id);
            let _ = events.send(Event::TimelineReady(media_id, result));
            let _ = events.send(Event::TimelineFinished(media_id));
        });
    }

    // ---- publish --------------------------------------------------------

    #[allow(dead_code)]
    fn estimate_output_size(
        &self,
        trim_start: f64,
        trim_end: f64,
        preset: &str,
        target_mb: f64,
    ) -> String {
        let Some(media) = &self.selected else {
            return String::new();
        };
        let source_duration = media.duration.max(0.05);
        let end = if trim_end > 0.0 {
            trim_end
        } else {
            source_duration
        };
        let duration = (end - trim_start).max(0.05);
        let ratio = duration / source_duration;
        let size = media.size_bytes as f64;
        match preset {
            "fit_bot" => "up to 49 MB".into(),
            "fit_x" => {
                let limit = self.settings.get_f64(X_LIMIT_MB).unwrap_or(512.0);
                format!("up to {:.0} MB", limit * 0.98)
            }
            "fit_both" => {
                let limit = self.settings.get_f64(X_LIMIT_MB).unwrap_or(512.0);
                format!("up to {:.0} MB", limit.min(50.0) * 0.98)
            }
            "custom" if target_mb > 0.0 => format!("up to {target_mb:.0} MB"),
            "smallest" => format_bytes((size * ratio * 0.28).min(duration * 750_000.0 / 8.0)),
            "balanced" => format_bytes((size * ratio * 0.62).min(duration * 2_500_000.0 / 8.0)),
            _ => format_bytes(size * ratio),
        }
    }

    #[allow(dead_code)]
    fn target_for(
        &self,
        preset: &str,
        target_mb: f64,
        telegram_enabled: bool,
        x_enabled: bool,
        telegram_mode: &str,
    ) -> f64 {
        let x_limit = self.settings.get_f64(X_LIMIT_MB).unwrap_or(512.0);
        resolve_target(
            preset,
            target_mb,
            telegram_enabled,
            x_enabled,
            telegram_mode,
            x_limit,
        )
    }

    fn publish(&mut self, payload: PublishPayload) {
        if self.publish_state.active {
            return;
        }
        let Some(media) = self
            .db
            .get_media(payload.media_id)
            .ok()
            .flatten()
            .or_else(|| self.selected.clone())
        else {
            self.toast(ToastKind::Error, "Choose a video before preparing a post.");
            return;
        };
        if media.duration <= 0.0 {
            // Mirror the original's readability gate: check the file first
            // (clears the selection + toasts when unreadable) and require a
            // fresh publish once it is verified.
            self.verify_selection(media.id);
            return;
        }
        if self.checking {
            self.toast(
                ToastKind::Info,
                "Wait for the selected video check to finish.",
            );
            return;
        }
        if !payload.telegram_enabled && !payload.x_enabled {
            self.toast(ToastKind::Warning, "Choose Telegram, X, or both.");
            return;
        }
        if payload.telegram_enabled {
            if payload.telegram_destination.trim().is_empty() {
                self.toast(ToastKind::Warning, "Choose a Telegram destination.");
                return;
            }
            if payload.telegram_mode == "bot" && !self.bot_configured {
                self.toast(
                    ToastKind::Warning,
                    "Connect a Telegram bot in Settings before sending.",
                );
                return;
            }
            if payload.telegram_mode == "personal" && !self.personal_configured {
                self.toast(
                    ToastKind::Warning,
                    "Sign in to your personal Telegram account in Settings before sending.",
                );
                return;
            }
        }
        if payload.telegram_enabled {
            let _ = self
                .settings
                .set(TELEGRAM_MODE, json!(payload.telegram_mode));
            let _ = self.settings.set(
                TELEGRAM_DESTINATION,
                json!(payload.telegram_destination.trim()),
            );
            self.settings_changed();
        }
        let post_id = match self.db.create_post(&PostValues {
            media_id: media.id,
            export_id: None,
            telegram_enabled: payload.telegram_enabled,
            x_enabled: payload.x_enabled,
            telegram_caption: payload.telegram_caption.clone(),
            x_caption: payload.x_caption.clone(),
            telegram_mode: payload.telegram_mode.clone(),
            telegram_destination: payload.telegram_destination.trim().to_string(),
            cleanup_policy: payload.cleanup_policy.clone(),
        }) {
            Ok(id) => id,
            Err(e) => {
                self.toast(ToastKind::Error, e.to_string());
                return;
            }
        };
        // Resolve the size target now: the presets ("fit_bot"/"fit_x"/
        // "fit_both") must enforce their limits regardless of the
        // custom-size field (mirrors the original's _target_for).
        let mut payload = payload;
        payload.target_mb = self.target_for(
            &payload.preset,
            payload.target_mb,
            payload.telegram_enabled,
            payload.x_enabled,
            &payload.telegram_mode,
        );
        self.publish_state = PublishState {
            active: true,
            progress: 0.0,
            stage: "Preparing video".into(),
            post_id,
            ..Default::default()
        };
        self.emit(Event::PublishStateChanged(self.publish_state.clone()));
        self.run_publish_async(media, payload, post_id);
    }

    fn run_publish_async(&mut self, media: MediaRow, payload: PublishPayload, post_id: i64) {
        let processor = Arc::clone(&self.processor);
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        let export_dir = self.export_dir.clone();
        let settings_snapshot = self.settings.as_map().unwrap_or_default();
        let bot_token = self.secrets.get("telegram_bot_token", "");
        let personal_credentials = (
            self.settings
                .get_string(TELEGRAM_API_ID)
                .unwrap_or_default(),
            self.secrets.get("telegram_api_hash", ""),
        );
        let cleanup_policy = payload.cleanup_policy.clone();
        std::thread::spawn(move || {
            let result = publish_job(
                &processor,
                &db,
                &events,
                &export_dir,
                &media,
                &payload,
                post_id,
                &settings_snapshot,
                &bot_token,
                &personal_credentials,
            );
            let _ = events.send(Event::PublishStateChanged(result));
            let _ = events.send(Event::HistoryRefreshed);
            let _ = events.send(Event::LibraryRefreshed);
            let _ = events.send(Event::CountsOnly);
            let _ = cleanup_policy;
        });
    }

    fn cancel_publish(&mut self) {
        self.processor.cancel();
    }

    fn retry_telegram(&mut self, post_id: i64) {
        let Some(post) = self.db.get_post(post_id).ok().flatten() else {
            return;
        };
        let Some(path) = post.export_path.clone().or(post.source_path.clone()) else {
            self.toast(
                ToastKind::Error,
                "The video used for this post is no longer available.",
            );
            return;
        };
        let path = PathBuf::from(path);
        if !path.is_file() {
            self.toast(
                ToastKind::Error,
                "The video used for this post is no longer available.",
            );
            return;
        }
        self.publish_state = PublishState {
            active: true,
            progress: 0.0,
            stage: "Retrying Telegram".into(),
            post_id,
            ..Default::default()
        };
        self.emit(Event::PublishStateChanged(self.publish_state.clone()));
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        let mode = post.telegram_mode.clone();
        let destination = post.telegram_destination.clone();
        let caption = post.telegram_caption.clone();
        let bot_token = self.secrets.get("telegram_bot_token", "");
        let personal_credentials = (
            self.settings
                .get_string(TELEGRAM_API_ID)
                .unwrap_or_default(),
            self.secrets.get("telegram_api_hash", ""),
        );
        let progress_events = events.clone();
        std::thread::spawn(move || {
            let outcome = send_telegram(
                &mode,
                &destination,
                &path,
                &caption,
                &bot_token,
                &personal_credentials,
                &(Arc::new(move |fraction: f64, stage: &str| {
                    let _ = progress_events.send(Event::PublishStateChanged(PublishState {
                        active: true,
                        progress: fraction,
                        stage: stage.to_string(),
                        post_id,
                        ..Default::default()
                    }));
                }) as ProgressCb),
            );
            match outcome {
                Ok(delivery) => {
                    let _ = db.update_post(
                        post_id,
                        &PostUpdate {
                            telegram_status: Some("sent".into()),
                            telegram_message_id: Some(delivery.message_id.clone()),
                            telegram_message_link: Some(delivery.link.clone()),
                            error: Some(String::new()),
                            ..Default::default()
                        },
                    );
                    let _ = db.add_attempt(
                        post_id,
                        "telegram",
                        "sent",
                        "Retry succeeded",
                        &delivery.message_id,
                        true,
                    );
                    let _ = events.send(Event::Toast(
                        ToastKind::Success,
                        "Telegram retry succeeded.".to_string(),
                    ));
                    let _ = events.send(Event::HistoryRefreshed);
                }
                Err(e) => {
                    let _ = db.update_post(
                        post_id,
                        &PostUpdate {
                            telegram_status: Some("failed".into()),
                            error: Some(e.to_string()),
                            ..Default::default()
                        },
                    );
                    let _ = db.add_attempt(post_id, "telegram", "failed", &e.to_string(), "", true);
                    let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
                    let _ = events.send(Event::HistoryRefreshed);
                }
            }
            let _ = events.send(Event::PublishStateChanged(PublishState {
                active: false,
                post_id,
                ..Default::default()
            }));
        });
    }

    fn mark_x_posted(&mut self, post_id: i64, url: &str) {
        let _ = self.db.update_post(
            post_id,
            &PostUpdate {
                x_status: Some("posted".into()),
                x_url: Some(url.trim().to_string()),
                ..Default::default()
            },
        );
        let _ = self.db.add_attempt(
            post_id,
            "x",
            "posted",
            "Confirmed by user",
            url.trim(),
            true,
        );
        self.apply_cleanup(post_id);
        self.refresh_history();
        self.toast(ToastKind::Success, "X post marked as posted.");
    }

    fn prepare_x_again(&mut self, post_id: i64) {
        let Some(post) = self.db.get_post(post_id).ok().flatten() else {
            return;
        };
        let Some(path) = post.export_path.clone().or(post.source_path.clone()) else {
            self.toast(
                ToastKind::Error,
                "The video used for this post is no longer available.",
            );
            return;
        };
        let path = PathBuf::from(path);
        if !path.is_file() {
            self.toast(
                ToastKind::Error,
                "The video used for this post is no longer available.",
            );
            return;
        }
        let caption = post.x_caption.clone();
        if let Err(e) = XAssistant::prepare(&path, &caption) {
            self.toast(ToastKind::Error, e.to_string());
            return;
        }
        let _ = self.db.update_post(
            post_id,
            &PostUpdate {
                x_status: Some("prepared".into()),
                ..Default::default()
            },
        );
        self.refresh_history();
    }

    fn trash_export(&mut self, post_id: i64) {
        let Some(post) = self.db.get_post(post_id).ok().flatten() else {
            return;
        };
        let Some(export_id) = post.export_id else {
            return;
        };
        let Some(export_path) = post.export_path.clone() else {
            return;
        };
        let is_generated = post.is_generated.unwrap_or(true);
        match move_generated_to_trash(&export_path, &self.export_dir, is_generated) {
            Ok(()) => {
                let _ = self.db.mark_export_cleanup(export_id, "trashed");
                self.refresh_history();
                self.toast(ToastKind::Success, "Generated video moved to Trash.");
            }
            Err(e) => self.toast(ToastKind::Error, e.to_string()),
        }
    }

    fn apply_cleanup(&mut self, post_id: i64) {
        let Some(post) = self.db.get_post(post_id).ok().flatten() else {
            return;
        };
        let (Some(export_id), Some(export_path)) = (post.export_id, post.export_path.clone())
        else {
            return;
        };
        if !post.is_generated.unwrap_or(true) {
            return;
        }
        let telegram_done = matches!(post.telegram_status.as_str(), "sent" | "not_requested");
        let x_done = matches!(post.x_status.as_str(), "posted" | "not_requested");
        let trash = match post.cleanup_policy.as_str() {
            "after_complete" => telegram_done && x_done,
            "after_telegram" => post.x_status == "not_requested" && telegram_done,
            _ => false,
        };
        if trash && move_generated_to_trash(&export_path, &self.export_dir, true).is_ok() {
            let _ = self.db.mark_export_cleanup(export_id, "trashed");
            self.refresh_history();
        }
    }

    // ---- telegram connection -------------------------------------------

    fn validate_bot_token(&mut self, token: String) {
        self.telegram_state.message = "Checking bot token".into();
        self.emit(Event::TelegramStateChanged(self.telegram_state.clone()));
        let events = self.events.clone();
        let mut bot = TelegramBotService::new(SecretStore::new(None));
        std::thread::spawn(move || match bot.validate(&token) {
            Ok(result) => {
                let username = result
                    .get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or("bot")
                    .to_string();
                let _ = events.send(Event::TelegramStateChanged(TelegramState {
                    bot: format!("@{username}"),
                    personal: "not signed in".into(),
                    message: "Bot connected".into(),
                    password_required: false,
                }));
                let _ = events.send(Event::MarkBotConfigured(username));
                let _ = events.send(Event::Toast(
                    ToastKind::Success,
                    "Telegram bot connected.".to_string(),
                ));
                let _ = events.send(Event::SettingsChanged(std::collections::HashMap::new()));
            }
            Err(e) => {
                let _ = events.send(Event::TelegramStateChanged(TelegramState {
                    bot: "not configured".into(),
                    personal: "not signed in".into(),
                    message: e.to_string(),
                    password_required: false,
                }));
                let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
            }
        });
    }

    fn validate_bot_destination(&mut self, destination: String) {
        let events = self.events.clone();
        let mut bot = TelegramBotService::new(SecretStore::new(None));
        std::thread::spawn(move || match bot.validate_destination(&destination) {
            Ok(chat) => {
                let title = chat
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&destination)
                    .to_string();
                let _ = events.send(Event::Toast(
                    ToastKind::Success,
                    format!("Telegram destination ready: {title}"),
                ));
                let _ = events.send(Event::SettingsChanged(std::collections::HashMap::new()));
            }
            Err(e) => {
                let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
            }
        });
    }

    fn mark_bot_configured(&mut self, username: String) {
        self.bot_configured = true;
        let _ = self.settings.set(TELEGRAM_BOT_CONFIGURED, json!(true));
        self.telegram_state.bot = if username.starts_with('@') {
            username
        } else {
            format!("@{username}")
        };
        self.settings_changed();
    }

    fn mark_personal_configured(&mut self, display: String) {
        self.personal_configured = true;
        let _ = self.settings.set(TELEGRAM_PERSONAL_CONFIGURED, json!(true));
        if !display.is_empty() {
            self.telegram_state.personal = display;
        }
        self.settings_changed();
    }

    fn disconnect_bot(&mut self) {
        self.secrets.delete("telegram_bot_token");
        self.bot_configured = false;
        let _ = self.settings.set(TELEGRAM_BOT_CONFIGURED, json!(false));
        self.telegram_state.bot = "not configured".into();
        self.telegram_state.message = "Bot token removed".into();
        self.emit(Event::TelegramStateChanged(self.telegram_state.clone()));
        self.settings_changed();
        self.toast(ToastKind::Success, "Telegram bot disconnected.");
    }

    fn begin_personal_login(&mut self, api_id: i32, api_hash: String, phone: String) {
        self.telegram_state.message = "Requesting Telegram sign-in code".into();
        self.telegram_state.password_required = false;
        self.emit(Event::TelegramStateChanged(self.telegram_state.clone()));
        let _ = self
            .settings
            .set(TELEGRAM_API_ID, json!(api_id.to_string()));
        let _ = self.settings.set(TELEGRAM_PHONE, json!(phone.clone()));
        let events = self.events.clone();
        let personal = Arc::clone(&self.personal);
        std::thread::spawn(
            move || match personal.lock().begin_login(api_id, &api_hash, &phone) {
                Ok(()) => {
                    let _ = events.send(Event::TelegramStateChanged(TelegramState {
                        bot: "configured".into(),
                        personal: "not signed in".into(),
                        message: "Enter the code Telegram sent".into(),
                        password_required: false,
                    }));
                    let _ = events.send(Event::SettingsChanged(std::collections::HashMap::new()));
                }
                Err(e) => {
                    let _ = events.send(Event::TelegramStateChanged(TelegramState {
                        bot: "configured".into(),
                        personal: "not signed in".into(),
                        message: e.to_string(),
                        password_required: false,
                    }));
                    let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
                }
            },
        );
    }

    fn complete_personal_login(&mut self, code: String, password: String) {
        let events = self.events.clone();
        let personal = Arc::clone(&self.personal);
        std::thread::spawn(
            move || match personal.lock().complete_login(&code, &password) {
                Ok(display) => {
                    let _ = events.send(Event::TelegramStateChanged(TelegramState {
                        bot: "configured".into(),
                        personal: display.clone(),
                        message: "Personal account connected".into(),
                        password_required: false,
                    }));
                    let _ = events.send(Event::MarkPersonalConfigured(display.clone()));
                    let _ = events.send(Event::Toast(
                        ToastKind::Success,
                        format!("Signed in as {display}."),
                    ));
                    let _ = events.send(Event::SettingsChanged(std::collections::HashMap::new()));
                }
                Err(e) => {
                    let is_password = e
                        .downcast_ref::<TelegramError>()
                        .is_some_and(|err| matches!(err, TelegramError::PasswordRequired(_)));
                    let _ = events.send(Event::TelegramStateChanged(TelegramState {
                        bot: "configured".into(),
                        personal: "not signed in".into(),
                        message: e.to_string(),
                        password_required: is_password,
                    }));
                    if !is_password {
                        let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
                    }
                }
            },
        );
    }

    fn load_telegram_dialogs(&mut self) {
        let api_id: i32 = self
            .settings
            .get_string(TELEGRAM_API_ID)
            .unwrap_or_default()
            .parse()
            .unwrap_or(0);
        let events = self.events.clone();
        let personal = Arc::clone(&self.personal);
        std::thread::spawn(move || match personal.lock().dialogs(api_id) {
            Ok(dialogs) => {
                let _ = events.send(Event::TelegramDialogsChanged(dialogs));
            }
            Err(e) => {
                let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
            }
        });
    }

    fn sign_out_personal(&mut self) {
        let api_id: i32 = self
            .settings
            .get_string(TELEGRAM_API_ID)
            .unwrap_or_default()
            .parse()
            .unwrap_or(0);
        let personal = Arc::clone(&self.personal);
        std::thread::spawn(move || {
            personal.lock().sign_out(api_id);
        });
        self.personal_configured = false;
        let _ = self
            .settings
            .set(TELEGRAM_PERSONAL_CONFIGURED, json!(false));
        self.telegram_state.personal = "not signed in".into();
        self.telegram_state.message = "Personal Telegram session removed".into();
        self.emit(Event::TelegramStateChanged(self.telegram_state.clone()));
        self.emit(Event::TelegramDialogsChanged(Vec::new()));
        self.settings_changed();
    }

    // ---- history / reveal / navigation ---------------------------------

    fn view_history_post(&mut self, post_id: i64) {
        let Some(post) = self.db.get_post(post_id).ok().flatten() else {
            self.toast(
                ToastKind::Error,
                "That history item is no longer available.",
            );
            return;
        };
        let Some(source) = post.source_path.clone() else {
            self.toast(
                ToastKind::Error,
                "The source video for this history item is no longer available.",
            );
            return;
        };
        let source = PathBuf::from(&source);
        let workspace = self.active();
        if !workspace.root.is_empty() {
            let root = PathBuf::from(&workspace.root);
            if !is_within(&source, &root) {
                self.toast(
                    ToastKind::Error,
                    "Open this video's library folder before viewing it in Prepare.",
                );
                return;
            }
        }
        // The workspace root may be unresolved (/tmp) while the row's
        // root_path is canonical (/private/tmp): compare resolved.
        if let Some(row) = self
            .db
            .get_media_by_path(&source.to_string_lossy())
            .ok()
            .flatten()
        {
            if !same_root(&row.root_path, &workspace.root) {
                self.toast(
                    ToastKind::Error,
                    "Open this video's library folder before viewing it in Prepare.",
                );
                return;
            }
        }
        if !source.is_file() {
            self.toast(
                ToastKind::Error,
                "The source video for this history item is no longer available.",
            );
            return;
        }
        self.reveal_media_path(&source);
    }

    fn reveal_selected(&mut self) {
        let Some(selected) = self.selected.clone() else {
            return;
        };
        let path = PathBuf::from(&selected.path);
        self.reveal_media_path(&path);
    }

    fn reveal_media_path(&mut self, path: &Path) {
        let Some(row) = self
            .db
            .get_media_by_path(&path.to_string_lossy())
            .ok()
            .flatten()
        else {
            self.toast(
                ToastKind::Error,
                "The selected video is no longer in the library.",
            );
            return;
        };
        let (root, sort_mode) = {
            let workspace = self.active();
            (workspace.root.clone(), workspace.sort_mode.clone())
        };
        // Mirrors the original's reveal gates (roots compared resolved,
        // like the original's .resolve()).
        if !row.root_path.is_empty() && !same_root(&row.root_path, &root) {
            self.toast(
                ToastKind::Error,
                "Open this video's library folder before viewing it in Prepare.",
            );
            return;
        }
        if !row.active || !row.valid {
            self.toast(
                ToastKind::Error,
                "This source video is not available in the current library.",
            );
            return;
        }
        self.record_navigation_origin();
        {
            let workspace = self.active_mut();
            workspace.folder = row.folder.clone();
            workspace.search.clear();
        }
        let folder = row.folder.clone();
        let media_id = row.id;
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        std::thread::spawn(move || {
            let index = reveal_index(&db, media_id, &folder, &sort_mode);
            if index.is_none() {
                let _ = events.send(Event::Toast(
                    ToastKind::Error,
                    "The selected video could not be shown in the library.".to_string(),
                ));
            }
            let _ = events.send(Event::RevealRequested {
                folder,
                media_index: index.unwrap_or(-1),
                folder_index: -1,
            });
            log::info!("library reveal resolved media {media_id} to index {index:?}");
            let _ = root;
        });
        self.library_generation += 1;
        self.library_offset = 0;
        self.library_has_more = false;
        self.refresh_library();
        self.emit(Event::NavigationRequested(Page::Library));
    }

    /// Emit a RevealRequested for a media's index in the current folder
    /// (no folder switch, no search change). Used to chase a selection that
    /// lives beyond the loaded library page.
    fn reveal_media_index(&mut self, media_id: i64) {
        let (folder, sort_mode) = {
            let workspace = self.active();
            (workspace.folder.clone(), workspace.sort_mode.clone())
        };
        let db = Arc::clone(&self.db);
        let events = self.events.clone();
        std::thread::spawn(move || {
            let index = reveal_index(&db, media_id, &folder, &sort_mode);
            let _ = events.send(Event::RevealRequested {
                folder,
                media_index: index.unwrap_or(-1),
                folder_index: -1,
            });
        });
    }

    fn navigate_selection(&mut self, direction: i32) {
        let Some(selected) = self.selected.as_ref().map(|m| m.id) else {
            // Nothing selected: pick the first row (or the last when going
            // backwards, mirroring the original).
            let (search, folder, sort_mode) = self.active_library_filter();
            let target = if direction < 0 {
                let offset = self
                    .db
                    .media_count(&search, &folder)
                    .unwrap_or(0)
                    .saturating_sub(1);
                let rows = self
                    .db
                    .list_media(&search, &folder, &sort_mode, 1, offset as i64)
                    .unwrap_or_default();
                rows.last().map(|row| row.id)
            } else {
                let rows = self
                    .db
                    .list_media(&search, &folder, &sort_mode, 1, 0)
                    .unwrap_or_default();
                rows.first().map(|row| row.id)
            };
            if let Some(id) = target {
                self.select_media_row(id);
            }
            return;
        };
        let (search, folder, sort_mode) = self.active_library_filter();
        let (found, prev, next) = self
            .db
            .navigation_neighbors(selected, &search, &folder, &sort_mode)
            .unwrap_or((false, 0, 0));
        if found {
            let target = if direction < 0 { prev } else { next };
            if target > 0 {
                self.select_media_row(target);
            }
        }
    }

    fn navigate_back(&mut self) {
        self.navigate_history(false);
    }

    fn navigate_forward(&mut self) {
        self.navigate_history(true);
    }

    /// Push the workspace's current folder/search/selection onto the back
    /// stack and clear the forward stack (mirrors the original's
    /// `_record_navigation_origin`). Called before user-driven navigation.
    fn record_navigation_origin(&mut self) {
        if self.nav_restoring {
            return;
        }
        let workspace = self.active_mut();
        let state = NavState {
            folder: workspace.folder.clone(),
            search: workspace.search.clone(),
            media_id: workspace.selected_media_id,
        };
        if workspace.nav_back.last() == Some(&state) {
            return;
        }
        workspace.nav_back.push(state);
        if workspace.nav_back.len() > 200 {
            workspace.nav_back.drain(..workspace.nav_back.len() - 200);
        }
        workspace.nav_forward.clear();
    }

    fn navigate_history(&mut self, forward: bool) {
        self.nav_restoring = true;
        let workspace = self.active_mut();
        let (mut source, destination) = if forward {
            (
                std::mem::take(&mut workspace.nav_forward),
                &mut workspace.nav_back,
            )
        } else {
            (
                std::mem::take(&mut workspace.nav_back),
                &mut workspace.nav_forward,
            )
        };
        let Some(state) = source.pop() else {
            self.nav_restoring = false;
            return;
        };
        let current = NavState {
            folder: workspace.folder.clone(),
            search: workspace.search.clone(),
            media_id: workspace.selected_media_id,
        };
        destination.push(current);
        destination.truncate(200);
        workspace.folder = state.folder.clone();
        workspace.search = state.search.clone();
        if forward {
            workspace.nav_forward = source;
        } else {
            workspace.nav_back = source;
        }
        let folder = state.folder;
        let search = state.search;
        let media_id = state.media_id;
        self.library_generation += 1;
        self.library_offset = 0;
        self.library_has_more = false;
        self.refresh_library();
        self.emit(Event::NavigationRestored {
            folder,
            search,
            folder_index: -1,
        });
        self.persist_workspaces();
        if media_id > 0 {
            // Validate the restored media before selecting (mirrors the
            // original's _resolve_navigation_state): dead rows or media from
            // a different root are skipped.
            let usable = self
                .db
                .get_media(media_id)
                .ok()
                .flatten()
                .map(|row| {
                    let root = self.active().root.clone();
                    (root.is_empty() || same_root(&row.root_path, &root))
                        && row.active
                        && row.valid
                        && std::path::Path::new(&row.path).is_file()
                })
                .unwrap_or(false);
            if usable {
                self.select_media_row(media_id);
            }
        }
        self.nav_restoring = false;
    }

    // ---- command dispatch ----------------------------------------------

    fn handle(&mut self, command: Command) {
        match command {
            Command::ChooseLibrary(root) => self.handle_set_setting(LIBRARY_ROOT, json!(root)),
            Command::SetSetting(key, value) => self.handle_set_setting(&key, value),
            Command::ActivateWorkspace(index) => {
                if let Some(workspace) = self.workspaces.get(index) {
                    let id = workspace.id.clone();
                    self.activate_workspace_by_id(&id, false);
                }
            }
            Command::CreateWorkspace(root) => self.handle_create_workspace(root),
            Command::CloseWorkspace(id) => self.handle_close_workspace(&id),
            Command::DuplicateWorkspace(id) => self.handle_duplicate_workspace(&id),
            Command::CloseOtherWorkspaces(id) => {
                let removed: Vec<Value> = self
                    .workspaces
                    .iter()
                    .filter(|w| w.id != id)
                    .map(|w| self.workspace_snapshot(w))
                    .collect();
                self.workspaces.retain(|w| w.id == id);
                for snapshot in removed.into_iter().rev() {
                    self.closed.push(snapshot);
                }
                if self.closed.len() > 10 {
                    self.closed.drain(..self.closed.len() - 10);
                }
                self.emit(Event::ClosedCountChanged(self.closed.len()));
                if self.workspaces.is_empty() {
                    // The id may be stale (captured before a queued close);
                    // keep the never-empty workspace invariant.
                    self.workspaces
                        .push(Workspace::new(new_id(), String::new(), None));
                }
                let target = self.workspaces.first().map(|w| w.id.clone());
                if let Some(target) = target {
                    self.active_id = String::new();
                    self.activate_workspace_by_id(&target, false);
                }
                self.persist_workspaces();
            }
            Command::CloseWorkspacesToRight(id) => {
                let Some(index) = self.workspaces.iter().position(|w| w.id == id) else {
                    return;
                };
                let removed: Vec<Value> = self
                    .workspaces
                    .iter()
                    .skip(index + 1)
                    .map(|w| self.workspace_snapshot(w))
                    .collect();
                self.workspaces.truncate(index + 1);
                for snapshot in removed.into_iter().rev() {
                    self.closed.push(snapshot);
                }
                if self.closed.len() > 10 {
                    self.closed.drain(..self.closed.len() - 10);
                }
                self.emit(Event::ClosedCountChanged(self.closed.len()));
                self.persist_workspaces();
            }
            Command::ReopenClosedWorkspace => self.handle_reopen_closed(),
            Command::RenameWorkspace(id, title) => self.handle_rename_workspace(&id, title),
            Command::RevealWorkspaceRoot(id) => {
                let root = self
                    .workspaces
                    .iter()
                    .find(|w| w.id == id)
                    .map(|w| w.root.clone())
                    .unwrap_or_default();
                if root.is_empty() {
                    self.toast(
                        ToastKind::Info,
                        "This workspace does not have a root folder yet.",
                    );
                } else if let Err(e) = XAssistant::reveal(Path::new(&root)) {
                    self.toast(ToastKind::Error, e.to_string());
                }
            }
            Command::SetSearch(search) => {
                let workspace = self.active_mut();
                if workspace.search == search {
                    return;
                }
                self.record_navigation_origin();
                let workspace = self.active_mut();
                workspace.search = search;
                self.library_generation += 1;
                self.library_offset = 0;
                self.library_has_more = false;
                self.refresh_library();
                self.queue_selection_neighbors();
                self.persist_workspaces();
            }
            Command::SetFolder(folder) => {
                let workspace = self.active_mut();
                if workspace.folder == folder {
                    return;
                }
                self.record_navigation_origin();
                let workspace = self.active_mut();
                workspace.folder = folder;
                self.library_generation += 1;
                self.library_offset = 0;
                self.library_has_more = false;
                self.refresh_library();
                self.queue_selection_neighbors();
                self.persist_workspaces();
            }
            Command::SetSortMode(mode) => {
                {
                    let workspace = self.active();
                    if workspace.sort_mode == mode {
                        return;
                    }
                }
                self.active_mut().sort_mode = mode.clone();
                let _ = self.settings.set(SORT_MODE, json!(mode));
                self.library_generation += 1;
                self.library_offset = 0;
                self.library_has_more = false;
                self.refresh_library();
                self.queue_selection_neighbors();
                self.persist_workspaces();
                self.settings_changed();
            }
            Command::SetFolderSortMode(mode) => {
                let workspace = self.active_mut();
                workspace.folder_sort_mode = mode.clone();
                let _ = self.settings.set(FOLDER_SORT_MODE, json!(mode));
                self.persist_workspaces();
                self.settings_changed();
                self.refresh_folders();
            }
            Command::LoadMoreLibrary => {
                self.load_more_library();
            }
            Command::LoadMoreHistory => {
                self.load_more_history();
            }
            Command::RefreshAll => {
                self.refresh_library();
                self.refresh_folders();
                self.refresh_history();
                self.refresh_counts();
            }
            Command::ScanLibrary => {
                self.request_scan("manual", true);
            }
            Command::CancelScan => self.cancel_scan("user"),
            Command::ScanBatchReady(id, generation) => {
                if scan_job_matches(self.scan.as_ref(), &id, generation) && self.active_id == id {
                    self.refresh_library_preserving_loaded();
                }
            }
            Command::ScanFinished(id, generation) => {
                if scan_job_matches(self.scan.as_ref(), &id, generation) {
                    self.scan = None;
                    self.emit_workspaces();
                    // The scan changed the manifest: refresh the library
                    // grid, counts, and explorer (mirrors the original's
                    // scan-session refresh).
                    self.library_generation += 1;
                    self.library_offset = 0;
                    self.library_has_more = false;
                    self.refresh_library();
                    self.refresh_counts();
                    self.refresh_folders();
                }
            }
            Command::SelectMedia(id) => {
                self.random_retry_media_id = None;
                self.select_media_row(id);
            }
            Command::ClearSelection => {
                self.selected = None;
                self.emit(Event::SelectedMediaChanged(None));
            }
            Command::NavigateSelection(direction) => self.navigate_selection(direction),
            Command::NavigateBack => self.navigate_back(),
            Command::NavigateForward => self.navigate_forward(),
            Command::RevealSelectedInLibrary => self.reveal_selected(),
            Command::RevealMedia(media_id) => self.reveal_media_index(media_id),
            Command::ViewHistoryPost(post_id) => self.view_history_post(post_id),
            Command::PickRandom => self.pick_random(),
            Command::RandomPicked(id) => {
                self.random_retry_media_id = Some(id);
                self.select_media_row(id);
            }
            Command::SetRandomFolderEnabled(folder, enabled) => {
                self.set_random_folder_enabled(&folder, enabled)
            }
            Command::ClearRandomFolders => {
                let workspace = self.active_mut();
                workspace.random_mode = "selected".into();
                workspace.random_folders.clear();
                let _ = self.settings.set(RANDOM_FOLDER_MODE, json!("selected"));
                let _ = self.settings.set(RANDOM_FOLDERS, json!([]));
                self.persist_workspaces();
                self.settings_changed();
                self.load_random_folder_options();
            }
            Command::SelectAllRandomFolders => {
                let workspace = self.active_mut();
                workspace.random_mode = "all".into();
                workspace.random_folders.clear();
                let _ = self.settings.set(RANDOM_FOLDER_MODE, json!("all"));
                let _ = self.settings.set(RANDOM_FOLDERS, json!([]));
                self.persist_workspaces();
                self.settings_changed();
                self.load_random_folder_options();
            }
            Command::LoadRandomFolderOptions => self.load_random_folder_options(),
            Command::ResetShuffle => self.reset_shuffle(),
            Command::EnsureThumbnail(id) => self.ensure_thumbnail(id),
            Command::ThumbnailFinished(id) => {
                self.thumb_queue.remove(&id);
            }
            Command::EnsurePreview(id) => self.ensure_preview(id),
            Command::LoadMoreFinished(library, generation, has_more, offset, loaded) => {
                // Always clear the in-flight flag (a refresh mid-flight
                // supersedes the page); apply state only when the marker
                // still belongs to the current generation.
                if library {
                    self.library_loading_more = false;
                    if generation == self.library_generation {
                        self.library_has_more = has_more;
                        if offset == 0 {
                            self.library_offset = loaded;
                        }
                    }
                } else {
                    self.history_loading_more = false;
                    if generation == self.history_generation {
                        self.history_has_more = has_more;
                        if offset == 0 {
                            self.history_offset = loaded;
                        }
                    }
                }
            }
            Command::SelectionCheckFinished => {
                self.checking = false;
            }
            Command::RecordNavigationOrigin => {
                self.record_navigation_origin();
            }
            Command::SelectionVerified(media_id, row) => {
                if self.selected.as_ref().map(|m| m.id) == Some(media_id) {
                    if self.random_retry_media_id == Some(media_id) {
                        self.random_retry_media_id = None;
                    }
                    self.selected = Some(row.clone());
                    self.emit(Event::SelectedMediaChanged(Some(row)));
                    self.ensure_thumbnail(media_id);
                    self.ensure_timeline(media_id);
                    if self
                        .settings
                        .get_string(PERFORMANCE_MODE)
                        .unwrap_or_default()
                        == "maximum"
                        && self.settings.get_bool(HOVER_PREVIEWS).unwrap_or(true)
                    {
                        self.ensure_preview(media_id);
                    }
                }
            }
            Command::SelectionVerifyFailed(media_id) => {
                if self.selected.as_ref().map(|s| s.id) == Some(media_id) {
                    self.selected = None;
                    self.emit(Event::SelectedMediaChanged(None));
                }
                if self.random_retry_media_id == Some(media_id) {
                    self.random_retry_media_id = None;
                    self.pick_random();
                }
            }
            Command::PickingFinished => {
                self.random_picking = false;
            }
            Command::PreviewFinished(id) => {
                self.preview_pending.lock().remove(&id);
            }
            Command::EnsureTimeline(id) => self.ensure_timeline(id),
            Command::TimelineFinished(id) => {
                if self.timeline_pending.as_ref().map(|(pending, _)| *pending) == Some(id) {
                    self.timeline_pending = None;
                }
            }
            Command::Publish(payload) => self.publish(payload),
            Command::CancelPublish => self.cancel_publish(),
            Command::RetryTelegram(post_id) => self.retry_telegram(post_id),
            Command::MarkXPosted(post_id, url) => self.mark_x_posted(post_id, &url),
            Command::PrepareXAgain(post_id) => self.prepare_x_again(post_id),
            Command::TrashExport(post_id) => self.trash_export(post_id),
            Command::SetHistorySearch(search) => {
                if self.history_search != search {
                    self.history_search = search;
                    self.history_generation += 1;
                    self.history_offset = 0;
                    self.history_has_more = false;
                    self.refresh_history();
                }
            }
            Command::MarkBotConfigured(username) => self.mark_bot_configured(username),
            Command::MarkPersonalConfigured(display) => self.mark_personal_configured(display),
            Command::ValidateBotToken(token) => self.validate_bot_token(token),
            Command::ValidateBotDestination(destination) => {
                self.validate_bot_destination(destination)
            }
            Command::DisconnectBot => self.disconnect_bot(),
            Command::BeginPersonalLogin(api_id, api_hash, phone) => {
                self.begin_personal_login(api_id, api_hash, phone)
            }
            Command::CompletePersonalLogin(code, password) => {
                self.complete_personal_login(code, password)
            }
            Command::LoadTelegramDialogs => self.load_telegram_dialogs(),
            Command::SignOutPersonal => self.sign_out_personal(),
            Command::SaveWorkspaceDraft(id, draft) => {
                if let Some(workspace) = self.workspaces.iter_mut().find(|w| w.id == id) {
                    workspace.draft = draft;
                }
                if !self.draft_save_pending {
                    self.draft_save_pending = true;
                    let events = self.events.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        let _ = events.send(Event::DraftsDirty);
                    });
                }
            }
            Command::FlushDrafts => {
                if self.draft_save_pending {
                    self.draft_save_pending = false;
                    self.persist_workspaces();
                }
            }
            Command::SearchSuggestions { query, scope } => {
                let db = Arc::clone(&self.db);
                let events = self.events.clone();
                std::thread::spawn(move || {
                    let results = if query.trim().is_empty() {
                        db.command_center_overview(9, &scope)
                    } else {
                        db.search_suggestions(&query, 9, &scope)
                    }
                    .unwrap_or_default();
                    let items: Vec<SearchResultItem> = results
                        .into_iter()
                        .map(|result| SearchResultItem {
                            kind: result.kind.to_string(),
                            media_id: result.media_id,
                            folder_path: result.folder_path,
                            title: result.title,
                            detail: result.detail,
                            count: result.count,
                        })
                        .collect();
                    let _ = events.send(Event::SearchResults(query, items));
                });
            }
            Command::Diagnostics => {
                let ffmpeg = ffmpeg_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Not found".into());
                let ffprobe = ffprobe_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Not found".into());
                let library_root = self.settings.get_string(LIBRARY_ROOT).unwrap_or_default();
                let library_inside_exports =
                    !library_root.is_empty() && is_within(&library_root, &self.export_dir);
                let database = self.db_path_display();
                let secret_backend = self.secrets.backend().to_string();
                let events = self.events.clone();
                std::thread::spawn(move || {
                    let (_, _, export_encoder) = cliprelay_core::media::hardware_encoder_info();
                    let _ = events.send(Event::DiagnosticsReady(Diagnostics {
                        ffmpeg,
                        ffprobe,
                        gstreamer: gpui_video_player::gst::version_string().to_string(),
                        export_encoder,
                        database,
                        secret_backend,
                        library_inside_exports,
                    }));
                });
            }
            Command::Shutdown => {
                self.shutting_down = true;
            }
        }
    }

    fn db_path_display(&self) -> String {
        cliprelay_core::paths::database_path()
            .to_string_lossy()
            .into_owned()
    }

    fn shutdown_now(&mut self) {
        if let Some(scan) = self.scan.take() {
            scan.cancel.store(true, Ordering::Relaxed);
        }
        self.processor.cancel();
        self.persist_workspaces();
    }
}

// ---- helper functions -----------------------------------------------------

/// Numeric rank for ISO-8601 index timestamps (lexicographic-compatible).
fn rank_indexed(iso: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut out = format!("{nanos:x}");
    if out.len() > 20 {
        out.truncate(20);
    }
    out
}

fn nav_from_value(value: Option<&Value>) -> Vec<NavState> {
    let Some(array) = value.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            Some(NavState {
                folder: obj
                    .get("folder")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                search: obj
                    .get("search")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                media_id: obj.get("mediaId").and_then(|v| v.as_i64()).unwrap_or(0),
            })
        })
        .collect()
}

fn nav_to_value(states: &[NavState]) -> Value {
    Value::Array(
        states
            .iter()
            .map(|state| {
                json!({
                    "folder": state.folder,
                    "search": state.search,
                    "mediaId": state.media_id,
                })
            })
            .collect(),
    )
}

#[allow(clippy::too_many_arguments)]
fn run_scan_job(
    job: &ScanJob,
    indexer: &MediaIndexer,
    _db: &Database,
    events: &flume::Sender<Event>,
    current_generation: &AtomicU64,
    verify_during_index: bool,
    thumbnails_during_index: bool,
    deep_scan: bool,
) -> Option<ScanResult> {
    let root = PathBuf::from(&job.root);
    let mut manifest_progress = |discovered: usize, name: &str| {
        let message = if name.is_empty() {
            format!("Found {} video filenames", discovered)
        } else {
            format!("Found {} video filenames: {name}", discovered)
        };
        if scan_generation_matches(current_generation, job.generation) {
            let _ = events.send(Event::ScanStateChanged(ScanState {
                active: true,
                cancelling: false,
                progress: -1.0,
                message,
                root_name: job.root.clone(),
            }));
        }
    };
    let mut sent_manifest_refresh = false;
    let mut last_manifest_refresh = Instant::now() - Duration::from_secs(1);
    let mut manifest_batch_ready = |_added: usize| {
        if (!sent_manifest_refresh || last_manifest_refresh.elapsed() >= Duration::from_millis(150))
            && scan_generation_matches(current_generation, job.generation)
        {
            sent_manifest_refresh = true;
            last_manifest_refresh = Instant::now();
            let _ = events.send(Event::ScanBatchReady(
                job.workspace_id.clone(),
                job.generation,
            ));
        }
    };
    let manifest = indexer.refresh_manifest(
        &root,
        256,
        job.include_index && verify_during_index,
        Some(&job.cancel),
        &mut manifest_progress,
        Some(&mut manifest_batch_ready),
    );
    let manifest = match manifest {
        Ok(result) => result,
        Err(_) => {
            if scan_generation_matches(current_generation, job.generation) {
                let _ = events.send(Event::ScanStateChanged(ScanState {
                    active: false,
                    cancelling: false,
                    progress: -1.0,
                    message: "Scan failed".into(),
                    root_name: job.root.clone(),
                }));
                let _ = events.send(Event::Toast(
                    ToastKind::Info,
                    "Scan stopped. Videos found so far are still available.".to_string(),
                ));
            }
            let _ = events.send(Event::ScanFinished(
                job.workspace_id.clone(),
                job.generation,
            ));
            return None;
        }
    };
    if scan_generation_matches(current_generation, job.generation) {
        log::info!(
            "library filename manifest ready: {} discovered, {} added, {} skipped",
            manifest.discovered,
            manifest.indexed,
            manifest.skipped
        );
    }
    if job.cancel.load(Ordering::Relaxed) {
        if scan_generation_matches(current_generation, job.generation) {
            let _ = events.send(Event::ScanStateChanged(ScanState {
                active: false,
                cancelling: false,
                progress: -1.0,
                message: "Scan stopped".into(),
                root_name: job.root.clone(),
            }));
            let _ = events.send(Event::Toast(
                ToastKind::Info,
                "Scan stopped. Videos found so far are still available.".to_string(),
            ));
        }
        let _ = events.send(Event::ScanFinished(
            job.workspace_id.clone(),
            job.generation,
        ));
        return Some(manifest);
    }
    if !job.include_index {
        if scan_generation_matches(current_generation, job.generation) {
            let _ = events.send(Event::ScanStateChanged(ScanState {
                active: false,
                cancelling: false,
                progress: 1.0,
                message: format!("{} filenames ready", manifest.discovered),
                root_name: job.root.clone(),
            }));
        }
        let _ = events.send(Event::ScanFinished(
            job.workspace_id.clone(),
            job.generation,
        ));
        return Some(manifest);
    }
    if scan_generation_matches(current_generation, job.generation) {
        let _ = events.send(Event::ScanStateChanged(ScanState {
            active: true,
            cancelling: false,
            progress: 0.0,
            message: "Checking library details".into(),
            root_name: job.root.clone(),
        }));
    }
    let mut progress = |completed: usize, total: usize, name: &str| {
        let verb = if verify_during_index && thumbnails_during_index {
            "Processing"
        } else if verify_during_index {
            "Checking"
        } else if thumbnails_during_index {
            "Creating thumbnail for"
        } else {
            "Adding"
        };
        let message = if total > 0 {
            format!("{verb} {} of {}: {name}", completed, total)
        } else {
            format!("{verb} {}: {name}", completed)
        };
        if scan_generation_matches(current_generation, job.generation) {
            let _ = events.send(Event::ScanStateChanged(ScanState {
                active: true,
                cancelling: false,
                progress: if total > 0 {
                    completed as f64 / total as f64
                } else {
                    -1.0
                },
                message,
                root_name: job.root.clone(),
            }));
        }
    };
    let mut last_item_refresh = Instant::now() - Duration::from_secs(1);
    let mut item_ready = |_media_id: i64| {
        if last_item_refresh.elapsed() >= Duration::from_millis(250)
            && scan_generation_matches(current_generation, job.generation)
        {
            last_item_refresh = Instant::now();
            let _ = events.send(Event::ScanBatchReady(
                job.workspace_id.clone(),
                job.generation,
            ));
        }
    };
    let scan = indexer.scan(
        &root,
        deep_scan,
        verify_during_index,
        thumbnails_during_index,
        4,
        Some(&job.cancel),
        &mut progress,
        Some(&mut item_ready),
    );
    match scan {
        Ok(result) => {
            let message = if verify_during_index {
                format!("{} videos checked", result.discovered)
            } else {
                format!("{} filenames ready", result.discovered)
            };
            if scan_generation_matches(current_generation, job.generation) {
                let _ = events.send(Event::ScanStateChanged(ScanState {
                    active: false,
                    cancelling: false,
                    progress: 1.0,
                    message,
                    root_name: job.root.clone(),
                }));
            }
            let _ = events.send(Event::ScanFinished(
                job.workspace_id.clone(),
                job.generation,
            ));
            Some(result)
        }
        Err(_) => {
            if scan_generation_matches(current_generation, job.generation) {
                let _ = events.send(Event::ScanStateChanged(ScanState {
                    active: false,
                    cancelling: false,
                    progress: -1.0,
                    message: "Scan failed".into(),
                    root_name: job.root.clone(),
                }));
                let _ = events.send(Event::Toast(
                    ToastKind::Info,
                    "Scan stopped. Videos found so far are still available.".to_string(),
                ));
            }
            let _ = events.send(Event::ScanFinished(
                job.workspace_id.clone(),
                job.generation,
            ));
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_job(
    processor: &MediaProcessor,
    db: &Database,
    events: &flume::Sender<Event>,
    export_dir: &Path,
    media: &MediaRow,
    payload: &PublishPayload,
    post_id: i64,
    settings: &HashMap<String, Value>,
    bot_token: &str,
    personal_credentials: &(String, String),
) -> PublishState {
    let mut publish = PublishState {
        active: true,
        progress: 0.0,
        stage: "Preparing video".into(),
        post_id,
        ..Default::default()
    };
    let progress: ProgressCb = {
        let events = events.clone();
        Arc::new(move |fraction, stage| {
            let _ = events.send(Event::PublishStateChanged(PublishState {
                active: true,
                progress: fraction * 0.65,
                stage: stage.to_string(),
                post_id,
                ..Default::default()
            }));
        })
    };
    let edits = normalize_edit_spec(&payload.edits);
    let export_result = processor.export(
        media,
        payload.trim_start,
        payload.trim_end,
        &payload.preset,
        payload.target_mb,
        &edits,
        &progress,
    );
    let export_result = match export_result {
        Ok(result) => result,
        Err(e) => {
            let _ = db.update_post(
                post_id,
                &PostUpdate {
                    telegram_status: if payload.telegram_enabled {
                        Some("failed".into())
                    } else {
                        None
                    },
                    x_status: if payload.x_enabled {
                        Some("failed".into())
                    } else {
                        None
                    },
                    error: Some(e.to_string()),
                    ..Default::default()
                },
            );
            publish.error = e.to_string();
            publish.stage = "Could not complete".into();
            publish.progress = 1.0;
            publish.active = false;
            let _ = events.send(Event::Toast(ToastKind::Error, e.to_string()));
            return publish;
        }
    };
    let export_id = match db.create_export(&cliprelay_core::db::ExportValues {
        media_id: media.id,
        path: export_result.path.to_string_lossy().into_owned(),
        trim_start: payload.trim_start,
        trim_end: payload.trim_end,
        preset: export_result.preset.clone(),
        target_mb: payload.target_mb,
        size_bytes: export_result.size_bytes as i64,
        duration: export_result.duration,
        is_generated: export_result.generated,
        edit_spec: payload.edits.clone(),
        cleanup_state: "kept".into(),
    }) {
        Ok(id) => id,
        Err(e) => {
            publish.error = e.to_string();
            publish.active = false;
            return publish;
        }
    };
    let _ = db.update_post(
        post_id,
        &PostUpdate {
            export_id: Some(export_id),
            ..Default::default()
        },
    );
    publish.output_path = export_result.path.to_string_lossy().into_owned();
    publish.output_size = format_bytes(export_result.size_bytes as f64);
    publish.output_encoder = export_result.encoder.clone();
    publish.hardware_accelerated = export_result.hardware_accelerated;
    let _ = events.send(Event::PublishStateChanged(publish.clone()));

    let mut completed_any = false;
    let mut delivery_errors: Vec<String> = Vec::new();
    let mut telegram_done = false;
    let mut x_done = false;

    if payload.telegram_enabled {
        let _ = events.send(Event::PublishStateChanged(PublishState {
            active: true,
            progress: 0.68,
            stage: "Sending to Telegram".into(),
            post_id,
            ..publish.clone()
        }));
        let _ = db.add_attempt(post_id, "telegram", "started", "", "", false);
        let telegram_progress: ProgressCb = {
            let events = events.clone();
            Arc::new(move |fraction, stage| {
                let _ = events.send(Event::PublishStateChanged(PublishState {
                    active: true,
                    progress: 0.68 + fraction * 0.27,
                    stage: stage.to_string(),
                    post_id,
                    ..Default::default()
                }));
            })
        };
        let outcome = send_telegram(
            &payload.telegram_mode,
            &payload.telegram_destination,
            &export_result.path,
            &payload.telegram_caption,
            bot_token,
            personal_credentials,
            &telegram_progress,
        );
        match outcome {
            Ok(delivery) => {
                let _ = db.update_post(
                    post_id,
                    &PostUpdate {
                        telegram_status: Some("sent".into()),
                        telegram_message_id: Some(delivery.message_id.clone()),
                        telegram_message_link: Some(delivery.link.clone()),
                        ..Default::default()
                    },
                );
                let _ = db.add_attempt(
                    post_id,
                    "telegram",
                    "sent",
                    &delivery.detail,
                    &delivery.message_id,
                    true,
                );
                completed_any = true;
                telegram_done = true;
            }
            Err(e) => {
                let _ = db.update_post(
                    post_id,
                    &PostUpdate {
                        telegram_status: Some("failed".into()),
                        ..Default::default()
                    },
                );
                let _ = db.add_attempt(post_id, "telegram", "failed", &e.to_string(), "", true);
                delivery_errors.push(format!("Telegram: {e}"));
            }
        }
    } else {
        telegram_done = true;
    }

    if payload.x_enabled {
        let _ = events.send(Event::PublishStateChanged(PublishState {
            active: true,
            progress: 0.96,
            stage: "Opening X composer".into(),
            post_id,
            ..publish.clone()
        }));
        match XAssistant::prepare(&export_result.path, &payload.x_caption) {
            Ok(()) => {
                let _ = db.update_post(
                    post_id,
                    &PostUpdate {
                        x_status: Some("prepared".into()),
                        ..Default::default()
                    },
                );
                let _ = db.add_attempt(
                    post_id,
                    "x",
                    "prepared",
                    "Caption prefilled and video copied",
                    "",
                    true,
                );
                completed_any = true;
                x_done = true;
            }
            Err(e) => {
                let _ = db.update_post(
                    post_id,
                    &PostUpdate {
                        x_status: Some("failed".into()),
                        ..Default::default()
                    },
                );
                let _ = db.add_attempt(post_id, "x", "failed", &e.to_string(), "", true);
                delivery_errors.push(format!("X: {e}"));
            }
        }
    } else {
        x_done = true;
    }

    if completed_any {
        let _ = db.increment_posted(media.id);
    }
    if !delivery_errors.is_empty() {
        let detail = delivery_errors.join(" · ");
        let _ = db.update_post(
            post_id,
            &PostUpdate {
                error: Some(detail.clone()),
                ..Default::default()
            },
        );
        publish.error = detail.clone();
        publish.stage = "Completed with an issue".into();
        publish.progress = 1.0;
        publish.active = false;
        let _ = events.send(Event::Toast(
            ToastKind::Warning,
            "One destination needs attention; the other completed steps were kept.".to_string(),
        ));
    } else {
        publish.stage = "Post prepared".into();
        publish.progress = 1.0;
        publish.active = false;
        let message = if payload.telegram_enabled && payload.x_enabled {
            "Telegram and X steps are ready.".to_string()
        } else {
            "Post step completed.".to_string()
        };
        let _ = events.send(Event::Toast(ToastKind::Success, message.to_string()));
    }

    // Cleanup policy.
    let policy = if payload.cleanup_policy.is_empty() {
        settings
            .get(CLEANUP_POLICY)
            .and_then(|v| v.as_str())
            .unwrap_or("keep")
            .to_string()
    } else {
        payload.cleanup_policy.clone()
    };
    if export_result.generated {
        let trash = match policy.as_str() {
            "after_complete" => telegram_done && x_done,
            "after_telegram" => !payload.x_enabled && telegram_done,
            _ => false,
        };
        if trash {
            let _ = move_generated_to_trash(&export_result.path, export_dir, true);
            let _ = db.mark_export_cleanup(export_id, "trashed");
        }
    }
    publish
}

#[allow(clippy::too_many_arguments)]
fn send_telegram(
    mode: &str,
    destination: &str,
    path: &Path,
    caption: &str,
    bot_token: &str,
    personal_credentials: &(String, String),
    progress: &ProgressCb,
) -> Result<TelegramDelivery> {
    if mode == "personal" {
        let (api_id, api_hash) = personal_credentials;
        let api_id: i32 = api_id.parse().unwrap_or(0);
        let personal = PersonalTelegram::new(SecretStore::new(None));
        personal
            .service
            .send_video(
                api_id,
                api_hash,
                &personal.secrets.get("telegram_personal_session", ""),
                destination,
                path,
                caption,
                progress.clone(),
            )
            .map(|(delivery, session)| {
                let _ = personal.secrets.set("telegram_personal_session", &session);
                delivery
            })
    } else {
        let mut bot = TelegramBotService::new(SecretStore::new(None));
        let _ = bot_token;
        bot.send_video(path, caption, destination, &mut |fraction, stage| {
            progress(fraction, stage);
        })
    }
}

/// Controller-side helpers exposed for the UI: value formatting.
#[allow(dead_code)]
pub fn format_duration_label(seconds: f64) -> String {
    if seconds <= 0.0 {
        "Unchecked".into()
    } else {
        format_duration(seconds)
    }
}

#[allow(dead_code)]
pub fn resolution_label(width: i64, height: i64) -> String {
    if width > 0 && height > 0 {
        format!("{width}×{height}")
    } else {
        "Unchecked".into()
    }
}

/// Size-target resolution shared by the publish path (mirrors the
/// original's `_target_for`).
fn build_random_options(
    rows: &[RandomFolder],
    selected_direct: &std::collections::HashSet<String>,
    random_mode: &str,
) -> (Vec<RandomFolderOption>, String, usize, bool, bool) {
    let mut options: Vec<RandomFolderOption> = Vec::new();
    for row in rows {
        let depth = if row.folder.is_empty() {
            0
        } else {
            row.folder.split('/').count()
        };
        let parent = if row.folder.is_empty() {
            String::new()
        } else {
            row.folder
                .rsplit_once('/')
                .map(|(p, _)| p.to_string())
                .unwrap_or_default()
        };
        let name = row
            .folder
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("Library root only")
            .to_string();
        let detail = if row.folder.is_empty() {
            "Files directly inside the chosen library".to_string()
        } else {
            row.folder.split('/').collect::<Vec<_>>().join("  /  ")
        };
        options.push(RandomFolderOption {
            folder: row.folder.clone(),
            name,
            detail,
            video_count: row.count,
            direct_video_count: row.direct_count,
            selection_state: 0,
            depth,
            has_children: false,
            expanded: depth <= 2,
            parent,
        });
    }
    // Children + tri-state: 2 = all direct descendants selected,
    // 1 = some, 0 = none.
    let mut direct_selected: std::collections::HashMap<String, bool> =
        std::collections::HashMap::new();
    for option in &options {
        if option.direct_video_count > 0 {
            direct_selected.insert(
                option.folder.clone(),
                selected_direct.contains(&option.folder),
            );
        }
    }
    let child_map: std::collections::HashMap<String, Vec<String>> = {
        let mut map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for option in &options {
            map.entry(option.parent.clone())
                .or_default()
                .push(option.folder.clone());
        }
        map
    };
    let mut descendant_direct: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for option in &options {
        let mut descendants: Vec<String> = Vec::new();
        let mut stack = vec![option.folder.clone()];
        while let Some(current) = stack.pop() {
            if let Some(children) = child_map.get(&current) {
                for child in children {
                    // The root row ("") can list itself as its own parent;
                    // skip self-references to keep the walk finite.
                    if child == &current {
                        continue;
                    }
                    if let Some(child_option) = options.iter().find(|o| &o.folder == child) {
                        if child_option.direct_video_count > 0 {
                            descendants.push(child.clone());
                        }
                    }
                    stack.push(child.clone());
                }
            }
        }
        descendant_direct.insert(option.folder.clone(), descendants);
    }
    for option in &mut options {
        option.has_children = child_map
            .get(&option.folder)
            .map(|children| !children.is_empty())
            .unwrap_or(false);
        if option.direct_video_count > 0 {
            // A direct folder's state also reflects its subtree (mirrors
            // the original: itself + every direct-video descendant):
            // 2 = all selected, 1 = some, 0 = none.
            let mut paths = descendant_direct
                .get(&option.folder)
                .cloned()
                .unwrap_or_default();
            if !paths.contains(&option.folder) {
                paths.push(option.folder.clone());
            }
            let selected = paths
                .iter()
                .filter(|p| direct_selected.get(*p).copied().unwrap_or(false))
                .count();
            option.selection_state = if selected == paths.len() && !paths.is_empty() {
                2
            } else if selected > 0 {
                1
            } else {
                0
            };
            continue;
        }
        let states: Vec<bool> = descendant_direct
            .get(&option.folder)
            .map(|descendants| {
                descendants
                    .iter()
                    .filter_map(|folder| direct_selected.get(folder).copied())
                    .collect()
            })
            .unwrap_or_default();
        if !states.is_empty() {
            let selected = states.iter().filter(|s| **s).count();
            option.selection_state = if selected == states.len() {
                2
            } else if selected > 0 {
                1
            } else {
                0
            };
        }
    }
    let selected_count = options
        .iter()
        .filter(|o| o.selection_state == 2 && o.direct_video_count > 0)
        .count();
    let all_selected = random_mode != "selected";
    let summary = {
        let direct_folders: Vec<&RandomFolderOption> = options
            .iter()
            .filter(|o| o.selection_state == 2 && o.direct_video_count > 0)
            .collect();
        if random_mode != "selected" {
            "All folders".to_string()
        } else if direct_folders.is_empty() {
            "No folders".to_string()
        } else if direct_folders.len() == 1 {
            let folder = &direct_folders[0].folder;
            if folder.is_empty() {
                "Library root".to_string()
            } else {
                folder
                    .rsplit('/')
                    .next()
                    .unwrap_or("Library root")
                    .to_string()
            }
        } else {
            format!("{} folders", direct_folders.len())
        }
    };
    (
        options,
        summary,
        selected_count,
        all_selected,
        selected_count > 0,
    )
}

fn resolve_target(
    preset: &str,
    target_mb: f64,
    telegram_enabled: bool,
    x_enabled: bool,
    telegram_mode: &str,
    x_limit_mb: f64,
) -> f64 {
    let x_limit = x_limit_mb * 0.98;
    match preset {
        "fit_bot" => 49.0,
        "fit_x" => x_limit,
        "fit_both" => {
            let mut limits = Vec::new();
            if telegram_enabled {
                limits.push(if telegram_mode == "bot" { 49.0 } else { 1950.0 });
            }
            if x_enabled {
                limits.push(x_limit);
            }
            if limits.is_empty() {
                target_mb
            } else {
                limits.into_iter().fold(f64::MAX, f64::min)
            }
        }
        _ => target_mb,
    }
}

/// Compare two library roots after resolving symlinks (/tmp -> /private/tmp),
/// mirroring the original's Path.resolve() comparisons.
fn same_root(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let resolve = |p: &str| {
        std::fs::canonicalize(p)
            .unwrap_or_else(|_| std::path::Path::new(p).to_path_buf())
            .to_string_lossy()
            .into_owned()
    };
    resolve(a) == resolve(b)
}

fn reveal_index(db: &Database, media_id: i64, folder: &str, sort_mode: &str) -> Option<i64> {
    db.media_index(media_id, "", folder, sort_mode)
        .ok()
        .flatten()
        .map(|index| index as i64)
}

#[cfg(test)]
mod controller_tests {
    use super::{
        build_random_options, resolve_target, reveal_index, scan_generation_matches,
        scan_job_matches, spawn_controller, ScanJob,
    };
    use crate::state::{Command, Event};
    use cliprelay_core::db::{Database, ManifestEntry, RandomFolder};
    use std::collections::HashSet;
    use std::sync::atomic::AtomicU64;
    use std::sync::{atomic::AtomicBool, Arc};

    #[test]
    fn stale_scan_events_are_rejected() {
        let job = ScanJob {
            cancel: Arc::new(AtomicBool::new(false)),
            workspace_id: "workspace-new".into(),
            root: "/library/new".into(),
            include_index: false,
            generation: 7,
        };
        assert!(scan_job_matches(Some(&job), "workspace-new", 7));
        assert!(!scan_job_matches(Some(&job), "workspace-old", 7));
        assert!(!scan_job_matches(Some(&job), "workspace-new", 6));
        assert!(!scan_job_matches(None, "workspace-new", 7));

        let latest = AtomicU64::new(7);
        assert!(scan_generation_matches(&latest, 7));
        assert!(!scan_generation_matches(&latest, 6));
    }

    #[test]
    fn reveal_index_finds_a_nested_source_in_library_order() {
        let directory = tempfile::tempdir().unwrap();
        let db = Database::open(directory.path().join("reveal.sqlite3")).unwrap();
        let root = directory
            .path()
            .join("library")
            .to_string_lossy()
            .into_owned();
        let entries = ["c.mp4", "a.mp4", "b.mp4"]
            .into_iter()
            .map(|name| ManifestEntry {
                root_path: root.clone(),
                path: format!("{root}/nested/{name}"),
                name: name.into(),
                relative_path: format!("nested/{name}"),
                folder: "nested".into(),
                size_bytes: 1,
                mtime: 1.0,
            })
            .collect::<Vec<_>>();
        db.upsert_manifest_batch(&entries).unwrap();
        db.activate_root(Some(&root)).unwrap();
        let target = db
            .get_media_by_path(&format!("{root}/nested/b.mp4"))
            .unwrap()
            .unwrap();

        assert_eq!(reveal_index(&db, target.id, "nested", "name"), Some(1));
        assert_eq!(reveal_index(&db, target.id, "other", "name"), None);
    }

    fn folder(path: &str, total: i64, direct: i64) -> RandomFolder {
        RandomFolder {
            folder: path.to_string(),
            count: total,
            direct_count: direct,
        }
    }

    #[test]
    fn random_tree_tri_states() {
        let rows = vec![
            folder("", 10, 0),
            folder("clips", 10, 3),
            folder("clips/shorts", 4, 4),
            folder("clips/long", 6, 6),
        ];
        // Nothing selected.
        let selected = HashSet::new();
        let (options, summary, count, all_selected, has) =
            build_random_options(&rows, &selected, "selected");
        assert_eq!(options.len(), 4);
        let root = options.iter().find(|o| o.folder.is_empty()).unwrap();
        assert_eq!(root.selection_state, 0);
        assert_eq!(summary, "No folders");
        assert!(!all_selected && !has);
        assert_eq!(count, 0);
        // One direct folder selected -> parent is partial (1).
        let mut selected = HashSet::new();
        selected.insert("clips/shorts".to_string());
        let (options, summary, count, all_selected, has) =
            build_random_options(&rows, &selected, "selected");
        let parent = options.iter().find(|o| o.folder == "clips").unwrap();
        assert_eq!(parent.selection_state, 1);
        let shorts = options.iter().find(|o| o.folder == "clips/shorts").unwrap();
        assert_eq!(shorts.selection_state, 2);
        assert_eq!(summary, "shorts");
        assert!(!all_selected && has);
        assert_eq!(count, 1);
        // "clips" itself carries direct videos, so selecting only its
        // children leaves it partial (1) — matching the original's
        // selected-count math.
        let mut selected = HashSet::new();
        selected.insert("clips/shorts".to_string());
        selected.insert("clips/long".to_string());
        let (options, _summary, count, _all_selected, _has) =
            build_random_options(&rows, &selected, "selected");
        let parent = options.iter().find(|o| o.folder == "clips").unwrap();
        assert_eq!(parent.selection_state, 1);
        assert_eq!(count, 2);
        // Selecting the folder itself as well completes the subtree (2).
        selected.insert("clips".to_string());
        let (options, summary, count, all_selected, has) =
            build_random_options(&rows, &selected, "all");
        let parent = options.iter().find(|o| o.folder == "clips").unwrap();
        assert_eq!(parent.selection_state, 2);
        assert_eq!(summary, "All folders");
        assert!(all_selected && has);
        assert_eq!(count, 3);
    }

    #[test]
    fn random_folder_options_can_be_reloaded_after_the_first_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("random-reload.sqlite3");
        drop(Database::open(&database_path).unwrap());
        let (events, incoming) = flume::unbounded();
        let controller = spawn_controller(Some(database_path), events);

        for _ in 0..2 {
            controller.send(Command::LoadRandomFolderOptions).unwrap();
            loop {
                let event = incoming
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .expect("controller should emit each requested random-folder snapshot");
                if matches!(event, Event::RandomFoldersChanged(..)) {
                    break;
                }
            }
        }

        let _ = controller.send(Command::Shutdown);
    }

    #[test]
    fn fit_presets_enforce_limits() {
        // Bot limit is 49 MB regardless of the custom-size field.
        assert_eq!(
            resolve_target("fit_bot", 0.0, true, false, "bot", 512.0),
            49.0
        );
        // X limit is 98% of the configured cap.
        assert_eq!(
            resolve_target("fit_x", 0.0, false, true, "bot", 100.0),
            98.0
        );
        // Fit both: the tighter of the two destinations wins.
        assert_eq!(
            resolve_target("fit_both", 0.0, true, true, "bot", 20.0),
            19.6
        );
        assert_eq!(
            resolve_target("fit_both", 0.0, true, true, "bot", 512.0),
            49.0
        );
        // Personal Telegram allows 1950 MB.
        assert_eq!(
            resolve_target("fit_both", 0.0, true, true, "personal", 512.0),
            501.76
        );
        // No destinations: fall back to the custom target.
        assert_eq!(
            resolve_target("fit_both", 33.0, false, false, "bot", 512.0),
            33.0
        );
        // Other presets pass the target through untouched.
        assert_eq!(
            resolve_target("balanced", 0.0, true, true, "bot", 512.0),
            0.0
        );
        assert_eq!(
            resolve_target("custom", 120.0, true, true, "bot", 512.0),
            120.0
        );
    }
}
