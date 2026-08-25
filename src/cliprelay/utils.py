from __future__ import annotations

import hashlib
import re
from datetime import datetime, timezone
from pathlib import Path


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def format_duration(seconds: float | int | None) -> str:
    value = max(0, int(float(seconds or 0)))
    hours, remainder = divmod(value, 3600)
    minutes, secs = divmod(remainder, 60)
    return f"{hours:d}:{minutes:02d}:{secs:02d}" if hours else f"{minutes:02d}:{secs:02d}"


def format_bytes(value: int | float | None) -> str:
    size = float(value or 0)
    units = ["B", "KB", "MB", "GB", "TB"]
    for unit in units:
        if size < 1024 or unit == units[-1]:
            return f"{size:.0f} {unit}" if unit == "B" else f"{size:.1f} {unit}"
        size /= 1024
    return "0 B"


def media_cache_key(path: str | Path, size: int, mtime: float) -> str:
    payload = f"{Path(path).resolve()}|{size}|{mtime:.6f}".encode("utf-8", "surrogatepass")
    return hashlib.sha256(payload).hexdigest()[:24]


def safe_stem(name: str, max_length: int = 72) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9._-]+", "_", Path(name).stem).strip("._-")
    return (cleaned or "clip")[:max_length]


def clamp(value: float, minimum: float, maximum: float) -> float:
    return max(minimum, min(maximum, value))
