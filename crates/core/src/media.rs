//! Media pipeline: discovery, FFprobe metadata, thumbnails, previews,
//! timeline strips, and FFmpeg exports. Ported 1:1 from `media.py` —
//! every argv sequence below matches the Python original.

use crate::db::{Database, MediaMetadata};
use crate::paths::{ffmpeg_path, ffprobe_path, preview_dir, thumbnail_dir, timeline_dir};
use crate::utils::{clamp, media_cache_key, safe_stem};
use anyhow::Result;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use parking_lot::Mutex;
use std::time::{Duration, Instant};

pub const VIDEO_EXTENSIONS: &[&str] = &[
    ".3g2", ".3gp", ".asf", ".avi", ".divx", ".dv", ".f4v", ".flv", ".h264", ".hevc", ".m2t",
    ".m2ts", ".m4v", ".mkv", ".mov", ".mp4", ".mpeg", ".mpg", ".mts", ".mxf", ".ogm", ".ogv",
    ".qt", ".rm", ".rmvb", ".ts", ".vob", ".webm", ".wmv", ".y4m",
];

pub const TRANSPORT_STREAM_EXTENSIONS: &[&str] = &[".m2t", ".m2ts", ".mts", ".ts"];
const TRANSPORT_STREAM_PACKET_SIZES: [usize; 3] = [188, 192, 204];
const TRANSPORT_STREAM_SAMPLE_BYTES: usize = 8192;

pub const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    "__macosx",
    "$recycle.bin",
    "system volume information",
];

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("{0}")]
    Message(String),
    #[error("Video processing was cancelled.")]
    ProcessingCancelled,
    #[error("Library scan stopped.")]
    ScanCancelled,
}

#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub discovered: usize,
    pub indexed: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub duration: f64,
    pub generated: bool,
    pub preset: String,
    pub encoder: String,
    pub hardware_accelerated: bool,
}

impl ExportResult {
    pub fn label(&self) -> String {
        self.encoder.clone()
    }
}

pub type CancelFlag = Arc<AtomicBool>;

/// Normalized edit spec: optional crop + rectangle overlays.
#[derive(Debug, Clone, Default)]
pub struct EditSpec {
    pub crop: Option<CropSpec>,
    pub overlays: Vec<OverlaySpec>,
}

#[derive(Debug, Clone, Copy)]
pub struct CropSpec {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct OverlaySpec {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl EditSpec {
    pub fn is_empty(&self) -> bool {
        self.crop.is_none() && self.overlays.is_empty()
    }
}

fn unit_number(raw: &serde_json::Value, default: f64) -> f64 {
    match raw {
        serde_json::Value::Number(n) => n.as_f64().filter(|v| v.is_finite()).unwrap_or(default),
        serde_json::Value::String(s) => s.parse::<f64>().ok().filter(|v| v.is_finite()).unwrap_or(default),
        _ => default,
    }
}

/// Port of `normalize_edit_spec`.
pub fn normalize_edit_spec(value: &serde_json::Value) -> EditSpec {
    let mut spec = EditSpec::default();
    if let Some(crop) = value.get("crop").and_then(|c| c.as_object()) {
        let enabled = crop.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
        if enabled {
            let x = clamp(unit_number(&crop["x"], 0.0), 0.0, 0.99);
            let y = clamp(unit_number(&crop["y"], 0.0), 0.0, 0.99);
            let width = clamp(unit_number(&crop["width"], 1.0), 0.01, 1.0 - x);
            let height = clamp(unit_number(&crop["height"], 1.0), 0.01, 1.0 - y);
            if x > 0.0001 || y > 0.0001 || width < 0.9999 || height < 0.9999 {
                spec.crop = Some(CropSpec { x, y, width, height });
            }
        }
    }
    if let Some(overlays) = value.get("overlays").and_then(|o| o.as_array()) {
        for overlay in overlays.iter().take(32) {
            let Some(obj) = overlay.as_object() else { continue };
            let x = clamp(unit_number(&obj["x"], 0.0), 0.0, 0.99);
            let y = clamp(unit_number(&obj["y"], 0.0), 0.0, 0.99);
            let width = clamp(unit_number(&obj["width"], 0.2), 0.01, 1.0 - x);
            let height = clamp(unit_number(&obj["height"], 0.2), 0.01, 1.0 - y);
            spec.overlays.push(OverlaySpec { x, y, width, height });
        }
    }
    spec
}

/// Build the `-vf` filter string from an edit spec + target height.
pub fn build_filter(edits: &EditSpec, height: i64) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(crop) = edits.crop {
        parts.push(format!(
            "crop=w='max(2,trunc(iw*{:.8}/2)*2)':h='max(2,trunc(ih*{:.8}/2)*2)':x='min(iw-ow,max(0,trunc(iw*{:.8}/2)*2))':y='min(ih-oh,max(0,trunc(ih*{:.8}/2)*2))'",
            crop.width, crop.height, crop.x, crop.y
        ));
    }
    for overlay in &edits.overlays {
        parts.push(format!(
            "drawbox=x='iw*{:.8}':y='ih*{:.8}':w='iw*{:.8}':h='ih*{:.8}':color=black:t=fill",
            overlay.x, overlay.y, overlay.width, overlay.height
        ));
    }
    parts.push(format!(
        "scale=w=-2:h='min(ih,{height})':force_original_aspect_ratio=decrease"
    ));
    parts.join(",")
}

// ---- process helpers -----------------------------------------------------

/// Run a command to completion with an optional wall-clock timeout and a
/// cooperative cancel flag. Kills the child on timeout/cancel.
fn run_command(
    mut command: Command,
    timeout: Duration,
    cancel: Option<&CancelFlag>,
) -> std::io::Result<std::process::ExitStatus> {
    let started = Instant::now();
    let mut child = command.spawn()?;
    let deadline = started + timeout;
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(status),
            None => {
                if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        MediaError::ScanCancelled,
                    ));
                }
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "process timed out",
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Run with captured output (used for ffprobe and small extraction jobs).
fn run_captured(
    mut command: Command,
    timeout: Duration,
    cancel: Option<&CancelFlag>,
) -> std::io::Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>)> {
    use std::io::Read;
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout_handle = std::thread::spawn(move || -> Vec<u8> {
        let mut buf = Vec::new();
        let mut reader = stdout;
        let _ = reader.read_to_end(&mut buf);
        buf
    });
    let stderr_handle = std::thread::spawn(move || -> Vec<u8> {
        let mut buf = Vec::new();
        let mut reader = stderr;
        let _ = reader.read_to_end(&mut buf);
        buf
    });
    let started = Instant::now();
    let deadline = started + timeout;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None => {
                if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        MediaError::ScanCancelled,
                    ));
                }
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "process timed out",
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();
    Ok((status, stdout, stderr))
}

/// Run an ffmpeg job, streaming `-progress pipe:1` lines to `progress`.
/// `stage` is the label shown by the UI. Returns stderr tail on error.
fn run_ffmpeg_progress(
    mut command: Command,
    duration: f64,
    cancel: Option<&CancelFlag>,
    progress: &Arc<dyn Fn(f64, &str) + Send + Sync>,
    stage: &str,
) -> Result<(), MediaError> {
    use std::io::{BufRead, BufReader};
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| MediaError::Message(format!("{e}")))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stderr_tail: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let tail = Arc::clone(&stderr_tail);
    let stderr_reader = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = Vec::new();
        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let mut tail = tail.lock();
                    tail.extend_from_slice(&line);
                    if tail.len() > 16000 {
                        let overflow = tail.len() - 16000;
                        tail.drain(..overflow);
                    }
                }
            }
        }
    });

    let progress = Arc::clone(progress);
    let stage = stage.to_string();
    let progress_reader = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if let Some(micros) = trimmed
                        .strip_prefix("out_time_ms=")
                        .and_then(|v| v.parse::<f64>().ok())
                    {
                        let fraction = clamp(micros / 1e6 / duration, 0.0, 1.0);
                        progress(fraction, &stage);
                    }
                }
            }
        }
    });

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stderr_reader.join();
                    let _ = progress_reader.join();
                    return Err(MediaError::ProcessingCancelled);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(MediaError::Message("FFmpeg could not process this video.".into()));
            }
        }
    };
    let _ = stderr_reader.join();
    let _ = progress_reader.join();
    if !status.success() {
        let tail = stderr_tail.lock();
        let text = String::from_utf8_lossy(&tail);
        let detail = if text.trim().is_empty() {
            "FFmpeg could not process this video.".to_string()
        } else {
            text.chars().rev().take(2000).collect::<String>().chars().rev().collect()
        };
        return Err(MediaError::Message(detail));
    }
    Ok(())
}

