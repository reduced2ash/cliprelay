//! Prepare workspace renderers: stage (frame + transport + timeline +
//! source strip), inspector tabs, edit/publish inspectors, action dock.

use crate::prepare::{COMPRESSION_OPTIONS, CLEANUP_OPTIONS};
use crate::prepare::{DragHandle, ShapeKind};
use crate::state::*;
use crate::theme::*;
use crate::widgets::*;
use crate::settings_import::*;
use cliprelay_core::media::CropSpec;
use cliprelay_core::utils::format_bytes;
use gpui::*;
use std::path::PathBuf;
use serde_json::json;

fn color_from_hex(hex: &str) -> Hsla {
    let hex = hex.trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).unwrap_or(0);
    let r = ((value >> 16) & 0xFF) as f32 / 255.0;
    let g = ((value >> 8) & 0xFF) as f32 / 255.0;
    let b = (value & 0xFF) as f32 / 255.0;
    Hsla::from(gpui::Rgba { r, g, b, a: 1.0 })
}

impl crate::App {
    /// The media stage: frame, transport, timeline, source strip.
    pub fn render_prepare_stage(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        panel_width: f32,
    ) -> impl Element {
        let duration = self.prepare.duration;
        let position = self.prepare.position;
        let trim_start = self.prepare.trim_start;
        let trim_end = self.prepare.trim_end;
        let playing = self.prepare.playing;
        let cut_active = self.prepare.cut_active();
        let frame_path = self.prepare.current_frame_path();
        let thumbnail = self
            .selected
            .as_ref()
            .and_then(|m| m.thumbnail_path.clone())
            .unwrap_or_default();
        let timeline_path = self
            .selected
            .as_ref()
            .and_then(|m| m.timeline_path.clone())
            .unwrap_or_default();
        let checking = self.checking;
        let disabled = checking;
        let track_width = (panel_width - 24.0).max(100.0);
        let track_height = 54.0;
        // Track origin in window coordinates.
        let is_studio = self.prepare.studio_mode;
        let track_left = if is_studio {
            // The body (sidebar + divider) precedes the studio stage.
            let collapsed = self.sidebar_collapsed || self.window_size.0 < 1080.0;
            let sidebar = if collapsed { 68.0 } else { 204.0 };
            sidebar + 1.0 + 12.0
        } else {
            self.window_size.0 - panel_width + 12.0
        };

        let mut stage = div()
            .id("prepare-stage")
            .w_full()
            .flex()
            .flex_col()
            .px(px(12.0))
            .pt(px(8.0))
            .gap(px(8.0));
        stage
            .interactivity()
            .on_mouse_move(cx.listener(move |app, event: &MouseMoveEvent, _window, cx| {
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                app.prepare_drag_move(x, y, track_left, track_width, duration, cx);
            }));
        stage.interactivity().on_mouse_up(
            MouseButton::Left,
            cx.listener(|app, _event: &MouseUpEvent, _window, cx| {
                let was_seek = matches!(app.prepare.drag, DragHandle::Seek | DragHandle::TrimIn);
                app.prepare.drag = DragHandle::None;
                if was_seek && app.prepare.duration > 0.0 {
                    // Ensure the frame at the final drag position is
                    // extracted even when the throttle dropped the last move.
                    app.prepare
                        .request_frame_at(app.prepare.position, true);
                }
                app.save_draft();
                cx.notify();
            }),
        );

        // Video frame.
        let source_ratio = self
            .selected
            .as_ref()
            .map(|m| {
                if m.width > 0 && m.height > 0 {
                    m.width as f32 / m.height as f32
                } else {
                    16.0 / 9.0
                }
            })
            .unwrap_or(16.0 / 9.0);
        let frame_height = (track_width / source_ratio).clamp(130.0, 360.0);
        // Frame rect in window coordinates (used by crop/mask drag math).
        // The workspace tabs live at the window BOTTOM, so the frame sits
        // under the 40px header + 42px context toolbar, then the studio
        // header (38) or the dock's checking strip (34) and the stage's
        // 8px top padding.
        let frame_x = track_left;
        let frame_top = 40.0 + 42.0
            + if is_studio { 38.0 } else if self.checking { 34.0 } else { 0.0 }
            + 8.0;
        self.prepare.frame_rect = (frame_x, frame_top, track_width, frame_height);
        let has_edits = self.prepare.has_edits();
        let mut frame = div()
            .id("prepare-frame")
            .w(px(track_width))
            .h(px(frame_height))
            .rounded(px(4.0))
            .bg(color_from_hex("#05070B"))
            .border_1()
            .border_color(if has_edits { theme.accent } else { theme.border_strong })
            .overflow_hidden()
            .relative();
        let image_source = frame_path
            .map(PathBuf::from)
            .or_else(|| {
                if !thumbnail.is_empty() {
                    Some(PathBuf::from(thumbnail))
                } else {
                    None
                }
            });
        if let Some(source) = image_source {
            frame = frame.child(
                img(source)
                    .w_full()
                    .h_full()
                    .object_fit(ObjectFit::Contain),
            );
        } else {
            frame = frame.child(
                div()
                    .w_full()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("▷", 40.0, theme.border_strong)),
            );
        }
        // Edit overlays (crop + masks).
        frame = frame.child(self.render_edit_overlays(cx, theme, track_width, frame_height));
        stage = stage.child(frame);

