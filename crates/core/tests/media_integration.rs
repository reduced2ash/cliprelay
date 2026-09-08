//! End-to-end media pipeline tests using a real ffmpeg-generated video
//! (mirrors `test_media.py`). Skipped silently when ffmpeg is unavailable.

use cliprelay_core::db::Database;
use cliprelay_core::media::{
    ensure_playback_proxy, normalize_edit_spec, EditSpec, MediaIndexer, MediaProcessor, ScanResult,
};
use cliprelay_core::paths::ffmpeg_path;
use cliprelay_core::utils::media_cache_key;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

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
fn large_manifest_streams_the_first_batch_before_discovery_finishes() {
    let fixture = Fixture::new();
    let total = 5_000usize;
    for folder in 0..20 {
        let dir = fixture.root.join(format!("folder-{folder:02}"));
        std::fs::create_dir_all(&dir).unwrap();
        for file in 0..(total / 20) {
            std::fs::write(dir.join(format!("clip-{file:04}.mp4")), b"manifest-only").unwrap();
        }
    }

    let started = Instant::now();
    let mut first_batch = None;
    let mut callbacks = 0usize;
    let mut batch_ready = |_added: usize| {
        callbacks += 1;
        if first_batch.is_none() {
            first_batch = Some((fixture.db.media_count("", "").unwrap(), started.elapsed()));
        }
    };
    let progress: &mut dyn FnMut(usize, &str) = &mut |_discovered, _name| {};
    let result = fixture
        .indexer
        .refresh_manifest(
            &fixture.root,
            128,
            true,
            None,
            progress,
            Some(&mut batch_ready),
        )
        .unwrap();
    let total_elapsed = started.elapsed();
    let (first_count, first_elapsed) = first_batch.expect("first manifest callback");
    eprintln!(
        "manifest timing: first item in {:?}, all {total} in {:?}",
        first_elapsed, total_elapsed
    );

    assert_eq!(
        first_count, 1,
        "the first commit must not wait for the walk"
    );
    assert_eq!(result.discovered, total);
    assert_eq!(fixture.db.media_count("", "").unwrap(), total);
    assert!(callbacks > 2, "expected bounded incremental batches");
    assert!(first_elapsed < total_elapsed);
    assert!(
        first_elapsed.as_secs_f64() < 2.0,
        "first batch took {first_elapsed:?}"
    );
}

#[test]
fn cancelling_manifest_keeps_committed_items_and_stops_stale_work() {
    let fixture = Fixture::new();
    for file in 0..500 {
        std::fs::write(
            fixture.root.join(format!("clip-{file:04}.mp4")),
            b"candidate",
        )
        .unwrap();
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_from_callback = Arc::clone(&cancel);
    let mut batch_ready = move |_added: usize| {
        cancel_from_callback.store(true, Ordering::Relaxed);
    };
    let progress: &mut dyn FnMut(usize, &str) = &mut |_discovered, _name| {};
    let result = fixture.indexer.refresh_manifest(
        &fixture.root,
        128,
        true,
        Some(&cancel),
        progress,
        Some(&mut batch_ready),
    );

    assert!(matches!(
        result,
        Err(cliprelay_core::media::MediaError::ScanCancelled)
    ));
    assert_eq!(fixture.db.media_count("", "").unwrap(), 1);
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

    // The preview must produce a tightly packable NV12 width and even height.
    let preview = fixture.indexer.ensure_preview(media_id).expect("preview");
    assert!(preview.is_file());
    let dims = probe_dimensions(&preview);
    assert!(
        dims.0 % 4 == 0 && dims.1 % 2 == 0,
        "preview dims {dims:?} are not tightly packable"
    );

    std::fs::write(&preview, b"truncated preview cache").unwrap();
    let repaired = fixture
        .indexer
        .ensure_preview(media_id)
        .expect("corrupted preview should be regenerated");
    assert_eq!(repaired, preview);
    assert!(std::fs::metadata(&repaired).unwrap().len() > 100);
    assert!(probe_dimensions(&repaired).0 > 0);
}

#[test]
fn preview_resets_non_square_source_pixels() {
    let fixture = Fixture::new();
    let Some(ffmpeg) = ffmpeg_path() else {
        panic!("ffmpeg required for media integration tests");
    };
    let video = fixture.root.join("non-square-sar.mp4");
    let status = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=562x1024:rate=24",
            "-vf",
            "setsar=1408/1405",
            "-an",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&video)
        .status()
        .expect("create non-square-pixel video");
    assert!(status.success());

    fixture.refresh();
    let metadata = fixture
        .indexer
        .probe(&video, &fixture.root, None)
        .expect("probe");
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    let preview = fixture.indexer.ensure_preview(media_id).expect("preview");
    assert_eq!(probe_sample_aspect_ratio(&preview), "1:1");
}

