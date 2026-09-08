//! Continuous, draft-only color selection. Persistence belongs to the settings editor.
use crate::theme::{parse_hex_color, THEME_PRESETS};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Surface {
    Square,
    Hue,
}

#[derive(Clone, Copy, Debug)]
struct Hsv {
    h: f32,
    s: f32,
    v: f32,
}
impl Hsv {
    fn from_color(color: Hsla) -> Self {
        let v = color.l + color.s * color.l.min(1.0 - color.l);
        Self {
            h: color.h,
            s: if v == 0.0 {
                0.0
            } else {
                2.0 * (1.0 - color.l / v)
            },
            v,
        }
    }
    fn color(self) -> Hsla {
        let l = self.v * (1.0 - self.s / 2.0);
        Hsla {
            h: self.h,
            s: if l == 0.0 || l == 1.0 {
                0.0
            } else {
                (self.v - l) / l.min(1.0 - l)
            },
            l,
            a: 1.0,
        }
    }
    fn select(&mut self, surface: Surface, bounds: Bounds<Pixels>, position: Point<Pixels>) {
        let x = ((position.x - bounds.left()) / bounds.size.width).clamp(0.0, 1.0);
        let y = ((position.y - bounds.top()) / bounds.size.height).clamp(0.0, 1.0);
        match surface {
            Surface::Square => {
                self.s = x;
                self.v = 1.0 - y;
            }
            Surface::Hue => self.h = x,
        }
    }
}