        // Transport row.
        let time_label = self.prepare.format_time(position);
        let duration_label = self.prepare.format_time(duration);
        stage = stage.child(
            div()
                .id("transport")
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(70.0))
                        .child(time_label)
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        ,
                )
                .child(div().flex_1())
                .child(workbench_button(
                    "back-5",
                    "",
                    "⏮",
                    ButtonKind::Ghost,
                    !disabled && duration > 0.0,
                    true,
                    "Back 5 seconds",
                    cx,
                    |app, cx| {
                        app.prepare.seek(app.prepare.position - 5.0, app.prepare.duration);
                        cx.notify();
                    },
                ))
                .child(
                    workbench_button(
                        "play-pause",
                        if playing { "Pause" } else { "Play" },
                        if playing { "⏸" } else { "▶" },
                        ButtonKind::Ghost,
                        !disabled && duration > 0.0,
                        false,
                        if playing { "Pause  ·  Space" } else { "Play  ·  Space" },
                        cx,
                        |app, cx| {
                            app.prepare.toggle_playback();
                            app.save_draft();
                            cx.notify();
                        },
                    )
                    .w(px(44.0)),
                )
                .child(workbench_button(
                    "forward-5",
                    "",
                    "⏭",
                    ButtonKind::Ghost,
                    !disabled && duration > 0.0,
                    true,
                    "Forward 5 seconds",
                    cx,
                    |app, cx| {
                        app.prepare.seek(app.prepare.position + 5.0, app.prepare.duration);
                        cx.notify();
                    },
                ))
                .child(if self.timeline_loading && !self.prepare.timeline_ready {
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .w(px(34.0))
                                .h(px(6.0))
                                .rounded(px(3.0))
                                .bg(theme.raised)
                                .overflow_hidden()
                                .relative()
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top_0()
                                        .h_full()
                                        .w(px(20.0))
                                        .bg(theme.accent),
                                ),
                        )
                        .child(
                            div()
                                .child("Filmstrip")
                                .text_size(px(12.0))
                                .text_color(theme.muted),
                        )
                        .into_any()
                } else {
                    div().into_any()
                })
                .child(div().flex_1())
                .child(
                    div()
                        .w(px(70.0))
                        .child(duration_label)
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        
                        .text_right(),
                ),
        );

        // Timeline track.
        let play_fraction = if duration > 0.0 {
            (position / duration).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let start_fraction = if duration > 0.0 {
            (trim_start / duration).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let end_fraction = if duration > 0.0 {
            (trim_end / duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let mut track = div()
            .id("timeline-track")
            .w(px(track_width))
            .h(px(track_height))
            .rounded(px(6.0))
            .bg(color_from_hex("#05070B"))
            .border_1()
            .border_color(if cut_active { theme.accent } else { theme.border_strong })
            .overflow_hidden()
            .relative()
            .opacity(if disabled { 0.55 } else { 1.0 });
        track.interactivity().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                if !app.checking {
                    let x: f32 = event.position.x.into();
                    let seconds =
                        ((x - track_left) / track_width * duration as f32).clamp(0.0, duration as f32);
                    app.prepare.drag = DragHandle::Seek;
                    app.prepare.drag_start_x = x as f64;
                    app.prepare.drag_start_value = seconds as f64;
                    app.prepare.seek(seconds as f64, duration);
                    cx.notify();
                }
            }),
        );
        // Filmstrip.
        if !timeline_path.is_empty() {
            track = track.child(
                img(PathBuf::from(timeline_path))
                    .w_full()
                    .h_full()
                    .object_fit(ObjectFit::Cover),
            );
        }
        // Outside-cut dims.
        track = track
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(px((start_fraction as f32) * track_width))
                    .bg(Hsla { h: 0.0, s: 0.0, l: 0.02, a: 0.7 }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .h_full()
                    .w(px(((1.0 - end_fraction) as f32) * track_width))
                    .bg(Hsla { h: 0.0, s: 0.0, l: 0.02, a: 0.7 }),
            );
        if cut_active {
            track = track.child(
                div()
                    .absolute()
                    .top_0()
                    .left(px((start_fraction as f32) * track_width))
                    .w(px(((end_fraction - start_fraction) as f32) * track_width))
                    .h_full()
                    .border_2()
                    .border_color(theme.accent),
            );
        }
        // Playhead.
        track = track
            .child(
                div()
                    .absolute()
                    .top_0()
                    .h_full()
                    .w(px(2.0))
                    .left(px(((play_fraction as f32) * track_width - 1.0).clamp(0.0, track_width - 2.0)))
                    .bg(Hsla { h: 0.55, s: 0.0, l: 0.97, a: 1.0 }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(((play_fraction as f32) * track_width - 3.5).clamp(0.0, track_width - 7.0)))
                    .w(px(7.0))
                    .h(px(7.0))
                    .rounded(px(4.0))
                    .bg(Hsla { h: 0.55, s: 0.0, l: 0.97, a: 1.0 }),
            );
        // Trim handles.
        track = track
            .child(self.trim_handle(
                cx,
                "trim-in",
                (start_fraction as f32) * track_width,
                track_height,
                true,
                DragHandle::TrimIn,
                track_left,
                track_width,
                duration,
            ))
            .child(self.trim_handle(
                cx,
                "trim-out",
                (end_fraction as f32) * track_width,
                track_height,
                false,
                DragHandle::TrimOut,
                track_left,
                track_width,
                duration,
            ));
        stage = stage.child(track);

        // Tick labels (0/25/50/75/100%) under the track, hidden in compact
        // mode like the original.
        if panel_width > 460.0 {
            let mut ticks = div()
                .w(px(track_width))
                .h(px(14.0))
                .relative()
                .flex_none();
            for index in 0..5 {
                let fraction = index as f32 / 4.0;
                let seconds = duration * fraction as f64;
                let label = self.prepare.format_time(seconds);
                ticks = ticks.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px((fraction * track_width - 16.0).clamp(0.0, track_width - 32.0)))
                        .w(px(32.0))
                        .child(label)
                        .text_size(px(10.0))
                        .text_color(theme.muted_soft),
                );
            }
            stage = stage.child(ticks);
        }

        // Precision row.
        let in_text = if self.focused_field.as_deref() == Some("prepare-in") {
            self.field_text("prepare-in")
        } else {
            self.prepare.format_time_precise(trim_start)
        };
        let out_text = if self.focused_field.as_deref() == Some("prepare-out") {
            self.field_text("prepare-out")
        } else {
            self.prepare.format_time_precise(trim_end)
        };
        stage = stage.child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(7.0))
                .child(
                    div()
                        .child("IN")
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    field(
                        "prepare-in",
                        "00:00.00",
                        &self
                            .fields
                            .get("prepare-in")
                            .cloned()
                            .unwrap_or_else(|| crate::widgets::FieldState {
                                text: in_text.clone(),
                                caret: in_text.chars().count(),
                                committed: false,
                            }),
                        self.focused_field.as_deref() == Some("prepare-in"),
                        !disabled,
                        false,
                        cx,
                    )
                    .w(px(90.0)),
                )
                .child(
                    div()
                        .child("OUT")
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    field(
                        "prepare-out",
                        "00:00.00",
                        &self
                            .fields
                            .get("prepare-out")
                            .cloned()
                            .unwrap_or_else(|| crate::widgets::FieldState {
                                text: out_text.clone(),
                                caret: out_text.chars().count(),
                                committed: false,
                            }),
                        self.focused_field.as_deref() == Some("prepare-out"),
                        !disabled,
                        false,
                        cx,
                    )
                    .w(px(90.0)),
                )
                .child(div().flex_1())
                .child(if cut_active {
                    div()
                        .child(format!("CUT  {}", self.prepare.format_time_precise(trim_end - trim_start)))
                        .text_size(px(12.0))
                        .text_color(theme.accent_text)
                        .font_weight(FontWeight::BOLD)
                        
                } else {
                    div()
                        .child(format!("FULL  {}", self.prepare.format_time_precise(duration)))
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .font_weight(FontWeight::BOLD)
                        
                })
                .child(if cut_active {
                    workbench_button(
                        "reset-cut",
                        "",
                        "↺",
                        ButtonKind::Ghost,
                        true,
                        true,
                        "Reset cut to full video",
                        cx,
                        |app, cx| {
                            app.prepare.reset_cut();
                            app.save_draft();
                            cx.notify();
                        },
                    )
                    .into_any()
                } else {
                    div().into_any()
                }),
        );

        // Source strip.
        let name = self
            .selected
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let path = self
            .selected
            .as_ref()
            .map(|m| m.path.clone())
            .unwrap_or_default();
        let size_label = self
            .selected
            .as_ref()
            .map(|m| format_bytes(m.size_bytes as f64))
            .unwrap_or_default();
        let resolution_label = self
            .selected
            .as_ref()
            .map(|m| {
                if m.width > 0 && m.height > 0 {
                    format!("{}×{}", m.width, m.height)
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();
        stage = stage.child(
            div()
                .w_full()
                .h(px(48.0))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .child(name)
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .font_weight(FontWeight::BOLD)
                        .text_ellipsis()
                        .max_w(px(150.0)),
                )
                .child(if self.selected.is_some() {
                    let mut parts = vec![size_label, self.prepare.format_time(duration)];
                    if !resolution_label.is_empty() {
                        parts.push(resolution_label);
                    }
                    div()
                        .flex_1()
                        .child(parts.join("  ·  "))
                        .text_size(px(12.0))
                        .text_color(theme.muted)
                        .text_ellipsis()
                } else {
                    div()
                })
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .id("source-path")
                                .child(if path.is_empty() { "—".to_string() } else { path.clone() })
                                .text_size(px(11.0))
                                .text_color(theme.muted)
                                .max_w(px(120.0))
                                .text_ellipsis()
                                .tooltip(move |_window, cx| {
                                    crate::tooltip_view(cx, path.clone().into())
                                }),
                        )
                        .child(workbench_button(
                            "reveal-in-library",
                            "Reveal in library",
                            "◎",
                            ButtonKind::Ghost,
                            true,
                            panel_width < 470.0,
                            "Reveal in library",
                            cx,
                            |app, cx| {
                                app.command(Command::RevealSelectedInLibrary);
                                cx.notify();
                            },
                        )),
                ),
        );
        let _ = path;
        stage
    }

    #[allow(clippy::too_many_arguments)]
    fn trim_handle(
        &self,
        cx: &mut Context<Self>,
        id: &'static str,
        x: f32,
        track_height: f32,
        is_in: bool,
        handle: DragHandle,
        track_left: f32,
        track_width: f32,
        duration: f64,
    ) -> impl Element {
        let theme = self.theme.clone();
        let label: SharedString = if is_in {
            format!("IN  {}", self.prepare.format_time_precise(self.prepare.trim_start)).into()
        } else {
            format!("OUT  {}", self.prepare.format_time_precise(self.prepare.trim_end)).into()
        };
        let _ = duration;
        let mut element = div()
            .id(id)
            .absolute()
            .top_0()
            .left(px((x - 5.0).clamp(0.0, track_width - 10.0)))
            .w(px(10.0))
            .h(px(track_height))
            .rounded(px(3.0))
            .bg(theme.accent)
            .border_1()
            .border_color(theme.accent_content)
            .cursor_ew_resize()
            .tooltip(move |_window, cx| crate::tooltip_view(cx, label.clone()));
        element.interactivity().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                if event.button == MouseButton::Left {
                    let current = match handle {
                        DragHandle::TrimIn => app.prepare.trim_start,
                        _ => app.prepare.trim_end,
                    };
                    app.prepare.drag = handle;
                    let x: f32 = event.position.x.into();
                    app.prepare.drag_start_x = x as f64;
                    app.prepare.drag_start_value = current;
                    let _ = track_left;
                    cx.notify();
                }
            }),
        );
        element
    }

    pub fn prepare_drag_move(
        &mut self,
        pointer_x: f32,
        pointer_y: f32,
        _track_left: f32,
        track_width: f32,
        duration: f64,
        cx: &mut Context<Self>,
    ) {
        if self.prepare.drag == DragHandle::StudioSplit {
            let delta = pointer_x - self.prepare.drag_start_x as f32;
            let width = (self.prepare.drag_start_value as f32 + delta).clamp(360.0, 620.0);
            if (width - self.prepare.studio_width as f32).abs() > 1.0 {
                self.prepare.studio_width = width as f64;
                cx.notify();
            }
            return;
        }
        if duration <= 0.0 {
            return;
        }
        match self.prepare.drag {
            DragHandle::Seek | DragHandle::TrimIn | DragHandle::TrimOut => {
                let delta = pointer_x - self.prepare.drag_start_x as f32;
                let seconds =
                    (self.prepare.drag_start_value + delta as f64 / track_width as f64 * duration)
                        .clamp(0.0, duration);
                match self.prepare.drag {
                    DragHandle::Seek => {
                        self.prepare.seek(seconds, duration);
                        cx.notify();
                    }
                    DragHandle::TrimIn => {
                        let max = (self.prepare.trim_end - 0.05).max(0.0);
                        let value = seconds.clamp(0.0, max);
                        if (value - self.prepare.trim_start).abs() > 0.001 {
                            self.prepare.trim_start = value;
                            self.prepare.seek(value, duration);
                            cx.notify();
                        }
                    }
                    DragHandle::TrimOut => {
                        let min = (self.prepare.trim_start + 0.05).min(duration);
                        let value = seconds.clamp(min, duration);
                        if (value - self.prepare.trim_end).abs() > 0.001 {
                            self.prepare.trim_end = value;
                            cx.notify();
                        }
                    }
                    _ => {}
                }
            }
            DragHandle::CropMove | DragHandle::CropResize(_) => {
                self.crop_drag_move(pointer_x, pointer_y, cx);
            }
            DragHandle::MaskMove(_) | DragHandle::MaskResize(_) => {
                self.mask_drag_move(pointer_x, pointer_y, cx);
            }
            DragHandle::None | DragHandle::StudioSplit => {}
        }
    }

    /// Start a crop or mask drag; records the pointer + starting geometry.
    fn start_edit_drag(
        &mut self,
        handle: DragHandle,
        pointer_x: f32,
        pointer_y: f32,
    ) {
        self.prepare.drag = handle;
        self.prepare.drag_start_x = pointer_x as f64;
        self.prepare.drag_start_y = pointer_y as f64;
        self.prepare.crop_start = self.prepare.crop;
        if let DragHandle::MaskMove(index) | DragHandle::MaskResize(index) = handle {
            if let Some(shape) = self.prepare.shapes.get(index) {
                self.prepare.shape_start = (*shape).into();
            }
        }
    }

    fn crop_drag_move(&mut self, pointer_x: f32, pointer_y: f32, cx: &mut Context<Self>) {
        let (fx, fy, fw, fh) = self.prepare.frame_rect;
        if fw <= 0.0 || fh <= 0.0 {
            return;
        }
        let nx = (((pointer_x - fx) / fw) as f64).clamp(0.0, 1.0);
        let ny = (((pointer_y - fy) / fh) as f64).clamp(0.0, 1.0);
        let dx = nx - self.prepare.drag_start_x;
        let dy = ny - self.prepare.drag_start_y;
        let min = 0.04;
        let start = self.prepare.crop_start;
        let mut crop = start;
        match self.prepare.drag {
            DragHandle::CropMove => {
                crop.x = (start.x + dx).clamp(0.0, 1.0 - start.width);
                crop.y = (start.y + dy).clamp(0.0, 1.0 - start.height);
            }
            DragHandle::CropResize(corner) => match corner {
                0 => {
                    // Top-left: shift x/y, shrink width/height.
                    let new_x = (start.x + dx).clamp(0.0, start.x + start.width - min);
                    let new_y = (start.y + dy).clamp(0.0, start.y + start.height - min);
                    crop.x = new_x;
                    crop.y = new_y;
                    crop.width = start.width + start.x - new_x;
                    crop.height = start.height + start.y - new_y;
                }
                1 => {
                    // Top-right: grow/shrink width from left edge, shift y.
                    crop.width = (start.width + dx).clamp(min, 1.0 - start.x);
                    let new_y = (start.y + dy).clamp(0.0, start.y + start.height - min);
                    crop.y = new_y;
                    crop.height = start.height + start.y - new_y;
                }
                2 => {
                    // Bottom-left: shift x, grow/shrink height from top.
                    let new_x = (start.x + dx).clamp(0.0, start.x + start.width - min);
                    crop.x = new_x;
                    crop.width = start.width + start.x - new_x;
                    crop.height = (start.height + dy).clamp(min, 1.0 - start.y);
                }
                _ => {
                    // Bottom-right.
                    crop.width = (start.width + dx).clamp(min, 1.0 - start.x);
                    crop.height = (start.height + dy).clamp(min, 1.0 - start.y);
                }
            },
            _ => {}
        }
        if (crop.x - self.prepare.crop.x).abs() > 0.0001
            || (crop.y - self.prepare.crop.y).abs() > 0.0001
            || (crop.width - self.prepare.crop.width).abs() > 0.0001
            || (crop.height - self.prepare.crop.height).abs() > 0.0001
        {
            self.prepare.crop = crop;
            self.save_draft();
            cx.notify();
        }
    }

    fn mask_drag_move(&mut self, pointer_x: f32, pointer_y: f32, cx: &mut Context<Self>) {
        let (fx, fy, fw, fh) = self.prepare.frame_rect;
        if fw <= 0.0 || fh <= 0.0 {
            return;
        }
        let nx = (((pointer_x - fx) / fw) as f64).clamp(0.0, 1.0);
        let ny = (((pointer_y - fy) / fh) as f64).clamp(0.0, 1.0);
        let dx = nx - self.prepare.drag_start_x;
        let dy = ny - self.prepare.drag_start_y;
        let min = 0.025;
        let index = match self.prepare.drag {
            DragHandle::MaskMove(i) | DragHandle::MaskResize(i) => i,
            _ => return,
        };
        let Some(shape) = self.prepare.shapes.get_mut(index) else {
            return;
        };
        let start = self.prepare.shape_start;
        match self.prepare.drag {
            DragHandle::MaskMove(_) => {
                shape.x = (start.0 + dx).clamp(0.0, 1.0 - start.2);
                shape.y = (start.1 + dy).clamp(0.0, 1.0 - start.3);
            }
            DragHandle::MaskResize(_) => {
                shape.width = (start.2 + dx).clamp(min, 1.0 - start.0);
                shape.height = (start.3 + dy).clamp(min, 1.0 - start.1);
                if (shape.width - shape.height).abs() < 0.015 {
                    // Snap to square when the user drags near a square ratio.
                    shape.kind = ShapeKind::Square;
                } else {
                    shape.kind = ShapeKind::Rectangle;
                }
            }
            _ => {}
        }
        self.save_draft();
        cx.notify();
    }

    /// Overlays for crop rect + black masks on the frame.
    fn render_edit_overlays(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        frame_width: f32,
        frame_height: f32,
    ) -> impl Element {
        let mut overlays = div().id("edit-overlays").absolute().top_0().left_0().size_full();
        let crop = self.prepare.crop;
        let crop_enabled = self.prepare.crop_enabled;
        let shapes = self.prepare.shapes.clone();
        let selected_shape = self.prepare.selected_shape;

        if crop_enabled {
            let x = (crop.x as f32 * frame_width).max(0.0);
            let y = (crop.y as f32 * frame_height).max(0.0);
            let w = (crop.width as f32 * frame_width).clamp(0.0, frame_width - x);
            let h = (crop.height as f32 * frame_height).clamp(0.0, frame_height - y);
            // Dim outside.
            for (top, height) in [(0.0, y), (y + h, frame_height - y - h)] {
                if height > 0.5 {
                    overlays = overlays.child(
                        div()
                            .absolute()
                            .top(px(top))
                            .left_0()
                            .w_full()
                            .h(px(height))
                            .bg(Hsla { h: 0.0, s: 0.0, l: 0.02, a: 0.6 }),
                    );
                }
            }
            for (left, width) in [(0.0, x), (x + w, frame_width - x - w)] {
                if width > 0.5 {
                    overlays = overlays.child(
                        div()
                            .absolute()
                            .top(px(y))
                            .left(px(left))
                            .w(px(width))
                            .h(px(h))
                            .bg(Hsla { h: 0.0, s: 0.0, l: 0.02, a: 0.6 }),
                    );
                }
            }
            // Crop body: move.
            let mut body = div()
                .id("crop-body")
                .absolute()
                .top(px(y))
                .left(px(x))
                .w(px(w))
                .h(px(h))
                .cursor_move();
            body.interactivity().on_mouse_down(
                MouseButton::Left,
                cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                    if event.button == MouseButton::Left {
                        let px: f32 = event.position.x.into();
                        let py: f32 = event.position.y.into();
                        app.start_edit_drag(DragHandle::CropMove, px, py);
                        cx.notify();
                    }
                }),
            );
            overlays = overlays.child(body);
            // Corner resize handles.
            let corners = [
                (0, x, y, "nwse"),
                (1, x + w, y, "nesw"),
                (2, x, y + h, "nesw"),
                (3, x + w, y + h, "nwse"),
            ];
            for (corner, hx, hy, cursor) in corners {
                let mut handle = div()
                    .id(SharedString::from(format!("crop-corner-{corner}")))
                    .absolute()
                    .top(px(hy - 9.0))
                    .left(px(hx - 9.0))
                    .w(px(18.0))
                    .h(px(18.0))
                    .rounded(px(4.0))
                    .bg(theme.accent)
                    .border_2()
                    .border_color(theme.accent_content)
                    .cursor_pointer();
                handle.interactivity().on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                        if event.button == MouseButton::Left {
                            let px: f32 = event.position.x.into();
                            let py: f32 = event.position.y.into();
                            app.start_edit_drag(DragHandle::CropResize(corner), px, py);
                            cx.notify();
                        }
                    }),
                );
                let _ = cursor;
                overlays = overlays.child(handle);
            }
        }
        for (index, shape) in shapes.iter().enumerate() {
            let selected = selected_shape == Some(index);
            let x = shape.x as f32 * frame_width;
            let y = shape.y as f32 * frame_height;
            let w = shape.width as f32 * frame_width;
            let h = shape.height as f32 * frame_height;
            let mut mask = div()
                .id(SharedString::from(format!("mask-{index}")))
                .absolute()
                .top(px(y))
                .left(px(x))
                .w(px(w))
                .h(px(h))
                .bg(Hsla { h: 0.0, s: 0.0, l: 0.01, a: 0.85 })
                .border_2()
                .border_color(if selected { theme.accent } else { theme.border_strong })
                .cursor_move();
            mask.interactivity().on_mouse_down(
                MouseButton::Left,
                cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                    if event.button == MouseButton::Left {
                        app.prepare.selected_shape = Some(index);
                        let px: f32 = event.position.x.into();
                        let py: f32 = event.position.y.into();
                        app.start_edit_drag(DragHandle::MaskMove(index), px, py);
                        cx.notify();
                    }
                }),
            );
            overlays = overlays.child(mask);
            if selected {
                // Bottom-right resize handle.
                let mut resize = div()
                    .id(SharedString::from(format!("mask-resize-{index}")))
                    .absolute()
                    .top(px(y + h - 9.0))
                    .left(px(x + w - 9.0))
                    .w(px(18.0))
                    .h(px(18.0))
                    .rounded(px(4.0))
                    .bg(theme.accent)
                    .border_2()
                    .border_color(theme.accent_content)
                    .cursor_pointer();
                resize.interactivity().on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                        if event.button == MouseButton::Left {
                            let px: f32 = event.position.x.into();
                            let py: f32 = event.position.y.into();
                            app.start_edit_drag(DragHandle::MaskResize(index), px, py);
                            cx.notify();
                        }
                    }),
                );
                overlays = overlays.child(resize);
            }
        }
        overlays
    }

    // ---- inspector ------------------------------------------------------

    pub fn render_prepare_tabs(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let active_tab = self.prepare.inspector_tab;
        let edits = self.prepare.has_edits();
        let publish_active = self.publish.active;
        let publish_error = !self.publish.error.is_empty();
        let (edit_state, publish_state) = if edits {
            (
                format!(
                    "{} change{}",
                    if self.prepare.shapes.is_empty() { 1 } else { self.prepare.shapes.len() },
                    if self.prepare.shapes.len() == 1 { "" } else { "s" }
                ),
                "Ready".to_string(),
            )
        } else {
            ("No edits".to_string(), "Ready".to_string())
        };
        let publish_label: String = if publish_active {
            "Working".to_string()
        } else if publish_error {
            "Result".to_string()
        } else if self.bot_connected() || self.personal_configured() {
            publish_state.clone()
        } else {
            "Needs setup".to_string()
        };
        let mut bar = div()
            .id("prepare-tabs")
            .w_full()
            .h(px(40.0))
            .bg(theme.surface_soft)
            .border_t_1()
            .border_color(theme.border)
            .flex()
            .flex_row();
        let mut edit_tab = div()
            .id("tab-edit")
            .flex_1()
            .h_full()
            .cursor_pointer()
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .bg(if active_tab == 0 { theme.active } else { theme.transparent() })
            .child(icon("▣", 14.0, if edits { theme.accent_text } else { theme.muted }))
            .child(
                div()
                    .child("Edit")
                    .text_size(px(13.0))
                    .text_color(if active_tab == 0 { theme.text } else { theme.text_soft })
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                div()
                    .child(edit_state)
                    .text_size(px(12.0))
                    .text_color(if edits { theme.accent_text } else { theme.muted }),
            )
            .on_click(cx.listener(|app, _event, _window, cx| {
                app.prepare.inspector_tab = 0;
                app.save_draft();
                cx.notify();
            }));
        if active_tab == 0 {
            edit_tab = edit_tab.child(
                div().absolute().top_0().left_0().w_full().h(px(2.0)).bg(theme.accent),
            );
        }
        let mut publish_tab = div()
            .id("tab-publish")
            .flex_1()
            .h_full()
            .cursor_pointer()
            .relative()
            .border_l_1()
            .border_color(theme.border)
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .bg(if active_tab == 1 { theme.active } else { theme.transparent() })
            .child(icon("➤", 14.0, theme.muted))
            .child(
                div()
                    .child("Publish")
                    .text_size(px(13.0))
                    .text_color(if active_tab == 1 { theme.text } else { theme.text_soft })
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                div()
                    .child(publish_label.clone())
                    .text_size(px(12.0))
                    .text_color(if publish_active || publish_error {
                        theme.accent_text
                    } else {
                        theme.muted
                    }),
            )
            .on_click(cx.listener(|app, _event, _window, cx| {
                app.prepare.inspector_tab = 1;
                app.save_draft();
                cx.notify();
            }));
        if active_tab == 1 {
            publish_tab = publish_tab.child(
                div().absolute().top_0().left_0().w_full().h(px(2.0)).bg(theme.accent),
            );
        }
        bar = bar.child(edit_tab).child(publish_tab);
        bar
    }

    pub fn render_prepare_inspector(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        panel_width: f32,
    ) -> impl Element {
        let mut column = div()
            .id("prepare-inspector")
            .w_full()
            .flex_1()
            .min_h(px(120.0))
            .flex()
            .flex_col()
            .overflow_hidden();
        let content = if self.prepare.inspector_tab == 0 {
            self.render_edit_inspector(cx, theme, panel_width).into_any()
        } else {
            self.render_publish_inspector(cx, theme, panel_width).into_any()
        };
        let mut scroll = div()
            .id("inspector-scroll")
            .flex_1()
            .min_h(px(0.0))
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .child(div().w_full().flex().flex_col().px(px(14.0)).pt(px(14.0)).pb(px(16.0)).child(content));
        if self.checking {
            scroll = scroll.opacity(0.5);
        }
        column = column.child(scroll);
        if self.prepare.inspector_tab == 1 {
            column = column.child(self.render_action_dock(cx, theme));
        }
        column
    }

    fn render_edit_inspector(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        _panel_width: f32,
    ) -> impl Element {
        let crop_enabled = self.prepare.crop_enabled;
        let shape_count = self.prepare.shapes.len();
        let has_edits = self.prepare.has_edits();
        let crop_original = self.prepare.crop_is_original();
        let mut column = div().w_full().flex().flex_col().gap(px(12.0));

        // FRAME section.
        column = column
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .child("FRAME")
                                    .text_size(px(12.0))
                                    .text_color(theme.muted)
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                div()
                                    .child("Crop the visible frame without changing the source.")
                                    .text_size(px(12.0))
                                    .text_color(theme.text_soft),
                            ),
                    )
                    .child(if crop_enabled {
                        status_pill("pill-crop", "Crop", PillState::Neutral).into_any()
                    } else {
                        div().into_any()
                    })
                    .child(if has_edits {
                        workbench_button(
                            "reset-frame-edits",
                            "",
                            "↺",
                            ButtonKind::Ghost,
                            true,
                            true,
                            "Reset all frame edits",
                            cx,
                            |app, cx| {
                                app.prepare.reset_crop();
                                app.prepare.clear_shapes();
                                app.save_draft();
                                cx.notify();
                            },
                        )
                        .into_any()
                    } else {
                        div().into_any()
                    }),
            )
            .child(checkbox(
                "enable-crop",
                "Enable crop",
                crop_enabled,
                true,
                cx,
                |app, cx, enabled| {
                    app.prepare.crop_enabled = enabled;
                    if !enabled {
                        app.prepare.reset_crop();
                    } else if app.prepare.crop_is_original() {
                        app.prepare.apply_crop_aspect(0.0, 16.0 / 9.0);
                    }
                    app.save_draft();
                    cx.notify();
                },
            ))
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
                            .child("Crop aspect")
                            .text_size(px(13.0))
                            .text_color(theme.text_soft),
                    )
                    .child(crop_aspect_combo(self, cx, theme)),
            )
            .child(if crop_enabled && !crop_original {
                button(
                    "reset-crop",
                    "Reset crop",
                    ButtonKind::Ghost,
                    Some("↺"),
                    true,
                    cx,
                    |app, cx| {
                        app.prepare.reset_crop();
                        app.save_draft();
                        cx.notify();
                    },
                )
                .into_any()
            } else {
                div().into_any()
            })
            .child(divider());

        // BLACK MASKS section.
        column = column
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
                            .child("BLACK MASKS")
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(if shape_count > 0 {
                        status_pill("pill-masks", &format!("{shape_count} masks"), PillState::Neutral)
                            .into_any()
                    } else {
                        div().into_any()
                    }),
            )
            .child(
                div()
                    .child("Select a mask here, then position it on the video.")
                    .text_size(px(12.0))
                    .text_color(theme.text_soft),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(button(
                        "add-rectangle",
                        "Add rectangle",
                        ButtonKind::Secondary,
                        Some("▭"),
                        true,
                        cx,
                        |app, cx| {
                            app.prepare.add_shape(false);
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(button(
                        "add-square",
                        "Add square",
                        ButtonKind::Secondary,
                        Some("□"),
                        true,
                        cx,
                        |app, cx| {
                            app.prepare.add_shape(true);
                            app.save_draft();
                            cx.notify();
                        },
                    )),
            );

        // Mask list.
        if self.prepare.shapes.is_empty() {
            column = column.child(
                div()
                    .w_full()
                    .py(px(10.0))
                    .child("No masks")
                    .text_size(px(12.0))
                    .text_color(theme.muted_soft),
            );
        } else {
            let mut list = div()
                .id("mask-list")
                .w_full()
                .rounded(px(4.0))
                .bg(theme.raised)
                .border_1()
                .border_color(theme.border)
                .flex()
                .flex_col()
                .overflow_hidden();
            let shapes = self.prepare.shapes.clone();
            let selected = self.prepare.selected_shape;
            for (index, shape) in shapes.iter().enumerate() {
                let kind = match shape.kind {
                    ShapeKind::Rectangle => "Rectangle",
                    ShapeKind::Square => "Square",
                };
                let is_selected = selected == Some(index);
                let label = format!("{kind} {}", index + 1);
                list = list.child(
                    div()
                        .id(SharedString::from(format!("mask-row-{index}")))
                        .w_full()
                        .h(px(36.0))
                        .px(px(11.0))
                        .cursor_pointer()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .bg(if is_selected { theme.active } else { theme.transparent() })
                        .child(icon(if shape.kind == ShapeKind::Rectangle { "▭" } else { "□" }, 14.0, if is_selected { theme.accent_text } else { theme.muted }))
                        .child(
                            div()
                                .flex_1()
                                .child(label)
                                .text_size(px(12.0))
                                .text_color(if is_selected { theme.text } else { theme.text_soft })
                                .font_weight(if is_selected { FontWeight::BOLD } else { FontWeight::MEDIUM }),
                        )
                        .child(if is_selected {
                            workbench_button(
                                "remove-mask",
                                "",
                                "🗑",
                                ButtonKind::Danger,
                                true,
                                true,
                                "Remove mask",
                                cx,
                                |app, cx| {
                                    app.prepare.remove_selected_shape();
                                    app.save_draft();
                                    cx.notify();
                                },
                            )
                            .into_any()
                        } else {
                            div().into_any()
                        })
                        .on_click(cx.listener(move |app, _event, _window, cx| {
                            app.prepare.selected_shape = Some(index);
                            cx.notify();
                        })),
                );
            }
            column = column.child(list);
            if self.prepare.shapes.len() > 1 {
                column = column.child(button(
                    "clear-masks",
                    "Clear all masks",
                    ButtonKind::Ghost,
                    Some("🗑"),
                    true,
                    cx,
                    |app, cx| {
                        app.prepare.clear_shapes();
                        app.save_draft();
                        cx.notify();
                    },
                ));
            }
        }
        column
    }

    fn render_publish_inspector(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        _panel_width: f32,
    ) -> impl Element {
        let estimated = self.estimate_output_size_label();
        let compression_index = self.prepare.compression_index as usize;
        let _cleanup_index = self.prepare.cleanup_index as usize;
        let mode_index = self.prepare.telegram_mode_index as usize;
        let bot_ready = self.bot_connected();
        let personal_ready = self.personal_configured();
        let connected = if mode_index == 1 { personal_ready } else { bot_ready };
        let destination = self.prepare.destination.clone();
        let destination_ready = !destination.trim().is_empty();
        let telegram_ready = connected && destination_ready;
        let x_duration_warning = {
            let limit = self
                .settings
                .get(X_DURATION_SECONDS)
                .and_then(|v| v.as_i64())
                .unwrap_or(140) as f64;
            self.prepare.duration > 0.0
                && (self.prepare.trim_end - self.prepare.trim_start) > limit
        };

        let mut column = div().w_full().flex().flex_col().gap(px(12.0));

        // OUTPUT.
        column = column
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
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .child("OUTPUT")
                                    .text_size(px(12.0))
                                    .text_color(theme.muted)
                                    .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                div()
                                    .child("Choose a destination-aware generated copy.")
                                    .text_size(px(12.0))
                                    .text_color(theme.text_soft),
                            ),
                    )
                    .child(status_pill("pill-estimate", &estimated, PillState::Neutral)),
            )
            .child(compression_combo(self, cx, theme))
            .child(if compression_index == 6 {
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .child("Maximum generated size")
                            .text_size(px(12.0))
                            .text_color(theme.text_soft),
                    )
                    .child(
                        field(
                            "target-mb",
                            "MB",
                            &self.fields.get("target-mb").cloned().unwrap_or_default(),
                            self.focused_field.as_deref() == Some("target-mb"),
                            true,
                            false,
                            cx,
                        )
                        .w(px(112.0)),
                    )
            } else {
                div()
            })
            .child(
                div()
                    .child(match compression_index {
                        0 => "Uses the source when it already fits.",
                        5 => "Prioritizes the smallest practical file.",
                        _ => "The estimate follows the current cut.",
                    })
                    .text_size(px(12.0))
                    .text_color(theme.muted),
            )
            .child(divider());

        // DESTINATIONS.
        column = column
            .child(
                div()
                    .child("DESTINATIONS")
                    .text_size(px(12.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon("➤", 16.0, if telegram_ready { theme.success } else { theme.muted }))
                    .child(
                        div()
                            .child("Telegram")
                            .text_size(px(13.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(div().flex_1())
                    .child(status_pill(
                        "pill-tg",
                        if telegram_ready { "Ready" } else { "Needs setup" },
                        if telegram_ready { PillState::Success } else { PillState::Warning },
                    )),
            )
            .child(mode_combo(self, cx, theme))
            .child(
                field(
                    "tg-destination-field",
                    if mode_index == 1 { "Username or chat ID" } else { "@channel or chat ID" },
                    &self.fields.get("tg-destination-field").cloned().unwrap_or_default(),
                    self.focused_field.as_deref() == Some("tg-destination-field"),
                    true,
                    false,
                    cx,
                ),
            )
            .child(
                div()
                    .child(if !connected {
                        if mode_index == 1 {
                            "Sign in under Settings before sending."
                        } else {
                            "Connect a bot in Settings before sending."
                        }
                    } else if !destination_ready {
                        "Enter a destination before sending."
                    } else if mode_index == 1 {
                        "Personal account connected to the selected destination."
                    } else {
                        "Bot connected to the selected destination."
                    })
                    .text_size(px(12.0))
                    .text_color(if telegram_ready { theme.success } else { theme.warning }),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon("𝕏", 16.0, if x_duration_warning { theme.warning } else { theme.text_soft }))
                    .child(
                        div()
                            .child("X")
                            .text_size(px(13.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(div().flex_1())
                    .child(status_pill(
                        "pill-x",
                        if x_duration_warning { "Check cut" } else { "Manual" },
                        if x_duration_warning { PillState::Warning } else { PillState::Neutral },
                    )),
            )
            .child(if x_duration_warning {
                div()
                    .child("Current cut exceeds the configured duration limit.")
                    .text_size(px(12.0))
                    .text_color(theme.warning)
            } else {
                div()
                    .child("Manual browser handoff")
                    .text_size(px(12.0))
                    .text_color(theme.muted)
            })
            .child(divider());

        // CAPTIONS.
        column = column
            .child(
                div()
                    .child("CAPTIONS")
                    .text_size(px(12.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::BOLD),
            )
            .child(checkbox(
                "shared-caption",
                "Shared caption",
                self.prepare.same_caption,
                true,
                cx,
                |app, cx, value| {
                    app.prepare.same_caption = value;
                    app.save_draft();
                    cx.notify();
                },
            ));
        let caption_limit = if mode_index == 1 { 4096 } else { 1024 };
        if self.prepare.same_caption {
            let caption = self.prepare.caption.clone();
            let caption_len = caption.chars().count();
            column = column
                .child(
                    div()
                        .child("Telegram and X")
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::BOLD),
                )
                .child(caption_area(self, cx, "caption-shared", "Caption for Telegram and X", &caption))
                .child(
                    div()
                        .w_full()
                        .text_right()
                        .child(format!(
                            "Telegram {} / {}  ·  X {} / 280",
                            group_digits(caption_len),
                            group_digits(caption_limit),
                            group_digits(caption_len)
                        ))
                        .text_size(px(12.0))
                        .text_color(if caption_len > caption_limit || caption_len > 280 {
                            theme.warning
                        } else {
                            theme.muted
                        })
                        ,
                );
        } else {
            let caption = self.prepare.caption.clone();
            let x_caption = self.prepare.x_caption.clone();
            let caption_len = caption.chars().count();
            let x_len = x_caption.chars().count();
            column = column
                .child(
                    div()
                        .child("Telegram")
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::BOLD),
                )
                .child(caption_area(self, cx, "caption-tg", "Telegram message", &caption))
                .child(
                    div()
                        .w_full()
                        .text_right()
                        .child(format!(
                            "Telegram {} / {}",
                            group_digits(caption_len),
                            group_digits(caption_limit)
                        ))
                        .text_size(px(12.0))
                        .text_color(if caption_len > caption_limit { theme.warning } else { theme.muted })
                        ,
                )
                .child(
                    div()
                        .child("X")
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::BOLD),
                )
                .child(caption_area(self, cx, "caption-x", "X post text", &x_caption))
                .child(
                    div()
                        .w_full()
                        .text_right()
                        .child(format!("X {} / 280", group_digits(x_len)))
                        .text_size(px(12.0))
                        .text_color(if x_len > 280 { theme.warning } else { theme.muted })
                        ,
                );
        }
        column = column.child(divider());

        // Generated copy.
        column = column
            .child(
                div()
                    .child("Generated copy")
                    .text_size(px(13.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                div()
                    .child("Cleanup never applies to the source.")
                    .text_size(px(12.0))
                    .text_color(theme.muted),
            )
            .child(cleanup_combo(self, cx, theme));
        column
    }

    fn render_action_dock(&mut self, cx: &mut Context<Self>, theme: &crate::theme::Theme) -> impl Element {
        let publish = self.publish.clone();
        let telegram_ready = self.telegram_ready();
        let checking = self.checking;
        let output_ready = !publish.output_path.is_empty();
        // The X handoff row only shows when X was an actual destination of
        // the last publish (mirrors the original's lastSubmitXEnabled).
        let output_x_ready = output_ready && self.prepare.last_submit_x_enabled;
        let wide = self.window_size.0 > 1200.0;

        let mut dock = div()
            .id("action-dock")
            .w_full()
            .bg(theme.surface_soft)
            .border_t_1()
            .border_color(theme.border)
            .px(px(12.0))
            .py(px(10.0))
            .flex()
            .flex_col()
            .gap(px(8.0));

        if checking {
            dock = dock.child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon("i", 15.0, theme.accent_text))
                    .child(
                        div()
                            .child("Publishing unlocks when the selected video is ready")
                            .text_size(px(12.0))
                            .text_color(theme.text_soft),
                    ),
            );
        } else if publish.active {
            let stage = if publish.stage.is_empty() {
                "Preparing generated copy".to_string()
            } else {
                publish.stage.clone()
            };
            dock = dock
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
                                .child(stage)
                                .text_size(px(12.0))
                                .text_color(theme.text_soft)
                                .text_ellipsis(),
                        )
                        .child(
                            div()
                                .child(format!("{}%", (publish.progress * 100.0).round() as i64))
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                ,
                        )
                        .child(workbench_button(
                            "cancel-publish",
                            "",
                            "✕",
                            ButtonKind::Danger,
                            true,
                            true,
                            "Cancel",
                            cx,
                            |app, cx| {
                                app.command(Command::CancelPublish);
                                cx.notify();
                            },
                        )),
                )
                .child(progress_bar(publish.progress, false));
        } else if output_x_ready {
            let output_path = publish.output_path.clone();
            let output_path_copy = output_path.clone();
            let output_path_drag = output_path.clone();
            let output_path_reveal = output_path.clone();
            let output_encoder = publish.output_encoder.clone();
            let output_size = publish.output_size.clone();
            let hardware = publish.hardware_accelerated;
            dock = dock
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(icon("✓", 15.0, theme.success))
                        .child(
                            div()
                                .child("X handoff ready · generated copy available")
                                .text_size(px(12.0))
                                .text_color(theme.text_soft)
                                .font_weight(FontWeight::BOLD),
                        ),
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
                                .child(if !output_encoder.is_empty() {
                                    format!(
                                        "{output_encoder}{}{}",
                                        if hardware { " · hardware" } else { "" },
                                        if output_size.is_empty() {
                                            String::new()
                                        } else {
                                            format!(" · {output_size}")
                                        }
                                    )
                                } else {
                                    String::new()
                                })
                                .text_size(px(11.0))
                                .text_color(theme.muted)
                                .text_ellipsis(),
                        )
                        .child(button(
                            "copy-video",
                            "Copy video",
                            ButtonKind::Secondary,
                            Some("⧉"),
                            true,
                            cx,
                            move |app, cx| {
                                if cliprelay_core::x::XAssistant::copy_file(std::path::Path::new(&output_path_copy)).is_ok() {
                                    app.toast(ToastKind::Success, "Video copied. Paste it into the X composer.");
                                }
                                cx.notify();
                            },
                        ))
                        .child(button(
                            "drag-video",
                            "Drag video",
                            ButtonKind::Secondary,
                            Some("↘"),
                            true,
                            cx,
                            move |app, cx| {
                                // Native drag-out has no GPUI equivalent;
                                // place the file on the clipboard so it can
                                // be pasted or dragged into the composer.
                                if cliprelay_core::x::XAssistant::copy_file(std::path::Path::new(&output_path_drag)).is_ok() {
                                    app.toast(ToastKind::Success, "Video copied. Paste it into the X composer.");
                                }
                                cx.notify();
                            },
                        ))
                        .child(button(
                            "show-in-folder",
                            "Show in folder",
                            ButtonKind::Secondary,
                            Some("▤"),
                            true,
                            cx,
                            move |_app, cx| {
                                let _ = cliprelay_core::x::XAssistant::reveal(std::path::Path::new(&output_path_reveal));
                                cx.notify();
                            },
                        )),
                );
        } else {
            let error = publish.error.clone();
            let estimate_label = self.estimate_output_size_label();
            dock = dock.child(
                div()
                    .w_full()
                    .child(if !error.is_empty() {
                        format!("Failed · {error}")
                    } else {
                        format!(
                            "{}  ·  X manual{}",
                            if telegram_ready {
                                "Telegram ready".to_string()
                            } else {
                                "Telegram needs setup".to_string()
                            },
                            if estimate_label.is_empty() {
                                String::new()
                            } else {
                                format!("  ·  {estimate_label}")
                            }
                        )
                    })
                    .text_size(px(12.0))
                    .text_color(if !error.is_empty() {
                        theme.error
                    } else if telegram_ready {
                        theme.text_soft
                    } else {
                        theme.warning
                    }),
            );
            if wide {
                dock = dock.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(
                            button(
                                "prepare-x",
                                "Prepare X",
                                ButtonKind::Secondary,
                                Some("𝕏"),
                                true,
                                cx,
                                |app, cx| {
                                    app.submit_publish("x", cx);
                                },
                            )
                            .flex_1(),
                        )
                        .child(
                            button(
                                "send-telegram",
                                "Send Telegram",
                                ButtonKind::Secondary,
                                Some("➤"),
                                telegram_ready,
                                cx,
                                |app, cx| {
                                    app.submit_publish("telegram", cx);
                                },
                            )
                            .flex_1(),
                        )
                        .child(
                            button(
                                "send-both",
                                "Send + prepare X",
                                ButtonKind::Primary,
                                Some("⇄"),
                                telegram_ready,
                                cx,
                                |app, cx| {
                                    app.submit_publish("both", cx);
                                },
                            )
                            .flex_1(),
                        ),
                );
            } else {
                dock = dock
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_row()
                            .gap(px(8.0))
                            .child(button(
                                "prepare-x-narrow",
                                "Prepare X",
                                ButtonKind::Secondary,
                                Some("𝕏"),
                                true,
                                cx,
                                |app, cx| {
                                    app.submit_publish("x", cx);
                                },
                            ))
                            .child(button(
                                "send-telegram-narrow",
                                "Send Telegram",
                                ButtonKind::Secondary,
                                Some("➤"),
                                telegram_ready,
                                cx,
                                |app, cx| {
                                    app.submit_publish("telegram", cx);
                                },
                            )),
                    )
                    .child(button(
                        "send-both-narrow",
                        "Send + prepare X",
                        ButtonKind::Primary,
                        Some("⇄"),
                        telegram_ready,
                        cx,
                        |app, cx| {
                            app.submit_publish("both", cx);
                        },
                    ));
            }
        }
        dock
    }

    pub fn telegram_ready(&self) -> bool {
        let mode = if self.prepare.telegram_mode_index == 1 {
            "personal"
        } else {
            "bot"
        };
        let connected = if mode == "personal" {
            self.personal_configured()
        } else {
            self.bot_connected()
        };
        connected && !self.prepare.destination.trim().is_empty()
    }

    pub fn estimate_output_size_label(&self) -> String {
        let preset = COMPRESSION_OPTIONS
            .get(self.prepare.compression_index as usize)
            .map(|(_, code)| *code)
            .unwrap_or("balanced");
        let target_mb: f64 = self.prepare.target_mb.parse().unwrap_or(0.0);
        let Some(selected) = &self.selected else {
            return String::new();
        };
        let source_duration = selected.duration.max(0.05);
        let trim_end = if self.prepare.trim_end > 0.0 {
            self.prepare.trim_end
        } else {
            source_duration
        };
        let duration = (trim_end - self.prepare.trim_start).max(0.05);
        let source_ratio = duration / source_duration;
        let preset_limit = match preset {
            "fit_bot" => Some(49.0),
            "fit_x" => Some(
                self.settings
                    .get(X_LIMIT_MB)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(512.0)
                    * 0.98,
            ),
            "fit_both" => Some(
                (self
                    .settings
                    .get(X_LIMIT_MB)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(512.0)
                    * 0.98)
                    .min(49.0),
            ),
            _ => None,
        };
        if let Some(limit) = preset_limit {
            return format!("up to {limit:.0} MB");
        }
        if preset == "custom" && target_mb > 0.0 {
            return format!("up to {target_mb:.0} MB");
        }
        let size = selected.size_bytes as f64;
        let estimate = if preset == "smallest" {
            (size * source_ratio * 0.28).min(duration * 750_000.0 / 8.0)
        } else if preset == "balanced" {
            (size * source_ratio * 0.62).min(duration * 2_500_000.0 / 8.0)
        } else {
            size * source_ratio
        };
        cliprelay_core::utils::format_bytes(estimate)
    }

    pub fn submit_publish(&mut self, action: &str, cx: &mut Context<Self>) {
        if self.publish.active {
            return;
        }
        let Some(selected) = self.selected.clone() else {
            self.toast(ToastKind::Error, "Choose a video before preparing a post.");
            return;
        };
        let send_telegram = action != "x";
        let prepare_x = action != "telegram";
        if !send_telegram && !prepare_x {
            return;
        }
        let preset = COMPRESSION_OPTIONS
            .get(self.prepare.compression_index as usize)
            .map(|(_, code)| *code)
            .unwrap_or("balanced")
            .to_string();
        let target_mb: f64 = self.prepare.target_mb.parse().unwrap_or(0.0);
        let mode = if self.prepare.telegram_mode_index == 1 {
            "personal"
        } else {
            "bot"
        };
        let cleanup_policy = CLEANUP_OPTIONS
            .get(self.prepare.cleanup_index as usize)
            .map(|(_, code)| *code)
            .unwrap_or("keep")
            .to_string();
        let payload = PublishPayload {
            media_id: selected.id,
            telegram_enabled: send_telegram,
            x_enabled: prepare_x,
            telegram_caption: self.prepare.caption_for("telegram"),
            x_caption: self.prepare.caption_for("x"),
            telegram_mode: mode.to_string(),
            telegram_destination: self.prepare.destination.clone(),
            cleanup_policy,
            preset,
            target_mb,
            trim_start: self.prepare.trim_start,
            trim_end: self.prepare.trim_end,
            edits: self.prepare.edit_spec(),
        };
        self.prepare.active_action = action.to_string();
        self.prepare.last_submit_x_enabled = prepare_x;
        self.command(Command::Publish(payload));
        cx.notify();
    }
}

