from __future__ import annotations

import asyncio
import json
import math
import os
import subprocess
import sys
import tempfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, Iterable

from .database import Database
from .paths import (
    ffmpeg_path,
    ffprobe_path,
    is_within,
    preview_dir,
    thumbnail_dir,
    timeline_dir,
)
from .utils import clamp, media_cache_key, safe_stem


VIDEO_EXTENSIONS = {
    ".3g2", ".3gp", ".asf", ".avi", ".divx", ".dv", ".f4v", ".flv", ".h264",
    ".hevc", ".m2t", ".m2ts", ".m4v", ".mkv", ".mov", ".mp4", ".mpeg", ".mpg",
    ".mts", ".mxf", ".ogm", ".ogv", ".qt", ".rm", ".rmvb", ".ts", ".vob", ".webm",
    ".wmv", ".y4m",
}

TRANSPORT_STREAM_EXTENSIONS = {".m2t", ".m2ts", ".mts", ".ts"}
TRANSPORT_STREAM_PACKET_SIZES = (188, 192, 204)
TRANSPORT_STREAM_SAMPLE_BYTES = 8192

IGNORED_DIR_NAMES = {
    ".git", ".svn", ".hg", "__macosx", "$recycle.bin", "system volume information",
}


class MediaError(RuntimeError):
    pass


class ProcessingCancelled(MediaError):
    pass


class ScanCancelled(MediaError):
    """Raised when a library scan is cooperatively stopped."""

    pass


def _looks_like_transport_stream(path: Path) -> bool:
    """Distinguish MPEG transport streams from source files such as TypeScript."""
    try:
        with path.open("rb") as handle:
            sample = handle.read(TRANSPORT_STREAM_SAMPLE_BYTES)
    except OSError:
        return False

    required_packets = 4
    for start, byte in enumerate(sample):
        if byte != 0x47:
            continue
        for packet_size in TRANSPORT_STREAM_PACKET_SIZES:
            positions = tuple(
                start + packet_size * index
                for index in range(required_packets)
            )
            if positions[-1] + 3 >= len(sample):
                continue
            if all(
                sample[position] == 0x47
                and sample[position + 3] & 0x30
                for position in positions
            ):
                return True
    return False


@dataclass(slots=True)
class ScanResult:
    discovered: int = 0
    indexed: int = 0
    skipped: int = 0
    failed: int = 0


@dataclass(slots=True)
class ExportResult:
    path: Path
    size_bytes: int
    duration: float
    generated: bool
    preset: str
    encoder: str = "stream copy"
    hardware_accelerated: bool = False


def normalize_edit_spec(value: Any) -> dict[str, Any]:
    """Return a bounded, FFmpeg-safe edit description using normalized coordinates."""
    source = value if isinstance(value, dict) else {}
    normalized: dict[str, Any] = {"crop": None, "overlays": []}

    def unit_number(raw: Any, default: float) -> float:
        try:
            number = float(raw)
        except (TypeError, ValueError):
            return default
        return number if math.isfinite(number) else default

    crop_value = source.get("crop")
    if isinstance(crop_value, dict) and bool(crop_value.get("enabled", True)):
        x = clamp(unit_number(crop_value.get("x"), 0), 0, 0.99)
        y = clamp(unit_number(crop_value.get("y"), 0), 0, 0.99)
        width = clamp(unit_number(crop_value.get("width"), 1), 0.01, 1 - x)
        height = clamp(unit_number(crop_value.get("height"), 1), 0.01, 1 - y)
        if x > 0.0001 or y > 0.0001 or width < 0.9999 or height < 0.9999:
            normalized["crop"] = {
                "x": x, "y": y, "width": width, "height": height,
            }

    overlay_values = source.get("overlays")
    if isinstance(overlay_values, (list, tuple)):
        for overlay in overlay_values[:32]:
            if not isinstance(overlay, dict):
                continue
            x = clamp(unit_number(overlay.get("x"), 0), 0, 0.99)
            y = clamp(unit_number(overlay.get("y"), 0), 0, 0.99)
            width = clamp(unit_number(overlay.get("width"), 0.2), 0.01, 1 - x)
            height = clamp(unit_number(overlay.get("height"), 0.2), 0.01, 1 - y)
            normalized["overlays"].append({
                "type": "rectangle",
                "x": x, "y": y, "width": width, "height": height,
            })
    return normalized


def _fraction(value: str | None) -> float:
    if not value:
        return 0.0
    try:
        if "/" in value:
            numerator, denominator = value.split("/", 1)
            return float(numerator) / float(denominator) if float(denominator) else 0.0
        return float(value)
    except (TypeError, ValueError, ZeroDivisionError):
        return 0.0


def _rotation(stream: dict[str, Any]) -> int:
    tags = stream.get("tags") or {}
    try:
        if "rotate" in tags:
            return int(float(tags["rotate"])) % 360
    except (TypeError, ValueError):
        pass
    for item in stream.get("side_data_list") or []:
        try:
            if "rotation" in item:
                return int(float(item["rotation"])) % 360
        except (TypeError, ValueError):
            continue
    return 0