// ---- MediaIndexer --------------------------------------------------------

pub struct MediaIndexer {
    pub database: Arc<Database>,
    pub export_dir: PathBuf,
}

impl MediaIndexer {
    pub fn available() -> bool {
        ffprobe_path().is_some() && ffmpeg_path().is_some()
    }

    fn ffmpeg(&self) -> Result<PathBuf, MediaError> {
        ffmpeg_path().ok_or_else(|| {
            MediaError::Message("FFmpeg is not available. Install FFmpeg or set CLIPRELAY_FFMPEG_DIR.".into())
        })
    }

    fn ffprobe(&self) -> Result<PathBuf, MediaError> {
        ffprobe_path().ok_or_else(|| {
            MediaError::Message("FFprobe is not available. Install FFmpeg or set CLIPRELAY_FFMPEG_DIR.".into())
        })
    }

    fn _looks_like_transport_stream(&self, path: &Path) -> bool {
        use std::io::Read;
        let Ok(mut file) = std::fs::File::open(path) else {
            return false;
        };
        let mut sample = vec![0u8; TRANSPORT_STREAM_SAMPLE_BYTES];
        let Ok(read) = file.read(&mut sample) else {
            return false;
        };
        sample.truncate(read);
        if sample.len() < 4 {
            return false;
        }
        for start in 0..sample.len() {
            if sample[start] != 0x47 {
                continue;
            }
            for packet_size in TRANSPORT_STREAM_PACKET_SIZES {
                let mut ok = true;
                for i in 0..4 {
                    let position = start + packet_size * i;
                    if position + 3 >= sample.len() {
                        ok = false;
                        break;
                    }
                    if sample[position] != 0x47 || (sample[position + 3] & 0x30) == 0 {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    return true;
                }
            }
        }
        false
    }

    fn _is_candidate(&self, path: &Path, deep_scan: bool) -> bool {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return false;
        };
        if name.starts_with('.') {
            return false;
        }
        let suffix = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
            .unwrap_or_default();
        if TRANSPORT_STREAM_EXTENSIONS.contains(&suffix.as_str()) {
            deep_scan || self._looks_like_transport_stream(path)
        } else {
            deep_scan || VIDEO_EXTENSIONS.contains(&suffix.as_str())
        }
    }