// ---- small combos ---------------------------------------------------------

fn crop_aspect_combo(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> impl Element {
    let options: [&str; 5] = [
        "Free crop",
        "Original frame",
        "Square (1:1)",
        "Landscape (16:9)",
        "Portrait (9:16)",
    ];
    let selected = app.prepare_crop_preset();
    let open = app.open_combos.contains("crop-aspect");
    let trigger = div()
        .id("crop-aspect-trigger")
        .w(px(190.0))
        .h(px(40.0))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if open { theme.accent } else { theme.border })
        .cursor_pointer()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            div()
                .child(options[selected])
                .text_size(px(13.0))
                .text_color(theme.text),
        )
        .child(icon(if open { "▴" } else { "▾" }, 12.0, theme.muted))
        .on_click(cx.listener(|app, _event, _window, cx| {
            app.toggle_combo("crop-aspect", cx);
        }));
    if !open {
        return trigger;
    }
    let mut menu = div()
        .id("crop-aspect")
        .w(px(190.0))
        .rounded(px(10.0))
        .bg(theme.surface_soft)
        .border_1()
        .border_color(theme.border_strong)
        .py(px(4.0))
        .flex()
        .flex_col();
    for (index, option) in options.iter().enumerate() {
        let option = *option;
        menu = menu.child(
            div()
                .id(SharedString::from(format!("crop-aspect-{index}")))
                .h(px(36.0))
                .px(px(10.0))
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(13.0))
                        .child(if index == selected { "✓" } else { "" })
                        .text_size(px(13.0))
                        .text_color(theme.accent_text),
                )
                .child(
                    div()
                        .child(option)
                        .text_size(px(13.0))
                        .text_color(theme.text_soft),
                )
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.close_combo("crop-aspect", cx);
                    app.apply_crop_preset(index);
                    app.save_draft();
                    cx.notify();
                })),
        );
    }
    menu
}

