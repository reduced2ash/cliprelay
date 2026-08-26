//! Library page: folder explorer, video grid with tiles, hover previews,
//! and the random-source popup.

use crate::settings_import::*;
use crate::state::*;
use crate::theme::*;
use crate::video_element::video as video_element;
use crate::widgets::*;
use cliprelay_core::db::MediaRow;
use gpui::prelude::*;
use gpui::*;
use gpui_video_player::Video;
use std::path::PathBuf;

/// Layout helpers shared by the pages.
#[allow(dead_code)]
pub struct Layout {
    pub window: f32,
    pub sidebar_width: f32,
    pub page_width: f32,
    pub explorer_visible: bool,
    pub explorer_width: f32,
    pub grid_left: f32,
    pub grid_width: f32,
    pub density_compact: bool,
    pub tile_min: f32,
    pub tile_gap: f32,
    pub tile_chrome: f32,
    pub columns: usize,
    pub cell_width: f32,
    pub poster_height: f32,
    pub cell_height: f32,
}

impl Layout {
    pub fn compute(app: &crate::App, window: (f32, f32)) -> Self {
        let window_width = window.0;
        // Narrow windows auto-collapse the sidebar (mirrors the original's
        // `navCollapsed = narrowWindow || sidebar_collapsed`).
        let sidebar_collapsed = app.sidebar_collapsed || window_width < 1080.0;
        let sidebar_width = if sidebar_collapsed {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_EXPANDED_WIDTH
        };
        let page_width = (window_width - sidebar_width).max(1.0);
        let explorer_visible = app.page == Page::Library
            && !app.settings_value(LIBRARY_ROOT).is_empty()
            && app.explorer_visible();
        let explorer_width = if explorer_visible {
            EXPLORER_WIDTH
        } else {
            0.0
        };
        let grid_left = sidebar_width + explorer_width;
        let mut grid_width = (page_width - explorer_width).max(1.0);
        // The docked Prepare panel shares the page row; the scrollbar
        // reserves 10px of content width inside the grid column.
        if app.selected.is_some() && !app.prepare.studio_mode {
            grid_width = (grid_width - app.prepare_dock_width()).max(1.0);
        }
        // The original insets the grid 14px per side (libraryGridInset);
        // the scrollbar reserves 10px inside the column.
        grid_width = (grid_width - 28.0 - 10.0).max(1.0);
        let density_compact = app.density == "compact";
        let tile_min = if density_compact {
            TILE_MIN_COMPACT
        } else {
            TILE_MIN_DEFAULT
        };
        let tile_gap = if density_compact {
            TILE_GAP_COMPACT
        } else {
            TILE_GAP_DEFAULT
        };
        let tile_chrome = if density_compact {
            TILE_CHROME_COMPACT
        } else {
            TILE_CHROME_DEFAULT
        };
        let columns = ((grid_width + tile_gap) / (tile_min + tile_gap))
            .floor()
            .max(1.0) as usize;
        let cell_width = grid_width / columns as f32;
        // The original floors the poster at 96px and rounds to a pixel.
        let poster_height = ((cell_width - tile_gap) * 9.0 / 16.0).round().max(96.0);
        let cell_height = poster_height + tile_chrome + tile_gap;
        Self {
            window: window_width,
            sidebar_width,
            page_width,
            explorer_visible,
            explorer_width,
            grid_left,
            grid_width,
            density_compact,
            tile_min,
            tile_gap,
            tile_chrome,
            columns,
            cell_width,
            poster_height,
            cell_height,
        }
    }
}

