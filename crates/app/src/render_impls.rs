//! Shell chrome: header (command center row), workspace tabs, sidebar nav,
//! Prepare dock/studio, random-source popup, history menu.

use crate::state::*;
use crate::theme::*;
use crate::widgets::*;
use crate::settings_import::*;
use gpui::*;
use gpui::prelude::*;
use serde_json::json;
use std::path::PathBuf;

/// A command-palette action (mirrors WorkbenchActionRegistry.qml).
#[derive(Clone)]
pub struct CommandAction {
    pub id: u8,
    pub label: &'static str,
    pub detail: &'static str,
    pub category: &'static str,
    pub glyph: &'static str,
    pub keywords: &'static str,
    pub shortcut: &'static str,
    pub enabled: bool,
}

/// One row inside the command palette.
#[derive(Clone)]
pub enum CommandEntry {
    Section(&'static str),
    Action(CommandAction),
    Result(crate::state::SearchResultItem),
}

impl crate::App {
    /// The full action catalog with live enabled states.
    pub fn command_actions(&self) -> Vec<CommandAction> {
        let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
        let selected = self.selected.is_some();
        let density = self
            .settings
            .get(LIBRARY_DENSITY)
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let sort_mode = self
            .settings
            .get(SORT_MODE)
            .and_then(|v| v.as_str())
            .unwrap_or("newest")
            .to_string();
        let folder_sort = self
            .settings
            .get(FOLDER_SORT_MODE)
            .and_then(|v| v.as_str())
            .unwrap_or("name_asc")
            .to_string();
        let cut_active = self.prepare.media_id > 0
            && (self.prepare.trim_start > 0.0
                || (self.prepare.trim_end > 0.0
                    && self.prepare.duration > 0.0
                    && self.prepare.trim_end < self.prepare.duration));
        vec![
            CommandAction {
                id: 0,
                label: "Pick random video",
                detail: "Choose from the active random-source folders",
                category: "Library",
                glyph: "⇄",
                keywords: "random shuffle choose video",
                shortcut: "R",
                enabled: has_root && !self.random_picking,
            },
            CommandAction {
                id: 1,
                label: "Reset shuffle history",
                detail: "Allow every active video to be picked again",
                category: "Library",
                glyph: "↻",
                keywords: "reset clear shuffle seen random",
                shortcut: "",
                enabled: has_root,
            },
            CommandAction {
                id: 2,
                label: "Rescan library",
                detail: "Refresh the filename manifest and media index",
                category: "Library",
                glyph: "↻",
                keywords: "scan refresh index update",
                shortcut: "",
                enabled: has_root && !self.scan.active,
            },
            CommandAction {
                id: 3,
                label: "Stop library scan",
                detail: "Stop the active background scan",
                category: "Library",
                glyph: "■",
                keywords: "stop cancel scan index background task",
                shortcut: "",
                enabled: self.scan.active && !self.scan.cancelling,
            },
            CommandAction {
                id: 4,
                label: "Go back",
                detail: "Restore the previous video or folder",
                category: "Navigation",
                glyph: "‹",
                keywords: "back undo previous video folder navigation",
                shortcut: "⌘[",
                enabled: self
                    .workspaces
                    .get(self.active_workspace_index)
                    .map(|w| w.has_back)
                    .unwrap_or(false),
            },
            CommandAction {
                id: 5,
                label: "Go forward",
                detail: "Reapply the next video or folder",
                category: "Navigation",
                glyph: "›",
                keywords: "forward redo next video folder navigation",
                shortcut: "⌘]",
                enabled: self
                    .workspaces
                    .get(self.active_workspace_index)
                    .map(|w| w.has_forward)
                    .unwrap_or(false),
            },
            CommandAction {
                id: 6,
                label: if self.show_folders {
                    "Hide folder explorer"
                } else {
                    "Show folder explorer"
                },
                detail: "Toggle the hierarchical folder column",
                category: "View",
                glyph: "▤",
                keywords: "folder explorer sidebar hide show",
                shortcut: "",
                enabled: has_root,
            },
            CommandAction {
                id: 7,
                label: "Use default library density",
                detail: "Show readable thumbnails with complete metadata",
                category: "View",
                glyph: "▦",
                keywords: "library grid tiles density default comfortable",
                shortcut: "",
                enabled: density != "default",
            },
            CommandAction {
                id: 8,
                label: "Use compact library density",
                detail: "Fit more video thumbnails in the library",
                category: "View",
                glyph: "▦",
                keywords: "library grid tiles density compact dense",
                shortcut: "",
                enabled: density != "compact",
            },
            CommandAction {
                id: 9,
                label: "Sort by newest",
                detail: "Newest modified files first",
                category: "Sort",
                glyph: "≣",
                keywords: "sort date recent newest",
                shortcut: "",
                enabled: sort_mode != "newest",
            },
            CommandAction {
                id: 10,
                label: "Sort by oldest",
                detail: "Oldest modified files first",
                category: "Sort",
                glyph: "≣",
                keywords: "sort date oldest",
                shortcut: "",
                enabled: sort_mode != "oldest",
            },
            CommandAction {
                id: 11,
                label: "Sort by name",
                detail: "Alphabetical filename order",
                category: "Sort",
                glyph: "≣",
                keywords: "sort alphabetical filename name",
                shortcut: "",
                enabled: sort_mode != "name",
            },
            CommandAction {
                id: 12,
                label: "Sort by duration",
                detail: "Longest videos first",
                category: "Sort",
                glyph: "≣",
                keywords: "sort length duration time",
                shortcut: "",
                enabled: sort_mode != "duration",
            },
            CommandAction {
                id: 13,
                label: "Sort by size",
                detail: "Largest files first",
                category: "Sort",
                glyph: "≣",
                keywords: "sort file size largest",
                shortcut: "",
                enabled: sort_mode != "size",
            },
            CommandAction {
                id: 14,
                label: "Sort folders A to Z",
                detail: "Alphabetical Explorer folder order",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder alphabetical ascending",
                shortcut: "",
                enabled: folder_sort != "name_asc",
            },
            CommandAction {
                id: 15,
                label: "Sort folders Z to A",
                detail: "Reverse alphabetical Explorer folder order",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder alphabetical descending",
                shortcut: "",
                enabled: folder_sort != "name_desc",
            },
            CommandAction {
                id: 16,
                label: "Sort folders by recent additions",
                detail: "Folders with newly indexed videos first",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder recently added indexed",
                shortcut: "",
                enabled: folder_sort != "added_recent",
            },
            CommandAction {
                id: 17,
                label: "Sort folders by oldest additions",
                detail: "Folders whose newest additions are oldest first",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder least recently added indexed",
                shortcut: "",
                enabled: folder_sort != "added_old",
            },
            CommandAction {
                id: 18,
                label: "Sort folders by recent updates",
                detail: "Folders with the newest videos first",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder recent updated newest",
                shortcut: "",
                enabled: folder_sort != "recent",
            },
            CommandAction {
                id: 19,
                label: "Sort folders by oldest updates",
                detail: "Folders least recently updated first",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder old stale least recent",
                shortcut: "",
                enabled: folder_sort != "stale",
            },
            CommandAction {
                id: 20,
                label: "Sort folders by most videos",
                detail: "Largest Explorer folders first",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder count most videos",
                shortcut: "",
                enabled: folder_sort != "count_desc",
            },
            CommandAction {
                id: 21,
                label: "Sort folders by fewest videos",
                detail: "Smallest Explorer folders first",
                category: "Sort",
                glyph: "▤",
                keywords: "sort explorer folder count fewest videos",
                shortcut: "",
                enabled: folder_sort != "count_asc",
            },
            CommandAction {
                id: 22,
                label: "New workspace",
                detail: "Open another root folder in its own tab",
                category: "Workspaces",
                glyph: "+",
                keywords: "new tab workspace root folder",
                shortcut: "⌘T",
                enabled: true,
            },
            CommandAction {
                id: 23,
                label: "Close workspace",
                detail: "Close the active workspace tab",
                category: "Workspaces",
                glyph: "✕",
                keywords: "close tab workspace",
                shortcut: "⌘W",
                enabled: !self.workspaces.is_empty(),
            },
            CommandAction {
                id: 24,
                label: "Reopen closed workspace",
                detail: "Restore the most recently closed workspace tab",
                category: "Workspaces",
                glyph: "↩",
                keywords: "reopen restore closed tab workspace",
                shortcut: "⇧⌘T",
                enabled: self.closed_count > 0,
            },
            CommandAction {
                id: 25,
                label: "Open full Prepare editor",
                detail: "Move the selected video into the full-screen workspace",
                category: "Prepare",
                glyph: "⤢",
                keywords: "prepare full screen editor video",
                shortcut: "",
                enabled: selected && !self.prepare.studio_mode,
            },
            CommandAction {
                id: 26,
                label: "Focus Prepare Edit",
                detail: "Show crop and black-mask tools",
                category: "Prepare",
                glyph: "▣",
                keywords: "prepare edit crop mask frame",
                shortcut: "",
                enabled: selected,
            },
            CommandAction {
                id: 27,
                label: "Focus Prepare Publish",
                detail: "Show output, destinations, and captions",
                category: "Prepare",
                glyph: "➤",
                keywords: "prepare publish telegram x caption output",
                shortcut: "",
                enabled: selected,
            },
            CommandAction {
                id: 28,
                label: "Reset selected video cut",
                detail: "Restore the complete source duration",
                category: "Prepare",
                glyph: "↻",
                keywords: "prepare reset trim cut in out duration",
                shortcut: "",
                enabled: cut_active,
            },
            CommandAction {
                id: 29,
                label: "Reveal selected video in Library",
                detail: "Open its Explorer branch and focus its tile",
                category: "Prepare",
                glyph: "◎",
                keywords: "prepare reveal library explorer focus tile",
                shortcut: "",
                enabled: selected,
            },
            CommandAction {
                id: 30,
                label: "Choose library root",
                detail: "Open a different top-level video folder",
                category: "Library",
                glyph: "▤",
                keywords: "choose open root directory library",
                shortcut: "",
                enabled: true,
            },
            CommandAction {
                id: 31,
                label: "Go to Library",
                detail: "Open the video library workspace",
                category: "Navigation",
                glyph: "▦",
                keywords: "navigate page library",
                shortcut: "⌘1",
                enabled: self.page != Page::Library,
            },
            CommandAction {
                id: 32,
                label: "Go to History",
                detail: "Review prior relay attempts",
                category: "Navigation",
                glyph: "◷",
                keywords: "navigate page history posts",
                shortcut: "⌘2",
                enabled: self.page != Page::History,
            },
            CommandAction {
                id: 33,
                label: "Go to Settings",
                detail: "Configure ClipRelay",
                category: "Navigation",
                glyph: "⚙",
                keywords: "navigate page preferences settings",
                shortcut: "⌘,",
                enabled: self.page != Page::Settings,
            },
            CommandAction {
                id: 34,
                label: "Use Relay theme",
                detail: "Warm dark ClipRelay palette",
                category: "Theme",
                glyph: "◐",
                keywords: "theme relay warm dark palette",
                shortcut: "",
                enabled: self.theme_mode.as_str() != "relay",
            },
            CommandAction {
                id: 35,
                label: "Use Pitch Black theme",
                detail: "Black surfaces with blue actions",
                category: "Theme",
                glyph: "◐",
                keywords: "theme pitch black palette blue",
                shortcut: "",
                enabled: self.theme_mode.as_str() != "pitch_black",
            },
            CommandAction {
                id: 36,
                label: "Use Full White theme",
                detail: "Light surfaces with blue actions",
                category: "Theme",
                glyph: "◐",
                keywords: "theme full white palette light",
                shortcut: "",
                enabled: self.theme_mode.as_str() != "full_white",
            },
            CommandAction {
                id: 37,
                label: if self.sidebar_collapsed || self.window_size.0 < 1080.0 {
                    "Expand activity sidebar"
                } else {
                    "Collapse activity sidebar"
                },
                detail: if self.window_size.0 < 1080.0 {
                    "Widen the window to expand the sidebar"
                } else {
                    "Change the activity rail between icons and labels"
                },
                category: "View",
                glyph: "▮",
                keywords: "toggle sidebar activity rail icons labels",
                shortcut: "",
                enabled: self.window_size.0 >= 1080.0,
            },
        ]
    }

