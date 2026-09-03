//! Library page: folder explorer, video grid with tiles, hover previews,
//! and the random-source popup.

use crate::settings_import::*;
use crate::state::*;
use crate::theme::*;
use crate::video_element::{video as video_element, VideoFit};
use crate::widgets::*;
use cliprelay_core::db::MediaRow;
use gpui::prelude::*;
use gpui::*;
use gpui_video_player::Video;
use std::collections::HashMap;
use std::path::PathBuf;

const LIBRARY_DRAG_SLOP: f32 = 6.0;
const LIBRARY_DRAG_SCROLL_SPEED: f32 = 2.4;

// Explorer rows keep one four-column rhythm: disclosure, folder, name, count.
// The fixed height is load-bearing twice over: it stops the flex column from
// compressing large trees, and it is what lets `uniform_list` virtualize the
// tree so a scroll frame costs the visible rows instead of the whole library.
const EXPLORER_ROW_INSET: f32 = 12.0;
const EXPLORER_ROW_HEIGHT: f32 = 40.0;
const EXPLORER_ROW_GAP: f32 = 7.0;
const EXPLORER_DISCLOSURE_SIZE: f32 = 20.0;
const EXPLORER_FOLDER_ICON_SIZE: f32 = 19.5;
const EXPLORER_COUNT_WIDTH: f32 = 44.0;
const EXPLORER_ROOT_KEY: &str = "__explorer_root__";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LibraryDragButton {
    Primary,
    Secondary,
}