impl crate::App {
    pub fn settings_value(&self, key: &str) -> String {
        self.settings
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn settings_bool(&self, key: &str) -> bool {
        self.settings
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn explorer_visible(&self) -> bool {
        self.show_folders
            && !(self.selected.is_some() && !self.prepare.studio_mode && self.window_size.0 < 980.0)
    }

    pub fn render_library(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        crate::widgets::set_current_theme(&theme);
        let layout = Layout::compute(self, self.window_size);
        let selected_id = self.selected.as_ref().map(|m| m.id).unwrap_or(0);
        let scanning = self.scan.active;
        let search = self.search_text.clone();

        let mut column = div()
            .id("library")
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_row()
            .bg(theme.workbench_canvas)
            .size_full();

        // Explorer column.
        if layout.explorer_visible {
            column = column.child(self.render_explorer(cx, &theme, &layout));
        }

        // Grid column.
        let mut grid = div()
            .id("library-grid")
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .relative();

        // Scan banner.
        if scanning {
            grid = grid.child(
                div()
                    .w_full()
                    .px(px(20.0))
                    .pb(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(progress_bar(self.scan.progress, self.scan.progress < 0.0))
                    .child(
                        div()
                            .child(self.scan.message.clone())
                            .text_size(px(12.0))
                            .text_color(theme.muted),
                    ),
            );
        }

        let row_count = self.library.rows.len();
        let has_more = self.library.has_more;
        let columns = layout.columns;
        let cell_width = layout.cell_width;
        let poster_height = layout.poster_height;
        let tile_chrome = layout.tile_chrome;
        let tile_gap = layout.tile_gap;
        let cell_height = layout.cell_height;
        let density = self.density.clone();
        if row_count == 0 && !scanning {
            grid = grid.child(self.render_empty_state(cx, &theme));
        } else {
            // One-shot reveal: scroll the selected tile into view, loading
            // further pages until the page holding the tile is available.
            if let Some((_folder, media_index, _folder_index)) = self.reveal_request.take() {
                if media_index >= 0 {
                    self.reveal_target_row = Some(media_index as usize);
                    if media_index as usize >= row_count && has_more {
                        let controller = self.controller.clone();
                        let _ = controller.send(Command::LoadMoreLibrary);
                    }
                }
            }
            self.apply_reveal_scroll();
            let grid_rows = library_grid_row_count(row_count, columns);
            let list = uniform_list(
                "tiles",
                grid_rows,
                cx.processor(
                    move |app, visible_grid_rows: std::ops::Range<usize>, _window, cx| {
                        let active_preview = app.active_preview_id;
                        let preview_video =
                            app.preview_video.as_ref().and_then(|(media_id, video)| {
                                (*media_id == active_preview).then(|| video.clone())
                            });
                        let mut rendered_rows = Vec::with_capacity(visible_grid_rows.len());
                        for grid_row_index in visible_grid_rows {
                            let start_index = grid_row_index.saturating_mul(columns);
                            let end_index = (start_index + columns).min(app.library.rows.len());
                            let visible_rows = app.library.rows[start_index..end_index].to_vec();
                            let mut tile_row = div()
                                .id(SharedString::from(format!("tile-row-{grid_row_index}")))
                                .w_full()
                                .h(px(cell_height))
                                .flex()
                                .flex_row()
                                .gap(px(tile_gap));

                            for (column_index, row) in visible_rows.iter().enumerate() {
                                let media_id = row.id;
                                if row.thumbnail_path.is_none()
                                    && app.thumbnail_states.get(&media_id).map(String::as_str)
                                        != Some("failed")
                                    && !app.thumbnail_requested.contains(&media_id)
                                {
                                    app.thumbnail_requested.insert(media_id);
                                    app.thumbnail_states.insert(media_id, "queued".into());
                                    let _ = app.controller.send(Command::EnsureThumbnail(media_id));
                                }
                                let is_hovered = app.hovered_tiles.get(&media_id) == Some(&true);
                                let is_selected = media_id == selected_id;
                                let is_active_preview = media_id == active_preview && is_hovered;
                                let state =
                                    app.thumbnail_states.get(&media_id).cloned().unwrap_or_else(
                                        || {
                                            if row.thumbnail_path.is_some() {
                                                "ready".into()
                                            } else {
                                                "idle".into()
                                            }
                                        },
                                    );
                                let tile_preview =
                                    is_active_preview.then(|| preview_video.clone()).flatten();
                                tile_row = tile_row.child(app.render_tile(
                                    cx,
                                    &theme,
                                    row,
                                    start_index + column_index,
                                    cell_width,
                                    tile_gap,
                                    poster_height,
                                    tile_chrome,
                                    is_selected,
                                    is_hovered,
                                    is_active_preview,
                                    &state,
                                    tile_preview,
                                    &density,
                                ));
                            }
                            rendered_rows.push(tile_row);
                        }
                        rendered_rows
                    },
                ),
            )
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .px(px(14.0))
            .pt(px(2.0))
            .pb(px(8.0))
            .track_scroll(self.library_scroll.clone())
            .on_scroll_wheel(cx.listener(|app, _event, _window, cx| {
                app.mark_library_scrolled(cx);
                app.hovered_tiles.clear();
                app.load_more_library_if_near_end();
                cx.notify();
            }));
            grid = grid.child(list);
        }

        column = column.child(grid);
        let _ = search;
        column
    }

    fn render_empty_state(
        &self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
    ) -> impl Element {
        let has_root = !self.settings_value(LIBRARY_ROOT).is_empty();
        let searching = !self.search_text.is_empty();
        let (title, body) = if !has_root {
            (
                "Bring your video folders together".to_string(),
                "Choose one top-level folder. ClipRelay finds videos inside every nested folder without moving your originals.".to_string(),
            )
        } else if searching {
            (
                "No matching videos".to_string(),
                "Try a broader search or switch folders.".to_string(),
            )
        } else {
            let body = if self.settings_bool(AUTO_INDEX) {
                "Rescan the library, or enable deep format detection for uncommon files."
            } else {
                "Background indexing is off. Pick randomly now, or rescan to build the visual library."
            };
            ("No videos found".to_string(), body.to_string())
        };
        let mut empty = div()
            .id("library-empty")
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(11.0))
            .child(
                div()
                    .w(px(58.0))
                    .h(px(58.0))
                    .rounded(px(16.0))
                    .bg(theme.active)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("▤", 27.0, theme.accent)),
            )
            .child(
                div()
                    .child(title)
                    .text_size(px(20.0))
                    .text_color(theme.text)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                div()
                    .child(body)
                    .text_size(px(13.0))
                    .text_color(theme.muted)
                    .max_w(px(440.0))
                    .text_align(TextAlign::Center),
            );
        if !has_root {
            empty = empty.child(button(
                "choose-root-empty",
                "Choose video library",
                ButtonKind::Primary,
                Some("▸"),
                true,
                cx,
                |app, cx| {
                    app.choose_library_folder(cx);
                },
            ));
        }
        empty
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tile(
        &self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        row: &MediaRow,
        _index: usize,
        cell_width: f32,
        tile_gap: f32,
        poster_height: f32,
        tile_chrome: f32,
        is_selected: bool,
        is_hovered: bool,
        is_active_preview: bool,
        thumbnail_state: &str,
        preview_video: Option<Video>,
        density: &str,
    ) -> impl Element {
        let media_id = row.id;
        let name = row.name.clone();
        let duration_label = if row.duration <= 0.0 {
            "Unchecked".to_string()
        } else {
            cliprelay_core::utils::format_duration(row.duration)
        };
        let size_label = cliprelay_core::utils::format_bytes(row.size_bytes as f64);
        let resolution = if row.width > 0 && row.height > 0 {
            format!("{}×{}", row.width, row.height)
        } else {
            "Unchecked".to_string()
        };
        let folder = row.folder.clone();
        let metadata = vec![size_label, resolution, folder]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("  ·  ");
        let compact = density == "compact";
        let thumbnail = row.thumbnail_path.clone().unwrap_or_default();
        let thumbnail_ok = !thumbnail.is_empty() && thumbnail_state != "failed";
        let media_path = row.path.clone();
        let tooltip_text: SharedString = if row.folder.is_empty() {
            row.name.clone().into()
        } else {
            format!("{}/{}", row.folder, row.name).into()
        };
        // The original shows the tooltip only when the name or metadata is
        // truncated; approximate truncation by the name's estimated width at
        // the 11px label size against the tile width.
        let tile_width = (cell_width - tile_gap).max(1.0);
        let would_truncate = (row.name.len() as f32) * 6.4 > tile_width - 14.0;

        let mut tile = div()
            .id(SharedString::from(format!("tile-{media_id}")))
            .w(px((cell_width - tile_gap).max(1.0)))
            .flex_none()
            .cursor_pointer()
            .flex()
            .flex_col()
            .tab_index(0)
            .when(is_selected, |tile| {
                tile.track_focus(&self.library_item_focus)
            })
            .focus(|style| style.border_2().border_color(theme.accent))
            .on_click(cx.listener(move |app, _event, _window, cx| {
                app.explorer_focus = false;
                app.command(Command::SelectMedia(media_id));
                cx.notify();
            }))
            .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                app.explorer_focus = false;
                app.command(Command::SelectMedia(media_id));
                cx.stop_propagation();
            }))
            .on_action(
                cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                    app.explorer_focus = false;
                    app.command(Command::SelectMedia(media_id));
                    cx.stop_propagation();
                }),
            )
            .on_hover(cx.listener(move |app, &hovered, _window, cx| {
                // Suppress hover churn during scroll: the pointer may appear
                // to sweep many tiles as the content moves under it, and each
                // hover would otherwise restart the preview pipeline.
                if std::time::Instant::now() < app.library_scroll_pause_until {
                    return;
                }
                if app.hovered_tiles.get(&media_id) != Some(&hovered) {
                    if hovered {
                        app.hovered_tiles.insert(media_id, true);
                        app.start_preview_timer(media_id, cx);
                    } else {
                        app.hovered_tiles.remove(&media_id);
                        app.stop_hover_preview(Some(media_id), cx);
                    }
                    cx.notify();
                }
            }))
            .when(would_truncate, |this| {
                this.tooltip(move |_window, cx| crate::tooltip_view(cx, tooltip_text.clone()))
            });

        // Poster.
        let mut poster = div()
            .w_full()
            .h(px(poster_height))
            .rounded(px(TILE_RADIUS))
            .bg(theme.ink)
            .overflow_hidden()
            .relative()
            .border_1()
            .border_color(if is_selected {
                theme.accent
            } else if is_hovered {
                theme.border_strong
            } else {
                theme.border
            })
            .when(is_hovered && !is_selected, |this| {
                this.border_color(theme.accent.opacity(0.55))
            });
        if is_active_preview {
            if let Some(video) = preview_video {
                poster = poster.child(video_element(
                    video,
                    SharedString::from(format!("hover-video-{media_id}")),
                    px(tile_width),
                    px(poster_height),
                ));
            } else if thumbnail_ok {
                // Keep the thumbnail stable while the debounced preview encode
                // and off-thread GStreamer startup complete.
                poster = poster.child(
                    img(PathBuf::from(thumbnail))
                        .w_full()
                        .h_full()
                        .object_fit(ObjectFit::Contain),
                );
            } else {
                poster = poster.child(self.poster_fallback(theme));
                if thumbnail_state == "failed" {
                    poster =
                        poster.child(div().absolute().top(px(8.0)).right(px(8.0)).child(icon(
                            "⚠",
                            22.0,
                            theme.warning,
                        )));
                }
            }
        } else if thumbnail_ok {
            poster = poster.child(
                img(PathBuf::from(thumbnail))
                    .w_full()
                    .h_full()
                    .object_fit(ObjectFit::Contain),
            );
        } else {
            poster = poster.child(self.poster_fallback(theme));
            if thumbnail_state == "failed" {
                poster = poster.child(div().absolute().top(px(8.0)).right(px(8.0)).child(icon(
                    "⚠",
                    22.0,
                    theme.warning,
                )));
            }
        }
        // Duration badge.
        let badge_margin = if compact { 5.0 } else { 7.0 };
        let badge_h = if compact { 18.0 } else { 20.0 };
        if row.duration > 0.0 {
            poster = poster.child(tabular(
                div()
                    .absolute()
                    .bottom(px(badge_margin))
                    .right(px(badge_margin))
                    .h(px(badge_h))
                    .px(px(if compact { 5.0 } else { 6.0 }))
                    .rounded(px(4.0))
                    .bg(theme.media_overlay)
                    .child(duration_label)
                    .text_size(px(if compact { 10.0 } else { 12.0 }))
                    .text_color(theme.media_text),
            ));
        }
        // Selection check.
        if is_selected {
            poster = poster.child(
                div()
                    .absolute()
                    .top(px(badge_margin))
                    .right(px(badge_margin))
                    .w(px(if compact { 20.0 } else { 22.0 }))
                    .h(px(if compact { 20.0 } else { 22.0 }))
                    .rounded(px(4.0))
                    .bg(theme.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("✓", 13.0, theme.accent_content)),
            );
        }
        tile = tile.child(poster);

        // Metadata chrome.
        let unchecked = row.duration <= 0.0;
        let posted = row.posted_count;
        let mut meta_row = div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(5.0))
            .child(if unchecked {
                icon("⚡", 12.0, theme.muted_soft)
            } else {
                div()
            })
            .child(tabular(
                div()
                    .flex_1()
                    .child(metadata)
                    .text_size(px(if compact { 11.0 } else { 12.0 }))
                    .text_color(theme.muted)
                    .text_ellipsis(),
            ));
        if posted > 0 {
            meta_row = meta_row.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(2.0))
                    .child(icon("✓", 10.0, theme.success))
                    .child(
                        div()
                            .child(crate::prepare_render::group_digits(posted.max(0) as usize))
                            .text_size(px(if compact { 11.0 } else { 12.0 }))
                            .text_color(theme.success),
                    ),
            );
        }
        tile = tile.child(
            div()
                .w_full()
                .pt(px(7.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .w_full()
                        .child(name)
                        .text_size(px(if compact { 12.0 } else { 13.0 }))
                        .text_color(if is_selected {
                            theme.text
                        } else {
                            theme.text_soft
                        })
                        .font_weight(if is_selected {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_ellipsis(),
                )
                .child(meta_row),
        );
        let _ = (tile_chrome, media_path);
        tile
    }

    fn poster_fallback(&self, theme: &crate::theme::Theme) -> impl Element {
        div()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .child(icon("▷", 26.0, theme.border_strong))
    }

    /// Folders visible in the explorer tree (ancestors all expanded).
    pub fn visible_folders(&self) -> Vec<FolderNode> {
        let nodes: Vec<FolderNode> = self.folders.clone();
        let expanded_map = self.folders_expanded.clone();
        let is_visible = |node: &FolderNode| -> bool {
            let mut ancestor = node.parent.clone();
            while !ancestor.is_empty() {
                if !expanded_map.get(&ancestor).copied().unwrap_or(true) {
                    return false;
                }
                ancestor = ancestor
                    .rsplit_once('/')
                    .map(|(p, _)| p.to_string())
                    .unwrap_or_default();
            }
            true
        };
        nodes
            .iter()
            .filter(|node| is_visible(node))
            .cloned()
            .collect()
    }

    /// Move the explorer keyboard selection by `delta` visible rows.
    pub fn explorer_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let rows = self.visible_folders();
        let target = match rows.iter().position(|n| n.folder == self.explorer_selected) {
            Some(index) => (index as isize + delta).clamp(0, rows.len() as isize) as usize,
            None => 0,
        };
        if target < rows.len() {
            let folder = rows[target].folder.clone();
            self.explorer_selected = folder.clone();
            self.command(Command::SetFolder(folder));
        } else if delta > 0 && !self.active_folder.is_empty() {
            // Below the last folder: back to the root ("All videos").
            self.explorer_selected = String::new();
            self.command(Command::SetFolder(String::new()));
        }
        cx.notify();
    }

    /// Explorer keyboard actions (left/right/space/enter) for the focused row.
    pub fn explorer_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(node) = self
            .visible_folders()
            .into_iter()
            .find(|n| n.folder == self.explorer_selected)
        else {
            if key == " " || key == "enter" {
                self.command(Command::SetFolder(String::new()));
                cx.notify();
                return true;
            }
            return false;
        };
        match key {
            "left" => {
                if node.has_children
                    && self
                        .folders_expanded
                        .get(&node.folder)
                        .copied()
                        .unwrap_or(true)
                {
                    self.folders_expanded.insert(node.folder.clone(), false);
                } else if !node.parent.is_empty() {
                    self.explorer_selected = node.parent.clone();
                }
                cx.notify();
                true
            }
            "right" => {
                if node.has_children {
                    let expanded = self
                        .folders_expanded
                        .get(&node.folder)
                        .copied()
                        .unwrap_or(true);
                    if !expanded {
                        self.folders_expanded.insert(node.folder.clone(), true);
                    } else if let Some(first) = self
                        .visible_folders()
                        .into_iter()
                        .find(|n| n.parent == node.folder)
                    {
                        let folder = first.folder.clone();
                        self.explorer_selected = folder.clone();
                        self.command(Command::SetFolder(folder));
                    }
                } else {
                    self.explorer_move(1, cx);
                }
                cx.notify();
                true
            }
            " " | "enter" => {
                let folder = node.folder.clone();
                self.explorer_selected = folder.clone();
                self.command(Command::SetFolder(folder));
                cx.notify();
                true
            }
            _ => false,
        }
    }

    fn render_explorer(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        layout: &Layout,
    ) -> impl Element {
        let _ = layout;
        // Keep every Explorer entry on the same four-column rhythm:
        // disclosure, folder, name, count. Fixed rows are important here —
        // allowing the flex column to shrink them is what made large folder
        // trees look compressed despite generous nominal dimensions.
        let row_inset = 12.0;
        let row_height = 40.0;
        let row_gap = 7.0;
        let disclosure_size = 20.0;
        let folder_icon_size = 19.5;
        let count_width = 44.0;
        let mut tree = div()
            .id("explorer")
            .w(px(EXPLORER_WIDTH))
            .flex_none()
            .bg(theme.workbench_explorer)
            .border_l_1()
            .border_r_1()
            .border_color(theme.workbench_border.opacity(0.62))
            .flex()
            .flex_col();
        // The global context toolbar owns the Explorer heading. Starting the
        // tree immediately avoids the duplicate header band that made this
        // column feel heavier than VS Code/Zed-style workbench navigation.
        let mut items = div()
            .id("explorer-tree")
            .flex_1()
            .flex()
            .flex_col()
            .pt(px(0.0))
            .relative()
            .top(px(-2.0))
            .overflow_scroll();
        // Root row.
        let root_active = self.active_folder.is_empty();
        let root_expanded = self
            .folders_expanded
            .get("__explorer_root__")
            .copied()
            .unwrap_or(true);
        items = items.child(
            div()
                .id("folder-root")
                .ml(px(row_inset))
                .h(px(row_height))
                .flex_none()
                .pr(px(14.0))
                .rounded_tl(px(4.0))
                .rounded_bl(px(4.0))
                .flex()
                .items_center()
                .gap(px(row_gap))
                .cursor_pointer()
                .tab_index(0)
                .focus(|style| style.bg(theme.workbench_selection.opacity(0.86)))
                .active(|style| style.bg(theme.accent_soft))
                .hover(|style| style.bg(theme.workbench_selection.opacity(0.68)))
                .bg(theme.transparent())
                .child(
                    div()
                        .id("folder-root-chevron")
                        .occlude()
                        .w(px(disclosure_size))
                        .h(px(disclosure_size))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            icon(
                                if root_expanded {
                                    "chevron-down"
                                } else {
                                    "chevron-right"
                                },
                                13.5,
                                theme.muted_soft,
                            )
                            .relative()
                            .left(px(-1.0)),
                        )
                        .on_click(cx.listener(|app, _event, _window, cx| {
                            let expanded = app
                                .folders_expanded
                                .get("__explorer_root__")
                                .copied()
                                .unwrap_or(true);
                            app.folders_expanded
                                .insert("__explorer_root__".to_string(), !expanded);
                            cx.notify();
                        })),
                )
                .child(
                    icon(
                        "folder",
                        folder_icon_size,
                        if root_active {
                            theme.accent_text
                        } else {
                            theme.muted
                        },
                    )
                    .relative()
                    .top(px(-1.0)),
                )
                .child(
                    div()
                        .child("All videos")
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .w(px(count_width))
                        .flex_none()
                        .child(format!("{}", self.counts.0))
                        .text_size(px(12.0))
                        .text_color(theme.muted_soft)
                        .font_weight(FontWeight::NORMAL)
                        .text_right(),
                )
                .on_click(cx.listener(|app, _event, window, cx| {
                    window.blur();
                    app.explorer_focus = true;
                    app.explorer_selected = String::new();
                    app.command(Command::SetFolder(String::new()));
                    cx.notify();
                }))
                .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
                    app.explorer_focus = true;
                    app.explorer_selected = String::new();
                    app.command(Command::SetFolder(String::new()));
                    cx.notify();
                    cx.stop_propagation();
                }))
                .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
                    app.explorer_focus = true;
                    app.explorer_selected = String::new();
                    app.command(Command::SetFolder(String::new()));
                    cx.notify();
                    cx.stop_propagation();
                })),
        );
        let active_folder = self.active_folder.clone();
        let expanded_map = self.folders_expanded.clone();
        let visible = self.visible_folders();
        if root_expanded {
            for node in visible {
                let folder = node.folder.clone();
                let is_active = active_folder == folder;
                let is_focused = self.explorer_selected == folder;
                let has_children = node.has_children;
                let expanded = expanded_map.get(&folder).copied().unwrap_or(node.depth < 1);
                let count = node.count;
                let name = node.name.clone();
                let indent = (8.0 + node.depth as f32 * 14.0).min(8.0 + 6.0 * 14.0);
                let mut row = div()
                    .id(SharedString::from(format!("folder-{folder}")))
                    .ml(px(row_inset))
                    .h(px(row_height))
                    .flex_none()
                    .pl(px(indent))
                    .pr(px(14.0))
                    .rounded_tl(px(4.0))
                    .rounded_bl(px(4.0))
                    .flex()
                    .items_center()
                    .gap(px(row_gap))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.workbench_selection.opacity(0.68)))
                    .bg(if is_active || is_focused {
                        theme.workbench_selection.opacity(0.86)
                    } else {
                        theme.transparent()
                    });
                // Disclosure chevron (own hit target; toggles expansion).
                if has_children {
                    let folder_for_toggle = folder.clone();
                    row = row.child(
                        div()
                            .id(SharedString::from(format!("folder-chevron-{folder}")))
                            .occlude()
                            .w(px(disclosure_size))
                            .h(px(disclosure_size))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                icon(
                                    if expanded {
                                        "chevron-down"
                                    } else {
                                        "chevron-right"
                                    },
                                    13.5,
                                    theme.muted_soft,
                                )
                                .relative()
                                .left(px(-1.0)),
                            )
                            .on_click(cx.listener(move |app, _event, _window, cx| {
                                let current = app
                                    .folders_expanded
                                    .get(&folder_for_toggle)
                                    .copied()
                                    .unwrap_or(true);
                                app.folders_expanded
                                    .insert(folder_for_toggle.clone(), !current);
                                cx.notify();
                            })),
                    );
                } else {
                    row = row.child(div().w(px(disclosure_size)).flex_none());
                }
                // Row body: select the folder.
                row = row
                    .child(
                        icon(
                            "folder",
                            folder_icon_size,
                            if is_active {
                                theme.accent_text
                            } else {
                                theme.muted
                            },
                        )
                        .relative()
                        .top(px(-1.0)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .child(name)
                            .text_size(px(13.5))
                            .text_color(if is_active {
                                theme.text
                            } else {
                                theme.text_soft
                            })
                            .font_weight(if is_active {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_ellipsis(),
                    )
                    .child(
                        div()
                            .w(px(count_width))
                            .flex_none()
                            .child(format!("{count}"))
                            .text_size(px(12.0))
                            .text_color(if is_active {
                                theme.text_soft
                            } else {
                                theme.muted_soft
                            })
                            .font_weight(FontWeight::NORMAL)
                            .text_right(),
                    )
                    .on_click(cx.listener(move |app, _event, _window, cx| {
                        app.explorer_focus = true;
                        app.explorer_selected = folder.clone();
                        app.command(Command::SetFolder(folder.clone()));
                        cx.notify();
                    }));
                items = items.child(row);
            }
        }
        let scanning = self.scan.active;
        let explorer_actions = div()
            .w_full()
            .h(px(52.0))
            .flex_none()
            .px(px(18.0))
            .border_t_1()
            .border_color(theme.workbench_border.opacity(0.46))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(workbench_button(
                "explorer-new-workspace",
                "",
                "plus",
                ButtonKind::Ghost,
                true,
                true,
                "Open a folder in a new workspace",
                cx,
                |app, cx| app.choose_new_workspace_folder(cx),
            ))
            .child(workbench_button(
                "explorer-workspace-actions",
                "",
                "ellipsis",
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
            ))
            .child(div().flex_1())
            .child(workbench_button(
                "explorer-rescan",
                "",
                if scanning { "square" } else { "refresh" },
                ButtonKind::Ghost,
                true,
                true,
                if scanning {
                    "Stop scan"
                } else {
                    "Rescan library"
                },
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
        tree = tree.child(items).child(explorer_actions);
        tree
    }
}

impl crate::App {
    pub fn library_columns(&self) -> usize {
        crate::library::Layout::compute(self, self.window_size).columns
    }

    pub fn library_cell_height(&self) -> f32 {
        crate::library::Layout::compute(self, self.window_size).cell_height
    }

    /// Scroll the grid so the pending reveal target is visible (no-op when
    /// the page holding it is not loaded yet). Contains-style: only scrolls
    /// when the row is outside the viewport.
    pub fn apply_reveal_scroll(&mut self) {
        let columns = self.library_columns();
        let Some(row_index) = take_loaded_reveal_grid_row(
            &mut self.reveal_target_row,
            self.library.rows.len(),
            columns,
        ) else {
            return;
        };
        self.library_scroll
            .scroll_to_item(row_index, gpui::ScrollStrategy::Top);
    }

    pub fn reset_library_scroll(&self) {
        self.library_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(0.0)));
    }

    fn load_more_library_if_near_end(&self) {
        let base_handle = self.library_scroll.0.borrow().base_handle.clone();
        if library_should_prefetch(
            base_handle.offset().y,
            base_handle.max_offset().height,
            px(self.library_cell_height() * 5.0),
            self.library.has_more,
        ) {
            let _ = self.controller.send(Command::LoadMoreLibrary);
        }
    }

    /// Toggle the random-source popup (with the original's 180ms guard).
    pub fn toggle_random_popup(&mut self, cx: &mut Context<Self>) {
        if self.random_popup_open {
            self.random_popup_open = false;
            self.mark_menu_closed();
        } else if self.menu_reopen_allowed() {
            self.random_popup_open = true;
            self.random_loading = true;
            self.command(Command::LoadRandomFolderOptions);
        }
        cx.notify();
    }

    /// Request a real video preview for a hovered tile (debounced).
    /// The actual `EnsurePreview` is sent only after `PREVIEW_DELAY_MS`
    /// and only if the same tile is still hovered and no scroll has
    /// occurred — this prevents the lag when the user scrolls with the
    /// pointer resting over the grid (hover events fire rapidly during
    /// scroll and would otherwise spam ffmpeg/GStreamer work).
    pub fn start_preview_timer(&mut self, media_id: i64, cx: &mut Context<Self>) {
        if !self
            .settings
            .get(HOVER_PREVIEWS)
            .and_then(|value| value.as_bool())
            .unwrap_or(true)
        {
            return;
        }
        if std::time::Instant::now() < self.library_scroll_pause_until {
            return;
        }
        if self.active_preview_id != 0 && self.active_preview_id != media_id {
            self.stop_hover_preview(None, cx);
        }
        self.preview_hover_generation = self.preview_hover_generation.wrapping_add(1);
        let generation = self.preview_hover_generation;
        self.active_preview_id = media_id;
        let tx = self.event_tx.clone();
        cx.spawn(
            move |_this: WeakEntity<crate::App>, _cx: &mut AsyncApp| async move {
                smol::Timer::after(std::time::Duration::from_millis(PREVIEW_DELAY_MS)).await;
                let _ = tx.send(Event::HoverCheck(media_id, generation));
            },
        )
        .detach();
    }

    pub fn mark_library_scrolled(&mut self, cx: &mut Context<Self>) {
        self.library_scroll_pause_until =
            std::time::Instant::now() + std::time::Duration::from_millis(350);
        self.stop_hover_preview(None, cx);
    }
}