    /// Iterative DFS discovery (LIFO), dirs not followed through symlinks,
    /// hidden dirs and the export dir skipped when inside the root.
    pub fn iter_candidate_paths(
        &self,
        root: &Path,
        deep_scan: bool,
        cancel: Option<&CancelFlag>,
    ) -> Result<Vec<PathBuf>, MediaError> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let export_inside_root = crate::paths::is_within(&self.export_dir, &root);
        let mut pending: Vec<PathBuf> = vec![root];
        let mut results = Vec::new();
        while let Some(current) = pending.pop() {
            if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return Err(MediaError::ScanCancelled);
            }
            let entries = match std::fs::read_dir(&current) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            let mut dirs: Vec<PathBuf> = Vec::new();
            for entry in entries.flatten() {
                if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                    return Err(MediaError::ScanCancelled);
                }
                let path = entry.path();
                let is_dir = entry
                    .file_type()
                    .map(|t| t.is_dir())
                    .unwrap_or(false);
                if is_dir {
                    let Some(name) = entry.file_name().to_str().map(|n| n.to_string()) else {
                        continue;
                    };
                    if name.starts_with('.') {
                        continue;
                    }
                    if IGNORED_DIR_NAMES.contains(&name.to_lowercase().as_str()) {
                        continue;
                    }
                    if export_inside_root && path == self.export_dir {
                        continue;
                    }
                    dirs.push(path);
                } else if self._is_candidate(&path, deep_scan) {
                    results.push(path);
                }
            }
            pending.extend(dirs);
        }
        Ok(results)
    }

    fn _fraction(raw: &str) -> f64 {
        if raw.is_empty() {
            return 0.0;
        }
        if let Some((num, den)) = raw.split_once('/') {
            let n: f64 = num.trim().parse().unwrap_or(0.0);
            let d: f64 = den.trim().parse().unwrap_or(0.0);
            if d == 0.0 {
                return 0.0;
            }
            return n / d;
        }
        raw.trim().parse().unwrap_or(0.0)
    }

    fn _rotation(stream: &serde_json::Value) -> i64 {
        let mut rotation = 0.0;
        if let Some(tags) = stream.get("tags").and_then(|t| t.as_object()) {
            if let Some(value) = tags.get("rotate").and_then(|v| v.as_str()) {
                rotation = value.parse::<f64>().unwrap_or(0.0);
            }
        }
        if rotation == 0.0 {
            if let Some(side_data) = stream.get("side_data_list").and_then(|s| s.as_array()) {
                for item in side_data {
                    if let Some(value) = item.get("rotation").and_then(|v| v.as_str()) {
                        rotation = value.parse::<f64>().unwrap_or(0.0);
                        if rotation != 0.0 {
                            break;
                        }
                    }
                }
            }
        }
        ((rotation as i64) % 360 + 360) % 360
    }

    /// FFprobe a single file; `None` when unreadable, missing a video
    /// stream, or with no positive duration.
    pub fn probe(&self, path: &Path, root: &Path, cancel: Option<&CancelFlag>) -> Option<MediaMetadata> {
        let ffprobe = self.ffprobe().ok()?;
        let mut command = Command::new(&ffprobe);
        command
            .arg("-v")
            .arg("error")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(path);
        let (status, stdout, _) =
            run_captured(command, Duration::from_secs(35), cancel).ok()?;
        if !status.success() {
            return None;
        }
        let json: serde_json::Value = serde_json::from_slice(&stdout).unwrap_or(serde_json::Value::Null);
        if json.is_null() {
            return None;
        }
        let streams = json.get("streams").and_then(|s| s.as_array()).cloned().unwrap_or_default();
        let video = streams
            .iter()
            .find(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("video"))?;
        let audio = streams
            .iter()
            .find(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("audio"));
        let format_duration = json
            .get("format")
            .and_then(|f| f.get("duration"))
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok())
            .filter(|v| v.is_finite());
        let stream_duration = video
            .get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok())
            .filter(|v| v.is_finite());
        let duration = format_duration.or(stream_duration).unwrap_or(0.0);
        if !duration.is_finite() || duration <= 0.0 {
            return None;
        }
        let mut width = video.get("width").and_then(|w| w.as_i64()).unwrap_or(0);
        let mut height = video.get("height").and_then(|h| h.as_i64()).unwrap_or(0);
        if matches!(Self::_rotation(video), 90 | 270) {
            std::mem::swap(&mut width, &mut height);
        }
        let avg = video
            .get("avg_frame_rate")
            .and_then(|r| r.as_str())
            .unwrap_or("");
        let r = video
            .get("r_frame_rate")
            .and_then(|r| r.as_str())
            .unwrap_or("");
        let frame_rate = Self::_fraction(avg).max(Self::_fraction(r));
        let stat = std::fs::metadata(path).ok()?;
        let relative = path.strip_prefix(root).unwrap_or(path);
        let folder = match relative.parent() {
            Some(parent) if parent.as_os_str().is_empty() => String::new(),
            Some(parent) => parent.to_string_lossy().replace('\\', "/"),
            None => String::new(),
        };
        Some(MediaMetadata {
            root_path: root.to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            relative_path: relative.to_string_lossy().replace('\\', "/"),
            folder,
            duration,
            width,
            height,
            size_bytes: stat.len() as i64,
            video_codec: video
                .get("codec_name")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
            audio_codec: audio
                .and_then(|a| a.get("codec_name"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
            frame_rate,
            mtime: stat
                .modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
                .unwrap_or(0.0),
        })
    }

    /// Stat-only metadata (probe fields zeroed).
    pub fn minimal_metadata(&self, path: &Path, root: &Path) -> Result<MediaMetadata> {
        let stat = std::fs::metadata(path)?;
        let relative = path.strip_prefix(root).unwrap_or(path);
        let folder = match relative.parent() {
            Some(parent) if parent.as_os_str().is_empty() => String::new(),
            Some(parent) => parent.to_string_lossy().replace('\\', "/"),
            None => String::new(),
        };
        Ok(MediaMetadata {
            root_path: root.to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            relative_path: relative.to_string_lossy().replace('\\', "/"),
            folder,
            duration: 0.0,
            width: 0,
            height: 0,
            size_bytes: stat.len() as i64,
            video_codec: String::new(),
            audio_codec: String::new(),
            frame_rate: 0.0,
            mtime: stat
                .modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
                .unwrap_or(0.0),
        })
    }

    /// Fast stat-only manifest sync. `batch_ready(added)` after each commit.
    pub fn refresh_manifest(
        &self,
        root_path: &Path,
        batch_size: usize,
        check_changes: bool,
        cancel: Option<&CancelFlag>,
        progress: &mut dyn FnMut(usize, &str),
        mut batch_ready: Option<&mut dyn FnMut(usize)>,
    ) -> Result<ScanResult, MediaError> {
        let root = root_path
            .canonicalize()
            .map_err(|_| MediaError::Message("The selected library folder is unavailable.".into()))?;
        let mut result = ScanResult::default();
        let mut batch: Vec<crate::db::ManifestEntry> = Vec::new();
        let mut present_paths: Vec<String> = Vec::new();
        let mut first_committed = false;
        let mut last_progress = Instant::now() - Duration::from_secs(1);
        let candidates = self.iter_candidate_paths(&root, false, cancel)?;
        let state_map = self.database.media_state_map(&root.to_string_lossy()).unwrap_or_default();
        let mut flush = |batch: &mut Vec<crate::db::ManifestEntry>,
                         result: &mut ScanResult,
                         first_committed: &mut bool,
                         database: &Arc<Database>|
         -> Result<(), MediaError> {
            if batch.is_empty() {
                return Ok(());
            }
            let added = database
                .upsert_manifest_batch(batch)
                .map_err(|e| MediaError::Message(format!("{e}")))?;
            result.indexed += added;
            *first_committed = true;
            if let Some(callback) = batch_ready.as_mut() {
                if added > 0 {
                    callback(added);
                }
            }
            batch.clear();
            Ok(())
        };
        for path in candidates {
            result.discovered += 1;
            if result.discovered == 1 || last_progress.elapsed() >= Duration::from_millis(250) {
                last_progress = Instant::now();
                progress(
                    result.discovered,
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                        .as_str(),
                );
            }
            let path_str = path.to_string_lossy().into_owned();
            let cached = state_map.get(&path_str);
            let entry = if let Some(cached_row) = cached {
                if !check_changes {
                    crate::db::ManifestEntry {
                        root_path: root.to_string_lossy().into_owned(),
                        path: cached_row.path.clone(),
                        name: cached_row.name.clone(),
                        relative_path: cached_row.relative_path.clone(),
                        folder: cached_row.folder.clone(),
                        size_bytes: cached_row.size_bytes,
                        mtime: cached_row.mtime,
                    }
                } else {
                    match self.stat_entry(&path, &root) {
                        Some(entry) => entry,
                        None => {
                            result.failed += 1;
                            continue;
                        }
                    }
                }
            } else {
                match self.stat_entry(&path, &root) {
                    Some(entry) => entry,
                    None => {
                        result.failed += 1;
                        continue;
                    }
                }
            };
            present_paths.push(path_str);
            batch.push(entry);
            if !first_committed || batch.len() >= batch_size.max(1) {
                flush(&mut batch, &mut result, &mut first_committed, &self.database)?;
            }
        }
        flush(&mut batch, &mut result, &mut first_committed, &self.database)?;
        result.skipped = self
            .database
            .invalidate_absent(&root.to_string_lossy(), &present_paths)
            .map_err(|e| MediaError::Message(format!("{e}")))?;
        progress(result.discovered, "");
        Ok(result)
    }

    fn stat_entry(&self, path: &Path, root: &Path) -> Option<crate::db::ManifestEntry> {
        let stat = std::fs::metadata(path).ok()?;
        let relative = path.strip_prefix(root).unwrap_or(path);
        let folder = match relative.parent() {
            Some(parent) if parent.as_os_str().is_empty() => String::new(),
            Some(parent) => parent.to_string_lossy().replace('\\', "/"),
            None => String::new(),
        };
        Some(crate::db::ManifestEntry {
            root_path: root.to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            relative_path: relative.to_string_lossy().replace('\\', "/"),
            folder,
            size_bytes: stat.len() as i64,
            mtime: stat
                .modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
                .unwrap_or(0.0),
        })
    }

    /// Full scan: probe + thumbnails, using a small worker pool.
    pub fn scan(
        &self,
        root_path: &Path,
        deep_scan: bool,
        verify_media: bool,
        generate_thumbnails: bool,
        workers: usize,
        cancel: Option<&CancelFlag>,
        progress: &mut dyn FnMut(usize, usize, &str),
        mut item_ready: Option<&mut dyn FnMut(i64)>,
    ) -> Result<ScanResult, MediaError> {
        let root = root_path
            .canonicalize()
            .map_err(|_| MediaError::Message("The selected library folder is unavailable.".into()))?;
        let mut result = ScanResult::default();
        let candidates = if verify_media {
            self.iter_candidate_paths(&root, deep_scan, cancel)?
        } else {
            // Trusted manifest: reuse valid rows from the database.
            self.database
                .manifest_paths(&root.to_string_lossy())
                .map_err(|e| MediaError::Message(format!("{e}")))?
        };
        result.discovered = candidates.len();
        let state_map = self.database.media_state_map(&root.to_string_lossy()).unwrap_or_default();
        let mut to_probe: Vec<PathBuf> = Vec::new();
        let mut cached_thumbnail_ids: Vec<i64> = Vec::new();
        let mut valid_paths: Vec<String> = Vec::new();

        for path in &candidates {
            let path_str = path.to_string_lossy().into_owned();
            let cached = state_map.get(&path_str);
            let needs_probe = if let Some(cached_row) = cached {
                if !verify_media {
                    !cached_row.valid
                } else {
                    !cached_row.valid || cached_row.duration <= 0.0
                }
            } else {
                match std::fs::metadata(path) {
                    Ok(stat) => {
                        let mtime = stat
                            .modified()
                            .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
                            .unwrap_or(0.0);
                        self.database
                            .media_needs_probe(&path_str, stat.len() as i64, mtime, verify_media)
                            .map_err(|e| MediaError::Message(format!("{e}")))?
                    }
                    Err(_) => {
                        result.failed += 1;
                        continue;
                    }
                }
            };
            if needs_probe {
                to_probe.push(path.clone());
            } else {
                result.skipped += 1;
                valid_paths.push(path_str.clone());
                if generate_thumbnails {
                    if let Some(cached_row) = cached {
                        if cached_row.thumbnail_path.is_none() {
                            cached_thumbnail_ids.push(cached_row.id);
                        }
                    }
                }
            }
        }

        let total = to_probe.len() + cached_thumbnail_ids.len();
        let mut completed = 0usize;
        let workers = workers.clamp(1, 8);

        if !verify_media {
            // Branch A: sequential minimal metadata.
            for path in to_probe {
                if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                    return Err(MediaError::ScanCancelled);
                }
                match self.minimal_metadata(&path, &root) {
                    Ok(metadata) => {
                        let id = self
                            .database
                            .upsert_media(&metadata)
                            .map_err(|e| MediaError::Message(format!("{e}")))?;
                        result.indexed += 1;
                        valid_paths.push(metadata.path.clone());
                        if let Some(callback) = item_ready.as_mut() {
                            callback(id);
                        }
                        let _ = self.ensure_thumbnail(id, cancel);
                        if let Some(callback) = item_ready.as_mut() {
                            callback(id);
                        }
                    }
                    Err(_) => {
                        result.failed += 1;
                    }
                }
                completed += 1;
                progress(
                    completed,
                    total,
                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().as_str(),
                );
            }
        } else {
            // Branch B: parallel probing.
            let probe_list: Vec<PathBuf> = to_probe.clone();
            let index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let database = Arc::clone(&self.database);
            let root_arc = Arc::new(root.clone());
            let (tx, rx) = crossbeam_channel::bounded::<Option<(PathBuf, Option<MediaMetadata>)>>(workers * 2);
            let mut handles = Vec::new();
            for _ in 0..workers {
                let index = Arc::clone(&index);
                let probe_list = probe_list.clone();
                let root = Arc::clone(&root_arc);
                let tx = tx.clone();
                let cancel = cancel.cloned();
                let indexer = MediaIndexer {
                    database: Arc::clone(&database),
                    export_dir: self.export_dir.clone(),
                };
                handles.push(std::thread::spawn(move || {
                    loop {
                        let idx = index.fetch_add(1, Ordering::Relaxed);
                        if idx >= probe_list.len() {
                            break;
                        }
                        let path = &probe_list[idx];
                        let metadata = if cancel.as_ref().is_some_and(|f| f.load(Ordering::Relaxed)) {
                            None
                        } else {
                            indexer.probe(path, &root, cancel.as_ref())
                        };
                        if tx.send(Some((path.clone(), metadata))).is_err() {
                            break;
                        }
                    }
                    let _ = tx.send(None);
                }));
            }
            drop(tx);
            for message in rx.iter() {
                match message {
                    Some((path, metadata)) => {
                        match metadata {
                            Some(metadata) => {
                                let id = self
                                    .database
                                    .upsert_media(&metadata)
                                    .map_err(|e| MediaError::Message(format!("{e}")))?;
                                result.indexed += 1;
                                valid_paths.push(metadata.path.clone());
                                if let Some(callback) = item_ready.as_mut() {
                                    callback(id);
                                }
                                let _ = self.ensure_thumbnail(id, cancel);
                                if let Some(callback) = item_ready.as_mut() {
                                    callback(id);
                                }
                            }
                            None => {
                                result.failed += 1;
                            }
                        }
                        completed += 1;
                        progress(
                            completed,
                            total,
                            path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().as_str(),
                        );
                    }
                    None => {
                        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                            for handle in handles.drain(..) {
                                let _ = handle.join();
                            }
                            return Err(MediaError::ScanCancelled);
                        }
                        // Count terminators; continue until all workers done.
                    }
                }
                if handles.iter().all(|h| h.is_finished()) && rx.is_empty() {
                    break;
                }
            }
            for handle in handles {
                let _ = handle.join();
            }
        }

        // Branch C: cached thumbnails.
        if !cached_thumbnail_ids.is_empty() {
            let index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let ids = cached_thumbnail_ids.clone();
            let state_map = Arc::new(state_map);
            let (tx, rx) = crossbeam_channel::bounded::<Option<(i64, Option<PathBuf>)>>(workers * 2);
            let mut handles = Vec::new();
            let thumb_workers = workers.min(4);
            for _ in 0..thumb_workers {
                let index = Arc::clone(&index);
                let ids = ids.clone();
                let tx = tx.clone();
                let cancel = cancel.cloned();
                let database = Arc::clone(&self.database);
                let indexer = MediaIndexer {
                    database,
                    export_dir: self.export_dir.clone(),
                };
                handles.push(std::thread::spawn(move || {
                    loop {
                        let idx = index.fetch_add(1, Ordering::Relaxed);
                        if idx >= ids.len() {
                            break;
                        }
                        let id = ids[idx];
                        let media = indexer.database.get_media(id).ok().flatten();
                        let thumbnail = if cancel.as_ref().is_some_and(|f| f.load(Ordering::Relaxed)) {
                            None
                        } else {
                            indexer.ensure_thumbnail_for(id, media.as_ref(), cancel.as_ref())
                        };
                        if tx.send(Some((id, thumbnail))).is_err() {
                            break;
                        }
                    }
                    let _ = tx.send(None);
                }));
            }
            drop(tx);
            for message in rx.iter() {
                match message {
                    Some((id, thumbnail)) => {
                        if thumbnail.is_some() {
                            if let Some(callback) = item_ready.as_mut() {
                                callback(id);
                            }
                        }
                        completed += 1;
                        let name = state_map
                            .values()
                            .find(|m| m.id == id)
                            .map(|m| m.name.clone())
                            .unwrap_or_else(|| "thumbnail".to_string());
                        progress(completed, total, &name);
                    }
                    None => {}
                }
                if handles.iter().all(|h| h.is_finished()) && rx.is_empty() {
                    break;
                }
            }
            for handle in handles {
                let _ = handle.join();
            }
        }

        self.database
            .invalidate_missing(&root.to_string_lossy(), &valid_paths)
            .map_err(|e| MediaError::Message(format!("{e}")))?;
        Ok(result)
    }

    // ---- thumbnails / previews / timeline ------------------------------

    pub fn ensure_thumbnail(&self, media_id: i64, cancel: Option<&CancelFlag>) -> Option<PathBuf> {
        let media = self.database.get_media(media_id).ok().flatten()?;
        self.ensure_thumbnail_for(media_id, Some(&media), cancel)
    }

    fn ensure_thumbnail_for(
        &self,
        media_id: i64,
        media: Option<&crate::db::MediaRow>,
        cancel: Option<&CancelFlag>,
    ) -> Option<PathBuf> {
        let media = media?;
        if let Some(existing) = media.thumbnail_path.as_deref() {
            let existing_path = Path::new(existing);
            if existing_path.is_file() && media_path_size(existing_path) > 0 {
                let _ = self
                    .database
                    .set_media_asset(media_id, "thumbnail_path", existing);
                return Some(PathBuf::from(existing));
            }
        }
        let ffmpeg = self.ffmpeg().ok()?;
        let key = media_cache_key(Path::new(&media.path), media.size_bytes as u64, media.mtime);
        let output = thumbnail_dir().join(format!("{key}.jpg"));
        if output.is_file() && media_path_size(&output) > 0 {
            let _ = self
                .database
                .set_media_asset(media_id, "thumbnail_path", &output.to_string_lossy());
            return Some(output);
        }
        let duration = media.duration.max(0.0);
        let timestamp = clamp(duration * 0.15, 0.0, (duration - 0.1).max(0.0));
        let mut command = Command::new(&ffmpeg);
        command
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-ss")
            .arg(format!("{timestamp:.3}"))
            .arg("-i")
            .arg(&media.path)
            .arg("-frames:v")
            .arg("1")
            .arg("-vf")
            .arg("scale=640:360:force_original_aspect_ratio=decrease")
            .arg("-q:v")
            .arg("3")
            .arg(&output);
        let result = run_command(command, Duration::from_secs(60), cancel);
        match result {
            Ok(status) if status.success() && output.is_file() && media_path_size(&output) > 0 => {
                let _ = self
                    .database
                    .set_media_asset(media_id, "thumbnail_path", &output.to_string_lossy());
                Some(output)
            }
            _ => {
                let _ = std::fs::remove_file(&output);
                None
            }
        }
    }

    pub fn ensure_preview(&self, media_id: i64) -> Option<PathBuf> {
        let media = self.ensure_metadata(media_id)?;
        if let Some(existing) = media.preview_path.as_deref() {
            if Path::new(existing).is_file() {
                return Some(PathBuf::from(existing));
            }
        }
        let ffmpeg = self.ffmpeg().ok()?;
        let key = media_cache_key(Path::new(&media.path), media.size_bytes as u64, media.mtime);
        let output = preview_dir().join(format!("{key}.mp4"));
        if output.is_file() && media_path_size(&output) > 0 {
            let _ = self
                .database
                .set_media_asset(media_id, "preview_path", &output.to_string_lossy());
            return Some(output);
        }
        let duration = media.duration.max(0.0);
        let start = clamp(duration * 0.12, 0.0, (duration - 0.1).max(0.0));
        let preview_length = (duration - start).clamp(1.0, 8.0);
        let partial = output.with_extension("partial.mp4");
        let mut command = Command::new(&ffmpeg);
        command
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-ss")
            .arg(format!("{start:.3}"))
            .arg("-i")
            .arg(&media.path)
            .arg("-t")
            .arg(format!("{preview_length:.3}"))
            .arg("-vf")
            .arg("scale=640:360:force_original_aspect_ratio=decrease")
            .arg("-an")
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("veryfast")
            .arg("-crf")
            .arg("29")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-movflags")
            .arg("+faststart")
            .arg(&partial);
        let result = run_command(command, Duration::from_secs(120), None);
        match result {
            Ok(status) if status.success() && partial.is_file() && media_path_size(&partial) > 0 => {
                let _ = std::fs::rename(&partial, &output);
                let _ = self
                    .database
                    .set_media_asset(media_id, "preview_path", &output.to_string_lossy());
                Some(output)
            }
            _ => {
                let _ = std::fs::remove_file(&partial);
                None
            }
        }
    }

    pub fn ensure_timeline(&self, media_id: i64, frame_count: usize) -> Option<PathBuf> {
        let media = self.ensure_metadata(media_id)?;
        if let Some(existing) = media.timeline_path.as_deref() {
            if Path::new(existing).is_file() {
                return Some(PathBuf::from(existing));
            }
        }
        let ffmpeg = self.ffmpeg().ok()?;
        let key = media_cache_key(Path::new(&media.path), media.size_bytes as u64, media.mtime);
        let output = timeline_dir().join(format!("{key}.jpg"));
        if output.is_file() && media_path_size(&output) > 0 {
            let _ = self
                .database
                .set_media_asset(media_id, "timeline_path", &output.to_string_lossy());
            return Some(output);
        }
        let duration = media.duration.max(0.1);
        let frames = frame_count.clamp(4, 20);
        let stem = output.with_extension("");
        let partial = output.with_extension("partial.jpg");
        let frame_pattern = output
            .with_extension(format!("partial-{:02}.jpg", 0))
            .with_file_name(format!(
                "{}.partial-%02d.jpg",
                stem.file_name().unwrap().to_string_lossy()
            ));
        let frame_paths: Vec<PathBuf> = (0..frames)
            .map(|i| {
                output.with_file_name(format!(
                    "{}.partial-{i:02}.jpg",
                    stem.file_name().unwrap().to_string_lossy()
                ))
            })
            .collect();
        let scale_filter = "scale=160:90:force_original_aspect_ratio=increase,crop=160:90";
        for i in 0..frames {
            let _ = std::fs::remove_file(&frame_paths[i]);
            let timestamp = duration * (i as f64 + 0.5) / frames as f64;
            let mut command = Command::new(&ffmpeg);
            command
                .arg("-hide_banner")
                .arg("-loglevel")
                .arg("error")
                .arg("-y")
                .arg("-ss")
                .arg(format!("{timestamp:.6}"))
                .arg("-i")
                .arg(&media.path)
                .arg("-frames:v")
                .arg("1")
                .arg("-vf")
                .arg(scale_filter)
                .arg("-q:v")
                .arg("4")
                .arg(&frame_paths[i]);
            let result = run_command(command, Duration::from_secs(20), None);
            let ok = matches!(result, Ok(status) if status.success())
                && frame_paths[i].is_file()
                && media_path_size(&frame_paths[i]) > 0;
            if !ok {
                for frame in &frame_paths {
                    let _ = std::fs::remove_file(frame);
                }
                let _ = std::fs::remove_file(&partial);
                return None;
            }
        }
        let mut command = Command::new(&ffmpeg);
        command
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-framerate")
            .arg("1")
            .arg("-start_number")
            .arg("0")
            .arg("-i")
            .arg(&frame_pattern)
            .arg("-frames:v")
            .arg("1")
            .arg("-vf")
            .arg(format!("tile={frames}x1:padding=0:margin=0"))
            .arg("-q:v")
            .arg("4")
            .arg(&partial);
        let result = run_command(command, Duration::from_secs(30), None);
        let ok = matches!(result, Ok(status) if status.success())
            && partial.is_file()
            && media_path_size(&partial) > 0;
        for frame in &frame_paths {
            let _ = std::fs::remove_file(frame);
        }
        if ok {
            let _ = std::fs::rename(&partial, &output);
            let _ = self
                .database
                .set_media_asset(media_id, "timeline_path", &output.to_string_lossy());
            Some(output)
        } else {
            let _ = std::fs::remove_file(&partial);
            None
        }
    }

    /// Ensure full probed metadata exists for a media id; probe on demand
    /// when stale, mark invalid on failure.
    pub fn ensure_metadata(&self, media_id: i64) -> Option<crate::db::MediaRow> {
        let media = self.database.get_media(media_id).ok()??;
        // Stat the file first (mirrors the original): a missing file is
        // marked invalid immediately, and in-place changes re-probe.
        let path = PathBuf::from(&media.path);
        let stat = std::fs::metadata(&path).ok();
        let Some(stat) = stat else {
            let _ = self.database.set_media_valid(media_id, false);
            return None;
        };
        let mtime = stat
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let changed = stat.len() != media.size_bytes as u64
            || (mtime - media.mtime).abs() > 0.001;
        if media.duration > 0.0 && media.valid && !changed {
            return Some(media);
        }
        let root = PathBuf::from(&media.root_path);
        match self.probe(&path, &root, None) {
            Some(metadata) => {
                let id = self.database.upsert_media(&metadata).ok()?;
                self.database.get_media(id).ok().flatten()
            }
            None => {
                let _ = self.database.set_media_valid(media_id, false);
                None
            }
        }
    }
}

