//! ClipRelay — GPUI desktop app.
//!
//! Single root view (`App`) owns all UI state; a background controller
//! thread (`controller`) owns services and streams `Event`s back here.

mod controller;
use crate::settings_import::*;
mod settings_import {
    pub use cliprelay_core::settings::*;
}
mod history;
mod icons;
mod library;
mod prepare;
mod settings_page;
mod state;
mod theme;
mod widgets;

use crate::controller::spawn_controller;
use crate::state::*;
use crate::theme::*;
use crate::widgets::*;
use cliprelay_core::db::MediaRow;

use cliprelay_core::telegram::DialogInfo;
use gpui::*;
use gpui_video_player::{Video, VideoOptions};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

actions!(
    cliprelay,
    [FocusNext, FocusPrevious, Activate, ActivateSpace]
);

#[derive(Clone)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
    pub shown_at: std::time::Instant,
}

pub struct App {
    pub controller: flume::Sender<Command>,
    pub event_tx: flume::Sender<Event>,
    pub page: Page,
    pub theme_mode: ThemeMode,
    pub ui_scale: f32,
    pub sidebar_collapsed: bool,
    pub density: String,
    pub prepare_expanded: bool,
    pub settings: HashMap<String, serde_json::Value>,
    pub counts: (i64, i64, i64),
    pub workspaces: Vec<WorkspaceInfo>,
    pub active_workspace_index: usize,
    pub scan: ScanState,
    pub publish: PublishState,
    pub telegram: TelegramState,
    pub dialogs: Vec<DialogInfo>,
    pub selected: Option<MediaRow>,
    pub checking: bool,
    pub timeline_loading: bool,
    pub library: LibraryPage,
    pub history: HistoryPage,
    pub folders: Vec<FolderNode>,
    pub folders_expanded: HashMap<String, bool>,
    pub random_options: Vec<RandomFolderOption>,
    pub random_summary: String,
    pub random_selected: usize,
    pub random_all_selected: bool,
    pub random_has_selection: bool,
    pub random_popup_open: bool,
    pub random_filter: String,
    pub random_selected_only: bool,
    pub random_expanded: std::collections::HashSet<String>,
    pub toasts: Vec<Toast>,
    next_toast_id: u64,
    pub diagnostics: Diagnostics,
    pub search_text: String,
    pub show_folders: bool,
    pub active_folder: String,
    pub hovered_tiles: HashMap<i64, bool>,
    pub preview_video: Option<(i64, Video)>,
    pub preview_video_loading: Option<(i64, u64)>,
    preview_video_load_task: Option<Task<()>>,
    prepare_video_generation: u64,
    prepare_video_load_task: Option<Task<()>>,
    pub prepare_video_loading: bool,
    pub prepare_video_error: Option<String>,
    pub active_preview_id: i64,
    pub reveal_request: Option<(String, i64, i64)>,
    pub pending_draft: Option<PrepareDraft>,
    pub prepare: prepare::PrepareState,
    pub settings_page: settings_page::SettingsUiState,
    pub history_search: String,
    history_search_generation: u64,
    history_search_task: Option<Task<()>>,
    pub history_more_menu_post: Option<i64>,
    pub history_more_menu_y: f32,
    history_more_menu_closed_at: std::time::Instant,
    menu_closed_at: std::time::Instant,
    pub command_results: Vec<SearchResultItem>,
    pub command_query: String,
    pub command_scope: String,
    pub command_selected: usize,
    pub command_open: bool,
    pub command_searching: bool,
    command_search_generation: u64,
    command_search_task: Option<Task<()>>,
    pub random_picking: bool,
    pub closed_count: usize,
    pub explorer_focus: bool,
    pub explorer_selected: String,
    pub random_tree_cursor: usize,
    pub random_loading: bool,
    pub window_title: String,
    pub capture_after_frames: u32,
    pub capture_after_target: u32,
    pub capture_started_at: std::time::Instant,
    pub capture_path: Option<std::path::PathBuf>,
    pub saved_bounds: (f32, f32),
    pub last_history_scroll_y: f32,
    pub reveal_target_row: Option<usize>,
    pub library_scroll: gpui::UniformListScrollHandle,
    pub history_scroll: gpui::ScrollHandle,
    pub tab_scroll: gpui::ScrollHandle,
    pub settings_scroll: gpui::ScrollHandle,
    pub thumbnail_states: HashMap<i64, String>,
    pub thumbnail_requested: std::collections::HashSet<i64>,
    pub preview_hover_generation: u64,
    pub library_scroll_pause_until: std::time::Instant,
    pub theme: crate::theme::Theme,
    pub window_size: (f32, f32),
    pub fields: HashMap<String, widgets::FieldState>,
    pub focused_field: Option<String>,
    platform_input_bounds: Option<Bounds<Pixels>>,
    platform_input_focus: Option<FocusHandle>,
    focus_handle: FocusHandle,
    pub open_combos: std::collections::HashSet<String>,
    pub key_captured: bool,
    pub workspace_menu_open: bool,
    pub workspace_menu_target: usize,
    pub renaming_workspace: Option<usize>,
    pub sort_menu_open: bool,
    pub pending_close_workspace: Option<usize>,
    pub activity_open: bool,
}

impl App {
    pub fn navigate_to(&mut self, page: Page, cx: &mut Context<Self>) {
        if page != Page::Library {
            self.random_popup_open = false;
            self.sort_menu_open = false;
            self.workspace_menu_open = false;
            self.activity_open = false;
            self.stop_hover_preview(None, cx);
        }
        if page == Page::Settings && self.page != Page::Settings {
            self.command(Command::Diagnostics);
        }
        self.page = page;
        cx.notify();
    }

