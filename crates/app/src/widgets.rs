//! Shared UI widgets — theme-aware primitives matching the design system
//! (44px targets, 2px focus rings, and shallow tactile control surfaces).

use crate::theme::*;
use gpui::prelude::*;
use gpui::*;
use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

/// A tiny text-field state kept on the App view.
#[derive(Clone, Debug, Default)]
pub struct FieldState {
    pub text: String,
    pub caret: usize,
    pub committed: bool,
    pub marked_range: Option<Range<usize>>,
}

pub fn icon(glyph: &'static str, size: f32, color: Hsla) -> Div {
    let container = div()
        .w(px(size))
        .h(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center();
    if let Some(path) = crate::icons::icon_path(glyph) {
        container.child(svg().path(path).w(px(size)).h(px(size)).text_color(color))
    } else {
        container.child(glyph).text_size(px(size)).text_color(color)
    }
}

#[derive(Clone)]
struct ButtonVisual {
    rest: Background,
    hover: Background,
    pressed: Background,
    rest_edge: Hsla,
    hover_edge: Hsla,
    pressed_edge: Hsla,
    rest_shadow: Vec<BoxShadow>,
    hover_shadow: Vec<BoxShadow>,
    pressed_shadow: Vec<BoxShadow>,
}

fn button_visual(theme: &crate::theme::Theme, kind: ButtonKind, compact: bool) -> ButtonVisual {
    let transparent: Background = theme.transparent().into();
    let neutral_edge = theme.tactile_edge(TactileState::Rest, false);
    let (rest, hover, pressed, rest_edge, hover_edge, pressed_edge, elevated) = match kind {
        ButtonKind::Primary => (
            theme.accent_control_face(TactileState::Rest),
            theme.accent_control_face(TactileState::Hover),
            theme.accent_control_face(TactileState::Pressed),
            theme.tactile_edge(TactileState::Rest, true),
            theme.tactile_edge(TactileState::Hover, true),
            theme.tactile_edge(TactileState::Pressed, true),
            true,
        ),
        ButtonKind::Secondary => (
            theme.control_face(TactileState::Rest),
            theme.control_face(TactileState::Hover),
            theme.control_face(TactileState::Pressed),
            neutral_edge,
            theme.tactile_edge(TactileState::Hover, false),
            theme.tactile_edge(TactileState::Pressed, false),
            true,
        ),
        ButtonKind::Ghost => (
            transparent,
            theme.control_face(TactileState::Hover),
            theme.control_face(TactileState::Pressed),
            theme.transparent(),
            theme.tactile_edge(TactileState::Hover, false),
            theme.tactile_edge(TactileState::Pressed, false),
            false,
        ),
        ButtonKind::Danger => (
            transparent,
            theme.danger_control_face(TactileState::Hover),
            theme.danger_control_face(TactileState::Pressed),
            theme.transparent(),
            theme.error.opacity(0.48),
            theme.error.opacity(0.36),
            false,
        ),
    };
    ButtonVisual {
        rest,
        hover,
        pressed,
        rest_edge,
        hover_edge,
        pressed_edge,
        rest_shadow: if elevated {
            theme.tactile_shadow(TactileState::Rest, compact)
        } else {
            Vec::new()
        },
        hover_shadow: theme.tactile_shadow(TactileState::Hover, compact),
        pressed_shadow: theme.tactile_shadow(TactileState::Pressed, compact),
    }
}

#[allow(dead_code)]
pub fn label(text: impl Into<SharedString>, size: f32, color: Hsla, weight: FontWeight) -> Div {
    div()
        .child(text.into())
        .text_size(px(size))
        .text_color(color)
        .font_weight(weight)
}

impl FieldState {
    /// Byte offset of the `char_index`-th character (caret is a char index).
    pub(crate) fn char_to_byte(text: &str, char_index: usize) -> usize {
        text.char_indices()
            .nth(char_index)
            .map(|(i, _)| i)
            .unwrap_or(text.len())
    }

    pub(crate) fn char_to_utf16(text: &str, char_index: usize) -> usize {
        text.chars().take(char_index).map(char::len_utf16).sum()
    }

    pub(crate) fn utf16_to_char(text: &str, utf16_index: usize) -> usize {
        let mut units = 0;
        for (index, ch) in text.chars().enumerate() {
            let next = units + ch.len_utf16();
            if next > utf16_index {
                return index;
            }
            units = next;
        }
        text.chars().count()
    }

    pub fn insert(&mut self, ch: char) {
        let byte = Self::char_to_byte(&self.text, self.caret);
        self.text.insert(byte, ch);
        self.caret = (self.caret + 1).min(self.text.chars().count());
    }

    pub fn insert_str(&mut self, s: &str) {
        let byte = Self::char_to_byte(&self.text, self.caret);
        self.text.insert_str(byte, s);
        self.caret += s.chars().count();
    }

    pub fn backspace(&mut self) {
        if self.caret > 0 {
            self.caret -= 1;
            let byte = Self::char_to_byte(&self.text, self.caret);
            if byte >= self.text.len() {
                self.text.pop();
            } else {
                self.text.remove(byte);
            }
        }
    }

    pub fn delete(&mut self) {
        let char_count = self.text.chars().count();
        if self.caret < char_count {
            let byte = Self::char_to_byte(&self.text, self.caret);
            self.text.remove(byte);
        }
    }

    pub fn move_left(&mut self) {
        if self.caret > 0 {
            self.caret -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.caret < self.text.chars().count() {
            self.caret += 1;
        }
    }

    pub fn rendered(&self) -> String {
        let byte = Self::char_to_byte(&self.text, self.caret);
        format!("{}▏{}", &self.text[..byte], &self.text[byte..])
    }
}

/// Transparent element that registers the focused App field with GPUI's
/// platform input bridge. This enables native IME/composition without adding
/// another visible layout node or a second field state model.
struct PlatformInputElement {
    app: Entity<crate::App>,
}

impl IntoElement for PlatformInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PlatformInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut gpui::App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        let current_focus = window.focused(cx);
        self.app.update(cx, |app, _cx| {
            app.platform_input_bounds = Some(bounds);
            if app.platform_input_focus.is_none() {
                app.platform_input_focus = current_focus;
            }
        });
        if let Some(focus_handle) = self.app.read(cx).platform_input_focus.clone() {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(bounds, self.app.clone()),
                cx,
            );
        }
    }
}