fn compression_combo(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> impl Element {
    let selected = app.prepare.compression_index as usize;
    let open = app.open_combos.contains("compression");
    let trigger = div()
        .id("compression-trigger")
        .w_full()
        .h(px(CONTROL_HEIGHT))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if open { theme.accent } else { theme.border })
        .cursor_pointer()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            div()
                .child(COMPRESSION_OPTIONS[selected].0)
                .text_size(px(13.0))
                .text_color(theme.text),
        )
        .child(icon(if open { "▴" } else { "▾" }, 12.0, theme.muted))
        .on_click(cx.listener(|app, _event, _window, cx| {
            app.toggle_combo("compression", cx);
        }));
    if !open {
        return trigger;
    }
    let mut menu = div()
        .id("compression")
        .w_full()
        .rounded(px(10.0))
        .bg(theme.surface_soft)
        .border_1()
        .border_color(theme.border_strong)
        .py(px(4.0))
        .flex()
        .flex_col();
    for (index, (label, _)) in COMPRESSION_OPTIONS.iter().enumerate() {
        menu = menu.child(
            div()
                .id(SharedString::from(format!("compression-{index}")))
                .h(px(40.0))
                .px(px(10.0))
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(13.0))
                        .child(if index == selected { "✓" } else { "" })
                        .text_size(px(13.0))
                        .text_color(theme.accent_text),
                )
                .child(
                    div()
                        .child(*label)
                        .text_size(px(13.0))
                        .text_color(if index == selected { theme.text } else { theme.text_soft }),
                )
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.close_combo("compression", cx);
                    app.prepare.compression_index = index as i64;
                    app.save_draft();
                    cx.notify();
                })),
        );
    }
    menu
}