fn media_path_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Lightweight ffprobe used for Telegram upload attributes: returns
/// `(duration_ms, width, height)` with zeros on any failure.
pub fn quick_probe_info(path: &Path) -> (i64, i64, i64) {
    let Some(ffprobe) = ffprobe_path() else {
        return (0, 0, 0);
    };
    let mut command = Command::new(&ffprobe);
    command
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path);
    let Ok((status, stdout, _)) = run_captured(command, Duration::from_secs(15), None) else {
        return (0, 0, 0);
    };
    if !status.success() {
        return (0, 0, 0);
    }
    let json: serde_json::Value = serde_json::from_slice(&stdout).unwrap_or(serde_json::Value::Null);
    let streams = json
        .get("streams")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let video = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("video"));
    let duration = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
        .or_else(|| {
            video
                .and_then(|v| v.get("duration"))
                .and_then(|d| d.as_str())
                .and_then(|d| d.parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    let width = video.and_then(|v| v.get("width")).and_then(|w| w.as_i64()).unwrap_or(0);
    let height = video.and_then(|v| v.get("height")).and_then(|h| h.as_i64()).unwrap_or(0);
    ((duration * 1000.0) as i64, width, height)
}

// ---- MediaProcessor ------------------------------------------------------

/// Encoder mode: "auto" resolves at export time (hardware when available),
/// "hardware" forces it, "software" never uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderMode {
    Auto,
    Hardware,
    Software,
}

impl EncoderMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "hardware" => EncoderMode::Hardware,
            "software" => EncoderMode::Software,
            _ => EncoderMode::Auto,
        }
    }
}

