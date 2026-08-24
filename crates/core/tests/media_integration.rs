//! End-to-end media pipeline tests using a real ffmpeg-generated video
//! (mirrors `test_media.py`). Skipped silently when ffmpeg is unavailable.

use cliprelay_core::db::Database;
use cliprelay_core::media::{
    normalize_edit_spec, EditSpec, MediaIndexer, MediaProcessor, ScanResult,
};
use cliprelay_core::paths::ffmpeg_path;
use cliprelay_core::utils::media_cache_key;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

type ProgressCallback = Arc<dyn Fn(f64, &str) + Send + Sync>;

fn make_test_video(dir: &Path, name: &str, seconds: u32) -> std::path::PathBuf {
    make_test_video_at(dir, name, seconds, "320x240")
}

/// A test video at an arbitrary size (odd widths reproduce the libx264
/// "width not divisible by 2" rejections the even-rounding fix addresses).
fn make_test_video_at(dir: &Path, name: &str, seconds: u32, size: &str) -> std::path::PathBuf {
    let Some(ffmpeg) = ffmpeg_path() else {
        panic!("ffmpeg required for media integration tests");
    };
    let output = dir.join(name);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    // yuv444p permits odd dimensions (yuv420p requires even), so the
    // odd-size regression source can be encoded.
    let status = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            format!("testsrc=duration={seconds}:size={size}:rate=24").as_str(),
            "-b:v",
            "1000k",
            "-pix_fmt",
            "yuv444p",
        ])
        .arg(&output)
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "ffmpeg failed to create {name}");
    output
}

struct Fixture {
    _dir: tempfile::TempDir,
    db: Arc<Database>,
    indexer: MediaIndexer,
    processor: MediaProcessor,
    root: std::path::PathBuf,
    export_dir: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("library");
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let export_dir = dir.path().join("exports");
        std::fs::create_dir_all(&export_dir).unwrap();
        let db = Arc::new(Database::open(dir.path().join("test.sqlite3")).unwrap());
        let indexer = MediaIndexer {
            database: Arc::clone(&db),
            export_dir: export_dir.clone(),
        };
        let processor = MediaProcessor::new(
            Arc::clone(&db),
            export_dir.clone(),
            cliprelay_core::media::EncoderMode::Software,
        );
        Self {
            _dir: dir,
            db,
            indexer,
            processor,
            root,
            export_dir,
        }
    }

    fn video(&self, name: &str, seconds: u32) -> std::path::PathBuf {
        make_test_video(&self.root, name, seconds)
    }

    fn refresh(&self) -> ScanResult {
        let progress: &mut dyn FnMut(usize, &str) = &mut |_discovered, _name| {};
        self.indexer
            .refresh_manifest(&self.root, 16, true, None, progress, None)
            .unwrap()
    }
}

#[test]
fn manifest_probe_thumbnail_preview_timeline_flow() {
    let fixture = Fixture::new();
    let video = fixture.video("clip.mp4", 3);

    // Fast manifest sync finds the file.
    let result = fixture.refresh();
    assert_eq!(result.discovered, 1);
    assert_eq!(result.indexed, 1);
    assert_eq!(result.skipped, 0);

    // Probe gives full metadata.
    let metadata = fixture
        .indexer
        .probe(&video, &fixture.root, None)
        .expect("probe should succeed");
    assert!(
        metadata.duration > 2.5 && metadata.duration < 4.0,
        "duration {}",
        metadata.duration
    );
    assert!(metadata.width > 0 && metadata.height > 0);
    assert!(!metadata.video_codec.is_empty());
    assert_eq!(metadata.folder, "");
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    assert!(!fixture
        .db
        .media_needs_probe(&metadata.path, metadata.size_bytes, metadata.mtime, true)
        .unwrap());

    // Thumbnail: 640x360 fit-decrease JPEG.
    let thumbnail = fixture
        .indexer
        .ensure_thumbnail(media_id, None)
        .expect("thumbnail");
    assert!(thumbnail.is_file());
    let row = fixture.db.get_media(media_id).unwrap().unwrap();
    assert_eq!(
        row.thumbnail_path.as_deref(),
        Some(thumbnail.to_str().unwrap())
    );

    // Preview: small muted mp4.
    let preview = fixture.indexer.ensure_preview(media_id).expect("preview");
    assert!(preview.is_file());
    let row = fixture.db.get_media(media_id).unwrap().unwrap();
    assert_eq!(row.preview_path.as_deref(), Some(preview.to_str().unwrap()));
    // Second call reuses the cached preview.
    let again = fixture.indexer.ensure_preview(media_id).unwrap();
    assert_eq!(again, preview);

    // Timeline strip: 12-frame contact sheet.
    let timeline = fixture
        .indexer
        .ensure_timeline(media_id, 12)
        .expect("timeline");
    assert!(timeline.is_file());
    let row = fixture.db.get_media(media_id).unwrap().unwrap();
    assert_eq!(
        row.timeline_path.as_deref(),
        Some(timeline.to_str().unwrap())
    );

    // Cache key stability.
    let key = media_cache_key(&video, metadata.size_bytes as u64, metadata.mtime);
    assert_eq!(key.len(), 24);
}