fn mode_combo(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> impl Element {
    let options: [&str; 2] = ["Bot", "Personal"];
    let selected = app.prepare.telegram_mode_index as usize;
    let open = app.open_combos.contains("tg-mode");
    let trigger = div()
        .id("tg-mode-trigger")
        .w(px(126.0))
        .h(px(CONTROL_HEIGHT))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if open { theme.accent } else { theme.border })
        .cursor_pointer()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            div()
                .child(options[selected])
                .text_size(px(13.0))
                .text_color(theme.text),
        )
        .child(icon(if open { "▴" } else { "▾" }, 12.0, theme.muted))
        .on_click(cx.listener(|app, _event, _window, cx| {
            app.toggle_combo("tg-mode", cx);
        }));
    if !open {
        return trigger;
    }
    let mut menu = div()
        .id("tg-mode")
        .w(px(126.0))
        .rounded(px(10.0))
        .bg(theme.surface_soft)
        .border_1()
        .border_color(theme.border_strong)
        .py(px(4.0))
        .flex()
        .flex_col();
    for (index, option) in options.iter().enumerate() {
        let option = *option;
        menu = menu.child(
            div()
                .id(SharedString::from(format!("tg-mode-{index}")))
                .h(px(40.0))
                .px(px(10.0))
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(13.0))
                        .child(if index == selected { "✓" } else { "" })
                        .text_size(px(13.0))
                        .text_color(theme.accent_text),
                )
                .child(
                    div()
                        .child(option)
                        .text_size(px(13.0))
                        .text_color(theme.text_soft),
                )
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.close_combo("tg-mode", cx);
                    app.prepare.telegram_mode_index = index as i64;
                    app.set_setting(TELEGRAM_MODE, json!(if index == 1 { "personal" } else { "bot" }), cx);
                    app.save_draft();
                    cx.notify();
                })),
        );
    }
    menu
}

