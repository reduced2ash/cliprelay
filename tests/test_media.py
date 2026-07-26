from __future__ import annotations

import asyncio
import shutil
import subprocess
from pathlib import Path

import pytest

from cliprelay.database import Database
from cliprelay.media import MediaIndexer, MediaProcessor, normalize_edit_spec


pytestmark = pytest.mark.skipif(
    not shutil.which("ffmpeg") or not shutil.which("ffprobe"),
    reason="FFmpeg is required for media integration tests",
)


def make_video(path: Path, duration: float = 2.0) -> None:
    subprocess.run(
        [
            shutil.which("ffmpeg") or "ffmpeg",
            "-hide_banner", "-loglevel", "error", "-y",
            "-f", "lavfi", "-i", "testsrc2=size=480x270:rate=24",
            "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100",
            "-t", str(duration), "-c:v", "libx264", "-pix_fmt", "yuv420p",
            "-c:a", "aac", str(path),
        ],
        check=True,
        timeout=40,
    )


def test_recursive_index_thumbnail_preview_and_cache(tmp_path: Path) -> None:
    library = tmp_path / "library"
    nested = library / "folder one" / "deeper"
    nested.mkdir(parents=True)
    video = nested / "clip sample.mp4"
    make_video(video)
    (nested / "notes.txt").write_text("not media", encoding="utf-8")
    database = Database(tmp_path / "db.sqlite3")
    indexer = MediaIndexer(database, tmp_path / "exports", workers=2)

    first = indexer.scan(library)
    assert first.discovered == 1
    assert first.indexed == 1
    row = database.list_media()[0]
    assert row["folder"] == "folder one/deeper"
    assert row["width"] == 480 and row["height"] == 270
    assert Path(row["thumbnail_path"]).is_file()
    assert indexer.ensure_preview(row["id"]).is_file()

    second = indexer.scan(library)
    assert second.skipped == 1
    assert second.indexed == 0

    video.write_bytes(b"no longer a video")
    third = indexer.scan(library)
    assert third.failed == 1
    assert database.list_media() == []


def test_fast_random_checks_only_the_selected_candidate(monkeypatch, tmp_path: Path) -> None:
    library = tmp_path / "library"
    nested = library / "nested"
    nested.mkdir(parents=True)
    videos = []
    for index in range(24):
        video = nested / f"clip-{index:02}.mp4"
        video.write_bytes(b"video")
        videos.append(video)
    (nested / "notes.txt").write_text("not a video", encoding="utf-8")
    database = Database(tmp_path / "db.sqlite3")
    database.activate_root(library)
    indexer = MediaIndexer(database, tmp_path / "exports")
    probes: list[Path] = []

    def fake_probe(path: Path, root: Path) -> dict:
        probes.append(path)
        metadata = indexer.minimal_metadata(path, root)
        assert metadata
        metadata.update(duration=3, width=640, height=360, video_codec="h264")
        return metadata

    monkeypatch.setattr(indexer, "probe", fake_probe)
    picked = indexer.fast_random(library, avoid_seen=True)

    assert picked and Path(picked["path"]) in videos
    assert len(probes) == 1
    assert database.counts()["media"] == 24
    assert sum(float(row["duration"]) > 0 for row in database.list_media()) == 1
    assert database.counts()["unseen"] == 23


def test_fast_random_recycles_seen_clips_after_unreadable_remainder(
    monkeypatch, tmp_path: Path
) -> None:
    library = tmp_path / "library"
    library.mkdir()
    good = library / "good.mp4"
    broken = library / "broken.mp4"
    good.write_bytes(b"good")
    broken.write_bytes(b"broken")
    database = Database(tmp_path / "db.sqlite3")
    database.activate_root(library)
    indexer = MediaIndexer(database, tmp_path / "exports")
    good_metadata = indexer.minimal_metadata(good, library)
    assert good_metadata
    good_metadata.update(duration=2, width=640, height=360, video_codec="h264")
    good_id = database.upsert_media(good_metadata)
    database.mark_seen(good_id)
    indexer.refresh_manifest(library)
    probes: list[Path] = []

    def fake_probe(path: Path, _root: Path) -> dict | None:
        probes.append(path)
        return good_metadata if path == good else None

    monkeypatch.setattr(indexer, "probe", fake_probe)
    picked = indexer.fast_random(library, avoid_seen=True)

    assert picked and Path(picked["path"]) == good
    assert probes == [broken]