    pub fn new(cx: &mut Context<Self>) -> Self {
        let (event_tx, event_rx) = flume::unbounded::<Event>();
        let controller = spawn_controller(None, event_tx.clone());
        // `--library PATH` overrides the stored library root on launch.
        if let Ok(library) = std::env::var("CLIPRELAY_LIBRARY") {
            if !library.trim().is_empty() {
                let _ = controller.send(Command::ChooseLibrary(library.trim().to_string()));
            }
        }
        // Prefill the setting-backed inputs (mirrors the original's binding
        // of these fields to the settings values).
        let query_at_boot = std::env::var("CLIPRELAY_QUERY").unwrap_or_default();
        let boot_toast: Option<(ToastKind, String)> =
            std::env::var("CLIPRELAY_BOOT_TOAST").ok().and_then(|v| {
                let (kind, text) = v.split_once(':')?;
                let kind = match kind {
                    "info" => ToastKind::Info,
                    "success" => ToastKind::Success,
                    _ => ToastKind::Error,
                };
                Some((kind, text.to_string()))
            });
        let mut fields = std::collections::HashMap::new();
        if !query_at_boot.is_empty() {
            fields.insert(
                "command-center".to_string(),
                crate::widgets::FieldState {
                    text: query_at_boot.clone(),
                    caret: query_at_boot.chars().count(),
                    committed: true,
                    marked_range: None,
                },
            );
        }
        if let Ok(settings) = boot_settings() {
            let seeds = [
                ("tg-destination", "telegram_destination"),
                ("tg-destination-field", "telegram_destination"),
                ("tg-api-id", "telegram_api_id"),
                ("tg-phone", "telegram_phone"),
                ("x-limit", "x_limit_mb"),
            ];
            for (id, key) in seeds {
                let text = settings
                    .get(key)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .or_else(|| {
                        settings
                            .get(key)
                            .and_then(|v| v.as_f64().map(|n| format!("{}", n as i64)))
                    })
                    .unwrap_or_default();
                if !text.is_empty() {
                    let caret = text.chars().count();
                    fields.insert(
                        id.to_string(),
                        crate::widgets::FieldState {
                            text,
                            caret,
                            committed: true,
                            marked_range: None,
                        },
                    );
                }
            }
        }
        let mut prepare_state = prepare::PrepareState::default();
        if let Ok(settings) = boot_settings() {
            if let Some(value) = settings
                .get("telegram_destination")
                .and_then(|v| v.as_str())
            {
                prepare_state.destination = value.to_string();
            }
        }
        // Dev-only: CLIPRELAY_PAGE=history|settings chooses the initial page
        // (used by screenshot-driven UI sweeps).
        let initial_page = match std::env::var("CLIPRELAY_PAGE").as_deref() {
            Ok("history") => Page::History,
            Ok("settings") => Page::Settings,
            _ => Page::Library,
        };
        let open_command_at_boot = std::env::var("CLIPRELAY_OPEN_COMMAND").is_ok();
        let open_random_at_boot = std::env::var("CLIPRELAY_OPEN_RANDOM").is_ok();
        if open_random_at_boot {
            controller.send(Command::LoadRandomFolderOptions).ok();
        }
        let open_sort_at_boot = std::env::var("CLIPRELAY_OPEN_SORT").is_ok();
        let open_activity_at_boot = std::env::var("CLIPRELAY_OPEN_ACTIVITY").is_ok();
        let open_workspace_menu_at_boot = std::env::var("CLIPRELAY_OPEN_WORKSPACE_MENU").is_ok();
        let settings_scroll_boot: f32 = std::env::var("CLIPRELAY_SETTINGS_SCROLL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        let app = Self {
            event_tx: event_tx.clone(),
            controller,
            page: initial_page,
            random_popup_open: open_random_at_boot,
            sort_menu_open: open_sort_at_boot,
            activity_open: open_activity_at_boot,
            workspace_menu_open: open_workspace_menu_at_boot,
            command_open: open_command_at_boot,
            focused_field: if open_command_at_boot {
                Some("command-center".to_string())
            } else {
                None
            },
            focus_handle: cx.focus_handle(),
            theme_mode: ThemeMode::Relay,
            ui_scale: 1.0,
            sidebar_collapsed: false,
            density: "default".into(),
            prepare_expanded: false,
            settings: HashMap::new(),
            counts: (0, 0, 0),
            workspaces: Vec::new(),
            active_workspace_index: 0,
            scan: ScanState::default(),
            publish: PublishState::default(),
            telegram: TelegramState::default(),
            dialogs: Vec::new(),
            selected: None,
            checking: false,
            timeline_loading: false,
            library: LibraryPage::default(),
            history: HistoryPage::default(),
            folders: Vec::new(),
            folders_expanded: HashMap::new(),
            random_options: Vec::new(),
            random_summary: "All folders".into(),
            random_selected: 0,
            random_all_selected: true,
            random_has_selection: false,

            random_filter: String::new(),
            random_selected_only: false,
            random_expanded: std::collections::HashSet::new(),
            toasts: boot_toast
                .map(|(kind, text)| {
                    vec![Toast {
                        id: 1,
                        kind,
                        message: text,
                        shown_at: std::time::Instant::now(),
                    }]
                })
                .unwrap_or_default(),
            next_toast_id: 2,
            diagnostics: Diagnostics::default(),
            search_text: String::new(),
            show_folders: true,
            active_folder: String::new(),
            hovered_tiles: HashMap::new(),
            preview_video: None,
            preview_video_loading: None,
            preview_video_load_task: None,
            prepare_video_generation: 0,
            prepare_video_load_task: None,
            prepare_video_loading: false,
            prepare_video_error: None,
            active_preview_id: 0,
            reveal_request: None,
            pending_draft: None,
            prepare: prepare_state,
            settings_page: settings_page::SettingsUiState::default(),
            history_search: String::new(),
            history_search_generation: 0,
            history_search_task: None,
            history_more_menu_post: None,
            history_more_menu_y: 0.0,
            history_more_menu_closed_at: std::time::Instant::now(),
            menu_closed_at: std::time::Instant::now(),
            command_results: Vec::new(),
            command_query: query_at_boot.clone(),
            command_scope: "all".into(),
            command_selected: 0,

            command_searching: false,
            command_search_generation: 0,
            command_search_task: None,
            random_picking: false,
            closed_count: 0,
            explorer_focus: false,
            explorer_selected: String::new(),
            random_tree_cursor: 0,
            random_loading: false,
            window_title: String::new(),
            capture_after_frames: 0,
            capture_after_target: std::env::var("CLIPRELAY_CAPTURE_AFTER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(220),
            capture_started_at: std::time::Instant::now(),
            capture_path: std::env::var("CLIPRELAY_CAPTURE")
                .ok()
                .map(std::path::PathBuf::from),
            saved_bounds: (0.0, 0.0),
            last_history_scroll_y: 0.0,
            reveal_target_row: None,
            library_scroll: gpui::UniformListScrollHandle::new(),
            history_scroll: gpui::ScrollHandle::new(),
            tab_scroll: gpui::ScrollHandle::new(),
            settings_scroll: gpui::ScrollHandle::new(),
            thumbnail_states: HashMap::new(),
            thumbnail_requested: std::collections::HashSet::new(),
            preview_hover_generation: 0,
            library_scroll_pause_until: std::time::Instant::now(),
            theme: crate::theme::Theme::relay(),
            window_size: (1460.0, 900.0),
            fields,
            platform_input_bounds: None,
            platform_input_focus: None,

            open_combos: std::collections::HashSet::new(),
            key_captured: false,
            workspace_menu_target: 0,
            renaming_workspace: None,

            pending_close_workspace: None,
        };

        if app.page == Page::Settings {
            app.command(Command::Diagnostics);
        }

        // Drive background events directly through GPUI's foreground executor.
        // This keeps model mutation out of render and removes the old permanent
        // 10 Hz redraw that existed only to drain a mutex-backed queue.
        cx.spawn(async move |this, cx| {
            while let Ok(event) = event_rx.recv_async().await {
                if this.update(cx, |app, cx| app.on_event(event, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |this, cx| loop {
            smol::Timer::after(Duration::from_millis(100)).await;
            if this
                .update(cx, |app, cx| app.on_event(Event::Tick, cx))
                .is_err()
            {
                break;
            }
        })
        .detach();

        let open_command_at_boot_2 = open_command_at_boot && !query_at_boot.is_empty();
        if initial_page != Page::Library || settings_scroll_boot > 0.0 || open_command_at_boot_2 {
            let settings_scroll = app.settings_scroll.clone();
            cx.spawn(
                async move |this: WeakEntity<crate::App>, cx: &mut AsyncApp| {
                    // Apply after the boot restore settles (the restore may
                    // navigate to the Library while revealing the selection).
                    smol::Timer::after(std::time::Duration::from_millis(2500)).await;
                    if initial_page != Page::Library {
                        if let Some(this) = this.upgrade() {
                            this.update(
                                cx,
                                |app: &mut crate::App, _cx: &mut gpui::Context<crate::App>| {
                                    app.page = initial_page;
                                    _cx.notify();
                                },
                            )
                            .ok();
                        }
                    }
                    if open_command_at_boot_2 {
                        if let Some(this) = this.upgrade() {
                            this.update(
                                cx,
                                |app: &mut crate::App, _cx: &mut gpui::Context<crate::App>| {
                                    app.command_open = true;
                                    app.open_command_center(_cx);
                                },
                            )
                            .ok();
                        }
                    }
                    if settings_scroll_boot > 0.0 {
                        // Wait for the settings page to render once so the
                        // scroll handle is bound to its container.
                        smol::Timer::after(std::time::Duration::from_millis(400)).await;
                        // Offset is negative for downward scroll (distance from
                        // the container top to the content top).
                        settings_scroll.set_offset(point(px(0.0), px(-settings_scroll_boot)));
                    }
                },
            )
            .detach();
        }

        app
    }

    pub fn command(&self, command: Command) {
        let _ = self.controller.send(command);
    }

    /// Save the active draft, then switch to the workspace at `index`.
    pub fn activate_workspace_at(&mut self, index: usize, cx: &mut Context<Self>) {
        self.save_draft();
        self.command(Command::ActivateWorkspace(index));
        cx.notify();
    }

    fn on_event(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::SettingsChanged(settings) => {
                if !settings.is_empty() {
                    self.settings = settings;
                    self.apply_settings();
                    if !self
                        .settings
                        .get(HOVER_PREVIEWS)
                        .and_then(|value| value.as_bool())
                        .unwrap_or(true)
                    {
                        self.stop_hover_preview(None, cx);
                    }
                    cx.notify();
                }
            }
            Event::CountsChanged(media, posts, unseen) => {
                self.counts = (media, posts, unseen);
                cx.notify();
            }
            Event::ScanStateChanged(state) => {
                self.scan = state;
                cx.notify();
            }
            Event::PublishStateChanged(state) => {
                self.publish = state;
                self.prepare.on_publish_state(&self.publish);
                cx.notify();
            }
            Event::TelegramStateChanged(state) => {
                self.telegram = state;
                cx.notify();
            }
            Event::MarkBotConfigured(username) => {
                self.command(Command::MarkBotConfigured(username));
            }
            Event::MarkPersonalConfigured(display) => {
                self.command(Command::MarkPersonalConfigured(display));
            }
            Event::TelegramDialogsChanged(dialogs) => {
                self.dialogs = dialogs;
                cx.notify();
            }
            Event::SelectedMediaChanged(row) => {
                // Keep the freshly selected tile inside the viewport; when it
                // lives beyond the loaded page, ask the controller for its
                // index and chase it across pages.
                if let Some(row) = &row {
                    if let Some(index) = self.library.rows.iter().position(|r| r.id == row.id) {
                        self.reveal_target_row = Some(index);
                        self.apply_reveal_scroll();
                    } else if row.folder == self.active_folder && self.search_text.is_empty() {
                        self.command(Command::RevealMedia(row.id));
                    }
                }
                let next_media_id = row.as_ref().map(|row| row.id).unwrap_or(0);
                let next_media_path = row.as_ref().map(|row| PathBuf::from(&row.path));
                let media_source_changed = self.prepare.media_id != next_media_id
                    || self.prepare.media_path.as_ref() != next_media_path.as_ref();
                if media_source_changed {
                    self.prepare_video_generation = self.prepare_video_generation.wrapping_add(1);
                    self.prepare_video_load_task.take();
                    if let Some(video) = self.prepare.video.take() {
                        Self::retire_video(video, cx);
                    }
                }
                self.selected = row;
                match self
                    .selected
                    .as_ref()
                    .map(|row| (row.id, row.duration, PathBuf::from(&row.path)))
                {
                    Some((media_id, duration, media_path)) => {
                        self.prepare.on_media_changed(media_id, duration);
                        for id in ["caption-shared", "caption-tg", "caption-x"] {
                            if let Some(field) = self.fields.get_mut(id) {
                                field.text.clear();
                                field.caret = 0;
                            }
                        }
                        if media_source_changed {
                            self.prepare.set_media_path(media_path.clone());
                            self.start_prepare_video(media_id, media_path, cx);
                        }
                        self.thumbnail_states
                            .entry(media_id)
                            .or_insert_with(|| "queued".into());
                        self.command(Command::EnsureThumbnail(media_id));
                        self.command(Command::EnsureTimeline(media_id));
                        if duration > 0.0 {
                            self.prepare.seek(0.0, duration);
                            // Selection is intentionally paused: background
                            // browsing must never surprise the user with
                            // sound or decoder work.
                            self.prepare.playing = false;
                        }
                        // Apply a pending draft when its media becomes selected.
                        if let Some(draft) = self.pending_draft.take() {
                            if draft.media_id == media_id {
                                self.apply_draft(draft);
                            } else {
                                self.pending_draft = Some(draft);
                            }
                        }
                    }
                    None => {
                        self.prepare.on_media_changed(0, 0.0);
                        self.prepare_video_loading = false;
                        self.prepare_video_error = None;
                        self.stop_hover_preview(None, cx);
                    }
                }
                cx.notify();
            }
            Event::SelectionCheckingChanged(media_id, checking) => {
                if self.selected.as_ref().map(|row| row.id) != Some(media_id) {
                    return;
                }
                self.checking = checking;
                if !checking {
                    self.command(Command::SelectionCheckFinished);
                }
                cx.notify();
            }
            Event::TimelineLoadingChanged(media_id, loading) => {
                if self.selected.as_ref().map(|row| row.id) != Some(media_id) {
                    return;
                }
                self.timeline_loading = loading;
                cx.notify();
            }
            Event::LibraryPage(page) => {
                if page.offset == 0 && page.generation >= self.library.generation {
                    let visible_ids: std::collections::HashSet<i64> =
                        page.rows.iter().map(|row| row.id).collect();
                    self.thumbnail_states
                        .retain(|media_id, _| visible_ids.contains(media_id));
                    self.thumbnail_requested
                        .retain(|media_id| visible_ids.contains(media_id));
                    let loaded = page.rows.len();
                    self.library = page;
                    // Next expected DB offset (the page's own offset is 0).
                    self.library.offset = loaded;
                    self.reset_library_scroll();
                } else if page_appendable(
                    self.library.generation,
                    self.library.offset,
                    page.generation,
                    page.offset,
                ) {
                    self.library.rows.extend(page.rows.iter().cloned());
                    self.library.has_more = page.has_more;
                    self.library.offset += page.rows.len();
                }
                // Chase a pending reveal across paged rows.
                if let Some(target) = self.reveal_target_row {
                    if target < self.library.rows.len() {
                        self.apply_reveal_scroll();
                        self.reveal_target_row = None;
                    } else if self.library.has_more {
                        self.command(Command::LoadMoreLibrary);
                    } else {
                        self.reveal_target_row = None;
                    }
                }
                cx.notify();
            }
            Event::LibraryRefreshed => {
                self.command(Command::RefreshAll);
            }
            Event::FoldersUpdated(nodes) => {
                // Sync local expansion state: keep user toggles, add new
                // nodes (top level expanded), drop removed ones.
                let mut next = HashMap::new();
                for node in &nodes {
                    let expanded = self
                        .folders_expanded
                        .get(&node.folder)
                        .copied()
                        .unwrap_or(node.depth < 1);
                    next.insert(node.folder.clone(), expanded);
                }
                self.folders_expanded = next;
                self.folders = nodes;
                cx.notify();
            }
            Event::HistoryPage(page) => {
                if page.offset == 0 && page.generation >= self.history.generation {
                    let loaded = page.rows.len();
                    self.history = page;
                    // Next expected DB offset (the page's own offset is 0).
                    self.history.offset = loaded;
                    self.history_scroll.set_offset(point(px(0.0), px(0.0)));
                    self.last_history_scroll_y = 0.0;
                } else if page_appendable(
                    self.history.generation,
                    self.history.offset,
                    page.generation,
                    page.offset,
                ) {
                    self.history.rows.extend(page.rows.iter().cloned());
                    self.history.has_more = page.has_more;
                    self.history.offset += page.rows.len();
                }
                cx.notify();
            }
            Event::HistoryRefreshed => {
                self.command(Command::SetHistorySearch(self.history_search.clone()));
                self.command(Command::RefreshAll);
            }
            Event::WorkspacesChanged(workspaces, _count) => {
                self.workspaces = workspaces;
                self.active_workspace_index =
                    self.workspaces.iter().position(|w| w.active).unwrap_or(0);
                cx.notify();
            }
            Event::ThumbnailReady(id, path) => {
                self.command(Command::ThumbnailFinished(id));
                self.thumbnail_states.insert(
                    id,
                    if path.is_some() {
                        "ready".into()
                    } else {
                        "failed".into()
                    },
                );
                self.thumbnail_requested.remove(&id);
                if let Some(path) = path {
                    let display_path = path.to_string_lossy().into_owned();
                    if let Some(row) = self.library.rows.iter_mut().find(|row| row.id == id) {
                        row.thumbnail_path = Some(display_path.clone());
                    }
                    if let Some(row) = self.selected.as_mut() {
                        if row.id == id {
                            row.thumbnail_path = Some(display_path);
                        }
                    }
                }
                cx.notify();
            }
            Event::PreviewDeferred(media_id) => {
                if self.page == Page::Library
                    && self.active_preview_id == media_id
                    && self.hovered_tiles.get(&media_id) == Some(&true)
                {
                    self.command(Command::EnsurePreview(media_id));
                }
            }
            Event::PreviewReady(id, path) => {
                if let Some(path) = path {
                    self.start_hover_video(id, path, cx);
                }
            }
            Event::TimelineReady(id, path) => {
                if self.selected.as_ref().map(|row| row.id) != Some(id) {
                    return;
                }
                self.timeline_loading = false;
                self.prepare.timeline_ready = true;
                if let Some(path) = path {
                    let display_path = path.to_string_lossy().into_owned();
                    if let Some(row) = self.library.rows.iter_mut().find(|row| row.id == id) {
                        row.timeline_path = Some(display_path.clone());
                    }
                    if let Some(row) = self.selected.as_mut() {
                        row.timeline_path = Some(display_path);
                    }
                }
                cx.notify();
            }
            Event::RandomFoldersChanged(
                options,
                summary,
                selected,
                all_selected,
                has_selection,
            ) => {
                self.random_loading = false;
                self.random_options = options;
                if !summary.is_empty() {
                    self.random_summary = summary;
                }
                self.random_selected = selected;
                self.random_all_selected = all_selected;
                self.random_has_selection = has_selection;
                // Seed expansion state (top two levels open by default).
                for option in &self.random_options {
                    if option.depth < 2 && !self.random_expanded.contains(&option.folder) {
                        self.random_expanded.insert(option.folder.clone());
                    }
                }
                cx.notify();
            }
            Event::Toast(kind, message) => {
                let id = self.next_toast_id;
                self.next_toast_id = self.next_toast_id.wrapping_add(1);
                self.toasts.push(Toast {
                    id,
                    kind,
                    message,
                    shown_at: std::time::Instant::now(),
                });
                cx.notify();
            }
            Event::NavigationRequested(page) => {
                self.navigate_to(page, cx);
            }
            Event::RevealRequested {
                folder,
                media_index,
                folder_index,
            } => {
                self.reveal_request = Some((folder, media_index, folder_index));
                self.page = Page::Library;
                cx.notify();
            }
            Event::NavigationRestored { folder, search, .. } => {
                self.search_text = search;
                self.active_folder = folder;
                self.reset_library_scroll();
                // Restored selection may live beyond the loaded page.
                if self.search_text.is_empty() {
                    if let Some(row) = self.selected.as_ref() {
                        let in_rows = self.library.rows.iter().any(|r| r.id == row.id);
                        if !in_rows && row.folder == self.active_folder {
                            self.command(Command::RevealMedia(row.id));
                        }
                    }
                }
                cx.notify();
            }
            Event::SelectionNavigationChanged(_prev, _next) => {
                cx.notify();
            }
            Event::TimelineFinished(id) => {
                self.command(Command::TimelineFinished(id));
            }
            Event::SelectionVerified(media_id, row) => {
                self.command(Command::SelectionVerified(media_id, row));
            }
            Event::LoadMoreFinished(library, generation, has_more, offset) => {
                self.command(Command::LoadMoreFinished(
                    library, generation, has_more, offset,
                ));
            }
            Event::NeighborPreload(previous, next) => {
                if previous > 0 {
                    self.command(Command::EnsurePreview(previous));
                }
                if next > 0 {
                    self.command(Command::EnsurePreview(next));
                }
            }
            Event::DiagnosticsReady(diagnostics) => {
                self.diagnostics = diagnostics;
                cx.notify();
            }
            Event::DraftRestoreRequested(draft) => {
                self.pending_draft = Some(draft.clone());
                self.apply_draft(draft);
                cx.notify();
            }
            Event::ShuffleDone => {
                cx.notify();
            }
            Event::CountsOnly => {
                self.command(Command::RefreshAll);
            }
            Event::Tick => {
                let now = std::time::Instant::now();
                let before = self.toasts.len();
                self.toasts.retain(|toast| {
                    now.duration_since(toast.shown_at) < Duration::from_millis(5200)
                });
                let playback_changed = self.prepare.tick();
                // History still uses a manual visible-window virtualizer.
                // The library uses GPUI's uniform list, which invalidates the
                // window directly as its scroll position changes.
                let scroll_changed = match self.page {
                    Page::History => {
                        let current = f32::from(self.history_scroll.offset().y);
                        if (current - self.last_history_scroll_y).abs() > 0.5 {
                            self.last_history_scroll_y = current;
                            true
                        } else {
                            false
                        }
                    }
                    Page::Library | Page::Settings => false,
                };
                let capture_pending = self.capture_path.is_some();
                if self.toasts.len() != before
                    || playback_changed
                    || scroll_changed
                    || capture_pending
                {
                    cx.notify();
                }
            }
            Event::HoverCheck(media_id, generation) => {
                if generation != self.preview_hover_generation {
                    return;
                }
                if std::time::Instant::now() < self.library_scroll_pause_until {
                    return;
                }
                if self.active_preview_id == media_id
                    && self.hovered_tiles.get(&media_id) == Some(&true)
                {
                    self.command(Command::EnsurePreview(media_id));
                }
            }
            Event::SearchResults(query, items) => {
                if query == self.command_needle() {
                    self.command_results = items;
                    self.command_searching = false;
                    self.command_selected = self
                        .command_entries()
                        .iter()
                        .position(|e| {
                            matches!(e, crate::render_impls::CommandEntry::Action(a) if a.enabled)
                                || matches!(e, crate::render_impls::CommandEntry::Result(_))
                        })
                        .unwrap_or(0);
                    cx.notify();
                }
            }
            Event::PickingChanged(picking) => {
                self.random_picking = picking;
                if !picking {
                    self.command(Command::PickingFinished);
                }
                cx.notify();
            }
            Event::ClosedCountChanged(count) => {
                self.closed_count = count;
                cx.notify();
            }
            Event::HistorySearchCommitted(generation, text) => {
                if generation == self.history_search_generation && text == self.history_search {
                    self.command(Command::SetHistorySearch(text));
                }
            }
            Event::CommandSearchCommitted {
                generation,
                query,
                scope,
                library_search,
            } => {
                if generation != self.command_search_generation
                    || query != self.command_needle()
                    || scope != self.effective_command_scope()
                {
                    return;
                }
                if let Some(search) = library_search {
                    if search == self.search_text {
                        self.command(Command::SetSearch(search));
                    }
                }
                self.command(Command::SearchSuggestions { query, scope });
            }
            Event::DraftsDirty => {
                self.command(Command::FlushDrafts);
            }
            Event::ScanFinished(id, generation) => {
                self.command(Command::ScanFinished(id, generation));
            }
            Event::RecordNavigationOrigin => {
                self.command(Command::RecordNavigationOrigin);
            }
            Event::SelectionVerifyFailed(media_id) => {
                self.command(Command::SelectionVerifyFailed(media_id));
            }
        }
    }

    fn apply_draft(&mut self, draft: PrepareDraft) {
        if draft.media_id != self.selected.as_ref().map(|m| m.id).unwrap_or(0) {
            return;
        }
        let duration = self.prepare.duration;
        self.prepare.trim_start = draft.trim_start.clamp(0.0, duration.max(0.0));
        self.prepare.trim_end = if draft.trim_end > 0.0 {
            draft.trim_end.clamp(
                self.prepare.trim_start,
                duration.max(self.prepare.trim_start),
            )
        } else {
            duration
        };
        self.prepare.inspector_tab = draft.inspector_tab.clamp(0, 1);
        self.prepare.studio_mode = draft.studio_tab > 0;
        self.prepare.same_caption = draft.same_caption;
        self.prepare.telegram_mode_index = draft.telegram_mode_index.clamp(0, 1);
        self.prepare.destination = draft.destination.clone();
        // The destination is a single source of truth: restoring a draft
        // updates the setting like the original's bound field.
        if !draft.destination.is_empty() {
            self.command(Command::SetSetting(
                TELEGRAM_DESTINATION.to_string(),
                json!(draft.destination),
            ));
        }
        self.prepare.caption = draft.caption;
        self.prepare.x_caption = draft.x_caption;
        self.prepare.compression_index = draft.compression_index.clamp(0, 6);
        self.prepare.target_mb = draft.target_size;
        self.prepare.cleanup_index = draft.cleanup_index.clamp(0, 2);
        self.prepare.studio_width = draft.studio_inspector_width.clamp(360.0, 620.0);
        self.prepare.load_edit_spec(&draft.edits);
        // The model captions changed externally: clear the field states so
        // the caption areas show the restored values and stale edits can't
        // overwrite them.
        for id in ["caption-shared", "caption-tg", "caption-x"] {
            if let Some(field) = self.fields.get_mut(id) {
                field.text.clear();
                field.caret = 0;
            }
        }
        if duration > 0.0 {
            self.prepare.seek(self.prepare.trim_start, duration);
            self.prepare.playing = true;
        }
    }
    fn capture_draft(&self) -> PrepareDraft {
        PrepareDraft {
            media_id: self.selected.as_ref().map(|m| m.id).unwrap_or(0),
            trim_start: self.prepare.trim_start,
            trim_end: self.prepare.trim_end,
            same_caption: self.prepare.same_caption,
            studio_tab: if self.prepare.studio_mode { 1 } else { 0 },
            inspector_tab: self.prepare.inspector_tab,
            telegram_mode_index: self.prepare.telegram_mode_index,
            destination: self.prepare.destination.clone(),
            caption: self.prepare.caption.clone(),
            x_caption: self.prepare.x_caption.clone(),
            compression_index: self.prepare.compression_index,
            target_size: self.prepare.target_mb.clone(),
            cleanup_index: self.prepare.cleanup_index,
            edit_scroll_y: self.prepare.edit_scroll_y,
            publish_scroll_y: self.prepare.publish_scroll_y,
            studio_inspector_width: self.prepare.studio_width,
            edits: self.prepare.edit_spec(),
        }
    }

    fn save_draft(&mut self) {
        if let Some(workspace) = self.workspaces.get(self.active_workspace_index) {
            let id = workspace.id.clone();
            let draft = self.capture_draft();
            self.command(Command::SaveWorkspaceDraft(
                id,
                serde_json::to_value(draft).unwrap_or(serde_json::Value::Null),
            ));
        }
    }

    fn apply_settings(&mut self) {
        let mode = self
            .settings
            .get(THEME_MODE)
            .and_then(|v| v.as_str())
            .unwrap_or("relay");
        let new_mode = ThemeMode::parse(mode);
        if new_mode != self.theme_mode {
            self.theme_mode = new_mode;
            self.theme = Theme::for_mode(new_mode);
        }
        if let Some(scale) = self.settings.get(UI_SCALE).and_then(|v| v.as_f64()) {
            self.ui_scale = scale as f32;
        }
        self.sidebar_collapsed = self
            .settings
            .get(SIDEBAR_COLLAPSED)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.prepare_expanded = self
            .settings
            .get(PREPARE_EXPANDED)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.density = self
            .settings
            .get(LIBRARY_DENSITY)
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
    }

    pub fn toast(&mut self, kind: ToastKind, message: impl Into<String>) {
        // Single toast slot like the original: a new message replaces the
        // previous one.
        self.toasts.clear();
        let id = self.next_toast_id;
        self.next_toast_id = self.next_toast_id.wrapping_add(1);
        self.toasts.push(Toast {
            id,
            kind,
            message: message.into(),
            shown_at: std::time::Instant::now(),
        });
    }

    pub fn set_setting(&mut self, key: &str, value: serde_json::Value, cx: &mut Context<Self>) {
        if [
            THEME_MODE,
            UI_SCALE,
            SIDEBAR_COLLAPSED,
            PREPARE_EXPANDED,
            LIBRARY_DENSITY,
        ]
        .contains(&key)
        {
            self.settings.insert(key.to_string(), value.clone());
            self.apply_settings();
        }
        self.command(Command::SetSetting(key.to_string(), value));
        cx.notify();
    }

    // ---- field helpers --------------------------------------------------

    pub fn field_text(&mut self, id: &str) -> String {
        self.fields
            .get(id)
            .map(|f| f.text.clone())
            .unwrap_or_default()
    }

    /// Open the command center and request results for the current query.
    /// The '>' prefix switches the command center into commands scope
    /// (mirrors the original's commandPrefix).
    pub fn command_prefix(&self) -> bool {
        self.command_scope != "commands" && self.command_query.trim_start().starts_with('>')
    }

    pub fn effective_command_scope(&self) -> String {
        if self.command_prefix() {
            "commands".into()
        } else {
            self.command_scope.clone()
        }
    }

    pub fn command_needle(&self) -> String {
        if self.command_prefix() {
            self.command_query.trim_start()[1..].trim().to_string()
        } else {
            self.command_query.trim().to_string()
        }
    }

    pub fn schedule_command_search(&mut self, cx: &mut Context<Self>) {
        self.command_search_generation = self.command_search_generation.wrapping_add(1);
        let generation = self.command_search_generation;
        let query = self.command_needle();
        let scope = self.effective_command_scope();
        let library_search = (!self.command_prefix()).then(|| self.command_query.clone());
        self.command_searching = !query.is_empty() && scope != "commands";
        let tx = self.event_tx.clone();
        self.command_search_task = Some(cx.spawn(
            move |_this: WeakEntity<crate::App>, _cx: &mut AsyncApp| async move {
                smol::Timer::after(Duration::from_millis(160)).await;
                let _ = tx.send(Event::CommandSearchCommitted {
                    generation,
                    query,
                    scope,
                    library_search,
                });
            },
        ));
    }

    pub fn open_command_center(&mut self, cx: &mut Context<Self>) {
        self.command_open = true;
        self.command_query = self.field_text("command-center");
        let needle = self.command_needle();
        let scope = self.effective_command_scope();
        self.command_searching = !needle.is_empty() && scope != "commands";
        self.command_selected = self
            .command_entries()
            .iter()
            .position(|e| {
                matches!(e, crate::render_impls::CommandEntry::Action(a) if a.enabled)
                    || matches!(e, crate::render_impls::CommandEntry::Result(_))
            })
            .unwrap_or(0);
        self.command(Command::SearchSuggestions {
            query: needle,
            scope,
        });
        cx.notify();
    }

    /// Close the command center; a commands-scope session resets to the
    /// library scope and restores the media query (mirrors the original's
    /// onClosed behavior).
    pub fn close_command_center(&mut self) {
        self.command_open = false;
        self.focused_field = None;
        if self.command_scope == "commands" {
            self.command_scope = "all".into();
            let state = self.fields.entry("command-center".to_string()).or_default();
            state.text = self.search_text.clone();
            state.caret = state.text.chars().count();
        }
    }

    /// Activate the currently highlighted command-center result.
    pub fn activate_command_selection(&mut self, cx: &mut Context<Self>) {
        let entries = self.command_entries();
        let Some(entry) = entries.get(self.command_selected).cloned() else {
            return;
        };
        self.close_command_center();
        match entry {
            crate::render_impls::CommandEntry::Action(action) => {
                if action.enabled {
                    self.run_command_action(action.id, cx);
                }
            }
            crate::render_impls::CommandEntry::Result(item) => {
                if item.kind == "media" {
                    self.command(Command::SelectMedia(item.media_id));
                    // Scroll the grid to the tile when it is already loaded;
                    // otherwise chase it across pages.
                    if let Some(index) = self
                        .library
                        .rows
                        .iter()
                        .position(|row| row.id == item.media_id)
                    {
                        self.reveal_target_row = Some(index);
                        self.apply_reveal_scroll();
                    } else {
                        self.command(Command::RevealMedia(item.media_id));
                    }
                } else {
                    self.command(Command::SetFolder(item.folder_path.clone()));
                    self.search_text.clear();
                    self.command(Command::SetSearch(String::new()));
                }
            }
            crate::render_impls::CommandEntry::Section(_) => {}
        }
        cx.notify();
    }

    pub fn field_state_mut(&mut self, id: &str) -> &mut widgets::FieldState {
        self.fields.entry(id.to_string()).or_default()
    }

    pub fn focus_field(&mut self, id: &str, cx: &mut Context<Self>) {
        self.focused_field = Some(id.to_string());
        self.platform_input_focus = None;
        // Seed the field from the committed value so editing starts from it
        // (mirrors the original's Binding restore on focus).
        let seed: Option<String> = match id {
            "prepare-in" => Some(self.prepare.format_time_precise(self.prepare.trim_start)),
            "prepare-out" => Some(self.prepare.format_time_precise(self.prepare.trim_end)),
            "caption-shared" | "caption-tg" => Some(self.prepare.caption.clone()),
            "caption-x" => Some(self.prepare.x_caption.clone()),
            _ => None,
        };
        if let Some(text) = seed {
            let state = self.fields.entry(id.to_string()).or_default();
            if state.text.is_empty() {
                state.text = text;
                state.caret = state.text.chars().count();
            }
        }
        if id == "command-center" {
            self.open_command_center(cx);
        }
    }

    pub fn combo_open(&self, id: &str) -> bool {
        self.open_combos.contains(id)
    }

    /// Mirrors the original's 180ms lastClosedAt reopen guard.
    pub fn menu_reopen_allowed(&self) -> bool {
        self.menu_closed_at.elapsed().as_millis() >= 180
    }

    pub fn mark_menu_closed(&mut self) {
        self.menu_closed_at = std::time::Instant::now();
    }

    pub fn toggle_combo(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.open_combos.remove(id) {
            self.open_combos.insert(id.to_string());
        }
        cx.notify();
    }

    pub fn close_combo(&mut self, id: &str, cx: &mut Context<Self>) {
        self.open_combos.remove(id);
        cx.notify();
    }

    // ---- folder choosers ------------------------------------------------

    pub fn choose_library_folder(&mut self, cx: &mut Context<Self>) {
        let controller = self.controller.clone();
        let folder = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose your video library".into()),
        });
        cx.spawn(
            move |_this: WeakEntity<crate::App>, _cx: &mut AsyncApp| async move {
                if let Ok(Ok(Some(mut paths))) = folder.await {
                    if let Some(path) = paths.pop() {
                        let _ = controller
                            .send(Command::ChooseLibrary(path.to_string_lossy().into_owned()));
                    }
                }
            },
        )
        .detach();
    }

    pub fn choose_export_folder(&mut self, cx: &mut Context<Self>) {
        let controller = self.controller.clone();
        let folder = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose where generated videos are kept".into()),
        });
        cx.spawn(
            move |_this: WeakEntity<crate::App>, _cx: &mut AsyncApp| async move {
                if let Ok(Ok(Some(mut paths))) = folder.await {
                    if let Some(path) = paths.pop() {
                        let _ = controller.send(Command::SetSetting(
                            EXPORT_DIR.to_string(),
                            json!(path.to_string_lossy().into_owned()),
                        ));
                    }
                }
            },
        )
        .detach();
    }