impl LibraryDragButton {
    fn from_mouse_button(button: MouseButton) -> Option<Self> {
        match button {
            MouseButton::Left => Some(Self::Primary),
            MouseButton::Right => Some(Self::Secondary),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LibraryDragUpdate {
    pub offset_y: f32,
    pub just_activated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LibraryDragRelease {
    None,
    ContextMenu(i64),
    ScrolledPrimary,
    ScrolledSecondary,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LibraryDragScroll {
    button: Option<LibraryDragButton>,
    start_x: f32,
    start_y: f32,
    last_x: f32,
    last_y: f32,
    start_offset_y: f32,
    media_id: Option<i64>,
    active: bool,
    suppress_primary_click: bool,
}

impl LibraryDragScroll {
    pub fn begin(
        &mut self,
        button: LibraryDragButton,
        position: (f32, f32),
        offset_y: f32,
        media_id: Option<i64>,
    ) {
        let same_press = self.button == Some(button)
            && (self.start_x - position.0).abs() < f32::EPSILON
            && (self.start_y - position.1).abs() < f32::EPSILON;
        if same_press {
            if media_id.is_some() {
                self.media_id = media_id;
            }
            return;
        }

        *self = Self {
            button: Some(button),
            start_x: position.0,
            start_y: position.1,
            last_x: position.0,
            last_y: position.1,
            start_offset_y: offset_y,
            media_id,
            active: false,
            suppress_primary_click: false,
        };
    }

    pub fn update(
        &mut self,
        button: LibraryDragButton,
        position: (f32, f32),
        max_offset_y: f32,
    ) -> Option<LibraryDragUpdate> {
        if self.button != Some(button)
            || (self.last_x - position.0).abs() < f32::EPSILON
                && (self.last_y - position.1).abs() < f32::EPSILON
        {
            return None;
        }
        self.last_x = position.0;
        self.last_y = position.1;

        let delta_x = position.0 - self.start_x;
        let delta_y = position.1 - self.start_y;
        let just_activated = !self.active
            && delta_x.mul_add(delta_x, delta_y * delta_y) > LIBRARY_DRAG_SLOP * LIBRARY_DRAG_SLOP;
        if just_activated {
            self.active = true;
        }
        if !self.active {
            return None;
        }

        Some(LibraryDragUpdate {
            offset_y: (self.start_offset_y + delta_y * LIBRARY_DRAG_SCROLL_SPEED)
                .clamp(-max_offset_y.max(0.0), 0.0),
            just_activated,
        })
    }

    pub fn finish(&mut self, button: LibraryDragButton) -> LibraryDragRelease {
        if self.button != Some(button) {
            return LibraryDragRelease::None;
        }

        let release = match (button, self.active, self.media_id) {
            (LibraryDragButton::Secondary, false, Some(media_id)) => {
                LibraryDragRelease::ContextMenu(media_id)
            }
            (LibraryDragButton::Primary, true, _) => {
                self.suppress_primary_click = true;
                LibraryDragRelease::ScrolledPrimary
            }
            (LibraryDragButton::Secondary, true, _) => LibraryDragRelease::ScrolledSecondary,
            _ => LibraryDragRelease::None,
        };
        self.button = None;
        self.media_id = None;
        self.active = false;
        release
    }

    pub fn cancel(&mut self) {
        self.button = None;
        self.media_id = None;
        self.active = false;
        self.suppress_primary_click = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn blocks_primary_click(&self) -> bool {
        self.active || self.suppress_primary_click
    }

    pub fn clear_primary_click_suppression(&mut self) {
        self.suppress_primary_click = false;
    }
}

fn library_thumbnail(
    path: PathBuf,
    width: f32,
    height: f32,
    fit_whole_frame: bool,
) -> impl Element {
    // This is a fixed 16:9 media well. Keep the source's intrinsic ratio out of
    // layout so Taffy cannot expand a portrait image beyond these bounds;
    // ObjectFit then applies that ratio only while painting inside the well.
    img(path)
        .w(px(width))
        .h(px(height))
        .flex_none()
        .infer_layout_aspect_ratio(false)
        .object_fit(if fit_whole_frame {
            ObjectFit::Contain
        } else {
            ObjectFit::Cover
        })
}

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
            && self.window_size.0 >= 820.0
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
            .bg(theme.canvas_background())
            .size_full();

        // Explorer column.
        if layout.explorer_visible {
            column = column.child(self.render_explorer(cx, &theme));
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
        let drag_cursor = if self.library_drag.is_active() {
            CursorStyle::ClosedHand
        } else {
            CursorStyle::OpenHand
        };
        if row_count == 0 && !scanning {
            grid = grid.child(self.render_empty_state(cx, &theme));
        } else {
            // One-shot reveal: scroll the selected tile into view, loading
            // further pages until the page holding the tile is available.
            if let Some((_, media_index)) = self
                .reveal_request
                .filter(|(generation, _)| *generation == self.library.generation)
            {
                self.reveal_request = None;
                if media_index >= 0 {
                    self.reveal_target_row = Some(media_index as usize);
                    if media_index as usize >= row_count && has_more {
                        let controller = self.controller.clone();
                        let _ = controller.send(Command::LoadMoreLibrary);
                    }
                }
            }
            self.apply_reveal_scroll(columns);
            let grid_rows = library_grid_row_count(row_count, columns);
            let list = uniform_list(
                "tiles",
                grid_rows,
                cx.processor(
                    move |app, visible_grid_rows: std::ops::Range<usize>, _window, cx| {
                        // Read framing from live App state whenever the
                        // virtualized list asks for visible rows. Capturing a
                        // copy when the list element is created can leave
                        // recycled rows on an earlier setting value.
                        let fit_thumbnails = app.settings_bool(FIT_LIBRARY_THUMBNAILS);
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
                                    fit_thumbnails,
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
            .cursor(drag_cursor)
            .track_scroll(self.library_scroll.clone())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|app, event: &MouseDownEvent, _window, _cx| {
                    app.begin_library_drag(event, None);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|app, event: &MouseDownEvent, _window, _cx| {
                    app.begin_library_drag(event, None);
                }),
            )
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
                    .child(if !has_root {
                        brand_mark(42.0).into_any()
                    } else {
                        icon("grid", 27.0, theme.accent).into_any()
                    }),
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
        } else if searching {
            empty = empty.child(button(
                "clear-library-search",
                "Clear search",
                ButtonKind::Secondary,
                Some("x"),
                true,
                cx,
                |app, cx| {
                    app.search_text.clear();
                    app.command_query.clear();
                    if let Some(field) = app.fields.get_mut("command-center") {
                        field.text.clear();
                        field.caret = 0;
                    }
                    app.command(Command::SetSearch(String::new()));
                    cx.notify();
                },
            ));
        } else {
            empty = empty.child(button(
                "rescan-empty-library",
                if self.scan.active {
                    "Scanning…"
                } else {
                    "Rescan workspace"
                },
                ButtonKind::Secondary,
                Some("refresh"),
                !self.scan.active,
                cx,
                |app, cx| {
                    app.command(Command::ScanLibrary);
                    cx.notify();
                },
            ));
        }
        empty
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tile(
        &mut self,
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
        fit_thumbnails: bool,
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
        let drag_scrolling = self.library_drag.is_active();

        let mut tile = div()
            .id(SharedString::from(format!("tile-{media_id}")))
            .w(px((cell_width - tile_gap).max(1.0)))
            .flex_none()
            .cursor(if drag_scrolling {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::PointingHand
            })
            .flex()
            .flex_col()
            .tab_index(0)
            .when(is_selected, |tile| {
                tile.track_focus(&self.library_item_focus)
            })
            .focus(|style| style.border_2().border_color(theme.accent))
            .on_click(cx.listener(move |app, event: &ClickEvent, _window, cx| {
                if !event.standard_click() {
                    return;
                }
                if !event.is_keyboard() && app.library_drag.blocks_primary_click() {
                    return;
                }
                app.explorer_focus = false;
                app.command(Command::SelectMedia(media_id));
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |app, event: &MouseDownEvent, _window, _cx| {
                    app.begin_library_drag(event, Some(media_id));
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |app, event: &MouseDownEvent, _window, _cx| {
                    app.begin_library_drag(event, Some(media_id));
                }),
            )
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
                    if fit_thumbnails {
                        VideoFit::Contain
                    } else {
                        VideoFit::Cover
                    },
                ));
            } else if thumbnail_ok {
                // Keep the thumbnail stable while the debounced preview encode
                // and off-thread GStreamer startup complete.
                poster = poster.child(library_thumbnail(
                    PathBuf::from(thumbnail),
                    tile_width,
                    poster_height,
                    fit_thumbnails,
                ));
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
            poster = poster.child(library_thumbnail(
                PathBuf::from(thumbnail),
                tile_width,
                poster_height,
                fit_thumbnails,
            ));
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
        if is_hovered || is_selected {
            let more_tooltip: SharedString = format!("More actions for {name}").into();
            poster = poster.child(
                div()
                    .id(SharedString::from(format!("tile-more-{media_id}")))
                    .occlude()
                    .absolute()
                    .top(px(badge_margin))
                    .left(px(badge_margin))
                    .w(px(if compact { 24.0 } else { 26.0 }))
                    .h(px(if compact { 24.0 } else { 26.0 }))
                    .rounded(px(RADIUS_SM))
                    .bg(theme.media_overlay)
                    .border_1()
                    .border_color(theme.media_text.opacity(0.24))
                    .cursor_pointer()
                    .tab_index(0)
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|style| style.border_color(theme.media_text.opacity(0.68)))
                    .focus(|style| style.border_2().border_color(theme.accent))
                    .child(icon("ellipsis", 14.0, theme.media_text))
                    .tooltip(move |_window, cx| crate::tooltip_view(cx, more_tooltip.clone()))
                    .on_click(cx.listener(move |app, _event, window, cx| {
                        app.open_video_context_menu(media_id, window, cx);
                        cx.stop_propagation();
                    }))
                    .on_action(cx.listener(move |app, _: &crate::Activate, window, cx| {
                        app.open_video_context_menu(media_id, window, cx);
                        cx.stop_propagation();
                    }))
                    .on_action(
                        cx.listener(move |app, _: &crate::ActivateSpace, window, cx| {
                            app.open_video_context_menu(media_id, window, cx);
                            cx.stop_propagation();
                        }),
                    ),
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
        let context_open = matches!(
            self.context_menu.as_ref(),
            Some(ContextMenuTarget::Video(row)) if row.id == media_id
        );
        let context_popup = context_open.then(|| self.render_context_menu(cx).into_any());
        anchored_overlay(
            tile,
            context_popup,
            OverlayPlacement::BelowStart,
            size(px(tile_width), px(poster_height + tile_chrome)),
        )
        .flex_none()
        .w(px(tile_width))
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

    /// Rebuild the Explorer's visible projection after folder data or an
    /// expansion state changes. Scrolling reads this cache instead of cloning
    /// and filtering the complete folder tree on every frame.
    pub fn rebuild_explorer_rows(&mut self) {
        let expanded_map = &self.folders_expanded;
        self.explorer_rows = explorer_visible_nodes(&self.folders, expanded_map);
    }

    /// Folders visible in the explorer tree (ancestors all expanded).
    pub fn visible_folders(&self) -> &[FolderNode] {
        &self.explorer_rows
    }

    /// Reveal visible-folder row `index`. The list is offset by one because
    /// row 0 is the library root.
    fn explorer_reveal_row(&self, index: usize) {
        self.explorer_scroll
            .scroll_to_item(index + 1, gpui::ScrollStrategy::Center);
    }

    /// Move the explorer keyboard selection by `delta` visible rows.
    pub fn explorer_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let row_count = self.explorer_rows.len();
        let target = match self
            .explorer_rows
            .iter()
            .position(|n| n.folder == self.explorer_selected)
        {
            Some(index) => (index as isize + delta).clamp(0, row_count as isize) as usize,
            None => 0,
        };
        if let Some(node) = self.explorer_rows.get(target) {
            let folder = node.folder.clone();
            self.explorer_selected = folder.clone();
            self.explorer_reveal_row(target);
            self.command(Command::SetFolder(folder));
        } else if delta > 0 && !self.library_location.folder.is_empty() {
            // Below the last folder: back to the root ("All videos").
            self.explorer_selected = String::new();
            self.explorer_scroll
                .scroll_to_item(0, gpui::ScrollStrategy::Center);
            self.command(Command::SetFolder(String::new()));
        }
        cx.notify();
    }

    /// Explorer keyboard actions (left/right/space/enter) for the focused row.
    pub fn explorer_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(node) = self
            .explorer_rows
            .iter()
            .find(|n| n.folder == self.explorer_selected)
            .cloned()
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
                    self.rebuild_explorer_rows();
                } else if !node.parent.is_empty() {
                    self.explorer_selected = node.parent.clone();
                    if let Some(index) = self
                        .explorer_rows
                        .iter()
                        .position(|row| row.folder == self.explorer_selected)
                    {
                        self.explorer_reveal_row(index);
                    }
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
                        self.rebuild_explorer_rows();
                    } else if let Some((index, folder)) = self
                        .explorer_rows
                        .iter()
                        .enumerate()
                        .find(|(_, row)| row.parent == node.folder)
                        .map(|(index, row)| (index, row.folder.clone()))
                    {
                        self.explorer_selected = folder.clone();
                        self.explorer_reveal_row(index);
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

    /// Folder whose media is currently previewed, when that media belongs to
    /// the active library root. Explorer indicators highlight that folder.
    fn explorer_preview_folder(&self, library_root: &std::path::Path) -> Option<String> {
        self.selected
            .as_ref()
            .filter(|media| {
                media.root_path.is_empty() || std::path::Path::new(&media.root_path) == library_root
            })
            .map(|media| media.folder.clone())
    }

    fn explorer_root_expanded(&self) -> bool {
        self.folders_expanded
            .get(EXPLORER_ROOT_KEY)
            .copied()
            .unwrap_or(true)
    }

    /// Explorer row 0: the library root, "All videos".
    fn render_explorer_root_row(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        library_root: &std::path::Path,
        preview_folder: Option<&str>,
    ) -> AnyElement {
        let indicators = self.library_location.folder_indicators("", preview_folder);
        let active = indicators.browsed;
        let expanded = self.explorer_root_expanded();
        let face: Background = if active {
            theme.selection_face(TactileState::Rest, false)
        } else {
            theme.transparent().into()
        };
        let context_path = library_root.to_path_buf();
        let row = div()
            .id("folder-root")
            .ml(px(EXPLORER_ROW_INSET))
            .h(px(EXPLORER_ROW_HEIGHT))
            .flex_none()
            .pr(px(14.0))
            .rounded_tl(px(4.0))
            .rounded_bl(px(4.0))
            .relative()
            .top(px(0.0))
            .border_1()
            .border_r_0()
            .border_color(if active {
                theme.tactile_edge(TactileState::Rest, false)
            } else {
                theme.transparent()
            })
            .bg(face)
            .shadow(if active {
                theme.tactile_shadow(TactileState::Rest, true)
            } else {
                Vec::new()
            })
            .flex()
            .items_center()
            .gap(px(EXPLORER_ROW_GAP))
            .cursor_pointer()
            .tab_index(0)
            .when(self.explorer_selected.is_empty(), |row| {
                row.track_focus(&self.explorer_item_focus)
            })
            .focus(|style| style.border_2().border_r_0().border_color(theme.accent))
            .active(|style| {
                style
                    .top(px(1.0))
                    .bg(theme.selection_face(TactileState::Pressed, false))
                    .border_color(theme.tactile_edge(TactileState::Pressed, false))
                    .shadow(theme.tactile_shadow(TactileState::Pressed, true))
            })
            .hover(|style| {
                style
                    .bg(theme.selection_face(TactileState::Hover, false))
                    .border_color(theme.tactile_edge(TactileState::Hover, false))
                    .shadow(theme.tactile_shadow(TactileState::Hover, true))
            })
            .child(
                div()
                    .id("folder-root-chevron")
                    .occlude()
                    .w(px(EXPLORER_DISCLOSURE_SIZE))
                    .h(px(EXPLORER_DISCLOSURE_SIZE))
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
                    .on_click(cx.listener(|app, _event, _window, cx| {
                        let expanded = app.explorer_root_expanded();
                        app.folders_expanded
                            .insert(EXPLORER_ROOT_KEY.to_string(), !expanded);
                        cx.notify();
                    })),
            )
            .child(
                icon(
                    "folder",
                    EXPLORER_FOLDER_ICON_SIZE,
                    if indicators.previewed {
                        theme.accent_text
                    } else if active {
                        theme.text
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
                    .text_color(if indicators.previewed {
                        theme.accent_text
                    } else {
                        theme.text
                    })
                    .font_weight(FontWeight::MEDIUM),
            )
            .child(div().flex_1())
            .child(
                div()
                    .w(px(EXPLORER_COUNT_WIDTH))
                    .flex_none()
                    .child(format!("{}", self.counts.0))
                    .text_size(px(12.0))
                    .text_color(theme.muted_soft)
                    .font_weight(FontWeight::NORMAL)
                    .text_right(),
            )
            .on_click(cx.listener(|app, event: &ClickEvent, window, cx| {
                if !event.standard_click() {
                    return;
                }
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
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |app, _event: &MouseDownEvent, window, cx| {
                    app.open_folder_context_menu(
                        String::new(),
                        context_path.clone(),
                        "Video library".to_string(),
                        window,
                        cx,
                    );
                }),
            );
        let context_open = matches!(
            self.context_menu.as_ref(),
            Some(ContextMenuTarget::Folder { relative_path, .. }) if relative_path.is_empty()
        );
        let context_popup = context_open.then(|| self.render_context_menu(cx).into_any());
        anchored_overlay(
            row,
            context_popup,
            OverlayPlacement::BelowStart,
            size(
                px(EXPLORER_WIDTH - EXPLORER_ROW_INSET),
                px(EXPLORER_ROW_HEIGHT),
            ),
        )
        .flex_none()
        .w_full()
        .h(px(EXPLORER_ROW_HEIGHT))
        .into_any_element()
    }

    /// Explorer row for `index` of the cached visible-folder projection.
    fn render_explorer_folder_row(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        library_root: &std::path::Path,
        preview_folder: Option<&str>,
        index: usize,
    ) -> AnyElement {
        let Some(node) = self.explorer_rows.get(index) else {
            return div()
                .w_full()
                .h(px(EXPLORER_ROW_HEIGHT))
                .flex_none()
                .into_any_element();
        };
        let folder = node.folder.clone();
        let name = node.name.clone();
        let count = node.count;
        let depth = node.depth;
        let has_children = node.has_children;
        let expanded = self
            .folders_expanded
            .get(&folder)
            .copied()
            .unwrap_or(depth < 1);
        let indicators = self
            .library_location
            .folder_indicators(&folder, preview_folder);
        let is_active = indicators.browsed;
        let is_previewed = indicators.previewed;
        let is_focused = self.explorer_selected == folder;
        let indent = (8.0 + depth as f32 * 14.0).min(8.0 + 6.0 * 14.0);
        // Keyboard focus and preview location never create another
        // persistent selected-row surface.
        let selected = is_active;
        let folder_path = library_root.join(&folder);
        let folder_tooltip: SharedString = folder_path.to_string_lossy().into_owned().into();
        let mut row = div()
            .id(SharedString::from(format!("folder-{folder}")))
            .ml(px(EXPLORER_ROW_INSET))
            .h(px(EXPLORER_ROW_HEIGHT))
            .flex_none()
            .pl(px(indent))
            .pr(px(14.0))
            .rounded_tl(px(4.0))
            .rounded_bl(px(4.0))
            .relative()
            .top(px(0.0))
            .border_1()
            .border_r_0()
            .border_color(if selected {
                theme.tactile_edge(TactileState::Rest, false)
            } else {
                theme.transparent()
            })
            .flex()
            .items_center()
            .gap(px(EXPLORER_ROW_GAP))
            .cursor_pointer()
            .tab_index(0)
            .when(is_focused, |row| row.track_focus(&self.explorer_item_focus))
            .focus(|style| style.border_2().border_r_0().border_color(theme.accent))
            .active(|style| {
                style
                    .top(px(1.0))
                    .bg(theme.selection_face(TactileState::Pressed, false))
                    .border_color(theme.tactile_edge(TactileState::Pressed, false))
                    .shadow(theme.tactile_shadow(TactileState::Pressed, true))
            })
            .hover(|style| {
                style
                    .bg(theme.selection_face(TactileState::Hover, false))
                    .border_color(theme.tactile_edge(TactileState::Hover, false))
                    .shadow(theme.tactile_shadow(TactileState::Hover, true))
            })
            .bg(if selected {
                theme.selection_face(TactileState::Rest, false)
            } else {
                theme.transparent().into()
            })
            .shadow(if selected {
                theme.tactile_shadow(TactileState::Rest, true)
            } else {
                Vec::new()
            })
            .tooltip(move |_window, cx| crate::tooltip_view(cx, folder_tooltip.clone()));
        // Disclosure chevron (own hit target; toggles expansion).
        if has_children {
            let folder_for_toggle = folder.clone();
            row = row.child(
                div()
                    .id(SharedString::from(format!("folder-chevron-{folder}")))
                    .occlude()
                    .w(px(EXPLORER_DISCLOSURE_SIZE))
                    .h(px(EXPLORER_DISCLOSURE_SIZE))
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
                        app.rebuild_explorer_rows();
                        cx.notify();
                    })),
            );
        } else {
            row = row.child(div().w(px(EXPLORER_DISCLOSURE_SIZE)).flex_none());
        }
        // Row body: select the folder.
        row = row
            .child(
                icon(
                    "folder",
                    EXPLORER_FOLDER_ICON_SIZE,
                    if is_previewed {
                        theme.accent_text
                    } else if is_active {
                        theme.text
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
                    .child(name.clone())
                    .text_size(px(13.5))
                    .text_color(if is_previewed {
                        theme.accent_text
                    } else if is_active {
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
                    .w(px(EXPLORER_COUNT_WIDTH))
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
            .on_click(cx.listener({
                let folder = folder.clone();
                move |app, event: &ClickEvent, _window, cx| {
                    if !event.standard_click() {
                        return;
                    }
                    app.explorer_focus = true;
                    app.explorer_selected = folder.clone();
                    app.command(Command::SetFolder(folder.clone()));
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener({
                    let folder = folder.clone();
                    move |app, _event: &MouseDownEvent, window, cx| {
                        app.open_folder_context_menu(
                            folder.clone(),
                            folder_path.clone(),
                            name.clone(),
                            window,
                            cx,
                        );
                    }
                }),
            );
        let context_open = matches!(
            self.context_menu.as_ref(),
            Some(ContextMenuTarget::Folder { relative_path, .. }) if relative_path == &folder
        );
        let context_popup = context_open.then(|| self.render_context_menu(cx).into_any());
        anchored_overlay(
            row,
            context_popup,
            OverlayPlacement::BelowStart,
            size(
                px(EXPLORER_WIDTH - EXPLORER_ROW_INSET),
                px(EXPLORER_ROW_HEIGHT),
            ),
        )
        .flex_none()
        .w_full()
        .h(px(EXPLORER_ROW_HEIGHT))
        .into_any_element()
    }

    fn render_explorer(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
    ) -> impl Element {
        let mut tree = div()
            .id("explorer")
            .w(px(EXPLORER_WIDTH))
            .flex_none()
            .bg(theme.explorer_background())
            .border_l_1()
            .border_r_1()
            .border_color(theme.workbench_border.opacity(0.62))
            .flex()
            .flex_col();
        // The global context toolbar owns the Explorer heading. Starting the
        // tree immediately avoids the duplicate header band that made this
        // column feel heavier than VS Code/Zed-style workbench navigation.
        //
        // Row 0 is the library root; the rest is the cached visible-folder
        // projection. Virtualizing here is what keeps a scroll frame
        // proportional to the viewport instead of the whole folder tree.
        let row_count = 1 + if self.explorer_root_expanded() {
            self.explorer_rows.len()
        } else {
            0
        };
        let mut items = uniform_list(
            "explorer-tree",
            row_count,
            cx.processor(|app, range: std::ops::Range<usize>, _window, cx| {
                // Read framing from live state whenever the list asks for
                // rows: capturing it when the element was created can leave a
                // recycled row on an earlier theme or library root.
                let theme = app.theme.clone();
                let library_root = PathBuf::from(app.settings_value(LIBRARY_ROOT));
                let preview_folder = app.explorer_preview_folder(&library_root);
                range
                    .map(|index| match index.checked_sub(1) {
                        None => app.render_explorer_root_row(
                            cx,
                            &theme,
                            &library_root,
                            preview_folder.as_deref(),
                        ),
                        Some(row) => app.render_explorer_folder_row(
                            cx,
                            &theme,
                            &library_root,
                            preview_folder.as_deref(),
                            row,
                        ),
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .flex_1()
        .min_h(px(0.0))
        .w_full()
        .relative()
        .top(px(-2.0))
        .track_scroll(self.explorer_scroll.clone());
        items.style().scrollbar_width = Some(px(10.0).into());

        tree = tree.child(items);
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
    pub fn apply_reveal_scroll(&mut self, columns: usize) {
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

    pub fn begin_library_drag(&mut self, event: &MouseDownEvent, media_id: Option<i64>) {
        let Some(button) = LibraryDragButton::from_mouse_button(event.button) else {
            return;
        };
        // An already-open item context menu closes on the new press; allow the
        // same gesture to select, drag, or open a different tile normally.
        let another_surface_owns_pointer =
            self.transient_surface_owns_global_shortcuts() && self.context_menu.is_none();
        if self.page != Page::Library || another_surface_owns_pointer {
            return;
        }
        let base_handle = self.library_scroll.0.borrow().base_handle.clone();
        self.library_drag.begin(
            button,
            (event.position.x.into(), event.position.y.into()),
            base_handle.offset().y.into(),
            media_id,
        );
    }

    pub fn update_library_drag(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(button) = event
            .pressed_button
            .and_then(LibraryDragButton::from_mouse_button)
        else {
            return;
        };
        let base_handle = self.library_scroll.0.borrow().base_handle.clone();
        let Some(update) = self.library_drag.update(
            button,
            (event.position.x.into(), event.position.y.into()),
            base_handle.max_offset().height.into(),
        ) else {
            return;
        };

        if update.just_activated {
            self.mark_library_scrolled(cx);
            self.hovered_tiles.clear();
        } else {
            self.library_scroll_pause_until =
                std::time::Instant::now() + std::time::Duration::from_millis(350);
        }
        let offset = base_handle.offset();
        base_handle.set_offset(point(offset.x, px(update.offset_y)));
        self.load_more_library_if_near_end();
        cx.notify();
    }

    pub fn finish_library_drag(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(button) = LibraryDragButton::from_mouse_button(event.button) else {
            return;
        };
        let release = self.library_drag.finish(button);
        if let LibraryDragRelease::ContextMenu(media_id) = release {
            self.open_video_context_menu(media_id, window, cx);
        }
        if release == LibraryDragRelease::ScrolledPrimary {
            cx.on_next_frame(window, |app, _window, _cx| {
                app.library_drag.clear_primary_click_suppression();
            });
        }
        if release != LibraryDragRelease::None {
            cx.notify();
        }
    }

    pub fn cancel_library_drag(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        if LibraryDragButton::from_mouse_button(event.button).is_some() {
            let was_active = self.library_drag.is_active();
            self.library_drag.cancel();
            if was_active {
                cx.notify();
            }
        }
    }

    /// Close the random-source popup and return keyboard focus to its trigger.
    pub fn close_random_popup(&mut self, cx: &mut Context<Self>) {
        self.random_popup_open = false;
        self.random_popup_focus_pending = false;
        self.random_source_focus_pending = true;
        self.mark_menu_closed();
        if self.focused_field.as_deref() == Some("random-filter") {
            self.focused_field = None;
            self.platform_input_focus = None;
        }
        cx.notify();
    }

    /// Toggle the random-source popup (with the original's 180ms guard).
    pub fn toggle_random_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.random_popup_open {
            self.close_random_popup(cx);
            return;
        } else if self.menu_reopen_allowed() {
            self.dismiss_root_popovers();
            self.random_popup_open = true;
            self.random_popup_focus_pending = true;
            self.random_loading = true;
            self.random_tree_cursor = 0;
            self.refresh_random_tree();
            self.command(Command::LoadRandomFolderOptions);
            window.focus(&self.random_popup_focus);
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

// Folders with every ancestor expanded, in tree order. This projection is
// cached so `render_explorer` can virtualize against it each frame instead of
// re-filtering the whole folder tree.
fn explorer_visible_nodes(
    folders: &[FolderNode],
    expanded_map: &HashMap<String, bool>,
) -> Vec<FolderNode> {
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
    folders
        .iter()
        .filter(|node| is_visible(node))
        .cloned()
        .collect()
}

#[cfg(test)]
mod explorer_projection_tests {
    use super::explorer_visible_nodes;
    use crate::state::FolderNode;
    use std::collections::HashMap;

    fn test_node(folder: &str, parent: &str, depth: usize) -> FolderNode {
        FolderNode {
            folder: folder.to_string(),
            name: folder.rsplit('/').next().unwrap_or(folder).to_string(),
            count: 1,
            depth,
            has_children: true,
            expanded: false,
            parent: parent.to_string(),
            latest_mtime: 0.0,
            latest_indexed: String::new(),
        }
    }

    #[test]
    fn only_nodes_with_all_ancestors_expanded_are_visible() {
        let folders = vec![
            test_node("a", "", 0),
            test_node("a/b", "a", 1),
            test_node("a/b/c", "a/b", 2),
            test_node("a/b/c/d", "a/b/c", 3),
            test_node("a/x", "a", 1),
        ];
        // Every ancestor defaults to expanded.
        let expanded = HashMap::new();
        let projected = explorer_visible_nodes(&folders, &expanded);
        let visible: Vec<_> = projected.iter().map(|node| node.folder.as_str()).collect();
        assert_eq!(visible, ["a", "a/b", "a/b/c", "a/b/c/d", "a/x"]);

        // Collapsing one ancestor hides its whole subtree, keeps siblings.
        let mut collapsed = HashMap::new();
        collapsed.insert("a/b".to_string(), false);
        let projected = explorer_visible_nodes(&folders, &collapsed);
        let visible: Vec<_> = projected.iter().map(|node| node.folder.as_str()).collect();
        assert_eq!(visible, ["a", "a/b", "a/x"]);
    }

    #[test]
    fn projection_keeps_tree_order_of_declaration() {
        let folders = vec![
            test_node("a", "", 0),
            test_node("a/b", "a", 1),
            test_node("z", "", 0),
        ];
        let projected = explorer_visible_nodes(&folders, &HashMap::new());
        let visible: Vec<_> = projected.iter().map(|node| node.folder.as_str()).collect();
        assert_eq!(visible, ["a", "a/b", "z"]);
    }
}

#[cfg(test)]
mod grid_tests {
    use super::{
        library_grid_row_count, library_should_prefetch, take_loaded_reveal_grid_row,
        LibraryDragButton, LibraryDragRelease, LibraryDragScroll,
    };
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

    #[test]
    fn library_drag_arbitrates_clicks_context_menus_and_fast_scroll() {
        let mut drag = LibraryDragScroll::default();
        drag.begin(LibraryDragButton::Primary, (100.0, 100.0), -120.0, Some(7));
        assert_eq!(
            drag.update(LibraryDragButton::Primary, (103.0, 104.0), 600.0),
            None
        );
        assert!(!drag.blocks_primary_click());

        let activated = drag
            .update(LibraryDragButton::Primary, (100.0, 90.0), 600.0)
            .expect("movement past the slop should start a drag");
        assert!(activated.just_activated);
        assert_eq!(activated.offset_y, -144.0);
        assert_eq!(
            drag.finish(LibraryDragButton::Primary),
            LibraryDragRelease::ScrolledPrimary
        );
        assert!(drag.blocks_primary_click());
        drag.clear_primary_click_suppression();
        assert!(!drag.blocks_primary_click());

        drag.begin(LibraryDragButton::Secondary, (40.0, 50.0), -80.0, None);
        drag.begin(LibraryDragButton::Secondary, (40.0, 50.0), -80.0, Some(42));
        assert_eq!(
            drag.finish(LibraryDragButton::Secondary),
            LibraryDragRelease::ContextMenu(42)
        );

        drag.begin(LibraryDragButton::Secondary, (40.0, 50.0), -580.0, Some(42));
        let clamped = drag
            .update(LibraryDragButton::Secondary, (40.0, 20.0), 600.0)
            .expect("secondary-button dragging should scroll too");
        assert_eq!(clamped.offset_y, -600.0);
        assert_eq!(
            drag.finish(LibraryDragButton::Secondary),
            LibraryDragRelease::ScrolledSecondary
        );
    }
}
