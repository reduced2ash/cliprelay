//! THESIS: Prepare is one release proof, not a miniature nonlinear editor.
//! OWN-WORLD: carbon pasteboard, warm proof stock, registration orange, slate notation.
//! STORY: review the footage, set the cut, then send from one continuous artifact.
//! FIRST VIEWPORT: library context remains visible beside an oversized video proof and one cut strip.
//! FORM: Release Flatplan, approved comp B, seed 0181ccb2.
//! FINISH: review and document every shipping raster and interaction state.
//!
//! Prepare workspace renderers: media proof, workflow folios, detailed Studio,
//! and delivery actions.

use crate::prepare::{DragHandle, ShapeKind};
use crate::prepare::{CLEANUP_OPTIONS, COMPRESSION_OPTIONS};
use crate::settings_import::*;
use crate::state::*;
use crate::theme::*;
use crate::video_element::video as video_element;
use crate::widgets::*;
use cliprelay_core::media::CropSpec;
use cliprelay_core::utils::format_bytes;
use gpui::*;
use serde_json::json;
use std::path::PathBuf;

const PREPARE_GUTTER: f32 = 10.0;
const PREPARE_CONTROL_HEIGHT: f32 = 44.0;
const PREPARE_SECTION_GAP: f32 = 10.0;

fn color_from_hex(hex: &str) -> Hsla {
    let hex = hex.trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).unwrap_or(0);
    let r = ((value >> 16) & 0xFF) as f32 / 255.0;
    let g = ((value >> 8) & 0xFF) as f32 / 255.0;
    let b = (value & 0xFF) as f32 / 255.0;
    Hsla::from(gpui::Rgba { r, g, b, a: 1.0 })
}

#[derive(Clone, Copy)]
pub(crate) struct PrepareFlatplan {
    pub carbon: Hsla,
    pub proof: Hsla,
    pub proof_raised: Hsla,
    pub ink: Hsla,
    pub ink_soft: Hsla,
    pub rule: Hsla,
    pub orange: Hsla,
    pub orange_pressed: Hsla,
    pub orange_soft: Hsla,
    pub slate: Hsla,
}

pub(crate) fn prepare_flatplan() -> PrepareFlatplan {
    PrepareFlatplan {
        carbon: color_from_hex("#101110"),
        proof: color_from_hex("#ECE2D3"),
        proof_raised: color_from_hex("#F6EEE2"),
        ink: color_from_hex("#151515"),
        ink_soft: color_from_hex("#514D47"),
        rule: color_from_hex("#6B655D"),
        orange: color_from_hex("#FD4E11"),
        orange_pressed: color_from_hex("#D83A00"),
        orange_soft: color_from_hex("#F4BEA8"),
        slate: color_from_hex("#085AA2"),
    }
}

pub(crate) fn prepare_flatplan_theme(base: &crate::theme::Theme) -> crate::theme::Theme {
    let palette = prepare_flatplan();
    let mut theme = base.clone();
    theme.ink = palette.proof;
    theme.surface = palette.proof;
    theme.surface_soft = color_from_hex("#E4D9C9");
    theme.raised = palette.proof_raised;
    theme.active = color_from_hex("#D8CBBB");
    theme.hover = color_from_hex("#E0D4C4");
    theme.text = palette.ink;
    theme.text_soft = palette.ink_soft;
    theme.muted = color_from_hex("#6A645C");
    theme.muted_soft = palette.ink_soft;
    theme.border = color_from_hex("#B7AB9B");
    theme.border_strong = palette.rule;
    theme.accent = palette.orange;
    theme.accent_pressed = palette.orange_pressed;
    theme.accent_soft = palette.orange_soft;
    theme.accent_text = color_from_hex("#B83200");
    theme.accent_content = palette.ink;
    theme
}

fn trim_target_left(x: f32, track_width: f32, is_in: bool) -> f32 {
    if is_in {
        x.clamp(0.0, track_width - PREPARE_CONTROL_HEIGHT)
    } else {
        (x - PREPARE_CONTROL_HEIGHT).clamp(0.0, track_width - PREPARE_CONTROL_HEIGHT)
    }
}

