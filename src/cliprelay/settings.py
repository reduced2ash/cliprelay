from __future__ import annotations

from pathlib import Path
from typing import Any

from .database import Database
from .paths import default_export_dir


DEFAULTS: dict[str, Any] = {
    "library_root": "",
    "export_dir": str(default_export_dir()),
    "fast_random": True,
    "auto_index": False,
    "verify_during_index": True,
    "thumbnails_during_index": False,
    "hover_previews": True,
    "deep_scan": False,
    "avoid_repeats": True,
    "random_folder_mode": "all",
    "random_folders": [],
    "ui_scale": 1.0,
    "theme_mode": "relay",
    "performance_mode": "automatic",
    "library_density": "default",
    "export_encoder": "auto",
    "sidebar_collapsed": False,
    "prepare_expanded": False,
    "sort_mode": "newest",
    "cleanup_policy": "keep",
    "x_limit_mb": 512,
    "x_duration_seconds": 140,
    "telegram_mode": "bot",
    "telegram_destination": "",
    "telegram_api_id": "",
    "telegram_phone": "",
    "telegram_bot_configured": False,
    "telegram_personal_configured": False,
}


class Settings:
    def __init__(self, database: Database):
        self.database = database

    def get(self, key: str, default: Any = None) -> Any:
        fallback = DEFAULTS.get(key, default)
        return self.database.get_setting(key, fallback)

    def set(self, key: str, value: Any) -> None:
        if key == "theme_mode":
            value = str(value)
            if value not in {"relay", "pitch_black", "full_white"}:
                value = "relay"
        if key == "random_folder_mode":
            value = str(value)
            if value not in {"all", "selected"}:
                value = "all"
        if key == "performance_mode":
            value = str(value)
            if value not in {"automatic", "maximum"}:
                value = "automatic"
        if key == "library_density":
            value = str(value)
            if value not in {"default", "compact"}:
                value = "default"
        if key == "export_encoder":
            value = str(value)
            if value not in {"auto", "hardware", "software"}:
                value = "auto"
        if key in {"library_root", "export_dir"} and value:
            value = str(Path(str(value)).expanduser().resolve())
            if key == "export_dir":
                Path(value).mkdir(parents=True, exist_ok=True)
        self.database.set_setting(key, value)

    def as_dict(self) -> dict[str, Any]:
        stored = self.database.get_settings()
        return {key: stored.get(key, value) for key, value in DEFAULTS.items()}