def test_lightweight_scan_skips_probe_thumbnail_and_nonvideo(monkeypatch, tmp_path: Path) -> None:
    library = tmp_path / "library"
    nested = library / "nested"
    nested.mkdir(parents=True)
    (nested / "clip.mp4").write_bytes(b"not decoded")
    (nested / "notes.txt").write_text("plain text", encoding="utf-8")
    database = Database(tmp_path / "db.sqlite3")
    database.activate_root(library)
    indexer = MediaIndexer(database, tmp_path / "exports")
    monkeypatch.setattr(
        indexer,
        "probe",
        lambda *_: pytest.fail("lightweight indexing must not run FFprobe"),
    )
    monkeypatch.setattr(
        indexer,
        "ensure_thumbnail",
        lambda *_: pytest.fail("thumbnail generation was explicitly disabled"),
    )
    ready: list[int] = []

    result = indexer.scan(
        library,
        deep_scan=True,
        verify_media=False,
        generate_thumbnails=False,
        item_ready=ready.append,
    )

    assert result.discovered == 1
    assert result.indexed == 1
    assert len(ready) == 1
    row = database.list_media()[0]
    assert row["duration"] == 0
    assert row["width"] == 0


def test_fast_manifest_rejects_typescript_but_keeps_transport_streams(
    tmp_path: Path,
) -> None:
    library = tmp_path / "library"
    library.mkdir()
    transport_stream = library / "camera.ts"
    make_video(transport_stream, 0.5)
    transport_module = library / "camera.mts"
    shutil.copyfile(transport_stream, transport_module)
    typescript = library / "application.ts"
    typescript.write_text(
        "export const selectedVideo = 'not media';\n",
        encoding="utf-8",
    )
    typescript_module = library / "application.mts"
    typescript_module.write_text(
        "export default function loadLibrary() { return []; }\n",
        encoding="utf-8",
    )
    database = Database(tmp_path / "db.sqlite3")
    database.activate_root(library)
    indexer = MediaIndexer(database, tmp_path / "exports")

    stale_metadata = indexer.minimal_metadata(typescript, library)
    assert stale_metadata
    database.upsert_media(stale_metadata)

    result = indexer.refresh_manifest(library)

    assert result.discovered == 2
    assert {
        Path(row["path"]).name
        for row in database.list_media()
    } == {"camera.ts", "camera.mts"}
    assert database.get_media_by_path(typescript)["valid"] == 0
    assert database.get_media_by_path(typescript_module) is None


def test_manifest_is_batched_persistent_and_preserves_verified_metadata(
    monkeypatch, tmp_path: Path
) -> None:
    library = tmp_path / "library"
    nested = library / "nested"
    nested.mkdir(parents=True)
    first = nested / "first.mp4"
    first.write_bytes(b"first")
    for index in range(55):
        (nested / f"clip-{index:02}.mov").write_bytes(b"video")
    (nested / "notes.txt").write_text("ignore me", encoding="utf-8")
    database = Database(tmp_path / "db.sqlite3")
    database.activate_root(library)
    indexer = MediaIndexer(database, tmp_path / "exports")
    verified = indexer.minimal_metadata(first, library)
    assert verified
    verified.update(duration=8, width=1280, height=720, video_codec="h264")
    database.upsert_media(verified)
    monkeypatch.setattr(
        indexer,
        "probe",
        lambda *_: pytest.fail("the filename manifest must not probe videos"),
    )
    batches: list[int] = []

    result = indexer.refresh_manifest(library, batches.append, batch_size=16)

    assert result.discovered == 56
    assert result.indexed == 55
    assert sum(batches) == 55
    assert batches[0] == 1
    assert len(batches) == 5
    assert database.get_media_by_path(first)["duration"] == 8
    assert len(database.manifest_paths(library)) == 56

    unchanged_batches: list[int] = []
    progress_updates: list[tuple[int, str]] = []
    monkeypatch.setattr(
        indexer,
        "_metadata_from_stat",
        lambda *_: pytest.fail("fast reconciliation must reuse cached file metadata"),
    )
    unchanged = indexer.refresh_manifest(
        library,
        unchanged_batches.append,
        batch_size=16,
        progress=lambda count, name: progress_updates.append((count, name)),
        check_changes=False,
    )
    assert unchanged.indexed == 0
    assert unchanged_batches == []
    assert progress_updates[0][0] == 1
    assert progress_updates[-1] == (56, "")

    monkeypatch.undo()

    first.write_bytes(b"changed-size")
    removed = nested / "clip-00.mov"
    removed.unlink()
    indexer.refresh_manifest(library)
    assert database.get_media_by_path(first)["duration"] == 0
    assert database.get_media_by_path(removed)["valid"] == 0
    assert len(database.manifest_paths(library)) == 55

    removed.write_bytes(b"video")
    indexer.refresh_manifest(library, check_changes=False)
    assert database.get_media_by_path(removed)["valid"] == 1


