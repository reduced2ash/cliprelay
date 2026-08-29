//! Prepare workspace: video stage, transport + timeline, edit inspector
//! (crop/masks), publish inspector, and the pinned action dock.

use crate::state::*;
use cliprelay_core::media::CropSpec;
use gpui_video_player::Video;
use std::path::PathBuf;
use std::time::Duration;

const MIN_TRIM_DURATION: f64 = 0.05;

pub const COMPRESSION_OPTIONS: [(&str, &str); 7] = [
    ("Original when possible", "original"),
    ("Balanced", "balanced"),
    ("Fit Telegram bot", "fit_bot"),
    ("Fit X", "fit_x"),
    ("Fit both", "fit_both"),
    ("Smallest practical", "smallest"),
    ("Custom size", "custom"),
];

pub const CLEANUP_OPTIONS: [(&str, &str); 3] = [
    ("Keep", "keep"),
    ("Trash when complete", "after_complete"),
    ("Trash after Telegram", "after_telegram"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DragHandle {
    None,
    StudioSplit,
    Seek,
    TrimIn,
    TrimOut,
    CropMove,
    CropResize(usize),
    MaskMove(usize),
    MaskResize(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Rectangle,
    Square,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Shape {
    pub kind: ShapeKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct PrepareState {
    pub media_id: i64,
    pub media_path: Option<PathBuf>,
    pub video: Option<Video>,
    pub duration: f64,
    pub position: f64,
    pub playing: bool,
    pub trim_start: f64,
    pub trim_end: f64,
    pub inspector_tab: i64, // 0 Edit, 1 Publish
    pub studio_mode: bool,
    pub compact_inspector_open: bool,
    pub studio_width: f64,
    pub same_caption: bool,
    pub telegram_mode_index: i64,
    pub destination: String,
    pub caption: String,
    pub x_caption: String,
    pub compression_index: i64,
    pub target_mb: String,
    pub cleanup_index: i64,
    pub crop_enabled: bool,
    pub crop: CropSpec,
    pub shapes: Vec<Shape>,
    pub selected_shape: Option<usize>,
    pub active_action: String,
    pub last_submit_x_enabled: bool,
    pub drag: DragHandle,
    pub drag_start_x: f64,
    pub drag_start_y: f64,
    pub drag_start_value: f64,
    pub frame_rect: (f32, f32, f32, f32),
    pub crop_start: CropSpec,
    pub shape_start: (f64, f64, f64, f64),
    pub dragging_shape: Option<usize>,
    pub editing_scroll: bool,
    pub edit_scroll_y: f64,
    pub publish_scroll_y: f64,
    pub timeline_ready: bool,
    pub position_field_in: String,
    pub position_field_out: String,
}

impl From<Shape> for (f64, f64, f64, f64) {
    fn from(shape: Shape) -> Self {
        (shape.x, shape.y, shape.width, shape.height)
    }
}

impl Default for PrepareState {
    fn default() -> Self {
        Self {
            media_id: 0,
            media_path: None,
            video: None,
            duration: 0.0,
            position: 0.0,
            playing: false,
            trim_start: 0.0,
            trim_end: 0.0,
            inspector_tab: 0,
            studio_mode: false,
            compact_inspector_open: false,
            studio_width: 420.0,
            same_caption: true,
            telegram_mode_index: 0,
            destination: String::new(),
            caption: String::new(),
            x_caption: String::new(),
            compression_index: 4,
            target_mb: String::new(),
            cleanup_index: 0,
            crop_enabled: false,
            crop: CropSpec {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            shapes: Vec::new(),
            selected_shape: None,
            active_action: String::new(),
            last_submit_x_enabled: false,
            drag: DragHandle::None,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_value: 0.0,
            frame_rect: (0.0, 0.0, 0.0, 0.0),
            crop_start: CropSpec {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            shape_start: (0.0, 0.0, 0.0, 0.0),
            dragging_shape: None,
            editing_scroll: false,
            edit_scroll_y: 0.0,
            publish_scroll_y: 0.0,
            timeline_ready: false,
            position_field_in: String::new(),
            position_field_out: String::new(),
        }
    }
}

impl PrepareState {
    pub fn on_media_changed(&mut self, media_id: i64, duration: f64) {
        if media_id != self.media_id {
            self.media_id = media_id;
            self.duration = duration;
            self.trim_start = 0.0;
            self.trim_end = duration;
            self.position = 0.0;
            self.playing = false;
            self.inspector_tab = 0;
            self.compact_inspector_open = false;
            self.active_action.clear();
            self.crop_enabled = false;
            self.crop = CropSpec {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            };
            self.shapes.clear();
            self.selected_shape = None;
            self.timeline_ready = false;
            self.video = None;
        } else if duration > 0.0 {
            self.duration = duration;
            if self.trim_end <= 0.0 {
                self.trim_end = duration;
            }
        }
    }

    /// Set the source path. GStreamer startup is orchestrated by the owning
    /// GPUI view so it never blocks this model or the application thread.
    pub fn set_media_path(&mut self, path: PathBuf) {
        self.media_path = Some(path);
        self.video = None;
    }

    pub fn on_publish_state(&mut self, publish: &PublishState) {
        if !publish.active {
            self.active_action.clear();
        }
    }

    pub fn has_edits(&self) -> bool {
        self.crop_enabled || !self.shapes.is_empty()
    }

    pub fn cut_active(&self) -> bool {
        self.duration > 0.0 && (self.trim_start > 0.05 || self.trim_end < self.duration - 0.05)
    }

    pub fn edit_spec(&self) -> serde_json::Value {
        let crop = if self.crop_enabled {
            Some(serde_json::json!({
                "enabled": true,
                "x": self.crop.x,
                "y": self.crop.y,
                "width": self.crop.width,
                "height": self.crop.height,
            }))
        } else {
            None
        };
        let overlays: Vec<serde_json::Value> = self
            .shapes
            .iter()
            .map(|shape| {
                serde_json::json!({
                    "type": "rectangle",
                    "x": shape.x,
                    "y": shape.y,
                    "width": shape.width,
                    "height": shape.height,
                })
            })
            .collect();
        serde_json::json!({ "crop": crop, "overlays": overlays })
    }

    pub fn load_edit_spec(&mut self, value: &serde_json::Value) {
        self.crop_enabled = false;
        self.shapes.clear();
        self.selected_shape = None;
        let spec = cliprelay_core::media::normalize_edit_spec(value);
        if let Some(crop) = spec.crop {
            self.crop_enabled = true;
            self.crop = crop;
        }
        for overlay in spec.overlays {
            self.shapes.push(Shape {
                kind: ShapeKind::Rectangle,
                x: overlay.x,
                y: overlay.y,
                width: overlay.width,
                height: overlay.height,
            });
        }
    }

    pub fn reset_cut(&mut self) {
        self.trim_start = 0.0;
        self.trim_end = self.duration;
        self.position = 0.0;
        if let Some(video) = &self.video {
            let _ = video.seek(Duration::from_secs_f64(0.0), true);
        }
    }

    /// Mark the current playhead as the start of the retained range.
    /// If it crosses the existing Out mark, reopen Out to the source end so
    /// the user's new mark is accepted instead of being silently clamped away.
    pub fn mark_in_at_playhead(&mut self) -> bool {
        if self.duration <= MIN_TRIM_DURATION {
            return false;
        }
        let previous = (self.trim_start, self.trim_end);
        let mark = self.position.clamp(0.0, self.duration - MIN_TRIM_DURATION);
        let current_end = if self.trim_end > 0.0 {
            self.trim_end.min(self.duration)
        } else {
            self.duration
        };
        self.trim_end = if mark >= current_end - MIN_TRIM_DURATION {
            self.duration
        } else {
            current_end
        };
        self.trim_start = mark.min((self.trim_end - MIN_TRIM_DURATION).max(0.0));
        previous != (self.trim_start, self.trim_end)
    }

    /// Mark the current playhead as the end of the retained range. Crossing
    /// the existing In mark symmetrically reopens In to the source beginning.
    pub fn mark_out_at_playhead(&mut self) -> bool {
        if self.duration <= MIN_TRIM_DURATION {
            return false;
        }
        let previous = (self.trim_start, self.trim_end);
        let mark = self.position.clamp(MIN_TRIM_DURATION, self.duration);
        if mark <= self.trim_start + MIN_TRIM_DURATION {
            self.trim_start = 0.0;
        }
        self.trim_end = mark
            .max(self.trim_start + MIN_TRIM_DURATION)
            .min(self.duration);
        previous != (self.trim_start, self.trim_end)
    }

    pub fn toggle_playback(&mut self) {
        if self.video.is_none() {
            self.playing = false;
            return;
        }
        self.playing = !self.playing;
        if let Some(video) = &self.video {
            if self.playing && self.position >= self.trim_end && self.trim_end > 0.0 {
                self.position = self.trim_start;
                let _ = video.seek(Duration::from_secs_f64(self.position), true);
            }
            video.set_paused(!self.playing);
            if self.playing {
                let _ = video.seek(Duration::from_secs_f64(self.position), true);
            }
        }
    }

    /// Called every 100ms by the app ticker. Returns true when the UI needs a
    /// repaint. Playback is owned exclusively by the GPUI/GStreamer player.
    pub fn tick(&mut self) -> bool {
        if !self.playing || self.media_id <= 0 {
            return false;
        }
        if let Some(video) = &self.video {
            if video.paused() == self.playing {
                video.set_paused(!self.playing);
            }
            let pos = video.position().as_secs_f64();
            if pos > 0.0 || self.position == 0.0 {
                self.position = pos;
            }
            if video.eos() || (self.trim_end > 0.0 && self.position >= self.trim_end) {
                let _ = video.seek(Duration::from_secs_f64(self.trim_start), true);
                self.position = self.trim_start;
                video.set_paused(false);
            }
            return true;
        }
        self.playing = false;
        true
    }

    pub fn seek(&mut self, seconds: f64, duration: f64) {
        self.position = seconds.clamp(0.0, duration.max(0.0));
        if let Some(video) = &self.video {
            let _ = video.seek(Duration::from_secs_f64(self.position), true);
        }
    }

    pub fn format_time(&self, seconds: f64) -> String {
        let value = seconds.max(0.0);
        let total = value as u64;
        let hours = total / 3600;
        let minutes = (total % 3600) / 60;
        let secs = total % 60;
        if hours > 0 {
            format!("{hours}:{minutes:02}:{secs:02}")
        } else {
            format!("{minutes:02}:{secs:02}")
        }
    }

    pub fn format_time_precise(&self, seconds: f64) -> String {
        // Floor to centiseconds like the original's formatTime (the floor
        // also keeps the fraction from overflowing into a malformed
        // 3-digit value and stays parseable back to the input).
        let value = seconds.max(0.0);
        let total_centis = (value * 100.0).floor() as u64;
        let total = total_centis / 100;
        let hundredths = total_centis % 100;
        let hours = total / 3600;
        let minutes = (total % 3600) / 60;
        let secs = total % 60;
        let base = if hours > 0 {
            format!("{hours}:{minutes:02}:{secs:02}")
        } else {
            format!("{minutes:02}:{secs:02}")
        };
        format!("{base}.{hundredths:02}")
    }

    pub fn add_shape(&mut self, square: bool) {
        let shape = if square {
            Shape {
                kind: ShapeKind::Square,
                x: 0.38,
                y: 0.36,
                width: 0.24,
                height: 0.24,
            }
        } else {
            Shape {
                kind: ShapeKind::Rectangle,
                x: 0.36,
                y: 0.41,
                width: 0.28,
                height: 0.18,
            }
        };
        self.shapes.push(shape);
        self.selected_shape = Some(self.shapes.len() - 1);
    }

    pub fn remove_selected_shape(&mut self) {
        if let Some(index) = self.selected_shape {
            if index < self.shapes.len() {
                self.shapes.remove(index);
            }
        }
        self.selected_shape = None;
    }

    pub fn clear_shapes(&mut self) {
        self.shapes.clear();
        self.selected_shape = None;
    }

    pub fn reset_crop(&mut self) {
        self.crop_enabled = false;
        self.crop = CropSpec {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        };
    }

    pub fn apply_crop_aspect(&mut self, target_ratio: f64, source_ratio: f64) {
        self.crop_enabled = true;
        if target_ratio <= 0.0 {
            // Free crop: keep the current crop unless it is still full-frame,
            // in which case start from a 0.84 inset (mirrors the original).
            if self.crop_is_original() {
                self.crop = CropSpec {
                    x: 0.08,
                    y: 0.08,
                    width: 0.84,
                    height: 0.84,
                };
            }
            return;
        }
        let normalized = target_ratio / source_ratio.max(0.01);
        let (width, height) = if normalized <= 1.0 {
            (normalized, 1.0)
        } else {
            (1.0, 1.0 / normalized)
        };
        self.crop = CropSpec {
            x: (1.0 - width) / 2.0,
            y: (1.0 - height) / 2.0,
            width,
            height,
        };
    }

    pub fn crop_is_original(&self) -> bool {
        self.crop.x.abs() < 0.001
            && self.crop.y.abs() < 0.001
            && (self.crop.width - 1.0).abs() < 0.001
            && (self.crop.height - 1.0).abs() < 0.001
    }

    pub fn caption_for(&self, platform: &str) -> String {
        if self.same_caption {
            self.caption.clone()
        } else if platform == "x" {
            self.x_caption.clone()
        } else {
            self.caption.clone()
        }
    }
}

pub(crate) fn playback_control_availability(video_ready: bool, duration: f64) -> (bool, bool) {
    (video_ready, video_ready && duration > 0.0)
}

#[cfg(test)]
mod prepare_tests {
    use super::*;

    #[test]
    fn metadata_arrival_updates_the_existing_selection() {
        let mut prepare = PrepareState::default();
        prepare.on_media_changed(42, 0.0);
        assert_eq!(prepare.duration, 0.0);
        assert_eq!(prepare.trim_end, 0.0);

        prepare.on_media_changed(42, 12.5);
        assert_eq!(prepare.duration, 12.5);
        assert_eq!(prepare.trim_end, 12.5);
        assert_eq!(prepare.media_id, 42);
    }

    #[test]
    fn playback_can_start_before_probe_metadata_finishes() {
        assert_eq!(playback_control_availability(false, 0.0), (false, false));
        assert_eq!(playback_control_availability(true, 0.0), (true, false));
        assert_eq!(playback_control_availability(true, 8.0), (true, true));
        assert_eq!(playback_control_availability(false, 8.0), (false, false));
    }

    #[test]
    fn enable_crop_starts_from_free_crop() {
        let mut prepare = PrepareState {
            duration: 10.0,
            ..Default::default()
        };
        // Fresh enable: full-frame crop -> 0.84 free crop inset.
        prepare.apply_crop_aspect(0.0, 16.0 / 9.0);
        assert!(prepare.crop_enabled);
        assert!((prepare.crop.x - 0.08).abs() < 1e-9);
        assert!((prepare.crop.y - 0.08).abs() < 1e-9);
        assert!((prepare.crop.width - 0.84).abs() < 1e-9);
        assert!((prepare.crop.height - 0.84).abs() < 1e-9);
        assert!(!prepare.crop_is_original());
    }

    #[test]
    fn enable_crop_keeps_existing_crop() {
        let mut prepare = PrepareState {
            duration: 10.0,
            crop: CropSpec {
                x: 0.1,
                y: 0.1,
                width: 0.5,
                height: 0.5,
            },
            ..Default::default()
        };
        // Re-enabling after an aspect crop keeps the current geometry.
        prepare.apply_crop_aspect(0.0, 16.0 / 9.0);
        assert!((prepare.crop.x - 0.1).abs() < 1e-9);
        assert!((prepare.crop.width - 0.5).abs() < 1e-9);
    }

    #[test]
    fn aspect_presets_center_crop() {
        let mut prepare = PrepareState {
            duration: 10.0,
            ..Default::default()
        };
        // 16:9 source, 1:1 target -> centered square of normalized height 1.
        prepare.apply_crop_aspect(1.0, 16.0 / 9.0);
        assert!((prepare.crop.width - 9.0 / 16.0).abs() < 1e-9);
        assert!((prepare.crop.height - 1.0).abs() < 1e-9);
        assert!((prepare.crop.x - (1.0 - prepare.crop.width) / 2.0).abs() < 1e-9);
        assert!(!prepare.crop_is_original());
    }

    #[test]
    fn caption_for_respects_same_caption_toggle() {
        let mut prepare = PrepareState {
            caption: "Shared caption".into(),
            x_caption: "X-only caption".into(),
            same_caption: true,
            ..Default::default()
        };
        assert_eq!(prepare.caption_for("telegram"), "Shared caption");
        assert_eq!(prepare.caption_for("x"), "Shared caption");
        prepare.same_caption = false;
        assert_eq!(prepare.caption_for("telegram"), "Shared caption");
        assert_eq!(prepare.caption_for("x"), "X-only caption");
    }

    #[test]
    fn format_times_are_stable() {
        let prepare = PrepareState::default();
        assert_eq!(prepare.format_time(65.0), "01:05");
        assert_eq!(prepare.format_time(3661.0), "1:01:01");
        assert_eq!(prepare.format_time_precise(65.4), "01:05.40");
        assert_eq!(prepare.format_time_precise(0.0), "00:00.00");
    }

    #[test]
    fn precise_time_never_overflows_hundredths() {
        let prepare = PrepareState::default();
        // Floors to centiseconds like the original's formatTime: 65.999
        // renders as 01:05.99 (never a malformed 01:05.100).
        assert_eq!(prepare.format_time_precise(65.999), "01:05.99");
        assert_eq!(prepare.format_time_precise(65.995), "01:05.99");
        assert_eq!(prepare.format_time_precise(65.994), "01:05.99");
        assert_eq!(prepare.format_time_precise(0.999), "00:00.99");
        assert_eq!(prepare.format_time_precise(0.0), "00:00.00");
        // Every output must round-trip through a parse back to ~the input.
        for seconds in [0.004, 0.995, 59.999, 65.999, 3599.999, 3661.25] {
            let text = prepare.format_time_precise(seconds);
            assert!(!text.contains(".1"), "no 3-digit hundredths: {text}");
            assert_eq!(text.split('.').nth(1).map(|f| f.len()), Some(2), "{text}");
        }
    }

    #[test]
    fn cut_active_requires_minimum_duration() {
        let mut prepare = PrepareState {
            duration: 10.0,
            trim_start: 0.0,
            trim_end: 10.0,
            ..Default::default()
        };
        assert!(!prepare.cut_active());
        prepare.trim_start = 1.0;
        prepare.trim_end = 3.0;
        assert!(prepare.cut_active());
        // Trims shorter than the 0.05s minimum don't count as a cut.
        prepare.trim_start = 0.02;
        prepare.trim_end = 10.0;
        assert!(!prepare.cut_active());
        prepare.trim_start = 0.06;
        prepare.trim_end = 10.0;
        assert!(prepare.cut_active());
        // Full frame is not a cut.
        prepare.trim_start = 0.0;
        prepare.trim_end = 10.0;
        assert!(!prepare.cut_active());
    }

    #[test]
    fn playhead_marks_create_and_cross_trim_ranges_intuitively() {
        let mut prepare = PrepareState {
            duration: 10.0,
            trim_end: 10.0,
            position: 3.0,
            ..Default::default()
        };
        assert!(prepare.mark_in_at_playhead());
        assert_eq!((prepare.trim_start, prepare.trim_end), (3.0, 10.0));

        prepare.position = 8.0;
        assert!(prepare.mark_out_at_playhead());
        assert_eq!((prepare.trim_start, prepare.trim_end), (3.0, 8.0));

        // A new In beyond Out reopens the end of the source.
        prepare.position = 9.0;
        assert!(prepare.mark_in_at_playhead());
        assert_eq!((prepare.trim_start, prepare.trim_end), (9.0, 10.0));

        // A new Out before In symmetrically reopens the beginning.
        prepare.position = 2.0;
        assert!(prepare.mark_out_at_playhead());
        assert_eq!((prepare.trim_start, prepare.trim_end), (0.0, 2.0));
    }

    #[test]
    fn playhead_marks_preserve_a_valid_minimum_range_at_source_edges() {
        let mut prepare = PrepareState {
            duration: 10.0,
            trim_end: 10.0,
            position: 10.0,
            ..Default::default()
        };
        assert!(prepare.mark_in_at_playhead());
        assert_eq!((prepare.trim_start, prepare.trim_end), (9.95, 10.0));

        prepare.position = 0.0;
        assert!(prepare.mark_out_at_playhead());
        assert_eq!((prepare.trim_start, prepare.trim_end), (0.0, 0.05));
    }
}