fn cleanup_combo(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> impl Element {
    let selected = app.prepare.cleanup_index as usize;
    let open = app.open_combos.contains("cleanup");
    let trigger = div()
        .id("cleanup-trigger")
        .w(px(184.0))
        .h(px(CONTROL_HEIGHT))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if open { theme.accent } else { theme.border })
        .cursor_pointer()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            div()
                .child(CLEANUP_OPTIONS[selected].0)
                .text_size(px(13.0))
                .text_color(theme.text),
        )
        .child(icon(if open { "▴" } else { "▾" }, 12.0, theme.muted))
        .on_click(cx.listener(|app, _event, _window, cx| {
            app.toggle_combo("cleanup", cx);
        }));
    if !open {
        return trigger;
    }
    let mut menu = div()
        .id("cleanup")
        .w(px(184.0))
        .rounded(px(10.0))
        .bg(theme.surface_soft)
        .border_1()
        .border_color(theme.border_strong)
        .py(px(4.0))
        .flex()
        .flex_col();
    for (index, (label, _)) in CLEANUP_OPTIONS.iter().enumerate() {
        menu = menu.child(
            div()
                .id(SharedString::from(format!("cleanup-{index}")))
                .h(px(40.0))
                .px(px(10.0))
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(13.0))
                        .child(if index == selected { "✓" } else { "" })
                        .text_size(px(13.0))
                        .text_color(theme.accent_text),
                )
                .child(
                    div()
                        .child(*label)
                        .text_size(px(13.0))
                        .text_color(theme.text_soft),
                )
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.close_combo("cleanup", cx);
                    app.prepare.cleanup_index = index as i64;
                    app.set_setting(
                        CLEANUP_POLICY,
                        json!(CLEANUP_OPTIONS[index].1),
                        cx,
                    );
                    app.save_draft();
                    cx.notify();
                })),
        );
    }
    menu
}