class MediaIndexer:
    def __init__(self, database: Database, export_dir: str | Path, workers: int = 4):
        self.database = database
        self.export_dir = Path(export_dir)
        self.workers = max(1, min(workers, 8))
        self.ffprobe = ffprobe_path()
        self.ffmpeg = ffmpeg_path()
        self._thumbnail_lock_guard = threading.Lock()
        self._thumbnail_locks: dict[str, threading.Lock] = {}
        self._scan_context = threading.local()

    @property
    def available(self) -> bool:
        return bool(self.ffprobe and self.ffmpeg)

    def _is_candidate(self, path: Path, deep_scan: bool) -> bool:
        if path.name.startswith("."):
            return False
        suffix = path.suffix.lower()
        if suffix in TRANSPORT_STREAM_EXTENSIONS:
            return deep_scan or _looks_like_transport_stream(path)
        return deep_scan or suffix in VIDEO_EXTENSIONS

    def iter_candidate_paths(
        self,
        root: Path,
        deep_scan: bool = False,
        cancel_event: threading.Event | None = None,
    ) -> Iterable[Path]:
        export_dir = self.export_dir.resolve()
        export_inside_root = is_within(export_dir, root)
        pending = [root]
        while pending:
            self._raise_if_scan_cancelled(cancel_event)
            current = pending.pop()
            try:
                with os.scandir(current) as entries:
                    for entry in entries:
                        self._raise_if_scan_cancelled(cancel_event)
                        path = Path(entry.path)
                        try:
                            if entry.is_dir(follow_symlinks=False):
                                if (
                                    not entry.name.startswith(".")
                                    and entry.name.lower() not in IGNORED_DIR_NAMES
                                    and not (export_inside_root and path == export_dir)
                                ):
                                    pending.append(path)
                                continue
                            if not self._is_candidate(path, deep_scan):
                                continue
                            yield path
                        except OSError:
                            continue
            except OSError:
                continue

    def iter_candidate_stats(
        self,
        root: Path,
        deep_scan: bool = False,
        cancel_event: threading.Event | None = None,
    ) -> Iterable[tuple[Path, os.stat_result]]:
        for path in self.iter_candidate_paths(root, deep_scan, cancel_event):
            try:
                yield path, path.stat()
            except OSError:
                continue

    def iter_candidates(
        self,
        root: Path,
        deep_scan: bool = False,
        cancel_event: threading.Event | None = None,
    ) -> Iterable[Path]:
        yield from self.iter_candidate_paths(root, deep_scan, cancel_event)

    def discover(
        self,
        root: Path,
        deep_scan: bool = False,
        cancel_event: threading.Event | None = None,
    ) -> list[Path]:
        return list(self.iter_candidates(root, deep_scan, cancel_event))

    @staticmethod
    def _raise_if_scan_cancelled(
        cancel_event: threading.Event | None,
    ) -> None:
        if cancel_event is not None and cancel_event.is_set():
            raise ScanCancelled("Library scan stopped.")

    def _current_scan_cancel_event(self) -> threading.Event | None:
        return getattr(self._scan_context, "cancel_event", None)

    def _with_scan_cancel_context(
        self,
        cancel_event: threading.Event | None,
        callback: Callable[..., Any],
        *args: Any,
    ) -> Any:
        previous = self._current_scan_cancel_event()
        self._scan_context.cancel_event = cancel_event
        try:
            self._raise_if_scan_cancelled(cancel_event)
            return callback(*args)
        finally:
            if previous is None:
                try:
                    del self._scan_context.cancel_event
                except AttributeError:
                    pass
            else:
                self._scan_context.cancel_event = previous

    def _run_scan_process(
        self,
        command: list[str],
        *,
        timeout: float,
        text: bool = False,
    ) -> subprocess.CompletedProcess[Any]:
        cancel_event = self._current_scan_cancel_event()
        if cancel_event is None:
            return subprocess.run(
                command,
                capture_output=True,
                text=text,
                timeout=timeout,
                check=False,
            )

        self._raise_if_scan_cancelled(cancel_event)
        process = subprocess.Popen(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=text,
        )
        deadline = time.monotonic() + timeout
        try:
            while True:
                self._raise_if_scan_cancelled(cancel_event)
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    process.kill()
                    process.communicate()
                    raise subprocess.TimeoutExpired(command, timeout)
                try:
                    stdout, stderr = process.communicate(
                        timeout=min(0.1, remaining)
                    )
                    return subprocess.CompletedProcess(
                        command,
                        process.returncode,
                        stdout,
                        stderr,
                    )
                except subprocess.TimeoutExpired:
                    continue
        except ScanCancelled:
            process.terminate()
            try:
                process.communicate(timeout=0.75)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate()
            raise

    def probe(self, path: Path, root: Path) -> dict[str, Any] | None:
        if not self.ffprobe:
            raise MediaError("FFprobe is not available. Install FFmpeg or set CLIPRELAY_FFMPEG_DIR.")
        try:
            stat = path.stat()
            command = [
                str(self.ffprobe), "-v", "error", "-print_format", "json",
                "-show_format", "-show_streams", str(path),
            ]
            completed = self._run_scan_process(
                command,
                text=True,
                timeout=35,
            )
            if completed.returncode != 0:
                return None
            payload = json.loads(completed.stdout or "{}")
        except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError):
            return None
        video_stream = next(
            (stream for stream in payload.get("streams", []) if stream.get("codec_type") == "video"),
            None,
        )
        if not video_stream:
            return None
        audio_stream = next(
            (stream for stream in payload.get("streams", []) if stream.get("codec_type") == "audio"),
            None,
        )
        format_data = payload.get("format") or {}
        duration = float(format_data.get("duration") or video_stream.get("duration") or 0)
        if not math.isfinite(duration) or duration <= 0:
            return None
        width = int(video_stream.get("width") or 0)
        height = int(video_stream.get("height") or 0)
        if _rotation(video_stream) in {90, 270}:
            width, height = height, width
        relative = path.relative_to(root)
        folder = "" if relative.parent == Path(".") else relative.parent.as_posix()
        return {
            "root_path": str(root),
            "path": str(path),
            "name": path.name,
            "relative_path": relative.as_posix(),
            "folder": folder,
            "duration": duration,
            "width": width,
            "height": height,
            "size_bytes": stat.st_size,
            "video_codec": str(video_stream.get("codec_name") or ""),
            "audio_codec": str((audio_stream or {}).get("codec_name") or ""),
            "frame_rate": _fraction(video_stream.get("avg_frame_rate") or video_stream.get("r_frame_rate")),
            "mtime": stat.st_mtime,
        }

    def minimal_metadata(self, path: Path, root: Path) -> dict[str, Any] | None:
        try:
            stat = path.stat()
        except (OSError, ValueError):
            return None
        return self._metadata_from_stat(path, root, stat)

    def _metadata_from_stat(
        self,
        path: Path,
        root: Path,
        stat: os.stat_result,
    ) -> dict[str, Any] | None:
        try:
            relative = path.relative_to(root)
        except ValueError:
            return None
        folder = "" if relative.parent == Path(".") else relative.parent.as_posix()
        return {
            "root_path": str(root),
            "path": str(path),
            "name": path.name,
            "relative_path": relative.as_posix(),
            "folder": folder,
            "duration": 0,
            "width": 0,
            "height": 0,
            "size_bytes": stat.st_size,
            "video_codec": "",
            "audio_codec": "",
            "frame_rate": 0,
            "mtime": stat.st_mtime,
        }

    def refresh_manifest(
        self,
        root_path: str | Path,
        batch_ready: Callable[[int], None] | None = None,
        batch_size: int = 256,
        progress: Callable[[int, str], None] | None = None,
        check_changes: bool = True,
        cancel_event: threading.Event | None = None,
    ) -> ScanResult:
        root = Path(root_path).expanduser().resolve()
        if not root.is_dir():
            raise MediaError("The selected library folder is unavailable.")
        result = ScanResult()
        present_paths: list[str] = []
        batch: list[dict[str, Any]] = []
        first_committed = False
        cached_rows = self.database.media_state_map(str(root))
        last_progress = 0.0

        def flush() -> None:
            self._raise_if_scan_cancelled(cancel_event)
            if not batch:
                return
            nonlocal first_committed
            added = self.database.upsert_manifest_batch(batch)
            result.indexed += added
            first_committed = True
            if batch_ready and added:
                batch_ready(added)
            batch.clear()

        for path in self.iter_candidate_paths(root, False, cancel_event):
            self._raise_if_scan_cancelled(cancel_event)
            result.discovered += 1
            now = time.monotonic()
            if progress and (
                result.discovered == 1 or now - last_progress >= 0.25
            ):
                progress(result.discovered, path.name)
                last_progress = now
            cached = cached_rows.get(str(path))
            if cached and not check_changes:
                metadata = {
                    "root_path": str(root),
                    "path": str(path),
                    "name": str(cached["name"]),
                    "relative_path": str(cached["relative_path"]),
                    "folder": str(cached["folder"]),
                    "size_bytes": int(cached["size_bytes"]),
                    "mtime": float(cached["mtime"]),
                }
            else:
                try:
                    metadata = self._metadata_from_stat(path, root, path.stat())
                except OSError:
                    metadata = None
            if not metadata:
                result.failed += 1
                continue
            present_paths.append(str(path))
            batch.append(metadata)
            if not first_committed or len(batch) >= max(1, batch_size):
                flush()
        flush()
        self._raise_if_scan_cancelled(cancel_event)
        result.skipped = self.database.invalidate_absent(str(root), present_paths)
        if progress:
            progress(result.discovered, "")
        return result

    def fast_random(
        self,
        root_path: str | Path,
        avoid_seen: bool = True,
    ) -> dict[str, Any] | None:
        """Compatibility helper backed by the persistent filename manifest."""
        root = Path(root_path).expanduser().resolve()
        if not root.is_dir():
            raise MediaError("The selected library folder is unavailable.")
        if not self.database.manifest_paths(str(root)):
            self.refresh_manifest(root)
        while row := self.database.random_media(avoid_seen):
            metadata = self.ensure_metadata(int(row["id"]))
            if metadata:
                return metadata
        return None

    def ensure_metadata(self, media_id: int) -> dict[str, Any] | None:
        media = self.database.get_media(media_id)
        if not media:
            return None
        path = Path(media["path"])
        root = Path(media["root_path"])
        try:
            stat = path.stat()
        except OSError:
            self.database.set_media_valid(media_id, False)
            return None
        if not self.database.media_needs_probe(
            str(path), stat.st_size, stat.st_mtime, require_probe=True
        ):
            return media
        metadata = self.probe(path, root)
        if not metadata:
            self.database.set_media_valid(media_id, False)
            return None
        refreshed_id = self.database.upsert_media(metadata)
        return self.database.get_media(refreshed_id)

    def scan(
        self,
        root_path: str | Path,
        deep_scan: bool = False,
        progress: Callable[[int, int, str], None] | None = None,
        *,
        verify_media: bool = True,
        generate_thumbnails: bool = True,
        item_ready: Callable[[int], None] | None = None,
        candidates: Iterable[Path] | None = None,
        cancel_event: threading.Event | None = None,
    ) -> ScanResult:
        root = Path(root_path).expanduser().resolve()
        if not root.is_dir():
            raise MediaError("The selected library folder is unavailable.")
        trusted_manifest = candidates is not None
        if candidates is not None:
            candidate_paths = []
            for candidate in candidates:
                self._raise_if_scan_cancelled(cancel_event)
                candidate_paths.append(candidate)
        else:
            candidate_paths = self.discover(
                root,
                deep_scan and verify_media,
                cancel_event,
            )
        result = ScanResult(discovered=len(candidate_paths))
        valid_paths: list[str] = []
        to_probe: list[Path] = []
        cached_thumbnail_ids: list[int] = []
        state_map = self.database.media_state_map(str(root))
        state_by_id = {
            int(state["id"]): state
            for state in state_map.values()
        }
        for path in candidate_paths:
            self._raise_if_scan_cancelled(cancel_event)
            cached = state_map.get(str(path))
            if trusted_manifest and cached:
                needs_probe = (
                    not int(cached["valid"])
                    or (verify_media and float(cached["duration"]) <= 0)
                )
            else:
                try:
                    stat = path.stat()
                except OSError:
                    result.failed += 1
                    continue
                needs_probe = self.database.media_needs_probe(
                    str(path), stat.st_size, stat.st_mtime, require_probe=verify_media
                )
            if needs_probe:
                to_probe.append(path)
            else:
                result.skipped += 1
                valid_paths.append(str(path))
                if (
                    generate_thumbnails
                    and cached
                    and not cached.get("thumbnail_path")
                ):
                    cached_thumbnail_ids.append(int(cached["id"]))
        total = len(to_probe) + len(cached_thumbnail_ids)
        completed_count = 0
        if to_probe and not verify_media:
            for path in to_probe:
                self._raise_if_scan_cancelled(cancel_event)
                completed_count += 1
                metadata = self.minimal_metadata(path, root)
                if metadata:
                    media_id = self.database.upsert_media(metadata)
                    result.indexed += 1
                    valid_paths.append(str(path))
                    if item_ready:
                        item_ready(media_id)
                    if generate_thumbnails:
                        self._with_scan_cancel_context(
                            cancel_event,
                            self.ensure_thumbnail,
                            media_id,
                            metadata,
                        )
                        if item_ready:
                            item_ready(media_id)
                else:
                    result.failed += 1
                if progress:
                    progress(completed_count, total, path.name)
        elif to_probe:
            pool = ThreadPoolExecutor(
                max_workers=self.workers,
                thread_name_prefix="media-probe",
            )
            paths = iter(to_probe)
            futures: dict[Any, Path] = {}

            def submit_probe(path: Path) -> None:
                future = pool.submit(
                    self._with_scan_cancel_context,
                    cancel_event,
                    self.probe,
                    path,
                    root,
                )
                futures[future] = path

            for _ in range(min(len(to_probe), self.workers * 2)):
                submit_probe(next(paths))
            try:
                while futures:
                    self._raise_if_scan_cancelled(cancel_event)
                    future = next(as_completed(tuple(futures)))
                    path = futures.pop(future)
                    completed_count += 1
                    try:
                        metadata = future.result()
                        if metadata:
                            media_id = self.database.upsert_media(metadata)
                            result.indexed += 1
                            valid_paths.append(str(path))
                            if item_ready:
                                item_ready(media_id)
                            if generate_thumbnails:
                                self._with_scan_cancel_context(
                                    cancel_event,
                                    self.ensure_thumbnail,
                                    media_id,
                                    metadata,
                                )
                                if item_ready:
                                    item_ready(media_id)
                        else:
                            result.failed += 1
                    except ScanCancelled:
                        raise
                    except Exception:
                        result.failed += 1
                    if progress:
                        progress(completed_count, total, path.name)
                    next_path = next(paths, None)
                    if next_path is not None:
                        submit_probe(next_path)
            finally:
                if cancel_event is not None and cancel_event.is_set():
                    for future in futures:
                        future.cancel()
                pool.shutdown(
                    wait=True,
                    cancel_futures=bool(
                        cancel_event is not None and cancel_event.is_set()
                    ),
                )
        if cached_thumbnail_ids:
            thumbnail_workers = min(self.workers, 4)
            pool = ThreadPoolExecutor(
                max_workers=thumbnail_workers,
                thread_name_prefix="media-thumbnail",
            )
            media_ids = iter(cached_thumbnail_ids)
            futures: dict[Any, int] = {}

            def submit_thumbnail(media_id: int) -> None:
                future = pool.submit(
                    self._with_scan_cancel_context,
                    cancel_event,
                    self.ensure_thumbnail,
                    media_id,
                    state_by_id.get(media_id),
                )
                futures[future] = media_id

            for _ in range(
                min(len(cached_thumbnail_ids), thumbnail_workers * 2)
            ):
                submit_thumbnail(next(media_ids))
            try:
                while futures:
                    self._raise_if_scan_cancelled(cancel_event)
                    future = next(as_completed(tuple(futures)))
                    media_id = futures.pop(future)
                    completed_count += 1
                    try:
                        thumbnail = future.result()
                    except ScanCancelled:
                        raise
                    except Exception:
                        thumbnail = None
                    media = self.database.get_media(media_id)
                    if thumbnail and item_ready:
                        item_ready(media_id)
                    if progress:
                        progress(
                            completed_count,
                            total,
                            str((media or {}).get("name") or "thumbnail"),
                        )
                    next_media_id = next(media_ids, None)
                    if next_media_id is not None:
                        submit_thumbnail(next_media_id)
            finally:
                if cancel_event is not None and cancel_event.is_set():
                    for future in futures:
                        future.cancel()
                pool.shutdown(
                    wait=True,
                    cancel_futures=bool(
                        cancel_event is not None and cancel_event.is_set()
                    ),
                )
        self._raise_if_scan_cancelled(cancel_event)
        self.database.invalidate_missing(str(root), valid_paths)
        return result

    def ensure_thumbnail(self, media_id: int, metadata: dict[str, Any] | None = None) -> Path | None:
        media = metadata or self.database.get_media(media_id)
        if not media or not self.ffmpeg:
            return None
        key = media_cache_key(media["path"], int(media["size_bytes"]), float(media["mtime"]))
        output = thumbnail_dir() / f"{key}.jpg"
        with self._thumbnail_lock_guard:
            thumbnail_lock = self._thumbnail_locks.setdefault(
                key,
                threading.Lock(),
            )
        with thumbnail_lock:
            if output.is_file() and output.stat().st_size > 0:
                self.database.set_media_asset(media_id, "thumbnail_path", str(output))
                return output
            timestamp = clamp(
                float(media["duration"]) * 0.15,
                0.0,
                max(0.0, float(media["duration"]) - 0.1),
            )
            command = [
                str(self.ffmpeg), "-hide_banner", "-loglevel", "error", "-y",
                "-ss", f"{timestamp:.3f}", "-i", str(media["path"]), "-frames:v", "1",
                "-vf", "scale=640:360:force_original_aspect_ratio=decrease",
                "-q:v", "3", str(output),
            ]
            try:
                completed = self._run_scan_process(
                    command,
                    timeout=60,
                )
            except ScanCancelled:
                output.unlink(missing_ok=True)
                raise
            if (
                completed.returncode == 0
                and output.is_file()
                and output.stat().st_size > 0
            ):
                self.database.set_media_asset(media_id, "thumbnail_path", str(output))
                return output
            output.unlink(missing_ok=True)
            return None

    def ensure_preview(self, media_id: int) -> Path | None:
        media = self.ensure_metadata(media_id)
        if not media or not self.ffmpeg:
            return None
        existing = media.get("preview_path")
        if existing and Path(existing).is_file():
            return Path(existing)
        key = media_cache_key(media["path"], int(media["size_bytes"]), float(media["mtime"]))
        output = preview_dir() / f"{key}.mp4"
        if output.is_file() and output.stat().st_size > 0:
            self.database.set_media_asset(media_id, "preview_path", str(output))
            return output
        start = clamp(float(media["duration"]) * 0.12, 0.0, max(0.0, float(media["duration"]) - 0.1))
        preview_length = min(8.0, max(1.0, float(media["duration"]) - start))
        partial = output.with_suffix(".partial.mp4")
        command = [
            str(self.ffmpeg), "-hide_banner", "-loglevel", "error", "-y",
            "-ss", f"{start:.3f}", "-i", str(media["path"]), "-t", f"{preview_length:.3f}",
            "-vf", "scale=640:360:force_original_aspect_ratio=decrease",
            "-an", "-c:v", "libx264", "-preset", "veryfast", "-crf", "29",
            "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(partial),
        ]
        completed = subprocess.run(command, capture_output=True, timeout=120, check=False)
        if completed.returncode == 0 and partial.is_file():
            partial.replace(output)
            self.database.set_media_asset(media_id, "preview_path", str(output))
            return output
        partial.unlink(missing_ok=True)
        return None

    def ensure_timeline(self, media_id: int, frame_count: int = 12) -> Path | None:
        media = self.ensure_metadata(media_id)
        if not media or not self.ffmpeg:
            return None
        existing = media.get("timeline_path")
        if existing and Path(existing).is_file():
            return Path(existing)
        key = media_cache_key(
            media["path"],
            int(media["size_bytes"]),
            float(media["mtime"]),
        )
        output = timeline_dir() / f"{key}.jpg"
        lock_key = f"timeline:{key}"
        with self._thumbnail_lock_guard:
            timeline_lock = self._thumbnail_locks.setdefault(
                lock_key,
                threading.Lock(),
            )
        with timeline_lock:
            if output.is_file() and output.stat().st_size > 0:
                self.database.set_media_asset(
                    media_id,
                    "timeline_path",
                    str(output),
                )
                return output
            duration = max(0.1, float(media.get("duration") or 0.1))
            frames = max(4, min(int(frame_count), 20))
            partial = output.with_name(f"{output.stem}.partial.jpg")
            frame_pattern = output.with_name(
                f"{output.stem}.partial-%02d.jpg"
            )
            frame_paths = [
                output.with_name(f"{output.stem}.partial-{index:02d}.jpg")
                for index in range(frames)
            ]
            scale_filter = (
                "scale=160:90:force_original_aspect_ratio=increase,"
                "crop=160:90"
            )
            try:
                for index, frame_path in enumerate(frame_paths):
                    frame_path.unlink(missing_ok=True)
                    timestamp = duration * (index + 0.5) / frames
                    completed = subprocess.run(
                        [
                            str(self.ffmpeg),
                            "-hide_banner",
                            "-loglevel",
                            "error",
                            "-y",
                            "-ss",
                            f"{timestamp:.6f}",
                            "-i",
                            str(media["path"]),
                            "-frames:v",
                            "1",
                            "-vf",
                            scale_filter,
                            "-q:v",
                            "4",
                            str(frame_path),
                        ],
                        capture_output=True,
                        timeout=20,
                        check=False,
                    )
                    if (
                        completed.returncode != 0
                        or not frame_path.is_file()
                        or frame_path.stat().st_size <= 0
                    ):
                        return None
                completed = subprocess.run(
                    [
                        str(self.ffmpeg),
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-y",
                        "-framerate",
                        "1",
                        "-start_number",
                        "0",
                        "-i",
                        str(frame_pattern),
                        "-frames:v",
                        "1",
                        "-vf",
                        f"tile={frames}x1:padding=0:margin=0",
                        "-q:v",
                        "4",
                        str(partial),
                    ],
                    capture_output=True,
                    timeout=30,
                    check=False,
                )
                if (
                    completed.returncode == 0
                    and partial.is_file()
                    and partial.stat().st_size > 0
                ):
                    partial.replace(output)
                    self.database.set_media_asset(
                        media_id,
                        "timeline_path",
                        str(output),
                    )
                    return output
                return None
            finally:
                partial.unlink(missing_ok=True)
                for frame_path in frame_paths:
                    frame_path.unlink(missing_ok=True)