static HARDWARE_ENCODER: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn hardware_encoder_candidates() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["h264_videotoolbox"]
    } else if cfg!(target_os = "windows") {
        &["h264_nvenc", "h264_qsv", "h264_amf"]
    } else {
        &[]
    }
}

/// Test helper: expose the hardware encoder detection.
pub fn detect_hardware_encoder_for_tests() -> Option<String> {
    detect_hardware_encoder()
}

fn detect_hardware_encoder() -> Option<String> {
    let cache = HARDWARE_ENCODER.get_or_init(|| Mutex::new(None));
    if let Some(encoder) = cache.lock().clone() {
        return Some(encoder);
    }
    let ffmpeg = ffmpeg_path()?;
    for encoder in hardware_encoder_candidates() {
        let mut command = Command::new(&ffmpeg);
        command
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("color=c=black:s=64x64:r=1:d=0.1")
            .arg("-frames:v")
            .arg("1")
            .arg("-an")
            .arg("-c:v")
            .arg(encoder)
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-f")
            .arg("null")
            .arg("-");
        let (status, _, _) = match run_captured(command, Duration::from_secs(12), None) {
            Ok(result) => result,
            Err(_) => continue,
        };
        if status.success() {
            *cache.lock() = Some(encoder.to_string());
            return Some(encoder.to_string());
        }
    }
    None
}

