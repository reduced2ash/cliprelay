//! App-wide sizing policy. Dimensions elsewhere are unzoomed layout pixels.
//! Display DPI and user zoom compose in GPUI, never in individual controls.
pub const SCALES: [f32; 6] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5];
pub const SCALE_LABELS: [&str; 6] = [
    "80% · Smaller",
    "90% · Compact",
    "100% · Default",
    "110% · Comfortable",
    "125% · Larger",
    "150% · Largest",
];

pub fn normalize_scale(value: f32) -> f32 {
    if !value.is_finite() {
        return 1.0;
    }
    SCALES
        .into_iter()
        .min_by(|a, b| (value - a).abs().total_cmp(&(value - b).abs()))
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scale_settings_preserve_old_values_and_recover_invalid_values() {
        for scale in SCALES {
            assert_eq!(normalize_scale(scale), scale);
        }
        assert_eq!(normalize_scale(f32::NAN), 1.0);
        assert_eq!(normalize_scale(f32::INFINITY), 1.0);
        assert_eq!(normalize_scale(-2.0), 0.8);
        assert_eq!(normalize_scale(20.0), 1.5);
        assert_eq!(normalize_scale(1.24), 1.25);
    }
}

/// Enough space for navigation, a useful grid, and the complete inspector.
pub const DOCK_MIN_WIDTH: f32 = 1000.0;

impl crate::App {
    pub fn prepare_is_focused(&self) -> bool {
        self.page == crate::state::Page::Library
            && (self.prepare.studio_mode
                || (self.selected.is_some()
                    && self.window_size.0 < DOCK_MIN_WIDTH
                    && !self.compact_library_visible))
    }
}

#[cfg(test)]
mod window_tests {
    use gpui::TestAppContext;

    #[gpui::test]
    fn zoom_composes_with_display_scale_without_resizing_native_window(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let native = window.bounds();
            let viewport = window.viewport_size();
            let dpi = window.scale_factor();
            for zoom in super::SCALES {
                window.set_ui_scale(zoom, cx);
                assert_eq!(window.bounds(), native);
                assert!(
                    (f32::from(window.viewport_size().width) * zoom - f32::from(viewport.width))
                        .abs()
                        < 0.01
                );
                assert!(
                    (f32::from(window.viewport_size().height) * zoom - f32::from(viewport.height))
                        .abs()
                        < 0.01
                );
                assert_eq!(window.scale_factor(), dpi * zoom);
            }
            window.set_ui_scale(1.0, cx);
            assert_eq!(window.viewport_size(), viewport);
        });
    }
}

#[cfg(test)]
mod ui_scale_tests {
    use crate::{point, px};
    use gpui::{FileDropEvent, MouseDownEvent, PlatformInput, ScrollDelta, ScrollWheelEvent};

    #[test]
    fn platform_zoom_converts_pointer_drop_and_pixel_scroll_but_keeps_lines() {
        let position = point(px(150.0), px(90.0));
        let expected = point(px(100.0), px(60.0));
        let event = PlatformInput::MouseDown(MouseDownEvent {
            position,
            ..Default::default()
        })
        .into_ui_coordinates(1.5);
        assert!(matches!(event, PlatformInput::MouseDown(e) if e.position == expected));
        for event in [
            FileDropEvent::Pending { position },
            FileDropEvent::Submit { position },
        ] {
            let event = PlatformInput::FileDrop(event).into_ui_coordinates(1.5);
            assert!(
                matches!(event, PlatformInput::FileDrop(FileDropEvent::Pending { position } | FileDropEvent::Submit { position }) if position == expected)
            );
        }
        for delta in [
            ScrollDelta::Pixels(position),
            ScrollDelta::Lines(point(2.0, -3.0)),
        ] {
            let event = PlatformInput::ScrollWheel(ScrollWheelEvent {
                position,
                delta,
                ..Default::default()
            })
            .into_ui_coordinates(1.5);
            let PlatformInput::ScrollWheel(event) = event else {
                panic!("scroll event changed kind")
            };
            assert_eq!(event.position, expected);
            match event.delta {
                ScrollDelta::Pixels(delta) => assert_eq!(delta, expected),
                ScrollDelta::Lines(delta) => assert_eq!(delta, point(2.0, -3.0)),
            }
        }
    }
}