/// Standard action button. The visible control stays compact while the
/// surrounding layout provides a comfortable pointer target.
pub fn button(
    id: impl Into<SharedString>,
    label: &str,
    kind: ButtonKind,
    icon: Option<&'static str>,
    enabled: bool,
    cx: &mut Context<crate::App>,
    on_click: impl Fn(&mut crate::App, &mut Context<crate::App>) + 'static,
) -> Stateful<Div> {
    let on_click = Rc::new(on_click);
    let click = Rc::clone(&on_click);
    let activate = Rc::clone(&on_click);
    let theme = current_theme();
    let visual = button_visual(&theme, kind, false);
    let focus_fill = visual.hover;
    let focus_shadow = visual.hover_shadow.clone();
    button_base(id, label, kind, icon, enabled).when(enabled, |this| {
        this.tab_index(0)
            .focus(move |style| {
                style
                    .bg(focus_fill)
                    .shadow(focus_shadow.clone())
                    .border_2()
                    .border_color(current_theme().accent)
            })
            .on_click(cx.listener(move |app, _event, window, cx| {
                // Pointer presses use the transient `active` fill below. Drop
                // pointer-acquired focus before invoking the action so the
                // highlight cannot persist, while allowing the action to focus
                // a field or popup when that is its intended result.
                window.blur();
                click(app, cx);
            }))
            .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                activate(app, cx);
                cx.stop_propagation();
            }))
            .on_action(
                cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                    on_click(app, cx);
                    cx.stop_propagation();
                }),
            )
    })
}

/// `button` variant whose callback also receives the click position, for
/// popups that anchor to the clicked control.
#[allow(clippy::too_many_arguments)]
pub fn button_at(
    id: impl Into<SharedString>,
    label: &str,
    kind: ButtonKind,
    icon: Option<&'static str>,
    enabled: bool,
    cx: &mut Context<crate::App>,
    on_click: impl Fn(&mut crate::App, Point<Pixels>, &mut Context<crate::App>) + 'static,
) -> Stateful<Div> {
    button_base(id, label, kind, icon, enabled).when(enabled, |this| {
        this.on_click(cx.listener(move |app, event: &ClickEvent, _window, cx| {
            on_click(app, event.position(), cx)
        }))
    })
}

