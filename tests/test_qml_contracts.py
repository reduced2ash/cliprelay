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


def test_prepare_uses_one_filmstrip_timeline_for_seek_and_cut() -> None:
    editor = (QML_DIR / "PrepareVideoEditor.qml").read_text(encoding="utf-8")
    panel = (QML_DIR / "PreparePanel.qml").read_text(encoding="utf-8")
    timeline = (QML_DIR / "VideoTimeline.qml").read_text(encoding="utf-8")

    assert "VideoTimeline {" in editor
    assert "filmstripUrl: controller.selectedMedia.timelineUrl" in editor
    assert "filmstripLoading: controller.selectedMediaTimelineLoading" in editor
    assert "onTrimStartEdited: function(seconds)" in editor
    assert "AppSlider {" not in editor
    assert "AppRangeSlider {" not in panel

    assert "readonly property int frameCount: 12" in timeline
    assert "sourceClipRect: Qt.rect(" in timeline
    assert "signal seekRequested(real seconds)" in timeline
    assert "signal trimStartEdited(real seconds)" in timeline
    assert "signal trimEndEdited(real seconds)" in timeline
    assert "id: playhead" in timeline
    assert "id: inHandle" in timeline
    assert "id: outHandle" in timeline
    assert "CUT  " in timeline


def test_library_grid_height_tracks_responsive_tile_width() -> None:
    source = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")

    assert "readonly property real tilePosterHeight" in source
    assert "Math.ceil(tilePosterHeight + tileChromeHeight)" in source
    assert "cellHeight: 220" not in source
    assert "height: libraryGrid.cellHeight" in source
    assert "clip: true" in source
