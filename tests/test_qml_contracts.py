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


def test_folder_explorer_is_compact_hierarchical_and_keyboard_navigable() -> None:
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    context = (QML_DIR / "LibraryContextToolbar.qml").read_text(
        encoding="utf-8"
    )
    row = (QML_DIR / "FolderTreeRow.qml").read_text(encoding="utf-8")

    assert 'text: "EXPLORER"' in context
    assert "folderTreeModel.totalCount.toLocaleString()" in context
    assert "folderTreeModel.collapseAll()" in context
    assert "Layout.preferredWidth: 204" in page
    assert "delegate: FolderTreeRow {" in page
    assert "folderModel.toggleExpanded(folderPath)" in page
    assert "folderModel.parentIndex(folderRow.folderPath)" in page
    assert "reuseItems: true" in page

    assert "height: 34" in row
    assert "required property int folderDepth" in row
    assert "readonly property int visualDepth: Math.min(folderDepth, 6)" in row
    assert "model: Math.max(0, root.visualDepth)" in row
    assert 'root.folderExpanded ? "chevronDown" : "chevronRight"' in row
    assert "Keys.onRightPressed" in row
    assert "Keys.onLeftPressed" in row


def test_settings_exposes_maximum_mode_and_live_performance_diagnostics() -> None:
    settings = (QML_DIR / "SettingsPage.qml").read_text(encoding="utf-8")
    library = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")

    assert 'text: "PERFORMANCE"' in settings
    assert '"Maximum performance"' in settings
    assert '"Prefer hardware"' in settings
    assert "performanceMonitor.setActive(visible)" in settings
    assert "FrameAnimation {" in settings
    assert "performanceMonitor.recordFrameBatch" in settings
    assert "root.performanceState.renderer" in settings
    assert "root.performanceState.framePacing" in settings
    assert "root.performanceState.frameSpikes" in settings
    assert 'controller.settings.performance_mode === "maximum"' in library


def test_frameless_window_has_custom_titlebar_and_resize_fallbacks() -> None:
    main = (QML_DIR / "Main.qml").read_text(encoding="utf-8")
    titlebar = (QML_DIR / "WindowTitleBar.qml").read_text(
        encoding="utf-8"
    )
    resize_frame = (QML_DIR / "WindowResizeFrame.qml").read_text(
        encoding="utf-8"
    )
    controls = (QML_DIR / "WindowControlButton.qml").read_text(
        encoding="utf-8"
    )

    assert "Qt.FramelessWindowHint" in main
    assert "WindowTitleBar {" in main
    assert "WindowResizeFrame {" in main
    assert "Qt.ExpandedClientAreaHint" not in main
    assert "Qt.NoTitleBarBackgroundHint" not in main
    assert "readonly property int titleBarHeight" in main

    assert "root.windowController.beginMove()" in titlebar
    assert "root.windowController.updateMove()" in titlebar
    assert "root.windowController.toggleZoom()" in titlebar
    assert "root.windowController.performPrimaryZoom()" in titlebar
    assert "CommandCenter {" in titlebar
    assert "WorkbenchActivityButton {" in titlebar
    assert "randomSourceButtonItem" in titlebar
    assert 'Accessible.name: accessibleLabel' in controls

    assert resize_frame.count("root.beginResize(") == 8
    assert "windowController.updateResize()" in resize_frame
    for edge in ("LeftEdge", "RightEdge", "TopEdge", "BottomEdge"):
        assert f"Qt.{edge}" in resize_frame


def test_combo_popup_treats_trigger_as_its_parent() -> None:
    combo = (QML_DIR / "AppComboBox.qml").read_text(encoding="utf-8")

    assert "Popup.CloseOnPressOutsideParent" in combo
    assert "Popup.CloseOnPressOutside\n" not in combo


def test_upper_workbench_uses_two_compact_bands_and_one_search_surface() -> None:
    main = (QML_DIR / "Main.qml").read_text(encoding="utf-8")
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    titlebar = (QML_DIR / "WindowTitleBar.qml").read_text(
        encoding="utf-8"
    )
    command = (QML_DIR / "CommandCenter.qml").read_text(
        encoding="utf-8"
    )
    actions = (QML_DIR / "WorkbenchActionRegistry.qml").read_text(
        encoding="utf-8"
    )
    theme = (QML_DIR / "Theme.qml").read_text(encoding="utf-8")

    assert "LibraryContextToolbar {" in main
    assert "Theme.workbenchTitleHeight" in main
    assert "Theme.workbenchContextHeight" in main
    assert "WorkbenchActionRegistry {" in main
    assert 'text: "Video library"' not in page
    assert 'placeholderText: "Search filenames and folders"' not in page
    assert "GridLayout {\n                    id: libraryToolbar" not in page

    assert "CommandCenter {" in titlebar
    assert '"Search videos, folders, and commands"' in command
    assert "interval: 120" in command
    assert "requestCommandSearch" in command
    assert "parent: Overlay.overlay" in command
    assert '\"id\": \"pick_random\"' in actions
    assert '\"id\": \"toggle_folders\"' in actions
    assert '\"id\": \"theme_full_white\"' in actions
    assert "readonly property int workbenchTitleHeight: 40" in theme
    assert "readonly property int workbenchContextHeight: 42" in theme