pub fn hardware_encoder_info() -> (bool, String, String) {
    let label = |encoder: &str| -> &'static str {
        match encoder {
            "h264_videotoolbox" => "VideoToolbox H.264",
            "h264_nvenc" => "NVIDIA NVENC H.264",
            "h264_qsv" => "Intel Quick Sync H.264",
            "h264_amf" => "AMD AMF H.264",
            "libx264" => "libx264",
            _ => "Unavailable",
        }
    };
    match detect_hardware_encoder() {
        Some(encoder) => (true, encoder.clone(), label(&encoder).to_string()),
        None => (false, String::new(), "Unavailable".to_string()),
    }
}

pub struct MediaProcessor {
    pub database: Arc<Database>,
    pub export_dir: PathBuf,
    encoder_mode: Mutex<EncoderMode>,
    cancel: CancelFlag,
    /// Edit spec of the in-flight export (needed to rebuild the filter with
    /// the target height on the two-pass path).
    last_edits: Mutex<Option<EditSpec>>,
}

impl MediaProcessor {
    pub fn new(database: Arc<Database>, export_dir: PathBuf, encoder_mode: EncoderMode) -> Self {
        Self {
            database,
            export_dir,
            encoder_mode: Mutex::new(encoder_mode),
            cancel: Arc::new(AtomicBool::new(false)),
            last_edits: Mutex::new(None),
        }
    }

    /// Set the encoder mode; safe to call while exports are running.
    pub fn set_encoder_mode(&self, mode: EncoderMode) {
        *self.encoder_mode.lock() = mode;
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn reset_cancel(&self) {
        self.cancel.store(false, Ordering::Relaxed);
    }

    fn ffmpeg(&self) -> Result<PathBuf, MediaError> {
        ffmpeg_path().ok_or_else(|| {
            MediaError::Message("FFmpeg is not available. Install FFmpeg or set CLIPRELAY_FFMPEG_DIR.".into())
        })
    }

    /// Port of `export()`: full pipeline with passthrough, hardware, and
    /// software (two-pass or CRF) paths. `progress(fraction 0..1, stage)`.
    pub fn export(
        &self,
        media: &crate::db::MediaRow,
        trim_start: f64,
        trim_end: f64,
        preset: &str,
        target_mb: f64,
        edits: &EditSpec,
        progress: &Arc<dyn Fn(f64, &str) + Send + Sync>,
    ) -> Result<ExportResult, MediaError> {
        self.reset_cancel();
        self.ffmpeg()?;
        let source = PathBuf::from(&media.path);
        let source_duration = media.duration.max(0.0);
        let start = clamp(trim_start, 0.0, (source_duration - 0.05).max(0.0));
        let end = clamp(
            if trim_end > 0.0 { trim_end } else { source_duration },
            start + 0.05,
            source_duration,
        );
        let duration = end - start;
        let no_trim = start <= 0.01 && (end - source_duration).abs() <= 0.05;
        let has_edits = !edits.is_empty();
        let compatible = matches!(media.video_codec.to_lowercase().as_str(), "h264" | "avc1")
            && matches!(media.audio_codec.to_lowercase().as_str(), "" | "aac")
            && source
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase() == "mp4")
                .unwrap_or(false);
        let target_passthrough = matches!(preset, "fit_bot" | "fit_x" | "fit_both" | "custom")
            && target_mb > 0.0
            && (media.size_bytes as f64) <= target_mb * 1024.0 * 1024.0 * 0.98;
        if no_trim && !has_edits && compatible && (preset == "original" || target_passthrough) {
            progress(1.0, "Using original video");
            return Ok(ExportResult {
                path: source.clone(),
                size_bytes: media.size_bytes as u64,
                duration,
                generated: false,
                preset: preset.to_string(),
                encoder: "stream copy".to_string(),
                hardware_accelerated: false,
            });
        }
        // Output naming.
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let name_suffix = if has_edits { "edited" } else { "prepared" };
        let stem = safe_stem(&media.name, 72);
        let mut output = self
            .export_dir
            .join(format!("{stem}_{name_suffix}_{timestamp}.mp4"));
        let mut counter = 2usize;
        while output.exists() {
            output = self
                .export_dir
                .join(format!("{stem}_{name_suffix}_{timestamp}-{counter}.mp4"));
            counter += 1;
        }
        let partial = output.with_extension("partial.mp4");
        let _ = std::fs::remove_file(&partial);

        // Keep the edit spec for the post-export size check pass.
        *self.last_edits.lock() = Some(edits.clone());

        let common = |command: &mut Command| {
            command
                .arg("-hide_banner")
                .arg("-y")
                .arg("-i")
                .arg(&source)
                .arg("-ss")
                .arg(format!("{start:.3}"))
                .arg("-t")
                .arg(format!("{duration:.3}"));
        };

        let encoder_mode = *self.encoder_mode.lock();
        let hardware_encoder: Option<String> = if encoder_mode == EncoderMode::Software {
            None
        } else {
            detect_hardware_encoder()
        };

        let (hardware_used, used_encoder) = if let Some(encoder) = hardware_encoder {
            let label = match encoder.as_str() {
                "h264_videotoolbox" => "VideoToolbox H.264",
                "h264_nvenc" => "NVIDIA NVENC H.264",
                "h264_qsv" => "Intel Quick Sync H.264",
                "h264_amf" => "AMD AMF H.264",
                _ => "Unavailable",
            };
            progress(0.01, &format!("Encoding with {label}"));
            let profile = hardware_profile(media, preset, duration, target_mb);
            let filter = build_filter(edits, profile.height);
            let mut command = Command::new(self.ffmpeg()?);
            common(&mut command);
            command
                .arg("-vf")
                .arg(&filter)
                .arg("-c:v")
                .arg(&encoder)
                .arg("-b:v")
                .arg(format!("{}k", profile.video_kbps))
                .arg("-maxrate")
                .arg(format!("{}k", (profile.video_kbps as f64 * 1.15) as i64))
                .arg("-bufsize")
                .arg(format!("{}k", profile.video_kbps * 2))
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg(format!("{}k", profile.audio_kbps))
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-movflags")
                .arg("+faststart")
                .arg("-progress")
                .arg("pipe:1")
                .arg("-nostats")
                .arg(&partial);
            match run_ffmpeg_progress(
                command,
                duration,
                Some(&self.cancel),
                progress,
                &format!("Encoding with {label}"),
            ) {
                Ok(()) => (true, label.to_string()),
                Err(MediaError::ProcessingCancelled) => {
                    let _ = std::fs::remove_file(&partial);
                    return Err(MediaError::ProcessingCancelled);
                }
                Err(_) => {
                    let _ = std::fs::remove_file(&partial);
                    if target_mb > 0.0 {
                        // Size check happens post-hoc in Python; mirror it:
                        // the failure path falls back to software regardless.
                    }
                    progress(0.01, "Hardware export unavailable · retrying with software");
                    (false, "libx264".to_string())
                }
            }
        } else {
            if encoder_mode == EncoderMode::Hardware {
                progress(0.01, "No compatible hardware encoder · using software");
            }
            (false, "libx264".to_string())
        };

        // Post-hardware size gate (Python checks before deciding fallback).
        if hardware_used && target_mb > 0.0 {
            if media_path_size(&partial) as f64 > target_mb * 1024.0 * 1024.0 * 1.015 {
                let _ = std::fs::remove_file(&partial);
                progress(0.01, "Hardware export unavailable · retrying with software");
                self.export_software(&media, start, duration, preset, target_mb, &partial, progress)?;
                return self.finish_export(partial, output, duration, preset, "libx264".to_string(), false, progress);
            }
        }

        if hardware_used {
            return self.finish_export(partial, output, duration, preset, used_encoder, true, progress);
        }

        self.export_software(&media, start, duration, preset, target_mb, &partial, progress)?;
        self.finish_export(partial, output, duration, preset, used_encoder, false, progress)
    }