@pytest.mark.asyncio
async def test_trim_compression_target_and_original_passthrough(tmp_path: Path) -> None:
    library = tmp_path / "library"
    library.mkdir()
    video = library / "source.mp4"
    make_video(video, 4.0)
    database = Database(tmp_path / "db.sqlite3")
    indexer = MediaIndexer(database, tmp_path / "exports")
    indexer.scan(library)
    media = database.list_media()[0]
    processor = MediaProcessor(tmp_path / "exports")
    progress: list[float] = []

    trimmed = await processor.export(media, 0.5, 2.25, "balanced", 0, lambda p, _: progress.append(p))
    assert trimmed.generated
    assert trimmed.path.is_file()
    assert trimmed.duration == pytest.approx(1.75, abs=0.02)
    assert progress[-1] == 1.0

    targeted = await processor.export(media, 0, 4, "custom", 0.25)
    assert targeted.path.stat().st_size < 500_000

    original = await processor.export(media, 0, 4, "original", 0)
    assert original.generated is False
    assert original.path == video
    assert video.is_file()

    already_within_limit = await processor.export(media, 0, 4, "fit_both", 49)
    assert already_within_limit.generated is False
    assert already_within_limit.path == video


def test_edit_spec_is_bounded_and_limited() -> None:
    edits = normalize_edit_spec({
        "crop": {"enabled": True, "x": -1, "y": 0.8, "width": 4, "height": 4},
        "overlays": [
            {"x": 0.95, "y": -2, "width": 1, "height": 0},
        ] * 40,
    })

    assert edits["crop"] == {
        "x": 0,
        "y": 0.8,
        "width": 1,
        "height": pytest.approx(0.2),
    }
    assert len(edits["overlays"]) == 32
    assert edits["overlays"][0]["x"] == pytest.approx(0.95)
    assert edits["overlays"][0]["width"] == pytest.approx(0.05)
    assert edits["overlays"][0]["height"] == pytest.approx(0.01)


@pytest.mark.asyncio
async def test_crop_and_black_rectangle_render_to_generated_copy(tmp_path: Path) -> None:
    library = tmp_path / "library"
    library.mkdir()
    video = library / "source.mp4"
    make_video(video, 2.0)
    original_size = video.stat().st_size
    database = Database(tmp_path / "db.sqlite3")
    indexer = MediaIndexer(database, tmp_path / "exports")
    indexer.scan(library)
    media = database.list_media()[0]
    processor = MediaProcessor(tmp_path / "exports")

    result = await processor.export(
        media, 0, 2, "original", 0,
        edits={
            "crop": {
                "enabled": True,
                "x": 0.21875,
                "y": 0,
                "width": 0.5625,
                "height": 1,
            },
            "overlays": [{"x": 0, "y": 0, "width": 0.25, "height": 0.25}],
        },
    )

    assert result.generated
    assert result.path.parent == tmp_path / "exports"
    assert "_edited_" in result.path.name
    assert result.path != video
    assert video.stat().st_size == original_size

    probe = subprocess.run(
        [
            shutil.which("ffprobe") or "ffprobe", "-v", "error",
            "-select_streams", "v:0", "-show_entries", "stream=width,height",
            "-of", "csv=p=0:s=x", str(result.path),
        ],
        capture_output=True, text=True, check=True, timeout=20,
    )
    assert probe.stdout.strip() == "270x270"

    black_sample = subprocess.run(
        [
            shutil.which("ffmpeg") or "ffmpeg", "-hide_banner", "-loglevel", "error",
            "-ss", "0.5", "-i", str(result.path),
            "-vf", "crop=8:8:8:8,format=rgb24", "-frames:v", "1",
            "-f", "rawvideo", "pipe:1",
        ],
        capture_output=True, check=True, timeout=20,
    )
    assert black_sample.stdout
    assert max(black_sample.stdout) < 24
