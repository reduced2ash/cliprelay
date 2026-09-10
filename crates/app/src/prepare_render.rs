//! Prepare workspace renderers: stage (frame + transport + timeline),
//! inspector tabs, edit/publish inspectors, action dock.

use crate::prepare::{DragHandle, MaskPreset, ShapeKind};
use crate::prepare::{CLEANUP_OPTIONS, COMPRESSION_OPTIONS};
use crate::settings_import::*;
use crate::state::*;
use crate::theme::*;
use crate::video_element::{video as video_element, VideoFit};
use crate::widgets::*;
use cliprelay_core::media::CropSpec;
use cliprelay_core::utils::format_bytes;
use gpui::prelude::FluentBuilder;
use gpui::*;
use serde_json::json;
use std::path::PathBuf;
use std::rc::Rc;

const PREPARE_GUTTER: f32 = 10.0;
const PREPARE_CONTROL_HEIGHT: f32 = 44.0;
const PREPARE_SECTION_GAP: f32 = 10.0;

fn prepare_frame_height(
    is_studio: bool,
    checking: bool,
    window_height: f32,
    track_width: f32,
) -> f32 {
    if is_studio {
        let reserved_height = if checking { 427.0 } else { 393.0 };
        // Focused Prepare gives the video the largest single region, but it is
        // an editor rather than a player. Reserve a real lower workbench for
        // the filmstrip and exact range controls instead of letting the frame
        // consume the entire height on a large monitor.
        let minimum_height = if window_height <= 560.0 { 118.0 } else { 170.0 };
        let available_height = (window_height - reserved_height).max(minimum_height);
        let width_aware_height = (track_width * 0.58).max(minimum_height);
        available_height.min(width_aware_height)
    } else {
        // Docked Prepare also owns the full tabbed inspector. Let the proofing
        // canvas yield first so those controls remain present at laptop
        // heights, with overflow handled by the inspector's scroll region.
        let reserved_height = if checking { 540.0 } else { 506.0 };
        let available_height = (window_height - reserved_height).max(118.0);
        let width_aware_height = (track_width * 0.87).clamp(280.0, 420.0);
        let viewport_cap = if window_height < 820.0 {
            220.0
        } else if window_height < 1000.0 {
            280.0
        } else if window_height < 1200.0 {
            320.0
        } else {
            380.0
        };
        available_height.min(width_aware_height).min(viewport_cap)
    }
}

fn fitted_media_rect(
    container_width: f32,
    container_height: f32,
    source_ratio: f32,
) -> (f32, f32, f32, f32) {
    let container_ratio = container_width / container_height.max(1.0);
    if source_ratio > container_ratio {
        let height = container_width / source_ratio.max(0.01);
        (
            0.0,
            (container_height - height) / 2.0,
            container_width,
            height,
        )
    } else {
        let width = container_height * source_ratio.max(0.01);
        (
            (container_width - width) / 2.0,
            0.0,
            width,
            container_height,
        )
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

#[allow(clippy::too_many_arguments)]
fn prepare_dock_destination_row(
    theme: &crate::theme::Theme,
    id: &'static str,
    label: &'static str,
    glyph: &'static str,
    detail: String,
    badge: &'static str,
    badge_color: Hsla,
    cx: &mut Context<crate::App>,
) -> impl Element {
    div()
        .id(id)
        .w_full()
        .h(px(56.0))
        .flex_none()
        .px(px(12.0))
        .border_b_1()
        .border_color(theme.border)
        .flex()
        .items_center()
        .gap(px(10.0))
        .cursor_pointer()
        .tab_index(0)
        .hover(|style| style.bg(theme.hover))
        .active(|style| style.bg(theme.accent_soft))
        .focus(|style| style.bg(theme.hover))
        .child(icon(glyph, 17.0, theme.text_soft))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(3.0))
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
                        .text_size(px(10.5))
                        .text_color(theme.muted)
                        .text_ellipsis(),
                ),
        )
        .child(
            div()
                .flex_none()
                .px(px(10.0))
                .h(px(24.0))
                .rounded(px(12.0))
                .bg(badge_color.opacity(0.10))
                .border_1()
                .border_color(badge_color.opacity(0.24))
                .flex()
                .items_center()
                .child(badge)
                .text_size(px(10.0))
                .text_color(badge_color)
                .font_weight(FontWeight::MEDIUM),
        )
        .child(icon("chevron-right", 13.0, theme.muted_soft))
        .tooltip(move |_window, cx| {
            crate::tooltip_view(
                cx,
                format!("Open {label} delivery settings in Studio").into(),
            )
        })
        .on_click(cx.listener(|app, _event, _window, cx| {
            app.prepare.inspector_tab = 1;
            app.open_selected_in_studio(cx);
        }))
        .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
            app.prepare.inspector_tab = 1;
            app.open_selected_in_studio(cx);
            cx.stop_propagation();
        }))
        .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
            app.prepare.inspector_tab = 1;
            app.open_selected_in_studio(cx);
            cx.stop_propagation();
        }))
}

fn prepare_inspector_tab(
    theme: &crate::theme::Theme,
    id: &'static str,
    label: &'static str,
    tab: i64,
    active_tab: i64,
    compact: bool,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    let selected = active_tab == tab;
    let rest_face: Background = if selected {
        theme.selection_face(TactileState::Rest, false)
    } else {
        theme.transparent().into()
    };
    let hover_face = if selected {
        theme.selection_face(TactileState::Hover, false)
    } else {
        theme.control_face(TactileState::Hover)
    };
    div()
        .id(id)
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .px(px(if compact { 6.0 } else { 8.0 }))
        .cursor_pointer()
        .relative()
        .top(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .tab_index(0)
        .bg(rest_face)
        .shadow(if selected {
            theme.tactile_shadow(TactileState::Rest, true)
        } else {
            Vec::new()
        })
        .hover(|style| {
            style
                .bg(hover_face)
                .shadow(theme.tactile_shadow(TactileState::Hover, true))
        })
        .active(|style| {
            style
                .top(px(1.0))
                .bg(theme.selection_face(TactileState::Pressed, false))
                .shadow(theme.tactile_shadow(TactileState::Pressed, true))
        })
        .focus(|style| {
            style
                .bg(hover_face)
                .shadow(theme.tactile_shadow(TactileState::Hover, true))
                .border_1()
                .border_color(theme.accent)
        })
        .child(
            div()
                .child(label)
                .text_size(px(if compact { 12.5 } else { 14.0 }))
                .text_color(if selected {
                    theme.accent_text
                } else {
                    theme.text_soft
                })
                .font_weight(FontWeight::MEDIUM),
        )
        .when(selected, |tab| {
            tab.child(
                div()
                    .absolute()
                    .bottom_0()
                    .left(px(0.0))
                    .w_full()
                    .h(px(2.0))
                    .bg(theme.accent),
            )
        })
        .on_click(cx.listener(move |app, _event, window, cx| {
            window.blur();
            app.prepare.inspector_tab = tab;
            app.save_draft();
            cx.notify();
        }))
        .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
            app.prepare.inspector_tab = tab;
            app.save_draft();
            cx.notify();
            cx.stop_propagation();
        }))
        .on_action(
            cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                app.prepare.inspector_tab = tab;
                app.save_draft();
                cx.notify();
                cx.stop_propagation();
            }),
        )
}

#[allow(clippy::too_many_arguments)]
fn studio_destination_row(
    theme: &crate::theme::Theme,
    id: &'static str,
    glyph: &'static str,
    title: &'static str,
    badge: &'static str,
    detail: String,
    badge_color: Hsla,
    compact: bool,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .when(compact, |row| {
            row.h(px(68.0))
                .px(px(10.0))
                .rounded(px(2.0))
                .bg(theme.surface_soft)
                .border_1()
        })
        .when(!compact, |row| {
            row.min_h(px(76.0)).px(px(10.0)).py(px(8.0)).border_b_1()
        })
        .border_color(theme.border)
        .cursor_pointer()
        .tab_index(0)
        .hover(|style| style.bg(current_theme().hover))
        .active(|style| style.bg(current_theme().accent_soft))
        .focus(|style| {
            style
                .bg(current_theme().hover)
                .border_1()
                .border_color(current_theme().accent)
        })
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(icon(
            glyph,
            if compact { 18.0 } else { 20.0 },
            theme.text_soft,
        ))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(if compact { 2.0 } else { 3.0 }))
                .child(
                    div()
                        .w_full()
                        .h(px(if compact { 24.0 } else { 26.0 }))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .child(title)
                                .text_size(px(if compact { 13.0 } else { 14.0 }))
                                .text_color(theme.text)
                                .font_weight(FontWeight::MEDIUM),
                        )
                        .child(
                            div()
                                .h(px(if compact { 22.0 } else { 26.0 }))
                                .px(px(if compact { 8.0 } else { 9.0 }))
                                .rounded(px(2.0))
                                .bg(badge_color.opacity(0.10))
                                .border_1()
                                .border_color(badge_color.opacity(0.32))
                                .flex()
                                .items_center()
                                .child(badge)
                                .text_size(px(if compact { 10.5 } else { 11.0 }))
                                .text_color(badge_color)
                                .font_weight(FontWeight::MEDIUM),
                        )
                        .child(icon(
                            "chevron-right",
                            if compact { 14.0 } else { 16.0 },
                            theme.muted,
                        )),
                )
                .child(
                    div()
                        .child(detail)
                        .text_size(px(if compact { 11.0 } else { 11.5 }))
                        .text_color(theme.muted)
                        .when(compact, |detail| detail.text_ellipsis()),
                ),
        )
        .on_click(cx.listener(|app, _event, window, cx| {
            window.blur();
            app.prepare.inspector_tab = 3;
            app.save_draft();
            cx.notify();
        }))
        .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
            app.prepare.inspector_tab = 3;
            app.save_draft();
            cx.stop_propagation();
            cx.notify();
        }))
        .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
            app.prepare.inspector_tab = 3;
            app.save_draft();
            cx.stop_propagation();
            cx.notify();
        }))
}

#[derive(Clone, Copy)]
enum EditTileVisual {
    FreeFrame,
    FrameRatio(f32, f32),
    Guides(i64),
    Mask(MaskPreset),
}