    // ---- video lifecycle ------------------------------------------------

    fn load_video(path: PathBuf, options: VideoOptions) -> Result<Video, String> {
        let uri = gpui_video_player::Url::from_file_path(&path)
            .map_err(|_| format!("invalid local video path: {}", path.display()))?;
        Video::new_with_options(&uri, options)
            .map_err(|error| format!("failed to open {}: {error:?}", path.display()))
    }

    fn retire_video(video: Video, cx: &Context<Self>) {
        cx.background_spawn(async move {
            // Keep one non-UI owner until the previous element tree has been
            // discarded. The final Video drop stops GStreamer and joins its
            // worker, so that final drop must not happen on the app thread.
            smol::Timer::after(Duration::from_millis(150)).await;
            drop(video);
        })
        .detach();
    }

    fn retire_video_async(video: Video, cx: &AsyncApp) {
        cx.background_spawn(async move {
            smol::Timer::after(Duration::from_millis(150)).await;
            drop(video);
        })
        .detach();
    }

    fn start_prepare_video(&mut self, media_id: i64, media_path: PathBuf, cx: &mut Context<Self>) {
        self.prepare_video_generation = self.prepare_video_generation.wrapping_add(1);
        let generation = self.prepare_video_generation;
        self.prepare_video_load_task.take();
        self.prepare_video_loading = true;
        self.prepare_video_error = None;
        let expected_path = media_path.clone();
        let load = cx.background_spawn(async move {
            Self::load_video(
                media_path,
                VideoOptions {
                    frame_buffer_capacity: Some(0),
                    looping: Some(false),
                    speed: Some(1.0),
                },
            )
        });
        self.prepare_video_load_task = Some(cx.spawn(async move |this, cx| match load.await {
            Ok(video) => {
                let retired = this.update(cx, move |app, cx| {
                    let current = app.prepare_video_generation == generation
                        && app.prepare.media_id == media_id
                        && app.prepare.media_path.as_ref() == Some(&expected_path);
                    if !current {
                        return Some(video);
                    }
                    video.set_muted(false);
                    video.set_volume(0.65);
                    video.set_looping(false);
                    video.set_paused(!app.prepare.playing);
                    app.prepare.position = 0.0;
                    app.prepare_video_loading = false;
                    app.prepare_video_error = None;
                    let previous = app.prepare.video.replace(video);
                    log::info!("gpui-video-player ready for {:?}", expected_path);
                    cx.notify();
                    previous
                });
                if let Ok(Some(video)) = retired {
                    Self::retire_video_async(video, cx);
                }
            }
            Err(error) => {
                let _ = this.update(cx, move |app, cx| {
                    if app.prepare_video_generation == generation
                        && app.prepare.media_id == media_id
                    {
                        log::warn!("{error}");
                        app.prepare_video_loading = false;
                        app.prepare_video_error = Some("Video playback unavailable".into());
                        app.prepare.playing = false;
                        cx.notify();
                    }
                });
            }
        }));
    }

