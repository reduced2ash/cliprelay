from __future__ import annotations

from pathlib import Path


QML_DIR = Path(__file__).resolve().parents[1] / "src" / "cliprelay" / "qml"


def test_prepare_video_autoplays_and_loops() -> None:
    source = (QML_DIR / "PrepareVideoEditor.qml").read_text(encoding="utf-8")

    assert "autoPlay: true" in source
    assert "loops: MediaPlayer.Infinite" in source
    assert "Qt.callLater(root.startAutoplay)" in source
    assert "onPositionChanged: function(position)" in source
    assert "previewPlayer.position = root.trimStart * 1000" in source
    assert "mediaStatus === MediaPlayer.EndOfMedia" in source