fn caption_area(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    id: &'static str,
    placeholder: &'static str,
    value: &str,
) -> impl Element {
    // Show the committed value while not editing; once the user types, the
    // field holds the live text (and on_field_changed commits it).
    let display = if app.focused_field.as_deref() == Some(id) {
        app.fields.get(id).cloned().unwrap_or_default()
    } else {
        app.fields
            .get(id)
            .filter(|f| !f.text.is_empty())
            .cloned()
            .unwrap_or_else(|| FieldState {
                text: value.to_string(),
                caret: value.chars().count(),
                committed: true,
            })
    };
    crate::widgets::text_area(
        id,
        placeholder,
        92.0,
        &display,
        app.focused_field.as_deref() == Some(id),
        cx,
    )
}

impl crate::App {
    pub fn prepare_crop_preset(&self) -> usize {
        let crop = self.prepare.crop;
        let full_frame =
            (crop.width - 1.0).abs() < 0.001 && (crop.height - 1.0).abs() < 0.001;
        if !self.prepare.crop_enabled || full_frame {
            1
        } else if (crop.width - crop.height).abs() < 0.001 {
            2
        } else if crop.width / crop.height.max(0.001) > 1.5 {
            3
        } else if crop.height / crop.width.max(0.001) > 1.5 {
            4
        } else {
            0
        }
    }