fn button_base(
    id: impl Into<SharedString>,
    label: &str,
    kind: ButtonKind,
    icon: Option<&'static str>,
    enabled: bool,
) -> Stateful<Div> {
    let theme = current_theme();
    let visual = button_visual(&theme, kind, false);
    let id: SharedString = id.into();
    let icon_color = if !enabled {
        theme.muted
    } else {
        match kind {
            ButtonKind::Primary => theme.accent_content,
            ButtonKind::Danger => theme.error,
            _ => theme.text,
        }
    };
    let mut element = div()
        .id(id)
        .h(px(CONTROL_HEIGHT))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .relative()
        .top(px(0.0))
        .border_1()
        .border_color(visual.rest_edge)
        .bg(visual.rest)
        .shadow(visual.rest_shadow.clone())
        .flex()
        .items_center()
        .justify_center()
        .gap(px(7.0))
        .cursor_pointer()
        .text_size(px(13.0))
        .font_weight(FontWeight::MEDIUM);
    match kind {
        ButtonKind::Primary => {
            element = element.text_color(theme.accent_content).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                }
            });
        }
        ButtonKind::Secondary => {
            element = element.text_color(theme.text).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                }
            });
        }
        ButtonKind::Ghost => {
            element = element.text_color(theme.text).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                }
            });
        }
        ButtonKind::Danger => {
            element = element.text_color(theme.error).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                }
            });
        }
    }
    if let Some(icon) = icon {
        element = element.child(self::icon(icon, 15.0, icon_color));
    }
    element = element.child(
        div()
            .child(label.to_string())
            .min_w(px(0.0))
            .text_ellipsis(),
    );
    let pressed_shadow = visual.pressed_shadow.clone();
    element
        .when(enabled, |this| {
            // The one-pixel travel and compressed contact shadow make the
            // pointer state read as a physical press without changing layout.
            this.active(move |style| {
                style
                    .top(px(1.0))
                    .bg(visual.pressed)
                    .border_color(visual.pressed_edge)
                    .shadow(pressed_shadow.clone())
            })
        })
        // Disabled: neutral raised fill + muted text (opacity alone washes
        // colored buttons out on light themes).
        .when(!enabled, |this| {
            this.bg(theme.raised)
                .border_color(theme.border.opacity(0.64))
                .shadow_none()
                .text_color(theme.muted)
                .cursor_default()
        })
}