    fn export_software(
        &self,
        media: &crate::db::MediaRow,
        start: f64,
        duration: f64,
        preset: &str,
        target_mb: f64,
        partial: &Path,
        progress: &Arc<dyn Fn(f64, &str) + Send + Sync>,
    ) -> Result<(), MediaError> {
        let source = PathBuf::from(&media.path);
        let common = |command: &mut Command| {
            command
                .arg("-hide_banner")
                .arg("-y")
                .arg("-i")
                .arg(&source)
                .arg("-ss")
                .arg(format!("{start:.3}"))
                .arg("-t")
                .arg(format!("{duration:.3}"));
        };
        let size_targeted = matches!(preset, "fit_bot" | "fit_x" | "fit_both" | "custom")
            && target_mb > 0.0;
        if size_targeted {
            // Two-pass with the exact Python bitrate math.
            let audio_kbps: i64 = if target_mb < 100.0 { 96 } else { 128 };
            let total_kbps = target_mb * 1024.0 * 1024.0 * 8.0 * 0.965 / duration / 1000.0;
            let video_kbps = (total_kbps - audio_kbps as f64).max(120.0) as i64;
            let height = if video_kbps >= 1800 {
                1080
            } else if video_kbps >= 700 {
                720
            } else {
                480
            };
            let pass_filter = build_filter(&EditSpec::default(), height);
            let _ = &pass_filter;
            // Rebuild filter with the computed height (crop/overlay unchanged,
            // only the scale target differs).
            let size_filter = self.size_filter_for_height(height);
            let tmp = std::env::temp_dir().join(format!(
                "cliprelay-pass-{}",
                std::process::id()
            ));
            let _ = std::fs::create_dir_all(&tmp);
            let passlog = tmp.join("passlog");
            let null_output = if cfg!(target_os = "windows") { "NUL" } else { "/dev/null" };

            // Pass 1.
            progress(0.01, "Measuring target size");
            let mut pass1 = Command::new(self.ffmpeg()?);
            common(&mut pass1);
            pass1
                .arg("-vf")
                .arg(&size_filter)
                .arg("-c:v")
                .arg("libx264")
                .arg("-preset")
                .arg("medium")
                .arg("-b:v")
                .arg(format!("{video_kbps}k"))
                .arg("-maxrate")
                .arg(format!("{}k", (video_kbps as f64 * 1.15) as i64))
                .arg("-bufsize")
                .arg(format!("{}k", video_kbps * 2))
                .arg("-pass")
                .arg("1")
                .arg("-passlogfile")
                .arg(&passlog)
                .arg("-an")
                .arg("-progress")
                .arg("pipe:1")
                .arg("-nostats")
                .arg("-f")
                .arg("null")
                .arg(null_output);
            let pass1_progress: Arc<dyn Fn(f64, &str) + Send + Sync> = {
                let progress = Arc::clone(progress);
                Arc::new(move |fraction, stage| progress(fraction * 0.45, stage))
            };
            run_ffmpeg_progress(
                pass1,
                duration,
                Some(&self.cancel),
                &pass1_progress,
                "Measuring target size",
            )
            .map_err(|e| {
                let _ = std::fs::remove_file(partial);
                e
            })?;

            // Pass 2.
            let mut pass2 = Command::new(self.ffmpeg()?);
            common(&mut pass2);
            pass2
                .arg("-vf")
                .arg(&size_filter)
                .arg("-c:v")
                .arg("libx264")
                .arg("-preset")
                .arg("medium")
                .arg("-b:v")
                .arg(format!("{video_kbps}k"))
                .arg("-maxrate")
                .arg(format!("{}k", (video_kbps as f64 * 1.15) as i64))
                .arg("-bufsize")
                .arg(format!("{}k", video_kbps * 2))
                .arg("-pass")
                .arg("2")
                .arg("-passlogfile")
                .arg(&passlog)
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg(format!("{audio_kbps}k"))
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-movflags")
                .arg("+faststart")
                .arg("-progress")
                .arg("pipe:1")
                .arg("-nostats")
                .arg(partial);
            let pass2_progress: Arc<dyn Fn(f64, &str) + Send + Sync> = {
                let progress = Arc::clone(progress);
                Arc::new(move |fraction, stage| progress(0.45 + fraction * 0.55, stage))
            };
            let result = run_ffmpeg_progress(
                pass2,
                duration,
                Some(&self.cancel),
                &pass2_progress,
                "Encoding video",
            );
            let _ = std::fs::remove_dir_all(&tmp);
            result.map_err(|e| {
                let _ = std::fs::remove_file(partial);
                e
            })?;
            Ok(())
        } else {
            // CRF presets.
            let (crf, height, audio_kbps) = match preset {
                "smallest" => (30, 720, 64),
                "original" => (20, media.height.max(1080), 160),
                _ => (23, 1080, 128),
            };
            let crf_filter = self.size_filter_for_height(height);
            let mut command = Command::new(self.ffmpeg()?);
            common(&mut command);
            command
                .arg("-vf")
                .arg(&crf_filter)
                .arg("-c:v")
                .arg("libx264")
                .arg("-preset")
                .arg("medium")
                .arg("-crf")
                .arg(crf.to_string())
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg(format!("{audio_kbps}k"))
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-movflags")
                .arg("+faststart")
                .arg("-progress")
                .arg("pipe:1")
                .arg("-nostats")
                .arg(partial);
            run_ffmpeg_progress(
                command,
                duration,
                Some(&self.cancel),
                progress,
                "Encoding video",
            )
            .map_err(|e| {
                let _ = std::fs::remove_file(partial);
                e
            })?;
            Ok(())
        }
    }