fn library_grid_row_count(item_count: usize, columns: usize) -> usize {
    item_count.div_ceil(columns.max(1))
}

fn take_loaded_reveal_grid_row(
    target: &mut Option<usize>,
    loaded_items: usize,
    columns: usize,
) -> Option<usize> {
    let media_index = (*target)?;
    if media_index >= loaded_items {
        return None;
    }
    target.take();
    Some(media_index / columns.max(1))
}

fn library_should_prefetch(
    offset_y: Pixels,
    max_offset_y: Pixels,
    lead: Pixels,
    has_more: bool,
) -> bool {
    has_more && max_offset_y + offset_y <= lead.max(px(0.0))
}

#[cfg(test)]
mod grid_tests {
    use super::{library_grid_row_count, library_should_prefetch, take_loaded_reveal_grid_row};
    use gpui::px;

    #[test]
    fn columns_fit_within_content_width() {
        // The invariant: `columns` tiles plus `columns-1` gaps must fit the
        // grid content width (grid_width after the scrollbar reserve).
        use crate::theme::{TILE_GAP_COMPACT, TILE_GAP_DEFAULT};
        for grid in [400.0f32, 800.0, 1200.0, 1800.0] {
            for (min, gap) in [(236.0f32, TILE_GAP_DEFAULT), (176.0, TILE_GAP_COMPACT)] {
                let columns = ((grid + gap) / (min + gap)).floor().max(1.0) as usize;
                let cell_width = grid / columns as f32;
                let row_width = columns as f32 * (cell_width - gap) + (columns as f32 - 1.0) * gap;
                assert!(
                    row_width <= grid + 0.01,
                    "grid {grid} gap {gap}: {columns} tiles need {row_width} > {grid}"
                );
            }
        }
    }