/// Compact toolbar button (28px; icon-only or icon+label).
#[allow(clippy::too_many_arguments)]
pub fn workbench_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    icon: &'static str,
    kind: ButtonKind,
    enabled: bool,
    icon_only: bool,
    tooltip: impl Into<SharedString>,
    cx: &mut Context<crate::App>,
    on_click: impl Fn(&mut crate::App, &mut Context<crate::App>) + 'static,
) -> Stateful<Div> {
    let theme = current_theme();
    let visual = button_visual(&theme, kind, true);
    let tooltip = tooltip.into();
    let id: SharedString = id.into();
    let icon_color = if !enabled {
        theme.muted_soft
    } else {
        match kind {
            ButtonKind::Primary => theme.accent_content,
            ButtonKind::Danger => theme.error,
            ButtonKind::Ghost => theme.muted,
            ButtonKind::Secondary => theme.text,
        }
    };
    let mut element = div()
        .id(id)
        .flex_none()
        .h(px(WORKBENCH_CONTROL_HEIGHT))
        .when(icon_only, |this| {
            this.w(px(WORKBENCH_CONTROL_HEIGHT)).px(px(0.0))
        })
        .when(!icon_only, |this| this.px(px(9.0)))
        .rounded(px(RADIUS_SM))
        .relative()
        .top(px(0.0))
        .border_1()
        .border_color(visual.rest_edge)
        .bg(visual.rest)
        .shadow(visual.rest_shadow.clone())
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .cursor_pointer()
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM);
    match kind {
        ButtonKind::Primary => {
            element = element.text_color(theme.accent_content).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                }
            });
        }
        ButtonKind::Secondary => {
            element = element.text_color(theme.text).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                }
            });
        }
        ButtonKind::Ghost => {
            element = element.text_color(theme.muted).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                        .text_color(theme.text)
                }
            });
        }
        ButtonKind::Danger => {
            element = element.text_color(theme.error).hover({
                let hover_shadow = visual.hover_shadow.clone();
                move |style| {
                    style
                        .bg(visual.hover)
                        .border_color(visual.hover_edge)
                        .shadow(hover_shadow.clone())
                }
            });
        }
    }
    element = element.child(self::icon(icon, 15.0, icon_color));
    if !icon_only {
        element = element.child(div().min_w(px(0.0)).text_ellipsis().child(label.into()));
    }
    let on_click = Rc::new(on_click);
    let click = Rc::clone(&on_click);
    let activate = Rc::clone(&on_click);
    let pressed_shadow = visual.pressed_shadow.clone();
    let focus_fill = visual.hover;
    let focus_shadow = visual.hover_shadow.clone();
    element
        .tooltip(move |_window, cx| crate::tooltip_view(cx, tooltip.clone()))
        .when(!enabled, |this| this.opacity(0.42).cursor_default())
        .when(enabled, |this| {
            this.active(move |style| {
                style
                    .top(px(1.0))
                    .bg(visual.pressed)
                    .border_color(visual.pressed_edge)
                    .shadow(pressed_shadow.clone())
            })
            .tab_index(0)
            .focus(move |style| {
                style
                    .bg(focus_fill)
                    .shadow(focus_shadow.clone())
                    .border_2()
                    .border_color(current_theme().accent)
            })
            .on_click(cx.listener(move |app, _event, window, cx| {
                window.blur();
                click(app, cx);
            }))
            .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                activate(app, cx);
                cx.stop_propagation();
            }))
            .on_action(cx.listener(
                move |app, _: &crate::ActivateSpace, _window, cx| {
                    on_click(app, cx);
                    cx.stop_propagation();
                },
            ))
        })
}

/// Text input field (44px, raised, 2px accent focus ring). Keyboard input is
/// handled by the view's key dispatch (see `App::on_key_down`).
pub fn field(
    id: &'static str,
    placeholder: &str,
    state: &FieldState,
    focused: bool,
    enabled: bool,
    password: bool,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    field_with_icon(id, placeholder, state, focused, enabled, password, None, cx)
}

/// Insert hair spaces between glyphs to approximate the original's
/// `font.letterSpacing` on uppercase section headers (gpui 0.2 has no
/// tracking API). The hair space ≈ the 1.2–1.3px letter spacing.
pub fn tracked(text: &str) -> String {
    text.chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("\u{200A}")
}

/// Apply the tabular-figures OpenType feature (tnum) so numeric text uses
/// fixed-width digits (matches the original's `font.features: { "tnum": 1 }`
/// on time codes, durations, and counts).
pub fn tabular<E: Styled + 'static>(element: E) -> E {
    element.font(gpui::Font {
        features: gpui::FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])),
        ..gpui::font(crate::platform_ui_font_family())
    })
}

/// `field` with an optional leading glyph (used by the command center).
#[allow(clippy::too_many_arguments)]
pub fn field_with_icon(
    id: &'static str,
    placeholder: &str,
    state: &FieldState,
    focused: bool,
    enabled: bool,
    password: bool,
    icon_glyph: Option<&'static str>,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    field_with_icon_hint(
        id,
        placeholder,
        state,
        focused,
        enabled,
        password,
        icon_glyph,
        None,
        cx,
    )
}