    fn size_filter_for_height(&self, height: i64) -> String {
        // The full filter chain depends on edits; recompute with the scale
        // height. This is called only after export() stored its edits.
        (*self.last_edits.lock()).as_ref()
            .map(|edits| build_filter(edits, height))
            .unwrap_or_else(|| build_filter(&EditSpec::default(), height))
    }

    fn finish_export(
        &self,
        partial: PathBuf,
        output: PathBuf,
        duration: f64,
        preset: &str,
        used_encoder: String,
        hardware_used: bool,
        progress: &Arc<dyn Fn(f64, &str) + Send + Sync>,
    ) -> Result<ExportResult, MediaError> {
        if !partial.is_file() || media_path_size(&partial) == 0 {
            let _ = std::fs::remove_file(&partial);
            return Err(MediaError::Message(
                "The export completed without producing a usable file.".into(),
            ));
        }
        std::fs::rename(&partial, &output).map_err(|e| MediaError::Message(format!("{e}")))?;
        progress(1.0, "Export ready");
        Ok(ExportResult {
            size_bytes: media_path_size(&output),
            path: output,
            duration,
            generated: true,
            preset: preset.to_string(),
            encoder: used_encoder,
            hardware_accelerated: hardware_used,
        })
    }
}

struct HardwareProfile {
    video_kbps: i64,
    audio_kbps: i64,
    height: i64,
}

fn hardware_profile(
    media: &crate::db::MediaRow,
    preset: &str,
    duration: f64,
    target_mb: f64,
) -> HardwareProfile {
    // Size-targeted presets compute the bitrate from the size budget first
    // (mirrors the original's `_hardware_profile`).
    if matches!(preset, "fit_bot" | "fit_x" | "fit_both" | "custom") && target_mb > 0.0 {
        let audio_kbps = if target_mb < 100.0 { 96 } else { 128 };
        let total_kbps = target_mb * 1024.0 * 1024.0 * 8.0 * 0.955 / duration.max(0.05) / 1000.0;
        let video_kbps = (total_kbps - audio_kbps as f64).max(120.0) as i64;
        let height = if video_kbps >= 1800 {
            1080
        } else if video_kbps >= 700 {
            720
        } else {
            480
        };
        return HardwareProfile {
            video_kbps,
            audio_kbps,
            height,
        };
    }
    let source_duration = media.duration.max(0.05).max(duration);
    let source_kbps = ((media.size_bytes as f64 * 8.0) / source_duration / 1000.0 - 128.0)
        .max(500.0) as i64;
    match preset {
        "smallest" => HardwareProfile {
            video_kbps: ((source_kbps as f64 * 0.42) as i64).clamp(350, 1600),
            audio_kbps: 64,
            height: 720,
        },
        "original" => HardwareProfile {
            video_kbps: ((source_kbps as f64 * 1.02) as i64).clamp(1200, 24000),
            audio_kbps: 160,
            height: media.height.max(1080),
        },
        _ => HardwareProfile {
            video_kbps: ((source_kbps as f64 * 0.72) as i64).clamp(700, 6500),
            audio_kbps: 128,
            height: 1080,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_spec_normalization() {
        // Empty spec.
        let spec = normalize_edit_spec(&serde_json::json!({}));
        assert!(spec.crop.is_none());
        assert!(spec.overlays.is_empty());
        // Disabled crop is dropped.
        let spec = normalize_edit_spec(&serde_json::json!({"crop": {"enabled": false}}));
        assert!(spec.crop.is_none());
        // Full-frame crop is dropped.
        let spec = normalize_edit_spec(&serde_json::json!({"crop": {"enabled": true, "x": 0, "y": 0, "width": 1, "height": 1}}));
        assert!(spec.crop.is_none());
        // Partial crop kept + clamped.
        let spec = normalize_edit_spec(&serde_json::json!({
            "crop": {"x": 0.25, "y": 0.5, "width": 2.0, "height": 0.4}
        }));
        let crop = spec.crop.unwrap();
        assert_eq!(crop.x, 0.25);
        assert_eq!(crop.y, 0.5);
        assert!((crop.width - 0.75).abs() < 1e-9);
        assert!((crop.height - 0.4).abs() < 1e-9);
        // Overlays always kept, capped at 32.
        let overlays: Vec<serde_json::Value> = (0..40)
            .map(|_| serde_json::json!({"x": 0, "y": 0, "width": 0.2, "height": 0.2}))
            .collect();
        let spec = normalize_edit_spec(&serde_json::json!({"overlays": overlays}));
        assert_eq!(spec.overlays.len(), 32);
    }

    #[test]
    fn filter_chain_shape() {
        let edits = EditSpec {
            crop: Some(CropSpec { x: 0.0, y: 0.0, width: 0.5, height: 1.0 }),
            overlays: vec![OverlaySpec { x: 0.1, y: 0.1, width: 0.2, height: 0.2 }],
        };
        let filter = build_filter(&edits, 720);
        assert!(filter.starts_with("crop=w="));
        assert!(filter.contains("drawbox=x='iw*0.10000000'"));
        assert!(filter.ends_with("scale=w=-2:h='min(ih,720)':force_original_aspect_ratio=decrease"));
        assert!(!filter.contains(' '));
    }

    #[test]
    fn candidate_rules() {
        let indexer = MediaIndexer {
            database: Arc::new(crate::db::Database::open(
                tempfile::tempdir().unwrap().path().join("t.sqlite3"),
            )
            .unwrap()),
            export_dir: PathBuf::from("/tmp/exports"),
        };
        // Hidden files are rejected.
        assert!(!indexer._is_candidate(Path::new("/a/.hidden.mp4"), false));
        // Known extension accepted.
        assert!(indexer._is_candidate(Path::new("/a/clip.MP4"), false));
        // Unknown extension rejected unless deep scan.
        assert!(!indexer._is_candidate(Path::new("/a/clip.xyz"), false));
        assert!(indexer._is_candidate(Path::new("/a/clip.xyz"), true));
        // TS extension requires sniffing (no file -> false).
        assert!(!indexer._is_candidate(Path::new("/a/clip.ts"), false));
    }

    #[test]
    fn hardware_profiles_are_sane() {
        let media = crate::db::MediaRow {
            id: 1,
            root_path: "/r".into(),
            path: "/r/a.mp4".into(),
            name: "a.mp4".into(),
            relative_path: "a.mp4".into(),
            folder: String::new(),
            duration: 60.0,
            width: 1920,
            height: 1080,
            size_bytes: 60_000_000,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            frame_rate: 30.0,
            mtime: 0.0,
            thumbnail_path: None,
            preview_path: None,
            timeline_path: None,
            active: true,
            valid: true,
            seen: false,
            posted_count: 0,
            indexed_at: String::new(),
            updated_at: String::new(),
        };
        let smallest = hardware_profile(&media, "smallest", 60.0, 0.0);
        assert!(smallest.video_kbps >= 350);
        assert!(smallest.video_kbps <= 1600);
        let original = hardware_profile(&media, "original", 60.0, 0.0);
        assert!(original.video_kbps >= 1200);
        assert!(original.video_kbps <= 24000);
        let balanced = hardware_profile(&media, "balanced", 60.0, 0.0);
        assert!(balanced.video_kbps >= 700);
        assert!(balanced.video_kbps <= 6500);
        // Size-targeted preset computes the bitrate from the budget.
        let fit = hardware_profile(&media, "custom", 60.0, 50.0);
        let budget_kbps = 50.0 * 1024.0 * 1024.0 * 8.0 * 0.955 / 60.0 / 1000.0;
        assert!((fit.video_kbps as f64 - (budget_kbps - 96.0)).abs() < 2.0);
        assert!(fit.video_kbps >= 120);
    }
}