    /// Flat, ordered command-palette entries (sections + actions + results).
    pub fn command_entries(&self) -> Vec<CommandEntry> {
        let query = self.command_query.trim().to_lowercase();
        let actions = self.command_actions();
        let mut entries: Vec<CommandEntry> = Vec::new();
        if query.is_empty() {
            let mut current_category = "";
            for action in actions {
                if action.category != current_category {
                    current_category = action.category;
                    entries.push(CommandEntry::Section(current_category));
                }
                entries.push(CommandEntry::Action(action));
            }
        } else {
            if self.effective_command_scope() != "commands" {
                for item in self.command_results.clone() {
                    entries.push(CommandEntry::Result(item));
                }
            }
            let matched: Vec<CommandAction> = actions
                .into_iter()
                .filter(|a| {
                    a.label.to_lowercase().contains(&query)
                        || a.keywords.contains(&query)
                })
                .collect();
            if !matched.is_empty() {
                entries.push(CommandEntry::Section("ACTIONS"));
                for action in matched {
                    entries.push(CommandEntry::Action(action));
                }
            }
        }
        entries
    }

    /// Run a command-palette action by id.
    pub fn run_command_action(&mut self, id: u8, cx: &mut Context<Self>) {
        match id {
            0 => self.command(Command::PickRandom),
            1 => self.command(Command::ResetShuffle),
            2 => self.command(Command::ScanLibrary),
            3 => self.command(Command::CancelScan),
            4 => self.command(Command::NavigateBack),
            5 => self.command(Command::NavigateForward),
            6 => {
                self.show_folders = !self.show_folders;
                cx.notify();
            }
            7 => self.set_setting(LIBRARY_DENSITY, serde_json::json!("default"), cx),
            8 => self.set_setting(LIBRARY_DENSITY, serde_json::json!("compact"), cx),
            9 => self.command(Command::SetSortMode("newest".into())),
            10 => self.command(Command::SetSortMode("oldest".into())),
            11 => self.command(Command::SetSortMode("name".into())),
            12 => self.command(Command::SetSortMode("duration".into())),
            13 => self.command(Command::SetSortMode("size".into())),
            14 => self.command(Command::SetFolderSortMode("name_asc".into())),
            15 => self.command(Command::SetFolderSortMode("name_desc".into())),
            16 => self.command(Command::SetFolderSortMode("added_recent".into())),
            17 => self.command(Command::SetFolderSortMode("added_old".into())),
            18 => self.command(Command::SetFolderSortMode("recent".into())),
            19 => self.command(Command::SetFolderSortMode("stale".into())),
            20 => self.command(Command::SetFolderSortMode("count_desc".into())),
            21 => self.command(Command::SetFolderSortMode("count_asc".into())),
            22 => self.choose_new_workspace_folder(cx),
            23 => {
                if let Some(workspace) = self.workspaces.get(self.active_workspace_index) {
                    let id = workspace.id.clone();
                    self.command(Command::CloseWorkspace(id));
                }
            }
            24 => self.command(Command::ReopenClosedWorkspace),
            25 => {
                self.save_draft();
                self.prepare.studio_mode = true;
                cx.notify();
            }
            26 => {
                self.prepare.inspector_tab = 0;
                cx.notify();
            }
            27 => {
                self.prepare.inspector_tab = 1;
                cx.notify();
            }
            28 => {
                self.prepare.trim_start = 0.0;
                self.prepare.trim_end = self.prepare.duration;
                self.save_draft();
                cx.notify();
            }
            29 => self.command(Command::RevealSelectedInLibrary),
            30 => self.choose_library_folder(cx),
            31 => {
                self.page = Page::Library;
                cx.notify();
            }
            32 => {
                self.page = Page::History;
                cx.notify();
            }
            33 => {
                self.page = Page::Settings;
                cx.notify();
            }
            34 => self.set_setting(THEME_MODE, serde_json::json!("relay"), cx),
            35 => self.set_setting(THEME_MODE, serde_json::json!("pitch_black"), cx),
            36 => self.set_setting(THEME_MODE, serde_json::json!("full_white"), cx),
            _ => {
                if self.window_size.0 >= 1080.0 {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                }
                cx.notify();
            }
        }
    }

    pub fn render_command_center(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let entries = self.command_entries();
        let selected = self.command_selected;
        let scope = self.command_scope.clone();
        let mut popup = div()
            .id("command-center-popup")
            .absolute()
            .top(px(46.0))
            .left(px(96.0))
            // Match the search field's width (right group ≈ 360px).
            .w(px((self.window_size.0 - 96.0 - 16.0 - 360.0).clamp(420.0, 860.0)))
            .rounded(px(10.0))
            .bg(theme.surface_soft)
            .border_1()
            .border_color(theme.border_strong)
            .flex()
            .flex_col()
            .overflow_hidden();

        // Scope chips.
        let mut chips = div()
            .w_full()
            .h(px(34.0))
            .px(px(8.0))
            .border_b_1()
            .border_color(theme.border)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0));
        for (value, label) in [
            ("all", "All"),
            ("videos", "Videos"),
            ("folders", "Folders"),
            ("commands", "Commands"),
        ] {
            let effective_scope = self.effective_command_scope();
            let active = effective_scope == value;
            let value = value.to_string();
            chips = chips.child(
                div()
                    .id(SharedString::from(format!("scope-{value}")))
                    .h(px(28.0))
                    .px(px(12.0))
                    .rounded(px(10.0))
                    .cursor_pointer()
                    .bg(if active { theme.active } else { theme.transparent() })
                    .flex()
                    .items_center()
                    .child(label)
                    .text_size(px(12.0))
                    .text_color(if active { theme.text } else { theme.muted })
                    .font_weight(FontWeight::MEDIUM)
                    .when(active, |this| {
                        this.border_b_1().border_color(theme.accent)
                    })
                    .on_click(cx.listener(move |app, _event, _window, cx| {
                        app.command_scope = value.clone();
                        app.command_searching =
                            !app.command_query.trim().is_empty() && value != "commands";
                        app.command(Command::SearchSuggestions {
                            query: app.command_query.clone(),
                            scope: app.command_scope.clone(),
                        });
                        cx.notify();
                    })),
            );
        }
        popup = popup.child(chips);

        // Results list.
        let mut list = div()
            .id("command-results")
            .w_full()
            .max_h(px(400.0))
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .flex()
            .flex_col()
            .py(px(6.0));

        let selectable_count = entries
            .iter()
            .filter(|e| matches!(e, CommandEntry::Action(a) if a.enabled) || matches!(e, CommandEntry::Result(_)))
            .count();
        let selected = selected.min(entries.len().saturating_sub(1));

