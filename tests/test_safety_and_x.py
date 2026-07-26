from __future__ import annotations

from pathlib import Path

import pytest

from cliprelay.cleanup import CleanupError, move_generated_to_trash
from cliprelay.x_assist import XAssistant


def test_x_intent_encodes_caption() -> None:
    url = bytes(XAssistant().intent_url("hello & a café #clip").toEncoded()).decode()
    assert url.startswith("https://x.com/intent/tweet?text=")
    assert "%26" in url
    assert "%23clip" in url
    assert "caf%C3%A9" in url


def test_cleanup_rejects_source_and_outside_file(tmp_path: Path) -> None:
    exports = tmp_path / "exports"
    exports.mkdir()
    source = tmp_path / "library" / "source.mp4"
    source.parent.mkdir()
    source.write_bytes(b"source")
    generated = exports / "generated.mp4"
    generated.write_bytes(b"generated")

    with pytest.raises(CleanupError, match="Source videos are protected"):
        move_generated_to_trash(source, exports, False)
    with pytest.raises(CleanupError, match="inside the configured exports"):
        move_generated_to_trash(source, exports, True)
    assert source.is_file()
    assert generated.is_file()


def test_cleanup_accepts_only_generated_file_inside_exports(monkeypatch, tmp_path: Path) -> None:
    from cliprelay import cleanup as cleanup_module

    exports = tmp_path / "exports"
    exports.mkdir()
    generated = exports / "cut.mp4"
    generated.write_bytes(b"generated")
    moved: list[str] = []
    monkeypatch.setattr(cleanup_module, "send2trash", moved.append)
    move_generated_to_trash(generated, exports, True)
    assert moved == [str(generated.resolve())]
