//! Shell chrome: header (command center row), workspace tabs, sidebar nav,
//! Prepare dock/studio, random-source popup, history menu.

use crate::settings_import::*;
use crate::shortcuts::{shortcut_keys, SHORTCUTS};
use crate::state::*;
use crate::theme::*;
use crate::widgets::*;
use gpui::prelude::*;
use gpui::*;
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
        let cut_active = self.prepare.media_id > 0 && self.prepare.cut_active();
        vec![
            CommandAction {
                id: 0,
                label: "Pick random video",
                detail: "Choose from the active random-source folders",
                category: "Library",
                glyph: "shuffle",
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
                glyph: "close",
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
                shortcut: "S",
                enabled: self.can_open_selected_in_studio() && !self.prepare.studio_mode,
            },
            CommandAction {
                id: 26,
                label: "Focus Prepare Edit",
                detail: "Show crop and black-mask tools",
                category: "Prepare",
                glyph: "square",
                keywords: "prepare edit crop mask frame",
                shortcut: "",
                enabled: selected,
            },
            CommandAction {
                id: 27,
                label: "Focus Prepare Publish",
                detail: "Show output, destinations, and captions",
                category: "Prepare",
                glyph: "send",
                keywords: "prepare publish telegram x caption output",
                shortcut: "",
                enabled: selected,
            },
            CommandAction {
                id: 38,
                label: "Set cut In at playhead",
                detail: "Keep the video starting at the current frame",
                category: "Prepare",
                glyph: "[",
                keywords: "prepare trim cut mark in start playhead",
                shortcut: "I",
                enabled: selected && self.prepare.duration > 0.0 && !self.checking,
            },
            CommandAction {
                id: 39,
                label: "Set cut Out at playhead",
                detail: "Keep the video ending at the current frame",
                category: "Prepare",
                glyph: "]",
                keywords: "prepare trim cut mark out end playhead",
                shortcut: "O",
                enabled: selected && self.prepare.duration > 0.0 && !self.checking,
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
                id: 40,
                label: "Use Frosted Glass theme",
                detail: "Neutral material over the system backdrop blur",
                category: "Theme",
                glyph: "◐",
                keywords: "theme frosted glass liquid chrome silver metallic translucent blur",
                shortcut: "",
                enabled: self.theme_mode.as_str() != "frosted_glass",
            },
            CommandAction {
                id: 41,
                label: "Use Graphite Glass theme",
                detail: "Silver graphite material over the system backdrop blur",
                category: "Theme",
                glyph: "◐",
                keywords: "theme graphite glass silver metallic translucent blur macos",
                shortcut: "",
                enabled: self.theme_mode.as_str() != "graphite_glass",
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
    /// Matching mirrors the original: the '>' prefix is stripped (the
    /// needle), and the haystack is label + detail + category + keywords.
    pub fn command_entries(&self) -> Vec<CommandEntry> {
        let query = self.command_needle().to_lowercase();
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
                    let haystack =
                        format!("{} {} {} {}", a.label, a.detail, a.category, a.keywords)
                            .to_lowercase();
                    haystack.contains(&query)
                })
                .collect();
            // Group matched commands by category like the original's
            // sectioned ListView (categories stay in registry order).
            let mut current_category = "";
            for action in matched {
                if action.category != current_category {
                    current_category = action.category;
                    entries.push(CommandEntry::Section(current_category));
                }
                entries.push(CommandEntry::Action(action));
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
                self.open_selected_in_studio(cx);
            }
            26 => {
                self.prepare.inspector_tab = 0;
                self.open_selected_in_studio(cx);
            }
            27 => {
                self.prepare.inspector_tab = 1;
                self.open_selected_in_studio(cx);
            }
            38 => {
                if self.prepare.mark_in_at_playhead() {
                    self.save_draft();
                }
                cx.notify();
            }
            39 => {
                if self.prepare.mark_out_at_playhead() {
                    self.save_draft();
                }
                cx.notify();
            }
            28 => {
                self.prepare.reset_cut();
                self.save_draft();
                cx.notify();
            }
            29 => self.command(Command::RevealSelectedInLibrary),
            30 => self.choose_library_folder(cx),
            31 => {
                self.navigate_to(Page::Library, cx);
            }
            32 => {
                self.navigate_to(Page::History, cx);
            }
            33 => {
                self.navigate_to(Page::Settings, cx);
            }
            34 => self.set_setting(THEME_MODE, serde_json::json!("relay"), cx),
            35 => self.set_setting(THEME_MODE, serde_json::json!("pitch_black"), cx),
            36 => self.set_setting(THEME_MODE, serde_json::json!("full_white"), cx),
            40 => self.set_setting(THEME_MODE, serde_json::json!("frosted_glass"), cx),
            41 => self.set_setting(THEME_MODE, serde_json::json!("graphite_glass"), cx),
            37 => {
                if self.window_size.0 >= 1080.0 {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                }
                cx.notify();
            }
            _ => {}
        }
    }

    pub fn render_command_center(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let entries = self.command_entries();
        let selected = self.command_selected;
        let scope = self.command_scope.clone();
        let popup_width = (self.window_size.0 - 340.0).clamp(360.0, 680.0);
        let results_height = (self.window_size.1 - 140.0).clamp(180.0, 320.0);
        let mut popup = div()
            .id("command-center-popup")
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.command_open {
                    app.close_command_center();
                    cx.notify();
                }
            }))
            .w(px(popup_width))
            .h(px(results_height + 80.0))
            .rounded(px(10.0))
            .bg(theme.overlay_surface())
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |popup| {
                popup.shadow(theme.material_shadow())
            })
            .when(!theme.is_frosted(), |popup| popup.shadow_lg())
            .flex()
            .flex_col()
            .overflow_hidden();

        // The query is already visible in the focused field immediately above
        // this surface. Keep the popup header to one quiet scope control rather
        // than repeating the search text in a second title row.
        let mut chips = div()
            .w_full()
            .h(px(44.0))
            .flex_none()
            .px(px(8.0))
            .border_b_1()
            .border_color(theme.border)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0));
        for (value, label) in [
            ("all", "All"),
            ("videos", "Videos"),
            ("folders", "Folders"),
            ("commands", "Commands"),
        ] {
            let effective_scope = self.effective_command_scope();
            let active = effective_scope == value;
            let value = value.to_string();
            let click_value = value.clone();
            let enter_value = value.clone();
            let chip_rest_face: Background = if active {
                theme.selection_face(TactileState::Rest, false)
            } else {
                theme.transparent().into()
            };
            let chip_hover_face = if active {
                theme.selection_face(TactileState::Hover, false)
            } else {
                theme.control_face(TactileState::Hover)
            };
            let chip_pressed_face = theme.selection_face(TactileState::Pressed, false);
            let chip_rest_edge = if active {
                theme.tactile_edge(TactileState::Rest, false)
            } else {
                theme.transparent()
            };
            let chip_hover_edge = theme.tactile_edge(TactileState::Hover, false);
            let chip_pressed_edge = theme.tactile_edge(TactileState::Pressed, false);
            let chip_focus_edge = theme.accent;
            let chip_rest_shadow = if active {
                theme.tactile_shadow(TactileState::Rest, true)
            } else {
                Vec::new()
            };
            let chip_hover_shadow = theme.tactile_shadow(TactileState::Hover, true);
            let chip_pressed_shadow = theme.tactile_shadow(TactileState::Pressed, true);
            chips = chips.child(
                div()
                    .id(SharedString::from(format!("scope-{value}")))
                    .h(px(34.0))
                    .px(px(12.0))
                    .rounded(px(6.0))
                    .relative()
                    .top(px(0.0))
                    .border_1()
                    .border_color(chip_rest_edge)
                    .cursor_pointer()
                    .bg(chip_rest_face)
                    .shadow(chip_rest_shadow)
                    .hover({
                        let hover_shadow = chip_hover_shadow.clone();
                        move |style| {
                            style
                                .bg(chip_hover_face)
                                .border_color(chip_hover_edge)
                                .shadow(hover_shadow.clone())
                        }
                    })
                    .active({
                        let pressed_shadow = chip_pressed_shadow.clone();
                        move |style| {
                            style
                                .top(px(1.0))
                                .bg(chip_pressed_face)
                                .border_color(chip_pressed_edge)
                                .shadow(pressed_shadow.clone())
                        }
                    })
                    .flex()
                    .items_center()
                    .child(label)
                    .text_size(px(13.0))
                    .text_color(if active { theme.text } else { theme.text_soft })
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .tab_index(0)
                    .focus(move |style| {
                        style
                            .bg(chip_hover_face)
                            .border_2()
                            .border_color(chip_focus_edge)
                    })
                    .on_click(cx.listener(move |app, _event, window, cx| {
                        window.blur();
                        app.command_scope = click_value.clone();
                        app.schedule_command_search(cx);
                        cx.notify();
                    }))
                    .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                        app.command_scope = enter_value.clone();
                        app.schedule_command_search(cx);
                        cx.notify();
                        cx.stop_propagation();
                    }))
                    .on_action(
                        cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                            app.command_scope = value.clone();
                            app.schedule_command_search(cx);
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    ),
            );
        }
        popup = popup.child(chips);

        // Results list.
        let mut list = div()
            .id("command-results")
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .px(px(8.0))
            .py(px(8.0));

        let selectable_count = entries
            .iter()
            .filter(|e| {
                matches!(e, CommandEntry::Action(a) if a.enabled)
                    || matches!(e, CommandEntry::Result(_))
            })
            .count();
        let selected = selected.min(entries.len().saturating_sub(1));

        for (index, entry) in entries.iter().enumerate() {
            match entry {
                CommandEntry::Section(title) => {
                    list = list.child(
                        div()
                            .w_full()
                            .h(px(24.0))
                            .flex_none()
                            .px(px(8.0))
                            .flex()
                            .items_center()
                            .child(title.to_string())
                            .text_size(px(11.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::SEMIBOLD),
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
                    let rest_face = if is_selected {
                        theme.selection_face(TactileState::Rest, false)
                    } else {
                        theme.transparent().into()
                    };
                    let rest_edge = if is_selected {
                        theme.tactile_edge(TactileState::Rest, false)
                    } else {
                        theme.transparent()
                    };
                    let rest_shadow = if is_selected {
                        theme.tactile_shadow(TactileState::Rest, true)
                    } else {
                        Vec::new()
                    };
                    let hover_face = if is_selected {
                        theme.selection_face(TactileState::Hover, false)
                    } else {
                        theme.control_face(TactileState::Hover)
                    };
                    let hover_edge = theme.tactile_edge(TactileState::Hover, false);
                    let hover_shadow = theme.tactile_shadow(TactileState::Hover, true);
                    let pressed_face = theme.selection_face(TactileState::Pressed, false);
                    let pressed_edge = theme.tactile_edge(TactileState::Pressed, false);
                    let pressed_shadow = theme.tactile_shadow(TactileState::Pressed, true);
                    let mut row = div()
                        .id(SharedString::from(format!("action-{action_id}")))
                        .w_full()
                        .h(px(54.0))
                        .flex_none()
                        .px(px(11.0))
                        .rounded(px(6.0))
                        .relative()
                        .top(px(0.0))
                        .border_1()
                        .border_color(rest_edge)
                        .bg(rest_face)
                        .shadow(rest_shadow)
                        .cursor(if enabled {
                            CursorStyle::PointingHand
                        } else {
                            CursorStyle::Arrow
                        })
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(10.0))
                        .hover({
                            let hover_shadow = hover_shadow.clone();
                            move |style| {
                                if !enabled {
                                    style
                                } else {
                                    style
                                        .bg(hover_face)
                                        .border_color(hover_edge)
                                        .shadow(hover_shadow.clone())
                                }
                            }
                        })
                        .when(enabled, |this| {
                            let pressed_shadow = pressed_shadow.clone();
                            this.active(move |style| {
                                style
                                    .top(px(1.0))
                                    .bg(pressed_face)
                                    .border_color(pressed_edge)
                                    .shadow(pressed_shadow.clone())
                            })
                        })
                        .opacity(if enabled { 1.0 } else { 0.46 })
                        .child(
                            div()
                                .w(px(22.0))
                                .h(px(22.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(
                                    glyph,
                                    16.0,
                                    if is_selected {
                                        theme.accent_text
                                    } else {
                                        theme.muted
                                    },
                                )),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .flex_col()
                                .gap(px(1.0))
                                .child(
                                    div()
                                        .child(title.to_string())
                                        .text_size(px(13.0))
                                        .text_color(theme.text)
                                        .font_weight(if is_selected {
                                            FontWeight::SEMIBOLD
                                        } else {
                                            FontWeight::MEDIUM
                                        })
                                        .text_ellipsis(),
                                )
                                .child(
                                    div()
                                        .child(detail.to_string())
                                        .text_size(px(11.5))
                                        .text_color(theme.text_soft)
                                        .text_ellipsis(),
                                ),
                        )
                        .child(if shortcut.is_empty() {
                            div().into_any()
                        } else {
                            div()
                                .child(shortcut.to_string())
                                .flex_none()
                                .h(px(22.0))
                                .px(px(7.0))
                                .rounded(px(4.0))
                                .bg(theme.raised)
                                .border_1()
                                .border_color(theme.border)
                                .flex()
                                .items_center()
                                .text_size(px(11.0))
                                .text_color(theme.muted)
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
                    let rest_face = if is_selected {
                        theme.selection_face(TactileState::Rest, false)
                    } else {
                        theme.transparent().into()
                    };
                    let rest_edge = if is_selected {
                        theme.tactile_edge(TactileState::Rest, false)
                    } else {
                        theme.transparent()
                    };
                    let rest_shadow = if is_selected {
                        theme.tactile_shadow(TactileState::Rest, true)
                    } else {
                        Vec::new()
                    };
                    let hover_face = if is_selected {
                        theme.selection_face(TactileState::Hover, false)
                    } else {
                        theme.control_face(TactileState::Hover)
                    };
                    let hover_edge = theme.tactile_edge(TactileState::Hover, false);
                    let hover_shadow = theme.tactile_shadow(TactileState::Hover, true);
                    let pressed_face = theme.selection_face(TactileState::Pressed, false);
                    let pressed_edge = theme.tactile_edge(TactileState::Pressed, false);
                    let pressed_shadow = theme.tactile_shadow(TactileState::Pressed, true);
                    let mut row = div()
                        .id(SharedString::from(format!("command-{index}")))
                        .w_full()
                        .h(px(54.0))
                        .flex_none()
                        .px(px(11.0))
                        .rounded(px(6.0))
                        .relative()
                        .top(px(0.0))
                        .border_1()
                        .border_color(rest_edge)
                        .bg(rest_face)
                        .shadow(rest_shadow)
                        .cursor_pointer()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(10.0))
                        .hover({
                            let hover_shadow = hover_shadow.clone();
                            move |style| {
                                style
                                    .bg(hover_face)
                                    .border_color(hover_edge)
                                    .shadow(hover_shadow.clone())
                            }
                        })
                        .active({
                            let pressed_shadow = pressed_shadow.clone();
                            move |style| {
                                style
                                    .top(px(1.0))
                                    .bg(pressed_face)
                                    .border_color(pressed_edge)
                                    .shadow(pressed_shadow.clone())
                            }
                        })
                        .child(
                            div()
                                .w(px(22.0))
                                .h(px(22.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(
                                    if kind == "folder" { "folder" } else { "play" },
                                    16.0,
                                    if is_selected {
                                        theme.accent_text
                                    } else {
                                        theme.muted
                                    },
                                )),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .child(title)
                                        .text_size(px(13.0))
                                        .text_color(theme.text)
                                        .font_weight(if is_selected {
                                            FontWeight::SEMIBOLD
                                        } else {
                                            FontWeight::MEDIUM
                                        })
                                        .text_ellipsis(),
                                )
                                .child(
                                    div()
                                        .child(detail)
                                        .text_size(px(11.5))
                                        .text_color(theme.text_soft)
                                        .text_ellipsis(),
                                ),
                        )
                        .child(if kind == "folder" {
                            tabular(
                                div()
                                    .flex_none()
                                    .child(format!("{count}"))
                                    .text_size(px(11.0))
                                    .text_color(theme.muted),
                            )
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

        if selectable_count == 0 && self.command_searching {
            list = list.child(
                div()
                    .w_full()
                    .h(px(82.0))
                    .flex_none()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .gap(px(8.0))
                    .child(icon("search", 15.0, theme.muted))
                    .child(
                        div()
                            .child("Searching library…")
                            .text_size(px(12.0))
                            .text_color(theme.text_soft)
                            .font_weight(FontWeight::MEDIUM),
                    ),
            );
        } else if selectable_count == 0 {
            let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
            let has_query = !self.command_query.trim().is_empty();
            let effective_scope = self.effective_command_scope();
            let title = if !has_root && effective_scope != "commands" {
                "Choose a library folder to search".to_string()
            } else if effective_scope == "commands" {
                "No commands match".to_string()
            } else if effective_scope == "folders" {
                if has_query {
                    "No folders match".to_string()
                } else {
                    "No folders indexed yet".to_string()
                }
            } else if scope == "videos" {
                if has_query {
                    "No videos match".to_string()
                } else {
                    "No videos indexed yet".to_string()
                }
            } else {
                "No results match".to_string()
            };
            let detail = if !has_root && scope != "commands" {
                "Use “Choose library root” in Commands."
            } else if has_query {
                "Try a shorter filename, folder, or command."
            } else {
                "Scan the library or choose another filter."
            };
            list = list.child(
                div()
                    .w_full()
                    .py(px(26.0))
                    .px(px(20.0))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .child(title)
                            .text_size(px(13.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_center(),
                    )
                    .child(
                        div()
                            .child(detail)
                            .text_size(px(11.5))
                            .text_color(theme.text_soft)
                            .text_center(),
                    ),
            );
        }
        popup = popup.child(list);

        // Hint footer.
        let result_count = entries
            .iter()
            .filter(|e| {
                matches!(e, CommandEntry::Action(_)) || matches!(e, CommandEntry::Result(_))
            })
            .count();
        popup = popup.child(
            div()
                .w_full()
                .h(px(36.0))
                .flex_none()
                .px(px(16.0))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(if self.command_searching {
                    div()
                        .child("Searching…".to_string())
                        .text_size(px(11.0))
                        .text_color(theme.warning)
                } else if !self.command_query.trim().is_empty() {
                    div()
                        .child(format!(
                            "{} result{}",
                            result_count,
                            if result_count == 1 { "" } else { "s" }
                        ))
                        .text_size(px(11.0))
                        .text_color(theme.muted)
                } else {
                    div()
                        .child("Suggested commands")
                        .text_size(px(11.0))
                        .text_color(theme.muted)
                })
                .child(div().flex_1())
                .child(
                    div()
                        .child("↑↓ Move  ·  Enter Open  ·  Esc Close")
                        .text_size(px(11.0))
                        .text_color(theme.muted_soft),
                ),
        );
        popup_fade(popup, "command-center-fade")
    }

    pub fn render_context_toolbar(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let page = self.page;
        let width = self.window_size.0;
        let nav_collapsed = width < 1080.0 || self.sidebar_collapsed;
        let activity_width = if nav_collapsed {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_EXPANDED_WIDTH
        };
        let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
        let studio_mode = self.prepare.studio_mode;
        let show_explorer =
            page == Page::Library && has_root && self.explorer_visible() && !studio_mode;
        let explorer_width = EXPLORER_WIDTH;
        let prepare_width =
            if page == Page::Library && self.selected.is_some() && !self.prepare.studio_mode {
                self.prepare_dock_width()
            } else {
                0.0
            };
        let show_prepare = prepare_width > 1.0;
        // Slot width for responsive thresholds (matches QML libraryContextSlot.width <680 / <500)
        let slot_reserved = activity_width
            + if show_explorer { explorer_width } else { 1.0 }
            + if show_prepare {
                prepare_width + 1.0
            } else {
                0.0
            };
        let slot_width = (width - slot_reserved).max(0.0);
        let compact_actions = slot_width < 680.0;
        let narrow_actions = slot_width < 500.0;

        let mut toolbar = div()
            .id("context-toolbar")
            .w_full()
            .min_w(px(0.0))
            .h(px(CONTEXT_TOOLBAR_HEIGHT))
            .bg(theme.canvas_background())
            .flex()
            .flex_row()
            .items_center()
            .overflow_hidden();

        // The first context cell is also the first activity-rail slot. This
        // keeps the navigation rhythm continuous instead of rendering Library
        // once above the rail and again immediately below it.
        let library_selected = page == Page::Library && !studio_mode;
        let can_expand_navigation = nav_collapsed && width >= 1080.0;
        let activity_item_width = if nav_collapsed {
            40.0
        } else {
            activity_width - 16.0
        };
        let library_rest_face: Background = if library_selected {
            theme.selection_face(TactileState::Rest, true)
        } else {
            theme.transparent().into()
        };
        let library_hover_face = if library_selected {
            theme.selection_face(TactileState::Hover, true)
        } else {
            theme.control_face(TactileState::Hover)
        };
        let library_pressed_face = theme.selection_face(TactileState::Pressed, true);
        let library_rest_edge = if library_selected {
            theme.tactile_edge(TactileState::Rest, true)
        } else {
            theme.transparent()
        };
        let library_hover_edge = theme.tactile_edge(TactileState::Hover, library_selected);
        let library_pressed_edge = theme.tactile_edge(TactileState::Pressed, true);
        let library_focus_edge = theme.accent;
        let library_rest_shadow = if library_selected {
            theme.tactile_shadow(TactileState::Rest, true)
        } else {
            Vec::new()
        };
        let library_hover_shadow = theme.tactile_shadow(TactileState::Hover, true);
        let library_pressed_shadow = theme.tactile_shadow(TactileState::Pressed, true);
        let activity = div()
            .flex_none()
            .w(px(activity_width))
            .h_full()
            .bg(theme.rail_background())
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("nav-library-header")
                    .w(px(activity_item_width))
                    .h(px(40.0))
                    .px(if nav_collapsed { px(0.0) } else { px(10.0) })
                    .rounded(px(3.0))
                    .relative()
                    .top(px(0.0))
                    .border_1()
                    .border_color(library_rest_edge)
                    .bg(library_rest_face)
                    .shadow(library_rest_shadow)
                    .cursor_pointer()
                    .tab_index(0)
                    .focus({
                        let hover_shadow = library_hover_shadow.clone();
                        move |style| {
                            style
                                .bg(library_hover_face)
                                .shadow(hover_shadow.clone())
                                .border_2()
                                .border_color(library_focus_edge)
                        }
                    })
                    .active({
                        let pressed_shadow = library_pressed_shadow.clone();
                        move |style| {
                            style
                                .top(px(1.0))
                                .bg(library_pressed_face)
                                .border_color(library_pressed_edge)
                                .shadow(pressed_shadow.clone())
                        }
                    })
                    .hover(move |style| {
                        style
                            .bg(library_hover_face)
                            .border_color(library_hover_edge)
                            .shadow(library_hover_shadow.clone())
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(9.0))
                    .child(icon(
                        "library",
                        18.0,
                        if library_selected {
                            theme.accent_text
                        } else {
                            theme.muted
                        },
                    ))
                    .when(!nav_collapsed, |this| {
                        this.child(
                            div()
                                .flex_1()
                                .child("Library")
                                .text_size(px(13.0))
                                .text_color(if library_selected {
                                    theme.text
                                } else {
                                    theme.text_soft
                                })
                                .font_weight(if library_selected {
                                    FontWeight::SEMIBOLD
                                } else {
                                    FontWeight::MEDIUM
                                }),
                        )
                    })
                    .tooltip(move |_window, cx| {
                        crate::tooltip_view(
                            cx,
                            if can_expand_navigation {
                                "Open Library  ·  Double-click to expand navigation"
                            } else {
                                "Open Library"
                            }
                            .into(),
                        )
                    })
                    .on_click(cx.listener(move |app, event: &ClickEvent, window, cx| {
                        window.blur();
                        if can_expand_navigation && event.click_count() >= 2 {
                            app.set_setting(SIDEBAR_COLLAPSED, json!(false), cx);
                        } else {
                            app.navigate_to(Page::Library, cx);
                        }
                    }))
                    .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                        app.navigate_to(Page::Library, cx);
                        cx.stop_propagation();
                    }))
                    .on_action(
                        cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                            app.navigate_to(Page::Library, cx);
                            cx.stop_propagation();
                        }),
                    ),
            );
        toolbar = toolbar.child(activity);

        // Explorer header
        if show_explorer {
            let explorer = div()
                .flex_none()
                .w(px(explorer_width))
                .h_full()
                .rounded_tl(px(3.0))
                .bg(theme.section_background())
                .border_t_1()
                .border_l_1()
                .border_r_1()
                .border_color(theme.workbench_border.opacity(0.62))
                .flex()
                .flex_row()
                .items_center()
                .pl(px(28.0))
                .pr(px(18.0))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .relative()
                        .top(px(1.0))
                        .child(tracked("EXPLORER"))
                        .text_size(px(12.5))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::SEMIBOLD),
                );
            toolbar = toolbar.child(explorer);
        } else {
            toolbar = toolbar.child(
                div()
                    .w(px(1.0))
                    .h_full()
                    .bg(theme.workbench_border.opacity(0.62))
                    .flex_none(),
            );
        }

        // Center slot (library context or page detail)
        let mut center = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(7.0))
            .pl(px(12.0))
            .pr(px(9.0))
            .bg(theme.canvas_background())
            .border_t_1()
            .border_color(theme.workbench_border.opacity(0.62))
            .overflow_hidden();

        match page {
            Page::Library => {
                let location = if studio_mode {
                    self.selected
                        .as_ref()
                        .map(|media| media.name.clone())
                        .unwrap_or_else(|| "No clip selected".to_string())
                } else if self.active_folder.is_empty() {
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
                center = center.child(div().w(px(7.0)).flex_none());
                if studio_mode {
                    center = center.child(icon(
                        "edit",
                        15.0,
                        if has_root {
                            theme.accent_text
                        } else {
                            theme.muted
                        },
                    ));
                }
                if !narrow_actions {
                    center = center
                        .child(
                            div()
                                .child(if studio_mode {
                                    "Prepare"
                                } else {
                                    "Video library"
                                })
                                .text_size(px(13.0))
                                .text_color(theme.accent_text)
                                .font_weight(FontWeight::SEMIBOLD),
                        )
                        .child(
                            div()
                                .child("/")
                                .text_size(px(13.0))
                                .text_color(theme.muted_soft),
                        );
                }
                center = center.child(
                    div()
                        .flex_1()
                        .min_w(px(42.0))
                        .child(location)
                        .text_size(px(13.0))
                        .text_color(if has_root {
                            theme.text_soft
                        } else {
                            theme.muted
                        })
                        .text_ellipsis(),
                );

                if has_root && !studio_mode {
                    center = center.child(workbench_button(
                        "toggle-folders",
                        if self.show_folders {
                            "Hide folders"
                        } else {
                            "Show folders"
                        },
                        "panel",
                        ButtonKind::Ghost,
                        true,
                        compact_actions,
                        "Toggle the hierarchical folder column",
                        cx,
                        |app, cx| {
                            app.show_folders = !app.show_folders;
                            cx.notify();
                        },
                    ));
                    center = center.child(workbench_button(
                        "choose-root",
                        "Choose root",
                        "folder",
                        ButtonKind::Ghost,
                        true,
                        compact_actions,
                        "Choose library root",
                        cx,
                        |app, cx| app.choose_library_folder(cx),
                    ));
                }

                if !narrow_actions && !studio_mode {
                    center =
                        center.child(div().w(px(1.0)).h(px(22.0)).bg(theme.border).flex_none());
                }

                if has_root && !studio_mode {
                    let sort_label = match self
                        .settings
                        .get(SORT_MODE)
                        .and_then(|value| value.as_str())
                        .unwrap_or("newest")
                    {
                        "newest" => "Newest",
                        "oldest" => "Oldest",
                        "name" => "Name",
                        "duration" => "Duration",
                        "size" => "Size",
                        _ => "Newest",
                    };
                    let sort_width = if narrow_actions { 96.0 } else { 112.0 };
                    let sort_popup = self
                        .sort_menu_open
                        .then(|| self.render_sort_menu(cx).into_any());
                    let sort_trigger = div()
                        .id("sort-trigger")
                        .track_focus(&self.sort_source_focus)
                        .size_full()
                        .px(px(9.0))
                        .rounded(px(RADIUS_SM))
                        .bg(if self.sort_menu_open {
                            theme.active
                        } else {
                            theme.raised
                        })
                        .border_1()
                        .border_color(if self.sort_menu_open {
                            theme.accent
                        } else {
                            theme.border
                        })
                        .hover(|style| style.bg(theme.hover))
                        .focus(|style| style.border_2().border_color(theme.accent))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .cursor_pointer()
                        .tab_index(0)
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .child(sort_label)
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::MEDIUM)
                                .text_ellipsis(),
                        )
                        .child(icon(
                            if self.sort_menu_open {
                                "chevron-up"
                            } else {
                                "chevron-down"
                            },
                            13.0,
                            if self.sort_menu_open {
                                theme.accent_text
                            } else {
                                theme.muted
                            },
                        ))
                        .tooltip(move |_window, cx| {
                            crate::tooltip_view(cx, "Sort library and Explorer".into())
                        })
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            app.toggle_sort_popup();
                            cx.notify();
                        }));
                    center = center.child(
                        anchored_overlay(
                            sort_trigger,
                            sort_popup,
                            OverlayPlacement::BelowEnd,
                            size(px(sort_width), px(30.0)),
                        )
                        .flex_none()
                        .w(px(sort_width))
                        .h(px(30.0)),
                    );
                }

                if !narrow_actions && has_root && !studio_mode {
                    center =
                        center.child(div().w(px(1.0)).h(px(22.0)).bg(theme.border).flex_none());
                }

                if has_root && !studio_mode {
                    let scanning = self.scan.active;
                    let cancelling = self.scan.cancelling;
                    let scanning_label = if cancelling {
                        "Stopping…"
                    } else if scanning {
                        "Stop scan"
                    } else {
                        "Rescan"
                    };
                    center = center.child(workbench_button(
                        "rescan",
                        scanning_label,
                        if cancelling || scanning {
                            "square"
                        } else {
                            "refresh"
                        },
                        ButtonKind::Ghost,
                        !(scanning && cancelling),
                        compact_actions,
                        "Rescan library",
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
                }
                toolbar = toolbar.child(center);
            }
            Page::History => {
                center = center
                    .child(icon("history", 15.0, theme.accent_text))
                    .child(
                        div()
                            .child("History")
                            .text_size(px(12.0))
                            .text_color(theme.accent_text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .child("/")
                            .text_size(px(12.0))
                            .text_color(theme.muted_soft),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child("Recorded relays and delivery outcomes")
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    );
                toolbar = toolbar.child(center);
            }
            Page::Settings => {
                center = center
                    .child(icon("settings", 15.0, theme.accent_text))
                    .child(
                        div()
                            .child("Settings")
                            .text_size(px(12.0))
                            .text_color(theme.accent_text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .child("/")
                            .text_size(px(12.0))
                            .text_color(theme.muted_soft),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child("Application preferences and integrations")
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    );
                toolbar = toolbar.child(center);
            }
        }

        // Prepare dock header (right side)
        if show_prepare {
            let (source_name, source_details, source_tooltip) = self.prepare_source_summary();
            let compact_reveal = prepare_width < 390.0;
            let prepare = div()
                .flex_none()
                .w(px(prepare_width))
                .h_full()
                .bg(theme.canvas_background())
                .border_t_1()
                .border_l_1()
                .border_b_1()
                .border_color(theme.workbench_border.opacity(0.62))
                .flex()
                .flex_row()
                .items_center()
                .px(px(12.0))
                .gap(px(8.0))
                .overflow_hidden()
                .child(
                    div()
                        .id("prepare-source-name")
                        .flex_1()
                        .min_w(px(40.0))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .child(source_name)
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::MEDIUM)
                                .text_ellipsis(),
                        )
                        .child(
                            div()
                                .child(source_details)
                                .text_size(px(10.0))
                                .text_color(theme.muted)
                                .text_ellipsis(),
                        )
                        .tooltip(move |_window, cx| {
                            crate::tooltip_view(cx, source_tooltip.clone().into())
                        }),
                )
                .child(
                    workbench_button(
                        "reveal-in-library",
                        if compact_reveal { "" } else { "Reveal" },
                        "folder",
                        ButtonKind::Secondary,
                        true,
                        compact_reveal,
                        "Reveal in library",
                        cx,
                        |app, cx| {
                            app.command(Command::RevealSelectedInLibrary);
                            cx.notify();
                        },
                    )
                    .h(px(32.0)),
                )
                .child(div().w(px(1.0)).h(px(22.0)).bg(theme.border))
                .child(workbench_button(
                    "toolbar-close",
                    "",
                    "close",
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
            toolbar = toolbar.child(prepare);
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
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.sort_menu_open {
                    app.sort_menu_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .w(px(268.0))
            .max_h(px((self.window_size.1 - 120.0).max(260.0)))
            .rounded(px(10.0))
            .bg(theme.overlay_surface())
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |menu| {
                menu.shadow(theme.material_shadow())
            })
            .when(!theme.is_frosted(), |menu| menu.shadow_lg())
            .py(px(6.0))
            .flex()
            .flex_col()
            .overflow_scroll()
            .scrollbar_width(px(10.0));

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
                    .h(px(24.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .child(title)
                    .text_size(px(10.0))
                    .text_color(theme.muted_soft)
                    .font_weight(FontWeight::SEMIBOLD),
            );
            for (label, value) in options.iter() {
                let label = *label;
                let value = *value;
                let selected = current == value;
                let rest_face = if selected {
                    theme.selection_face(TactileState::Rest, false)
                } else {
                    theme.transparent().into()
                };
                let rest_edge = if selected {
                    theme.tactile_edge(TactileState::Rest, false)
                } else {
                    theme.transparent()
                };
                let rest_shadow = if selected {
                    theme.tactile_shadow(TactileState::Rest, true)
                } else {
                    Vec::new()
                };
                let hover_face = if selected {
                    theme.selection_face(TactileState::Hover, false)
                } else {
                    theme.control_face(TactileState::Hover)
                };
                let hover_edge = theme.tactile_edge(TactileState::Hover, false);
                let hover_shadow = theme.tactile_shadow(TactileState::Hover, true);
                let pressed_face = theme.selection_face(TactileState::Pressed, false);
                let pressed_edge = theme.tactile_edge(TactileState::Pressed, false);
                let pressed_shadow = theme.tactile_shadow(TactileState::Pressed, true);
                let mut item = div()
                    .id(SharedString::from(format!("sort-{value}")))
                    .w_full()
                    .h(px(32.0))
                    .mx(px(6.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .relative()
                    .top(px(0.0))
                    .border_1()
                    .border_color(rest_edge)
                    .cursor_pointer()
                    .tab_index(0)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(10.0))
                    .bg(rest_face)
                    .shadow(rest_shadow)
                    .hover({
                        let hover_shadow = hover_shadow.clone();
                        move |style| {
                            style
                                .bg(hover_face)
                                .border_color(hover_edge)
                                .shadow(hover_shadow.clone())
                        }
                    })
                    .active({
                        let pressed_shadow = pressed_shadow.clone();
                        move |style| {
                            style
                                .top(px(1.0))
                                .bg(pressed_face)
                                .border_color(pressed_edge)
                                .shadow(pressed_shadow.clone())
                        }
                    })
                    .focus(move |style| style.border_2().border_color(theme.accent))
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
                            .text_color(if selected {
                                theme.text
                            } else {
                                theme.text_soft
                            })
                            .font_weight(if selected {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            }),
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
            .child(add_section(
                self,
                "EXPLORER FOLDERS",
                &folder_options,
                &folder_sort_mode,
                cx,
            ));
        popup_fade(menu, "sort-menu-fade")
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
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.activity_open {
                    app.activity_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .w(px(326.0))
            .rounded(px(10.0))
            .bg(theme.overlay_surface())
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |popup| {
                popup.shadow(theme.material_shadow())
            })
            .when(!theme.is_frosted(), |popup| popup.shadow_lg())
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
                        .child(tracked("BACKGROUND ACTIVITY"))
                        .text_size(px(10.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::SEMIBOLD),
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
                        .text_color(if active_count > 0 {
                            theme.accent_text
                        } else {
                            theme.muted_soft
                        })
                        .font_weight(FontWeight::SEMIBOLD),
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
            div()
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
                                if app.scan.cancelling {
                                    "Stopping scan"
                                } else {
                                    "Stop scan"
                                },
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
                        .child(
                            div()
                                .w(px(120.0))
                                .child(progress_bar(progress, indeterminate)),
                        ),
                )
        };

        if scanning {
            let title = SharedString::from(format!("Library scan  ·  {}", self.scan.root_name));
            list = list.child(add_row(
                self,
                title,
                if scan_message.is_empty() {
                    "Updating the index".to_string()
                } else {
                    scan_message
                },
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
        popup_fade(popup, "activity-fade")
    }

    pub fn render_workspace_menu(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let index = self.workspace_menu_target;
        let target = self.workspaces.get(index).cloned();
        let workspace_count = self.workspaces.len();
        let closed_available = self.closed_count_known();
        let mut menu = div()
            .id("workspace-menu")
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.workspace_menu_open {
                    app.workspace_menu_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .w(px(260.0))
            .rounded(px(10.0))
            .bg(theme.overlay_surface())
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |menu| {
                menu.shadow(theme.material_shadow())
            })
            .when(!theme.is_frosted(), |menu| menu.shadow_lg())
            .px(px(6.0))
            .py(px(6.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .overflow_hidden();

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
                .flex_none()
                .px(px(9.0))
                .rounded(px(6.0))
                .relative()
                .top(px(0.0))
                .border_1()
                .border_color(theme.transparent())
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(icon(glyph, 16.0, theme.muted))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(label)
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .text_ellipsis(),
                );
            item = item.when(!enabled, |this| this.opacity(0.46).cursor_default());
            item = item.when(enabled, |this| {
                let hover_face = theme.control_face(TactileState::Hover);
                let hover_edge = theme.tactile_edge(TactileState::Hover, false);
                let hover_shadow = theme.tactile_shadow(TactileState::Hover, false);
                let pressed_face = theme.control_face(TactileState::Pressed);
                let pressed_edge = theme.tactile_edge(TactileState::Pressed, false);
                let pressed_shadow = theme.tactile_shadow(TactileState::Pressed, false);
                let focus_face = theme.control_face(TactileState::Hover);
                let focus_edge = theme.accent;
                this.cursor_pointer()
                    .tab_index(0)
                    .hover(move |style| {
                        style
                            .bg(hover_face)
                            .border_color(hover_edge)
                            .shadow(hover_shadow.clone())
                    })
                    .active(move |style| {
                        style
                            .top(px(1.0))
                            .bg(pressed_face)
                            .border_color(pressed_edge)
                            .shadow(pressed_shadow.clone())
                    })
                    .focus(move |style| style.bg(focus_face).border_2().border_color(focus_edge))
            });
            if let Some(command) = command {
                let click_command = command.clone();
                let enter_command = command.clone();
                item = item.when(enabled, |this| {
                    this.on_click(cx.listener(move |app, _event, _window, cx| {
                        app.workspace_menu_open = false;
                        app.command(click_command.clone());
                        cx.notify();
                    }))
                    .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                        app.workspace_menu_open = false;
                        app.command(enter_command.clone());
                        cx.notify();
                        cx.stop_propagation();
                    }))
                    .on_action(cx.listener(
                        move |app, _: &crate::ActivateSpace, _window, cx| {
                            app.workspace_menu_open = false;
                            app.command(command.clone());
                            cx.notify();
                            cx.stop_propagation();
                        },
                    ))
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
                .child(
                    add_item(self, "ws-rename", "Rename workspace", "✎", true, None, cx)
                        .on_click(cx.listener(move |app, _event, _window, cx| {
                            app.workspace_menu_open = false;
                            app.begin_workspace_rename(index);
                            cx.notify();
                        }))
                        .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                            app.workspace_menu_open = false;
                            app.begin_workspace_rename(index);
                            cx.notify();
                            cx.stop_propagation();
                        }))
                        .on_action(cx.listener(
                            move |app, _: &crate::ActivateSpace, _window, cx| {
                                app.workspace_menu_open = false;
                                app.begin_workspace_rename(index);
                                cx.notify();
                                cx.stop_propagation();
                            },
                        )),
                )
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
                        if cancelling {
                            "Stopping scan…"
                        } else {
                            "Stop library scan"
                        },
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
                    "close",
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
                .child(
                    add_item(self, "ws-new", "New workspace…", "+", true, None, cx)
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            app.workspace_menu_open = false;
                            app.choose_new_workspace_folder(cx);
                        }))
                        .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
                            app.workspace_menu_open = false;
                            app.choose_new_workspace_folder(cx);
                            cx.stop_propagation();
                        }))
                        .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
                            app.workspace_menu_open = false;
                            app.choose_new_workspace_folder(cx);
                            cx.stop_propagation();
                        })),
                )
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
        popup_fade(menu, "workspace-menu-fade")
    }

    fn begin_workspace_rename(&mut self, index: usize) {
        self.renaming_workspace = Some(index);
        self.platform_input_focus = None;
        let title = self
            .workspaces
            .get(index)
            .map(|workspace| workspace.title.clone())
            .unwrap_or_default();
        let state = self.field_state_mut("workspace-rename");
        state.text = title;
        state.caret = state.text.chars().count();
        self.focused_field = Some("workspace-rename".to_string());
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
        cx.spawn(
            move |_this: WeakEntity<crate::App>, _cx: &mut AsyncApp| async move {
                if let Ok(Ok(Some(mut paths))) = folder.await {
                    if let Some(path) = paths.pop() {
                        let _ = controller.send(Command::CreateWorkspace(
                            path.to_string_lossy().into_owned(),
                        ));
                    }
                }
            },
        )
        .detach();
    }

    pub fn render_header(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
        let picking = self.random_picking;
        let width = self.window_size.0;
        let compact = width < 1120.0;
        let very_compact = width < 1000.0;
        let titlebar_leading = if cfg!(target_os = "macos") {
            116.0
        } else {
            16.0
        };
        let titlebar_trailing = 10.0;
        let mut header = div()
            .id("header")
            .w_full()
            .min_w(px(0.0))
            .h(px(TITLE_BAR_HEIGHT))
            .pl(px(titlebar_leading))
            .pr(px(titlebar_trailing))
            .bg(theme.chrome_background())
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .overflow_hidden();

        // Linux compositors own modifier-drag move/resize gestures. A full-header
        // `xdg_toplevel.move` listener competes with that contract and, because
        // GPUI bubbles through overlapping hitboxes, also turns presses on the
        // search field into window moves. Keep the legacy custom-titlebar path
        // only on platforms that do not delegate this interaction to Wayland/X11.
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        {
            header = header.child(
                div()
                    .id("titlebar-drag")
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|_app, _event, window, _cx| {
                            if !window.is_fullscreen() {
                                window.start_window_move();
                            }
                        }),
                    )
                    .on_click(cx.listener(|_app, event: &gpui::ClickEvent, window, _cx| {
                        if event.click_count() >= 2 && !window.is_fullscreen() {
                            window.zoom_window();
                        }
                    })),
            );
        }

        header = header.child(
            workbench_button(
                "command-palette",
                "",
                "terminal",
                ButtonKind::Primary,
                true,
                true,
                "Command palette  ·  ⌘⇧P",
                cx,
                |app, cx| {
                    if app.command_open {
                        app.close_command_center();
                    } else {
                        app.command_scope = "commands".to_string();
                        app.command_query.clear();
                        let state = app.field_state_mut("command-center");
                        state.text.clear();
                        state.caret = 0;
                        app.open_command_center(cx);
                    }
                    cx.notify();
                },
            )
            .w(px(34.0))
            .h(px(34.0)),
        );

        header = header.child(div().w(px(12.0)).flex_none());

        let can_back = self
            .workspaces
            .get(self.active_workspace_index)
            .map(|workspace| workspace.has_back)
            .unwrap_or(false);
        let can_forward = self
            .workspaces
            .get(self.active_workspace_index)
            .map(|workspace| workspace.has_forward)
            .unwrap_or(false);
        header = header.child(
            workbench_button(
                "nav-back",
                "",
                "chevron-left",
                ButtonKind::Ghost,
                can_back,
                true,
                "Go back  ·  ⌘[",
                cx,
                |app, cx| {
                    app.command(Command::NavigateBack);
                    cx.notify();
                },
            )
            .w(px(32.0))
            .h(px(32.0)),
        );
        header = header.child(
            workbench_button(
                "nav-forward",
                "",
                "chevron-right",
                ButtonKind::Ghost,
                can_forward,
                true,
                "Go forward  ·  ⌘]",
                cx,
                |app, cx| {
                    app.command(Command::NavigateForward);
                    cx.notify();
                },
            )
            .w(px(32.0))
            .h(px(32.0)),
        );

        header = header.child(div().w(px(if compact { 6.0 } else { 12.0 })).flex_none());

        let empty_field = FieldState::default();
        let search_hint = if cfg!(target_os = "macos") {
            "⌘K"
        } else {
            "Ctrl K"
        };
        let show_hint = !very_compact;
        let command_popup = self
            .command_open
            .then(|| self.render_command_center(cx).into_any());
        let search_field = crate::widgets::field_with_icon_hint(
            "command-center",
            if self.effective_command_scope() == "commands" {
                "Run a command"
            } else if very_compact {
                "Search ClipRelay"
            } else {
                "Search videos, folders, and commands"
            },
            self.fields.get("command-center").unwrap_or(&empty_field),
            self.focused_field.as_deref() == Some("command-center"),
            true,
            false,
            Some("search"),
            if show_hint { Some(search_hint) } else { None },
            cx,
        )
        .w_full()
        .h_full()
        .px(px(10.0))
        .rounded(px(5.0))
        .bg(theme.raised);
        header = header.child(
            anchored_overlay(
                search_field,
                command_popup,
                OverlayPlacement::BelowStart,
                size(px(0.0), px(34.0)),
            )
            .flex_1()
            .min_w(px(if very_compact { 230.0 } else { 280.0 }))
            .max_w(px(680.0))
            .h(px(34.0)),
        );

        let has_query = !self
            .fields
            .get("command-center")
            .map(|field| field.text.trim().is_empty())
            .unwrap_or(true);
        if has_query && !very_compact {
            header = header.child(
                workbench_button(
                    "clear-search",
                    "",
                    "x",
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
                )
                .w(px(28.0))
                .h(px(28.0)),
            );
        }

        header = header.child(div().w(px(if compact { 2.0 } else { 7.0 })).flex_none());

        let activity_active =
            self.scan.active || self.checking || self.timeline_loading || self.publish.active;
        let activity_popup = self
            .activity_open
            .then(|| self.render_activity_popup(cx).into_any());
        let activity_trigger = div()
            .id("activity-button")
            .track_focus(&self.activity_source_focus)
            .size_full()
            .rounded(px(RADIUS_SM))
            .relative()
            .top(px(0.0))
            .border_1()
            .border_color(if self.activity_open {
                theme.tactile_edge(TactileState::Rest, false)
            } else {
                theme.transparent()
            })
            .cursor_pointer()
            .tab_index(0)
            .flex()
            .items_center()
            .justify_center()
            .bg(if self.activity_open {
                theme.selection_face(TactileState::Rest, false)
            } else {
                theme.transparent().into()
            })
            .shadow(if self.activity_open {
                theme.tactile_shadow(TactileState::Rest, true)
            } else {
                Vec::new()
            })
            .hover(|style| {
                style
                    .bg(theme.control_face(TactileState::Hover))
                    .border_color(theme.tactile_edge(TactileState::Hover, false))
                    .shadow(theme.tactile_shadow(TactileState::Hover, true))
            })
            .active(|style| {
                style
                    .top(px(1.0))
                    .bg(theme.control_face(TactileState::Pressed))
                    .border_color(theme.tactile_edge(TactileState::Pressed, false))
                    .shadow(theme.tactile_shadow(TactileState::Pressed, true))
            })
            .focus(|style| style.border_2().border_color(theme.accent))
            .child(icon("activity", 15.0, theme.muted))
            .when(activity_active, |this| {
                this.child(
                    div()
                        .absolute()
                        .top(px(3.0))
                        .right(px(3.0))
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(theme.accent)
                        .border_1()
                        .border_color(theme.surface),
                )
            })
            .tooltip(move |_window, cx| {
                crate::tooltip_view(
                    cx,
                    if activity_active {
                        "Background activity".into()
                    } else {
                        "No background work  ·  View activity".into()
                    },
                )
            })
            .on_click(cx.listener(|app, _event, _window, cx| {
                app.toggle_activity_popup();
                cx.notify();
            }));
        header = header.child(
            anchored_overlay(
                activity_trigger,
                activity_popup,
                OverlayPlacement::BelowEnd,
                size(px(30.0), px(30.0)),
            )
            .flex_none()
            .w(px(30.0))
            .h(px(30.0)),
        );

        let summary = self.random_summary.clone();
        {
            let tooltip: SharedString = format!("Random sources: {summary}").into();
            let width_px = if very_compact {
                116.0
            } else if compact {
                132.0
            } else {
                150.0
            };
            let source_selected = self.random_has_selection || self.random_popup_open;
            let source_rest_face: Background = if source_selected {
                theme.selection_face(TactileState::Rest, false)
            } else {
                theme.transparent().into()
            };
            let source_hover_face = if source_selected {
                theme.selection_face(TactileState::Hover, false)
            } else {
                theme.control_face(TactileState::Hover)
            };
            let mut sources = div()
                .id("random-sources")
                .track_focus(&self.random_source_focus)
                .flex_none()
                .h(px(32.0))
                .px(px(9.0))
                .w(px(width_px))
                .rounded(px(RADIUS_SM))
                .relative()
                .top(px(0.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .cursor_pointer()
                .tab_index(0)
                .bg(source_rest_face)
                .shadow(if source_selected {
                    theme.tactile_shadow(TactileState::Rest, true)
                } else {
                    Vec::new()
                })
                .border_1()
                .border_color(if self.random_popup_open {
                    theme.accent
                } else if source_selected {
                    theme.tactile_edge(TactileState::Rest, false)
                } else {
                    theme.transparent()
                })
                .hover(|style| {
                    style
                        .bg(source_hover_face)
                        .border_color(theme.tactile_edge(TactileState::Hover, false))
                        .shadow(theme.tactile_shadow(TactileState::Hover, true))
                })
                .active(|style| {
                    style
                        .top(px(1.0))
                        .bg(theme.selection_face(TactileState::Pressed, false))
                        .border_color(theme.tactile_edge(TactileState::Pressed, false))
                        .shadow(theme.tactile_shadow(TactileState::Pressed, true))
                })
                .focus(|style| style.border_2().border_color(theme.accent))
                .tooltip(move |_window, cx| crate::tooltip_view(cx, tooltip.clone()))
                .when(!has_root, |this| this.opacity(0.42).cursor_default())
                .child(icon("folders", 14.0, theme.muted))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(summary)
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .text_ellipsis(),
                )
                .child(icon(
                    if self.random_popup_open {
                        "chevron-up"
                    } else {
                        "chevron-down"
                    },
                    12.0,
                    theme.muted,
                ));
            if has_root {
                sources = sources.on_click(cx.listener(|app, _event, _window, cx| {
                    app.toggle_random_popup(cx);
                }));
            }
            let random_popup = self
                .random_popup_open
                .then(|| self.render_random_popup(cx).into_any());
            header = header.child(
                anchored_overlay(
                    sources,
                    random_popup,
                    OverlayPlacement::BelowEnd,
                    size(px(width_px), px(32.0)),
                )
                .flex_none()
                .w(px(width_px))
                .h(px(32.0)),
            );
        }

        if !compact {
            header = header.child(
                workbench_button(
                    "reset-shuffle",
                    "",
                    "refresh",
                    ButtonKind::Ghost,
                    has_root,
                    true,
                    "Reset shuffle history",
                    cx,
                    |app, cx| {
                        app.command(Command::ResetShuffle);
                        cx.notify();
                    },
                )
                .w(px(28.0))
                .h(px(28.0)),
            );
        }

        let pick_label = if picking {
            "Picking…"
        } else if very_compact {
            "Random"
        } else {
            "Pick random"
        };
        let pick_width = if very_compact {
            96.0
        } else if compact {
            112.0
        } else {
            136.0
        };
        header = header.child(
            workbench_button(
                "pick-random",
                pick_label,
                "shuffle",
                ButtonKind::Primary,
                has_root,
                false,
                "Pick random video  ·  R",
                cx,
                |app, cx| {
                    app.command(Command::PickRandom);
                    cx.notify();
                },
            )
            .w(px(pick_width))
            .h(px(34.0)),
        );

        header
    }

    pub fn render_workspace_tabs(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let workspaces = self.workspaces.clone();
        let active_index = self.active_workspace_index;
        let action_rail_width = SIDEBAR_COLLAPSED_WIDTH;
        let available = (self.window_size.0 - action_rail_width - 8.0).max(160.0);
        let tab_width = (available / workspaces.len().max(1) as f32).clamp(132.0, 218.0);
        let renaming = self.renaming_workspace;
        let empty_field = FieldState::default();
        let mut tabs = div()
            .id("workspace-tabs-scroll")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .flex_row()
            .overflow_x_scroll()
            // Workspace tabs still scroll when the window cannot fit them,
            // but the scrollbar must not reserve a dark gutter beneath the
            // strip. Wheel/trackpad scrolling and programmatic reveal remain.
            .scrollbar_width(px(0.0))
            .track_scroll(&self.tab_scroll);

        for (index, workspace) in workspaces.iter().enumerate() {
            let id = workspace.id.clone();
            let middle_click_id = id.clone();
            let title = workspace.title.clone();
            let root = workspace.root.clone();
            let active = index == active_index;
            let scanning = workspace.scanning;
            let cancelling = workspace.scan_cancelling;
            let is_renaming = renaming == Some(index);
            let tab_rest_face: Background = if active {
                theme.selection_face(TactileState::Rest, false)
            } else {
                theme.transparent().into()
            };
            let tab_hover_face = if active {
                theme.selection_face(TactileState::Hover, false)
            } else {
                theme.control_face(TactileState::Hover)
            };
            let mut tab = div()
                .id(SharedString::from(format!("tab-{id}")))
                .w(px(tab_width))
                .flex_none()
                .h_full()
                .px(px(11.0))
                .relative()
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .rounded_tl(px(3.0))
                .rounded_tr(px(3.0))
                .top(px(0.0))
                .bg(tab_rest_face)
                .shadow(if active {
                    theme.tactile_shadow(TactileState::Rest, true)
                } else {
                    Vec::new()
                })
                .border_r_1()
                .border_color(theme.border)
                .hover(|style| {
                    style
                        .bg(tab_hover_face)
                        .shadow(theme.tactile_shadow(TactileState::Hover, true))
                })
                .active(|style| {
                    style
                        .top(px(1.0))
                        .bg(theme.selection_face(TactileState::Pressed, false))
                        .shadow(theme.tactile_shadow(TactileState::Pressed, true))
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
                .child(icon(
                    if cancelling {
                        "square"
                    } else if scanning {
                        "refresh"
                    } else {
                        "folder"
                    },
                    14.5,
                    if active || scanning {
                        theme.accent_text
                    } else {
                        theme.muted
                    },
                ));
            if is_renaming {
                tab = tab.child(
                    field(
                        "workspace-rename",
                        "",
                        self.fields.get("workspace-rename").unwrap_or(&empty_field),
                        self.focused_field.as_deref() == Some("workspace-rename"),
                        true,
                        false,
                        cx,
                    )
                    .flex_1()
                    .min_w(px(0.0))
                    .h(px(26.0))
                    .px(px(6.0)),
                );
            } else {
                tab = tab
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(title)
                            .text_size(px(12.5))
                            .text_color(if active { theme.text } else { theme.text_soft })
                            .font_weight(if active {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            })
                            .text_ellipsis(),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("tab-close-{id}")))
                            .flex_none()
                            .w(px(24.0))
                            .h(px(24.0))
                            .rounded(px(RADIUS_SM))
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.active).text_color(theme.text))
                            .child(icon("x", 12.5, theme.muted_soft))
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
                        let title = app
                            .workspaces
                            .get(index)
                            .map(|workspace| workspace.title.clone())
                            .unwrap_or_default();
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
                    app.dismiss_root_popovers();
                    app.workspace_menu_source = crate::WorkspaceMenuSource::Tab(index);
                    app.workspace_menu_open = true;
                    cx.notify();
                }),
            );
            tab = tab.when(
                self.workspace_menu_source == crate::WorkspaceMenuSource::Tab(index),
                |tab| tab.track_focus(&self.workspace_source_focus),
            );
            if active {
                tab = tab.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(6.0))
                        .right(px(6.0))
                        .h(px(2.0))
                        .rounded_b(px(1.0))
                        .bg(theme.accent),
                );
            }
            let tab_menu = (self.workspace_menu_open
                && self.workspace_menu_source == crate::WorkspaceMenuSource::Tab(index))
            .then(|| self.render_workspace_menu(cx).into_any());
            tabs = tabs.child(
                anchored_overlay(
                    tab,
                    tab_menu,
                    OverlayPlacement::AboveEnd,
                    size(px(tab_width), px(WORKSPACE_TAB_HEIGHT)),
                )
                .flex_none()
                .w(px(tab_width))
                .h(px(WORKSPACE_TAB_HEIGHT)),
            );
        }

        let gap = 0.0;
        let active_left = active_index as f32 * (tab_width + gap);
        let scroll = f32::from(self.tab_scroll.offset().x);
        let viewport_width = available.max(100.0);
        if active_left < scroll {
            self.tab_scroll.set_offset(point(px(active_left), px(0.0)));
        } else if active_left + tab_width > scroll + viewport_width {
            self.tab_scroll
                .set_offset(point(px(active_left + tab_width - viewport_width), px(0.0)));
        }

        let tab_actions_popup = (self.workspace_menu_open
            && self.workspace_menu_source == crate::WorkspaceMenuSource::TabActions)
            .then(|| self.render_workspace_menu(cx).into_any());
        let tab_actions_button = workbench_button(
            "workspace-actions",
            "",
            "ellipsis",
            ButtonKind::Ghost,
            true,
            true,
            "Workspace actions",
            cx,
            |app, cx| {
                app.workspace_menu_target = app.active_workspace_index;
                app.toggle_workspace_popup(crate::WorkspaceMenuSource::TabActions);
                cx.notify();
            },
        )
        .when(
            self.workspace_menu_source == crate::WorkspaceMenuSource::TabActions,
            |button| button.track_focus(&self.workspace_source_focus),
        );
        let actions = div()
            .flex_none()
            .w(px(action_rail_width))
            .h_full()
            .px(px(4.0))
            .border_l_1()
            .border_color(theme.border)
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(workbench_button(
                "new-workspace",
                "",
                "plus",
                ButtonKind::Ghost,
                true,
                true,
                "New workspace",
                cx,
                |app, cx| app.choose_new_workspace_folder(cx),
            ))
            .child(
                anchored_overlay(
                    tab_actions_button,
                    tab_actions_popup,
                    OverlayPlacement::AboveEnd,
                    size(px(WORKBENCH_CONTROL_HEIGHT), px(WORKBENCH_CONTROL_HEIGHT)),
                )
                .flex_none()
                .w(px(WORKBENCH_CONTROL_HEIGHT))
                .h(px(WORKBENCH_CONTROL_HEIGHT)),
            );

        div()
            .id("workspace-tabs")
            .w_full()
            .min_w(px(0.0))
            .h(px(WORKSPACE_TAB_HEIGHT))
            .flex_none()
            .bg(theme.section_background())
            .border_t_1()
            .border_color(theme.workbench_border.opacity(0.62))
            .flex()
            .flex_row()
            .child(tabs)
            .child(actions)
    }

    fn render_shortcut_guide(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let width = (self.window_size.0 - 32.0).clamp(276.0, 390.0);
        let max_height = (self.window_size.1 - 48.0).clamp(260.0, 560.0);
        // The guide has dense text, so Frosted Glass gets one denser neutral
        // overlay layer than a lightweight menu while remaining material-like.
        let guide_surface: Background = if self.theme_mode == ThemeMode::FrostedGlass {
            Hsla::from(Rgba {
                r: 5.0 / 255.0,
                g: 6.0 / 255.0,
                b: 9.0 / 255.0,
                a: 1.0,
            })
            .opacity(0.84)
            .into()
        } else {
            theme.overlay_surface()
        };
        let guide_row_face: Background = if self.theme_mode == ThemeMode::FrostedGlass {
            Hsla::from(Rgba {
                r: 21.0 / 255.0,
                g: 24.0 / 255.0,
                b: 32.0 / 255.0,
                a: 1.0,
            })
            .opacity(0.72)
            .into()
        } else {
            theme.control_face(TactileState::Rest)
        };
        let mut content = div()
            .id("shortcut-guide-content")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .px(px(12.0))
            .pb(px(12.0))
            .flex()
            .flex_col()
            .gap(px(6.0));
        let mut current_group = "";
        for (index, definition) in SHORTCUTS.iter().enumerate() {
            if definition.group != current_group {
                current_group = definition.group;
                content = content.child(
                    div()
                        .id(SharedString::from(format!("shortcut-guide-group-{index}")))
                        .pt(px(if index == 0 { 2.0 } else { 9.0 }))
                        .pb(px(2.0))
                        .child(definition.group)
                        .text_size(px(10.5))
                        .text_color(theme.muted)
                        .font_weight(FontWeight::SEMIBOLD),
                );
            }
            content = content.child(
                div()
                    .id(SharedString::from(format!("shortcut-guide-row-{index}")))
                    .w_full()
                    .min_h(px(32.0))
                    .px(px(8.0))
                    .rounded(px(RADIUS_SM))
                    .bg(guide_row_face)
                    .border_1()
                    .border_color(theme.tactile_edge(TactileState::Rest, false))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(definition.label)
                            .text_size(px(12.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::MEDIUM)
                            .text_ellipsis(),
                    )
                    .child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .py(px(3.0))
                            .rounded(px(4.0))
                            .bg(theme.raised)
                            .border_1()
                            .border_color(theme.border)
                            .child(shortcut_keys(*definition))
                            .text_size(px(10.5))
                            .text_color(theme.text_soft)
                            .font_weight(FontWeight::MEDIUM),
                    ),
            );
        }

        div()
            .id("shortcut-guide")
            .track_focus(&self.shortcut_guide_focus)
            .tab_index(0)
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, window, cx| {
                if app.shortcut_guide_open {
                    app.close_shortcut_guide(window);
                    cx.notify();
                }
            }))
            .w(px(width))
            .max_h(px(max_height))
            .rounded(px(10.0))
            .bg(guide_surface)
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |popup| {
                popup.shadow(theme.material_shadow())
            })
            .when(!theme.is_frosted(), |popup| popup.shadow_lg())
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .h(px(52.0))
                    .flex_none()
                    .px(px(14.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .gap(px(9.0))
                    .child(icon("keyboard", 17.0, theme.accent))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child("Keyboard shortcuts")
                            .text_size(px(14.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .child("Esc")
                            .text_size(px(11.0))
                            .text_color(theme.muted),
                    ),
            )
            .child(content)
    }

    pub fn render_context_menu(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let target = self.context_menu.clone();
        let selected_index = self.context_menu_selected;
        let (title, detail, items, path_is_present) = match target.as_ref() {
            Some(ContextMenuTarget::Video(row)) => (
                "Video actions",
                row.name.clone(),
                context_menu_items(target.as_ref().unwrap()),
                target.as_ref().is_some_and(ContextMenuTarget::has_path),
            ),
            Some(ContextMenuTarget::Folder { name, .. }) => (
                "Folder actions",
                name.clone(),
                context_menu_items(target.as_ref().unwrap()),
                target.as_ref().is_some_and(ContextMenuTarget::has_path),
            ),
            None => ("Actions", String::new(), Vec::new(), false),
        };
        let mut menu = div()
            .id("item-context-menu")
            .track_focus(&self.context_menu_focus)
            .tab_index(0)
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, window, cx| {
                if app.context_menu.is_some() {
                    app.close_context_menu(window);
                    cx.notify();
                }
            }))
            .w(px(254.0))
            .max_h(px((self.window_size.1 - 28.0).clamp(220.0, 430.0)))
            .rounded(px(10.0))
            .bg(theme.overlay_surface())
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |menu| {
                menu.shadow(theme.material_shadow())
            })
            .when(!theme.is_frosted(), |menu| menu.shadow_lg())
            .py(px(6.0))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .px(px(12.0))
                    .pb(px(7.0))
                    .pt(px(3.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .child(title)
                            .text_size(px(11.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .child(detail)
                            .text_size(px(12.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::MEDIUM)
                            .text_ellipsis(),
                    ),
            );
        for (index, item) in items.into_iter().enumerate() {
            if item.separator_before {
                menu = menu.child(div().my(px(5.0)).mx(px(10.0)).h(px(1.0)).bg(theme.border));
            }
            let selected = selected_index == index;
            let face = if selected {
                theme.selection_face(TactileState::Rest, false)
            } else {
                theme.transparent().into()
            };
            let selected_shadow = if selected {
                theme.tactile_shadow(TactileState::Rest, true)
            } else {
                Vec::new()
            };
            menu = menu.child(
                div()
                    .id(SharedString::from(format!(
                        "context-menu-{:?}",
                        item.action
                    )))
                    .mx(px(6.0))
                    .h(px(34.0))
                    .px(px(8.0))
                    .rounded(px(RADIUS_SM))
                    .bg(face)
                    .border_1()
                    .border_color(if selected {
                        theme.tactile_edge(TactileState::Rest, false)
                    } else {
                        theme.transparent()
                    })
                    .shadow(selected_shadow)
                    .when(path_is_present, |row| {
                        row.cursor_pointer()
                            .hover(|style| {
                                style
                                    .bg(theme.control_face(TactileState::Hover))
                                    .border_color(theme.tactile_edge(TactileState::Hover, false))
                                    .shadow(theme.tactile_shadow(TactileState::Hover, true))
                            })
                            .active(|style| {
                                style
                                    .top(px(1.0))
                                    .bg(theme.control_face(TactileState::Pressed))
                                    .border_color(theme.tactile_edge(TactileState::Pressed, false))
                                    .shadow(theme.tactile_shadow(TactileState::Pressed, true))
                            })
                            .on_click(cx.listener(move |app, _event, window, cx| {
                                app.activate_context_menu_item(index, window, cx);
                            }))
                    })
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(9.0))
                    .opacity(if path_is_present { 1.0 } else { 0.46 })
                    .child(icon(
                        item.glyph,
                        15.0,
                        if selected {
                            theme.accent_text
                        } else {
                            theme.muted
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(item.label)
                            .text_size(px(12.5))
                            .text_color(theme.text)
                            .font_weight(if selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_ellipsis(),
                    ),
            );
        }
        popup_fade(menu, "item-context-menu-fade")
    }

    pub fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let collapsed = self.sidebar_collapsed || self.window_size.0 < 1080.0;
        let narrow = self.window_size.0 < 1080.0;
        let width = if collapsed {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_EXPANDED_WIDTH
        };
        let page = self.page;
        let mut sidebar = div()
            .id("sidebar")
            .w(px(width))
            .flex_none()
            .h_full()
            .bg(theme.rail_background())
            .py(px(8.0))
            .flex()
            .flex_col()
            .gap(px(3.0));

        let nav_item = |app: &mut crate::App,
                        id: &'static str,
                        label: &'static str,
                        glyph: &'static str,
                        target: Page,
                        cx: &mut Context<crate::App>|
         -> Stateful<Div> {
            let selected = page == target && !(target == Page::Library && app.prepare.studio_mode);
            let theme = app.theme.clone();
            let rest_face: Background = if selected {
                theme.selection_face(TactileState::Rest, true)
            } else {
                theme.transparent().into()
            };
            let hover_face = if selected {
                theme.selection_face(TactileState::Hover, true)
            } else {
                theme.control_face(TactileState::Hover)
            };
            let pressed_face = theme.selection_face(TactileState::Pressed, true);
            let rest_edge = if selected {
                theme.tactile_edge(TactileState::Rest, true)
            } else {
                theme.transparent()
            };
            let hover_edge = theme.tactile_edge(TactileState::Hover, selected);
            let pressed_edge = theme.tactile_edge(TactileState::Pressed, true);
            let focus_edge = theme.accent;
            let icon_color = if selected {
                theme.accent_text
            } else {
                theme.muted
            };
            let label_color = if selected { theme.text } else { theme.muted };
            let rest_shadow = if selected {
                theme.tactile_shadow(TactileState::Rest, true)
            } else {
                Vec::new()
            };
            let hover_shadow = theme.tactile_shadow(TactileState::Hover, true);
            let pressed_shadow = theme.tactile_shadow(TactileState::Pressed, true);
            let mut item = div()
                .id(id)
                .when(collapsed, |this| this.mx(px(10.0)).w(px(40.0)))
                .when(!collapsed, |this| this.w_full())
                .h(px(40.0))
                .px(if collapsed { px(0.0) } else { px(12.0) })
                .rounded(px(RADIUS_SM))
                .relative()
                .top(px(0.0))
                .border_1()
                .border_color(rest_edge)
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(9.0))
                .cursor_pointer()
                .tab_index(0)
                .focus({
                    let hover_shadow = hover_shadow.clone();
                    move |style| {
                        style
                            .bg(hover_face)
                            .shadow(hover_shadow.clone())
                            .border_2()
                            .border_color(focus_edge)
                    }
                })
                .active({
                    let pressed_shadow = pressed_shadow.clone();
                    move |style| {
                        style
                            .top(px(1.0))
                            .bg(pressed_face)
                            .border_color(pressed_edge)
                            .shadow(pressed_shadow.clone())
                    }
                })
                .bg(rest_face)
                .shadow(rest_shadow)
                .hover(move |style| {
                    style
                        .bg(hover_face)
                        .border_color(hover_edge)
                        .shadow(hover_shadow.clone())
                })
                .child(icon(glyph, 17.0, icon_color));
            if !collapsed {
                item = item.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(label)
                        .text_size(px(13.0))
                        .text_color(label_color)
                        .font_weight(if selected {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_ellipsis(),
                );
            }
            item.on_click(cx.listener(move |app, _event, window, cx| {
                window.blur();
                app.navigate_to(target, cx);
            }))
            .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                app.navigate_to(target, cx);
                cx.stop_propagation();
            }))
            .on_action(cx.listener(
                move |app, _: &crate::ActivateSpace, _window, cx| {
                    app.navigate_to(target, cx);
                    cx.stop_propagation();
                },
            ))
        };

        sidebar = sidebar.child(nav_item(
            self,
            "nav-history",
            "History",
            "history",
            Page::History,
            cx,
        ));
        if self.prepare.studio_mode {
            sidebar = sidebar.child(
                div()
                    .id("nav-prepare")
                    .w_full()
                    .h(px(40.0))
                    .px(if collapsed { px(0.0) } else { px(12.0) })
                    .relative()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .gap(px(9.0))
                    .bg(theme.active)
                    .child(icon("edit", 17.0, theme.accent_text))
                    .when(!collapsed, |this| {
                        this.child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .child("Prepare")
                                .text_size(px(13.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_ellipsis(),
                        )
                    })
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(8.0))
                            .w(px(2.0))
                            .h(px(24.0))
                            .bg(theme.accent),
                    ),
            );
        }
        let guide_popup = self
            .shortcut_guide_open
            .then(|| self.render_shortcut_guide(cx).into_any());
        let guide_trigger = div()
            .id("sidebar-shortcuts")
            .track_focus(&self.shortcut_guide_source_focus)
            .w_full()
            .h(px(40.0))
            .px(if collapsed { px(0.0) } else { px(12.0) })
            .rounded(px(RADIUS_SM))
            .border_1()
            .border_color(if self.shortcut_guide_open {
                theme.tactile_edge(TactileState::Rest, false)
            } else {
                theme.transparent()
            })
            .bg(if self.shortcut_guide_open {
                theme.control_face(TactileState::Rest)
            } else {
                theme.transparent().into()
            })
            .cursor_pointer()
            .tab_index(0)
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(9.0))
            .hover(|style| {
                style
                    .bg(theme.control_face(TactileState::Hover))
                    .border_color(theme.tactile_edge(TactileState::Hover, false))
                    .shadow(theme.tactile_shadow(TactileState::Hover, true))
            })
            .active(|style| {
                style
                    .top(px(1.0))
                    .bg(theme.control_face(TactileState::Pressed))
                    .border_color(theme.tactile_edge(TactileState::Pressed, false))
                    .shadow(theme.tactile_shadow(TactileState::Pressed, true))
            })
            .focus(|style| style.border_2().border_color(theme.accent))
            .child(icon("keyboard", 17.0, theme.muted))
            .when(!collapsed, |this| {
                this.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child("Keyboard shortcuts")
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .text_ellipsis(),
                )
            })
            .tooltip(move |_window, cx| crate::tooltip_view(cx, "Keyboard shortcuts".into()))
            .on_click(cx.listener(|app, _event, window, cx| {
                app.toggle_shortcut_guide(window, cx);
            }))
            .on_action(cx.listener(|app, _: &crate::Activate, window, cx| {
                app.toggle_shortcut_guide(window, cx);
                cx.stop_propagation();
            }))
            .on_action(cx.listener(|app, _: &crate::ActivateSpace, window, cx| {
                app.toggle_shortcut_guide(window, cx);
                cx.stop_propagation();
            }));
        sidebar = sidebar.child(
            anchored_overlay(
                guide_trigger,
                guide_popup,
                OverlayPlacement::AboveStart,
                size(px(if collapsed { 40.0 } else { width }), px(40.0)),
            )
            .flex_none()
            .when(collapsed, |owner| owner.mx(px(10.0)).w(px(40.0)))
            .when(!collapsed, |owner| owner.w_full())
            .h(px(40.0)),
        );
        sidebar = sidebar.child(div().flex_1()).child(nav_item(
            self,
            "nav-settings",
            "Settings",
            "settings",
            Page::Settings,
            cx,
        ));
        if !collapsed && !narrow {
            sidebar = sidebar.child(
                div()
                    .id("toggle-sidebar")
                    .w_full()
                    .h(px(40.0))
                    .px(px(12.0))
                    .cursor_pointer()
                    .tab_index(0)
                    .focus(|style| style.bg(theme.hover))
                    .active(|style| style.bg(theme.accent_soft))
                    .flex()
                    .items_center()
                    .gap(px(9.0))
                    .hover(|style| style.bg(theme.hover))
                    .child(icon("chevron-left", 16.0, theme.muted))
                    .child(
                        div()
                            .flex_1()
                            .child("Collapse sidebar")
                            .text_size(px(12.0))
                            .text_color(theme.muted),
                    )
                    .tooltip(move |_window, cx| crate::tooltip_view(cx, "Collapse sidebar".into()))
                    .on_click(cx.listener(|app, _event, window, cx| {
                        window.blur();
                        app.set_setting(SIDEBAR_COLLAPSED, json!(true), cx);
                    }))
                    .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
                        app.set_setting(SIDEBAR_COLLAPSED, json!(true), cx);
                        cx.stop_propagation();
                    }))
                    .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
                        app.set_setting(SIDEBAR_COLLAPSED, json!(true), cx);
                        cx.stop_propagation();
                    })),
            );
        }
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
            .bg(theme.canvas_background())
            .border_l_1()
            .border_color(theme.workbench_border.opacity(0.62))
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
                    .child(div().w(px(58.0)).child(progress_bar(0.0, true))),
            );
        }

        // Keep the complete Prepare workflow available in the dock. Studio is
        // a focused presentation of the same workflow, not the only place
        // where editing and delivery settings can be reached.
        panel = panel.child(self.render_prepare_stage(cx, &theme, dock_width));
        panel = panel.child(self.render_prepare_tabs(cx));
        panel = panel.child(self.render_prepare_inspector(cx, &theme, dock_width));
        panel = panel.child(self.render_prepare_dock_footer(cx, &theme));
        panel
    }

    pub fn render_prepare_studio(&mut self, cx: &mut Context<Self>) -> impl Element {
        let app_theme = self.theme.clone();
        let theme = if app_theme.is_frosted() {
            app_theme.clone()
        } else {
            crate::theme::Theme::prepare_studio()
        };
        crate::widgets::set_current_theme(&theme);
        let width = self.window_size.0;
        let checking = self.checking;
        // A narrow Studio becomes a deliberate one-pane workspace. Keeping a
        // usable inspector matters more than squeezing both panes into slivers.
        let compact_studio = width < 980.0;
        let compact_inspector_open = self.prepare.compact_inspector_open;
        let inspector_width = self.prepare.studio_width.clamp(400.0, 520.0) as f32;
        let studio_inset = 12.0;
        let splitter_width = 12.0;
        let stage_width = if compact_studio {
            (width - studio_inset * 2.0).max(420.0)
        } else {
            (width - studio_inset * 2.0 - inspector_width - splitter_width).max(420.0)
        };

        let selected_name = self
            .selected
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_default();

        let header = div()
            .id("studio-header")
            .w_full()
            .h(px(52.0))
            .flex_none()
            .px(px(studio_inset))
            .bg(if theme.is_frosted() {
                theme.chrome_background()
            } else {
                theme.ink.into()
            })
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .child("Prepare")
                    .text_size(px(14.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::MEDIUM),
            )
            .child(
                div()
                    .child("/")
                    .text_size(px(14.0))
                    .text_color(theme.muted_soft),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(80.0))
                    .child(selected_name)
                    .text_size(px(14.0))
                    .text_color(theme.muted)
                    .font_weight(FontWeight::MEDIUM)
                    .text_ellipsis(),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded(px(4.0))
                            .bg(if checking {
                                theme.warning
                            } else {
                                theme.success
                            }),
                    )
                    .child(
                        div()
                            .child(if checking { "Checking" } else { "Ready" })
                            .text_size(px(13.5))
                            .text_color(theme.text_soft),
                    ),
            )
            .when(compact_studio, |header| {
                header.child(
                    button(
                        "studio-pane-toggle",
                        if compact_inspector_open {
                            "Video"
                        } else {
                            "Inspector"
                        },
                        ButtonKind::Secondary,
                        Some("panel"),
                        true,
                        cx,
                        move |app, cx| {
                            app.prepare.compact_inspector_open =
                                !app.prepare.compact_inspector_open;
                            cx.notify();
                        },
                    )
                    .h(px(34.0))
                    .px(px(12.0))
                    .rounded(px(2.0))
                    .bg(theme.surface_soft),
                )
            })
            .child(
                button(
                    "studio-mode-active",
                    "Back to Prepare",
                    ButtonKind::Secondary,
                    Some("arrow-left"),
                    true,
                    cx,
                    |app, cx| {
                        app.prepare.studio_mode = false;
                        app.prepare.compact_inspector_open = false;
                        cx.notify();
                    },
                )
                .h(px(34.0))
                .px(px(12.0))
                .rounded(px(2.0))
                .bg(theme.surface_soft),
            );

        let mut stage_col = div()
            .id("studio-stage")
            .w(px(stage_width))
            .h_full()
            .min_h(px(0.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.ink)
            .flex()
            .flex_col();
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
        if compact_studio && compact_inspector_open {
            stage_col = stage_col.child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .bg(theme.surface)
                    .flex()
                    .flex_col()
                    .child(self.render_prepare_tabs(cx))
                    .child(self.render_prepare_inspector(cx, &theme, stage_width)),
            );
        } else {
            stage_col = stage_col.child(self.render_prepare_stage(cx, &theme, stage_width));
        }

        let mut workbench = div()
            .id("studio-workbench")
            .flex_1()
            .min_h(px(0.0))
            .min_w(px(0.0))
            .px(px(studio_inset))
            .pt(px(4.0))
            .pb(px(studio_inset))
            .bg(if theme.is_frosted() {
                theme.canvas_background()
            } else {
                theme.ink.into()
            })
            .flex()
            .flex_row()
            .child(stage_col);

        if !compact_studio {
            workbench = workbench
                .child(
                    div()
                        .id("studio-splitter")
                        .w(px(splitter_width))
                        .h_full()
                        .bg(if theme.is_frosted() {
                            theme.canvas_background()
                        } else {
                            theme.ink.into()
                        })
                        .cursor_ew_resize()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                                app.prepare.drag = crate::prepare::DragHandle::StudioSplit;
                                app.prepare.drag_start_x = f64::from(event.position.x);
                                app.prepare.drag_start_value = app.prepare.studio_width;
                                cx.notify();
                            }),
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
                                app.prepare.studio_width = 428.0;
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
                        .min_h(px(0.0))
                        .bg(theme.surface)
                        .border_1()
                        .border_color(theme.border)
                        .flex()
                        .flex_col()
                        .child(self.render_prepare_tabs(cx))
                        .child(self.render_prepare_inspector(cx, &theme, inspector_width)),
                );
        }

        let studio = div()
            .id("prepare-studio")
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .bg(if theme.is_frosted() {
                theme.transparent()
            } else {
                theme.ink
            })
            .flex()
            .flex_col()
            .child(header)
            .child(workbench);
        crate::widgets::set_current_theme(&app_theme);
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
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.random_popup_open {
                    app.random_popup_open = false;
                    app.mark_menu_closed();
                    cx.notify();
                }
            }))
            .w(px(456.0))
            .max_h(px((self.window_size.1 - 64.0).clamp(320.0, 548.0)))
            .rounded(px(10.0))
            .bg(theme.overlay_surface())
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |popup| {
                popup.shadow(theme.material_shadow())
            })
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
                        .child(tracked("RANDOM SOURCES"))
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::SEMIBOLD),
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
                    "close",
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
                        .border_color(if all_selected {
                            theme.accent
                        } else {
                            theme.border_strong
                        })
                        .bg(if all_selected {
                            theme.accent
                        } else {
                            theme.raised
                        })
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
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    div()
                        .child(if all_selected {
                            "SELECTED"
                        } else {
                            "SELECT ALL"
                        })
                        .text_size(px(10.0))
                        .text_color(if all_selected {
                            theme.accent_text
                        } else {
                            theme.muted_soft
                        })
                        .font_weight(FontWeight::SEMIBOLD),
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
                                .text_color(if filter_text.is_empty() {
                                    theme.muted
                                } else {
                                    theme.text
                                }),
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
                        .bg(if selected_only {
                            theme.active
                        } else {
                            theme.transparent()
                        })
                        .border_1()
                        .border_color(if selected_only {
                            theme.accent
                        } else {
                            theme.transparent()
                        })
                        .flex()
                        .items_center()
                        .child("Selected only")
                        .text_size(px(11.0))
                        .text_color(if selected_only {
                            theme.text
                        } else {
                            theme.muted_soft
                        })
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
                        .opacity(if self.random_filter.is_empty() {
                            0.0
                        } else {
                            1.0
                        })
                        .child(icon("close", 11.0, theme.muted))
                        .tooltip(move |_window, cx| {
                            crate::tooltip_view(cx, "Clear folder search".into())
                        })
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
                        .font_weight(FontWeight::SEMIBOLD),
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
                .bg(if cursor_hit {
                    theme.active
                } else {
                    theme.transparent()
                })
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
                        .child(icon(
                            if expanded { "▾" } else { "▸" },
                            11.0,
                            theme.muted_soft,
                        ))
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
                        .border_color(if state > 0 {
                            theme.accent
                        } else {
                            theme.border_strong
                        })
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
                .child(icon(
                    "▸",
                    14.0,
                    if state > 0 {
                        theme.accent_text
                    } else {
                        theme.muted
                    },
                ))
                .child(
                    div()
                        .flex_1()
                        .child(name)
                        .text_size(px(12.0))
                        .text_color(if state > 0 {
                            theme.text
                        } else {
                            theme.text_soft
                        })
                        .font_weight(if state > 0 {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_ellipsis(),
                )
                .child(
                    div()
                        .child(format!("{count}"))
                        .text_size(px(10.0))
                        .text_color(if state > 0 {
                            theme.accent_text
                        } else {
                            theme.muted_soft
                        }),
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
                        div()
                            .w(px(120.0))
                            .child(crate::widgets::progress_bar(0.0, true))
                            .into_any()
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
                        .font_weight(FontWeight::SEMIBOLD)
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
        popup_fade(popup, "random-fade")
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
            .occlude()
            .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|app, _event, _window, cx| {
                if app.history_more_menu_post.take().is_some() {
                    app.history_more_menu_closed_at = std::time::Instant::now();
                    cx.notify();
                }
            }))
            .w(px(224.0))
            .rounded(px(10.0))
            .bg(theme.overlay_surface())
            .border_1()
            .border_color(theme.border_strong)
            .when(theme.is_frosted(), |menu| {
                menu.shadow(theme.material_shadow())
            })
            .when(!theme.is_frosted(), |menu| menu.shadow_lg())
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
                    "external",
                    true,
                    Some(Command::PrepareXAgain(post_id)),
                    cx,
                ));
            }
            if has_telegram_url {
                let link = post.telegram_message_link.clone();
                menu = menu.child(
                    add_item(
                        self,
                        "menu-open-tg",
                        "Open Telegram post",
                        "external",
                        true,
                        None,
                        cx,
                    )
                    .on_click(cx.listener(move |app, _event, _window, cx| {
                        let _ = std::process::Command::new("open").arg(&link).spawn();
                        app.history_more_menu_post = None;
                        cx.notify();
                    })),
                );
            }
            if has_x_url {
                let link = post.x_url.clone();
                menu = menu.child(
                    add_item(
                        self,
                        "menu-open-x",
                        "Open X post",
                        "external",
                        true,
                        None,
                        cx,
                    )
                    .on_click(cx.listener(move |app, _event, _window, cx| {
                        let _ = std::process::Command::new("open").arg(&link).spawn();
                        app.history_more_menu_post = None;
                        cx.notify();
                    })),
                );
            }
            let path = post
                .export_path
                .clone()
                .or_else(|| post.source_path.clone())
                .unwrap_or_default();
            menu = menu.child(
                add_item(
                    self,
                    "menu-reveal",
                    "Show video in folder",
                    "▤",
                    !path.is_empty(),
                    None,
                    cx,
                )
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    if !path.is_empty() {
                        let _ = cliprelay_core::x::XAssistant::reveal(std::path::Path::new(&path));
                    }
                    app.history_more_menu_post = None;
                    cx.notify();
                })),
            );
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

    pub fn open_selected_in_player(&mut self, cx: &mut Context<Self>) {
        if let Some(selected) = &self.selected {
            let path = PathBuf::from(&selected.path);
            self.open_media_in_default_player(path, cx);
        }
    }
}

impl crate::App {
    /// Width of the docked Prepare panel.
    pub fn prepare_dock_width(&self) -> f32 {
        let window = self.window_size.0;
        let sidebar = if self.sidebar_collapsed || window < 1080.0 {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_EXPANDED_WIDTH
        };
        let page_width = (window - sidebar).max(1.0);
        let explorer = if window >= 980.0 && self.explorer_visible() {
            EXPLORER_WIDTH
        } else {
            0.0
        };
        let min_grid = if window < 900.0 { 280.0 } else { 360.0 };
        let preferred = if window < 900.0 {
            (page_width * 0.42).clamp(280.0, 320.0)
        } else if window < 1120.0 {
            (page_width * 0.40).clamp(350.0, 410.0)
        } else if window < 1480.0 {
            (page_width * 0.34).clamp(420.0, 500.0)
        } else if window >= 1800.0 {
            (page_width * 0.32).clamp(560.0, 720.0)
        } else {
            (page_width * 0.345).clamp(520.0, 600.0)
        };
        let available = (page_width - explorer - min_grid).max(280.0);
        if self.prepare_expanded && window >= 1180.0 {
            available.min(680.0)
        } else {
            preferred.min(available).max(280.0)
        }
    }
}