    fn start_hover_video(&mut self, media_id: i64, preview_path: PathBuf, cx: &mut Context<Self>) {
        let generation = self.preview_hover_generation;
        if !preview_request_is_current(
            self.active_preview_id,
            self.preview_hover_generation,
            self.hovered_tiles.get(&media_id) == Some(&true),
            media_id,
            generation,
        ) || std::time::Instant::now() < self.library_scroll_pause_until
        {
            return;
        }
        if self.preview_video.as_ref().map(|(id, _)| *id) == Some(media_id)
            || self.preview_video_loading == Some((media_id, generation))
        {
            return;
        }

        self.preview_video_load_task.take();
        self.preview_video_loading = Some((media_id, generation));
        let expected_path = preview_path.clone();
        let load = cx.background_spawn(async move {
            let video = Self::load_video(
                preview_path,
                VideoOptions {
                    // Hover previews render the most recent frame. Retaining a
                    // second raw-frame queue only increases copies and memory.
                    frame_buffer_capacity: Some(0),
                    looping: Some(true),
                    speed: Some(1.0),
                },
            )?;
            video.set_muted(true);
            video.set_volume(0.0);
            Ok::<Video, String>(video)
        });
        self.preview_video_load_task = Some(cx.spawn(async move |this, cx| match load.await {
            Ok(video) => {
                let retired = this.update(cx, move |app, cx| {
                    if app.preview_video_loading == Some((media_id, generation)) {
                        app.preview_video_loading = None;
                    }
                    let current = preview_request_is_current(
                        app.active_preview_id,
                        app.preview_hover_generation,
                        app.hovered_tiles.get(&media_id) == Some(&true),
                        media_id,
                        generation,
                    ) && std::time::Instant::now() >= app.library_scroll_pause_until;
                    if !current {
                        return Some(video);
                    }
                    let previous = app.preview_video.replace((media_id, video));
                    log::debug!(
                        "hover video ready for {} from {:?}",
                        media_id,
                        expected_path
                    );
                    cx.notify();
                    previous.map(|(_, video)| video)
                });
                if let Ok(Some(video)) = retired {
                    Self::retire_video_async(video, cx);
                }
            }
            Err(error) => {
                let _ = this.update(cx, |app, cx| {
                    if app.preview_video_loading == Some((media_id, generation)) {
                        app.preview_video_loading = None;
                        log::warn!("hover preview {media_id}: {error}");
                        cx.notify();
                    }
                });
            }
        }));
    }