#[test]
fn playback_proxy_is_full_length_tightly_packable_and_cached() {
    let fixture = Fixture::new();
    let video = make_test_video_at(&fixture.root, "prepare-source.mp4", 2, "586x232");

    let proxy = ensure_playback_proxy(&video, None).expect("playback proxy");
    assert!(proxy.is_file());
    assert_ne!(proxy, video);
    let dims = probe_dimensions(&proxy);
    assert_eq!(dims.0 % 4, 0, "proxy width must not require NV12 padding");
    assert_eq!(dims.1 % 2, 0, "proxy height must be even");

    let cached = ensure_playback_proxy(&video, None).expect("cached playback proxy");
    assert_eq!(cached, proxy);
}

#[test]
fn playback_proxy_encode_cancels_for_a_new_source() {
    let fixture = Fixture::new();
    let video = make_test_video_at(&fixture.root, "cancelled-prepare.mp4", 6, "1280x720");
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_from_switch = Arc::clone(&cancel);
    let switch = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));
        cancel_from_switch.store(true, Ordering::Release);
    });

    let started = Instant::now();
    let error = ensure_playback_proxy(&video, Some(&cancel))
        .expect_err("stale Prepare proxy should be cancelled");
    switch.join().unwrap();
    assert!(error.to_string().to_lowercase().contains("cancel"));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "stale proxy encode did not stop promptly"
    );

    cancel.store(false, Ordering::Release);
    let replacement =
        ensure_playback_proxy(&video, Some(&cancel)).expect("replacement proxy should proceed");
    assert!(replacement.is_file());
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

fn probe_sample_aspect_ratio(path: &Path) -> String {
    let Some(ffprobe) = cliprelay_core::paths::ffprobe_path() else {
        panic!("ffprobe required");
    };
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=sample_aspect_ratio",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .expect("run ffprobe");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
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
fn rotation_only_exports_a_rotated_copy_even_with_original_preset() {
    let fixture = Fixture::new();
    let video = fixture.video("rotation.mp4", 1);
    let metadata = fixture.indexer.probe(&video, &fixture.root, None).unwrap();
    let media_id = fixture.db.upsert_media(&metadata).unwrap();
    let row = fixture.db.get_media(media_id).unwrap().unwrap();
    let spec = normalize_edit_spec(&serde_json::json!({"rotation": 90}));
    let progress: ProgressCallback = Arc::new(|_, _| {});
    let result = fixture
        .processor
        .export(&row, 0.0, 0.0, "original", 0.0, &spec, &progress)
        .unwrap();
    assert!(result.generated);
    assert_ne!(result.path, video);
    let output = fixture
        .indexer
        .probe(&result.path, &fixture.export_dir, None)
        .unwrap();
    assert_eq!((output.width, output.height), (240, 320));
    assert!(video.is_file());
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
fn verified_scan_does_not_generate_thumbnails_when_disabled() {
    let fixture = Fixture::new();
    fixture.video("lazy.mp4", 2);
    fixture.refresh();
    let mut ready = 0usize;
    let mut item_ready = |_media_id: i64| ready += 1;
    let progress: &mut dyn FnMut(usize, usize, &str) = &mut |_done, _total, _name| {};
    let result = fixture
        .indexer
        .scan(
            &fixture.root,
            false,
            true,
            false,
            2,
            None,
            progress,
            Some(&mut item_ready),
        )
        .unwrap();
    assert_eq!(result.discovered, 1);
    assert_eq!(result.failed, 0);
    assert_eq!(
        ready, 1,
        "metadata should publish once without eager asset work"
    );
    let row = fixture
        .db
        .list_media("", "", "name", 1, 0)
        .unwrap()
        .remove(0);
    assert!(row.duration > 0.0);
    assert!(row.thumbnail_path.is_none());
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