/// `field_with_icon` plus an optional right-side hint chip (e.g. ⌘K).
#[allow(clippy::too_many_arguments)]
pub fn field_with_icon_hint(
    id: &'static str,
    placeholder: &str,
    state: &FieldState,
    focused: bool,
    enabled: bool,
    password: bool,
    icon_glyph: Option<&'static str>,
    hint: Option<&'static str>,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    let theme = current_theme();
    let mut element = div()
        .id(id)
        .flex_1()
        .h(px(CONTROL_HEIGHT))
        .px(px(13.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if focused { theme.accent } else { theme.border })
        .hover(|style| style.border_color(theme.border_strong))
        .flex()
        .items_center()
        .gap(px(7.0))
        .text_size(px(13.0))
        .text_color(theme.text)
        .cursor_text();
    if focused {
        element = element.relative().child(
            div()
                .absolute()
                .inset_0()
                .child(PlatformInputElement { app: cx.entity() }),
        );
    }
    if focused {
        element = element.border_2().border_color(theme.accent);
    }
    if let Some(glyph) = icon_glyph {
        element = element.child(icon(glyph, 14.0, theme.muted));
    }
    let display = if password && !state.text.is_empty() {
        "•".repeat(state.text.chars().count())
    } else if state.text.is_empty() {
        placeholder.to_string()
    } else {
        state.rendered()
    };
    element = element.child(
        div()
            .child(display)
            .text_color(if state.text.is_empty() {
                theme.muted
            } else {
                theme.text
            })
            .text_ellipsis(),
    );
    if let Some(hint) = hint {
        element = element.child(
            div()
                .flex_none()
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .child(hint.to_string())
                .text_size(px(10.0))
                .text_color(theme.muted_soft),
        );
    }
    element
        .when(!enabled, |this| this.opacity(0.46).cursor_default())
        .when(enabled, |this| {
            this.tab_index(0)
                .focus(|style| style.border_2().border_color(current_theme().accent))
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.focus_field(id, cx);
                }))
                .on_key_down(cx.listener(move |app, event, window, cx| {
                    if app.focused_field.as_deref() != Some(id) {
                        app.focus_field(id, cx);
                    }
                    app.on_key_down(event, window, cx);
                    cx.stop_propagation();
                }))
                .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                    app.focus_field(id, cx);
                    app.handle_field_key(id, "enter", None, false, cx);
                    cx.stop_propagation();
                }))
                .on_action(
                    cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                        app.focus_field(id, cx);
                        app.handle_field_key(id, "space", Some(" "), false, cx);
                        cx.stop_propagation();
                    }),
                )
        })
}

/// Multiline-ish caption area (fixed height, wraps).
pub fn text_area(
    id: &'static str,
    placeholder: &str,
    height: f32,
    state: &FieldState,
    focused: bool,
    subtle_focus: bool,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    let theme = current_theme();
    let focus_border = if subtle_focus {
        theme.border_strong
    } else {
        theme.accent
    };
    let mut element = div()
        .id(id)
        .flex_1()
        .h(px(height))
        .px(px(13.0))
        .py(px(11.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if focused { focus_border } else { theme.border })
        .text_size(px(13.0))
        .text_color(theme.text)
        .cursor_text();
    if focused {
        element = element.relative().child(
            div()
                .absolute()
                .inset_0()
                .child(PlatformInputElement { app: cx.entity() }),
        );
    }
    if focused {
        element = if subtle_focus {
            element
                .bg(theme.hover)
                .border_1()
                .border_color(focus_border)
        } else {
            element.border_2().border_color(focus_border)
        };
    }
    let display = if state.text.is_empty() {
        placeholder.to_string()
    } else {
        state.rendered()
    };
    element = element.child(
        div()
            .w_full()
            .child(display)
            .text_color(if state.text.is_empty() {
                theme.muted
            } else {
                theme.text
            }),
    );
    element
        .tab_index(0)
        .focus(move |style| {
            if subtle_focus {
                style
                    .bg(current_theme().hover)
                    .border_1()
                    .border_color(current_theme().border_strong)
            } else {
                style.border_2().border_color(current_theme().accent)
            }
        })
        .on_click(cx.listener(move |app, _event, _window, cx| {
            app.focus_field(id, cx);
        }))
        .on_key_down(cx.listener(move |app, event, window, cx| {
            if app.focused_field.as_deref() != Some(id) {
                app.focus_field(id, cx);
            }
            app.on_key_down(event, window, cx);
            cx.stop_propagation();
        }))
        .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
            app.focus_field(id, cx);
            app.handle_field_key(id, "enter", None, false, cx);
            cx.stop_propagation();
        }))
        .on_action(
            cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                app.focus_field(id, cx);
                app.handle_field_key(id, "space", Some(" "), false, cx);
                cx.stop_propagation();
            }),
        )
}