fn edit_tile_preview(
    theme: &crate::theme::Theme,
    visual: EditTileVisual,
    selected: bool,
    compact: bool,
) -> Div {
    let scale = if compact { 0.75 } else { 1.0 };
    let line = if selected {
        theme.accent_text
    } else {
        theme.muted
    };
    let canvas = theme.ink;
    match visual {
        EditTileVisual::FreeFrame => div()
            .w(px(32.0 * scale))
            .h(px(22.0 * scale))
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(30.0 * scale))
                    .h(px(20.0 * scale))
                    .rounded(px(2.0 * scale))
                    .border_1()
                    .border_color(line.opacity(0.48)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(5.0 * scale))
                    .left(px(7.0 * scale))
                    .w(px(18.0 * scale))
                    .h(px(12.0 * scale))
                    .border_1()
                    .border_color(line),
            ),
        EditTileVisual::FrameRatio(width, height) => div()
            .w(px(32.0 * scale))
            .h(px(22.0 * scale))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(width * scale))
                    .h(px(height * scale))
                    .rounded(px(2.0 * scale))
                    .bg(canvas)
                    .border_1()
                    .border_color(line),
            ),
        EditTileVisual::Guides(mode) => {
            let mut preview = div()
                .w(px(32.0 * scale))
                .h(px(22.0 * scale))
                .rounded(px(2.0 * scale))
                .bg(canvas)
                .border_1()
                .border_color(line.opacity(if mode == 0 { 0.42 } else { 0.82 }))
                .relative();
            if mode == 1 {
                for position in [10.0, 21.0] {
                    preview = preview.child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(position * scale))
                            .w(px(1.0))
                            .h_full()
                            .bg(line.opacity(0.72)),
                    );
                }
                for position in [7.0, 14.0] {
                    preview = preview.child(
                        div()
                            .absolute()
                            .top(px(position * scale))
                            .left_0()
                            .w_full()
                            .h(px(1.0))
                            .bg(line.opacity(0.72)),
                    );
                }
            } else if mode == 2 {
                preview = preview
                    .child(
                        div()
                            .absolute()
                            .top(px(4.0 * scale))
                            .left(px(5.0 * scale))
                            .w(px(20.0 * scale))
                            .h(px(12.0 * scale))
                            .border_1()
                            .border_color(line.opacity(0.80)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(10.0 * scale))
                            .left(px(14.0 * scale))
                            .w(px(4.0 * scale))
                            .h(px(1.0))
                            .bg(line.opacity(0.80)),
                    );
            }
            preview
        }
        EditTileVisual::Mask(preset) => {
            let (left, top, width, height) = match preset {
                MaskPreset::Box => (8.0, 6.0, 16.0, 10.0),
                MaskPreset::Square => (10.0, 5.0, 12.0, 12.0),
                MaskPreset::LowerBar => (5.0, 13.0, 22.0, 5.0),
            };
            div()
                .w(px(32.0 * scale))
                .h(px(22.0 * scale))
                .rounded(px(2.0 * scale))
                .bg(canvas)
                .border_1()
                .border_color(line.opacity(0.52))
                .relative()
                .child(
                    div()
                        .absolute()
                        .top(px(top * scale))
                        .left(px(left * scale))
                        .w(px(width * scale))
                        .h(px(height * scale))
                        .bg(line),
                )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn edit_choice_tile(
    theme: &crate::theme::Theme,
    id: &'static str,
    label: &'static str,
    selected: bool,
    enabled: bool,
    height: f32,
    compact: bool,
    visual: EditTileVisual,
    cx: &mut Context<crate::App>,
    on_select: impl Fn(&mut crate::App, &mut Context<crate::App>) + 'static,
) -> Stateful<Div> {
    let on_select = Rc::new(on_select);
    let click = Rc::clone(&on_select);
    let activate = Rc::clone(&on_select);
    let rest_face = if selected {
        theme.selection_face(TactileState::Rest, false)
    } else {
        theme.control_face(TactileState::Rest)
    };
    let hover_face = if selected {
        theme.selection_face(TactileState::Hover, false)
    } else {
        theme.control_face(TactileState::Hover)
    };
    div()
        .id(id)
        .flex_1()
        .min_w(px(0.0))
        .h(px(height))
        .rounded(px(2.0))
        .relative()
        .top(px(0.0))
        .bg(rest_face)
        .shadow(theme.tactile_shadow(TactileState::Rest, true))
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .cursor_pointer()
        .opacity(if enabled { 1.0 } else { 0.42 })
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(if compact { 3.0 } else { 5.0 }))
        .child(
            edit_tile_preview(theme, visual, selected, compact)
                .when(compact, |preview| preview.flex_none()),
        )
        .child(
            div()
                .child(label)
                .when(compact, |label| label.flex_none().line_height(px(14.0)))
                .text_size(px(11.0))
                .text_color(if selected {
                    theme.accent_text
                } else {
                    theme.text_soft
                })
                .font_weight(FontWeight::MEDIUM)
                .text_ellipsis(),
        )
        .when(enabled, |tile| {
            tile.tab_index(0)
                .hover(|style| {
                    style
                        .bg(hover_face)
                        .border_color(theme.tactile_edge(TactileState::Hover, selected))
                        .shadow(theme.tactile_shadow(TactileState::Hover, true))
                })
                .active(|style| {
                    style
                        .top(px(1.0))
                        .bg(theme.selection_face(TactileState::Pressed, selected))
                        .border_color(theme.tactile_edge(TactileState::Pressed, selected))
                        .shadow(theme.tactile_shadow(TactileState::Pressed, true))
                })
                .focus(|style| style.border_2().border_color(current_theme().accent))
                .on_click(cx.listener(move |app, _event, window, cx| {
                    window.blur();
                    click(app, cx);
                }))
                .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                    activate(app, cx);
                    cx.stop_propagation();
                }))
                .on_action(
                    cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                        on_select(app, cx);
                        cx.stop_propagation();
                    }),
                )
        })
}

fn toggle_prepare_crop(app: &mut crate::App, cx: &mut Context<crate::App>) {
    if app.prepare.crop_enabled {
        app.prepare.reset_crop();
    } else {
        app.apply_crop_preset(0);
    }
    app.save_draft();
    cx.notify();
}