fn trim_drag_for_pointer(
    pointer_x: f32,
    trim_start_x: f32,
    trim_end_x: f32,
    track_width: f32,
) -> DragHandle {
    let in_left = trim_target_left(trim_start_x, track_width, true);
    let out_left = trim_target_left(trim_end_x, track_width, false);
    let in_zone = (in_left..=in_left + PREPARE_CONTROL_HEIGHT).contains(&pointer_x);
    let out_zone = (out_left..=out_left + PREPARE_CONTROL_HEIGHT).contains(&pointer_x);
    match (in_zone, out_zone) {
        (true, true) => {
            if (pointer_x - trim_start_x).abs() <= (pointer_x - trim_end_x).abs() {
                DragHandle::TrimIn
            } else {
                DragHandle::TrimOut
            }
        }
        (true, false) => DragHandle::TrimIn,
        (false, true) => DragHandle::TrimOut,
        (false, false) => DragHandle::Seek,
    }
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
        let (play_enabled, seek_enabled) =
            crate::prepare::playback_control_availability(self.prepare.video.is_some(), duration);
        let track_width = (panel_width - PREPARE_GUTTER * 2.0).max(100.0);
        let is_studio = self.prepare.studio_mode;
        let track_height = if is_studio { 58.0 } else { 52.0 };
        // Track origin in window coordinates.
        let track_left = if is_studio {
            // The body (sidebar + divider) precedes the studio stage.
            let collapsed = self.sidebar_collapsed || self.window_size.0 < 1080.0;
            let sidebar = if collapsed { 68.0 } else { 204.0 };
            sidebar + 1.0 + PREPARE_GUTTER
        } else {
            self.window_size.0 - panel_width + PREPARE_GUTTER
        };

        let mut stage = div()
            .id("prepare-stage")
            .w_full()
            .flex()
            .flex_col()
            .px(px(PREPARE_GUTTER))
            .py(px(10.0))
            .gap(px(8.0))
            .bg(theme.ink);
        stage.interactivity().on_mouse_move(cx.listener(
            move |app, event: &MouseMoveEvent, _window, cx| {
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                app.prepare_drag_move(x, y, track_left, track_width, duration, cx);
            },
        ));
        stage.interactivity().on_mouse_up(
            MouseButton::Left,
            cx.listener(|app, _event: &MouseUpEvent, _window, cx| {
                app.prepare.drag = DragHandle::None;
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
        let minimum_frame_height = if is_studio {
            if self.checking {
                56.0
            } else {
                90.0
            }
        } else {
            170.0
        };
        let available_height = if is_studio {
            let checking_allowance = if self.checking { 464.0 } else { 430.0 };
            (self.window_size.1 - checking_allowance).clamp(minimum_frame_height, 680.0)
        } else {
            // Controls grow to accessible targets; the proof yields first so
            // the active workflow and next action remain in the first view.
            (self.window_size.1 - 590.0).clamp(170.0, 400.0)
        };
        let frame_height =
            (track_width / source_ratio).clamp(minimum_frame_height, available_height);
        // Frame bounds in window coordinates for crop and mask drag math.
        let frame_x = track_left;
        let frame_top = 40.0
            + 42.0
            + if is_studio {
                52.0 + if self.checking { 34.0 } else { 0.0 }
            } else if self.checking {
                34.0
            } else {
                0.0
            }
            + 10.0;
        self.prepare.frame_rect = (frame_x, frame_top, track_width, frame_height);
        let has_edits = self.prepare.has_edits();
        let mut frame = div()
            .id("prepare-frame")
            .w(px(track_width))
            .h(px(frame_height))
            .rounded(px(0.0))
            .bg(color_from_hex("#05070B"))
            .border_1()
            .border_color(if has_edits {
                theme.accent
            } else {
                theme.border_strong
            })
            .overflow_hidden()
            .relative();
        let image_source = (!thumbnail.is_empty()).then(|| PathBuf::from(thumbnail));
        if let Some(video) = self.prepare.video.clone() {
            frame = frame.child(video_element(
                video,
                "prepare-video",
                px(track_width),
                px(frame_height),
            ));
        } else if let Some(source) = image_source {
            frame = frame.child(img(source).w_full().h_full().object_fit(ObjectFit::Contain));
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
        if self.prepare_video_loading || self.prepare_video_error.is_some() {
            let message = self
                .prepare_video_error
                .as_deref()
                .unwrap_or("Loading video…")
                .to_string();
            frame = frame.child(
                div()
                    .absolute()
                    .bottom(px(10.0))
                    .left(px(10.0))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(0.0))
                    .bg(theme.media_overlay)
                    .text_size(px(11.0))
                    .text_color(theme.media_text)
                    .child(message),
            );
        }
        // Edit overlays (crop + masks).
        frame = frame.child(self.render_edit_overlays(cx, theme, track_width, frame_height));
        stage = stage.child(frame);

        // Transport row (precise centiseconds like the original).
        let time_label = self.prepare.format_time_precise(position);
        let duration_label = self.prepare.format_time_precise(duration);
        let compact_transport = panel_width < 400.0;
        let transport_time_width = if compact_transport { 54.0 } else { 70.0 };
        stage = stage.child(
            div()
                .id("transport")
                .w_full()
                .h(px(PREPARE_CONTROL_HEIGHT))
                .px(px(0.0))
                .border_b_1()
                .border_color(theme.border_strong)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(tabular(
                    div()
                        .w(px(transport_time_width))
                        .child(time_label)
                        .text_size(px(12.0))
                        .text_color(theme.text_soft),
                ))
                .child(div().flex_1())
                .child(
                    workbench_button(
                        "back-5",
                        "−5",
                        "",
                        ButtonKind::Ghost,
                        seek_enabled,
                        false,
                        "Back 5 seconds",
                        cx,
                        |app, cx| {
                            app.prepare
                                .seek(app.prepare.position - 5.0, app.prepare.duration);
                            cx.notify();
                        },
                    )
                    .h(px(PREPARE_CONTROL_HEIGHT))
                    .opacity(if seek_enabled { 1.0 } else { 0.65 }),
                )
                .child(
                    workbench_button(
                        "play-pause",
                        if playing { "Pause" } else { "Play" },
                        "",
                        ButtonKind::Secondary,
                        play_enabled,
                        false,
                        if playing {
                            "Pause  ·  Space"
                        } else {
                            "Play  ·  Space"
                        },
                        cx,
                        |app, cx| {
                            app.prepare.toggle_playback();
                            app.save_draft();
                            cx.notify();
                        },
                    )
                    .h(px(PREPARE_CONTROL_HEIGHT))
                    .opacity(if play_enabled { 1.0 } else { 0.65 }),
                )
                .child(
                    workbench_button(
                        "forward-5",
                        "+5",
                        "",
                        ButtonKind::Ghost,
                        seek_enabled,
                        false,
                        "Forward 5 seconds",
                        cx,
                        |app, cx| {
                            app.prepare
                                .seek(app.prepare.position + 5.0, app.prepare.duration);
                            cx.notify();
                        },
                    )
                    .h(px(PREPARE_CONTROL_HEIGHT))
                    .opacity(if seek_enabled { 1.0 } else { 0.65 }),
                )
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
                .child(if compact_transport {
                    div().into_any()
                } else {
                    tabular(
                        div()
                            .w(px(transport_time_width))
                            .child(duration_label)
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .text_right(),
                    )
                    .into_any()
                }),
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
        let trim_start_x = (start_fraction as f32) * track_width;
        let trim_end_x = (end_fraction as f32) * track_width;
        let mut track = div()
            .id("timeline-track")
            .w(px(track_width))
            .h(px(track_height))
            .rounded(px(0.0))
            .bg(color_from_hex("#05070B"))
            .border_1()
            .border_color(if cut_active {
                theme.accent
            } else {
                theme.border_strong
            })
            .overflow_hidden()
            .relative()
            .opacity(if disabled { 0.55 } else { 1.0 });
        track.interactivity().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |app, event: &MouseDownEvent, _window, cx| {
                if !app.checking {
                    let pointer_x: f32 = event.position.x.into();
                    let local_x = (pointer_x - track_left).clamp(0.0, track_width);
                    let drag =
                        trim_drag_for_pointer(local_x, trim_start_x, trim_end_x, track_width);
                    let value = match drag {
                        DragHandle::TrimIn => app.prepare.trim_start,
                        DragHandle::TrimOut => app.prepare.trim_end,
                        _ => local_x as f64 / track_width as f64 * duration,
                    };
                    app.prepare.drag = drag;
                    app.prepare.drag_start_x = pointer_x as f64;
                    app.prepare.drag_start_value = value;
                    if drag == DragHandle::Seek {
                        app.prepare.seek(value, duration);
                    }
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
                    .bg(Hsla {
                        h: 0.0,
                        s: 0.0,
                        l: 0.02,
                        a: 0.7,
                    }),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .h_full()
                    .w(px(((1.0 - end_fraction) as f32) * track_width))
                    .bg(Hsla {
                        h: 0.0,
                        s: 0.0,
                        l: 0.02,
                        a: 0.7,
                    }),
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
        track =
            track
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .h_full()
                        .w(px(2.0))
                        .left(px(((play_fraction as f32) * track_width - 1.0)
                            .clamp(0.0, track_width - 2.0)))
                        .bg(Hsla {
                            h: 0.55,
                            s: 0.0,
                            l: 0.97,
                            a: 1.0,
                        }),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(((play_fraction as f32) * track_width - 3.5)
                            .clamp(0.0, track_width - 7.0)))
                        .w(px(7.0))
                        .h(px(7.0))
                        .rounded(px(4.0))
                        .bg(Hsla {
                            h: 0.55,
                            s: 0.0,
                            l: 0.97,
                            a: 1.0,
                        }),
                );
        // Trim handles. Pointer dispatch happens on the track so overlapping
        // 44px targets select the nearest actual trim boundary.
        track = track
            .child(self.trim_handle(
                cx,
                "trim-in",
                trim_start_x,
                track_height,
                true,
                DragHandle::TrimIn,
                track_width,
                duration,
            ))
            .child(self.trim_handle(
                cx,
                "trim-out",
                trim_end_x,
                track_height,
                false,
                DragHandle::TrimOut,
                track_width,
                duration,
            ));
        stage = stage.child(track);

        // Tick labels are detail for a wide editing surface, not a
        // permanent second ruler in the compact dock.
        if panel_width > 620.0 {
            let mut ticks = div().w(px(track_width)).h(px(14.0)).relative().flex_none();
            for index in 0..5 {
                let fraction = index as f32 / 4.0;
                let seconds = duration * fraction as f64;
                let label = self.prepare.format_time(seconds);
                ticks = ticks.child(tabular(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(
                            (fraction * track_width - 16.0).clamp(0.0, track_width - 32.0)
                        ))
                        .w(px(32.0))
                        .child(label)
                        .text_size(px(10.0))
                        .text_color(theme.muted_soft),
                ));
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
        let compact_precision = panel_width < 400.0;
        let input_width = 90.0;
        let precision_label_width = 24.0;
        let in_state =
            self.fields
                .get("prepare-in")
                .cloned()
                .unwrap_or_else(|| crate::widgets::FieldState {
                    text: in_text.clone(),
                    caret: in_text.chars().count(),
                    committed: false,
                    marked_range: None,
                });
        let out_state =
            self.fields
                .get("prepare-out")
                .cloned()
                .unwrap_or_else(|| crate::widgets::FieldState {
                    text: out_text.clone(),
                    caret: out_text.chars().count(),
                    committed: false,
                    marked_range: None,
                });
        let disabled_time_field = |text: String| {
            div()
                .flex_none()
                .w(px(input_width))
                .h(px(PREPARE_CONTROL_HEIGHT))
                .px(px(13.0))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.raised)
                .flex()
                .items_center()
                .child(text)
                .text_size(px(13.0))
                .text_color(theme.muted)
                .into_any()
        };
        let in_field = if disabled {
            disabled_time_field(in_text)
        } else {
            field_in_theme(
                "prepare-in",
                "00:00.00",
                &in_state,
                self.focused_field.as_deref() == Some("prepare-in"),
                true,
                false,
                theme,
                cx,
            )
            .flex_none()
            .w(px(input_width))
            .h(px(PREPARE_CONTROL_HEIGHT))
            .rounded(px(0.0))
            .bg(theme.raised)
            .border_color(theme.border_strong)
            .into_any()
        };
        let out_field = if disabled {
            disabled_time_field(out_text)
        } else {
            field_in_theme(
                "prepare-out",
                "00:00.00",
                &out_state,
                self.focused_field.as_deref() == Some("prepare-out"),
                true,
                false,
                theme,
                cx,
            )
            .flex_none()
            .w(px(input_width))
            .h(px(PREPARE_CONTROL_HEIGHT))
            .rounded(px(0.0))
            .bg(theme.raised)
            .border_color(theme.border_strong)
            .into_any()
        };
        let mut precision = div()
            .w_full()
            .min_w(px(0.0))
            .h(px(PREPARE_CONTROL_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            .child(
                div()
                    .flex_none()
                    .w(px(precision_label_width))
                    .child("IN")
                    .text_size(px(10.0))
                    .text_color(theme.muted)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(in_field)
            .child(
                div()
                    .flex_none()
                    .w(px(precision_label_width))
                    .child("OUT")
                    .text_size(px(10.0))
                    .text_color(theme.muted)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(out_field);
        let mut trailing =
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(if cut_active {
                    tabular(
                        div()
                            .child(format!(
                                "CUT  {}",
                                self.prepare.format_time_precise(trim_end - trim_start)
                            ))
                            .text_size(px(12.0))
                            .text_color(theme.accent_text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .into_any()
                } else {
                    tabular(
                        div()
                            .child(format!(
                                "FULL  {}",
                                self.prepare.format_time_precise(duration)
                            ))
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .into_any()
                });
        if cut_active {
            trailing = trailing.child(workbench_button(
                "reset-cut",
                "",
                "refresh",
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
            ));
        }
        if compact_precision {
            stage = stage
                .child(precision)
                .child(div().w_full().flex().justify_end().child(trailing));
        } else {
            precision = precision.child(div().flex_1()).child(trailing);
            stage = stage.child(precision);
        }

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
        let source_tooltip = path.clone();
        let compact_source = panel_width < 420.0;
        stage = stage.child(
            div()
                .w_full()
                .h(px(PREPARE_CONTROL_HEIGHT))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("prepare-source-name")
                        .flex_1()
                        .min_w(px(40.0))
                        .child(name)
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::MEDIUM)
                        .text_ellipsis()
                        .tooltip(move |_window, cx| {
                            crate::tooltip_view(cx, source_tooltip.clone().into())
                        }),
                )
                .child(if self.selected.is_some() {
                    let mut parts = vec![size_label];
                    if !compact_source {
                        parts.push(self.prepare.format_time(duration));
                        if !resolution_label.is_empty() {
                            parts.push(resolution_label);
                        }
                    }
                    div()
                        .flex_none()
                        .child(parts.join("  ·  "))
                        .text_size(px(11.0))
                        .text_color(theme.muted)
                        .text_ellipsis()
                } else {
                    div()
                })
                .child(
                    workbench_button(
                        "reveal-in-library",
                        if compact_source { "" } else { "Reveal" },
                        "↗",
                        ButtonKind::Ghost,
                        true,
                        compact_source,
                        "Reveal in library",
                        cx,
                        |app, cx| {
                            app.command(Command::RevealSelectedInLibrary);
                            cx.notify();
                        },
                    )
                    .h(px(PREPARE_CONTROL_HEIGHT)),
                ),
        );
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
        track_width: f32,
        duration: f64,
    ) -> impl Element {
        let theme = self.theme.clone();
        let label: SharedString = if is_in {
            format!(
                "IN  {}",
                self.prepare.format_time_precise(self.prepare.trim_start)
            )
            .into()
        } else {
            format!(
                "OUT  {}",
                self.prepare.format_time_precise(self.prepare.trim_end)
            )
            .into()
        };
        let bar_color = if self.prepare.drag == handle {
            theme.accent_pressed
        } else {
            theme.accent
        };
        let target_left = trim_target_left(x, track_width, is_in);
        let bar_left = if is_in {
            x - target_left
        } else {
            x - target_left - 10.0
        }
        .clamp(0.0, 34.0);
        let element = div()
            .id(id)
            .absolute()
            .top_0()
            .left(px(target_left))
            .w(px(44.0))
            .h(px(track_height))
            .cursor_ew_resize()
            .tab_index(0)
            .focus(|style| style.border_2().border_color(prepare_flatplan().slate))
            .tooltip(move |_window, cx| crate::tooltip_view(cx, label.clone()))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(bar_left))
                    .w(px(10.0))
                    .h_full()
                    .rounded(px(3.0))
                    .bg(bar_color)
                    .border_1()
                    .border_color(theme.accent_content)
                    .hover(|style| style.bg(theme.accent_pressed)),
            )
            .on_key_down(cx.listener(move |app, event: &KeyDownEvent, _window, cx| {
                let direction = match event.keystroke.key.as_str() {
                    "left" => -1.0,
                    "right" => 1.0,
                    _ => 0.0,
                };
                if direction != 0.0 {
                    let step = if event.keystroke.modifiers.shift {
                        1.0
                    } else {
                        0.05
                    };
                    if is_in {
                        app.prepare.trim_start = (app.prepare.trim_start + direction * step)
                            .clamp(0.0, (app.prepare.trim_end - 0.05).max(0.0));
                        app.prepare
                            .seek(app.prepare.trim_start, app.prepare.duration);
                    } else {
                        app.prepare.trim_end = (app.prepare.trim_end + direction * step)
                            .clamp((app.prepare.trim_start + 0.05).min(duration), duration);
                    }
                    app.save_draft();
                    cx.notify();
                    cx.stop_propagation();
                }
            }));
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
                let seconds = (self.prepare.drag_start_value
                    + delta as f64 / track_width as f64 * duration)
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
    fn start_edit_drag(&mut self, handle: DragHandle, pointer_x: f32, pointer_y: f32) {
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

    fn nudge_crop_keyboard(&mut self, key: &str, resize: bool, cx: &mut Context<Self>) -> bool {
        let step = 0.01;
        let mut crop = self.prepare.crop;
        let handled = if resize {
            match key {
                "left" => {
                    crop.width = (crop.width - step).max(0.04);
                    true
                }
                "right" => {
                    crop.width = (crop.width + step).min(1.0 - crop.x);
                    true
                }
                "up" => {
                    crop.height = (crop.height - step).max(0.04);
                    true
                }
                "down" => {
                    crop.height = (crop.height + step).min(1.0 - crop.y);
                    true
                }
                _ => false,
            }
        } else {
            match key {
                "left" => {
                    crop.x = (crop.x - step).max(0.0);
                    true
                }
                "right" => {
                    crop.x = (crop.x + step).min(1.0 - crop.width);
                    true
                }
                "up" => {
                    crop.y = (crop.y - step).max(0.0);
                    true
                }
                "down" => {
                    crop.y = (crop.y + step).min(1.0 - crop.height);
                    true
                }
                _ => false,
            }
        };
        if handled {
            self.prepare.crop = crop;
            self.save_draft();
            cx.notify();
        }
        handled
    }

    fn nudge_mask_keyboard(
        &mut self,
        index: usize,
        key: &str,
        resize: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let step = 0.01;
        let Some(shape) = self.prepare.shapes.get_mut(index) else {
            return false;
        };
        let handled = if resize {
            match key {
                "left" => {
                    shape.width = (shape.width - step).max(0.025);
                    true
                }
                "right" => {
                    shape.width = (shape.width + step).min(1.0 - shape.x);
                    true
                }
                "up" => {
                    shape.height = (shape.height - step).max(0.025);
                    true
                }
                "down" => {
                    shape.height = (shape.height + step).min(1.0 - shape.y);
                    true
                }
                _ => false,
            }
        } else {
            match key {
                "left" => {
                    shape.x = (shape.x - step).max(0.0);
                    true
                }
                "right" => {
                    shape.x = (shape.x + step).min(1.0 - shape.width);
                    true
                }
                "up" => {
                    shape.y = (shape.y - step).max(0.0);
                    true
                }
                "down" => {
                    shape.y = (shape.y + step).min(1.0 - shape.height);
                    true
                }
                _ => false,
            }
        };
        if handled {
            self.prepare.selected_shape = Some(index);
            self.save_draft();
            cx.notify();
        }
        handled
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
        let mut overlays = div()
            .id("edit-overlays")
            .absolute()
            .top_0()
            .left_0()
            .size_full();
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
                            .bg(Hsla {
                                h: 0.0,
                                s: 0.0,
                                l: 0.02,
                                a: 0.6,
                            }),
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
                            .bg(Hsla {
                                h: 0.0,
                                s: 0.0,
                                l: 0.02,
                                a: 0.6,
                            }),
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
                .cursor_move()
                .tab_index(0)
                .focus(|style| style.border_2().border_color(prepare_flatplan().slate))
                .tooltip(|_window, cx| {
                    crate::tooltip_view(cx, "Move crop · Shift+Arrow resizes".into())
                })
                .on_key_down(cx.listener(|app, event: &KeyDownEvent, _window, cx| {
                    if app.nudge_crop_keyboard(
                        event.keystroke.key.as_str(),
                        event.keystroke.modifiers.shift,
                        cx,
                    ) {
                        cx.stop_propagation();
                    }
                }));
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
                .bg(Hsla {
                    h: 0.0,
                    s: 0.0,
                    l: 0.01,
                    a: 0.85,
                })
                .border_2()
                .border_color(if selected {
                    theme.accent
                } else {
                    theme.border_strong
                })
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
        let theme = prepare_flatplan_theme(&self.theme);
        let palette = prepare_flatplan();
        let active_tab = self.prepare.inspector_tab;
        let edits = self.prepare.has_edits();
        let review_status = if self.checking { "CHECKING" } else { "READY" };
        let trim_status = if active_tab == 0 {
            "ACTIVE"
        } else if edits {
            "EDITED"
        } else {
            "CLEAN"
        };
        let send_status = if self.publish.active {
            "WORKING"
        } else if !self.publish.error.is_empty() {
            "RESULT"
        } else if self.bot_connected() || self.personal_configured() {
            "READY"
        } else {
            "SETUP"
        };

        let review = div()
            .id("phase-review")
            .w_full()
            .h(px(42.0))
            .px(px(12.0))
            .border_t_1()
            .border_color(theme.border_strong)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .w(px(28.0))
                    .child("01")
                    .text_size(px(18.0))
                    .text_color(theme.text)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                div()
                    .child("REVIEW")
                    .text_size(px(12.0))
                    .text_color(theme.text)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(div().flex_1())
            .child(
                div()
                    .child(review_status)
                    .text_size(px(10.0))
                    .text_color(if self.checking {
                        theme.accent_text
                    } else {
                        theme.muted
                    })
                    .font_weight(FontWeight::SEMIBOLD),
            );

        let mut trim = div()
            .id("phase-trim")
            .w_full()
            .h(px(44.0))
            .px(px(12.0))
            .border_t_1()
            .border_color(theme.border_strong)
            .bg(if active_tab == 0 {
                theme.raised
            } else {
                theme.surface
            })
            .cursor_pointer()
            .tab_index(0)
            .focus(|style| style.border_2().border_color(palette.slate))
            .hover(|style| style.bg(theme.hover))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .w(px(28.0))
                    .child("02")
                    .text_size(px(18.0))
                    .text_color(if active_tab == 0 {
                        theme.accent
                    } else {
                        theme.text
                    })
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                div()
                    .child("TRIM")
                    .text_size(px(12.0))
                    .text_color(theme.text)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(div().flex_1())
            .child(
                div()
                    .child(trim_status)
                    .text_size(px(10.0))
                    .text_color(if active_tab == 0 {
                        theme.accent_text
                    } else {
                        theme.muted
                    })
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .on_click(cx.listener(|app, _event, _window, cx| {
                app.prepare.inspector_tab = 0;
                app.save_draft();
                cx.notify();
            }));
        trim = trim
            .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
                app.prepare.inspector_tab = 0;
                app.save_draft();
                cx.notify();
            }))
            .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
                app.prepare.inspector_tab = 0;
                app.save_draft();
                cx.notify();
            }));

        let mut send = div()
            .id("phase-send")
            .w_full()
            .h(px(44.0))
            .px(px(12.0))
            .border_t_1()
            .border_b_1()
            .border_color(theme.border_strong)
            .bg(if active_tab == 1 {
                theme.raised
            } else {
                theme.surface
            })
            .cursor_pointer()
            .tab_index(0)
            .focus(|style| style.border_2().border_color(palette.slate))
            .hover(|style| style.bg(theme.hover))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .w(px(28.0))
                    .child("03")
                    .text_size(px(18.0))
                    .text_color(if active_tab == 1 {
                        theme.accent
                    } else {
                        theme.text
                    })
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                div()
                    .child("SEND")
                    .text_size(px(12.0))
                    .text_color(theme.text)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(div().flex_1())
            .child(
                div()
                    .child(send_status)
                    .text_size(px(10.0))
                    .text_color(if active_tab == 1 {
                        theme.accent_text
                    } else {
                        theme.muted
                    })
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .on_click(cx.listener(|app, _event, _window, cx| {
                app.prepare.inspector_tab = 1;
                app.save_draft();
                cx.notify();
            }));
        send = send
            .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
                app.prepare.inspector_tab = 1;
                app.save_draft();
                cx.notify();
            }))
            .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
                app.prepare.inspector_tab = 1;
                app.save_draft();
                cx.notify();
            }));

        div()
            .id("prepare-phases")
            .w_full()
            .bg(theme.surface)
            .flex()
            .flex_col()
            .child(review)
            .child(trim)
            .child(send)
    }

    pub fn render_prepare_dock_phase_body(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
    ) -> impl Element {
        let cut_duration = (self.prepare.trim_end - self.prepare.trim_start).max(0.0);
        let cut_status = if self.prepare.cut_active() {
            "CUT SET"
        } else {
            "FULL SOURCE"
        };
        let mut body = div()
            .id("prepare-dock-phase-body")
            .w_full()
            .px(px(12.0))
            .py(px(10.0))
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border_strong)
            .flex()
            .flex_col()
            .gap(px(8.0));

        if self.checking {
            body = body
                .child(
                    div()
                        .child("REVIEWING SOURCE")
                        .text_size(px(11.0))
                        .text_color(theme.accent_text)
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    div()
                        .child("Edit and delivery actions unlock when the selected file check completes.")
                        .text_size(px(11.0))
                        .text_color(theme.text_soft),
                )
                .child(progress_bar(0.0, true));
        } else {
            body = body
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
                                .child(format!(
                                    "CUT  {}",
                                    self.prepare.format_time_precise(cut_duration)
                                ))
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::SEMIBOLD),
                        )
                        .child(
                            div()
                                .child(cut_status)
                                .text_size(px(10.0))
                                .text_color(if self.prepare.cut_active() {
                                    theme.accent_text
                                } else {
                                    theme.muted
                                })
                                .font_weight(FontWeight::SEMIBOLD),
                        ),
                )
                .child(
                    div()
                        .child(format!(
                            "IN {}  ·  OUT {}",
                            self.prepare
                                .format_time_precise(self.prepare.trim_start),
                            self.prepare.format_time_precise(self.prepare.trim_end)
                        ))
                        .text_size(px(11.0))
                        .text_color(theme.text_soft),
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
                                .child("Crop, masks, captions, and delivery limits continue in Studio.")
                                .text_size(px(11.0))
                                .text_color(theme.muted),
                        )
                        .child(
                            workbench_button(
                                "dock-open-studio-detail",
                                "Studio",
                                "",
                                ButtonKind::Ghost,
                                true,
                                false,
                                "Open detailed Prepare tools",
                                cx,
                                |app, cx| {
                                    app.prepare.studio_mode = true;
                                    cx.notify();
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT))
                            .rounded(px(0.0))
                            .border_1()
                            .border_color(theme.border_strong)
                            .text_color(theme.text),
                        ),
                );
        }
        body
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
        let checking = self.checking;
        let content = if checking {
            div()
                .w_full()
                .py(px(18.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .child("REVIEWING SOURCE")
                        .text_size(px(11.0))
                        .text_color(theme.accent_text)
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    div()
                        .child("Edit and delivery controls unlock after the selected file check completes.")
                        .text_size(px(12.0))
                        .text_color(theme.text_soft),
                )
                .child(progress_bar(0.0, true))
                .into_any()
        } else if self.prepare.inspector_tab == 0 {
            self.render_edit_inspector(cx, theme, panel_width)
                .into_any()
        } else {
            self.render_publish_inspector(cx, theme, panel_width)
                .into_any()
        };
        let scroll = div()
            .id("inspector-scroll")
            .flex_1()
            .min_h(px(0.0))
            .overflow_scroll()
            .scrollbar_width(px(8.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .px(px(PREPARE_GUTTER))
                    .pt(px(10.0))
                    .pb(px(12.0))
                    .child(content),
            );
        column = column.child(scroll);
        if self.prepare.inspector_tab == 1 && !checking {
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
        let mut column = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(PREPARE_SECTION_GAP));

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
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .child(tracked("FRAME"))
                                    .text_size(px(11.0))
                                    .text_color(theme.muted)
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(
                                div()
                                    .child("Crop the visible frame without changing the source.")
                                    .text_size(px(11.0))
                                    .text_color(theme.text_soft)
                                    .text_ellipsis(),
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
            .child(
                checkbox(
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
                )
                .text_color(theme.text),
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
                            .child("Crop aspect")
                            .text_size(px(12.0))
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
                .h(px(PREPARE_CONTROL_HEIGHT))
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
                            .child(tracked("BLACK MASKS"))
                            .text_size(px(11.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(if shape_count > 0 {
                        status_pill(
                            "pill-masks",
                            &format!("{shape_count} masks"),
                            PillState::Neutral,
                        )
                        .into_any()
                    } else {
                        div().into_any()
                    }),
            )
            .child(
                div()
                    .child("Select a mask; arrows move it and Shift+arrows resize.")
                    .text_size(px(11.0))
                    .text_color(theme.text_soft)
                    .text_ellipsis(),
            )
            .child(
                div()
                    .w_full()
                    .min_w(px(0.0))
                    .flex()
                    .flex_row()
                    .gap(px(6.0))
                    .child(
                        button(
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
                        )
                        .h(px(PREPARE_CONTROL_HEIGHT))
                        .flex_1()
                        .min_w(px(0.0))
                        .px(px(8.0)),
                    )
                    .child(
                        button(
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
                        )
                        .h(px(PREPARE_CONTROL_HEIGHT))
                        .flex_1()
                        .min_w(px(0.0))
                        .px(px(8.0)),
                    ),
            );

        // Mask list.
        if self.prepare.shapes.is_empty() {
            column = column.child(
                div()
                    .w_full()
                    .py(px(6.0))
                    .child("No masks")
                    .text_size(px(12.0))
                    .text_color(theme.muted_soft),
            );
        } else {
            let mut list = div()
                .id("mask-list")
                .w_full()
                .rounded(px(2.0))
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
                        .h(px(PREPARE_CONTROL_HEIGHT))
                        .px(px(9.0))
                        .cursor_pointer()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .bg(if is_selected {
                            theme.active
                        } else {
                            theme.transparent()
                        })
                        .tab_index(0)
                        .focus(|style| style.border_2().border_color(prepare_flatplan().slate))
                        .tooltip(|_window, cx| {
                            crate::tooltip_view(
                                cx,
                                "Select mask · Arrow keys move · Shift+Arrow resizes".into(),
                            )
                        })
                        .child(icon(
                            if shape.kind == ShapeKind::Rectangle {
                                "▭"
                            } else {
                                "□"
                            },
                            14.0,
                            if is_selected {
                                theme.accent_text
                            } else {
                                theme.muted
                            },
                        ))
                        .child(
                            div()
                                .flex_1()
                                .child(label)
                                .text_size(px(12.0))
                                .text_color(if is_selected {
                                    theme.text
                                } else {
                                    theme.text_soft
                                })
                                .font_weight(if is_selected {
                                    FontWeight::SEMIBOLD
                                } else {
                                    FontWeight::MEDIUM
                                }),
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
                        .on_key_down(cx.listener(move |app, event: &KeyDownEvent, _window, cx| {
                            if app.nudge_mask_keyboard(
                                index,
                                event.keystroke.key.as_str(),
                                event.keystroke.modifiers.shift,
                                cx,
                            ) {
                                cx.stop_propagation();
                            }
                        }))
                        .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                            app.prepare.selected_shape = Some(index);
                            cx.notify();
                        }))
                        .on_click(cx.listener(move |app, _event, _window, cx| {
                            app.prepare.selected_shape = Some(index);
                            cx.notify();
                        })),
                );
            }
            column = column.child(list);
            if self.prepare.shapes.len() > 1 {
                column = column.child(
                    button(
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
                    )
                    .h(px(PREPARE_CONTROL_HEIGHT)),
                );
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
        let connected = if mode_index == 1 {
            personal_ready
        } else {
            bot_ready
        };
        let destination = self.prepare.destination.clone();
        let destination_ready = !destination.trim().is_empty();
        let telegram_ready = connected && destination_ready;
        let x_duration_warning = {
            let limit = self
                .settings
                .get(X_DURATION_SECONDS)
                .and_then(|v| v.as_i64())
                .unwrap_or(140) as f64;
            self.prepare.duration > 0.0 && (self.prepare.trim_end - self.prepare.trim_start) > limit
        };

        let mut column = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(PREPARE_SECTION_GAP));

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
                                    .child(tracked("OUTPUT"))
                                    .text_size(px(11.0))
                                    .text_color(theme.muted)
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(
                                div()
                                    .child("Choose a destination-aware generated copy.")
                                    .text_size(px(11.0))
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
                        field_in_theme(
                            "target-mb",
                            "MB",
                            &self.fields.get("target-mb").cloned().unwrap_or_default(),
                            self.focused_field.as_deref() == Some("target-mb"),
                            true,
                            false,
                            theme,
                            cx,
                        )
                        .w(px(112.0))
                        .h(px(PREPARE_CONTROL_HEIGHT)),
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
                    .child(tracked("DESTINATIONS"))
                    .text_size(px(11.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon(
                        "➤",
                        16.0,
                        if telegram_ready {
                            theme.success
                        } else {
                            theme.muted
                        },
                    ))
                    .child(
                        div()
                            .child("Telegram")
                            .text_size(px(13.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(div().flex_1())
                    .child(status_pill(
                        "pill-tg",
                        if telegram_ready {
                            "Ready"
                        } else {
                            "Needs setup"
                        },
                        if telegram_ready {
                            PillState::Success
                        } else {
                            PillState::Warning
                        },
                    )),
            )
            .child(mode_combo(self, cx, theme))
            .child(
                field_in_theme(
                    "tg-destination-field",
                    if mode_index == 1 {
                        "Username or chat ID"
                    } else {
                        "@channel or chat ID"
                    },
                    &self
                        .fields
                        .get("tg-destination-field")
                        .cloned()
                        .unwrap_or_default(),
                    self.focused_field.as_deref() == Some("tg-destination-field"),
                    true,
                    false,
                    theme,
                    cx,
                )
                .h(px(PREPARE_CONTROL_HEIGHT)),
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
                    .text_color(if telegram_ready {
                        theme.success
                    } else {
                        theme.warning
                    }),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon(
                        "𝕏",
                        16.0,
                        if x_duration_warning {
                            theme.warning
                        } else {
                            theme.text_soft
                        },
                    ))
                    .child(
                        div()
                            .child("X")
                            .text_size(px(13.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(div().flex_1())
                    .child(status_pill(
                        "pill-x",
                        if x_duration_warning {
                            "Check cut"
                        } else {
                            "Manual"
                        },
                        if x_duration_warning {
                            PillState::Warning
                        } else {
                            PillState::Neutral
                        },
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
                    .child(tracked("CAPTIONS"))
                    .text_size(px(11.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                checkbox(
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
                )
                .text_color(theme.text),
            );
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
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(caption_area(
                    self,
                    cx,
                    theme,
                    "caption-shared",
                    "Caption for Telegram and X",
                    &caption,
                ))
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
                        }),
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
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(caption_area(
                    self,
                    cx,
                    theme,
                    "caption-tg",
                    "Telegram message",
                    &caption,
                ))
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
                        .text_color(if caption_len > caption_limit {
                            theme.warning
                        } else {
                            theme.muted
                        }),
                )
                .child(
                    div()
                        .child("X")
                        .text_size(px(12.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(caption_area(
                    self,
                    cx,
                    theme,
                    "caption-x",
                    "X post text",
                    &x_caption,
                ))
                .child(
                    div()
                        .w_full()
                        .text_right()
                        .child(format!("X {} / 280", group_digits(x_len)))
                        .text_size(px(12.0))
                        .text_color(if x_len > 280 {
                            theme.warning
                        } else {
                            theme.muted
                        }),
                );
        }
        column = column.child(divider());

        // Generated copy.
        column = column
            .child(
                div()
                    .child("Generated copy")
                    .text_size(px(12.0))
                    .text_color(theme.text_soft)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                div()
                    .child("Cleanup never applies to the source.")
                    .text_size(px(11.0))
                    .text_color(theme.muted),
            )
            .child(cleanup_combo(self, cx, theme));
        column
    }

    pub(crate) fn render_action_dock(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
    ) -> impl Element {
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
            .px(px(PREPARE_GUTTER))
            .py(px(8.0))
            .flex()
            .flex_col()
            .gap(px(6.0));

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
                                .text_color(theme.text),
                        )
                        .child(
                            workbench_button(
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
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT)),
                        ),
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
                                .font_weight(FontWeight::SEMIBOLD),
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
                        .child(
                            button_in_theme(
                                "copy-video",
                                "Copy video",
                                ButtonKind::Secondary,
                                Some("⧉"),
                                true,
                                theme,
                                cx,
                                move |app, cx| {
                                    if cliprelay_core::x::XAssistant::copy_file(
                                        std::path::Path::new(&output_path_copy),
                                    )
                                    .is_ok()
                                    {
                                        app.toast(
                                            ToastKind::Success,
                                            "Video copied. Paste it into the X composer.",
                                        );
                                    }
                                    cx.notify();
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT)),
                        )
                        .child(
                            button_in_theme(
                                "drag-video",
                                "Drag video",
                                ButtonKind::Secondary,
                                Some("↘"),
                                true,
                                theme,
                                cx,
                                move |app, cx| {
                                    // Native drag-out has no GPUI equivalent;
                                    // place the file on the clipboard so it can
                                    // be pasted or dragged into the composer.
                                    if cliprelay_core::x::XAssistant::copy_file(
                                        std::path::Path::new(&output_path_drag),
                                    )
                                    .is_ok()
                                    {
                                        app.toast(
                                            ToastKind::Success,
                                            "Video copied. Paste it into the X composer.",
                                        );
                                    }
                                    cx.notify();
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT)),
                        )
                        .child(
                            button_in_theme(
                                "show-in-folder",
                                "Show in folder",
                                ButtonKind::Secondary,
                                Some("▤"),
                                true,
                                theme,
                                cx,
                                move |_app, cx| {
                                    let _ = cliprelay_core::x::XAssistant::reveal(
                                        std::path::Path::new(&output_path_reveal),
                                    );
                                    cx.notify();
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT)),
                        ),
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
                        .gap(px(6.0))
                        .child(
                            button_in_theme(
                                "prepare-x",
                                "Prepare X",
                                ButtonKind::Secondary,
                                Some("𝕏"),
                                true,
                                theme,
                                cx,
                                |app, cx| {
                                    app.submit_publish("x", cx);
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT))
                            .flex_1(),
                        )
                        .child(
                            button_in_theme(
                                "send-telegram",
                                "Send Telegram",
                                ButtonKind::Secondary,
                                Some("➤"),
                                telegram_ready,
                                theme,
                                cx,
                                |app, cx| {
                                    app.submit_publish("telegram", cx);
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT))
                            .flex_1(),
                        )
                        .child(
                            button_in_theme(
                                "send-both",
                                "Send + prepare X",
                                ButtonKind::Primary,
                                Some("⇄"),
                                telegram_ready,
                                theme,
                                cx,
                                |app, cx| {
                                    app.submit_publish("both", cx);
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT))
                            .rounded(px(0.0))
                            .bg(theme.accent)
                            .text_color(theme.accent_content)
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
                            .gap(px(6.0))
                            .child(
                                button_in_theme(
                                    "prepare-x-narrow",
                                    "Prepare X",
                                    ButtonKind::Secondary,
                                    Some("𝕏"),
                                    true,
                                    theme,
                                    cx,
                                    |app, cx| {
                                        app.submit_publish("x", cx);
                                    },
                                )
                                .h(px(PREPARE_CONTROL_HEIGHT)),
                            )
                            .child(
                                button_in_theme(
                                    "send-telegram-narrow",
                                    "Send Telegram",
                                    ButtonKind::Secondary,
                                    Some("➤"),
                                    telegram_ready,
                                    theme,
                                    cx,
                                    |app, cx| {
                                        app.submit_publish("telegram", cx);
                                    },
                                )
                                .h(px(PREPARE_CONTROL_HEIGHT)),
                            ),
                    )
                    .child(
                        button_in_theme(
                            "send-both-narrow",
                            "Send + prepare X",
                            ButtonKind::Primary,
                            Some("⇄"),
                            telegram_ready,
                            theme,
                            cx,
                            |app, cx| {
                                app.submit_publish("both", cx);
                            },
                        )
                        .h(px(PREPARE_CONTROL_HEIGHT))
                        .rounded(px(0.0))
                        .bg(theme.accent)
                        .text_color(theme.accent_content),
                    );
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
        if self.checking {
            self.toast(
                ToastKind::Info,
                "Publishing unlocks when the selected video is ready.",
            );
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
        if send_telegram && !self.telegram_ready() {
            self.toast(
                ToastKind::Warning,
                "Finish the selected Telegram destination setup before sending.",
            );
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
    // Disabled until the crop is enabled (mirrors the original's combo).
    let crop_enabled = app.prepare.crop_enabled;
    let mut trigger = div()
        .id("crop-aspect-trigger")
        .w(px(190.0))
        .h(px(PREPARE_CONTROL_HEIGHT))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if open { theme.accent } else { theme.border })
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
        .child(icon(if open { "▴" } else { "▾" }, 12.0, theme.muted));
    if crop_enabled {
        trigger = trigger
            .cursor_pointer()
            .on_click(cx.listener(|app, _event, _window, cx| {
                app.toggle_combo("crop-aspect", cx);
            }));
    } else {
        trigger = trigger.opacity(0.46).cursor_default();
    }
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
                .h(px(PREPARE_CONTROL_HEIGHT))
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
        .h(px(PREPARE_CONTROL_HEIGHT))
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
        menu =
            menu.child(
                div()
                    .id(SharedString::from(format!("compression-{index}")))
                    .h(px(PREPARE_CONTROL_HEIGHT))
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
                    .child(div().child(*label).text_size(px(13.0)).text_color(
                        if index == selected {
                            theme.text
                        } else {
                            theme.text_soft
                        },
                    ))
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
        .h(px(PREPARE_CONTROL_HEIGHT))
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
                .h(px(PREPARE_CONTROL_HEIGHT))
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
                    app.set_setting(
                        TELEGRAM_MODE,
                        json!(if index == 1 { "personal" } else { "bot" }),
                        cx,
                    );
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
        .h(px(PREPARE_CONTROL_HEIGHT))
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
                .h(px(PREPARE_CONTROL_HEIGHT))
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
                    app.set_setting(CLEANUP_POLICY, json!(CLEANUP_OPTIONS[index].1), cx);
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
    theme: &Theme,
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
                marked_range: None,
            })
    };
    crate::widgets::text_area(
        id,
        placeholder,
        76.0,
        &display,
        app.focused_field.as_deref() == Some(id),
        theme,
        cx,
    )
}

impl crate::App {
    pub fn prepare_crop_preset(&self) -> usize {
        let crop = self.prepare.crop;
        let full_frame = (crop.width - 1.0).abs() < 0.001 && (crop.height - 1.0).abs() < 0.001;
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
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod prepare_tests {
    use super::{group_digits, trim_drag_for_pointer, trim_target_left};
    use crate::prepare::DragHandle;

    #[test]
    fn group_digits_formats_thousands() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1000), "1,000");
        assert_eq!(group_digits(1024), "1,024");
        assert_eq!(group_digits(1234567), "1,234,567");
    }

    #[test]
    fn trim_targets_stay_inside_track_at_full_source_bounds() {
        assert_eq!(trim_target_left(0.0, 260.0, true), 0.0);
        assert_eq!(trim_target_left(260.0, 260.0, false), 216.0);
        assert!(trim_drag_for_pointer(30.0, 0.0, 260.0, 260.0) == DragHandle::TrimIn);
        assert!(trim_drag_for_pointer(230.0, 0.0, 260.0, 260.0) == DragHandle::TrimOut);
        assert!(trim_drag_for_pointer(130.0, 0.0, 260.0, 260.0) == DragHandle::Seek);
    }

    #[test]
    fn overlapping_trim_targets_dispatch_to_nearest_boundary() {
        assert!(trim_drag_for_pointer(105.0, 100.0, 120.0, 260.0) == DragHandle::TrimIn);
        assert!(trim_drag_for_pointer(115.0, 100.0, 120.0, 260.0) == DragHandle::TrimOut);
        assert!(trim_drag_for_pointer(110.0, 100.0, 120.0, 260.0) == DragHandle::TrimIn);
    }
}