    pub fn apply_crop_preset(&mut self, index: usize) {
        let source_ratio = self
            .selected
            .as_ref()
            .map(|m| {
                if m.width > 0 && m.height > 0 {
                    m.width as f64 / m.height as f64
                } else {
                    16.0 / 9.0
                }
            })
            .unwrap_or(16.0 / 9.0);
        match index {
            0 => {
                self.prepare.crop_enabled = true;
                if self.prepare.crop_is_original() {
                    self.prepare.crop = CropSpec {
                        x: 0.08,
                        y: 0.08,
                        width: 0.84,
                        height: 0.84,
                    };
                }
            }
            1 => {
                self.prepare.reset_crop();
            }
            2 => self.prepare.apply_crop_aspect(1.0, source_ratio),
            3 => self.prepare.apply_crop_aspect(16.0 / 9.0, source_ratio),
            4 => self.prepare.apply_crop_aspect(9.0 / 16.0, source_ratio),
            _ => {}
        }
    }
}

/// Format an integer with thousands separators (1,024), matching the
/// original's toLocaleString() counters.
pub fn group_digits(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod prepare_tests {
    use super::group_digits;

    #[test]
    fn group_digits_formats_thousands() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1000), "1,000");
        assert_eq!(group_digits(1024), "1,024");
        assert_eq!(group_digits(1234567), "1,234,567");
    }
}