#[derive(Debug)]
pub struct ColorPicker {
    hsv: Hsv,
    original: Hsla,
    dragging: Option<Surface>,
    focus: FocusHandle,
    focus_pending: bool,
}
impl ColorPicker {
    pub fn new(color: Hsla, cx: &mut Context<Self>) -> Self {
        Self {
            hsv: Hsv::from_color(color),
            original: color,
            dragging: None,
            focus: cx.focus_handle(),
            focus_pending: true,
        }
    }
    pub fn color(&self) -> Hsla {
        self.hsv.color()
    }
    pub fn set_color(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.hsv = Hsv::from_color(color);
        cx.notify();
    }
    fn surface(&self, surface: Surface, cx: &mut Context<Self>) -> impl IntoElement {
        let hsv = self.hsv;
        let entity = cx.entity().downgrade();
        div()
            .id(if surface == Surface::Square {
                "picker-square"
            } else {
                "picker-hue"
            })
            .debug_selector(move || {
                if surface == Surface::Square {
                    "picker-square"
                } else {
                    "picker-hue"
                }
                .to_string()
            })
            .tab_index(0)
            .cursor_crosshair()
            .border_1()
            .border_color(crate::widgets::current_theme().border_strong)
            .focus(|style| style.border_color(crate::widgets::current_theme().accent))
            .on_key_down(cx.listener(move |picker, event: &KeyDownEvent, _, cx| {
                let direction = match event.keystroke.key.as_str() {
                    "left" | "down" => -0.01,
                    "right" | "up" => 0.01,
                    _ => return,
                };
                match surface {
                    Surface::Hue => picker.hsv.h = (picker.hsv.h + direction).rem_euclid(1.0),
                    Surface::Square => match event.keystroke.key.as_str() {
                        "left" | "right" => {
                            picker.hsv.s = (picker.hsv.s + direction).clamp(0.0, 1.0)
                        }
                        _ => picker.hsv.v = (picker.hsv.v + direction).clamp(0.0, 1.0),
                    },
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                canvas(
                    |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                    move |bounds, hitbox, window, _| {
                        let gradient = |angle, a: Hsla, b: Hsla| {
                            linear_gradient(
                                angle,
                                linear_color_stop(a, 0.0),
                                linear_color_stop(b, 1.0),
                            )
                        };
                        match surface {
                            Surface::Square => {
                                window.paint_quad(fill(
                                    bounds,
                                    gradient(
                                        90.0,
                                        white(),
                                        Hsla {
                                            h: hsv.h,
                                            s: 1.0,
                                            l: 0.5,
                                            a: 1.0,
                                        },
                                    ),
                                ));
                                window.paint_quad(fill(
                                    bounds,
                                    gradient(180.0, Hsla { a: 0.0, ..black() }, black()),
                                ));
                            }
                            Surface::Hue => {
                                for segment in 0..6 {
                                    let a = Hsla {
                                        h: segment as f32 / 6.0,
                                        s: 1.0,
                                        l: 0.5,
                                        a: 1.0,
                                    };
                                    let b = Hsla {
                                        h: (segment + 1) as f32 / 6.0,
                                        ..a
                                    };
                                    let left =
                                        bounds.left() + bounds.size.width * (segment as f32 / 6.0);
                                    let right = bounds.left()
                                        + bounds.size.width * ((segment + 1) as f32 / 6.0);
                                    window.paint_quad(fill(
                                        Bounds::from_corners(
                                            point(left, bounds.top()),
                                            point(right, bounds.bottom()),
                                        ),
                                        gradient(90.0, a, b),
                                    ));
                                }
                            }
                        }
                        // Paint the indicator without a hitbox: the dot can never block dragging.
                        let x = if surface == Surface::Square {
                            hsv.s
                        } else {
                            hsv.h
                        };
                        let y = if surface == Surface::Square {
                            1.0 - hsv.v
                        } else {
                            0.5
                        };
                        let center = point(
                            bounds.left()
                                + (bounds.size.width * x)
                                    .clamp(px(6.0), bounds.size.width - px(6.0)),
                            bounds.top()
                                + (bounds.size.height * y)
                                    .clamp(px(6.0), bounds.size.height - px(6.0)),
                        );
                        let marker =
                            Bounds::new(center - point(px(6.0), px(6.0)), size(px(12.0), px(12.0)));
                        window.paint_quad(quad(
                            marker,
                            px(6.0),
                            Hsla { a: 0.0, ..black() },
                            px(2.0),
                            white(),
                            BorderStyle::Solid,
                        ));
                        let down = entity.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase.bubble()
                                && event.button == MouseButton::Left
                                && hitbox.is_hovered(window)
                            {
                                let _ = down.update(cx, |picker, cx| {
                                    picker.dragging = Some(surface);
                                    picker.hsv.select(surface, bounds, event.position);
                                    cx.notify();
                                });
                            }
                        });
                        let moved = entity.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase.capture() {
                                let _ = moved.update(cx, |picker, cx| {
                                    if picker.dragging == Some(surface) {
                                        if event.pressed_button == Some(MouseButton::Left) {
                                            picker.hsv.select(surface, bounds, event.position);
                                            cx.notify();
                                        } else {
                                            picker.dragging = None;
                                        }
                                    }
                                });
                            }
                        });
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase.capture() && event.button == MouseButton::Left {
                                let _ = entity.update(cx, |picker, cx| {
                                    if picker.dragging == Some(surface) {
                                        picker.hsv.select(surface, bounds, event.position);
                                        picker.dragging = None;
                                        cx.notify();
                                    }
                                });
                            }
                        });
                    },
                )
                .w_full()
                .h(px(if surface == Surface::Square {
                    176.0
                } else {
                    22.0
                })),
            )
    }
}
impl Render for ColorPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_pending {
            window.focus(&self.focus);
            self.focus_pending = false;
        }
        let theme = crate::widgets::current_theme();
        let mut presets = div().flex().flex_wrap().gap(px(6.0));
        for hex in THEME_PRESETS {
            let (r, g, b) = parse_hex_color(hex).unwrap();
            let color = Hsla::from(Rgba { r, g, b, a: 1.0 });
            presets = presets.child(
                div()
                    .id(SharedString::from(format!("picker-{hex}")))
                    .tab_index(0)
                    .w(px(22.0))
                    .h(px(22.0))
                    .rounded(px(4.0))
                    .bg(color)
                    .cursor_pointer()
                    .border_1()
                    .border_color(theme.border_strong)
                    .focus(|style| style.border_2().border_color(white()))
                    .on_click(cx.listener(move |picker, _, _, cx| picker.set_color(color, cx)))
                    .on_action(cx.listener(move |picker, _: &crate::Activate, _, cx| {
                        picker.set_color(color, cx)
                    })),
            );
        }
        div()
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(self.surface(Surface::Square, cx))
            .child(self.surface(Surface::Hue, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().w(px(24.0)).h(px(24.0)).bg(self.original))
                    .child(div().w(px(24.0)).h(px(24.0)).bg(self.color()))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.text)
                            .child("Original / New"),
                    ),
            )
            .child(presets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::hsla_to_hex;
    use core::prelude::v1::test;

    #[test]
    fn hsv_roundtrips_and_keeps_hue_through_black() {
        for hex in THEME_PRESETS {
            let (r, g, b) = parse_hex_color(hex).unwrap();
            let original = Hsla::from(Rgba { r, g, b, a: 1.0 });
            assert_eq!(hsla_to_hex(Hsv::from_color(original).color()), *hex);
        }
        let mut hsv = Hsv {
            h: 0.62,
            s: 0.7,
            v: 0.0,
        };
        assert_eq!(hsla_to_hex(hsv.color()), "#000000");
        hsv.v = 0.8;
        assert_eq!(hsv.color().h, 0.62);
    }

    #[gpui::test]
    fn drag_is_continuous_clamped_and_stops_on_release(cx: &mut TestAppContext) {
        let (picker, cx) = cx.add_window_view(|_, cx| ColorPicker::new(red(), cx));
        cx.run_until_parked();
        let square = cx.debug_bounds("picker-square").unwrap();
        let start = square.origin + point(px(50.0), px(50.0));
        cx.simulate_mouse_move(start, None, Modifiers::none());
        picker.read_with(cx, |picker, _| {
            assert_eq!(hsla_to_hex(picker.color()), "#FF0000")
        });
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
        let before = picker.read_with(cx, |picker, _| picker.hsv.s);
        cx.simulate_mouse_move(
            start + point(px(1.0), px(0.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        picker.read_with(cx, |picker, _| assert!(picker.hsv.s > before));
        let outside = square.bottom_right() + point(px(40.0), px(40.0));
        cx.simulate_mouse_move(outside, Some(MouseButton::Left), Modifiers::none());
        picker.read_with(cx, |picker, _| {
            assert_eq!(picker.hsv.s, 1.0);
            assert_eq!(picker.hsv.v, 0.0);
        });
        cx.simulate_mouse_up(outside, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(start, None, Modifiers::none());
        picker.read_with(cx, |picker, _| assert_eq!(picker.hsv.v, 0.0));
        let hue = cx.debug_bounds("picker-hue").unwrap();
        let start = hue.origin + point(px(5.0), px(8.0));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(
            start + point(px(70.0), px(0.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        let selected = picker.read_with(cx, |picker, _| picker.hsv.h);
        assert!(selected > 0.0);
        cx.simulate_mouse_up(
            start + point(px(70.0), px(0.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        cx.simulate_mouse_move(start, None, Modifiers::none());
        picker.read_with(cx, |picker, _| assert_eq!(picker.hsv.h, selected));
        cx.simulate_keystrokes("right");
        picker.read_with(cx, |picker, _| assert!(picker.hsv.h > selected));
    }
}