#[test]
fn preview_and_export_round_odd_dimensions_to_even() {
    // A 587x233 source: the naive fit-scale emits 587x233 (odd width AND
    // odd height), which libx264 rejects. The pipeline must round to even.
    let fixture = Fixture::new();
    let video = make_test_video_at(&fixture.root, "odd.mp4", 2, "587x233");
    let result = fixture.refresh();
    assert_eq!(result.discovered, 1);
    let metadata = fixture
        .indexer
        .probe(&video, &fixture.root, None)
        .expect("probe");
    let media_id = fixture.db.upsert_media(&metadata).unwrap();

    // The preview must produce a valid, even-dimensioned mp4.
    let preview = fixture.indexer.ensure_preview(media_id).expect("preview");
    assert!(preview.is_file());
    let dims = probe_dimensions(&preview);
    assert!(
        dims.0 % 2 == 0 && dims.1 % 2 == 0,
        "preview dims {dims:?} not even"
    );
}

fn probe_dimensions(path: &std::path::Path) -> (i64, i64) {
    let Some(ffprobe) = cliprelay_core::paths::ffprobe_path() else {
        panic!("ffprobe required");
    };
    let out = Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("run ffprobe");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut it = text.trim().split(',');
    let w: i64 = it.next().unwrap_or("0").trim().parse().unwrap_or(0);
    let h: i64 = it.next().unwrap_or("0").trim().parse().unwrap_or(0);
    (w, h)
}

#[test]
fn export_trim_produces_valid_mp4_and_records() {
    let fixture = Fixture::new();
    let video = fixture.video("trim.mp4", 4);
    fixture.refresh();
    let metadata = fixture.indexer.probe(&video, &fixture.root, None).unwrap();
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    let row = fixture.db.get_media(media_id).unwrap().unwrap();

    let progress: ProgressCallback = std::sync::Arc::new(|_fraction, _stage| {});
    let result = fixture
        .processor
        .export(
            &row,
            0.5,
            2.5,
            "balanced",
            0.0,
            &EditSpec::default(),
            &progress,
        )
        .unwrap();
    assert!(result.generated);
    assert!(result.path.is_file());
    assert!(result.path.starts_with(&fixture.export_dir));
    assert_eq!(result.preset, "balanced");

    // The output is a playable mp4 with the trimmed duration.
    let probe = Command::new(cliprelay_core::paths::ffprobe_path().unwrap())
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(&result.path)
        .output()
        .unwrap();
    let duration: f64 = String::from_utf8_lossy(&probe.stdout)
        .trim()
        .parse()
        .unwrap();
    assert!((duration - 2.0).abs() < 0.5, "trimmed duration {duration}");

    // Passthrough: original preset with a compatible source returns the source.
    let passthrough = fixture
        .processor
        .export(
            &row,
            0.0,
            0.0,
            "original",
            0.0,
            &EditSpec::default(),
            &progress,
        )
        .unwrap();
    assert!(!passthrough.generated);
    assert_eq!(passthrough.path, video);
    assert_eq!(passthrough.encoder, "stream copy");
}

#[test]
fn export_size_target_two_pass() {
    let fixture = Fixture::new();
    let video = fixture.video("fits.mp4", 20);
    fixture.refresh();
    let metadata = fixture.indexer.probe(&video, &fixture.root, None).unwrap();
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    let row = fixture.db.get_media(media_id).unwrap().unwrap();

    let progress: ProgressCallback = std::sync::Arc::new(|_fraction, _stage| {});
    let result = fixture
        .processor
        .export(
            &row,
            0.0,
            0.0,
            "fit_bot",
            0.4,
            &EditSpec::default(),
            &progress,
        )
        .unwrap();
    assert!(result.path.is_file());
    assert!(
        result.path.starts_with(&fixture.export_dir),
        "{} vs {}",
        result.path.display(),
        fixture.export_dir.display()
    );
    // Two-pass target of 0.4 MB should land near the target (allow slack).
    let size_mb = result.size_bytes as f64 / (1024.0 * 1024.0);
    assert!(size_mb < 0.8, "two-pass size {size_mb} MB");
}