class MediaProcessor:
    def __init__(self, export_dir: str | Path):
        self.export_dir = Path(export_dir)
        self.export_dir.mkdir(parents=True, exist_ok=True)
        self.ffmpeg = ffmpeg_path()
        self._active: set[asyncio.subprocess.Process] = set()
        self._cancel_requested = False
        self._encoder_mode = "software"
        self._hardware_encoder_checked = False
        self._hardware_encoder: str | None = None
        self._hardware_encoder_lock = threading.Lock()

    @property
    def encoder_mode(self) -> str:
        return self._encoder_mode

    def set_encoder_mode(self, mode: str) -> None:
        self._encoder_mode = (
            str(mode)
            if str(mode) in {"hardware", "software"}
            else "software"
        )

    @staticmethod
    def _hardware_encoder_candidates() -> tuple[str, ...]:
        if sys.platform == "darwin":
            return ("h264_videotoolbox",)
        if sys.platform == "win32":
            # Probe the device, not just FFmpeg's compiled encoder list. A
            # bundled FFmpeg can expose all four while only one is usable.
            return ("h264_nvenc", "h264_qsv", "h264_amf")
        return ()

    @staticmethod
    def _encoder_label(encoder: str | None) -> str:
        return {
            "h264_videotoolbox": "VideoToolbox H.264",
            "h264_nvenc": "NVIDIA NVENC H.264",
            "h264_qsv": "Intel Quick Sync H.264",
            "h264_amf": "AMD AMF H.264",
            "libx264": "libx264",
        }.get(str(encoder or ""), "Unavailable")

    def hardware_encoder_info(self) -> dict[str, Any]:
        encoder = self._detect_hardware_encoder()
        return {
            "available": bool(encoder),
            "encoder": encoder or "",
            "label": self._encoder_label(encoder),
        }

    def _detect_hardware_encoder(self) -> str | None:
        if self._hardware_encoder_checked:
            return self._hardware_encoder
        with self._hardware_encoder_lock:
            if self._hardware_encoder_checked:
                return self._hardware_encoder
            selected: str | None = None
            if self.ffmpeg:
                for encoder in self._hardware_encoder_candidates():
                    command = [
                        str(self.ffmpeg),
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-f",
                        "lavfi",
                        "-i",
                        "color=c=black:s=64x64:r=1:d=0.1",
                        "-frames:v",
                        "1",
                        "-an",
                        "-c:v",
                        encoder,
                        "-pix_fmt",
                        "yuv420p",
                        "-f",
                        "null",
                        "-",
                    ]
                    try:
                        completed = subprocess.run(
                            command,
                            capture_output=True,
                            timeout=12,
                            check=False,
                        )
                    except (OSError, subprocess.TimeoutExpired):
                        continue
                    if completed.returncode == 0:
                        selected = encoder
                        break
            self._hardware_encoder = selected
            self._hardware_encoder_checked = True
            return selected

    def set_export_dir(self, export_dir: str | Path) -> None:
        self.export_dir = Path(export_dir).expanduser().resolve()
        self.export_dir.mkdir(parents=True, exist_ok=True)

    def cancel(self) -> None:
        self._cancel_requested = True
        for process in list(self._active):
            try:
                process.terminate()
            except ProcessLookupError:
                pass

    async def _run_ffmpeg(
        self,
        command: list[str],
        duration: float,
        progress: Callable[[float, str], None] | None,
    ) -> None:
        if self._cancel_requested:
            raise ProcessingCancelled("Video processing was cancelled.")
        process = await asyncio.create_subprocess_exec(
            *command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        self._active.add(process)
        stderr_chunks: list[bytes] = []

        async def read_stderr() -> None:
            assert process.stderr
            while chunk := await process.stderr.read(4096):
                stderr_chunks.append(chunk)
                if sum(map(len, stderr_chunks)) > 16000:
                    stderr_chunks.pop(0)

        stderr_task = asyncio.create_task(read_stderr())
        try:
            assert process.stdout
            while line := await process.stdout.readline():
                text = line.decode("utf-8", "replace").strip()
                if text.startswith("out_time_ms=") and duration > 0 and progress:
                    try:
                        micros = float(text.split("=", 1)[1])
                        progress(clamp(micros / 1_000_000 / duration, 0, 1), "Encoding video")
                    except ValueError:
                        pass
                if self._cancel_requested:
                    process.terminate()
                    raise ProcessingCancelled("Video processing was cancelled.")
            return_code = await process.wait()
            await stderr_task
            if self._cancel_requested:
                raise ProcessingCancelled("Video processing was cancelled.")
            if return_code != 0:
                detail = b"".join(stderr_chunks).decode("utf-8", "replace").strip()
                raise MediaError(detail[-2000:] or "FFmpeg could not process this video.")
        except ProcessingCancelled:
            if process.returncode is None:
                process.terminate()
                await process.wait()
            raise
        finally:
            self._active.discard(process)
            if not stderr_task.done():
                stderr_task.cancel()

    def _filter(self, height: int, edits: Any = None) -> str:
        edit_spec = normalize_edit_spec(edits)
        filters: list[str] = []
        crop = edit_spec["crop"]
        if crop:
            filters.append(
                "crop="
                f"w='max(2,trunc(iw*{crop['width']:.8f}/2)*2)':"
                f"h='max(2,trunc(ih*{crop['height']:.8f}/2)*2)':"
                f"x='min(iw-ow,max(0,trunc(iw*{crop['x']:.8f}/2)*2))':"
                f"y='min(ih-oh,max(0,trunc(ih*{crop['y']:.8f}/2)*2))'"
            )
        for overlay in edit_spec["overlays"]:
            filters.append(
                "drawbox="
                f"x='iw*{overlay['x']:.8f}':"
                f"y='ih*{overlay['y']:.8f}':"
                f"w='iw*{overlay['width']:.8f}':"
                f"h='ih*{overlay['height']:.8f}':"
                "color=black:t=fill"
            )
        filters.append(
            f"scale=w=-2:h='min(ih,{height})':force_original_aspect_ratio=decrease"
        )
        return ",".join(filters)

    @staticmethod
    def _hardware_profile(
        media: dict[str, Any],
        preset: str,
        duration: float,
        target_mb: float,
    ) -> tuple[int, int, int]:
        if preset in {"fit_bot", "fit_x", "fit_both", "custom"} and target_mb > 0:
            audio_kbps = 96 if target_mb < 100 else 128
            total_kbps = target_mb * 1024 * 1024 * 8 * 0.955 / duration / 1000
            video_kbps = max(120, int(total_kbps - audio_kbps))
            height = 1080 if video_kbps >= 1800 else 720 if video_kbps >= 700 else 480
            return video_kbps, height, audio_kbps

        source_duration = max(0.05, float(media.get("duration") or duration))
        source_kbps = max(
            500,
            int(float(media.get("size_bytes") or 0) * 8 / source_duration / 1000) - 128,
        )
        if preset == "smallest":
            return max(350, min(1600, int(source_kbps * 0.42))), 720, 64
        if preset == "original":
            return (
                max(1200, min(24000, int(source_kbps * 1.02))),
                max(1080, int(media.get("height") or 1080)),
                160,
            )
        return max(700, min(6500, int(source_kbps * 0.72))), 1080, 128

    async def _export_hardware(
        self,
        common: list[str],
        partial: Path,
        media: dict[str, Any],
        duration: float,
        preset: str,
        target_mb: float,
        edit_spec: dict[str, Any],
        encoder: str,
        progress_args: list[str],
        progress: Callable[[float, str], None] | None,
    ) -> None:
        video_kbps, height, audio_kbps = self._hardware_profile(
            media,
            preset,
            duration,
            target_mb,
        )
        label = self._encoder_label(encoder)
        command = [
            *common,
            "-vf",
            self._filter(height, edit_spec),
            "-c:v",
            encoder,
            "-b:v",
            f"{video_kbps}k",
            "-maxrate",
            f"{int(video_kbps * 1.15)}k",
            "-bufsize",
            f"{int(video_kbps * 2)}k",
            "-c:a",
            "aac",
            "-b:a",
            f"{audio_kbps}k",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
            *progress_args,
            str(partial),
        ]
        if progress:
            progress(0.01, f"Encoding with {label}")
        await self._run_ffmpeg(
            command,
            duration,
            (
                lambda value, _stage: progress(
                    value,
                    f"Encoding with {label}",
                )
                if progress
                else None
            ),
        )
        if (
            target_mb > 0
            and partial.is_file()
            and partial.stat().st_size > target_mb * 1024 * 1024 * 1.015
        ):
            raise MediaError(
                "The hardware pass exceeded the requested file limit."
            )

    async def export(
        self,
        media: dict[str, Any],
        trim_start: float,
        trim_end: float,
        preset: str,
        target_mb: float,
        progress: Callable[[float, str], None] | None = None,
        edits: Any = None,
    ) -> ExportResult:
        if not self.ffmpeg:
            raise MediaError("FFmpeg is not available. Install FFmpeg or set CLIPRELAY_FFMPEG_DIR.")
        self._cancel_requested = False
        source = Path(media["path"])
        source_duration = float(media["duration"])
        start = clamp(trim_start, 0, max(0, source_duration - 0.05))
        end = clamp(trim_end or source_duration, start + 0.05, source_duration)
        duration = end - start
        no_trim = start <= 0.01 and abs(end - source_duration) <= 0.05
        edit_spec = normalize_edit_spec(edits)
        has_edits = bool(edit_spec["crop"] or edit_spec["overlays"])
        compatible = (
            str(media.get("video_codec", "")).lower() in {"h264", "avc1"}
            and str(media.get("audio_codec", "")).lower() in {"", "aac"}
            and source.suffix.lower() == ".mp4"
        )
        target_passthrough = (
            preset in {"fit_bot", "fit_x", "fit_both", "custom"}
            and target_mb > 0
            and source.stat().st_size <= target_mb * 1024 * 1024 * 0.98
        )
        if no_trim and not has_edits and compatible and (preset == "original" or target_passthrough):
            if progress:
                progress(1.0, "Using original video")
            return ExportResult(source, source.stat().st_size, duration, False, preset)

        timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        name_suffix = "edited" if has_edits else "prepared"
        output = self.export_dir / f"{safe_stem(source.name)}_{name_suffix}_{timestamp}.mp4"
        counter = 2
        while output.exists():
            output = self.export_dir / (
                f"{safe_stem(source.name)}_{name_suffix}_{timestamp}-{counter}.mp4"
            )
            counter += 1
        partial = output.with_suffix(".partial.mp4")
        common = [
            str(self.ffmpeg), "-hide_banner", "-y", "-i", str(source),
            "-ss", f"{start:.3f}", "-t", f"{duration:.3f}",
        ]
        progress_args = ["-progress", "pipe:1", "-nostats"]

        try:
            hardware_encoder: str | None = None
            hardware_used = False
            if self._encoder_mode == "hardware":
                hardware_encoder = await asyncio.to_thread(
                    self._detect_hardware_encoder
                )
                if hardware_encoder:
                    try:
                        await self._export_hardware(
                            common,
                            partial,
                            media,
                            duration,
                            preset,
                            target_mb,
                            edit_spec,
                            hardware_encoder,
                            progress_args,
                            progress,
                        )
                        hardware_used = True
                    except ProcessingCancelled:
                        raise
                    except MediaError:
                        partial.unlink(missing_ok=True)
                        if progress:
                            progress(
                                0.01,
                                "Hardware export unavailable · retrying with software",
                            )
                else:
                    if progress:
                        progress(
                            0.01,
                            "No compatible hardware encoder · using software",
                        )

            if not hardware_used:
                if preset in {"fit_bot", "fit_x", "fit_both", "custom"} and target_mb > 0:
                    audio_kbps = 96 if target_mb < 100 else 128
                    total_kbps = target_mb * 1024 * 1024 * 8 * 0.965 / duration / 1000
                    video_kbps = max(120, int(total_kbps - audio_kbps))
                    height = 1080 if video_kbps >= 1800 else 720 if video_kbps >= 700 else 480
                    with tempfile.TemporaryDirectory(prefix="cliprelay-pass-") as temporary:
                        passlog = str(Path(temporary) / "passlog")
                        null_output = "NUL" if sys.platform == "win32" else "/dev/null"
                        first = [
                            *common, "-vf", self._filter(height, edit_spec), "-c:v", "libx264", "-preset", "medium",
                            "-b:v", f"{video_kbps}k", "-maxrate", f"{int(video_kbps * 1.15)}k",
                            "-bufsize", f"{int(video_kbps * 2)}k", "-pass", "1",
                            "-passlogfile", passlog, "-an", *progress_args, "-f", "null", null_output,
                        ]
                        if progress:
                            progress(0.01, "Measuring target size")
                        await self._run_ffmpeg(
                            first,
                            duration,
                            lambda p, s: progress(p * 0.45, s) if progress else None,
                        )
                        second = [
                            *common, "-vf", self._filter(height, edit_spec), "-c:v", "libx264", "-preset", "medium",
                            "-b:v", f"{video_kbps}k", "-maxrate", f"{int(video_kbps * 1.15)}k",
                            "-bufsize", f"{int(video_kbps * 2)}k", "-pass", "2",
                            "-passlogfile", passlog, "-c:a", "aac", "-b:a", f"{audio_kbps}k",
                            "-pix_fmt", "yuv420p", "-movflags", "+faststart", *progress_args, str(partial),
                        ]
                        await self._run_ffmpeg(
                            second,
                            duration,
                            lambda p, s: progress(0.45 + p * 0.55, s) if progress else None,
                        )
                else:
                    if preset == "smallest":
                        crf, height, audio = 30, 720, 64
                    elif preset == "original":
                        crf, height, audio = (
                            20,
                            max(1080, int(media.get("height") or 1080)),
                            160,
                        )
                    else:
                        crf, height, audio = 23, 1080, 128
                    command = [
                        *common, "-vf", self._filter(height, edit_spec), "-c:v", "libx264", "-preset", "medium",
                        "-crf", str(crf), "-c:a", "aac", "-b:a", f"{audio}k",
                        "-pix_fmt", "yuv420p", "-movflags", "+faststart", *progress_args, str(partial),
                    ]
                    await self._run_ffmpeg(command, duration, progress)
            if not partial.is_file() or partial.stat().st_size == 0:
                raise MediaError("The export completed without producing a usable file.")
            partial.replace(output)
            if progress:
                progress(1.0, "Export ready")
            used_encoder = (
                hardware_encoder
                if hardware_used and hardware_encoder
                else "libx264"
            )
            return ExportResult(
                output,
                output.stat().st_size,
                duration,
                True,
                preset,
                self._encoder_label(used_encoder),
                hardware_used,
            )
        except Exception:
            partial.unlink(missing_ok=True)
            raise