/// Checkbox with label (21px indicator, 44px min target).
pub fn checkbox(
    id: &'static str,
    label: &str,
    checked: bool,
    enabled: bool,
    cx: &mut Context<crate::App>,
    on_toggle: impl Fn(&mut crate::App, &mut Context<crate::App>, bool) + 'static,
) -> Stateful<Div> {
    let theme = current_theme();
    let indicator = div()
        .id(SharedString::from(format!("{id}-indicator")))
        .flex_none()
        .w(px(21.0))
        .h(px(21.0))
        .rounded(px(5.0))
        .relative()
        .top(px(0.0))
        .border_1()
        .border_color(if checked {
            theme.tactile_edge(TactileState::Rest, true)
        } else {
            theme.tactile_edge(TactileState::Rest, false)
        })
        .bg(if checked {
            theme.accent_control_face(TactileState::Rest)
        } else {
            theme.control_face(TactileState::Rest)
        })
        .shadow(theme.tactile_shadow(TactileState::Rest, true))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.0))
        .text_color(theme.accent_content)
        .hover(|style| {
            style
                .bg(theme.accent_control_face(TactileState::Hover))
                .border_color(theme.tactile_edge(TactileState::Hover, true))
                .shadow(theme.tactile_shadow(TactileState::Hover, true))
        })
        .active(|style| {
            style
                .top(px(1.0))
                .bg(theme.accent_control_face(TactileState::Pressed))
                .border_color(theme.tactile_edge(TactileState::Pressed, true))
                .shadow(theme.tactile_shadow(TactileState::Pressed, true))
        })
        .when(checked, |this| {
            this.child(icon("check", 13.0, theme.accent_content))
        });
    let on_toggle = Rc::new(on_toggle);
    let click = Rc::clone(&on_toggle);
    let activate = Rc::clone(&on_toggle);
    div()
        .id(id)
        .h(px(CONTROL_HEIGHT))
        .px(px(4.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .cursor_pointer()
        .opacity(if enabled { 1.0 } else { 0.45 })
        .child(indicator)
        .child(label.to_string())
        .text_size(px(13.0))
        .text_color(theme.text)
        .font_weight(FontWeight::MEDIUM)
        .when(enabled, |this| {
            this.tab_index(0)
                .focus(|style| style.border_2().border_color(current_theme().accent))
        })
        .on_click(cx.listener(move |app, _event, _window, cx| {
            if enabled {
                click(app, cx, !checked)
            }
        }))
        .when(enabled, |this| {
            this.on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                activate(app, cx, !checked);
                cx.stop_propagation();
            }))
            .on_action(cx.listener(
                move |app, _: &crate::ActivateSpace, _window, cx| {
                    on_toggle(app, cx, !checked);
                    cx.stop_propagation();
                },
            ))
        })
}

/// Status pill (28px, icon + label).
pub fn status_pill(id: &'static str, label: &str, state: PillState) -> impl Element {
    let theme = current_theme();
    let (bg, color, glyph) = match state {
        PillState::Neutral => (theme.raised, theme.text_soft, "i"),
        PillState::Success => (theme.success_soft, theme.success, "✓"),
        PillState::Warning => (theme.warning_soft, theme.warning, "!"),
        PillState::Error => (theme.error_soft, theme.error, "!"),
    };
    div()
        .id(id)
        .h(px(26.0))
        .px(px(10.0))
        .rounded(px(13.0))
        .bg(bg)
        .border_1()
        .border_color(color.opacity(0.22))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(icon(glyph, 13.0, color))
        .child(label.to_string())
        .text_size(px(12.0))
        .text_color(color)
        .font_weight(FontWeight::SEMIBOLD)
}