        for (index, entry) in entries.iter().enumerate() {
            match entry {
                CommandEntry::Section(title) => {
                    list = list.child(
                        div()
                            .w_full()
                            .h(px(24.0))
                            .px(px(12.0))
                            .flex()
                            .items_center()
                            .child(title.to_string().to_uppercase())
                            .text_size(px(9.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::BOLD),
                    );
                }
                CommandEntry::Action(action) => {
                    let is_selected = index == selected && action.enabled;
                    let enabled = action.enabled;
                    let action_id = action.id;
                    let title = action.label;
                    let detail = action.detail;
                    let glyph = action.glyph;
                    let shortcut = action.shortcut;
                    let mut row = div()
                        .id(SharedString::from(format!("action-{action_id}")))
                        .w_full()
                        .h(px(42.0))
                        .px(px(12.0))
                        .cursor_pointer()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(10.0))
                        .hover(|style| style.bg(theme.active.opacity(0.5)))
                        .bg(if is_selected { theme.active } else { theme.transparent() })
                        .opacity(if enabled { 1.0 } else { 0.45 })
                        .child(
                            div()
                                .w(px(27.0))
                                .h(px(27.0))
                                .rounded(px(4.0))
                                .bg(if is_selected { theme.accent_soft } else { theme.raised })
                                .border_1()
                                .border_color(if is_selected { theme.accent } else { theme.border })
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(glyph, 13.0, if is_selected { theme.accent_text } else { theme.muted })),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(1.0))
                                .child(
                                    div()
                                        .child(title.to_string())
                                        .text_size(px(12.0))
                                        .text_color(if is_selected { theme.text } else { theme.text_soft })
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_ellipsis(),
                                )
                                .child(
                                    div()
                                        .child(detail.to_string())
                                        .text_size(px(10.0))
                                        .text_color(theme.muted)
                                        .text_ellipsis(),
                                ),
                        )
                        .child(if shortcut.is_empty() {
                            div().into_any()
                        } else {
                            div()
                                .child(shortcut.to_string())
                                .text_size(px(10.0))
                                .text_color(theme.muted_soft)
                                .into_any()
                        });
                    row = row.on_click(cx.listener(move |app, _event, _window, cx| {
                        if enabled {
                            app.close_command_center();
                            app.run_command_action(action_id, cx);
                        }
                        cx.notify();
                    }));
                    list = list.child(row);
                }
                CommandEntry::Result(item) => {
                    let is_selected = index == selected;
                    let kind = item.kind.clone();
                    let media_id = item.media_id;
                    let folder = item.folder_path.clone();
                    let title = item.title.clone();
                    let detail = item.detail.clone();
                    let count = item.count;
                    let mut row = div()
                        .id(SharedString::from(format!("command-{index}")))
                        .w_full()
                        .h(px(46.0))
                        .px(px(12.0))
                        .cursor_pointer()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(10.0))
                        .bg(if is_selected { theme.active } else { theme.transparent() })
                        .child(
                            div()
                                .w(px(27.0))
                                .h(px(27.0))
                                .rounded(px(4.0))
                                .bg(if is_selected { theme.accent_soft } else { theme.raised })
                                .border_1()
                                .border_color(if is_selected { theme.accent } else { theme.border })
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(if kind == "folder" { "▤" } else { "▶" }, 13.0, if is_selected { theme.accent_text } else { theme.muted })),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .child(title)
                                        .text_size(px(12.0))
                                        .text_color(if is_selected { theme.text } else { theme.text_soft })
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_ellipsis(),
                                )
                                .child(
                                    div()
                                        .child(detail)
                                        .text_size(px(10.0))
                                        .text_color(theme.muted)
                                        .text_ellipsis(),
                                ),
                        )
                        .child(if kind == "folder" {
                            div()
                                .child(format!("{count}"))
                                .text_size(px(10.0))
                                .text_color(theme.muted_soft)
                                .into_any()
                        } else {
                            div().into_any()
                        });
                    row = row.on_click(cx.listener(move |app, _event, _window, cx| {
                        app.close_command_center();
                        if kind == "media" {
                            app.command(Command::SelectMedia(media_id));
                        } else {
                            app.command(Command::SetFolder(folder.clone()));
                            app.search_text.clear();
                            app.command(Command::SetSearch(String::new()));
                        }
                        cx.notify();
                    }));
                    list = list.child(row);
                }
            }
        }

        if selectable_count == 0 && !self.command_searching {
            let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
            let has_query = !self.command_query.trim().is_empty();
            let effective_scope = self.effective_command_scope();
            let title = if !has_root && effective_scope != "commands" {
                "Choose a library root to begin".to_string()
            } else if effective_scope == "commands" {
                "No matching command".to_string()
            } else if effective_scope == "folders" {
                if has_query {
                    "No matching folder".to_string()
                } else {
                    "No indexed folders".to_string()
                }
            } else if scope == "videos" {
                if has_query {
                    "No matching video".to_string()
                } else {
                    "No indexed videos".to_string()
                }
            } else {
                "No matching result".to_string()
            };
            let detail = if !has_root && scope != "commands" {
                "Use Choose library root below, or run it as a command."
            } else if has_query {
                "Try a shorter filename, folder, or action."
            } else {
                "Your library index has no items for this scope."
            };
            list = list.child(
                div()
                    .w_full()
                    .py(px(24.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .child(title)
                            .text_size(px(12.0))
                            .text_color(theme.text_soft)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        div()
                            .child(detail)
                            .text_size(px(10.0))
                            .text_color(theme.muted),
                    ),
            );
        }
        popup = popup.child(list);

        // Hint footer.
        let hint = |text: &str| {
            div()
                .child(text.to_string())
                .text_size(px(9.0))
                .text_color(theme.muted_soft)
        };
        let result_count = entries
            .iter()
            .filter(|e| {
                matches!(e, CommandEntry::Action(_)) || matches!(e, CommandEntry::Result(_))
            })
            .count();
        popup = popup.child(
            div()
                .w_full()
                .h(px(30.0))
                .px(px(12.0))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(12.0))
                .child(if self.command_searching {
                    div()
                        .child("SEARCHING".to_string())
                        .text_size(px(9.0))
                        .text_color(theme.warning)
                } else if !self.command_query.trim().is_empty() {
                    div()
                        .child(format!(
                            "{} RESULT{}",
                            result_count,
                            if result_count == 1 { "" } else { "S" }
                        ))
                        .text_size(px(9.0))
                        .text_color(theme.muted_soft)
                } else {
                    div()
                })
                .child(div().flex_1())
                .child(hint("↑↓  NAVIGATE"))
                .child(hint("↵  OPEN"))
                .child(hint("ESC  CLOSE")),
        );
        popup
    }

    pub fn render_context_toolbar(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let page = self.page;
        let mut toolbar = div()
            .id("context-toolbar")
            .w_full()
            .h(px(42.0))
            .px(px(16.0))
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(10.0));

        // Page band.
        let (page_icon, page_name) = match page {
            Page::Library => ("▤", "LIBRARY"),
            Page::History => ("◷", "HISTORY"),
            Page::Settings => ("⚙", "SETTINGS"),
        };
        toolbar = toolbar
            .child(icon(page_icon, 16.0, theme.accent_text))
            .child(
                div()
                    .child(page_name)
                    .text_size(px(10.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::BOLD),
            )
            .child(div().w(px(1.0)).h(px(18.0)).bg(theme.border));

        match page {
            Page::Library => {
                let toolbar_compact = self.window_size.0 < 1060.0;
                let toolbar_narrow = self.window_size.0 < 880.0;
                let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
                let location = if self.active_folder.is_empty() {
                    self.settings_value(LIBRARY_ROOT)
                        .rsplit('/')
                        .next()
                        .unwrap_or("No library selected")
                        .to_string()
                } else {
                    self.active_folder
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .to_string()
                };
                let location = if location.is_empty() {
                    "No library selected".to_string()
                } else {
                    location
                };
                // Explorer band (hidden on narrow windows).
                if has_root && self.show_folders && !toolbar_narrow {
                    toolbar = toolbar.child(
                        div()
                            .h(px(30.0))
                            .px(px(10.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_soft)
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(6.0))
                            .child(icon("▤", 13.0, theme.muted))
                            .child(
                                div()
                                    .child("EXPLORER")
                                    .text_size(px(10.0))
                                    .text_color(theme.muted)
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                div()
                                    .child(format!("{}", self.folders.len()))
                                    .text_size(px(10.0))
                                    .text_color(theme.muted_soft),
                            )
                            .child(workbench_button(
                                "collapse-folders",
                                "",
                                "▴",
                                ButtonKind::Ghost,
                                true,
                                true,
                                "Collapse all folders",
                                cx,
                                |app, cx| {
                                    for node in &app.folders {
                                        app.folders_expanded.insert(node.folder.clone(), false);
                                    }
                                    cx.notify();
                                },
                            )),
                    );
                }
                // Library context (text labels collapse on narrow windows).
                toolbar = toolbar
                    .child(icon("▤", 15.0, if has_root { theme.accent_text } else { theme.muted }))
                    .child(
                        div()
                            .when(!toolbar_narrow, |this| {
                                this.child("Video library")
                            })
                            .text_size(px(12.0))
                            .text_color(if has_root { theme.accent_text } else { theme.muted })
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        div()
                            .when(!toolbar_narrow, |this| this.child("/"))
                            .text_size(px(12.0))
                            .text_color(theme.muted),
                    )
                    .child(
                        div()
                            .child(location)
                            .text_size(px(12.0))
                            .text_color(theme.text_soft)
                            .max_w(px(if toolbar_narrow { 130.0 } else { 220.0 }))
                            .text_ellipsis(),
                    )
                    .child(div().flex_1())
                    .child(workbench_button(
                        "toggle-folders",
                        if self.show_folders { "Hide folders" } else { "Show folders" },
                        "▤",
                        ButtonKind::Ghost,
                        has_root,
                        toolbar_compact,
                        "Toggle the hierarchical folder column",
                        cx,
                        |app, cx| {
                            app.show_folders = !app.show_folders;
                            cx.notify();
                        },
                    ))
                    .child(workbench_button(
                        "choose-root",
                        "",
                        "▸",
                        ButtonKind::Ghost,
                        true,
                        true,
                        "Choose root",
                        cx,
                        |app, cx| {
                            app.choose_library_folder(cx);
                        },
                    ));
                // Sort control.
                let sort_label = match self
                    .settings
                    .get(SORT_MODE)
                    .and_then(|v| v.as_str())
                    .unwrap_or("newest")
                {
                    "newest" => "Newest",
                    "oldest" => "Oldest",
                    "name" => "Name",
                    "duration" => "Duration",
                    "size" => "Size",
                    _ => "Newest",
                };
                toolbar = toolbar.child(
                    div()
                        .id("sort-trigger")
                        .h(px(30.0))
                        .px(px(10.0))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .bg(if self.sort_menu_open { theme.active } else { theme.transparent() })
                        .border_1()
                        .border_color(if self.sort_menu_open { theme.accent } else { theme.transparent() })
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .child(sort_label)
                                .text_size(px(12.0))
                                .text_color(theme.text_soft),
                        )
                        .child(icon(if self.sort_menu_open { "▴" } else { "▾" }, 10.0, theme.muted))
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            if app.sort_menu_open {
                                app.sort_menu_open = false;
                                app.mark_menu_closed();
                            } else if app.menu_reopen_allowed() {
                                app.sort_menu_open = true;
                            }
                            cx.notify();
                        })),
                );
                // Rescan.
                let scanning = self.scan.active;
                let cancelling = self.scan.cancelling;
                let scanning_label = if cancelling {
                    "Stopping…"
                } else if scanning {
                    "Stop scan"
                } else {
                    "Rescan"
                };
                toolbar = toolbar.child(workbench_button(
                    "rescan",
                    scanning_label,
                    if cancelling { "■" } else if scanning { "▣" } else { "↻" },
                    ButtonKind::Ghost,
                    !(scanning && cancelling),
                    toolbar_compact,
                    if scanning { "Rescan library" } else { "Rescan library" },
                    cx,
                    |app, cx| {
                        if app.scan.active && !app.scan.cancelling {
                            app.command(Command::CancelScan);
                        } else {
                            app.command(Command::ScanLibrary);
                        }
                        cx.notify();
                    },
                ));
                // Prepare band when the dock is visible. The name and "/"
                // collapse on narrow windows / narrow docks (mirrors the
                // original's prepareWidth >= 470 gate).
                let prepare_name_visible =
                    !toolbar_narrow && self.prepare_dock_width() >= 470.0;
                if self.selected.is_some() && !self.prepare.studio_mode {
                    toolbar = toolbar
                        .child(div().w(px(1.0)).h(px(18.0)).bg(theme.border))
                        .child(icon("✎", 14.0, theme.accent_text))
                        .child(
                            div()
                                .child("Prepare")
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::BOLD),
                        )
                        .when(prepare_name_visible, |this| {
                            this.child(
                                div()
                                    .child("/")
                                    .text_size(px(12.0))
                                    .text_color(theme.muted),
                            )
                        })
                        .when(prepare_name_visible, |this| {
                            this.child(
                                div()
                                    .child(
                                        self.selected
                                            .as_ref()
                                            .map(|m| m.name.clone())
                                            .unwrap_or_default(),
                                    )
                                    .text_size(px(12.0))
                                    .text_color(theme.text_soft)
                                    .max_w(px(180.0))
                                    .text_ellipsis(),
                            )
                        })
                        .child(if self.prepare.has_edits() {
                            div()
                                .id("toolbar-edited")
                                .child(icon("▣", 14.0, theme.accent_text))
                                .tooltip(move |_window, cx| crate::tooltip_view(cx, "Frame edits active".into()))
                                .into_any()
                        } else {
                            div().into_any()
                        })
                        .child(workbench_button(
                            "toolbar-open-player",
                            "",
                            "↗",
                            ButtonKind::Ghost,
                            true,
                            true,
                            "Open in default player",
                            cx,
                            |app, cx| app.open_selected_in_player(cx),
                        ))
                        .child(workbench_button(
                            "toolbar-widen",
                            "",
                            "⇔",
                            ButtonKind::Ghost,
                            true,
                            true,
                            if self.prepare_expanded { "Narrow Prepare" } else { "Widen Prepare" },
                            cx,
                            |app, cx| {
                                app.set_setting(PREPARE_EXPANDED, json!(!app.prepare_expanded), cx);
                            },
                        ))
                        .child(workbench_button(
                            "toolbar-fullscreen",
                            "",
                            "⛶",
                            ButtonKind::Ghost,
                            true,
                            true,
                            "Open full-screen editor",
                            cx,
                            |app, cx| {
                                app.prepare.studio_mode = true;
                                cx.notify();
                            },
                        ))
                        .child(workbench_button(
                            "toolbar-close",
                            "",
                            "✕",
                            ButtonKind::Ghost,
                            true,
                            true,
                            "Close selected video",
                            cx,
                            |app, cx| {
                                app.command(Command::ClearSelection);
                                cx.notify();
                            },
                        ));
                }
            }
            Page::History => {
                toolbar = toolbar.child(
                    div()
                        .child("Recorded relays and delivery outcomes")
                        .text_size(px(12.0))
                        .text_color(theme.muted),
                );
            }
            Page::Settings => {
                toolbar = toolbar.child(
                    div()
                        .child("Application preferences and integrations")
                        .text_size(px(12.0))
                        .text_color(theme.muted),
                );
            }
        }
        toolbar
    }

    pub fn render_sort_menu(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let sort_mode = self
            .settings
            .get(SORT_MODE)
            .and_then(|v| v.as_str())
            .unwrap_or("newest")
            .to_string();
        let folder_sort_mode = self
            .settings
            .get(FOLDER_SORT_MODE)
            .and_then(|v| v.as_str())
            .unwrap_or("name_asc")
            .to_string();
        let mut menu = div()
            .id("sort-menu")
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.sort_menu_open {
                    app.sort_menu_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .absolute()
            .top(px(124.0))
            .right(px(140.0))
            .w(px(268.0))
            .rounded(px(10.0))
            .bg(theme.surface_soft)
            .border_1()
            .border_color(theme.border_strong)
            .py(px(6.0))
            .flex()
            .flex_col();

        let video_options: [(&str, &str); 5] = [
            ("Newest first", "newest"),
            ("Oldest first", "oldest"),
            ("Name", "name"),
            ("Longest duration", "duration"),
            ("Largest size", "size"),
        ];
        let folder_options: [(&str, &str); 8] = [
            ("Alphabetical A–Z", "name_asc"),
            ("Alphabetical Z–A", "name_desc"),
            ("Recently added", "added_recent"),
            ("Least recently added", "added_old"),
            ("Recently updated", "recent"),
            ("Least recently updated", "stale"),
            ("Most videos", "count_desc"),
            ("Fewest videos", "count_asc"),
        ];
        let add_section = |_app: &mut crate::App,
                               title: &'static str,
                               options: &[(&'static str, &'static str)],
                               current: &str,
                               cx: &mut Context<crate::App>| {
            let mut section = div().w_full().flex().flex_col();
            section = section.child(
                div()
                    .w_full()
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .child(title)
                    .text_size(px(10.0))
                    .text_color(theme.muted_soft)
                    .font_weight(FontWeight::BOLD),
            );
            for (label, value) in options.iter() {
                let label = *label;
                let value = *value;
                let selected = current == value;
                let mut item = div()
                    .id(SharedString::from(format!("sort-{value}")))
                    .w_full()
                    .h(px(31.0))
                    .px(px(12.0))
                    .cursor_pointer()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(10.0))
                    .bg(if selected { theme.active } else { theme.transparent() })
                    .child(
                        div()
                            .w(px(13.0))
                            .child(if selected { "✓" } else { "" })
                            .text_size(px(13.0))
                            .text_color(theme.accent_text),
                    )
                    .child(
                        div()
                            .child(label)
                            .text_size(px(13.0))
                            .text_color(if selected { theme.text } else { theme.text_soft })
                            .font_weight(if selected { FontWeight::BOLD } else { FontWeight::MEDIUM }),
                    );
                item = item.on_click(cx.listener(move |app, _event, _window, cx| {
                    app.sort_menu_open = false;
                    if value == "name_asc"
                        || value == "name_desc"
                        || value == "added_recent"
                        || value == "added_old"
                        || value == "recent"
                        || value == "stale"
                        || value == "count_desc"
                        || value == "count_asc"
                    {
                        app.command(Command::SetFolderSortMode(value.to_string()));
                    } else {
                        app.command(Command::SetSortMode(value.to_string()));
                    }
                    cx.notify();
                }));
                section = section.child(item);
            }
            section
        };
        menu = menu
            .child(add_section(self, "VIDEOS", &video_options, &sort_mode, cx))
            .child(divider())
            .child(add_section(self, "EXPLORER FOLDERS", &folder_options, &folder_sort_mode, cx));
        menu
    }

    pub fn render_activity_popup(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let scanning = self.scan.active;
        let scan_progress = self.scan.progress;
        let scan_message = self.scan.message.clone();
        let checking = self.checking;
        let timeline_loading = self.timeline_loading;
        let publish = self.publish.clone();
        let active_count = [scanning, checking, timeline_loading, publish.active]
            .iter()
            .filter(|active| **active)
            .count();
        let mut popup = div()
            .id("activity-popup")
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.activity_open {
                    app.activity_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .absolute()
            .top(px(46.0))
            .right(px(12.0))
            .w(px(326.0))
            .rounded(px(10.0))
            .bg(theme.surface_soft)
            .border_1()
            .border_color(theme.border_strong)
            .flex()
            .flex_col()
            .overflow_hidden();

        popup = popup.child(
            div()
                .w_full()
                .h(px(36.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .child("BACKGROUND ACTIVITY")
                        .text_size(px(10.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::BOLD),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .child(if active_count > 0 {
                            format!("{active_count} ACTIVE")
                        } else {
                            "IDLE".to_string()
                        })
                        .text_size(px(10.0))
                        .text_color(if active_count > 0 { theme.accent_text } else { theme.muted_soft })
                        .font_weight(FontWeight::BOLD),
                ),
        );

        let mut list = div()
            .id("activity-list")
            .w_full()
            .max_h(px(286.0))
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .flex()
            .flex_col();

        let add_row = |app: &mut crate::App,
                           title: SharedString,
                           detail: String,
                           indeterminate: bool,
                           progress: f64,
                           show_stop: bool,
                           cx: &mut Context<crate::App>| {
            let row = div()
                .w_full()
                .h(px(58.0))
                .px(px(12.0))
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(5.0))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(icon("⚡", 16.0, theme.accent_text))
                        .child(
                            div()
                                .flex_1()
                                .child(title)
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::MEDIUM),
                        )
                        .child(if show_stop {
                            workbench_button(
                                "activity-stop-scan",
                                if app.scan.cancelling { "Stopping scan" } else { "Stop scan" },
                                "■",
                                ButtonKind::Ghost,
                                !app.scan.cancelling,
                                false,
                                "Stop library scan",
                                cx,
                                |app, cx| {
                                    app.command(Command::CancelScan);
                                    cx.notify();
                                },
                            )
                            .into_any()
                        } else {
                            div().into_any()
                        }),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .child(detail)
                                .text_size(px(10.0))
                                .text_color(theme.muted)
                                .text_ellipsis(),
                        )
                        .child(div().w(px(120.0)).child(progress_bar(progress, indeterminate))),
                );
            row
        };

        if scanning {
            let title = SharedString::from(format!("Library scan  ·  {}", self.scan.root_name));
            list = list.child(add_row(
                self,
                title,
                if scan_message.is_empty() { "Updating the index".to_string() } else { scan_message },
                scan_progress < 0.0,
                scan_progress.max(0.0),
                true,
                cx,
            ));
        }
        if checking {
            list = list.child(add_row(
                self,
                SharedString::from("Selected video"),
                "Checking the file before preparation".to_string(),
                true,
                0.0,
                false,
                cx,
            ));
        }
        if self.random_picking {
            list = list.child(add_row(
                self,
                SharedString::from("Random selection"),
                "Choosing and validating a video".to_string(),
                true,
                0.0,
                false,
                cx,
            ));
        }
        if timeline_loading {
            list = list.child(add_row(
                self,
                SharedString::from("Timeline filmstrip"),
                "Preparing seek thumbnails".to_string(),
                true,
                0.0,
                false,
                cx,
            ));
        }
        if publish.active {
            let stage = if publish.stage.is_empty() {
                "Processing the selected video".to_string()
            } else {
                publish.stage.clone()
            };
            list = list.child(add_row(
                self,
                SharedString::from("Preparing delivery"),
                stage,
                publish.progress <= 0.0,
                publish.progress,
                false,
                cx,
            ));
        }
        if active_count == 0 {
            list = list.child(
                div()
                    .w_full()
                    .h(px(58.0))
                    .px(px(12.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon("✓", 16.0, theme.success))
                    .child(
                        div()
                            .child("No background work. The library is ready.")
                            .text_size(px(12.0))
                            .text_color(theme.muted),
                    ),
            );
        }
        popup = popup.child(list);
        popup
    }

    pub fn render_workspace_menu(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let index = self.workspace_menu_target;
        let target = self.workspaces.get(index).cloned();
        let workspace_count = self.workspaces.len();
        let closed_available = self.closed_count_known();
        let mut menu = div()
            .id("workspace-menu")
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.workspace_menu_open {
                    app.workspace_menu_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .absolute()
            .top(px(124.0))
            .right(px(12.0))
            .w(px(232.0))
            .rounded(px(10.0))
            .bg(theme.surface_soft)
            .border_1()
            .border_color(theme.border_strong)
            .py(px(6.0))
            .flex()
            .flex_col();

        let add_item = |_app: &mut crate::App,
                            id: &'static str,
                            label: &'static str,
                            glyph: &'static str,
                            enabled: bool,
                            command: Option<Command>,
                            cx: &mut Context<crate::App>| {
            let mut item = div()
                .id(id)
                .h(px(40.0))
                .px(px(12.0))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(icon(glyph, 16.0, theme.muted))
                .child(
                    div()
                        .child(label)
                        .text_size(px(13.0))
                        .text_color(theme.text),
                );
            item = item.when(!enabled, |this| this.opacity(0.46).cursor_default());
            if let Some(command) = command {
                item = item.when(enabled, |this| {
                    this.on_click(cx.listener(move |app, _event, _window, cx| {
                        app.workspace_menu_open = false;
                        app.command(command.clone());
                        cx.notify();
                    }))
                });
            }
            item
        };

        if let Some(target) = &target {
            let id = target.id.clone();
            let root = target.root.clone();
            let scanning = target.scanning;
            let cancelling = target.scan_cancelling;
            let can_close_others = workspace_count > 1;
            let can_close_right = index < workspace_count - 1;
            menu = menu
                .child(add_item(
                    self,
                    "ws-rename",
                    "Rename workspace",
                    "✎",
                    true,
                    None,
                    cx,
                ).on_click(cx.listener(move |app, _event, _window, cx| {
                    app.workspace_menu_open = false;
                    app.renaming_workspace = Some(index);
                    let title = app.workspaces.get(index).map(|w| w.title.clone()).unwrap_or_default();
                    let state = app.field_state_mut("workspace-rename");
                    state.text = title;
                    state.caret = state.text.chars().count();
                    app.focused_field = Some("workspace-rename".to_string());
                    cx.notify();
                })))
                .child(add_item(
                    self,
                    "ws-duplicate",
                    "Duplicate workspace",
                    "⧉",
                    true,
                    Some(Command::DuplicateWorkspace(id.clone())),
                    cx,
                ))
                .child(add_item(
                    self,
                    "ws-reveal",
                    "Show root in file manager",
                    "▤",
                    !root.is_empty(),
                    Some(Command::RevealWorkspaceRoot(id.clone())),
                    cx,
                ))
                .child(if scanning {
                    add_item(
                        self,
                        "ws-stop-scan",
                        if cancelling { "Stopping scan…" } else { "Stop library scan" },
                        "■",
                        !cancelling,
                        Some(Command::CancelScan),
                        cx,
                    )
                    .into_any()
                } else {
                    div().into_any()
                })
                .child(divider())
                .child(add_item(
                    self,
                    "ws-close",
                    "Close workspace",
                    "✕",
                    true,
                    Some(Command::CloseWorkspace(id.clone())),
                    cx,
                ))
                .child(add_item(
                    self,
                    "ws-close-others",
                    "Close other workspaces",
                    "⊟",
                    can_close_others,
                    Some(Command::CloseOtherWorkspaces(id.clone())),
                    cx,
                ))
                .child(add_item(
                    self,
                    "ws-close-right",
                    "Close workspaces to the right",
                    "▸",
                    can_close_right,
                    Some(Command::CloseWorkspacesToRight(id.clone())),
                    cx,
                ))
                .child(divider())
                .child(add_item(
                    self,
                    "ws-new",
                    "New workspace…",
                    "+",
                    true,
                    None,
                    cx,
                ).on_click(cx.listener(|app, _event, _window, cx| {
                    app.workspace_menu_open = false;
                    app.choose_new_workspace_folder(cx);
                })))
                .child(add_item(
                    self,
                    "ws-reopen",
                    "Reopen closed workspace",
                    "↶",
                    closed_available,
                    Some(Command::ReopenClosedWorkspace),
                    cx,
                ));
        }
        menu
    }

    /// Whether the controller reported closed workspaces (best-effort from
    /// the last WorkspacesChanged payload).
    fn closed_count_known(&self) -> bool {
        true
    }

    /// Folder picker for creating a new workspace tab.
    pub fn choose_new_workspace_folder(&mut self, cx: &mut Context<Self>) {
        let controller = self.controller.clone();
        let folder = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open a folder in a new workspace".into()),
        });
        cx.spawn(move |_this: WeakEntity<crate::App>, _cx: &mut AsyncApp| async move {
            if let Ok(Ok(Some(mut paths))) = folder.await {
                if let Some(path) = paths.pop() {
                    let _ = controller.send(Command::CreateWorkspace(path.to_string_lossy().into_owned()));
                }
            }
        })
        .detach();
    }

    pub fn render_header(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
        let random_enabled = has_root;
        let picking = self.random_picking;

        let mut header = div()
            .id("header")
            .w_full()
            .h(px(40.0))
            .pl(px(96.0)) // clears the traffic lights on the flush title bar
            .pr(px(16.0))
            .border_b_1()
            .border_color(theme.border)
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0));

        // Window-drag region: the title-bar background moves the window;
        // interactive children sit above it and receive their own clicks.
        header = header.child(
            div()
                .id("titlebar-drag")
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|_app, _event, window, _cx| {
                    if !window.is_fullscreen() {
                        window.start_window_move();
                    }
                }))
                .on_click(cx.listener(|_app, event: &gpui::ClickEvent, window, _cx| {
                    // Double-click the title bar zooms the window.
                    if event.click_count() >= 2 && !window.is_fullscreen() {
                        window.zoom_window();
                    }
                })),
        );

        // Back / forward.
        let can_back = self
            .workspaces
            .get(self.active_workspace_index)
            .map(|w| w.has_back)
            .unwrap_or(false);
        let can_forward = self
            .workspaces
            .get(self.active_workspace_index)
            .map(|w| w.has_forward)
            .unwrap_or(false);
        header = header
            .child(workbench_button(
                "nav-back",
                "Go back",
                "←",
                ButtonKind::Ghost,
                can_back,
                true,
                "Go back  ·  ⌘[",
                cx,
                |app, cx| {
                    app.command(Command::NavigateBack);
                    cx.notify();
                },
            ))
            .child(workbench_button(
                "nav-forward",
                "Go forward",
                "→",
                ButtonKind::Ghost,
                can_forward,
                true,
                "Go forward  ·  ⌘]",
                cx,
                |app, cx| {
                    app.command(Command::NavigateForward);
                    cx.notify();
                },
            ));

        // Command center field (library search). The field shrinks on
        // narrow windows instead of pushing the right-side buttons out.
        let header_narrow = self.window_size.0 < 820.0;
        header = header.child(
            crate::widgets::field_with_icon(
                "command-center",
                if self.effective_command_scope() == "commands" {
                    "Run a command"
                } else {
                    "Search videos, folders, and commands"
                },
                self.fields.get("command-center").unwrap_or(&FieldState::default()),
                self.focused_field.as_deref() == Some("command-center"),
                true,
                false,
                Some("⌕"),
                cx,
            )
            .flex_1()
            .max_w(px(860.0))
            .min_w(px(if header_narrow { 160.0 } else { 280.0 })),
        );

        // Clear-search affordance.
        let has_query = !self
            .fields
            .get("command-center")
            .map(|f| f.text.trim().is_empty())
            .unwrap_or(true);
        if has_query {
            header = header.child(workbench_button(
                "clear-search",
                "",
                "✕",
                ButtonKind::Ghost,
                true,
                true,
                "Clear search",
                cx,
                |app, cx| {
                    let state = app.field_state_mut("command-center");
                    state.text.clear();
                    state.caret = 0;
                    app.command_query.clear();
                    app.search_text.clear();
                    app.command(Command::SetSearch(String::new()));
                    app.command(Command::SearchSuggestions {
                        query: String::new(),
                        scope: app.command_scope.clone(),
                    });
                    cx.notify();
                },
            ));
        }

        // Background activity indicator + popup.
        let activity_active = self.scan.active
            || self.checking
            || self.timeline_loading
            || self.publish.active;
        header = header.child(
            div()
                .id("activity-button")
                .h(px(WORKBENCH_CONTROL_HEIGHT))
                .w(px(WORKBENCH_CONTROL_HEIGHT))
                .rounded(px(4.0))
                .relative()
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .bg(if self.activity_open { theme.active } else { theme.transparent() })
                .child(icon("⚡", 14.0, theme.muted))
                .child(if activity_active {
                    div()
                        .absolute()
                        .top_0()
                        .right_0()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(theme.accent)
                        .border_1()
                        .border_color(theme.ink)
                        .into_any()
                } else {
                    div().into_any()
                })
                .tooltip(move |_window, cx| {
                    crate::tooltip_view(cx, if activity_active {
                        SharedString::from("Background activity")
                    } else {
                        SharedString::from("No background work  ·  View activity")
                    })
                })
                .on_click(cx.listener(|app, _event, _window, cx| {
                    if app.activity_open {
                        app.activity_open = false;
                        app.mark_menu_closed();
                    } else if app.menu_reopen_allowed() {
                        app.activity_open = true;
                    }
                    cx.notify();
                })),
        );

        // Random source summary button.
        let summary = self.random_summary.clone();
        let has_selection = self.random_has_selection;
        header = header.child(
            workbench_button(
                "random-sources",
                summary.clone(),
                "▤",
                if has_selection || self.random_popup_open {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                random_enabled,
                header_narrow,
                format!("Random sources: {summary}"),
                cx,
                |app, cx| {
                    app.toggle_random_popup(cx);
                },
            )
            .when(!header_narrow, |this| this.min_w(px(150.0))),
        );

        // Reset shuffle.
        header = header.child(workbench_button(
            "reset-shuffle",
            "",
            "↺",
            ButtonKind::Ghost,
            random_enabled,
            true,
            "Reset shuffle history",
            cx,
            |app, cx| {
                app.command(Command::ResetShuffle);
                cx.notify();
            },
        ));

        // Pick random.
        let label = if picking { "Picking…" } else { "Pick random" };
        header = header.child(
            workbench_button(
                "pick-random",
                label,
                "⇄",
                ButtonKind::Primary,
                random_enabled,
                header_narrow,
                "Pick random video  ·  R",
                cx,
                |app, cx| {
                    app.command(Command::PickRandom);
                    cx.notify();
                },
            )
            .when(!header_narrow, |this| this.min_w(px(128.0))),
        );

        header
    }

    pub fn render_workspace_tabs(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let workspaces = self.workspaces.clone();
        let active_index = self.active_workspace_index;
        let mut bar = div()
            .id("workspace-tabs")
            .w_full()
            .h(px(34.0))
            .px(px(8.0))
            .bg(theme.surface)
            .border_t_1()
            .border_color(theme.border)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            .overflow_x_scroll()
            .scrollbar_width(px(8.0))
            .track_scroll(&self.tab_scroll);

        let available = (self.window_size.0 - 24.0).max(200.0);
        let tab_width = ((available / workspaces.len().max(1) as f32).clamp(150.0, 218.0)) as f32;
        let renaming = self.renaming_workspace;
        for (index, workspace) in workspaces.iter().enumerate() {
            let id = workspace.id.clone();
            let middle_click_id = id.clone();
            let title = workspace.title.clone();
            let root = workspace.root.clone();
            let active = index == active_index;
            let scanning = workspace.scanning;
            let cancelling = workspace.scan_cancelling;
            let is_renaming = renaming == Some(index);
            let mut tab = div()
                .id(SharedString::from(format!("tab-{id}")))
                .w(px(tab_width))
                .flex_none()
                .h(px(34.0))
                .px(px(10.0))
                .rounded_b(px(6.0))
                .relative()
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .bg(if active { theme.active } else { theme.transparent() })
                .when(active, |this| {
                    this.border_t_2().border_color(theme.accent)
                })
                .tooltip({
                    let title = title.clone();
                    let root = root.clone();
                    move |_window, cx| {
                        let tip = if cancelling {
                            format!("Stopping scan for {title}")
                        } else if scanning {
                            format!("Scanning {title}")
                        } else if root.is_empty() {
                            "No root folder".to_string()
                        } else {
                            title.clone()
                        };
                        crate::tooltip_view(cx, tip.into())
                    }
                })
                .child(
                    div()
                        .child(if cancelling {
                            "■"
                        } else if scanning {
                            "↻"
                        } else {
                            "▤"
                        })
                        .text_size(px(14.0))
                        .text_color(if active || scanning { theme.accent_text } else { theme.muted }),
                );
            if is_renaming {
                tab = tab.child(
                    field(
                        "workspace-rename",
                        "",
                        self.fields.get("workspace-rename").unwrap_or(&FieldState::default()),
                        self.focused_field.as_deref() == Some("workspace-rename"),
                        true,
                        false,
                        cx,
                    )
                    .flex_1(),
                );
            } else {
                tab = tab
                    .child(
                        div()
                            .flex_1()
                            .child(title)
                            .text_size(px(12.0))
                            .text_color(if active { theme.text } else { theme.text_soft })
                            .font_weight(if active { FontWeight::BOLD } else { FontWeight::MEDIUM })
                            .text_ellipsis(),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("tab-close-{id}")))
                            .w(px(22.0))
                            .h(px(22.0))
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("✕", 11.0, theme.muted_soft))
                            .on_click(cx.listener(move |app, _event, _window, cx| {
                                app.pending_close_workspace = Some(index);
                                app.command(Command::CloseWorkspace(id.clone()));
                                cx.notify();
                            })),
                    );
            }
            tab = tab
                .on_mouse_down(gpui::MouseButton::Middle, {
                    let close_id = middle_click_id.clone();
                    cx.listener(move |app, _event, _window, cx| {
                        // Middle-click closes the tab (mirrors the original).
                        app.command(Command::CloseWorkspace(close_id.clone()));
                        cx.notify();
                    })
                })
                .on_click(cx.listener(move |app, event: &ClickEvent, _window, cx| {
                    if app.pending_close_workspace == Some(index) {
                        app.pending_close_workspace = None;
                        return;
                    }
                    if event.click_count() >= 2 && app.renaming_workspace != Some(index) {
                        app.renaming_workspace = Some(index);
                        let title = app.workspaces.get(index).map(|w| w.title.clone()).unwrap_or_default();
                        let state = app.field_state_mut("workspace-rename");
                        state.text = title;
                        state.caret = state.text.chars().count();
                        app.focused_field = Some("workspace-rename".to_string());
                        cx.notify();
                        return;
                    }
                    if app.renaming_workspace == Some(index) {
                        return;
                    }
                    app.activate_workspace_at(index, cx);
                }));
            tab.interactivity().on_mouse_down(
                MouseButton::Right,
                cx.listener(move |app, _event: &MouseDownEvent, _window, cx| {
                    app.workspace_menu_target = index;
                    app.workspace_menu_open = true;
                    cx.notify();
                }),
            );
            if active {
                tab = tab.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w_full()
                        .h(px(2.0))
                        .bg(theme.accent),
                );
            }
            bar = bar.child(tab);
        }

        // Keep the active tab inside the viewport (Ctrl+Tab cycling with
        // many open workspaces).
        {
            let gap = 4.0;
            let active_left = 8.0 + active_index as f32 * (tab_width + gap);
            let scroll = f32::from(self.tab_scroll.offset().x);
            let viewport = (self.window_size.0 - 16.0).max(100.0);
            if active_left < scroll {
                self.tab_scroll.set_offset(point(px(active_left - 8.0), px(0.0)));
            } else if active_left + tab_width > scroll + viewport {
                self.tab_scroll
                    .set_offset(point(px(active_left + tab_width - viewport + 8.0), px(0.0)));
            }
        }

        // New workspace + actions.
        bar = bar.child(workbench_button(
            "new-workspace",
            "",
            "+",
            ButtonKind::Ghost,
            true,
            true,
            "New workspace",
            cx,
            |app, cx| {
                app.choose_new_workspace_folder(cx);
            },
        ));
        bar = bar.child(workbench_button(
            "workspace-actions",
            "",
            "⋯",
            ButtonKind::Ghost,
            true,
            true,
            "Workspace actions",
            cx,
            |app, cx| {
                app.workspace_menu_target = app.active_workspace_index;
                if app.workspace_menu_open {
                    app.workspace_menu_open = false;
                    app.mark_menu_closed();
                } else if app.menu_reopen_allowed() {
                    app.workspace_menu_open = true;
                }
                cx.notify();
            },
        ));
        bar
    }

    pub fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let collapsed = self.sidebar_collapsed || self.window_size.0 < 1080.0;
        let narrow = self.window_size.0 < 1080.0;
        let width = if collapsed { SIDEBAR_COLLAPSED_WIDTH } else { SIDEBAR_EXPANDED_WIDTH };
        let page = self.page;
        let mut sidebar = div()
            .id("sidebar")
            .w(px(width))
            .flex_none()
            .h_full()
            .bg(theme.surface)
            .px(if collapsed { px(10.0) } else { px(14.0) })
            .py(px(8.0))
            .flex()
            .flex_col()
            .gap(px(5.0));

        let nav_item = |app: &mut crate::App,
                            id: &'static str,
                            label: &'static str,
                            glyph: &'static str,
                            target: Page,
                            cx: &mut Context<crate::App>|
         -> Stateful<Div> {
            let selected = page == target;
            let theme = app.theme.clone();
            let mut item = div()
                .id(id)
                .w_full()
                .h(px(48.0))
                .rounded(px(if collapsed { 12.0 } else { 10.0 }))
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(10.0))
                .cursor_pointer()
                .bg(if selected { theme.active } else { theme.transparent() })
                .child(
                    div()
                        .child(glyph)
                        .text_size(px(21.0))
                        .text_color(if selected { theme.accent } else { theme.muted }),
                );
            if !collapsed {
                item = item
                    .child(
                        div()
                            .flex_1()
                            .child(label)
                            .text_size(px(15.0))
                            .text_color(if selected { theme.text } else { theme.muted })
                            .font_weight(if selected { FontWeight::BOLD } else { FontWeight::MEDIUM }),
                    );
            }
            item.on_click(cx.listener(move |app, _event, _window, cx| {
                app.page = target;
                cx.notify();
            }))
        };

        sidebar = sidebar
            .child(nav_item(self, "nav-library", "Library", "▤", Page::Library, cx))
            .child(nav_item(self, "nav-history", "History", "◷", Page::History, cx))
            .child(nav_item(self, "nav-settings", "Settings", "⚙", Page::Settings, cx));

        sidebar = sidebar.child(div().flex_1());
        // Collapse toggle.
        sidebar = sidebar.child(
            div()
                .id("toggle-sidebar")
                .w_full()
                .h(px(48.0))
                .rounded(px(10.0))
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(10.0))
                .child(icon(if collapsed { "▸" } else { "◂" }, 18.0, theme.muted))
                .when(!collapsed, |this| {
                    this.child(
                        div()
                            .flex_1()
                            .child(if collapsed {
                                "Expand sidebar"
                            } else {
                                "Collapse sidebar"
                            })
                            .text_size(px(13.0))
                            .text_color(theme.muted),
                    )
                })
                .opacity(if narrow { 0.45 } else { 1.0 })
                .tooltip(move |_window, cx| {
                    let text = if narrow {
                        "Widen the window to expand the sidebar".to_string()
                    } else if collapsed {
                        "Expand sidebar".to_string()
                    } else {
                        "Collapse sidebar".to_string()
                    };
                    crate::tooltip_view(cx, text.into())
                })
                .on_click(cx.listener(|app, _event, _window, cx| {
                    if app.window_size.0 >= 1080.0 {
                        app.set_setting(SIDEBAR_COLLAPSED, json!(!app.sidebar_collapsed), cx);
                    }
                })),
        );
        sidebar
    }

    /// Docked Prepare panel to the right of the library.
    pub fn render_prepare_dock(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let dock_width = self.prepare_dock_width();
        let mut panel = div()
            .id("prepare-dock")
            .w(px(dock_width))
            .flex_none()
            .h_full()
            .bg(theme.surface)
            .border_l_1()
            .border_color(theme.border)
            .flex()
            .flex_col();

        // Status strip while checking.
        if self.checking {
            panel = panel.child(
                div()
                    .w_full()
                    .h(px(34.0))
                    .bg(theme.accent_soft)
                    .px(px(12.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon("i", 15.0, theme.accent_text))
                    .child(
                        div()
                            .child("Checking selected video")
                            .text_size(px(12.0))
                            .text_color(theme.text_soft)
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .w(px(58.0))
                            .child(progress_bar(0.0, true)),
                    ),
            );
        }

        // Stage.
        panel = panel.child(self.render_prepare_stage(cx, &theme, dock_width));
        // Inspector tabs.
        panel = panel.child(self.render_prepare_tabs(cx));
        // Inspector content.
        panel = panel.child(self.render_prepare_inspector(cx, &theme, dock_width));
        panel
    }

    pub fn render_prepare_studio(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let width = self.window_size.0;
        // The stage column starts after the sidebar + divider; on narrow
        // windows the inspector gives up width before the stage overflows
        // (mirrors the original's resolvedStudioInspectorWidth squeeze).
        let collapsed = self.sidebar_collapsed || width < 1080.0;
        let sidebar = if collapsed { 68.0 } else { 204.0 };
        let available = (width - sidebar - 1.0 - 9.0 - 280.0).max(0.0);
        let inspector_width = self.prepare.studio_width.clamp(300.0, 620.0) as f32;
        let inspector_width = inspector_width.min(available);
        let stage_width = (width - sidebar - 1.0 - inspector_width - 9.0).max(280.0);

        let mut studio = div()
            .id("prepare-studio")
            .flex_1()
            .min_w(px(0.0))
            .bg(theme.ink)
            .flex()
            .flex_row();

        // Header band on top of the stage column.
        let mut stage_col = div()
            .id("studio-stage")
            .w(px(stage_width))
            .flex()
            .flex_col();
        stage_col = stage_col.child(
            div()
                .w_full()
                .h(px(38.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(icon("✎", 14.0, theme.accent_text))
                .child(
                    div()
                        .child("Prepare")
                        .text_size(px(12.0))
                        .text_color(theme.text)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    div()
                        .child("/")
                        .text_size(px(12.0))
                        .text_color(theme.muted),
                )
                .child(
                    div()
                        .flex_1()
                        .child(
                            self.selected
                                .as_ref()
                                .map(|m| m.name.clone())
                                .unwrap_or_default(),
                        )
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::MEDIUM)
                        .text_ellipsis(),
                )
                .child(workbench_button(
                    "studio-open-player",
                    "",
                    "↗",
                    ButtonKind::Ghost,
                    true,
                    true,
                    "Open in default player",
                    cx,
                    |app, cx| app.open_selected_in_player(cx),
                ))
                .child(workbench_button(
                    "studio-exit",
                    "",
                    "⛶",
                    ButtonKind::Ghost,
                    true,
                    true,
                    "Exit full-screen editor  ·  Escape",
                    cx,
                    |app, cx| {
                        app.prepare.studio_mode = false;
                        cx.notify();
                    },
                ))
                .child(workbench_button(
                    "studio-close",
                    "",
                    "✕",
                    ButtonKind::Ghost,
                    true,
                    true,
                    "Close selected video",
                    cx,
                    |app, cx| {
                        app.prepare.studio_mode = false;
                        app.command(Command::ClearSelection);
                        cx.notify();
                    },
                )),
        );
        if self.checking {
            stage_col = stage_col.child(
                div()
                    .w_full()
                    .h(px(34.0))
                    .bg(theme.accent_soft)
                    .px(px(12.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon("i", 15.0, theme.accent_text))
                    .child(
                        div()
                            .child("Checking selected video")
                            .text_size(px(12.0))
                            .text_color(theme.text_soft),
                    ),
            );
        }
        stage_col = stage_col.child(self.render_prepare_stage(cx, &theme, stage_width));
        stage_col = stage_col.child(self.render_prepare_tabs(cx));
        stage_col = stage_col.child(self.render_prepare_inspector(cx, &theme, stage_width));
        studio = studio.child(stage_col);

        // Inspector divider (draggable splitter) + inspector.
        studio = studio
            .child(
                div()
                    .id("studio-splitter")
                    .w(px(9.0))
                    .h_full()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.border)
                    .bg(theme.ink)
                    .cursor_ew_resize()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(
                            move |app, event: &MouseDownEvent, _window, cx| {
                                app.prepare.drag = crate::prepare::DragHandle::StudioSplit;
                                app.prepare.drag_start_x = f64::from(event.position.x);
                                app.prepare.drag_start_value = app.prepare.studio_width;
                                cx.notify();
                            },
                        ),
                    )
                    .on_mouse_move(cx.listener(
                        move |app, event: &MouseMoveEvent, _window, cx| {
                            let x: f32 = event.position.x.into();
                            let y: f32 = event.position.y.into();
                            app.prepare_drag_move(x, y, 0.0, 1.0, 1.0, cx);
                        },
                    ))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|app, _event: &MouseUpEvent, _window, cx| {
                            app.prepare.drag = crate::prepare::DragHandle::None;
                            app.save_draft();
                            cx.notify();
                        }),
                    )
                    .on_click(cx.listener(|app, event: &ClickEvent, _window, cx| {
                        if event.click_count() >= 2 {
                            app.prepare.studio_width = 460.0;
                            app.save_draft();
                            cx.notify();
                        }
                    })),
            )
            .child(
                div()
                    .w(px(inspector_width))
                    .flex_none()
                    .h_full()
                    .bg(theme.surface)
                    .child(self.render_prepare_inspector(cx, &theme, inspector_width)),
            );
        studio
    }

    /// Random-source tree rows currently visible (filter + expansion).
    pub fn random_visible_options(&self) -> Vec<crate::state::RandomFolderOption> {
        let options = self.random_options.clone();
        let filter_text = self.random_filter.to_lowercase();
        let selected_only = self.random_selected_only;
        let expanded_set = self.random_expanded.clone();
        options
            .into_iter()
            .filter(|option| {
                if selected_only && option.selection_state == 0 {
                    return false;
                }
                if !filter_text.is_empty() {
                    return option.name.to_lowercase().contains(&filter_text)
                        || option.folder.to_lowercase().contains(&filter_text);
                }
                let mut ancestor = option.parent.clone();
                while !ancestor.is_empty() {
                    if !expanded_set.contains(&ancestor) {
                        return false;
                    }
                    ancestor = ancestor
                        .rsplit_once('/')
                        .map(|(p, _)| p.to_string())
                        .unwrap_or_default();
                }
                true
            })
            .collect()
    }

    pub fn render_random_popup(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let options = self.random_options.clone();
        let all_selected = self.random_all_selected;
        let selected_count = self.random_selected;
        let mut popup = div()
            .id("random-popup")
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.random_popup_open {
                    app.random_popup_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .absolute()
            .top(px(96.0))
            .right(px(12.0))
            .w(px(456.0))
            .max_h(px(548.0))
            .rounded(px(10.0))
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border_strong)
            
            .flex()
            .flex_col()
            .overflow_hidden();

        // Header.
        popup = popup.child(
            div()
                .w_full()
                .h(px(38.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(icon("▤", 15.0, theme.accent_text))
                .child(
                    div()
                        .child("RANDOM SOURCES")
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    div()
                        .flex_1()
                        .child(if self.random_loading {
                            "INDEXING".to_string()
                        } else {
                            format!("{} NODES", options.len())
                        })
                        .text_size(px(10.0))
                        .text_color(if self.random_loading {
                            theme.warning
                        } else {
                            theme.muted_soft
                        }),
                )
                .child(workbench_button(
                    "random-collapse",
                    "",
                    "▴",
                    ButtonKind::Ghost,
                    true,
                    true,
                    "Collapse source tree",
                    cx,
                    |app, cx| {
                        app.random_expanded.clear();
                        cx.notify();
                    },
                ))
                .child(workbench_button(
                    "random-expand",
                    "",
                    "▾",
                    ButtonKind::Ghost,
                    true,
                    true,
                    "Expand source tree",
                    cx,
                    |app, cx| {
                        for option in &app.random_options {
                            if option.has_children {
                                app.random_expanded.insert(option.folder.clone());
                            }
                        }
                        cx.notify();
                    },
                ))
                .child(workbench_button(
                    "random-close",
                    "",
                    "✕",
                    ButtonKind::Ghost,
                    true,
                    true,
                    "Close random sources",
                    cx,
                    |app, cx| {
                        app.random_popup_open = false;
                        cx.notify();
                    },
                )),
        );

        // Entire library row.
        popup = popup.child(
            div()
                .id("random-all")
                .w_full()
                .h(px(34.0))
                .px(px(10.0))
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(15.0))
                        .h(px(15.0))
                        .rounded(px(3.0))
                        .border_1()
                        .border_color(if all_selected { theme.accent } else { theme.border_strong })
                        .bg(if all_selected { theme.accent } else { theme.raised })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if all_selected {
                            icon("✓", 11.0, theme.accent_content)
                        } else {
                            div()
                        }),
                )
                .child(icon("▤", 14.0, theme.muted))
                .child(
                    div()
                        .flex_1()
                        .child("Entire library")
                        .text_size(px(12.0))
                        .text_color(theme.text)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    div()
                        .child(if all_selected { "SELECTED" } else { "SELECT ALL" })
                        .text_size(px(10.0))
                        .text_color(if all_selected { theme.accent_text } else { theme.muted_soft })
                        .font_weight(FontWeight::BOLD),
                )
                .on_click(cx.listener(|app, _event, _window, cx| {
                    if app.random_all_selected {
                        app.command(Command::ClearRandomFolders);
                    } else {
                        app.command(Command::SelectAllRandomFolders);
                    }
                    cx.notify();
                })),
        );

        // Filter row.
        let filter_text = self.random_filter.clone();
        let selected_only = self.random_selected_only;
        let matches_filter = |option: &RandomFolderOption| -> bool {
            if selected_only && option.selection_state == 0 {
                return false;
            }
            if filter_text.is_empty() {
                return true;
            }
            option
                .name
                .to_lowercase()
                .contains(&filter_text.to_lowercase())
                || option
                    .folder
                    .to_lowercase()
                    .contains(&filter_text.to_lowercase())
        };
        let visible_count = options.iter().filter(|o| matches_filter(o)).count();
        popup = popup.child(
            div()
                .w_full()
                .h(px(36.0))
                .px(px(8.0))
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("random-filter")
                        .flex_1()
                        .h(px(32.0))
                        .px(px(10.0))
                        .rounded(px(10.0))
                        .bg(theme.raised)
                        .border_1()
                        .border_color(if self.focused_field.as_deref() == Some("random-filter") {
                            theme.accent
                        } else {
                            theme.border
                        })
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(icon("⌕", 14.0, theme.muted))
                        .child(
                            div()
                                .child(if filter_text.is_empty() {
                                    "Filter source tree".to_string()
                                } else {
                                    filter_text.clone()
                                })
                                .text_size(px(12.0))
                                .text_color(if filter_text.is_empty() { theme.muted } else { theme.text }),
                        )
                        .cursor_text()
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            app.focus_field("random-filter", cx);
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .id("random-selected-only")
                        .h(px(28.0))
                        .px(px(10.0))
                        .rounded(px(10.0))
                        .cursor_pointer()
                        .bg(if selected_only { theme.active } else { theme.transparent() })
                        .border_1()
                        .border_color(if selected_only { theme.accent } else { theme.transparent() })
                        .flex()
                        .items_center()
                        .child("Selected only")
                        .text_size(px(11.0))
                        .text_color(if selected_only { theme.text } else { theme.muted_soft })
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            app.random_selected_only = !app.random_selected_only;
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .id("random-clear-filter")
                        .h(px(28.0))
                        .px(px(10.0))
                        .rounded(px(10.0))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .opacity(if self.random_filter.is_empty() { 0.0 } else { 1.0 })
                        .child("✕")
                        .text_size(px(11.0))
                        .text_color(theme.muted)
                        .tooltip(move |_window, cx| crate::tooltip_view(cx, "Clear folder search".into()))
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            app.random_filter.clear();
                            if let Some(field) = app.fields.get_mut("random-filter") {
                                field.text.clear();
                                field.caret = 0;
                            }
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .id("random-clear")
                        .h(px(28.0))
                        .px(px(10.0))
                        .rounded(px(10.0))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .opacity(if self.random_has_selection { 1.0 } else { 0.46 })
                        .child("Clear")
                        .text_size(px(11.0))
                        .text_color(theme.muted)
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            app.command(Command::ClearRandomFolders);
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .child(format!("{visible_count} VISIBLE"))
                        .text_size(px(10.0))
                        .text_color(theme.muted_soft)
                        .font_weight(FontWeight::BOLD),
                ),
        );

        // Tree.
        let mut tree = div()
            .id("random-tree")
            .flex_1()
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .flex()
            .flex_col();
        if options.is_empty() {
            tree = tree.child(
                div()
                    .w_full()
                    .py(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .child(if self.scan.active {
                                "Source folders appear as videos are indexed."
                            } else {
                                "Rescan the library to build the source tree."
                            })
                            .text_size(px(12.0))
                            .text_color(theme.muted),
                    ),
            );
        }
        let expanded_set = self.random_expanded.clone();
        let is_visible = |option: &RandomFolderOption| -> bool {
            if !matches_filter(option) {
                return false;
            }
            if !filter_text.is_empty() || selected_only {
                // Filtering auto-expands matching branches.
                return true;
            }
            let mut ancestor = option.parent.clone();
            while !ancestor.is_empty() {
                if !expanded_set.contains(&ancestor) {
                    return false;
                }
                ancestor = ancestor
                    .rsplit_once('/')
                    .map(|(p, _)| p.to_string())
                    .unwrap_or_default();
            }
            true
        };
        let mut rendered = 0usize;
        for option in &options {
            if !is_visible(option) {
                continue;
            }
            rendered += 1;
            let cursor_hit = rendered - 1 == self.random_tree_cursor;
            let folder = option.folder.clone();
            let name = option.name.clone();
            let count = option.video_count;
            let state = option.selection_state;
            let depth = option.depth;
            let has_children = option.has_children;
            let expanded = expanded_set.contains(&folder);
            let indent = (8.0 + depth as f32 * 12.0).min(8.0 + 8.0 * 12.0);
            let mut row = div()
                .id(SharedString::from(format!("random-{folder}")))
                .h(px(32.0))
                .pl(px(indent))
                .pr(px(8.0))
                .cursor_pointer()
                .bg(if cursor_hit { theme.active } else { theme.transparent() })
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0));
            // Disclosure chevron.
            if has_children {
                let folder_for_toggle = folder.clone();
                row = row.child(
                    div()
                        .id(SharedString::from(format!("random-chevron-{folder}")))
                        .occlude()
                        .w(px(18.0))
                        .h(px(18.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(if expanded { "▾" } else { "▸" }, 11.0, theme.muted_soft))
                        .on_click(cx.listener(move |app, _event, _window, cx| {
                            if !app.random_expanded.remove(&folder_for_toggle) {
                                app.random_expanded.insert(folder_for_toggle.clone());
                            }
                            cx.notify();
                        })),
                );
            } else {
                row = row.child(div().w(px(18.0)).flex_none());
            }
            row = row
                .child(
                    div()
                        .w(px(15.0))
                        .h(px(15.0))
                        .rounded(px(3.0))
                        .border_1()
                        .border_color(if state > 0 { theme.accent } else { theme.border_strong })
                        .bg(if state == 2 {
                            theme.accent
                        } else if state == 1 {
                            theme.accent_soft
                        } else {
                            theme.raised
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if state == 2 {
                            icon("✓", 11.0, theme.accent_content).into_any()
                        } else if state == 1 {
                            div().w(px(7.0)).h(px(1.5)).bg(theme.accent_text).into_any()
                        } else {
                            div().into_any()
                        }),
                )
                .child(icon("▸", 14.0, if state > 0 { theme.accent_text } else { theme.muted }))
                .child(
                    div()
                        .flex_1()
                        .child(name)
                        .text_size(px(12.0))
                        .text_color(if state > 0 { theme.text } else { theme.text_soft })
                        .font_weight(if state > 0 { FontWeight::BOLD } else { FontWeight::MEDIUM })
                        .text_ellipsis(),
                )
                .child(
                    div()
                        .child(format!("{count}"))
                        .text_size(px(10.0))
                        .text_color(if state > 0 { theme.accent_text } else { theme.muted_soft }),
                )
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.command(Command::SetRandomFolderEnabled(folder.clone(), state == 0));
                    cx.notify();
                }));
            tree = tree.child(row);
        }
        if rendered == 0 && options.is_empty() {
            // No options at all: show the loading or empty-tree hint
            // (mirrors the original's states).
            tree = tree.child(
                div()
                    .w_full()
                    .py(px(24.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .child(if self.random_loading {
                                "Building source tree…"
                            } else if self.scan.active {
                                "Source folders appear as videos are indexed."
                            } else {
                                "Rescan the library to build the source tree."
                            })
                            .text_size(px(12.0))
                            .text_color(theme.muted),
                    )
                    .child(if self.random_loading {
                        div().w(px(120.0)).child(crate::widgets::progress_bar(0.0, true)).into_any()
                    } else {
                        div().into_any()
                    }),
            );
        } else if rendered == 0 && !options.is_empty() {
            tree = tree.child(
                div()
                    .w_full()
                    .py(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .child(if self.random_selected_only {
                                "No selected source folders match."
                            } else {
                                "No source folders match."
                            })
                            .text_size(px(12.0))
                            .text_color(theme.muted),
                    ),
            );
        }
        popup = popup.child(tree);

        // Footer.
        popup = popup.child(
            div()
                .w_full()
                .h(px(44.0))
                .px(px(12.0))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(if all_selected {
                            "ENTIRE LIBRARY".to_string()
                        } else if selected_count == 0 {
                            "NO SOURCES SELECTED".to_string()
                        } else {
                            format!("{selected_count} SOURCE FOLDERS")
                        })
                        .text_size(px(10.0))
                        .text_color(if selected_count == 0 && !all_selected {
                            theme.warning
                        } else {
                            theme.text_soft
                        })
                        .font_weight(FontWeight::BOLD)
                        .child(
                            div()
                                .child("Parent checks include every nested folder")
                                .text_size(px(9.0))
                                .text_color(theme.muted_soft),
                        ),
                )
                .child(button(
                    "random-done",
                    "Done",
                    ButtonKind::Secondary,
                    None,
                    true,
                    cx,
                    |app, cx| {
                        app.random_popup_open = false;
                        cx.notify();
                    },
                )),
        );
        popup
    }

    pub fn render_history_menu(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let post_id = self.history_more_menu_post.unwrap_or(0);
        let post = self
            .history
            .rows
            .iter()
            .find(|row| row.id == post_id)
            .cloned();
        let mut menu = div()
            .id("history-menu")
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.history_more_menu_post.take().is_some() {
                    app.history_more_menu_closed_at = std::time::Instant::now();
                    cx.notify();
                }
            }))
            .absolute()
            .right(px(24.0))
            .top(px(120.0))
            .w(px(224.0))
            .rounded(px(10.0))
            .bg(theme.surface_soft)
            .border_1()
            .border_color(theme.border_strong)
            
            .py(px(6.0))
            .flex()
            .flex_col();

        let add_item = |_app: &mut crate::App,
                            id: &'static str,
                            label: &str,
                            glyph: &'static str,
                            enabled: bool,
                            command: Option<Command>,
                            cx: &mut Context<crate::App>| {
            let mut item = div()
                .id(id)
                .h(px(40.0))
                .px(px(12.0))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(icon(glyph, 16.0, theme.muted))
                .child(
                    div()
                        .child(label.to_string())
                        .text_size(px(13.0))
                        .text_color(theme.text),
                );
            item = item.when(!enabled, |this| this.opacity(0.46).cursor_default());
            if let Some(command) = command {
                item = item.when(enabled, |this| {
                    this.on_click(cx.listener(move |app, _event, _window, cx| {
                        app.command(command.clone());
                        app.history_more_menu_post = None;
                        cx.notify();
                    }))
                });
            }
            item
        };

        if let Some(post) = post {
            let can_prepare_x = matches!(post.x_status.as_str(), "prepared" | "failed");
            let has_telegram_url = !post.telegram_message_link.is_empty();
            let has_x_url = !post.x_url.is_empty();
            let can_trash = post.is_generated.unwrap_or(false)
                && post.cleanup_state.as_deref() != Some("trashed");
            if can_prepare_x {
                menu = menu.child(add_item(
                    self,
                    "menu-prepare-x",
                    "Prepare X again",
                    "𝕏",
                    true,
                    Some(Command::PrepareXAgain(post_id)),
                    cx,
                ));
            }
            if has_telegram_url {
                let link = post.telegram_message_link.clone();
                menu = menu.child(add_item(
                    self,
                    "menu-open-tg",
                    "Open Telegram post",
                    "↗",
                    true,
                    None,
                    cx,
                ).on_click(cx.listener(move |app, _event, _window, cx| {
                    let _ = std::process::Command::new("open").arg(&link).spawn();
                    app.history_more_menu_post = None;
                    cx.notify();
                })));
            }
            if has_x_url {
                let link = post.x_url.clone();
                menu = menu.child(add_item(
                    self,
                    "menu-open-x",
                    "Open X post",
                    "↗",
                    true,
                    None,
                    cx,
                ).on_click(cx.listener(move |app, _event, _window, cx| {
                    let _ = std::process::Command::new("open").arg(&link).spawn();
                    app.history_more_menu_post = None;
                    cx.notify();
                })));
            }
            let path = post
                .export_path
                .clone()
                .or_else(|| post.source_path.clone())
                .unwrap_or_default();
            menu = menu.child(add_item(
                self,
                "menu-reveal",
                "Show video in folder",
                "▤",
                !path.is_empty(),
                None,
                cx,
            ).on_click(cx.listener(move |app, _event, _window, cx| {
                if !path.is_empty() {
                    let _ = cliprelay_core::x::XAssistant::reveal(std::path::Path::new(&path));
                }
                app.history_more_menu_post = None;
                cx.notify();
            })));
            if can_trash {
                menu = menu.child(add_item(
                    self,
                    "menu-trash",
                    "Move generated video to Trash",
                    "🗑",
                    true,
                    Some(Command::TrashExport(post_id)),
                    cx,
                ));
            }
        }
        menu
    }

    pub fn open_selected_in_player(&mut self, _cx: &mut Context<Self>) {
        if let Some(selected) = &self.selected {
            let path = PathBuf::from(&selected.path);
            if path.is_file() {
                let _ = std::process::Command::new("open").arg(&path).spawn();
            } else {
                self.toast(ToastKind::Error, "That file is no longer available.");
            }
        }
    }
}

impl crate::App {
    /// Width of the docked Prepare panel.
    pub fn prepare_dock_width(&self) -> f32 {
        let window = self.window_size.0;
        let sidebar = if self.sidebar_collapsed {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_EXPANDED_WIDTH
        };
        let page_width = (window - sidebar).max(1.0);
        if self.prepare_expanded {
            // Port of the original's expanded formula: the dock widens when
            // space allows but never squeezes the grid below 460px.
            let base = (page_width * 0.34).clamp(360.0, 420.0);
            let explorer = if self.explorer_visible() {
                EXPLORER_WIDTH
            } else {
                0.0
            };
            let grid_floor = (page_width - explorer - 460.0).max(1.0);
            base.max(grid_floor).min(680.0)
        } else {
            (page_width * 0.34).max(360.0).min(420.0)
        }
    }
}
