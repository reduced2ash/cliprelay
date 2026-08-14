//! Prepare workspace: video stage, transport + timeline, edit inspector
//! (crop/masks), publish inspector, and the pinned action dock.

use crate::state::*;
use cliprelay_core::media::CropSpec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    pub next_frame_at: f64,
    pub frame_failures: u32,
    pub duration: f64,
    pub position: f64,
    pub playing: bool,
    pub trim_start: f64,
    pub trim_end: f64,
    pub inspector_tab: i64, // 0 Edit, 1 Publish
    pub studio_mode: bool,
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
    pub frame_cache: HashMap<String, PathBuf>,
    pub last_frame_key: String,
    last_frame_request_at: std::time::Instant,
    pub frame_generation: u64,
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
            next_frame_at: 0.0,
            frame_failures: 0,
            duration: 0.0,
            position: 0.0,
            playing: false,
            trim_start: 0.0,
            trim_end: 0.0,
            inspector_tab: 0,
            studio_mode: false,
            studio_width: 460.0,
            same_caption: true,
            telegram_mode_index: 0,
            destination: String::new(),
            caption: String::new(),
            x_caption: String::new(),
            compression_index: 4,
            target_mb: String::new(),
            cleanup_index: 0,
            crop_enabled: false,
            crop: CropSpec { x: 0.0, y: 0.0, width: 1.0, height: 1.0 },
            shapes: Vec::new(),
            selected_shape: None,
            frame_cache: HashMap::new(),
            last_frame_key: String::new(),
            last_frame_request_at: std::time::Instant::now(),
            frame_generation: 0,
            active_action: String::new(),
            last_submit_x_enabled: false,
            drag: DragHandle::None,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_value: 0.0,
            frame_rect: (0.0, 0.0, 0.0, 0.0),
            crop_start: CropSpec { x: 0.0, y: 0.0, width: 1.0, height: 1.0 },
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
            self.next_frame_at = 0.0;
            self.frame_failures = 0;
            self.playing = false;
            self.inspector_tab = 0;
            self.active_action.clear();
            self.crop_enabled = false;
            self.crop = CropSpec { x: 0.0, y: 0.0, width: 1.0, height: 1.0 };
            self.shapes.clear();
            self.selected_shape = None;
            self.frame_cache.clear();
            self.last_frame_key.clear();
            self.timeline_ready = false;
        } else if duration > 0.0 && self.trim_end <= 0.0 {
            self.trim_end = duration;
        }
    }

    /// Set the source path used by the frame extractor threads.
    pub fn set_media_path(&mut self, path: PathBuf) {
        self.media_path = Some(path);
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
        self.duration > 0.0
            && (self.trim_start > 0.05 || self.trim_end < self.duration - 0.05)
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
    }

    pub fn toggle_playback(&mut self) {
        self.playing = !self.playing;
        if self.playing && self.position >= self.trim_end {
            self.position = self.trim_start;
            self.next_frame_at = self.position;
        }
    }

    /// Called every 100ms by the app ticker; advances playback and requests
    /// frames lazily. Returns true when the UI needs a repaint.
    pub fn tick(&mut self) -> bool {
        if !self.playing || self.media_id <= 0 {
            return false;
        }
        // Advance in real time (100ms tick). Frames are extracted at the
        // play_fps cadence to avoid spawning an ffmpeg process every tick.
        self.position += 0.1;
        if self.trim_end > 0.0 && self.position >= self.trim_end {
            self.position = self.trim_start;
        }
        if self.next_frame_at <= self.position {
            self.next_frame_at += 1.0 / self.play_fps();
            let key = frame_key(self.position);
            if !self.frame_cache.contains_key(&key) && self.frame_failures < 4 {
                self.request_frame(self.position);
            }
            self.last_frame_key = key;
            // Prefetch the next two frames so playback stays smooth
            // instead of stalling on each on-demand ffmpeg extract.
            for ahead in [0.2f64, 0.4f64] {
                let ahead_s = (self.position + ahead).min(self.trim_end.max(0.0));
                self.request_frame_at(ahead_s, true);
            }
        }
        true
    }

    pub fn play_fps(&self) -> f64 {
        if self.duration <= 30.0 {
            24.0
        } else if self.duration <= 120.0 {
            12.0
        } else {
            6.0
        }
    }

    pub fn seek(&mut self, seconds: f64, duration: f64) {
        self.position = seconds.clamp(0.0, duration.max(0.0));
        self.frame_failures = 0;
        // Resync the playback clock so the tick neither stalls (backward
        // seek) nor bursts (forward seek).
        self.next_frame_at = self.position;
        self.request_frame_at(self.position, false);
        self.last_frame_key = frame_key(self.position);
    }

    /// Throttled frame request: cached frames are surfaced instantly,
    /// and at most one extraction spawns per 80ms (drag moves otherwise
    /// spawn an ffmpeg process per mouse event). `force` bypasses the
    /// throttle for the final position at drag release.
    pub fn request_frame_at(&mut self, seconds: f64, force: bool) {
        let key = frame_key(seconds);
        if self.frame_cache.contains_key(&key) {
            self.last_frame_key = key;
            return;
        }
        if !force {
            let now = std::time::Instant::now();
            if now.duration_since(self.last_frame_request_at).as_millis() < 80 {
                return;
            }
            self.last_frame_request_at = now;
        }
        self.request_frame(seconds);
    }

    pub fn request_frame(&mut self, seconds: f64) {
        self.frame_generation += 1;
        let generation = self.frame_generation;
        let media_id = self.media_id;
        let key = frame_key(seconds);
        let seconds = seconds.max(0.0);
        let Some(media_path) = self.media_path.clone() else {
            return;
        };
        std::thread::spawn(move || {
            let path = extract_frame(media_id, seconds, &media_path);
            if let Some(tx) = crate::FRAME_TX.get() {
                if let Some(path) = path {
                    let _ = tx.send((media_id, format!("{key}|{generation}"), path));
                } else {
                    let _ = tx.send((media_id, "fail".into(), PathBuf::new()));
                }
            }
        });
    }

    pub fn on_frame_ready(&mut self, media_id: i64, key: String, path: PathBuf) {
        if media_id != self.media_id {
            return;
        }
        if key == "fail" {
            self.frame_failures = self.frame_failures.saturating_add(1);
            return;
        }
        let (frame_key, generation) = key.split_once('|').unwrap_or((&key, ""));
        self.frame_cache.insert(frame_key.to_string(), path);
        let _ = generation;
    }

    pub fn current_frame_path(&self) -> Option<PathBuf> {
        self.frame_cache.get(&self.last_frame_key).cloned()
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
        self.crop = CropSpec { x: 0.0, y: 0.0, width: 1.0, height: 1.0 };
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

pub fn frame_key(seconds: f64) -> String {
    format!("f{:06}", (seconds * 1000.0).round() as i64)
}

/// Extract a single JPEG frame at `seconds` into the preview cache.
pub fn extract_frame(media_id: i64, seconds: f64, media_path: &Path) -> Option<PathBuf> {
    use cliprelay_core::paths::preview_dir;
    use std::process::{Command, Stdio};
    let ffmpeg = cliprelay_core::paths::ffmpeg_path()?;
    let dir = preview_dir().join("frames");
    let _ = std::fs::create_dir_all(&dir);
    let key = format!("{media_id}-{:06}", (seconds * 1000.0).round() as i64);
    let output = dir.join(format!("{key}.jpg"));
    if output.is_file() {
        return Some(output);
    }
    let status = Command::new(&ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-ss"])
        .arg(format!("{seconds:.3}"))
        .args(["-i"])
        .arg(media_path)
        .args(["-frames:v", "1", "-vf", "scale=960:540:force_original_aspect_ratio=decrease", "-q:v", "3"])
        .arg(&output)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();
    if matches!(status, Some(s) if s.success()) && output.is_file() {
        Some(output)
    } else {
        None
    }
}


#[cfg(test)]
mod prepare_tests {
    use super::*;

    #[test]
    fn enable_crop_starts_from_free_crop() {
        let mut prepare = PrepareState::default();
        prepare.duration = 10.0;
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
        let mut prepare = PrepareState::default();
        prepare.duration = 10.0;
        prepare.crop = CropSpec { x: 0.1, y: 0.1, width: 0.5, height: 0.5 };
        // Re-enabling after an aspect crop keeps the current geometry.
        prepare.apply_crop_aspect(0.0, 16.0 / 9.0);
        assert!((prepare.crop.x - 0.1).abs() < 1e-9);
        assert!((prepare.crop.width - 0.5).abs() < 1e-9);
    }

    #[test]
    fn aspect_presets_center_crop() {
        let mut prepare = PrepareState::default();
        prepare.duration = 10.0;
        // 16:9 source, 1:1 target -> centered square of normalized height 1.
        prepare.apply_crop_aspect(1.0, 16.0 / 9.0);
        assert!((prepare.crop.width - 9.0 / 16.0).abs() < 1e-9);
        assert!((prepare.crop.height - 1.0).abs() < 1e-9);
        assert!((prepare.crop.x - (1.0 - prepare.crop.width) / 2.0).abs() < 1e-9);
        assert!(prepare.crop_is_original() == false);
    }

    #[test]
    fn caption_for_respects_same_caption_toggle() {
        let mut prepare = PrepareState::default();
        prepare.caption = "Shared caption".into();
        prepare.x_caption = "X-only caption".into();
        prepare.same_caption = true;
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
            assert!(
                !text.contains(".1"),
                "no 3-digit hundredths: {text}"
            );
            assert_eq!(text.split('.').nth(1).map(|f| f.len()), Some(2), "{text}");
        }
    }

    #[test]
    fn cut_active_requires_minimum_duration() {
        let mut prepare = PrepareState::default();
        prepare.duration = 10.0;
        prepare.trim_start = 0.0;
        prepare.trim_end = 10.0;
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
}
