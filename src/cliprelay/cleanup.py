from __future__ import annotations

from pathlib import Path

from send2trash import send2trash

from .paths import is_within


class CleanupError(RuntimeError):
    pass


def move_generated_to_trash(path: str | Path, export_dir: str | Path, is_generated: bool) -> None:
    target = Path(path).resolve()
    if not is_generated:
        raise CleanupError("Source videos are protected and cannot be cleaned up by ClipRelay.")
    if not is_within(target, Path(export_dir).resolve()):
        raise CleanupError("Only files inside the configured exports folder can be moved to Trash.")
    if not target.is_file():
        raise CleanupError("The generated file is no longer available.")
    send2trash(str(target))
