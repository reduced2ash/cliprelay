from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

from platformdirs import user_cache_dir, user_config_dir, user_data_dir, user_log_dir


APP_NAME = "ClipRelay"
APP_SLUG = "cliprelay"


def config_dir() -> Path:
    path = Path(user_config_dir(APP_NAME, appauthor=False))
    path.mkdir(parents=True, exist_ok=True)
    return path


def data_dir() -> Path:
    path = Path(user_data_dir(APP_NAME, appauthor=False))
    path.mkdir(parents=True, exist_ok=True)
    return path


def cache_dir() -> Path:
    path = Path(user_cache_dir(APP_NAME, appauthor=False))
    path.mkdir(parents=True, exist_ok=True)
    return path


def log_dir() -> Path:
    path = Path(user_log_dir(APP_NAME, appauthor=False))
    path.mkdir(parents=True, exist_ok=True)
    return path


def default_export_dir() -> Path:
    base = Path.home() / ("Movies" if sys.platform == "darwin" else "Videos")
    path = base / APP_NAME / "Exports"
    path.mkdir(parents=True, exist_ok=True)
    return path


def database_path() -> Path:
    return data_dir() / "cliprelay.sqlite3"


def thumbnail_dir() -> Path:
    path = cache_dir() / "thumbnails"
    path.mkdir(parents=True, exist_ok=True)
    return path


def preview_dir() -> Path:
    path = cache_dir() / "previews"
    path.mkdir(parents=True, exist_ok=True)
    return path


def bundled_binary(name: str) -> Path | None:
    executable = f"{name}.exe" if sys.platform == "win32" else name
    candidates: list[Path] = []
    bundle_root = getattr(sys, "_MEIPASS", None)
    if bundle_root:
        candidates.append(Path(bundle_root) / "bin" / executable)
    candidates.append(Path(__file__).resolve().parent / "bin" / executable)
    custom_dir = os.environ.get("CLIPRELAY_FFMPEG_DIR")
    if custom_dir:
        candidates.insert(0, Path(custom_dir) / executable)
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    resolved = shutil.which(name)
    return Path(resolved) if resolved else None


def ffmpeg_path() -> Path | None:
    return bundled_binary("ffmpeg")


def ffprobe_path() -> Path | None:
    return bundled_binary("ffprobe")


def is_within(path: str | Path, parent: str | Path) -> bool:
    try:
        Path(path).resolve().relative_to(Path(parent).resolve())
        return True
    except (OSError, ValueError):
        return False
