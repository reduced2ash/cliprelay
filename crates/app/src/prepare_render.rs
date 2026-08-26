//! Prepare workspace renderers: stage (frame + transport + timeline +
//! source strip), inspector tabs, edit/publish inspectors, action dock.

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
const PREPARE_CONTROL_HEIGHT: f32 = 32.0;
const PREPARE_SECTION_GAP: f32 = 10.0;

fn prepare_frame_height(
    is_studio: bool,
    checking: bool,
    window_height: f32,
    track_width: f32,
) -> f32 {
    if is_studio {
        let reserved_height = if checking { 468.0 } else { 434.0 };
        // Focused Prepare gives the video the largest single region, but it is
        // an editor rather than a player. Reserve a real lower workbench for
        // the filmstrip and exact range controls instead of letting the frame
        // consume the entire height on a large monitor.
        let minimum_height = if window_height <= 560.0 { 118.0 } else { 170.0 };
        let available_height = (window_height - reserved_height).max(minimum_height);
        let width_aware_height = (track_width * 0.58).max(minimum_height);
        available_height.min(width_aware_height)
    } else {
        // The dock reserves room for the expanded filmstrip, source details,
        // and its always-available delivery footer. Tall windows still reach
        // the width-aware preview size; short windows yield from the video
        // well instead of pushing the primary action below the workspace bar.
        let reserved_height = if checking { 454.0 } else { 420.0 };
        let available_height = (window_height - reserved_height).max(220.0);
        // A wide dock earns a visibly larger proofing canvas, but the lower
        // delivery handoff and filmstrip must stay above the workspace tabs.
        let width_aware_height = (track_width * 0.68).clamp(280.0, 420.0);
        let viewport_cap = if window_height < 1000.0 {
            300.0
        } else if window_height < 1200.0 {
            360.0
        } else {
            420.0
        };
        available_height.min(width_aware_height).min(viewport_cap)
    }
}

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
        let track_height = if self.prepare.studio_mode && self.window_size.1 <= 560.0 {
            48.0
        } else if self.prepare.studio_mode {
            78.0
        } else if self.window_size.1 >= 1100.0 {
            92.0
        } else if self.window_size.1 >= 900.0 {
            72.0
        } else {
            68.0
        };
        // Track origin in window coordinates.
        let is_studio = self.prepare.studio_mode;
        let track_left = if is_studio {
            // Focused Studio replaces the global sidebar and command chrome.
            PREPARE_GUTTER
        } else {
            self.window_size.0 - panel_width + PREPARE_GUTTER
        };
        let stage_gap = if is_studio && self.window_size.1 <= 560.0 {
            4.0
        } else {
            6.0
        };

        let mut stage = div()
            .id("prepare-stage")
            .w_full()
            .flex()
            .flex_col()
            .px(px(PREPARE_GUTTER))
            .py(px(6.0))
            .gap(px(stage_gap))
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
                let committed_boundary = match app.prepare.drag {
                    DragHandle::TrimIn => Some("IN"),
                    DragHandle::TrimOut => Some("OUT"),
                    _ => None,
                };
                app.prepare.drag = DragHandle::None;
                app.save_draft();
                if let Some(boundary) = committed_boundary {
                    log::info!(
                        "Prepare cut {boundary} drag committed: in={:.3} out={:.3}",
                        app.prepare.trim_start,
                        app.prepare.trim_end
                    );
                }
                cx.notify();
            }),
        );

        // Video frame.
        // Portrait and landscape clips receive the same proofing area and
        // letterbox inside it; the source ratio must not collapse the canvas.
        let frame_height =
            prepare_frame_height(is_studio, self.checking, self.window_size.1, track_width);
        // Frame rect in window coordinates (used by crop/mask drag math).
        // The workspace tabs live at the window bottom, so the frame sits
        // below the app and context toolbars, the optional studio/status
        // header, and this stage's compact top inset.
        let frame_x = track_left;
        let frame_top = if is_studio {
            44.0 + if self.checking { 34.0 } else { 0.0 }
        } else {
            TITLE_BAR_HEIGHT
                + CONTEXT_TOOLBAR_HEIGHT
                + if self.checking { 34.0 } else { 0.0 }
        } + 6.0;
        self.prepare.frame_rect = (frame_x, frame_top, track_width, frame_height);
        let has_edits = self.prepare.has_edits();
        let mut frame = div()
            .id("prepare-frame")
            .w(px(track_width))
            .h(px(frame_height))
            .rounded(px(2.0))
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
                    .rounded(px(4.0))
                    .bg(theme.media_overlay)
                    .text_size(px(11.0))
                    .text_color(theme.text)
                    .child(message),
            );
        }
        // Edit overlays (crop + masks).
        frame = frame.child(self.render_edit_overlays(cx, theme, track_width, frame_height));
        stage = stage.child(frame);

        // Transport row (precise centiseconds like the original).
        let time_label = self.prepare.format_time_precise(position);
        let duration_label = self.prepare.format_time_precise(duration);
        let transport_control = if is_studio { 40.0 } else { 36.0 };
        let transport_group = div()
            .flex_none()
            .w(px(transport_control * 3.0 + 2.0))
            .h(px(transport_control))
            .bg(theme.surface_soft)
            .border_1()
            .border_color(theme.border_strong)
            .flex()
            .items_center()
            .child(
                workbench_button(
                    "back-5",
                    "",
                    "skip-back",
                    ButtonKind::Ghost,
                    seek_enabled,
                    true,
                    "Back 5 seconds",
                    cx,
                    |app, cx| {
                        app.prepare
                            .seek(app.prepare.position - 5.0, app.prepare.duration);
                        cx.notify();
                    },
                )
                .w(px(transport_control))
                .h(px(transport_control - 2.0)),
            )
            .child(div().flex_none().w(px(1.0)).h_full().bg(theme.border))
            .child(
                workbench_button(
                    "play-pause",
                    "",
                    if playing { "pause" } else { "play" },
                    ButtonKind::Ghost,
                    play_enabled,
                    true,
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
                .w(px(transport_control))
                .h(px(transport_control - 2.0))
                .bg(theme.active),
            )
            .child(div().flex_none().w(px(1.0)).h_full().bg(theme.border))
            .child(
                workbench_button(
                    "forward-5",
                    "",
                    "skip-forward",
                    ButtonKind::Ghost,
                    seek_enabled,
                    true,
                    "Forward 5 seconds",
                    cx,
                    |app, cx| {
                        app.prepare
                            .seek(app.prepare.position + 5.0, app.prepare.duration);
                        cx.notify();
                    },
                )
                .w(px(transport_control))
                .h(px(transport_control - 2.0)),
            );
        let transport_height = if is_studio && self.window_size.1 <= 560.0 {
            PREPARE_CONTROL_HEIGHT
        } else if is_studio {
            48.0
        } else {
            44.0
        };
        stage = stage.child(
            div()
                .id("transport")
                .w_full()
                .h(px(transport_height))
                .px(px(4.0))
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(tabular(
                    div()
                        .flex_1()
                        .min_w(px(70.0))
                        .child(time_label)
                        .text_size(px(12.0))
                        .text_color(theme.text_soft),
                ))
                .child(transport_group)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(70.0))
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
            .rounded(px(2.0))
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
                    let x: f32 = event.position.x.into();
                    let seconds = ((x - track_left) / track_width * duration as f32)
                        .clamp(0.0, duration as f32);
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
        if self.timeline_loading && !self.prepare.timeline_ready {
            track = track.child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(theme.media_overlay)
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(7.0))
                    .child(div().w(px(42.0)).child(progress_bar(0.0, true)))
                    .child(
                        div()
                            .child("Building filmstrip")
                            .text_size(px(11.0))
                            .text_color(theme.media_text),
                    ),
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
        let selected_range_width = ((end_fraction - start_fraction) as f32) * track_width;
        if cut_active {
            let mut selection = div()
                .absolute()
                .top_0()
                .left(px((start_fraction as f32) * track_width))
                .w(px(selected_range_width))
                .h_full()
                .border_2()
                .border_color(theme.accent)
                .flex()
                .items_end()
                .justify_center()
                .pb(px(4.0));
            if selected_range_width >= 112.0 {
                selection = selection.child(
                    div()
                        .px(px(6.0))
                        .py(px(2.0))
                        .bg(theme.media_overlay)
                        .child(format!(
                            "CUT  {}",
                            self.prepare.format_time_precise(trim_end - trim_start)
                        ))
                        .text_size(px(10.0))
                        .text_color(theme.media_text)
                        .font_weight(FontWeight::SEMIBOLD),
                );
            }
            track = track.child(selection);
        } else if !is_studio && duration > 0.0 && !self.timeline_loading {
            track = track.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_end()
                    .justify_center()
                    .pb(px(5.0))
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .bg(theme.media_overlay)
                            .child("Drag IN / OUT to cut")
                            .text_size(px(10.0))
                            .text_color(theme.media_text),
                    ),
            );
        }
        // Playhead. Blue stays exclusive to temporal focus.
        let playhead = color_from_hex("#4DA3FF");
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
                        .bg(playhead),
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
                        .bg(playhead),
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

        // Tick labels are detail for a wide editing surface, not a
        // permanent second ruler in the compact dock.
        if is_studio && panel_width > 620.0 {
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

        // Exact time fields belong in focused Studio. The dock keeps the same
        // information as quiet readouts anchored to the trim lane.
        if is_studio {
            let show_mark_labels = panel_width >= 560.0;
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
            let mut precision = div()
                .w_full()
                .min_w(px(0.0))
                .h(px(40.0))
                .px(px(8.0))
                .bg(theme.surface)
                .border_t_1()
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .child("IN")
                        .text_size(px(10.0))
                        .text_color(theme.muted)
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    field(
                        "prepare-in",
                        "00:00.00",
                        &self.fields.get("prepare-in").cloned().unwrap_or_else(|| {
                            crate::widgets::FieldState {
                                text: in_text.clone(),
                                caret: in_text.chars().count(),
                                committed: false,
                                marked_range: None,
                            }
                        }),
                        self.focused_field.as_deref() == Some("prepare-in"),
                        !disabled,
                        false,
                        cx,
                    )
                    .flex_none()
                    .w(px(92.0))
                    .h(px(PREPARE_CONTROL_HEIGHT)),
                )
                .child(workbench_button(
                    "mark-in-playhead",
                    if show_mark_labels { "Set In" } else { "" },
                    "mark-in",
                    ButtonKind::Secondary,
                    !disabled && duration > 0.0,
                    !show_mark_labels,
                    "Set In to playhead  ·  I",
                    cx,
                    |app, cx| {
                        if app.prepare.mark_in_at_playhead() {
                            app.save_draft();
                        }
                        cx.notify();
                    },
                ))
                .child(div().flex_1())
                .child(if cut_active {
                    tabular(
                        div()
                            .child(format!(
                                "CUT  {}",
                                self.prepare.format_time_precise(trim_end - trim_start)
                            ))
                            .text_size(px(11.0))
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
                            .text_size(px(11.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .into_any()
                })
                .child(div().flex_1())
                .child(workbench_button(
                    "mark-out-playhead",
                    if show_mark_labels { "Set Out" } else { "" },
                    "mark-out",
                    ButtonKind::Secondary,
                    !disabled && duration > 0.0,
                    !show_mark_labels,
                    "Set Out to playhead  ·  O",
                    cx,
                    |app, cx| {
                        if app.prepare.mark_out_at_playhead() {
                            app.save_draft();
                        }
                        cx.notify();
                    },
                ))
                .child(
                    div()
                        .child("OUT")
                        .text_size(px(10.0))
                        .text_color(theme.muted)
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    field(
                        "prepare-out",
                        "00:00.00",
                        &self.fields.get("prepare-out").cloned().unwrap_or_else(|| {
                            crate::widgets::FieldState {
                                text: out_text.clone(),
                                caret: out_text.chars().count(),
                                committed: false,
                                marked_range: None,
                            }
                        }),
                        self.focused_field.as_deref() == Some("prepare-out"),
                        !disabled,
                        false,
                        cx,
                    )
                    .flex_none()
                    .w(px(92.0))
                    .h(px(PREPARE_CONTROL_HEIGHT)),
                );
            if cut_active {
                precision = precision.child(workbench_button(
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
            stage = stage.child(precision);
        } else {
            let mut range_readout = div()
                .id("dock-range-readout")
                .w_full()
                .h(px(24.0))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(tabular(
                    div()
                        .child(format!(
                            "IN  {}",
                            self.prepare.format_time_precise(trim_start)
                        ))
                        .text_size(px(11.0))
                        .text_color(theme.text_soft),
                ))
                .child(div().flex_1())
                .child(
                    div()
                        .child(if cut_active {
                            format!(
                                "CUT  {}",
                                self.prepare.format_time_precise(trim_end - trim_start)
                            )
                        } else {
                            "DRAG EDGES TO CUT".to_string()
                        })
                        .text_size(px(10.0))
                        .text_color(if cut_active {
                            theme.accent_text
                        } else {
                            theme.muted
                        })
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(div().flex_1())
                .child(tabular(
                    div()
                        .child(format!(
                            "OUT  {}",
                            self.prepare.format_time_precise(trim_end)
                        ))
                        .text_size(px(11.0))
                        .text_color(theme.text_soft),
                ));
            if cut_active {
                range_readout = range_readout.child(workbench_button(
                    "reset-cut-dock",
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
            stage = stage.child(range_readout);
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
        let mut source_parts = vec![size_label, self.prepare.format_time(duration)];
        if !resolution_label.is_empty() {
            source_parts.push(resolution_label);
        }
        let source_details = source_parts.join("  ·  ");
        if is_studio {
            stage = stage.child(
                div()
                    .w_full()
                    .h(px(34.0))
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
                    .child(
                        div()
                            .flex_none()
                            .child(source_details)
                            .text_size(px(11.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    )
                    .child(workbench_button(
                        "reveal-in-library",
                        "Reveal",
                        "target",
                        ButtonKind::Ghost,
                        true,
                        panel_width < 520.0,
                        "Reveal in library",
                        cx,
                        |app, cx| {
                            app.command(Command::RevealSelectedInLibrary);
                            cx.notify();
                        },
                    )),
            );
        } else {
            stage = stage.child(
                div()
                    .w_full()
                    .h(px(46.0))
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
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .child(name)
                                    .text_size(px(12.0))
                                    .text_color(theme.text_soft)
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
                    .child(workbench_button(
                        "reveal-in-library",
                        "",
                        "target",
                        ButtonKind::Ghost,
                        true,
                        true,
                        "Reveal in library",
                        cx,
                        |app, cx| {
                            app.command(Command::RevealSelectedInLibrary);
                            cx.notify();
                        },
                    )),
            );
        }
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
        let _ = duration;
        let handle_width = 20.0;
        let handle_left = (x - handle_width / 2.0).clamp(0.0, track_width - handle_width);
        let boundary_in_handle = (x - handle_left).clamp(2.0, handle_width - 2.0);
        let rail_left = (boundary_in_handle - 2.0).clamp(0.0, handle_width - 4.0);
        let active_color = if self.prepare.drag == handle {
            theme.accent_pressed
        } else {
            theme.accent
        };
        let mut element = div()
            .id(id)
            .absolute()
            .top_0()
            .left(px(handle_left))
            .w(px(handle_width))
            .h(px(track_height))
            .hover(|style| style.bg(theme.accent_soft))
            .active(|style| style.bg(theme.accent_soft))
            .cursor_ew_resize()
            .tooltip(move |_window, cx| crate::tooltip_view(cx, label.clone()))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(rail_left))
                    .w(px(4.0))
                    .h_full()
                    .bg(active_color),
            )
            .child(if is_in {
                div()
                    .absolute()
                    .top(px(4.0))
                    .left(px(boundary_in_handle))
                    .h(px(17.0))
                    .px(px(5.0))
                    .bg(active_color)
                    .flex()
                    .items_center()
                    .child("IN")
                    .text_size(px(9.0))
                    .text_color(theme.accent_content)
                    .font_weight(FontWeight::SEMIBOLD)
                    .into_any()
            } else {
                div()
                    .absolute()
                    .top(px(4.0))
                    .right(px(handle_width - boundary_in_handle))
                    .h(px(17.0))
                    .px(px(5.0))
                    .bg(active_color)
                    .flex()
                    .items_center()
                    .child("OUT")
                    .text_size(px(9.0))
                    .text_color(theme.accent_content)
                    .font_weight(FontWeight::SEMIBOLD)
                    .into_any()
            });
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
                    // This handle sits inside the seekable timeline. GPUI
                    // bubbles mouse-down from the handle into that parent;
                    // without stopping here, the timeline immediately
                    // replaces TrimIn/TrimOut with Seek and no drag can cut.
                    cx.stop_propagation();
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
            let width = (self.prepare.drag_start_value as f32 + delta).clamp(380.0, 500.0);
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
        let theme = self.theme.clone();
        let active_tab = self.prepare.inspector_tab;
        let edits = self.prepare.has_edits();
        let publish_active = self.publish.active;
        let publish_error = !self.publish.error.is_empty();
        let publish_label = if publish_active {
            "Working"
        } else if publish_error {
            "Result"
        } else if self.bot_connected() || self.personal_configured() {
            "Ready"
        } else {
            "Setup"
        };

        let mut edit_tab = div()
            .id("tab-edit")
            .flex_1()
            .min_w(px(94.0))
            .h_full()
            .px(px(10.0))
            .cursor_pointer()
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .bg(if active_tab == 0 {
                theme.surface
            } else {
                theme.transparent()
            })
            .hover(|style| style.bg(theme.hover))
            .active(|style| style.bg(theme.accent_soft))
            .tab_index(0)
            .focus(|style| style.bg(current_theme().hover))
            .child(icon(
                "square",
                13.0,
                if active_tab == 0 {
                    theme.accent_text
                } else {
                    theme.muted
                },
            ))
            .child(
                div()
                    .child("Edit")
                    .text_size(px(12.0))
                    .text_color(if active_tab == 0 {
                        theme.text
                    } else {
                        theme.text_soft
                    })
                    .font_weight(FontWeight::MEDIUM),
            )
            .child(
                div()
                    .child(if edits { "Edited" } else { "Clean" })
                    .text_size(px(10.0))
                    .text_color(if edits {
                        theme.accent_text
                    } else {
                        theme.muted
                    }),
            )
            .on_click(cx.listener(|app, _event, window, cx| {
                window.blur();
                app.prepare.inspector_tab = 0;
                app.save_draft();
                cx.notify();
            }))
            .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
                app.prepare.inspector_tab = 0;
                app.save_draft();
                cx.notify();
                cx.stop_propagation();
            }))
            .on_action(
                cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
                    app.prepare.inspector_tab = 0;
                    app.save_draft();
                    cx.notify();
                    cx.stop_propagation();
                }),
            );
        if active_tab == 0 {
            edit_tab = edit_tab.child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .w_full()
                    .h(px(2.0))
                    .bg(theme.accent),
            );
        }

        let mut publish_tab = div()
            .id("tab-publish")
            .flex_1()
            .min_w(px(94.0))
            .h_full()
            .px(px(10.0))
            .cursor_pointer()
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .bg(if active_tab == 1 {
                theme.surface
            } else {
                theme.transparent()
            })
            .hover(|style| style.bg(theme.hover))
            .active(|style| style.bg(theme.accent_soft))
            .tab_index(0)
            .focus(|style| style.bg(current_theme().hover))
            .child(icon(
                "send",
                13.0,
                if active_tab == 1 {
                    theme.accent_text
                } else {
                    theme.muted
                },
            ))
            .child(
                div()
                    .child("Deliver")
                    .text_size(px(12.0))
                    .text_color(if active_tab == 1 {
                        theme.text
                    } else {
                        theme.text_soft
                    })
                    .font_weight(FontWeight::MEDIUM),
            )
            .child(div().child(publish_label).text_size(px(10.0)).text_color(
                if publish_active || publish_error {
                    theme.accent_text
                } else {
                    theme.muted
                },
            ))
            .on_click(cx.listener(|app, _event, window, cx| {
                window.blur();
                app.prepare.inspector_tab = 1;
                app.save_draft();
                cx.notify();
            }))
            .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
                app.prepare.inspector_tab = 1;
                app.save_draft();
                cx.notify();
                cx.stop_propagation();
            }))
            .on_action(
                cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
                    app.prepare.inspector_tab = 1;
                    app.save_draft();
                    cx.notify();
                    cx.stop_propagation();
                }),
            );
        if active_tab == 1 {
            publish_tab = publish_tab.child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .w_full()
                    .h(px(2.0))
                    .bg(theme.accent),
            );
        }

        div()
            .id("prepare-tabs")
            .w_full()
            .h(px(42.0))
            .bg(theme.ink)
            .border_t_1()
            .border_b_1()
            .border_color(theme.border)
            .flex()
            .flex_row()
            .child(edit_tab)
            .child(publish_tab)
            .child(div().flex_1())
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
            self.render_edit_inspector(cx, theme, panel_width)
                .into_any()
        } else {
            self.render_publish_inspector(cx, theme, panel_width)
                .into_any()
        };
        let mut scroll = div()
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
        if self.checking {
            scroll = scroll.opacity(0.5);
        }
        column = column.child(scroll);
        if self.prepare.inspector_tab == 1 {
            column = column.child(self.render_action_dock(cx, theme, panel_width));
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
                    .child("Select a mask here, then position it on the video.")
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
                        field(
                            "target-mb",
                            "MB",
                            &self.fields.get("target-mb").cloned().unwrap_or_default(),
                            self.focused_field.as_deref() == Some("target-mb"),
                            true,
                            false,
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
                        "send",
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
                field(
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
                        "external",
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
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(caption_area(
                    self,
                    cx,
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

    fn render_action_dock(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        panel_width: f32,
    ) -> impl Element {
        let publish = self.publish.clone();
        let telegram_ready = self.telegram_ready();
        let checking = self.checking;
        let output_ready = !publish.output_path.is_empty();
        // The X handoff row only shows when X was an actual destination of
        // the last publish (mirrors the original's lastSubmitXEnabled).
        let output_x_ready = output_ready && self.prepare.last_submit_x_enabled;
        let wide = panel_width >= 560.0;

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
                        .child(workbench_button(
                            "cancel-publish",
                            "",
                            "close",
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
                            button(
                                "copy-video",
                                "Copy video",
                                ButtonKind::Secondary,
                                Some("⧉"),
                                true,
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
                            button(
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
                            button(
                                "show-in-folder",
                                "Show in folder",
                                ButtonKind::Secondary,
                                Some("▤"),
                                true,
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
                            button(
                                "prepare-x",
                                "Prepare X",
                                ButtonKind::Secondary,
                                Some("external"),
                                true,
                                cx,
                                |app, cx| {
                                    app.submit_publish("x", cx);
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT))
                            .flex_1(),
                        )
                        .child(
                            button(
                                "send-telegram",
                                "Send Telegram",
                                ButtonKind::Secondary,
                                Some("send"),
                                telegram_ready,
                                cx,
                                |app, cx| {
                                    app.submit_publish("telegram", cx);
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT))
                            .flex_1(),
                        )
                        .child(
                            button(
                                "send-both",
                                "Send + prepare X",
                                ButtonKind::Primary,
                                Some("shuffle"),
                                telegram_ready,
                                cx,
                                |app, cx| {
                                    app.submit_publish("both", cx);
                                },
                            )
                            .h(px(PREPARE_CONTROL_HEIGHT))
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
                                button(
                                    "prepare-x-narrow",
                                    "Prepare X",
                                    ButtonKind::Secondary,
                                    Some("external"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        app.submit_publish("x", cx);
                                    },
                                )
                                .h(px(PREPARE_CONTROL_HEIGHT)),
                            )
                            .child(
                                button(
                                    "send-telegram-narrow",
                                    "Send Telegram",
                                    ButtonKind::Secondary,
                                    Some("send"),
                                    telegram_ready,
                                    cx,
                                    |app, cx| {
                                        app.submit_publish("telegram", cx);
                                    },
                                )
                                .h(px(PREPARE_CONTROL_HEIGHT)),
                            ),
                    )
                    .child(
                        button(
                            "send-both-narrow",
                            "Send + prepare X",
                            ButtonKind::Primary,
                            Some("shuffle"),
                            telegram_ready,
                            cx,
                            |app, cx| {
                                app.submit_publish("both", cx);
                            },
                        )
                        .h(px(PREPARE_CONTROL_HEIGHT)),
                    );
            }
        }
        dock
    }

    /// Tall docks use the lower workspace for a read-only preparation summary.
    /// Editing remains owned by Studio, so these rows never create a second
    /// state model or hide a more precise control behind a compact substitute.
    pub fn render_prepare_dock_summary(
        &self,
        theme: &crate::theme::Theme,
    ) -> impl Element {
        let telegram_ready = self.telegram_ready();
        let telegram_detail = if telegram_ready {
            if self.prepare.destination.trim().is_empty() {
                "Connected and ready to send".to_string()
            } else {
                format!("Send to {}", self.prepare.destination.trim())
            }
        } else {
            "Connect a bot or personal account in Studio".to_string()
        };
        let caption = if self.prepare.same_caption || self.prepare.x_caption.trim().is_empty() {
            self.prepare.caption.trim()
        } else {
            self.prepare.x_caption.trim()
        };
        let caption_preview = if caption.is_empty() {
            div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(6.0))
                .child(icon("pencil", 16.0, theme.muted_soft))
                .child(
                    div()
                        .child("No caption added")
                        .text_size(px(11.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    div()
                        .child("Add an optional caption in Studio")
                        .text_size(px(10.0))
                        .text_color(theme.muted_soft),
                )
                .into_any()
        } else {
            div()
                .flex_1()
                .min_h(px(0.0))
                .child(caption.to_string())
                .text_size(px(11.0))
                .text_color(theme.text_soft)
                .into_any()
        };
        let destination_row = |label: &'static str,
                               glyph: &'static str,
                               detail: String,
                               badge: &'static str,
                               badge_color: Hsla| {
            div()
                .w_full()
                .flex_none()
                .h(px(56.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(icon(glyph, 17.0, theme.text_soft))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .child(label)
                                .text_size(px(12.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::MEDIUM),
                        )
                        .child(
                            div()
                                .child(detail)
                                .text_size(px(10.0))
                                .text_color(theme.muted)
                                .text_ellipsis(),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .px(px(7.0))
                        .h(px(24.0))
                        .border_1()
                        .border_color(badge_color.opacity(0.55))
                        .flex()
                        .items_center()
                        .child(badge)
                        .text_size(px(10.0))
                        .text_color(badge_color)
                        .font_weight(FontWeight::MEDIUM),
                )
        };

        div()
            .id("prepare-dock-summary")
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .overflow_scroll()
            .scrollbar_width(px(8.0))
            .border_t_1()
            .border_color(theme.border)
            .child(destination_row(
                "X",
                "external",
                "Prepared for deliberate browser handoff".to_string(),
                "Manual",
                theme.text_soft,
            ))
            .child(destination_row(
                "Telegram",
                "send",
                telegram_detail,
                if telegram_ready { "Ready" } else { "Needs setup" },
                if telegram_ready {
                    theme.success
                } else {
                    theme.warning
                },
            ))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h(px(60.0))
                    .px(px(12.0))
                    .py(px(9.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .gap(px(5.0))
                    .child(
                        div()
                            .child("Caption")
                            .text_size(px(10.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(caption_preview),
            )
    }

    /// Laptop-height docks keep a compact destination handoff visible instead
    /// of leaving the lower panel blank or duplicating Studio's form controls.
    pub fn render_prepare_dock_compact_summary(
        &self,
        theme: &crate::theme::Theme,
    ) -> impl Element {
        let telegram_ready = self.telegram_ready();
        let destination = |glyph: &'static str,
                           label: &'static str,
                           state: &'static str,
                           state_color: Hsla| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .h(px(48.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(icon(glyph, 15.0, theme.text_soft))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(label)
                        .text_size(px(11.0))
                        .text_color(theme.text)
                        .font_weight(FontWeight::MEDIUM)
                        .text_ellipsis(),
                )
                .child(
                    div()
                        .child(state)
                        .text_size(px(10.0))
                        .text_color(state_color),
                )
        };

        div()
            .id("prepare-dock-compact-summary")
            .w_full()
            .flex_1()
            .min_h(px(48.0))
            .border_t_1()
            .border_color(theme.border)
            .flex()
            .items_start()
            .child(destination(
                "external",
                "X handoff",
                "Manual",
                theme.text_soft,
            ))
            .child(div().w(px(1.0)).h(px(48.0)).bg(theme.border))
            .child(destination(
                "send",
                "Telegram",
                if telegram_ready { "Ready" } else { "Setup" },
                if telegram_ready {
                    theme.success
                } else {
                    theme.warning
                },
            ))
    }

    /// Docked Prepare deliberately exposes one compact delivery decision.
    /// Configuration and generated-copy management remain in Studio.
    pub fn render_prepare_dock_footer(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
    ) -> impl Element {
        let telegram_ready = self.telegram_ready();
        let publish = self.publish.clone();
        let estimate = self.estimate_output_size_label();
        let mut footer = div()
            .id("prepare-dock-footer")
            .w_full()
            .mt_auto()
            .bg(theme.surface)
            .border_t_1()
            .border_color(theme.border)
            .px(px(PREPARE_GUTTER))
            .py(px(8.0))
            .flex()
            .flex_col()
            .gap(px(7.0));

        if publish.active {
            let stage = if publish.stage.is_empty() {
                "Preparing generated copy".to_string()
            } else {
                publish.stage.clone()
            };
            return footer
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .child(stage)
                                .text_size(px(11.0))
                                .text_color(theme.text_soft)
                                .text_ellipsis(),
                        )
                        .child(
                            div()
                                .child(format!("{}%", (publish.progress * 100.0).round() as i64))
                                .text_size(px(11.0))
                                .text_color(theme.text),
                        )
                        .child(workbench_button(
                            "dock-cancel-publish",
                            "",
                            "close",
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
        }

        let error = publish.error.clone();
        let status_text = if !error.is_empty() {
            format!("Failed · {error}")
        } else if !publish.output_path.is_empty() {
            "Generated copy ready".to_string()
        } else if telegram_ready {
            if estimate.is_empty() {
                "Telegram ready · X manual".to_string()
            } else {
                format!("Telegram ready · X manual · {estimate}")
            }
        } else if estimate.is_empty() {
            "Telegram needs setup · X manual".to_string()
        } else {
            format!("Telegram needs setup · X manual · {estimate}")
        };
        footer = footer.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(if !error.is_empty() {
                            theme.error
                        } else if telegram_ready || !publish.output_path.is_empty() {
                            theme.success
                        } else {
                            theme.warning
                        }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(status_text)
                        .text_size(px(11.0))
                        .text_color(if !error.is_empty() {
                            theme.error
                        } else {
                            theme.text_soft
                        })
                        .text_ellipsis(),
                ),
        );

        let action_label = if telegram_ready {
            "Send + prepare X"
        } else {
            "Prepare X"
        };
        footer.child(
            div()
                .w_full()
                .h(px(PREPARE_CONTROL_HEIGHT))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    button(
                        "dock-open-studio",
                        "Open Studio",
                        ButtonKind::Secondary,
                        Some("maximize"),
                        true,
                        cx,
                        |app, cx| {
                            app.prepare.studio_mode = true;
                            cx.notify();
                        },
                    )
                    .h(px(PREPARE_CONTROL_HEIGHT))
                    .flex_1(),
                )
                .child(
                    button(
                        "dock-publish",
                        action_label,
                        ButtonKind::Primary,
                        Some("send"),
                        true,
                        cx,
                        move |app, cx| {
                            app.submit_publish(if telegram_ready { "both" } else { "x" }, cx);
                        },
                    )
                    .h(px(PREPARE_CONTROL_HEIGHT))
                    .flex_1(),
                ),
        )
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
    use super::{group_digits, prepare_frame_height};

    #[test]
    fn studio_stage_balances_media_and_the_editing_workbench() {
        assert!((prepare_frame_height(true, false, 1404.0, 1580.0) - 916.4).abs() < 0.01);
        assert_eq!(prepare_frame_height(true, false, 760.0, 880.0), 326.0);
        assert_eq!(prepare_frame_height(true, true, 520.0, 600.0), 118.0);
    }

    #[test]
    fn dock_stage_grows_with_a_wide_panel_without_overstretching() {
        assert_eq!(prepare_frame_height(false, false, 760.0, 370.0), 280.0);
        assert!((prepare_frame_height(false, false, 1440.0, 592.0) - 402.56).abs() < 0.01);
        assert_eq!(prepare_frame_height(false, false, 1440.0, 900.0), 420.0);
    }

    #[test]
    fn group_digits_formats_thousands() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1000), "1,000");
        assert_eq!(group_digits(1024), "1,024");
        assert_eq!(group_digits(1234567), "1,234,567");
    }
}