    pub fn stop_hover_preview(&mut self, media_id: Option<i64>, cx: &mut Context<Self>) {
        if media_id.is_some_and(|id| self.active_preview_id != id) {
            return;
        }
        self.preview_hover_generation = self.preview_hover_generation.wrapping_add(1);
        self.active_preview_id = 0;
        self.preview_video_loading = None;
        self.preview_video_load_task.take();
        if let Some((_, video)) = self.preview_video.take() {
            Self::retire_video(video, cx);
        }
    }

    // ---- key dispatch ---------------------------------------------------

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let key = keystroke.key.as_str();
        let modifiers = keystroke.modifiers;
        // GPUI maps the platform shortcut modifier to Command on macOS
        // and the logo key elsewhere. ClipRelay follows desktop convention:
        // Command on macOS, Control on Linux and Windows.
        let cmd = if cfg!(target_os = "macos") {
            modifiers.platform
        } else {
            modifiers.control
        };
        let shift = modifiers.shift;

        // Text field editing takes priority.
        if let Some(field_id) = self.focused_field.clone() {
            let handled =
                self.handle_field_key(&field_id, key, keystroke.key_char.as_deref(), cmd, cx);
            if handled {
                return;
            }
        }

        match key {
            "1" if cmd => self.navigate_to(Page::Library, cx),
            "2" if cmd => self.navigate_to(Page::History, cx),
            "," if cmd => self.navigate_to(Page::Settings, cx),
            "f" if cmd && modifiers.control => {
                window.toggle_fullscreen();
                cx.notify();
            }
            "f" if cmd => {
                // Find always opens the global command center (mirrors the
                // original's StandardKey.Find handler).
                self.focus_field("command-center", cx);
                cx.notify();
            }
            "k" if cmd => {
                self.focus_field("command-center", cx);
                cx.notify();
            }
            "p" if cmd && shift => {
                // Commands scope (mirrors focusCommands in the original).
                self.command_scope = "commands".into();
                if let Some(field) = self.fields.get_mut("command-center") {
                    field.text.clear();
                    field.caret = 0;
                }
                self.focus_field("command-center", cx);
                cx.notify();
            }
            "m" if cmd => {
                window.minimize_window();
            }
            "f11" => {
                window.toggle_fullscreen();
                cx.notify();
            }
            "w" if cmd && shift => {
                cx.quit();
            }
            "[" if cmd || modifiers.alt => {
                self.command(Command::NavigateBack);
            }
            "]" if cmd || modifiers.alt => {
                self.command(Command::NavigateForward);
            }
            "t" if cmd => {
                self.choose_new_workspace_folder(cx);
            }
            "w" if cmd && !shift => {
                if let Some(workspace) = self.workspaces.get(self.active_workspace_index) {
                    let id = workspace.id.clone();
                    self.command(Command::CloseWorkspace(id));
                }
            }
            "t" if cmd && shift => {
                self.command(Command::ReopenClosedWorkspace);
            }
            "tab" if modifiers.control => {
                let direction = if shift { -1 } else { 1 };
                self.save_draft();
                let count = self.workspaces.len();
                if count > 0 {
                    let next = (self.active_workspace_index as isize + direction as isize)
                        .rem_euclid(count as isize) as usize;
                    self.activate_workspace_at(next, cx);
                }
            }
            "r" if !cmd && !modifiers.alt => {
                // Global like the original: jump to Library and pick.
                self.page = Page::Library;
                self.command(Command::PickRandom);
            }
            "up" | "down" if self.random_popup_open && self.focused_field.is_none() => {
                let rows = self.random_visible_options();
                if !rows.is_empty() {
                    let delta = if key == "up" { -1 } else { 1 };
                    self.random_tree_cursor = (self.random_tree_cursor as isize + delta)
                        .rem_euclid(rows.len() as isize)
                        as usize;
                }
                cx.notify();
            }
            " " | "space" if self.random_popup_open && self.focused_field.is_none() => {
                if let Some(option) = self.random_visible_options().get(self.random_tree_cursor) {
                    let folder = option.folder.clone();
                    let state = option.selection_state;
                    self.command(Command::SetRandomFolderEnabled(folder, state == 0));
                }
                cx.notify();
            }
            "left" | "right" if self.random_popup_open && self.focused_field.is_none() => {
                if let Some(option) = self.random_visible_options().get(self.random_tree_cursor) {
                    let folder = option.folder.clone();
                    if option.has_children && !self.random_expanded.remove(&folder) {
                        self.random_expanded.insert(folder);
                    }
                }
                cx.notify();
            }
            "left" | "right" if self.explorer_focus && self.page == Page::Library => {
                self.explorer_key(key, cx);
            }
            "up" | "down" if self.explorer_focus && self.page == Page::Library => {
                self.explorer_move(if key == "up" { -1 } else { 1 }, cx);
            }
            " " | "space" | "enter" if self.explorer_focus && self.page == Page::Library => {
                self.explorer_key(key, cx);
            }
            "left" | "right" if !self.random_popup_open => {
                // Global like the original: jump to Library and navigate.
                self.page = Page::Library;
                self.explorer_focus = false;
                let direction = if key == "left" { -1 } else { 1 };
                self.command(Command::NavigateSelection(direction));
            }
            "up" | "down"
                if self.page == Page::Library
                    && !self.random_popup_open
                    && self.focused_field.is_none()
                    && !self.explorer_focus =>
            {
                let columns = self.library_columns().max(1) as isize;
                let delta = if key == "up" { -columns } else { columns };
                let rows = &self.library.rows;
                let target = match self.selected.as_ref() {
                    None => rows.first().map(|r| r.id),
                    Some(current) => {
                        let index = rows.iter().position(|r| r.id == current.id);
                        index
                            .and_then(|i| {
                                let target =
                                    (i as isize + delta).clamp(0, rows.len() as isize - 1) as usize;
                                rows.get(target).map(|r| r.id)
                            })
                            .or(Some(current.id))
                    }
                };
                if let Some(id) = target {
                    self.command(Command::SelectMedia(id));
                }
                cx.notify();
            }
            " " | "space" => {
                if self.page == Page::Library && self.selected.is_some() {
                    self.prepare.toggle_playback();
                    self.save_draft();
                    cx.notify();
                }
            }
            "escape" => {
                if self.random_popup_open {
                    self.random_popup_open = false;
                    self.mark_menu_closed();
                } else if self.command_open {
                    self.command_open = false;
                    self.focused_field = None;
                } else if self.sort_menu_open {
                    self.sort_menu_open = false;
                    self.mark_menu_closed();
                } else if self.workspace_menu_open {
                    self.workspace_menu_open = false;
                    self.mark_menu_closed();
                } else if self.activity_open {
                    self.activity_open = false;
                    self.mark_menu_closed();
                } else if !self.open_combos.is_empty() {
                    self.open_combos.clear();
                } else if self.focused_field.as_deref() == Some("workspace-rename") {
                    // Escape cancels a pending rename (mirrors the original).
                    self.focused_field = None;
                    self.renaming_workspace = None;
                } else if self.focused_field.is_some() {
                    self.focused_field = None;
                } else if self.prepare.studio_mode {
                    self.prepare.studio_mode = false;
                } else if window.is_fullscreen() {
                    window.toggle_fullscreen();
                } else if self.history_more_menu_post.take().is_some() {
                    self.history_more_menu_closed_at = std::time::Instant::now();
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn handle_field_key(
        &mut self,
        field_id: &str,
        key: &str,
        key_char: Option<&str>,
        cmd: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if field_id == "command-center" && self.command_open {
            match key {
                "down" | "up" => {
                    let entries = self.command_entries();
                    if !entries.is_empty() {
                        let direction: isize = if key == "down" { 1 } else { -1 };
                        let mut next = self.command_selected as isize;
                        let mut guard = 0;
                        while guard < entries.len() as isize * 2 {
                            next = (next + direction).rem_euclid(entries.len() as isize);
                            match entries.get(next as usize) {
                                Some(crate::render_impls::CommandEntry::Action(a)) if a.enabled => {
                                    break;
                                }
                                Some(crate::render_impls::CommandEntry::Result(_)) => break,
                                _ => {}
                            }
                            guard += 1;
                        }
                        self.command_selected = next.max(0) as usize;
                        cx.notify();
                    }
                    return true;
                }
                "enter" => {
                    self.activate_command_selection(cx);
                    return true;
                }
                "escape" => {
                    self.close_command_center();
                    cx.notify();
                    return true;
                }
                _ => {}
            }
        }
        let platform_input_ready = self.platform_input_focus.is_some();
        let field = self.field_state_mut(field_id);
        match key {
            "backspace" => {
                field.backspace();
                self.on_field_changed(field_id, cx);
                true
            }
            "delete" => {
                field.delete();
                self.on_field_changed(field_id, cx);
                true
            }
            "left" => {
                field.move_left();
                cx.notify();
                true
            }
            "right" => {
                field.move_right();
                cx.notify();
                true
            }
            "enter" => {
                self.focused_field = None;
                self.on_field_commit(field_id, cx);
                cx.notify();
                true
            }
            "escape" => {
                self.focused_field = None;
                cx.notify();
                true
            }
            "v" if cmd => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    field.insert_str(&text);
                    self.on_field_changed(field_id, cx);
                }
                true
            }
            "c" if cmd => {
                let text = field.text.clone();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                true
            }
            "x" if cmd => {
                let text = field.text.clone();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                field.text.clear();
                field.caret = 0;
                self.on_field_changed(field_id, cx);
                true
            }
            "a" if cmd => {
                field.caret = field.text.chars().count();
                cx.notify();
                true
            }
            _ => {
                if !platform_input_ready {
                    if let Some(ch) = key_char {
                        if ch.chars().count() == 1 && !cmd {
                            field.insert(ch.chars().next().unwrap());
                            self.on_field_changed(field_id, cx);
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    fn on_field_changed(&mut self, field_id: &str, cx: &mut Context<Self>) {
        let text = self.field_text(field_id);
        if field_id == "command-center" {
            self.command_query = text.clone();
            self.command_open = true;
            if !self.command_prefix() {
                // Outside command mode the field drives the library search.
                self.search_text = text.clone();
                self.reset_library_scroll();
            }
            self.schedule_command_search(cx);
            cx.notify();
            return;
        }
        if field_id == "random-filter" {
            self.random_filter = text;
            cx.notify();
            return;
        }
        match field_id {
            "caption-shared" => {
                self.prepare.caption = text.clone();
                self.prepare.x_caption = text.clone();
                self.prepare.same_caption = true;
                self.save_draft();
            }
            "caption-tg" => {
                self.prepare.caption = text.clone();
                self.save_draft();
            }
            "caption-x" => {
                self.prepare.x_caption = text.clone();
                self.save_draft();
            }
            "history-search" => {
                self.history_search = text.clone();
                // Debounce like the original's 180ms history timer.
                self.history_search_generation += 1;
                let generation = self.history_search_generation;
                let tx = self.event_tx.clone();
                self.history_search_task = Some(cx.spawn(
                    move |_this: WeakEntity<crate::App>, _cx: &mut AsyncApp| async move {
                        smol::Timer::after(Duration::from_millis(180)).await;
                        let _ = tx.send(Event::HistorySearchCommitted(generation, text));
                    },
                ));
            }
            "prepare-in" => {
                if let Ok(seconds) = parse_time(&text) {
                    let max = (self.prepare.trim_end - 0.05).max(0.0);
                    let clamped = seconds.clamp(0.0, max);
                    self.prepare.trim_start = clamped;
                    self.prepare.seek(clamped, self.prepare.duration);
                    self.save_draft();
                }
            }
            "prepare-out" => {
                if let Ok(seconds) = parse_time(&text) {
                    let min = (self.prepare.trim_start + 0.05).min(self.prepare.duration);
                    let clamped = seconds.clamp(min, self.prepare.duration.max(min));
                    self.prepare.trim_end = clamped;
                    self.save_draft();
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn on_field_commit(&mut self, field_id: &str, cx: &mut Context<Self>) {
        let text = self.field_text(field_id);
        match field_id {
            "prepare-in" => {
                if let Ok(seconds) = parse_time(&text) {
                    let max = (self.prepare.trim_end - 0.05).max(0.0);
                    self.prepare.trim_start = seconds.clamp(0.0, max);
                    self.prepare
                        .seek(self.prepare.trim_start, self.prepare.duration);
                }
            }
            "prepare-out" => {
                if let Ok(seconds) = parse_time(&text) {
                    let min = (self.prepare.trim_start + 0.05).min(self.prepare.duration);
                    self.prepare.trim_end = seconds.clamp(min, self.prepare.duration.max(min));
                }
            }
            "x-limit" => {
                let mb: f64 = text.parse().unwrap_or(512.0);
                let mb = if mb == 0.0 { 512.0 } else { mb };
                self.set_setting(X_LIMIT_MB, json!(mb), cx);
            }
            "workspace-rename" => {
                if let Some(index) = self.renaming_workspace {
                    if let Some(workspace) = self.workspaces.get(index) {
                        let id = workspace.id.clone();
                        self.command(Command::RenameWorkspace(id, text));
                    }
                }
                self.renaming_workspace = None;
            }
            "tg-bot-token" => {}
            "tg-destination" => {
                let destination = text.trim().to_string();
                self.set_setting(TELEGRAM_DESTINATION, json!(destination.clone()), cx);
                // Single source of truth: the prepare pane follows.
                self.prepare.destination = destination;
                self.save_draft();
            }
            "tg-destination-field" => {
                let destination = text.trim().to_string();
                self.prepare.destination = destination.clone();
                self.set_setting(TELEGRAM_DESTINATION, json!(destination), cx);
                self.save_draft();
            }
            _ => {}
        }
        self.save_draft();
        cx.notify();
    }

    // ---- render ---------------------------------------------------------

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl Element {
        let size = window.bounds().size;
        self.window_size = (size.width.into(), size.height.into());
        // Persist the window size (throttled) so the next launch restores it.
        if (self.window_size.0 - self.saved_bounds.0).abs() > 24.0
            || (self.window_size.1 - self.saved_bounds.1).abs() > 24.0
        {
            self.saved_bounds = self.window_size;
            self.command(Command::SetSetting(
                WINDOW_BOUNDS.to_string(),
                serde_json::json!(format!(
                    "{}x{}",
                    self.window_size.0.round() as i64,
                    self.window_size.1.round() as i64
                )),
            ));
        }
        // Dev-only surface capture: after ~40 rendered frames (the layout is
        // settled), ask the window to save its rendered surface as a PNG.
        // Works with the physical display asleep (the Metal drawable is
        // read back after the GPU finishes).
        if let Some(path) = self.capture_path.clone() {
            // Wall-clock based: the boot envs (the page switch ~2.5s in)
            // settle well before the threshold. Frames are unreliable
            // because the render loop only repaints dirty windows.
            let elapsed = self.capture_started_at.elapsed();
            let target = std::time::Duration::from_millis((self.capture_after_target as u64) * 100);
            if elapsed >= target {
                self.capture_path = None;
                eprintln!("[capture] firing for {:?}", path);
                window.request_surface_capture(path);
            } else {
                self.capture_after_frames += 1;
            }
        }

        // Reflect the active library in the window title.
        {
            let root_label = self.settings_value(LIBRARY_ROOT);
            let library_name = root_label.rsplit('/').next().unwrap_or("");
            let title = if library_name.is_empty() {
                "ClipRelay".to_string()
            } else {
                format!("ClipRelay — {library_name}")
            };
            if self.window_title != title {
                self.window_title = title.clone();
                window.set_window_title(&title);
            }
        }

        let theme = self.theme.clone();
        widgets::set_current_theme(&theme);

        let mut root = div()
            .id("cliprelay")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.ink)
            .text_color(theme.text)
            .text_size(px(15.0))
            .font_family("System Font")
            .track_focus(&self.focus_handle)
            .key_context("cliprelay")
            .on_action(cx.listener(|_app, _: &FocusNext, window, _cx| {
                window.focus_next();
            }))
            .on_action(cx.listener(|_app, _: &FocusPrevious, window, _cx| {
                window.focus_prev();
            }))
            .on_action(cx.listener(|app, _: &Activate, _window, cx| {
                if let Some(field_id) = app.focused_field.clone() {
                    app.handle_field_key(&field_id, "enter", None, false, cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|app, _: &ActivateSpace, _window, cx| {
                if let Some(field_id) = app.focused_field.clone() {
                    app.handle_field_key(&field_id, "space", Some(" "), false, cx);
                    cx.stop_propagation();
                }
            }))
            .on_key_down(cx.listener(|app, event, window, cx| {
                app.on_key_down(event, window, cx);
            }));

        // Header (command center row).
        root = root.child(self.render_header(cx));
        // Context toolbar.
        root = root.child(self.render_context_toolbar(cx));

        let mut body = div()
            .id("body")
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_row()
            .min_h(px(0.0));
        body = body.child(self.render_sidebar(cx));
        body = body.child(div().w(px(1.0)).h_full().bg(self.theme.border).flex_none());
        let page = self.page;
        body = body.child(match page {
            Page::Library => {
                if self.selected.is_some() && !self.prepare.studio_mode {
                    let mut row = div()
                        .id("library-row")
                        .flex_1()
                        .flex()
                        .flex_row()
                        .min_w(px(0.0));
                    row = row.child(self.render_library(cx));
                    row = row.child(self.render_prepare_dock(cx));
                    row.into_any()
                } else if self.prepare.studio_mode && self.selected.is_some() {
                    self.render_prepare_studio(cx).into_any()
                } else {
                    self.render_library(cx).into_any()
                }
            }
            Page::History => self.render_history(cx).into_any(),
            Page::Settings => self.render_settings(cx).into_any(),
        });

        root = root.child(body);
        // Workspace tabs at the window bottom (mirrors the original).
        root = root.child(self.render_workspace_tabs(cx));

        // Toasts.
        let toasts = self.toasts.clone();
        if !toasts.is_empty() {
            let toast_x = ((self.window_size.0 - 460.0) / 2.0).max(0.0);
            let mut toast_column = div()
                .absolute()
                .bottom(px(24.0))
                .left(px(toast_x))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .w(px(460.0));
            for toast in toasts {
                let toast_id = toast.id;
                let (bg, border, glyph, color) = match toast.kind {
                    ToastKind::Info => (theme.surface_soft, theme.accent, "i", theme.accent),
                    ToastKind::Success => (theme.success_soft, theme.success, "✓", theme.success),
                    ToastKind::Warning => (theme.warning_soft, theme.warning, "!", theme.warning),
                    ToastKind::Error => (theme.error_soft, theme.error, "!", theme.error),
                };
                toast_column = toast_column.child(
                    div()
                        .id(SharedString::from(format!("toast-{toast_id}")))
                        .w_full()
                        .min_h(px(52.0))
                        .px(px(16.0))
                        .py(px(12.0))
                        .rounded(px(10.0))
                        .bg(bg)
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(10.0))
                        .child(icon(glyph, 18.0, color))
                        .child(
                            div()
                                .flex_1()
                                .child(toast.message.clone())
                                .text_size(px(13.0))
                                .text_color(theme.text),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("toast-dismiss-{toast_id}")))
                                .h(px(28.0))
                                .px(px(10.0))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child("Dismiss")
                                .text_size(px(12.0))
                                .text_color(theme.muted)
                                .child(icon("✕", 11.0, theme.muted))
                                .on_click(cx.listener(move |app, _event, _window, cx| {
                                    app.toasts.retain(|toast| toast.id != toast_id);
                                    cx.notify();
                                })),
                        ),
                );
            }
            root = root.child(toast_column);
        }

        // History "More actions" menu overlay.
        if self.page == Page::History && self.history_more_menu_post.is_some() {
            root = root.child(self.render_history_menu(cx));
        }

        // Random-source popup.
        if self.random_popup_open {
            root = root.child(self.render_random_popup(cx));
        }

        // Command-center popup.
        if self.command_open {
            root = root.child(self.render_command_center(cx));
        }

        // Workspace context menu.
        if self.workspace_menu_open {
            root = root.child(self.render_workspace_menu(cx));
        }

        // Sort control popup.
        if self.sort_menu_open {
            root = root.child(self.render_sort_menu(cx));
        }

        // Background activity popup.
        if self.activity_open {
            root = root.child(self.render_activity_popup(cx));
        }

        root
    }
}

impl App {
    fn replace_platform_text(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        text: &str,
        selected_utf16: Option<std::ops::Range<usize>>,
        mark: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.focused_field.clone() else {
            return;
        };
        let state = self.fields.entry(id.clone()).or_default();
        let replacement = range_utf16
            .map(|range| {
                widgets::FieldState::utf16_to_char(&state.text, range.start)
                    ..widgets::FieldState::utf16_to_char(&state.text, range.end)
            })
            .or_else(|| state.marked_range.clone())
            .unwrap_or(state.caret..state.caret);
        let start = replacement.start.min(state.text.chars().count());
        let end = replacement.end.max(start).min(state.text.chars().count());
        let start_byte = widgets::FieldState::char_to_byte(&state.text, start);
        let end_byte = widgets::FieldState::char_to_byte(&state.text, end);
        state.text.replace_range(start_byte..end_byte, text);
        let inserted = text.chars().count();
        state.marked_range = (mark && !text.is_empty()).then_some(start..start + inserted);
        let selection_offset = selected_utf16
            .map(|range| widgets::FieldState::utf16_to_char(text, range.end))
            .unwrap_or(inserted);
        state.caret = (start + selection_offset).min(state.text.chars().count());
        self.on_field_changed(&id, cx);
    }
}

impl EntityInputHandler for App {
    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let id = self.focused_field.as_ref()?;
        let state = self.fields.get(id)?;
        let start = widgets::FieldState::utf16_to_char(&state.text, range_utf16.start);
        let end = widgets::FieldState::utf16_to_char(&state.text, range_utf16.end).max(start);
        actual_range.replace(
            widgets::FieldState::char_to_utf16(&state.text, start)
                ..widgets::FieldState::char_to_utf16(&state.text, end),
        );
        let start_byte = widgets::FieldState::char_to_byte(&state.text, start);
        let end_byte = widgets::FieldState::char_to_byte(&state.text, end);
        Some(state.text[start_byte..end_byte].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let id = self.focused_field.as_ref()?;
        let state = self.fields.get(id)?;
        let caret = widgets::FieldState::char_to_utf16(&state.text, state.caret);
        Some(UTF16Selection {
            range: caret..caret,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        let id = self.focused_field.as_ref()?;
        let state = self.fields.get(id)?;
        state.marked_range.as_ref().map(|range| {
            widgets::FieldState::char_to_utf16(&state.text, range.start)
                ..widgets::FieldState::char_to_utf16(&state.text, range.end)
        })
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(id) = self.focused_field.clone() {
            if let Some(state) = self.fields.get_mut(&id) {
                state.marked_range = None;
            }
        }
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_platform_text(range_utf16, text, None, false, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        text: &str,
        selected_utf16: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_platform_text(range_utf16, text, selected_utf16, true, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let id = self.focused_field.as_ref()?;
        let state = self.fields.get(id)?;
        let index = widgets::FieldState::utf16_to_char(&state.text, range_utf16.start);
        let x = bounds.left() + px(13.0 + index as f32 * 7.0);
        Some(Bounds::new(
            point(x.min(bounds.right()), bounds.top()),
            size(px(1.0), bounds.size.height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let id = self.focused_field.as_ref()?;
        let state = self.fields.get(id)?;
        let left = self
            .platform_input_bounds
            .map(|bounds| f32::from(bounds.left()))
            .unwrap_or(0.0);
        let approximate = ((f32::from(point.x) - left - 13.0).max(0.0) / 7.0).round() as usize;
        let index = approximate.min(state.text.chars().count());
        Some(widgets::FieldState::char_to_utf16(&state.text, index))
    }
}

#[cfg(test)]
mod time_tests {
    use super::{parse_time, preview_request_is_current};

    #[test]
    fn parse_time_rejects_non_finite() {
        assert!(parse_time("nan").is_err());
        assert!(parse_time("1:nan").is_err());
        assert!(parse_time("inf").is_err());
        assert!(parse_time("-5").is_err());
        assert!(parse_time("").is_err());
        assert!(parse_time("1:30").is_ok());
        assert_eq!(parse_time("1:30").unwrap(), 90.0);
        assert_eq!(parse_time("12.5").unwrap(), 12.5);
    }

    #[test]
    fn hover_preview_rejects_stale_or_inactive_loads() {
        assert!(preview_request_is_current(42, 7, true, 42, 7));
        assert!(!preview_request_is_current(42, 8, true, 42, 7));
        assert!(!preview_request_is_current(0, 7, true, 42, 7));
        assert!(!preview_request_is_current(42, 7, false, 42, 7));
        assert!(!preview_request_is_current(42, 7, true, 99, 7));
    }
}

fn parse_time(text: &str) -> Result<f64, ()> {
    let text = text.trim();
    if text.is_empty() {
        return Err(());
    }
    let parts: Vec<&str> = text.split(':').collect();
    let mut seconds = 0.0;
    for part in parts {
        let value: f64 = part.parse().map_err(|_| ())?;
        if !value.is_finite() || value < 0.0 {
            return Err(());
        }
        seconds = seconds * 60.0 + value;
    }
    if !seconds.is_finite() {
        return Err(());
    }
    Ok(seconds)
}

fn preview_request_is_current(
    active_media_id: i64,
    active_generation: u64,
    hovered: bool,
    request_media_id: i64,
    request_generation: u64,
) -> bool {
    hovered && active_media_id == request_media_id && active_generation == request_generation
}

// Widget helpers exposed for the widgets module.

pub fn tooltip_view(cx: &mut gpui::App, text: SharedString) -> AnyView {
    // Minimal tooltip: a label in a floating surface (rendered by GPUI's
    // tooltip machinery as a separate view).
    struct Tooltip(SharedString);
    impl Render for Tooltip {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let theme = widgets::current_theme();
            div()
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(theme.raised)
                .border_1()
                .border_color(theme.border_strong)
                .child(self.0.clone())
                .text_size(px(12.0))
                .text_color(theme.text)
        }
    }
    let view = cx.new(|_cx| Tooltip(text));
    view.into()
}

// Theme helpers used by widgets.
impl crate::theme::Theme {
    pub fn transparent(&self) -> Hsla {
        Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 0.0,
        }
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render(window, cx)
    }
}

/// Minimal CLI parsing (mirrors the original's app.py arguments):
/// `--data-dir PATH`, `--library PATH`, `--window-width N`,
/// `--window-height N`.
fn parse_cli_args() -> (f32, f32) {
    let mut width = 1460.0f32;
    let mut height = 900.0f32;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" | "--data_dir" => {
                if let Some(path) = args.next() {
                    std::env::set_var("CLIPRELAY_DATA_DIR", path);
                }
            }
            "--library" => {
                if let Some(path) = args.next() {
                    std::env::set_var("CLIPRELAY_LIBRARY", path);
                }
            }
            "--window-width" | "--window_width" => {
                if let Some(value) = args.next() {
                    if let Ok(v) = value.parse::<f32>() {
                        width = v.max(940.0);
                    }
                }
            }
            "--window-height" | "--window_height" => {
                if let Some(value) = args.next() {
                    if let Ok(v) = value.parse::<f32>() {
                        height = v.max(660.0);
                    }
                }
            }
            _ => {}
        }
    }
    (width, height)
}

/// Best-effort restore of the last window size (`WINDOW_BOUNDS`).
fn boot_settings() -> Result<std::collections::HashMap<String, serde_json::Value>, anyhow::Error> {
    let db_path = cliprelay_core::paths::database_path();
    let db = cliprelay_core::db::Database::open(&db_path)?;
    let settings = cliprelay_core::settings::Settings::new(std::sync::Arc::new(db));
    settings.as_map()
}

fn saved_window_size() -> Option<(f32, f32)> {
    let db_path = cliprelay_core::paths::database_path();
    let db = cliprelay_core::db::Database::open(&db_path).ok()?;
    let settings = cliprelay_core::settings::Settings::new(std::sync::Arc::new(db));
    let value = settings.get_string(WINDOW_BOUNDS).ok()?;
    let parts: Vec<&str> = value.split('x').collect();
    if parts.len() != 2 {
        return None;
    }
    let width: f32 = parts[0].trim().parse().ok()?;
    let height: f32 = parts[1].trim().parse().ok()?;
    if width >= 700.0 && height >= 520.0 && width < 10000.0 && height < 10000.0 {
        Some((width, height))
    } else {
        None
    }
}

/// Apply the relay app icon to the dock (mirrors the original's
/// `setWindowIcon`). The SVG is embedded so the bare binary shows the
/// proper icon without a bundle.
#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)] // the objc 0.2 macros reference the cargo-clippy cfg
fn apply_dock_icon() {
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let svg: &[u8] = include_bytes!("../../../src/cliprelay/assets/cliprelay.svg");
        let app: *mut objc::runtime::Object = msg_send![class!(NSApplication), sharedApplication];
        let data: *mut objc::runtime::Object =
            msg_send![class!(NSData), dataWithBytes: svg.as_ptr() length: svg.len()];
        let image: *mut objc::runtime::Object = msg_send![class!(NSImage), alloc];
        let image: *mut objc::runtime::Object = msg_send![image, initWithData: data];
        let _: () = msg_send![app, setApplicationIconImage: image];
    }
}

#[cfg(not(target_os = "macos"))]
fn apply_dock_icon() {}

#[cfg(target_os = "linux")]
fn should_prefer_x11(
    desktop: Option<&str>,
    display_available: bool,
    wayland_available: bool,
) -> bool {
    display_available
        && wayland_available
        && desktop.is_some_and(|value| {
            value
                .split(':')
                .any(|part| part.eq_ignore_ascii_case("niri"))
        })
}

#[cfg(target_os = "linux")]
fn configure_linux_backend() {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
    let display_available = std::env::var_os("DISPLAY").is_some_and(|value| !value.is_empty());
    let wayland_available =
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty());

    // GPUI 0.2.2's native Wayland Blade path can accept input under niri
    // without presenting the resulting frames. Its X11 path presents
    // reliably through XWayland, so prefer that path when it is available.
    if should_prefer_x11(desktop.as_deref(), display_available, wayland_available) {
        std::env::remove_var("WAYLAND_DISPLAY");
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_linux_backend() {}

fn init_bundled_gstreamer() {
    use std::path::PathBuf;
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let manifest_plugins =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/gst-plugins");
    let manifest_plugins_release =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/release/gst-plugins");
    let candidates = [
        exe_dir.clone().map(|d| d.join("gst-plugins")),
        exe_dir.clone().map(|d| d.join("lib/gstreamer-1.0")),
        exe_dir.clone().map(|d| d.join("../lib/gstreamer-1.0")),
        exe_dir.map(|d| d.join("../lib64/gstreamer-1.0")),
        Some(manifest_plugins),
        Some(manifest_plugins_release),
        Some(PathBuf::from("/tmp/gst_extract/usr/lib/gstreamer-1.0")),
    ];
    for cand in candidates.into_iter().flatten() {
        if cand.exists() {
            let existing = std::env::var("GST_PLUGIN_PATH").unwrap_or_default();
            let new_path = if existing.is_empty() {
                cand.to_string_lossy().to_string()
            } else {
                format!("{}:{}", cand.display(), existing)
            };
            unsafe { std::env::set_var("GST_PLUGIN_PATH", &new_path) };
            log::info!("Using bundled GStreamer plugins from {}", cand.display());
            let scanner_candidates = [
                cand.join("../../libexec/gstreamer-1.0/gst-plugin-scanner"),
                PathBuf::from("/tmp/gst_extract/usr/lib/gstreamer-1.0/gst-plugin-scanner"),
                PathBuf::from("/usr/lib/gstreamer-1.0/gst-plugin-scanner"),
                PathBuf::from("/usr/libexec/gstreamer-1.0/gst-plugin-scanner"),
            ];
            for sc in scanner_candidates {
                if sc.exists() {
                    unsafe { std::env::set_var("GST_PLUGIN_SCANNER", sc) };
                    break;
                }
            }
            break;
        }
    }
}

fn main() {
    init_bundled_gstreamer();
    env_logger::init();
    configure_linux_backend();
    let (mut window_width, mut window_height) = parse_cli_args();
    let cli_size = std::env::args().any(|a| {
        a == "--window-width"
            || a == "--window_width"
            || a == "--window-height"
            || a == "--window_height"
    });
    if !cli_size {
        if let Some((saved_w, saved_h)) = saved_window_size() {
            window_width = saved_w;
            window_height = saved_h;
        }
    }
    Application::new()
        .with_assets(icons::ClipRelayAssets)
        .run(move |app| {
            app.bind_keys([
                KeyBinding::new("tab", FocusNext, Some("cliprelay")),
                KeyBinding::new("shift-tab", FocusPrevious, Some("cliprelay")),
                KeyBinding::new("enter", Activate, Some("cliprelay")),
                KeyBinding::new("space", ActivateSpace, Some("cliprelay")),
            ]);
            // After the platform is up (the gpui registers its NSApplication
            // ivars during init, so the icon must be applied later).
            apply_dock_icon();
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(window_width), px(window_height)),
                    app,
                ))),
                // The original's min is 940x660, but tiling WMs and half-screen
                // layouts routinely give the app less; the adaptive header and
                // toolbar keep everything reachable down to this size.
                window_min_size: Some(size(px(700.0), px(520.0))),
                app_id: Some("cliprelay".to_string()),
                titlebar: Some(TitlebarOptions {
                    title: Some("ClipRelay".into()),
                    // Custom flush title bar: the app's own header row is the
                    // title bar (the native one is hidden), with the traffic
                    // lights parked over it.
                    appears_transparent: true,
                    // Centered in the 40px title bar (measured empirically:
                    // y=27 lands the lights on the bar's vertical midline).
                    traffic_light_position: Some(point(px(18.0), px(27.0))),
                }),
                ..Default::default()
            };
            let window_result = app.open_window(options, |window, cx| {
                let view = cx.new(App::new);
                window.focus(&view.read(cx).focus_handle);
                view
            });
            if let Err(error) = window_result {
                log::error!("ClipRelay could not open its main window: {error:#}");
                eprintln!("ClipRelay could not open its main window: {error:#}");
                app.quit();
                return;
            }
            // Quit when the window closes (gpui does not exit by default).
            app.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
        });
}

mod prepare_render;
mod render_impls;

/// A paged result may be appended only when it belongs to the current
/// generation and is the exact next expected offset (dedupes duplicate and
/// out-of-order pages).
fn page_appendable(
    current_generation: u64,
    current_offset: usize,
    page_generation: u64,
    page_offset: usize,
) -> bool {
    page_generation == current_generation && page_offset == current_offset
}

#[cfg(test)]
mod page_tests {
    use super::page_appendable;

    #[test]
    fn pages_apply_in_order_and_dedupe() {
        // Exact next offset, same generation.
        assert!(page_appendable(3, 100, 3, 100));
        // Duplicate page (same offset) is rejected.
        assert!(!page_appendable(3, 200, 3, 100));
        // Out-of-order page is rejected.
        assert!(!page_appendable(3, 100, 3, 200));
        // Stale generation is rejected.
        assert!(!page_appendable(4, 100, 3, 100));
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_backend_tests {
    use super::should_prefer_x11;

    #[test]
    fn niri_prefers_x11_only_when_both_backends_are_available() {
        assert!(should_prefer_x11(Some("niri"), true, true));
        assert!(should_prefer_x11(Some("GNOME:niri"), true, true));
        assert!(!should_prefer_x11(Some("niri"), false, true));
        assert!(!should_prefer_x11(Some("niri"), true, false));
        assert!(!should_prefer_x11(Some("GNOME"), true, true));
        assert!(!should_prefer_x11(None, true, true));
    }
}