fn edit_crop_switch(
    theme: &crate::theme::Theme,
    enabled: bool,
    is_studio: bool,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    let rest_face = if enabled {
        theme.selection_face(TactileState::Rest, true)
    } else {
        theme.control_face(TactileState::Rest)
    };
    let hover_face = if enabled {
        theme.selection_face(TactileState::Hover, true)
    } else {
        theme.control_face(TactileState::Hover)
    };
    let mut track = div()
        .w(px(if is_studio { 42.0 } else { 32.0 }))
        .h(px(if is_studio { 22.0 } else { 18.0 }))
        .px(px(3.0))
        .rounded(px(if is_studio { 11.0 } else { 10.0 }))
        .bg(if enabled {
            theme.accent_control_face(TactileState::Rest)
        } else {
            theme.control_face(TactileState::Pressed)
        })
        .shadow(theme.tactile_shadow(TactileState::Pressed, true))
        .border_1()
        .border_color(if enabled {
            theme.accent
        } else {
            theme.border_strong
        })
        .flex()
        .items_center();
    track = if enabled {
        track.justify_end()
    } else {
        track.justify_start()
    };
    track = track.child(
        div()
            .w(px(if is_studio { 14.0 } else { 12.0 }))
            .h(px(if is_studio { 14.0 } else { 12.0 }))
            .rounded(px(if is_studio { 7.0 } else { 6.0 }))
            .bg(if enabled {
                theme.accent_content
            } else {
                theme.muted
            })
            .shadow(theme.tactile_shadow(TactileState::Rest, true)),
    );

    div()
        .id("edit-crop-switch")
        .w_full()
        .h(px(if is_studio { 68.0 } else { 44.0 }))
        .px(px(if is_studio { 14.0 } else { 10.0 }))
        .rounded(px(2.0))
        .relative()
        .top(px(0.0))
        .bg(rest_face)
        .shadow(theme.tactile_shadow(TactileState::Rest, true))
        .border_1()
        .border_color(if enabled {
            theme.accent.opacity(if is_studio { 0.72 } else { 0.38 })
        } else {
            theme.border
        })
        .cursor_pointer()
        .tab_index(0)
        .flex()
        .items_center()
        .gap(px(if is_studio { 11.0 } else { 8.0 }))
        .hover(|style| {
            style
                .bg(hover_face)
                .border_color(theme.tactile_edge(TactileState::Hover, enabled))
                .shadow(theme.tactile_shadow(TactileState::Hover, true))
        })
        .active(|style| {
            style
                .top(px(1.0))
                .bg(theme.selection_face(TactileState::Pressed, enabled))
                .border_color(theme.tactile_edge(TactileState::Pressed, enabled))
                .shadow(theme.tactile_shadow(TactileState::Pressed, true))
        })
        .focus(|style| style.border_2().border_color(current_theme().accent))
        .child(icon(
            "crop",
            if is_studio { 21.0 } else { 18.0 },
            if enabled {
                theme.accent_text
            } else {
                theme.text_soft
            },
        ))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(if is_studio { 3.0 } else { 0.0 }))
                .child(
                    div()
                        .child("Crop frame")
                        .text_size(px(if is_studio { 14.0 } else { 13.0 }))
                        .text_color(theme.text)
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .when(is_studio, |copy| {
                    copy.child(
                        div()
                            .child(if enabled {
                                "Drag the frame or use the position controls"
                            } else {
                                "Keep every pixel from the source"
                            })
                            .text_size(px(11.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    )
                }),
        )
        .child(
            div()
                .child(if enabled { "ON" } else { "OFF" })
                .text_size(px(if is_studio { 10.0 } else { 9.5 }))
                .text_color(if enabled {
                    theme.accent_text
                } else {
                    theme.muted
                })
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(track)
        .on_click(cx.listener(|app, _event, window, cx| {
            window.blur();
            toggle_prepare_crop(app, cx);
        }))
        .on_action(cx.listener(|app, _: &crate::Activate, _window, cx| {
            toggle_prepare_crop(app, cx);
            cx.stop_propagation();
        }))
        .on_action(cx.listener(|app, _: &crate::ActivateSpace, _window, cx| {
            toggle_prepare_crop(app, cx);
            cx.stop_propagation();
        }))
}

impl crate::App {
    /// Selected-source copy shared by the dock header and Studio source strip.
    pub(crate) fn prepare_source_summary(&self) -> (String, String, String) {
        let name = self
            .selected
            .as_ref()
            .map(|media| media.name.clone())
            .unwrap_or_default();
        let path = self
            .selected
            .as_ref()
            .map(|media| media.path.clone())
            .unwrap_or_default();
        let size_label = self
            .selected
            .as_ref()
            .map(|media| format_bytes(media.size_bytes as f64))
            .unwrap_or_default();
        let resolution_label = self
            .selected
            .as_ref()
            .map(|media| {
                if media.width > 0 && media.height > 0 {
                    format!("{}×{}", media.width, media.height)
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();
        let mut details = vec![size_label, self.prepare.format_time(self.prepare.duration)];
        if !resolution_label.is_empty() {
            details.push(resolution_label);
        }
        (name, details.join("  ·  "), path)
    }

    /// The media stage: frame, transport, timeline, and Studio source strip.
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
        let is_studio = self.prepare_is_focused();
        let stacked_precision = is_studio && panel_width < 780.0;
        let precision_height = if stacked_precision { 118.0 } else { 70.0 };
        let stage_gutter = if is_studio { 16.0 } else { PREPARE_GUTTER };
        let studio_media_right_inset = if is_studio { 13.0 } else { 0.0 };
        let track_width = (panel_width - stage_gutter * 2.0 - studio_media_right_inset).max(100.0);
        let track_height = if is_studio && self.window_size.1 <= 560.0 {
            48.0
        } else if is_studio {
            78.0
        } else {
            42.0
        };
        // Track origin in window coordinates.
        let track_left = if is_studio {
            // The focused workbench has a 12px outer inset and a 1px panel
            // seam before the stage's own media gutter.
            13.0 + stage_gutter
        } else {
            self.window_size.0 - panel_width + PREPARE_GUTTER
        };
        let stage_gap = if is_studio { 0.0 } else { 6.0 };

        let mut stage = div()
            .id("prepare-stage")
            .flex_none()
            .w_full()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .px(px(stage_gutter))
            .pt(px(if is_studio { 17.0 } else { 6.0 }))
            .pb(px(if is_studio { 0.0 } else { 6.0 }))
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
        let mut frame_height =
            prepare_frame_height(is_studio, self.checking, self.window_size.1, track_width);
        if is_studio {
            // Reserve the inset panel's outside spacing and optional second
            // row in the media budget instead of squeezing its controls.
            frame_height = (frame_height - (precision_height - 70.0) - 28.0).max(118.0);
        }
        let source_ratio = self
            .selected
            .as_ref()
            .map(|media| {
                if media.width > 0 && media.height > 0 {
                    media.width as f32 / media.height as f32
                } else {
                    16.0 / 9.0
                }
            })
            .unwrap_or(16.0 / 9.0);
        let (media_left, media_top, media_width, media_height) = fitted_media_rect(
            track_width,
            frame_height,
            self.prepare.oriented_ratio(source_ratio as f64) as f32,
        );
        let has_edits = self.prepare.has_edits();
        let mut frame = div()
            .id("prepare-frame")
            .flex_none()
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
        // Measure the media after layout so crop/mask input follows resizing,
        // application zoom and the short-window stage's scroll offset.
        let app = cx.entity().downgrade();
        frame = frame.child(
            canvas(
                move |bounds, _, cx| {
                    let _ = app.update(cx, |app, _| {
                        let x = f32::from(bounds.origin.x) + media_left;
                        let y = f32::from(bounds.origin.y) + media_top;
                        app.prepare.frame_rect = (x, y, media_width, media_height);
                        app.prepare.mask_frame_rect = if app.prepare.crop_enabled {
                            (
                                x + app.prepare.crop.x as f32 * media_width,
                                y + app.prepare.crop.y as f32 * media_height,
                                app.prepare.crop.width as f32 * media_width,
                                app.prepare.crop.height as f32 * media_height,
                            )
                        } else {
                            app.prepare.frame_rect
                        };
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );
        let image_source = (!thumbnail.is_empty()).then(|| PathBuf::from(thumbnail));
        if let Some(video) = self.prepare.video.clone() {
            frame = frame.child(video_element(
                video,
                "prepare-video",
                px(track_width),
                px(frame_height),
                VideoFit::Contain,
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
        frame = frame.child(self.render_edit_overlays(
            cx,
            theme,
            media_left,
            media_top,
            media_width,
            media_height,
        ));
        stage = stage.child(frame);

        // Transport row (precise centiseconds like the original).
        let time_label = self.prepare.format_time_precise(position);
        let duration_label = self.prepare.format_time_precise(duration);
        let transport_control = if is_studio { 40.0 } else { 36.0 };
        let transport_side_width = if is_studio { 42.0 } else { 40.0 };
        let transport_play_width = if is_studio { 52.0 } else { 54.0 };
        let transport_group = div()
            .flex_none()
            .h(px(transport_control))
            .flex()
            .items_center()
            .gap(px(if is_studio { 6.0 } else { 5.0 }))
            .child(
                workbench_button(
                    "back-5",
                    "",
                    "skip-back",
                    ButtonKind::Secondary,
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
                .w(px(transport_side_width))
                .h(px(transport_control)),
            )
            .child(
                workbench_button(
                    "play-pause",
                    "",
                    if playing { "pause" } else { "play" },
                    ButtonKind::Secondary,
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
                .w(px(transport_play_width))
                .h(px(transport_control))
                .bg(theme.active),
            )
            .child(
                workbench_button(
                    "forward-5",
                    "",
                    "skip-forward",
                    ButtonKind::Secondary,
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
                .w(px(transport_side_width))
                .h(px(transport_control)),
            );
        let transport_height = if is_studio && self.window_size.1 <= 560.0 {
            32.0
        } else if is_studio {
            52.0
        } else {
            48.0
        };
        stage = stage.child(
            div()
                .id("transport")
                .w_full()
                .h(px(transport_height))
                .px(px(6.0))
                // The docked stage contributes 6px above this row. Bias the
                // row's 12px of spare height upward so both optical gaps land
                // at 9px while keeping every transport hit target unchanged.
                .when(!is_studio, |transport| transport.pt(px(3.0)).pb(px(9.0)))
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(if is_studio { 10.0 } else { 8.0 }))
                .child(tabular(
                    div()
                        .flex_1()
                        .min_w(px(70.0))
                        .child(time_label)
                        .text_size(px(if is_studio { 13.0 } else { 12.0 }))
                        .text_color(theme.text_soft),
                ))
                .child(transport_group)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(70.0))
                        .child(duration_label)
                        .text_size(px(if is_studio { 13.0 } else { 12.0 }))
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
            .flex_none()
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
        stage = stage.child(track.mt(px(if is_studio { 9.0 } else { 0.0 })));

        if is_studio {
            stage = stage.child(
                div()
                    .id("studio-range-summary")
                    .w_full()
                    .h(px(50.0))
                    .flex_none()
                    .px(px(8.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .child(tabular(
                        div()
                            .flex_1()
                            .child(format!(
                                "IN   {}",
                                self.prepare.format_time_precise(trim_start)
                            ))
                            .text_size(px(13.0))
                            .text_color(theme.accent_text),
                    ))
                    .child(tabular(
                        div()
                            .flex_1()
                            .child(format!(
                                "CUT   {}",
                                self.prepare.format_time_precise(trim_end - trim_start)
                            ))
                            .text_size(px(13.0))
                            .text_color(theme.accent_text)
                            .text_center(),
                    ))
                    .child(tabular(
                        div()
                            .flex_1()
                            .child(format!(
                                "OUT   {}",
                                self.prepare.format_time_precise(trim_end)
                            ))
                            .text_size(px(13.0))
                            .text_color(theme.accent_text)
                            .text_right(),
                    )),
            );
        }

        // Exact time fields belong in focused Studio. The dock keeps the same
        // information as quiet readouts anchored to the trim lane.
        let mut studio_precision: Option<AnyElement> = None;
        if is_studio {
            let mut range_fields = div()
                .id("studio-precision-fields")
                .w(px(336.0))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(12.0))
                .when(stacked_precision, |row| row.w_full());
            for (id, button_id, label, value, is_out) in [
                ("prepare-in", "mark-in-playhead", "In", trim_start, false),
                ("prepare-out", "mark-out-playhead", "Out", trim_end, true),
            ] {
                let focused = self.focused_field.as_deref() == Some(id);
                let text = if focused {
                    self.field_text(id)
                } else {
                    self.prepare.format_time_precise(value)
                };
                range_fields = range_fields.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            button(
                                button_id,
                                label,
                                ButtonKind::Secondary,
                                None,
                                !disabled && duration > 0.0,
                                cx,
                                move |app, cx| {
                                    let changed = if is_out {
                                        app.prepare.mark_out_at_playhead()
                                    } else {
                                        app.prepare.mark_in_at_playhead()
                                    };
                                    if changed {
                                        app.save_draft();
                                    }
                                    cx.notify();
                                },
                            )
                            .flex_none()
                            .w(px(44.0))
                            .h(px(42.0))
                            .px(px(8.0))
                            .text_size(px(13.0))
                            .rounded(px(2.0))
                            .tooltip(move |_window, cx| {
                                crate::tooltip_view(cx, format!("Set {label} to playhead").into())
                            }),
                        )
                        .child(tabular(
                            field(
                                id,
                                "00:00.00",
                                &self.fields.get(id).cloned().unwrap_or_else(|| {
                                    crate::widgets::FieldState {
                                        text: text.clone(),
                                        caret: text.chars().count(),
                                        committed: false,
                                        marked_range: None,
                                    }
                                }),
                                focused,
                                !disabled,
                                false,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(104.0))
                            .h(px(42.0)),
                        )),
                );
            }
            let summary_actions = div()
                .id("studio-precision-actions")
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .when(stacked_precision, |row| row.w_full().flex_none())
                .child(
                    div()
                        .flex_none()
                        .min_w(px(80.0))
                        .h(px(42.0))
                        .flex()
                        .flex_col()
                        .justify_center()
                        .gap(px(2.0))
                        .child(
                            div()
                                .child("Duration")
                                .text_size(px(12.0))
                                .text_color(theme.muted),
                        )
                        .child(tabular(
                            div()
                                .child(self.prepare.format_time_precise(trim_end - trim_start))
                                .text_size(px(13.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::MEDIUM),
                        )),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .when(cut_active, |row| {
                            row.child(
                                workbench_button(
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
                                )
                                .w(px(36.0))
                                .h(px(42.0)),
                            )
                        })
                        .child(
                            button(
                                "set-active-playhead",
                                "Set to playhead",
                                ButtonKind::Secondary,
                                Some("play"),
                                !disabled && duration > 0.0,
                                cx,
                                |app, cx| {
                                    let changed =
                                        if app.focused_field.as_deref() == Some("prepare-out") {
                                            app.prepare.mark_out_at_playhead()
                                        } else {
                                            app.prepare.mark_in_at_playhead()
                                        };
                                    if changed {
                                        app.save_draft();
                                    }
                                    cx.notify();
                                },
                            )
                            .h(px(42.0))
                            .px(px(12.0)),
                        ),
                );
            let precision = div()
                .id("studio-precision")
                .w_full()
                .min_w(px(0.0))
                .h(px(precision_height))
                .flex_none()
                .mt(px(12.0))
                .mb(px(16.0))
                .px(px(16.0))
                .py(px(12.0))
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .rounded(px(RADIUS_SM))
                .flex()
                .items_center()
                .gap(px(16.0))
                .when(stacked_precision, |panel| {
                    panel.flex_col().items_start().gap(px(8.0))
                })
                .child(range_fields)
                .child(summary_actions);
            studio_precision = Some(precision.into_any());
        } else {
            let mut range_readout = div()
                .id("dock-range-readout")
                .w_full()
                .h(px(30.0))
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

        // Focused Studio retains its in-workspace source strip. Docked Prepare
        // presents the same information once in the surrounding shell header.
        if is_studio {
            let (name, source_details, source_tooltip) = self.prepare_source_summary();
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
                            .id("prepare-source-name")
                            .flex_1()
                            .min_w(px(40.0))
                            .child(name)
                            .text_size(px(13.0))
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
                            .text_size(px(12.0))
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
        }
        if let Some(precision) = studio_precision {
            stage = stage.child(precision);
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
        let theme = current_theme();
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
        let (frame_x, frame_y, frame_width, frame_height) = match handle {
            DragHandle::MaskMove(_) | DragHandle::MaskResize(_) => self.prepare.mask_frame_rect,
            _ => self.prepare.frame_rect,
        };
        self.prepare.drag_start_x = if frame_width > 0.0 {
            ((pointer_x - frame_x) / frame_width).clamp(0.0, 1.0) as f64
        } else {
            0.0
        };
        self.prepare.drag_start_y = if frame_height > 0.0 {
            ((pointer_y - frame_y) / frame_height).clamp(0.0, 1.0) as f64
        } else {
            0.0
        };
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
        let (fx, fy, fw, fh) = self.prepare.mask_frame_rect;
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
                    shape.preset = MaskPreset::Square;
                } else {
                    shape.kind = ShapeKind::Rectangle;
                    shape.preset = if shape.width > 0.60 && shape.height <= 0.20 {
                        MaskPreset::LowerBar
                    } else {
                        MaskPreset::Box
                    };
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
        media_left: f32,
        media_top: f32,
        media_width: f32,
        media_height: f32,
    ) -> impl Element {
        let mut overlays = div()
            .id("edit-overlays")
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let crop = self.prepare.crop;
        let crop_enabled = self.prepare.crop_enabled;
        let guide_mode = if self.prepare.inspector_tab == 0 {
            self.prepare.guide_mode
        } else {
            0
        };
        let shapes = self.prepare.shapes.clone();
        let selected_shape = self.prepare.selected_shape;

        if crop_enabled {
            let x = media_left + (crop.x as f32 * media_width).max(0.0);
            let y = media_top + (crop.y as f32 * media_height).max(0.0);
            let w = (crop.width as f32 * media_width).clamp(0.0, media_left + media_width - x);
            let h = (crop.height as f32 * media_height).clamp(0.0, media_top + media_height - y);
            // Dim outside.
            for (top, height) in [
                (media_top, y - media_top),
                (y + h, media_top + media_height - y - h),
            ] {
                if height > 0.5 {
                    overlays = overlays.child(
                        div()
                            .absolute()
                            .top(px(top))
                            .left(px(media_left))
                            .w(px(media_width))
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
            for (left, width) in [
                (media_left, x - media_left),
                (x + w, media_left + media_width - x - w),
            ] {
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
                .border_1()
                .border_color(theme.accent)
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
                    .rounded(px(2.0))
                    .bg(theme.ink)
                    .border_2()
                    .border_color(theme.accent)
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().w(px(4.0)).h(px(4.0)).bg(theme.accent));
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

        if guide_mode > 0 {
            let (x, y, w, h) = if crop_enabled {
                (
                    media_left + crop.x as f32 * media_width,
                    media_top + crop.y as f32 * media_height,
                    crop.width as f32 * media_width,
                    crop.height as f32 * media_height,
                )
            } else {
                (media_left, media_top, media_width, media_height)
            };
            let guide_color = theme.text.opacity(0.48);
            if guide_mode == 1 {
                for fraction in [1.0 / 3.0, 2.0 / 3.0] {
                    overlays = overlays
                        .child(
                            div()
                                .absolute()
                                .top(px(y))
                                .left(px(x + w * fraction))
                                .w(px(1.0))
                                .h(px(h))
                                .bg(guide_color),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(y + h * fraction))
                                .left(px(x))
                                .w(px(w))
                                .h(px(1.0))
                                .bg(guide_color),
                        );
                }
            } else {
                let inset_x = w * 0.10;
                let inset_y = h * 0.10;
                overlays = overlays
                    .child(
                        div()
                            .absolute()
                            .top(px(y + inset_y))
                            .left(px(x + inset_x))
                            .w(px((w - inset_x * 2.0).max(1.0)))
                            .h(px((h - inset_y * 2.0).max(1.0)))
                            .border_1()
                            .border_color(guide_color),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(y + h / 2.0))
                            .left(px(x + w / 2.0 - 8.0))
                            .w(px(16.0))
                            .h(px(1.0))
                            .bg(guide_color),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(y + h / 2.0 - 8.0))
                            .left(px(x + w / 2.0))
                            .w(px(1.0))
                            .h(px(16.0))
                            .bg(guide_color),
                    );
            }
        }

        let (mask_left, mask_top, mask_width, mask_height) = if crop_enabled {
            (
                media_left + crop.x as f32 * media_width,
                media_top + crop.y as f32 * media_height,
                crop.width as f32 * media_width,
                crop.height as f32 * media_height,
            )
        } else {
            (media_left, media_top, media_width, media_height)
        };
        for (index, shape) in shapes.iter().enumerate() {
            let selected = selected_shape == Some(index);
            let shape_id = shape.id;
            let x = mask_left + shape.x as f32 * mask_width;
            let y = mask_top + shape.y as f32 * mask_height;
            let w = shape.width as f32 * mask_width;
            let h = shape.height as f32 * mask_height;
            let mut mask = div()
                .id(SharedString::from(format!("mask-{shape_id}")))
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
                    .id(SharedString::from(format!("mask-resize-{shape_id}")))
                    .absolute()
                    .top(px(y + h - 9.0))
                    .left(px(x + w - 9.0))
                    .w(px(18.0))
                    .h(px(18.0))
                    .rounded(px(2.0))
                    .bg(theme.ink)
                    .border_2()
                    .border_color(theme.accent)
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().w(px(4.0)).h(px(4.0)).bg(theme.accent));
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
        let theme = current_theme();
        let active_tab = self.prepare.inspector_tab;
        let compact = !self.prepare_is_focused();
        let edit = prepare_inspector_tab(&theme, "tab-edit", "Edit", 0, active_tab, compact, cx);
        let clean = prepare_inspector_tab(&theme, "tab-clean", "Clean", 2, active_tab, compact, cx);
        let deliver =
            prepare_inspector_tab(&theme, "tab-deliver", "Deliver", 1, active_tab, compact, cx);
        let setup = prepare_inspector_tab(&theme, "tab-setup", "Setup", 3, active_tab, compact, cx);

        div()
            .id("prepare-tabs")
            .w_full()
            .h(px(if compact { 38.0 } else { 42.0 }))
            .flex_none()
            .bg(theme.surface_soft)
            .when(compact, |tabs| tabs.border_t_1())
            .border_b_1()
            .border_color(theme.border)
            .flex()
            .flex_row()
            .child(edit)
            .child(clean)
            .child(deliver)
            .child(setup)
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
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .overflow_hidden();
        let content = match self.prepare.inspector_tab {
            0 => self.render_edit_console(cx, theme, panel_width).into_any(),
            2 => self
                .render_cleanup_inspector(cx, theme, panel_width)
                .into_any(),
            3 => self
                .render_setup_inspector(cx, theme, panel_width)
                .into_any(),
            _ => self
                .render_publish_inspector(cx, theme, panel_width)
                .into_any(),
        };
        let inspector_padding = if self.prepare_is_focused() {
            20.0
        } else {
            PREPARE_GUTTER
        };
        let inspector_top = if self.prepare_is_focused() { 26.0 } else { 8.0 };
        let inspector_bottom = if self.prepare_is_focused() {
            20.0
        } else {
            14.0
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
                    .px(px(inspector_padding))
                    .pt(px(inspector_top))
                    .pb(px(inspector_bottom))
                    .child(content),
            );
        scroll = match self.prepare.inspector_tab {
            0 => scroll
                .track_scroll(&self.prepare_edit_scroll)
                .on_scroll_wheel(cx.listener(|app, _event, _window, cx| {
                    let next = (-f32::from(app.prepare_edit_scroll.offset().y)).max(0.0) as f64;
                    if (app.prepare.edit_scroll_y - next).abs() > 0.5 {
                        app.prepare.edit_scroll_y = next;
                        app.save_draft();
                    }
                    cx.notify();
                })),
            1 => scroll
                .track_scroll(&self.prepare_publish_scroll)
                .on_scroll_wheel(cx.listener(|app, _event, _window, cx| {
                    let next = (-f32::from(app.prepare_publish_scroll.offset().y)).max(0.0) as f64;
                    if (app.prepare.publish_scroll_y - next).abs() > 0.5 {
                        app.prepare.publish_scroll_y = next;
                        app.save_draft();
                    }
                    cx.notify();
                })),
            _ => scroll,
        };
        if self.checking {
            scroll = scroll.opacity(0.5);
        }
        column = column.child(scroll);
        if self.prepare_is_focused() && self.prepare.inspector_tab == 1 {
            column = column.child(self.render_action_dock(cx, theme, panel_width));
        }
        column
    }

    fn render_edit_console(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        _panel_width: f32,
    ) -> impl Element {
        let is_studio = self.prepare_is_focused();
        let crop_enabled = self.prepare.crop_enabled;
        let crop_preset = self.prepare_crop_preset();
        let guide_mode = self.prepare.guide_mode;
        let shape_count = self.prepare.shapes.len();
        let selected_shape = self.prepare.selected_shape;
        let has_edits = self.prepare.has_edits();
        let tile_height = if is_studio { 66.0 } else { 44.0 };
        let compact_control = if is_studio { 36.0 } else { 28.0 };
        let crop = self.prepare.crop;
        let crop_position = if crop_enabled {
            format!(
                "X {:02}%  ·  Y {:02}%",
                (crop.x * 100.0).round() as i64,
                (crop.y * 100.0).round() as i64
            )
        } else {
            "X 00%  ·  Y 00%".to_string()
        };
        let crop_size = if crop_enabled {
            format!(
                "{:02}% × {:02}%",
                (crop.width * 100.0).round() as i64,
                (crop.height * 100.0).round() as i64
            )
        } else {
            "100% × 100%".to_string()
        };

        let mut column = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(if is_studio { 14.0 } else { 6.0 }))
            .when(is_studio, |column| column.child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .child("Frame & privacy")
                                    .text_size(px(15.0))
                                    .text_color(theme.text)
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(
                                div()
                                    .child("Shape the composition and cover anything that should not ship.")
                                    .text_size(px(12.0))
                                    .text_color(theme.muted),
                            ),
                    )
            ))
            .child(
                div().w_full().flex().items_center().gap(px(6.0))
                    .child(edit_crop_switch(theme, crop_enabled, is_studio, cx)
                        .flex_1().min_w(px(0.0)))
                    .child(workbench_button("rotate-left", "Rotate left", "↶", ButtonKind::Secondary,
                        !self.checking, true, "Rotate video 90° left", cx,
                        |app, cx| app.rotate_prepare(-1, cx))
                        .w(px(if is_studio { 56.0 } else { 44.0 }))
                        .h(px(if is_studio { 68.0 } else { 44.0 })))
                    .child(workbench_button("rotate-right", "Rotate right", "↻", ButtonKind::Secondary,
                        !self.checking, true, "Rotate video 90° right", cx,
                        |app, cx| app.rotate_prepare(1, cx))
                        .w(px(if is_studio { 56.0 } else { 44.0 }))
                        .h(px(if is_studio { 68.0 } else { 44.0 })))
            )
            .child(
                div().w_full().h(px(28.0)).flex().items_center().gap(px(6.0))
                    .child(div().flex_1().child("Framing preset").text_size(px(11.0)).text_color(theme.muted))
                    .child(workbench_button("rotation-reset", &format!("{}°", self.prepare.rotation as u16 * 90),
                        "refresh", ButtonKind::Ghost, !self.checking && self.prepare.rotation != 0,
                        false, "Reset video rotation", cx,
                        |app, cx| app.rotate_prepare(-(app.prepare.rotation as i32), cx))
                        .w(px(62.0)).h(px(28.0)))
                    .child(button("reset-frame-edits", "Reset all", ButtonKind::Ghost,
                        Some("refresh"), has_edits && !self.checking, cx, |app, cx| {
                            app.rotate_prepare(-(app.prepare.rotation as i32), cx);
                            app.prepare.reset_crop();
                            app.prepare.clear_shapes();
                            app.save_draft();
                            cx.notify();
                        }).h(px(28.0)).px(px(7.0)).text_size(px(11.0)))
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(if is_studio { 6.0 } else { 5.0 }))
                    .child(edit_choice_tile(
                        theme,
                        "crop-preset-free",
                        "Free",
                        crop_preset == 0,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::FreeFrame,
                        cx,
                        |app, cx| {
                            app.apply_crop_preset(0);
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "crop-preset-full",
                        "Full",
                        crop_preset == 1,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::FrameRatio(30.0, 20.0),
                        cx,
                        |app, cx| {
                            app.apply_crop_preset(1);
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "crop-preset-square",
                        "1:1",
                        crop_preset == 2,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::FrameRatio(20.0, 20.0),
                        cx,
                        |app, cx| {
                            app.apply_crop_preset(2);
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "crop-preset-landscape",
                        "16:9",
                        crop_preset == 3,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::FrameRatio(30.0, 17.0),
                        cx,
                        |app, cx| {
                            app.apply_crop_preset(3);
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "crop-preset-portrait",
                        "9:16",
                        crop_preset == 4,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::FrameRatio(12.0, 21.0),
                        cx,
                        |app, cx| {
                            app.apply_crop_preset(4);
                            app.save_draft();
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .w_full()
                    .min_h(px(if is_studio { 62.0 } else { 40.0 }))
                    .px(px(if is_studio { 10.0 } else { 8.0 }))
                    .rounded(px(2.0))
                    .bg(theme.surface_soft)
                    .border_1()
                    .border_color(if is_studio {
                        theme.border
                    } else {
                        theme.border.opacity(0.72)
                    })
                    .flex()
                    .items_center()
                    .gap(px(if is_studio { 9.0 } else { 7.0 }))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .child(crop_position)
                                    .text_size(px(if is_studio { 12.0 } else { 11.0 }))
                                    .text_color(theme.text_soft)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(tabular(
                                div()
                                    .child(crop_size)
                                    .text_size(px(if is_studio { 11.0 } else { 10.0 }))
                                    .text_color(theme.muted),
                            )),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .gap(px(3.0))
                            .child(
                                workbench_button(
                                    "crop-nudge-left",
                                    "",
                                    "arrow-left",
                                    ButtonKind::Secondary,
                                    crop_enabled,
                                    true,
                                    "Nudge crop left",
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_crop(-0.01, 0.0) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .w(px(compact_control))
                                .h(px(compact_control))
                                .when(!is_studio, |control| control.text_size(px(11.0)).line_height(px(16.0))),
                            )
                            .child(
                                workbench_button(
                                    "crop-nudge-up",
                                    "",
                                    "arrow-up",
                                    ButtonKind::Secondary,
                                    crop_enabled,
                                    true,
                                    "Nudge crop up",
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_crop(0.0, -0.01) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .w(px(compact_control))
                                .h(px(compact_control))
                                .when(!is_studio, |control| control.text_size(px(11.0)).line_height(px(16.0))),
                            )
                            .child(
                                workbench_button(
                                    "crop-center",
                                    "",
                                    "target",
                                    ButtonKind::Secondary,
                                    crop_enabled,
                                    true,
                                    "Center crop",
                                    cx,
                                    |app, cx| {
                                        if app.prepare.center_crop() {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .w(px(compact_control))
                                .h(px(compact_control))
                                .when(!is_studio, |control| control.text_size(px(11.0)).line_height(px(16.0))),
                            )
                            .child(
                                workbench_button(
                                    "crop-nudge-down",
                                    "",
                                    "arrow-down",
                                    ButtonKind::Secondary,
                                    crop_enabled,
                                    true,
                                    "Nudge crop down",
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_crop(0.0, 0.01) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .w(px(compact_control))
                                .h(px(compact_control))
                                .when(!is_studio, |control| control.text_size(px(11.0)).line_height(px(16.0))),
                            )
                            .child(
                                workbench_button(
                                    "crop-nudge-right",
                                    "",
                                    "arrow-right",
                                    ButtonKind::Secondary,
                                    crop_enabled,
                                    true,
                                    "Nudge crop right",
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_crop(0.01, 0.0) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .w(px(compact_control))
                                .h(px(compact_control))
                                .when(!is_studio, |control| control.text_size(px(11.0)).line_height(px(16.0))),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .child("Preview guides")
                            .text_size(px(if is_studio { 11.0 } else { 10.5 }))
                            .text_color(if is_studio {
                                theme.text_soft
                            } else {
                                theme.muted
                            })
                            .font_weight(if is_studio {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            }),
                    )
                    .child(
                        div()
                            .child("preview only")
                            .text_size(px(if is_studio { 10.0 } else { 9.5 }))
                            .text_color(theme.muted_soft),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(if is_studio { 6.0 } else { 5.0 }))
                    .child(edit_choice_tile(
                        theme,
                        "guide-none",
                        "Off",
                        guide_mode == 0,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::Guides(0),
                        cx,
                        |app, cx| {
                            app.prepare.guide_mode = 0;
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "guide-thirds",
                        "Thirds",
                        guide_mode == 1,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::Guides(1),
                        cx,
                        |app, cx| {
                            app.prepare.guide_mode = 1;
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "guide-safe",
                        "Safe area",
                        guide_mode == 2,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::Guides(2),
                        cx,
                        |app, cx| {
                            app.prepare.guide_mode = 2;
                            app.save_draft();
                            cx.notify();
                        },
                    )),
            )
            .child(divider())
            .child(
                div()
                    .w_full()
                    .flex()
                    .when(is_studio, |header| header.items_start())
                    .when(!is_studio, |header| header.items_center())
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .child("Privacy masks")
                                    .text_size(px(if is_studio { 14.0 } else { 12.0 }))
                                    .text_color(theme.text)
                                    .font_weight(if is_studio {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    }),
                            )
                            .when(is_studio, |header| header.child(
                                div()
                                    .child("Cover a face, handle, notification, or lower-third.")
                                    .text_size(px(11.0))
                                    .text_color(theme.muted),
                            )),
                    )
                    .child(
                        div()
                            .child(if shape_count == 1 {
                                "1 mask".to_string()
                            } else {
                                format!("{shape_count} masks")
                            })
                            .text_size(px(if is_studio { 11.0 } else { 10.0 }))
                            .text_color(if shape_count > 0 {
                                theme.text_soft
                            } else {
                                theme.muted
                            }),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(if is_studio { 6.0 } else { 5.0 }))
                    .child(edit_choice_tile(
                        theme,
                        "mask-preset-box",
                        "Box",
                        false,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::Mask(MaskPreset::Box),
                        cx,
                        |app, cx| {
                            app.prepare.add_mask_preset(MaskPreset::Box);
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "mask-preset-square",
                        "Square",
                        false,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::Mask(MaskPreset::Square),
                        cx,
                        |app, cx| {
                            app.prepare.add_mask_preset(MaskPreset::Square);
                            app.save_draft();
                            cx.notify();
                        },
                    ))
                    .child(edit_choice_tile(
                        theme,
                        "mask-preset-lower-bar",
                        "Lower bar",
                        false,
                        true,
                        tile_height,
                        !is_studio,
                        EditTileVisual::Mask(MaskPreset::LowerBar),
                        cx,
                        |app, cx| {
                            app.prepare.add_mask_preset(MaskPreset::LowerBar);
                            app.save_draft();
                            cx.notify();
                        },
                    )),
            );

        if self.prepare.shapes.is_empty() {
            column = column.child(
                div()
                    .w_full()
                    .min_h(px(if is_studio { 64.0 } else { 46.0 }))
                    .px(px(if is_studio { 12.0 } else { 9.0 }))
                    .rounded(px(2.0))
                    .bg(if is_studio {
                        theme.surface_soft
                    } else {
                        theme.surface_soft.opacity(0.68)
                    })
                    .border_1()
                    .border_color(if is_studio {
                        theme.border
                    } else {
                        theme.border.opacity(0.68)
                    })
                    .flex()
                    .items_center()
                    .gap(px(if is_studio { 11.0 } else { 8.0 }))
                    .child(edit_tile_preview(
                        theme,
                        EditTileVisual::Mask(MaskPreset::Box),
                        false,
                        !is_studio,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .child("No masks on this clip")
                                    .text_size(px(if is_studio { 12.0 } else { 11.0 }))
                                    .text_color(theme.text_soft)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .when(is_studio, |empty| {
                                empty.child(
                                div()
                                    .child(
                                        "Choose a shape above, then drag it directly on the video.",
                                    )
                                    .text_size(px(11.0))
                                    .text_color(theme.muted),
                            )
                            }),
                    ),
            );
        } else {
            let mut list = div()
                .id("mask-console-list")
                .w_full()
                .rounded(px(2.0))
                .bg(theme.surface_soft)
                .border_1()
                .border_color(theme.border)
                .flex()
                .flex_col()
                .overflow_hidden();
            let shapes = self.prepare.shapes.clone();
            for (index, shape) in shapes.iter().enumerate() {
                let is_selected = selected_shape == Some(index);
                let preset = shape.preset;
                let kind = match preset {
                    MaskPreset::Square => "Square",
                    MaskPreset::LowerBar => "Lower bar",
                    MaskPreset::Box => "Box",
                };
                let detail = format!(
                    "{:.0}% × {:.0}%  ·  X {:.0}%  Y {:.0}%",
                    shape.width * 100.0,
                    shape.height * 100.0,
                    shape.x * 100.0,
                    shape.y * 100.0
                );
                list = list.child(
                    div()
                        .id(SharedString::from(format!("mask-console-row-{}", shape.id)))
                        .w_full()
                        .h(px(if is_studio { 58.0 } else { 40.0 }))
                        .px(px(if is_studio { 10.0 } else { 8.0 }))
                        .cursor_pointer()
                        .tab_index(0)
                        .bg(if is_selected {
                            theme.active
                        } else {
                            theme.transparent()
                        })
                        .border_b_1()
                        .border_color(theme.border.opacity(0.72))
                        .flex()
                        .items_center()
                        .gap(px(if is_studio { 10.0 } else { 8.0 }))
                        .hover(|style| style.bg(current_theme().hover))
                        .active(|style| style.bg(current_theme().accent_soft))
                        .focus(|style| style.border_2().border_color(current_theme().accent))
                        .child(edit_tile_preview(
                            theme,
                            EditTileVisual::Mask(preset),
                            is_selected,
                            !is_studio,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .flex_col()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .child(format!("Mask {}", index + 1))
                                        .text_size(px(if is_studio { 12.0 } else { 11.0 }))
                                        .text_color(if is_selected {
                                            theme.accent_text
                                        } else {
                                            theme.text
                                        })
                                        .font_weight(FontWeight::SEMIBOLD),
                                )
                                .child(
                                    div()
                                        .child(format!("{kind}  ·  {detail}"))
                                        .text_size(px(if is_studio { 10.5 } else { 9.5 }))
                                        .text_color(theme.muted)
                                        .text_ellipsis(),
                                ),
                        )
                        .child(if is_selected {
                            div()
                                .child("SELECTED")
                                .text_size(px(if is_studio { 9.0 } else { 8.5 }))
                                .text_color(theme.accent_text)
                                .font_weight(FontWeight::SEMIBOLD)
                                .into_any()
                        } else {
                            div().into_any()
                        })
                        .on_click(cx.listener(move |app, _event, window, cx| {
                            window.blur();
                            app.prepare.selected_shape = Some(index);
                            cx.notify();
                        }))
                        .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                            app.prepare.selected_shape = Some(index);
                            cx.stop_propagation();
                            cx.notify();
                        })),
                );
            }
            column = column.child(list);

            if selected_shape.is_some() {
                column = column
                    .child(
                        div()
                            .child("SELECTED MASK POSITION")
                            .text_size(px(10.0))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .gap(px(if is_studio { 6.0 } else { 5.0 }))
                            .child(
                                button(
                                    "nudge-selected-mask-left",
                                    "Left",
                                    ButtonKind::Secondary,
                                    Some("arrow-left"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_selected_shape(-0.01, 0.0) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "nudge-selected-mask-up",
                                    "Up",
                                    ButtonKind::Secondary,
                                    Some("arrow-up"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_selected_shape(0.0, -0.01) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "nudge-selected-mask-down",
                                    "Down",
                                    ButtonKind::Secondary,
                                    Some("arrow-down"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_selected_shape(0.0, 0.01) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "nudge-selected-mask-right",
                                    "Right",
                                    ButtonKind::Secondary,
                                    Some("arrow-right"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.nudge_selected_shape(0.01, 0.0) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .gap(px(if is_studio { 6.0 } else { 5.0 }))
                            .child(
                                button(
                                    "shrink-selected-mask",
                                    "Smaller",
                                    ButtonKind::Secondary,
                                    Some("contract"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.resize_selected_shape(-0.08) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "grow-selected-mask",
                                    "Larger",
                                    ButtonKind::Secondary,
                                    Some("expand"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.resize_selected_shape(0.08) {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "center-selected-mask",
                                    "Center",
                                    ButtonKind::Secondary,
                                    Some("target"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.center_selected_shape() {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .gap(px(if is_studio { 6.0 } else { 5.0 }))
                            .child(
                                button(
                                    "duplicate-selected-mask",
                                    "Duplicate",
                                    ButtonKind::Secondary,
                                    Some("copy"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        if app.prepare.duplicate_selected_shape() {
                                            app.save_draft();
                                        }
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "remove-selected-mask",
                                    "Remove",
                                    ButtonKind::Danger,
                                    Some("trash"),
                                    true,
                                    cx,
                                    |app, cx| {
                                        app.prepare.remove_selected_shape();
                                        app.save_draft();
                                        cx.notify();
                                    },
                                )
                                .h(px(compact_control))
                                .when(!is_studio, |control| {
                                    control.text_size(px(11.0)).line_height(px(16.0))
                                })
                                .flex_1(),
                            ),
                    );
            }
            if shape_count > 1 {
                column = column.child(
                    button(
                        "clear-all-masks",
                        "Clear all masks",
                        ButtonKind::Ghost,
                        Some("trash"),
                        true,
                        cx,
                        |app, cx| {
                            app.prepare.clear_shapes();
                            app.save_draft();
                            cx.notify();
                        },
                    )
                    .h(px(compact_control))
                    .when(!is_studio, |control| {
                        control.text_size(px(11.0)).line_height(px(16.0))
                    }),
                );
            }
        }

        column.child(
            div()
                .w_full()
                .pt(px(2.0))
                .flex()
                .when(is_studio, |row| row.items_start())
                .when(!is_studio, |row| row.items_center())
                .gap(px(if is_studio { 8.0 } else { 6.0 }))
                .child(icon(
                    "info",
                    if is_studio { 14.0 } else { 12.0 },
                    theme.muted,
                ))
                .child(
                    div()
                        .flex_1()
                        .child("Drag on video, or edit a selected mask with the keyboard.")
                        .text_size(px(if is_studio { 11.0 } else { 10.5 }))
                        .text_color(theme.muted),
                ),
        )
    }

    #[allow(dead_code)]
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
                                app.rotate_prepare(-(app.prepare.rotation as i32), cx);
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
                        .id(SharedString::from(format!("mask-row-{}", shape.id)))
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

    fn render_cleanup_inspector(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        _panel_width: f32,
    ) -> impl Element {
        let is_studio = self.prepare_is_focused();
        let cleanup_index = self.prepare.cleanup_index as usize;
        let cleanup_detail = match cleanup_index {
            1 => "Move the generated copy to trash after every requested destination succeeds.",
            2 => "Move the generated copy to trash after Telegram accepts the upload.",
            _ => "Keep the generated copy in ClipRelay's export folder.",
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(if is_studio { 10.0 } else { 9.0 }))
            .child(
                div()
                    .child("Choose what happens to the prepared derivative after delivery.")
                    .text_size(px(11.5))
                    .text_color(theme.muted),
            )
            .child(cleanup_combo(self, cx, theme, true))
            .child(
                div()
                    .child(cleanup_detail)
                    .text_size(px(11.0))
                    .text_color(theme.text_soft),
            )
            .child(divider())
            .child(
                div()
                    .w_full()
                    .min_h(px(if is_studio { 56.0 } else { 54.0 }))
                    .px(px(10.0))
                    .py(px(8.0))
                    .bg(theme.surface_soft)
                    .border_1()
                    .border_color(theme.border.opacity(0.72))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon(
                        "check",
                        if is_studio { 15.0 } else { 14.0 },
                        theme.success,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .when(is_studio, |content| content.min_w(px(0.0)))
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .child("Source protected")
                                    .text_size(px(if is_studio { 12.5 } else { 12.0 }))
                                    .text_color(theme.text)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(
                                div()
                                    .child("Cleanup applies only to generated copies. Your original video is never modified or removed.")
                                    .text_size(px(if is_studio { 11.0 } else { 10.5 }))
                                    .text_color(theme.muted)
                                    .when(is_studio, |detail| {
                                        detail.whitespace_normal().line_clamp(2)
                                    }),
                            ),
                    ),
            )
    }

    fn render_setup_inspector(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        _panel_width: f32,
    ) -> impl Element {
        let is_studio = self.prepare_is_focused();
        let telegram_connected = self.bot_connected() || self.personal_configured();
        let telegram_mode = self.prepare.telegram_mode_index as usize;
        let telegram_status = if telegram_connected {
            "Telegram connection available"
        } else {
            "Telegram needs setup"
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(if is_studio { 10.0 } else { 9.0 }))
            .child(
                div()
                    .child("Manage destination credentials and handoff defaults in Settings.")
                    .text_size(px(11.5))
                    .text_color(theme.muted),
            )
            .child(divider())
            .child(
                div()
                    .w_full()
                    .min_h(px(if is_studio { 72.0 } else { 58.0 }))
                    .flex()
                    .items_center()
                    .gap(px(if is_studio { 12.0 } else { 9.0 }))
                    .child(icon(
                        "send",
                        if is_studio { 20.0 } else { 17.0 },
                        if telegram_connected {
                            theme.success
                        } else {
                            theme.text_soft
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .child("Telegram")
                                    .text_size(px(if is_studio { 13.0 } else { 12.0 }))
                                    .text_color(theme.text)
                                    .font_weight(if is_studio {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    }),
                            )
                            .child(
                                div()
                                    .child(telegram_status)
                                    .text_size(px(if is_studio { 11.0 } else { 10.5 }))
                                    .text_color(if telegram_connected {
                                        theme.success
                                    } else {
                                        theme.warning
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(mode_combo(self, cx, theme, !is_studio))
                    .child(
                        field(
                            "tg-destination-field",
                            if telegram_mode == 1 {
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
                        .flex_1()
                        .h(px(if is_studio {
                            PREPARE_CONTROL_HEIGHT
                        } else {
                            38.0
                        })),
                    ),
            )
            .child(divider())
            .child(
                div()
                    .w_full()
                    .min_h(px(if is_studio { 72.0 } else { 58.0 }))
                    .flex()
                    .items_center()
                    .gap(px(if is_studio { 12.0 } else { 9.0 }))
                    .child(
                        div()
                            .w(px(if is_studio { 20.0 } else { 17.0 }))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("X")
                            .text_size(px(if is_studio { 18.0 } else { 15.0 }))
                            .text_color(theme.text),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .child("X")
                                    .text_size(px(if is_studio { 13.0 } else { 12.0 }))
                                    .text_color(theme.text)
                                    .font_weight(if is_studio {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    }),
                            )
                            .child(
                                div()
                                    .child("Manual browser handoff · no API connection required")
                                    .text_size(px(if is_studio { 11.0 } else { 10.5 }))
                                    .text_color(theme.muted),
                            ),
                    ),
            )
            .child(divider())
            .child(
                button(
                    "open-delivery-settings",
                    "Open Settings",
                    ButtonKind::Secondary,
                    Some("settings"),
                    true,
                    cx,
                    |app, cx| {
                        app.prepare.studio_mode = false;
                        app.navigate_to(Page::Settings, cx);
                    },
                )
                .h(px(if is_studio { 38.0 } else { 34.0 })),
            )
    }

    fn render_publish_inspector(
        &mut self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        _panel_width: f32,
    ) -> impl Element {
        let is_studio = self.prepare_is_focused();
        let compact = !is_studio;
        let estimated = self.estimate_output_size_label();
        let compression_index = self.prepare.compression_index as usize;
        let mode_index = self.prepare.telegram_mode_index as usize;
        let connected = if mode_index == 1 {
            self.personal_configured()
        } else {
            self.bot_connected()
        };
        let destination_ready = !self.prepare.destination.trim().is_empty();
        let telegram_ready = connected && destination_ready;
        let telegram_detail = if !connected {
            if mode_index == 1 {
                "Sign in to your personal account in Settings before sending."
            } else {
                "Connect a bot in Settings before sending."
            }
        } else if !destination_ready {
            "Choose a Telegram destination in Setup before sending."
        } else {
            "Connected and ready for the selected destination."
        }
        .to_string();
        let x_duration_warning = {
            let limit = self
                .settings
                .get(X_DURATION_SECONDS)
                .and_then(|v| v.as_i64())
                .unwrap_or(140) as f64;
            self.prepare.duration > 0.0 && (self.prepare.trim_end - self.prepare.trim_start) > limit
        };
        let x_detail = if x_duration_warning {
            "Current cut exceeds the configured duration limit."
        } else {
            "Manual browser handoff."
        }
        .to_string();
        let same_caption = self.prepare.same_caption;

        let shared_toggle = button(
            "caption-mode-shared",
            "One for both",
            ButtonKind::Secondary,
            None,
            true,
            cx,
            |app, cx| {
                app.prepare.same_caption = true;
                app.save_draft();
                cx.notify();
            },
        )
        .h(px(if is_studio { 32.0 } else { 28.0 }))
        .px(px(if is_studio { 9.0 } else { 8.0 }))
        .text_size(px(if is_studio { 12.0 } else { 11.5 }))
        .flex_1()
        .bg(if same_caption {
            if is_studio {
                theme.accent_soft
            } else {
                theme.active
            }
        } else {
            if is_studio {
                theme.raised
            } else {
                theme.transparent()
            }
        })
        .border_color(if same_caption {
            if is_studio {
                theme.accent
            } else {
                theme.border_strong
            }
        } else {
            if is_studio {
                theme.border
            } else {
                theme.transparent()
            }
        })
        .text_color(if same_caption {
            theme.accent_text
        } else {
            theme.text_soft
        });
        let separate_toggle = button(
            "caption-mode-separate",
            "Separate",
            ButtonKind::Secondary,
            None,
            true,
            cx,
            |app, cx| {
                if app.prepare.x_caption.is_empty() {
                    app.prepare.x_caption = app.prepare.caption.clone();
                }
                app.prepare.same_caption = false;
                app.save_draft();
                cx.notify();
            },
        )
        .h(px(if is_studio { 32.0 } else { 28.0 }))
        .px(px(if is_studio { 9.0 } else { 8.0 }))
        .text_size(px(if is_studio { 12.0 } else { 11.5 }))
        .flex_1()
        .bg(if same_caption {
            if is_studio {
                theme.raised
            } else {
                theme.transparent()
            }
        } else {
            if is_studio {
                theme.accent_soft
            } else {
                theme.active
            }
        })
        .border_color(if same_caption {
            if is_studio {
                theme.border
            } else {
                theme.transparent()
            }
        } else {
            if is_studio {
                theme.accent
            } else {
                theme.border_strong
            }
        })
        .text_color(if same_caption {
            theme.text_soft
        } else {
            theme.accent_text
        });

        let mut column = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(if is_studio { 10.0 } else { 9.0 }))
            .child(
                div()
                    .child(tracked("OUTPUT"))
                    .text_size(px(11.0))
                    .text_color(theme.muted)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                div()
                    .w_full()
                    .mt(px(0.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(compression_combo(self, cx, theme, true)),
                    )
                    .child(
                        div()
                            .h(px(38.0))
                            .px(px(8.0))
                            .rounded(px(2.0))
                            .bg(theme.raised)
                            .border_1()
                            .border_color(theme.border_strong)
                            .flex()
                            .items_center()
                            .child(estimated)
                            .text_size(px(11.5))
                            .text_color(theme.text_soft),
                    ),
            );
        if compression_index == 6 {
            column = column.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .child("Maximum generated size")
                            .text_size(px(if is_studio { 13.0 } else { 12.0 }))
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
                        .h(px(38.0)),
                    ),
            );
        }
        column = column
            .child(divider())
            .child(
                div()
                    .child(tracked("DESTINATIONS"))
                    .text_size(px(11.0))
                    .text_color(theme.muted)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(studio_destination_row(
                theme,
                "deliver-destination-telegram",
                "send",
                "Telegram",
                if telegram_ready {
                    "Ready"
                } else {
                    "Needs setup"
                },
                telegram_detail,
                if telegram_ready {
                    theme.success
                } else {
                    theme.warning
                },
                compact,
                cx,
            ))
            .child(studio_destination_row(
                theme,
                "deliver-destination-x",
                "x-brand",
                "X",
                if x_duration_warning {
                    "Check cut"
                } else {
                    "Manual"
                },
                x_detail,
                if x_duration_warning {
                    theme.warning
                } else {
                    theme.text_soft
                },
                compact,
                cx,
            ))
            .child(
                div()
                    .w_full()
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .gap(px(if is_studio { 6.0 } else { 8.0 }))
                    .child(
                        div()
                            .flex_1()
                            .child(tracked("CAPTIONS"))
                            .text_size(px(if is_studio { 11.0 } else { 10.5 }))
                            .text_color(theme.muted)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .w(px(if is_studio { 196.0 } else { 184.0 }))
                            .h_full()
                            .when(compact, |group| {
                                group
                                    .px(px(2.0))
                                    .py(px(2.0))
                                    .rounded(px(2.0))
                                    .bg(theme.surface_soft)
                                    .border_1()
                                    .border_color(theme.border.opacity(0.72))
                            })
                            .flex()
                            .gap(px(if is_studio { 4.0 } else { 2.0 }))
                            .child(shared_toggle)
                            .child(separate_toggle),
                    ),
            );

        let caption_limit = if mode_index == 1 { 4096 } else { 1024 };
        if same_caption {
            let caption = self.prepare.caption.clone();
            let caption_len = caption.chars().count();
            column = column.child(
                div()
                    .w_full()
                    .h(px(40.0))
                    .relative()
                    .child(caption_area(
                        self,
                        cx,
                        "caption-shared",
                        "Add a caption for Telegram and X (optional)",
                        &caption,
                        40.0,
                        76.0,
                        false,
                        true,
                    ))
                    .child(
                        div()
                            .absolute()
                            .right(px(12.0))
                            .top(px(13.0))
                            .child(format!("{} / 280", group_digits(caption_len)))
                            .text_size(px(11.0))
                            .text_color(if caption_len > caption_limit || caption_len > 280 {
                                theme.warning
                            } else {
                                theme.muted
                            }),
                    ),
            );
        } else {
            let telegram_caption = self.prepare.caption.clone();
            let x_caption = self.prepare.x_caption.clone();
            let telegram_len = telegram_caption.chars().count();
            let x_len = x_caption.chars().count();
            column = column
                .child(
                    div()
                        .child("Telegram")
                        .text_size(px(if is_studio { 12.0 } else { 11.0 }))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(40.0))
                        .relative()
                        .child(caption_area(
                            self,
                            cx,
                            "caption-tg",
                            "Telegram caption (optional)",
                            &telegram_caption,
                            40.0,
                            84.0,
                            false,
                            true,
                        ))
                        .child(
                            div()
                                .absolute()
                                .right(px(12.0))
                                .top(px(13.0))
                                .child(format!(
                                    "{} / {}",
                                    group_digits(telegram_len),
                                    group_digits(caption_limit)
                                ))
                                .text_size(px(11.0))
                                .text_color(if telegram_len > caption_limit {
                                    theme.warning
                                } else {
                                    theme.muted
                                }),
                        ),
                )
                .child(
                    div()
                        .child("X")
                        .text_size(px(if is_studio { 12.0 } else { 11.0 }))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(40.0))
                        .relative()
                        .child(caption_area(
                            self,
                            cx,
                            "caption-x",
                            "X caption (optional)",
                            &x_caption,
                            40.0,
                            76.0,
                            false,
                            true,
                        ))
                        .child(
                            div()
                                .absolute()
                                .right(px(12.0))
                                .top(px(13.0))
                                .child(format!("{} / 280", group_digits(x_len)))
                                .text_size(px(11.0))
                                .text_color(if x_len > 280 {
                                    theme.warning
                                } else {
                                    theme.muted
                                }),
                        ),
                );
        }

        column.child(
            div()
                .mt(px(2.0))
                .min_h(px(44.0))
                .when(is_studio, |row| row.py(px(8.0)))
                .when(compact, |row| row.pt(px(8.0)))
                .border_t_1()
                .border_color(theme.border)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(icon("delivery-info", 16.0, theme.muted))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child("The trimmed clip will be prepared and delivered to the destinations above.")
                        .text_size(px(if is_studio { 11.5 } else { 11.0 }))
                        .text_color(theme.muted)
                        .whitespace_normal()
                        .line_clamp(2),
                ),
        )
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
        let _panel_width = panel_width;

        let mut dock = div()
            .id("action-dock")
            .w_full()
            .bg(theme.surface_soft)
            .border_t_1()
            .border_color(theme.border)
            .px(px(12.0))
            .py(px(12.0))
            .min_h(px(156.0))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(10.0));

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
            if !error.is_empty() {
                dock = dock.child(
                    div()
                        .w_full()
                        .h(px(34.0))
                        .flex()
                        .items_center()
                        .child(format!("Failed · {error}"))
                        .text_size(px(12.0))
                        .text_color(theme.error),
                );
            } else {
                dock =
                    dock.child(
                        div()
                            .w_full()
                            .h(px(34.0))
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .child(div().w(px(7.0)).h(px(7.0)).rounded(px(4.0)).bg(
                                if telegram_ready {
                                    theme.success
                                } else {
                                    theme.warning
                                },
                            ))
                            .child(
                                div()
                                    .child(if telegram_ready {
                                        "Telegram ready"
                                    } else {
                                        "Telegram needs setup"
                                    })
                                    .text_size(px(12.5))
                                    .text_color(if telegram_ready {
                                        theme.text_soft
                                    } else {
                                        theme.warning
                                    }),
                            )
                            .child(div().child("·").text_size(px(12.5)).text_color(theme.muted))
                            .child(
                                div()
                                    .child("X manual")
                                    .text_size(px(12.5))
                                    .text_color(theme.muted),
                            )
                            .when(!estimate_label.is_empty(), |row| {
                                row.child(
                                    div()
                                        .child(format!("·  {estimate_label}"))
                                        .text_size(px(12.5))
                                        .text_color(theme.muted),
                                )
                            }),
                    );
                if telegram_ready {
                    dock = dock.child(
                        div()
                            .w_full()
                            .h(px(66.0))
                            .flex()
                            .gap(px(8.0))
                            .child(
                                button(
                                    "prepare-x",
                                    "Prepare X",
                                    ButtonKind::Secondary,
                                    Some("x-brand"),
                                    true,
                                    cx,
                                    |app, cx| app.submit_publish("x", cx),
                                )
                                .h_full()
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "send-telegram",
                                    "Send Telegram",
                                    ButtonKind::Secondary,
                                    Some("send"),
                                    true,
                                    cx,
                                    |app, cx| app.submit_publish("telegram", cx),
                                )
                                .h_full()
                                .flex_1(),
                            )
                            .child(
                                button(
                                    "send-both",
                                    "Both",
                                    ButtonKind::Primary,
                                    Some("shuffle"),
                                    true,
                                    cx,
                                    |app, cx| app.submit_publish("both", cx),
                                )
                                .h_full()
                                .flex_1(),
                            ),
                    );
                } else {
                    dock = dock.child(
                        div()
                            .w_full()
                            .h(px(66.0))
                            .flex()
                            .gap(px(10.0))
                            .child(
                                button(
                                    "studio-action-back",
                                    "",
                                    ButtonKind::Secondary,
                                    None,
                                    true,
                                    cx,
                                    |app, cx| {
                                        app.prepare.studio_mode = false;
                                        app.prepare.compact_inspector_open = false;
                                        cx.notify();
                                    },
                                )
                                .h_full()
                                .flex_1()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(10.0))
                                        .child(icon("arrow-left", 19.0, theme.text))
                                        .child(
                                            div()
                                                .child("Back to Prepare")
                                                .text_size(px(15.0))
                                                .text_color(theme.text),
                                        ),
                                ),
                            )
                            .child(
                                button(
                                    "prepare-x",
                                    "",
                                    ButtonKind::Primary,
                                    None,
                                    true,
                                    cx,
                                    |app, cx| app.submit_publish("x", cx),
                                )
                                .h_full()
                                .flex_1()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(10.0))
                                        .child(icon("send", 21.0, theme.accent_content))
                                        .child(
                                            div()
                                                .child("Prepare X")
                                                .text_size(px(15.0))
                                                .text_color(theme.accent_content),
                                        ),
                                ),
                            ),
                    );
                }
            }
        }
        dock
    }

    /// Tall docks keep delivery handoff compact, expose the same caption state
    /// as Studio, and use the remaining lower workspace for a concise summary
    /// of what Prepare will produce.
    pub fn render_prepare_dock_summary(
        &mut self,
        cx: &mut Context<Self>,
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
        let shared_caption = self.prepare.same_caption;
        let (caption_id, caption_value) = if shared_caption {
            ("caption-shared", self.prepare.caption.clone())
        } else {
            ("caption-x", self.prepare.x_caption.clone())
        };
        let caption_len = caption_value.chars().count();
        let caption_count = format!("{} / 280", group_digits(caption_len));
        let cut_duration = (self.prepare.trim_end - self.prepare.trim_start).max(0.0);
        let cut_label = if self.prepare.cut_active() {
            format!(
                "Trimmed · {}",
                self.prepare.format_time_precise(cut_duration)
            )
        } else {
            format!(
                "Full range · {}",
                self.prepare.format_time_precise(cut_duration)
            )
        };
        let output_label = {
            let estimate = self.estimate_output_size_label();
            if estimate.is_empty() {
                "Estimate pending".to_string()
            } else {
                estimate
            }
        };
        let fit_label = COMPRESSION_OPTIONS
            .get(self.prepare.compression_index as usize)
            .map(|(label, _)| (*label).to_string())
            .unwrap_or_else(|| "Balanced".to_string());
        let caption_editor = caption_area(
            self,
            cx,
            caption_id,
            "Add a caption (optional)",
            &caption_value,
            36.0,
            58.0,
            true,
            false,
        );

        div()
            .id("prepare-dock-summary")
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .scrollbar_width(px(8.0))
            .border_t_1()
            .border_color(theme.border)
            .child(prepare_dock_destination_row(
                theme,
                "dock-destination-x",
                "X",
                "x",
                "Prepared for deliberate browser handoff".to_string(),
                "Manual",
                theme.text_soft,
                cx,
            ))
            .child(prepare_dock_destination_row(
                theme,
                "dock-destination-telegram",
                "Telegram",
                "send",
                telegram_detail,
                if telegram_ready {
                    "Ready"
                } else {
                    "Needs setup"
                },
                if telegram_ready {
                    theme.success
                } else {
                    theme.warning
                },
                cx,
            ))
            .child(
                div()
                    .w_full()
                    .h(px(54.0))
                    .flex_none()
                    .px(px(12.0))
                    .py(px(9.0))
                    .relative()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .h(px(36.0))
                            .flex_none()
                            .relative()
                            .flex()
                            .child(caption_editor)
                            .child(tabular(
                                div()
                                    .absolute()
                                    .right(px(11.0))
                                    .top(px(10.0))
                                    .child(caption_count)
                                    .text_size(px(10.5))
                                    .text_color(if caption_len > 280 {
                                        theme.warning
                                    } else {
                                        theme.muted
                                    }),
                            )),
                    ),
            )
            .child(
                div()
                    .id("dock-preparation-summary")
                    .w_full()
                    .h(px(80.0))
                    .flex_none()
                    .px(px(12.0))
                    .py(px(10.0))
                    .border_t_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(9.0))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .child("PREPARATION SUMMARY")
                                    .text_size(px(9.5))
                                    .text_color(theme.muted)
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.0))
                                    .child(icon("check", 12.0, theme.success))
                                    .child(
                                        div()
                                            .child("Source remains unchanged")
                                            .text_size(px(10.0))
                                            .text_color(theme.text_soft),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(38.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .child("CUT")
                                            .text_size(px(9.0))
                                            .text_color(theme.muted_soft)
                                            .font_weight(FontWeight::SEMIBOLD),
                                    )
                                    .child(tabular(
                                        div()
                                            .child(cut_label)
                                            .text_size(px(10.5))
                                            .text_color(theme.text_soft)
                                            .text_ellipsis(),
                                    )),
                            )
                            .child(div().w(px(1.0)).h(px(30.0)).bg(theme.border))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .pl(px(10.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .child("OUTPUT")
                                            .text_size(px(9.0))
                                            .text_color(theme.muted_soft)
                                            .font_weight(FontWeight::SEMIBOLD),
                                    )
                                    .child(
                                        div()
                                            .child(output_label)
                                            .text_size(px(10.5))
                                            .text_color(theme.text_soft)
                                            .text_ellipsis(),
                                    ),
                            )
                            .child(div().w(px(1.0)).h(px(30.0)).bg(theme.border))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .pl(px(10.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .child("FIT POLICY")
                                            .text_size(px(9.0))
                                            .text_color(theme.muted_soft)
                                            .font_weight(FontWeight::SEMIBOLD),
                                    )
                                    .child(
                                        div()
                                            .child(fit_label)
                                            .text_size(px(10.5))
                                            .text_color(theme.text_soft)
                                            .text_ellipsis(),
                                    ),
                            ),
                    ),
            )
    }

    /// Laptop-height docks keep a compact destination handoff visible instead
    /// of leaving the lower panel blank or duplicating Studio's form controls.
    pub fn render_prepare_dock_compact_summary(&self, theme: &crate::theme::Theme) -> impl Element {
        let telegram_ready = self.telegram_ready();
        let destination =
            |glyph: &'static str, label: &'static str, state: &'static str, state_color: Hsla| {
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
                            .h(px(22.0))
                            .px(px(8.0))
                            .rounded(px(11.0))
                            .bg(state_color.opacity(0.10))
                            .border_1()
                            .border_color(state_color.opacity(0.22))
                            .flex()
                            .items_center()
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
            .flex_none()
            .w_full()
            .mt_auto()
            .bg(theme.canvas_background())
            .border_t_1()
            .border_color(theme.border)
            .px(px(PREPARE_GUTTER))
            .py(px(6.0))
            .flex()
            .flex_col()
            .gap(px(6.0));

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
                .h(px(36.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    button(
                        "dock-open-studio",
                        "Open Studio",
                        ButtonKind::Secondary,
                        Some("maximize"),
                        true,
                        cx,
                        |app, cx| {
                            app.open_selected_in_studio(cx);
                        },
                    )
                    .h(px(36.0))
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
                    .h(px(36.0))
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
                .text_size(px(14.0))
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
        .rounded(px(MENU_RADIUS))
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
    compact: bool,
) -> impl Element {
    let selected = app.prepare.compression_index as usize;
    let open = app.open_combos.contains("compression");
    let trigger = div()
        .id("compression-trigger")
        .w_full()
        .h(px(if compact {
            38.0
        } else {
            PREPARE_CONTROL_HEIGHT
        }))
        .px(px(if compact { 10.0 } else { 12.0 }))
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
                .text_size(px(if compact { 12.0 } else { 13.0 }))
                .text_color(theme.text),
        )
        .child(icon(if open { "▴" } else { "▾" }, 14.0, theme.muted))
        .on_click(cx.listener(|app, _event, _window, cx| {
            app.toggle_combo("compression", cx);
        }));
    if !open {
        return trigger;
    }
    let mut menu = div()
        .id("compression")
        .w_full()
        .rounded(px(MENU_RADIUS))
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
                    .h(px(if compact {
                        38.0
                    } else {
                        PREPARE_CONTROL_HEIGHT
                    }))
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
                            .text_size(px(14.0))
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
    compact: bool,
) -> impl Element {
    let options: [&str; 2] = ["Bot", "Personal"];
    let selected = app.prepare.telegram_mode_index as usize;
    let open = app.open_combos.contains("tg-mode");
    let trigger = div()
        .id("tg-mode-trigger")
        .w(px(if compact { 112.0 } else { 126.0 }))
        .h(px(if compact {
            38.0
        } else {
            PREPARE_CONTROL_HEIGHT
        }))
        .px(px(if compact { 10.0 } else { 12.0 }))
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
                .text_size(px(if compact { 12.0 } else { 13.0 }))
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
        .w(px(if compact { 112.0 } else { 126.0 }))
        .rounded(px(MENU_RADIUS))
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
                .h(px(if compact {
                    38.0
                } else {
                    PREPARE_CONTROL_HEIGHT
                }))
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
    compact: bool,
) -> impl Element {
    let selected = app.prepare.cleanup_index as usize;
    let open = app.open_combos.contains("cleanup");
    let trigger = div()
        .id("cleanup-trigger")
        .w(px(if compact { 172.0 } else { 184.0 }))
        .h(px(if compact {
            38.0
        } else {
            PREPARE_CONTROL_HEIGHT
        }))
        .px(px(if compact { 10.0 } else { 12.0 }))
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
                .text_size(px(if compact { 12.0 } else { 13.0 }))
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
        .w(px(if compact { 172.0 } else { 184.0 }))
        .rounded(px(MENU_RADIUS))
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
                .h(px(if compact {
                    38.0
                } else {
                    PREPARE_CONTROL_HEIGHT
                }))
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

#[allow(clippy::too_many_arguments)]
fn caption_area(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    id: &'static str,
    placeholder: &'static str,
    value: &str,
    height: f32,
    right_padding: f32,
    compact_dock: bool,
    subtle_focus: bool,
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
    let mut field = crate::widgets::text_area(
        id,
        placeholder,
        height,
        &display,
        app.focused_field.as_deref() == Some(id),
        subtle_focus,
        cx,
    );
    if right_padding > 0.0 {
        field = field.pr(px(right_padding));
    }
    if compact_dock {
        field = field
            .rounded(px(2.0))
            .bg(app.theme.transparent())
            .border_color(app.theme.border_strong.opacity(0.72))
            .py(px(0.0))
            .flex()
            .items_center();
    }
    field
}

impl crate::App {
    fn rotate_prepare(&mut self, turns: i32, cx: &mut Context<Self>) {
        if self.checking {
            return;
        }
        self.prepare.rotate(turns);
        if let Some(video) = &self.prepare.video {
            crate::video_element::set_rotation(video, self.prepare.rotation);
            // A paused frame must also be decoded again with its new orientation.
            self.prepare
                .seek(self.prepare.position, self.prepare.duration);
        }
        self.save_draft();
        cx.notify();
    }

    pub fn prepare_crop_preset(&self) -> usize {
        let crop = self.prepare.crop;
        let full_frame = (crop.width - 1.0).abs() < 0.001 && (crop.height - 1.0).abs() < 0.001;
        if !self.prepare.crop_enabled || full_frame {
            1
        } else {
            let source_ratio = self
                .selected
                .as_ref()
                .map(|media| {
                    if media.width > 0 && media.height > 0 {
                        media.width as f64 / media.height as f64
                    } else {
                        16.0 / 9.0
                    }
                })
                .unwrap_or(16.0 / 9.0);
            let visible_ratio =
                self.prepare.oriented_ratio(source_ratio) * crop.width / crop.height.max(0.001);
            if (visible_ratio - 1.0).abs() < 0.02 {
                2
            } else if (visible_ratio - 16.0 / 9.0).abs() < 0.03 {
                3
            } else if (visible_ratio - 9.0 / 16.0).abs() < 0.03 {
                4
            } else {
                0
            }
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
        let source_ratio = self.prepare.oriented_ratio(source_ratio);
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
    use super::{fitted_media_rect, group_digits, prepare_frame_height};

    #[test]
    fn studio_stage_balances_media_and_the_editing_workbench() {
        assert!((prepare_frame_height(true, false, 1404.0, 1580.0) - 916.4).abs() < 0.01);
        assert_eq!(prepare_frame_height(true, false, 941.0, 1176.0), 548.0);
        assert_eq!(prepare_frame_height(true, false, 760.0, 880.0), 367.0);
        assert_eq!(prepare_frame_height(true, true, 520.0, 600.0), 118.0);
    }

    #[test]
    fn dock_stage_yields_to_the_persistent_inspector() {
        assert_eq!(prepare_frame_height(false, false, 760.0, 370.0), 220.0);
        assert_eq!(prepare_frame_height(false, false, 1440.0, 592.0), 380.0);
        assert_eq!(prepare_frame_height(false, false, 1440.0, 900.0), 380.0);
    }

    #[test]
    fn edit_overlays_follow_the_contained_video_pixels() {
        let portrait = fitted_media_rect(1000.0, 500.0, 9.0 / 16.0);
        assert_eq!(portrait.1, 0.0);
        assert_eq!(portrait.3, 500.0);
        assert!((portrait.2 - 281.25).abs() < 0.01);
        assert!((portrait.0 - 359.375).abs() < 0.01);

        let landscape = fitted_media_rect(500.0, 500.0, 16.0 / 9.0);
        assert_eq!(landscape.0, 0.0);
        assert_eq!(landscape.2, 500.0);
        assert!((landscape.3 - 281.25).abs() < 0.01);
        assert!((landscape.1 - 109.375).abs() < 0.01);
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