/// Bounded progress bar (7px).
pub fn progress_bar(value: f64, indeterminate: bool) -> impl Element {
    let theme = current_theme();
    div()
        .h(px(7.0))
        .w_full()
        .rounded(px(4.0))
        .bg(theme.raised)
        .overflow_hidden()
        .relative()
        .child(if indeterminate {
            div()
                .absolute()
                .top(px(0.0))
                .left(px(-60.0))
                .h_full()
                .w(px(60.0))
                .bg(theme.accent)
                .rounded(px(4.0))
                .with_animation(
                    "progress-sweep",
                    Animation::new(Duration::from_millis(1400)).repeat(),
                    |this, delta| this.left(px(delta * 240.0 - 60.0)),
                )
                .into_any()
        } else {
            div()
                .h_full()
                .w(px((value.clamp(0.0, 1.0) * 100.0) as f32))
                .bg(theme.accent)
                .rounded(px(4.0))
                .into_any()
        })
}

/// 120 ms ease-in-out fade applied to popups when they mount.
pub fn popup_fade<E: Styled + IntoElement + 'static>(
    element: E,
    key: &'static str,
) -> AnimationElement<E> {
    element.with_animation(
        key,
        Animation::new(Duration::from_millis(120)).with_easing(ease_in_out),
        |this, delta| this.opacity(delta),
    )
}

pub fn divider() -> Div {
    div().h(px(1.0)).w_full().bg(current_theme().border)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PillState {
    Neutral,
    Success,
    Warning,
    Error,
}

// Theme lookup shared by widgets (set by the app during render).
thread_local! {
    static CURRENT_THEME: std::cell::RefCell<crate::theme::Theme> =
        std::cell::RefCell::new(crate::theme::Theme::relay());
}

pub fn set_current_theme(theme: &crate::theme::Theme) {
    CURRENT_THEME.with(|cell| *cell.borrow_mut() = theme.clone());
}

pub fn current_theme() -> crate::theme::Theme {
    CURRENT_THEME.with(|cell| cell.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::FieldState;

    fn assert_renders(state: &FieldState) {
        // rendered() must never slice mid-character.
        let _ = state.rendered();
    }

    #[test]
    fn caret_survives_multibyte_input() {
        let mut state = FieldState::default();
        for ch in "héllo 😀 world".chars() {
            state.insert(ch);
            assert_renders(&state);
            assert_eq!(state.caret, state.text.chars().count());
        }
        assert_eq!(state.text, "héllo 😀 world");
        // Move to the middle (after "hé") and edit there.
        state.caret = 2;
        state.insert('x');
        assert_eq!(state.text, "héxllo 😀 world");
        assert_renders(&state);
        state.backspace();
        assert_eq!(state.text, "héllo 😀 world");
        assert_renders(&state);
        // Delete from the middle.
        state.caret = 4;
        state.delete();
        assert_eq!(state.text, "héll 😀 world");
        assert_renders(&state);
        // Paste with multi-byte content.
        state.insert_str("é🙂");
        assert_renders(&state);
        // Walk to the end, then backspace everything.
        while state.caret < state.text.chars().count() {
            state.move_right();
            assert_renders(&state);
        }
        while state.caret > 0 {
            state.backspace();
            assert_renders(&state);
        }
        assert!(state.text.is_empty());
        // Walk left/right on an empty string stays safe.
        state.move_left();
        state.move_right();
        state.backspace();
        assert_renders(&state);
    }

    #[test]
    fn caret_clamps_to_char_count() {
        let mut state = FieldState {
            text: "abc🙂".into(),
            caret: 99,
            ..Default::default()
        };
        state.insert('z');
        assert_eq!(state.text, "abc🙂z");
        assert_renders(&state);
        state.caret = 99;
        state.backspace();
        assert_eq!(state.text, "abc🙂");
        assert_renders(&state);
    }

    #[test]
    fn utf16_offsets_preserve_surrogate_pairs() {
        let text = "a🙂b";
        assert_eq!(FieldState::char_to_utf16(text, 0), 0);
        assert_eq!(FieldState::char_to_utf16(text, 1), 1);
        assert_eq!(FieldState::char_to_utf16(text, 2), 3);
        assert_eq!(FieldState::char_to_utf16(text, 3), 4);
        assert_eq!(FieldState::utf16_to_char(text, 1), 1);
        assert_eq!(FieldState::utf16_to_char(text, 2), 1);
        assert_eq!(FieldState::utf16_to_char(text, 3), 2);
    }
}