    #[test]
    fn grid_rows_include_a_partial_final_row() {
        assert_eq!(library_grid_row_count(0, 4), 0);
        assert_eq!(library_grid_row_count(1, 4), 1);
        assert_eq!(library_grid_row_count(4, 4), 1);
        assert_eq!(library_grid_row_count(5, 4), 2);
    }

    #[test]
    fn reveal_scroll_is_consumed_once_after_target_loads() {
        let mut pending = Some(17);
        assert_eq!(take_loaded_reveal_grid_row(&mut pending, 12, 4), None);
        assert_eq!(pending, Some(17));

        assert_eq!(take_loaded_reveal_grid_row(&mut pending, 20, 4), Some(4));
        assert_eq!(pending, None);
        assert_eq!(take_loaded_reveal_grid_row(&mut pending, 20, 4), None);
    }

    #[test]
    fn prefetch_uses_negative_gpui_offsets_and_a_bounded_lead() {
        assert!(!library_should_prefetch(
            px(-200.0),
            px(1000.0),
            px(250.0),
            true
        ));
        assert!(library_should_prefetch(
            px(-760.0),
            px(1000.0),
            px(250.0),
            true
        ));
        assert!(library_should_prefetch(
            px(-1000.0),
            px(1000.0),
            px(250.0),
            true
        ));
        assert!(!library_should_prefetch(
            px(-1000.0),
            px(1000.0),
            px(250.0),
            false
        ));
    }
}
