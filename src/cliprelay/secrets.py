from __future__ import annotations

import json
import os
from pathlib import Path

import keyring

from .paths import APP_NAME, config_dir


class SecretStore:
    """Uses the OS keychain, with a permission-restricted local fallback."""

    def __init__(self, base_dir: str | Path | None = None):
        target = Path(base_dir) if base_dir else config_dir()
        target.mkdir(parents=True, exist_ok=True)
        self.fallback_path = target / "secrets.json"
        self.backend = "system keychain"

    def _read_fallback(self) -> dict[str, str]:
        try:
            return json.loads(self.fallback_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return {}

    def _write_fallback(self, values: dict[str, str]) -> None:
        self.fallback_path.write_text(json.dumps(values, ensure_ascii=False), encoding="utf-8")
        try:
            os.chmod(self.fallback_path, 0o600)
        except OSError:
            pass

    def get(self, key: str, default: str = "") -> str:
        try:
            value = keyring.get_password(APP_NAME, key)
            if value is not None:
                return value
        except Exception:
            self.backend = "restricted local file"
        return self._read_fallback().get(key, default)

    def set(self, key: str, value: str) -> None:
        try:
            keyring.set_password(APP_NAME, key, value)
            return
        except Exception:
            self.backend = "restricted local file"
        values = self._read_fallback()
        if value:
            values[key] = value
        else:
            values.pop(key, None)
        self._write_fallback(values)

    def delete(self, key: str) -> None:
        try:
            keyring.delete_password(APP_NAME, key)
        except Exception:
            pass
        values = self._read_fallback()
        values.pop(key, None)
        self._write_fallback(values)