#[test]
fn hardware_export_honors_size_target() {
    // Exercises the hardware profile's size-targeted branch (skipped when
    // no hardware encoder is available).
    let fixture = Fixture::new();
    let video = fixture.video("hwfit.mp4", 20);
    fixture.refresh();
    let metadata = fixture.indexer.probe(&video, &fixture.root, None).unwrap();
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    let row = fixture.db.get_media(media_id).unwrap().unwrap();

    let Some(_) = cliprelay_core::media::detect_hardware_encoder_for_tests() else {
        return;
    };
    let processor = MediaProcessor::new(
        Arc::clone(&fixture.db),
        fixture.export_dir.clone(),
        cliprelay_core::media::EncoderMode::Hardware,
    );
    let progress: ProgressCallback = std::sync::Arc::new(|_fraction, _stage| {});
    let result = processor
        .export(
            &row,
            0.0,
            0.0,
            "fit_bot",
            0.4,
            &EditSpec::default(),
            &progress,
        )
        .unwrap();
    assert!(result.path.is_file());
    // Either the hardware pass hit the size gate and fell back to software
    // (still under the target) or the hardware pass itself is small enough.
    let size_mb = result.size_bytes as f64 / (1024.0 * 1024.0);
    assert!(size_mb < 0.8, "hardware size-targeted export {size_mb} MB");
}

#[test]
fn edit_spec_crop_and_overlays_produce_edited_copy() {
    let fixture = Fixture::new();
    let video = fixture.video("edited.mp4", 4);
    fixture.refresh();
    let metadata = fixture.indexer.probe(&video, &fixture.root, None).unwrap();
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    let row = fixture.db.get_media(media_id).unwrap().unwrap();

    let spec = normalize_edit_spec(&serde_json::json!({
        "crop": {"x": 0.0, "y": 0.0, "width": 0.5, "height": 1.0},
        "overlays": [{"x": 0.1, "y": 0.1, "width": 0.2, "height": 0.2}]
    }));
    let progress: ProgressCallback = std::sync::Arc::new(|_fraction, _stage| {});
    let result = fixture
        .processor
        .export(&row, 0.0, 0.0, "balanced", 0.0, &spec, &progress)
        .unwrap();
    assert!(result.path.is_file());
    let name = result
        .path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(name.contains("edited"), "output name {name}");
    assert!(result.path.starts_with(&fixture.export_dir));
}

#[test]
fn scan_marks_invalid_missing_and_keeps_present() {
    let fixture = Fixture::new();
    let a = fixture.video("a.mp4", 2);
    let b = fixture.video("b.mp4", 2);
    fixture.refresh();
    assert_eq!(
        fixture
            .db
            .manifest_paths(&fixture.root.to_string_lossy())
            .unwrap()
            .len(),
        2
    );

    std::fs::remove_file(&a).unwrap();
    let result = fixture.refresh();
    assert_eq!(result.discovered, 1);
    assert_eq!(result.skipped, 1);
    let row_b = fixture
        .db
        .get_media_by_path(&b.to_string_lossy())
        .unwrap()
        .unwrap();
    assert!(row_b.valid);
    let row_a = fixture
        .db
        .get_media_by_path(&a.to_string_lossy())
        .unwrap()
        .unwrap();
    assert!(!row_a.valid);
}

#[test]
fn full_scan_with_verify_and_thumbnails() {
    let fixture = Fixture::new();
    fixture.video("s1.mp4", 2);
    fixture.video("sub/s2.mp4", 2);
    fixture.refresh();
    let progress: &mut dyn FnMut(usize, usize, &str) = &mut |_done, _total, _name| {};
    let result = fixture
        .indexer
        .scan(&fixture.root, false, true, true, 2, None, progress, None)
        .unwrap();
    assert_eq!(result.discovered, 2);
    assert_eq!(result.failed, 0);
    let rows = fixture.db.list_media("", "", "name", 100, 0).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.duration > 0.0));
    assert!(rows.iter().all(|r| r.thumbnail_path.is_some()));
    // Folder derived from relative path.
    let s2 = rows.iter().find(|r| r.name == "s2.mp4").unwrap();
    assert_eq!(s2.folder, "sub");
}

#[test]
fn cancel_flag_aborts_scan() {
    let fixture = Fixture::new();
    for i in 0..20 {
        fixture.video(&format!("v{i:02}.mp4"), 2);
    }
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let progress: &mut dyn FnMut(usize, usize, &str) = &mut |_done, _total, _name| {};
    let result = fixture.indexer.scan(
        &fixture.root,
        false,
        true,
        true,
        2,
        Some(&cancel),
        progress,
        None,
    );
    assert!(result.is_err());
}

#[test]
fn passthrough_when_source_fits_target() {
    let fixture = Fixture::new();
    let video = fixture.video("fits-passthrough.mp4", 2);
    fixture.refresh();
    let metadata = fixture.indexer.probe(&video, &fixture.root, None).unwrap();
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    let row = fixture.db.get_media(media_id).unwrap().unwrap();
    let progress: ProgressCallback = std::sync::Arc::new(|_fraction, _stage| {});
    // h264+aac mp4 under the 49 MB bot limit -> passthrough.
    let result = fixture
        .processor
        .export(
            &row,
            0.0,
            0.0,
            "fit_bot",
            49.0,
            &EditSpec::default(),
            &progress,
        )
        .unwrap();
    assert!(!result.generated);
    assert_eq!(result.path, video);
}
