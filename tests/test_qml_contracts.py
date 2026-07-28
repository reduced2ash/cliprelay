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
    assert "tilePosterHeight + tileChromeHeight + tileGap" in source
    assert "cellHeight: 220" not in source
    assert "height: libraryGrid.cellHeight" in source
    assert "clip: true" in source


def test_library_canvas_uses_workbench_density_and_one_hover_preview() -> None:
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    tile = (QML_DIR / "MediaTile.qml").read_text(encoding="utf-8")
    settings = (QML_DIR / "SettingsPage.qml").read_text(encoding="utf-8")
    actions = (QML_DIR / "WorkbenchActionRegistry.qml").read_text(
        encoding="utf-8"
    )
    theme = (QML_DIR / "Theme.qml").read_text(encoding="utf-8")

    assert "property int activePreviewMediaId: 0" in page
    assert 'controller.settings.library_density || "default"' in page
    assert "Theme.libraryTileMinDefault" in page
    assert "Theme.libraryTileMinCompact" in page
    assert "Theme.libraryGridGapDefault" in page
    assert "Theme.libraryGridGapCompact" in page
    assert "id: libraryFooter" not in page
    assert "root.activePreviewMediaId === tileCell.mediaId" in page
    assert "root.activePreviewMediaId = mediaId" in page

    assert "property bool compact: false" in tile
    assert "property bool previewActive: false" in tile
    assert "signal previewRequested(int mediaId)" in tile
    assert "signal previewReleased(int mediaId)" in tile
    assert "interval: Theme.libraryPreviewDelay" in tile
    assert "root.previewActive" in tile
    assert "scale: tileTap.pressed" not in tile
    assert "Image.PreserveAspectFit" in tile
    assert "VideoOutput.PreserveAspectFit" in tile
    assert 'fields.push(root.mediaUnchecked ? "Unchecked"' in tile
    assert tile.count("font.family: Theme.monoFamily") == 1
    assert "Thumbnail queued" not in tile
    assert "Thumbnail pending" not in tile
    assert "Preparing preview" not in (
        QML_DIR / "PrepareVideoEditor.qml"
    ).read_text(encoding="utf-8")

    assert "font.family: Theme.monoFamily" not in page
    assert "font.pixelSize: Theme.textXs" in page

    assert 'text: "Library density"' in settings
    assert 'model: ["Default", "Compact"]' in settings
    assert '"library_density"' in settings
    assert '"id": "density_default"' in actions
    assert '"id": "density_compact"' in actions
    assert 'setSetting("library_density", "compact")' in actions

    assert "readonly property int libraryGridInset: 14" in theme
    assert "readonly property int libraryTileRadius: 4" in theme
    assert "readonly property int libraryPreviewDelay: 350" in theme


def test_folder_explorer_is_compact_hierarchical_and_keyboard_navigable() -> None:
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    context = (QML_DIR / "LibraryContextToolbar.qml").read_text(
        encoding="utf-8"
    )
    row = (QML_DIR / "FolderTreeRow.qml").read_text(encoding="utf-8")
    sort_control = (QML_DIR / "LibrarySortControl.qml").read_text(
        encoding="utf-8"
    )

    assert 'text: "EXPLORER"' in context
    assert "folderTreeModel.totalCount.toLocaleString()" in context
    assert "folderTreeModel.collapseAll()" in context
    assert "Layout.preferredWidth: 204" in page
    assert "delegate: FolderTreeRow {" in page
    assert "folderModel.toggleExpanded(folderPath)" in page
    assert "folderModel.parentIndex(folderRow.folderPath)" in page
    assert "reuseItems: true" in page
    assert "LibrarySortControl {" in context
    assert 'text: "VIDEOS"' in sort_control
    assert 'text: "EXPLORER FOLDERS"' in sort_control
    assert '"mode": "name_asc"' in sort_control
    assert '"mode": "added_recent"' in sort_control
    assert '"mode": "recent"' in sort_control
    assert '"mode": "count_desc"' in sort_control
    assert "Popup.CloseOnPressOutsideParent" in sort_control
    assert '"folder_sort_" + modelData.mode' in sort_control
    assert 'setSetting("folder_sort_mode", mode)' in page

    assert "height: 34" in row
    assert "required property int folderDepth" in row
    assert "readonly property int visualDepth: Math.min(folderDepth, 6)" in row
    assert "model: Math.max(0, root.visualDepth)" in row
    assert 'root.folderExpanded ? "chevronDown" : "chevronRight"' in row
    assert "Keys.onRightPressed" in row
    assert "Keys.onLeftPressed" in row


def test_global_media_shortcuts_and_random_source_tree_are_window_wide() -> None:
    main = (QML_DIR / "Main.qml").read_text(encoding="utf-8")
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    row = (QML_DIR / "RandomSourceTreeRow.qml").read_text(
        encoding="utf-8"
    )

    assert "readonly property bool textEntryActive" in main
    assert "item instanceof TextInput" in main
    assert "item instanceof TextField" in main
    for sequence in ('"R"', '"Left"', '"Right"'):
        shortcut = main.index(f"sequence: {sequence}")
        assert "context: Qt.WindowShortcut" in main[
            shortcut:shortcut + 120
        ]
    assert "enabled: !window.textEntryActive" in main
    assert "window.currentPage = 0" in main
    assert "controller.pickRandom()" in main
    assert "libraryPage.navigateSelection(-1)" in main
    assert "libraryPage.navigateSelection(1)" in main
    assert "event.key === Qt.Key_R" not in main

    assert 'text: "RANDOM SOURCES"' in page
    assert 'placeholderText: "Filter source tree"' in page
    assert "delegate: RandomSourceTreeRow {" in page
    assert "randomFolderModel.toggleExpanded(folderPath)" in page
    assert "randomFolderModel.collapseAll()" in page
    assert "randomFolderModel.expandAll()" in page
    assert 'text: "Parent checks include every nested folder"' in page

    assert "height: 32" in row
    assert "required property int folderSelectionState" in row
    assert "required property int folderDepth" in row
    assert "readonly property bool partiallySelected" in row
    assert "root.selectionRequested(!root.fullySelected)" in row
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
    assert 'text: "Open command palette"' in titlebar
    assert 'iconName: "command"' in titlebar
    assert "onClicked: root.toggleCommands()" in titlebar
    assert '"ClipRelay Library"' not in titlebar
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
    assert "requestCommandOverview" in command
    assert "function libraryScope()" in command
    assert "root.appController.requestCommandSearch(\n"
    assert "root.mediaQuery,\n                scope" in command
    assert "root.appController.requestCommandOverview(scope)" in command
    assert 'property string activeScope: "all"' in command
    assert '{ "id": "videos", "label": "Videos" }' in command
    assert '{ "id": "folders", "label": "Folders" }' in command
    assert '{ "id": "commands", "label": "Commands" }' in command
    assert "root.libraryRows.concat(root.commandRows)" in command
    assert "root.quickCommandRows.concat(root.libraryRows)" in command
    assert 'row.kind === "folder") {\n            root.mediaQuery = ""' in command
    assert "popupType: Popup.Item" in command
    assert "parent: root" in command
    assert "y: root.height + 5" in command
    assert "mapToItem" not in command
    assert '\"id\": \"pick_random\"' in actions
    assert '\"id\": \"navigate_back\"' in actions
    assert '\"id\": \"navigate_forward\"' in actions
    assert '\"id\": \"previous_random\"' not in actions
    assert '\"id\": \"previous_video\"' not in actions
    assert '\"id\": \"next_video\"' not in actions
    assert 'action(\"navigate_back\")' in titlebar
    assert 'action(\"navigate_forward\")' in titlebar
    assert 'triggerAction(\"navigate_back\")' in main
    assert 'triggerAction(\"navigate_forward\")' in main
    assert "libraryPage.navigateSelection(-1)" in main
    assert "libraryPage.navigateSelection(1)" in main
    assert "pickPreviousRandom" not in main
    assert "onLibraryNavigationRestored" in page
    assert "const folderIndex = folderModel.expandTo(root.currentFolder)" in page
    assert "folderList.positionViewAtIndex(\n"
    assert "folderIndex,\n                ListView.Center" in page
    assert '\"id\": \"toggle_folders\"' in actions
    assert '\"id\": \"theme_full_white\"' in actions
    assert "readonly property int workbenchTitleHeight: 40" in theme
    assert "readonly property int workbenchContextHeight: 42" in theme


def test_docked_prepare_uses_the_shared_pane_aware_context_bar() -> None:
    main = (QML_DIR / "Main.qml").read_text(encoding="utf-8")
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    context = (QML_DIR / "LibraryContextToolbar.qml").read_text(
        encoding="utf-8"
    )
    panel = (QML_DIR / "PreparePanel.qml").read_text(encoding="utf-8")

    assert "showPrepare: libraryPage.prepareDocked" in main
    assert "prepareWidth: libraryPage.prepareDockWidth" in main
    assert "libraryPage: libraryPage" in main

    assert "readonly property bool prepareDocked:" in page
    assert "readonly property real prepareDockWidth:" in page
    assert "Layout.preferredWidth: root.prepareDockWidth" in page
    assert "function openSelectedInDefaultPlayer()" in page
    assert "function togglePrepareWidth()" in page
    assert "function openPrepareFullscreen()" in page
    assert "function closePrepare()" in page

    assert "readonly property bool showPrepareContext:" in context
    assert "libraryContextSlot.width < 680" in context
    assert 'text: "Prepare"' in context
    assert "root.libraryPage.openSelectedInDefaultPlayer()" in context
    assert "root.libraryPage.togglePrepareWidth()" in context
    assert "root.libraryPage.openPrepareFullscreen()" in context
    assert "root.libraryPage.closePrepare()" in context

    assert "visible: root.studioMode" in panel
    assert "height: root.studioMode ? 70 : 0" in panel
    assert (
        "anchors.top: root.studioMode ? prepareHeader.bottom : parent.top"
        in panel
    )
    assert 'text: "Full-screen editor"' in panel
    assert 'text: "Widen Prepare"' not in panel
    assert "signal fullScreenRequested()" not in panel


def test_workspace_context_keeps_sidebar_geometry_stable_on_every_page() -> None:
    main = (QML_DIR / "Main.qml").read_text(encoding="utf-8")
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    context = (QML_DIR / "LibraryContextToolbar.qml").read_text(
        encoding="utf-8"
    )
    theme = (QML_DIR / "Theme.qml").read_text(encoding="utf-8")

    assert "currentPage: window.currentPage" in main
    assert "Layout.preferredHeight: Theme.workbenchContextHeight" in main
    assert "visible: window.currentPage === 0" not in main
    assert "readonly property bool libraryPageActive: currentPage === 0" in context
    assert '? "History" : currentPage === 2 ? "Settings" : "Library"' in context
    assert "libraryPageActive && hasLibrary && showFolders" in context
    assert "visible: !root.libraryPageActive" in context
    assert "readonly property int workspaceTabHeight: 34" in theme
    assert "WorkspaceTabBar {" in main
    assert "Layout.preferredHeight: Theme.workspaceTabHeight" in main
    assert "id: sidebarFooter" not in main
    assert "id: libraryFooter" not in page


def test_global_workspace_tabs_are_compact_contextual_and_browser_like() -> None:
    main = (QML_DIR / "Main.qml").read_text(encoding="utf-8")
    bar = (QML_DIR / "WorkspaceTabBar.qml").read_text(encoding="utf-8")
    page = (QML_DIR / "LibraryPage.qml").read_text(encoding="utf-8")
    actions = (QML_DIR / "WorkbenchActionRegistry.qml").read_text(
        encoding="utf-8"
    )

    assert "WorkspaceTabBar {" in main
    assert "StandardKey.Close" in main
    assert "controller.activeWorkspaceId" in main
    assert '"Ctrl+Tab"' in main
    assert '"Ctrl+Shift+Tab"' in main
    assert 'Qt.platform.os === "osx" ? "Meta+T" : "Ctrl+T"' in main
    assert "libraryPage.captureWorkspaceDraft()" in main

    assert "implicitHeight: Theme.workspaceTabHeight" in bar
    assert "model: root.appController.workspaceTabs" in bar
    assert "Qt.RightButton" in bar
    assert 'text: "Rename workspace"' in bar
    assert 'text: "Duplicate workspace"' in bar
    assert 'text: "Close other workspaces"' in bar
    assert 'text: "Close workspaces to the right"' in bar
    assert 'text: "Reopen closed workspace"' in bar
    assert "root.libraryPage.activateWorkspace(" in bar
    assert "root.libraryPage.closeWorkspace(" in bar

    assert "function captureWorkspaceDraft()" in page
    assert "function chooseNewWorkspaceFolder()" in page
    assert "controller.createWorkspace(selectedFolder)" in page
    assert '"id": "new_workspace"' in actions
    assert '"id": "close_workspace"' in actions
    assert '"id": "reopen_workspace"' in actions


def test_clickable_controls_center_icons_and_use_keyboard_only_focus_rings() -> None:
    workbench_button = (QML_DIR / "WorkbenchButton.qml").read_text(
        encoding="utf-8"
    )
    app_button = (QML_DIR / "AppButton.qml").read_text(encoding="utf-8")

    assert "contentItem: Item {" in workbench_button
    assert "anchors.centerIn: parent" in workbench_button
    assert "readonly property real resolvedVerticalPadding" in app_button
    assert "topPadding: resolvedVerticalPadding" in app_button
    assert "bottomPadding: resolvedVerticalPadding" in app_button
    assert "Math.floor((height - 20) / 2)" in app_button
    assert "verticalAlignment: Text.AlignVCenter" in app_button
    assert "contentItem: Item {" in app_button
    assert "anchors.centerIn: parent" in app_button
    assert "visible: control.iconOnly && control.iconName.length > 0" in app_button
    assert "Item { Layout.fillWidth: true; visible: control.iconOnly }" not in app_button

    for filename in (
        "WorkbenchButton.qml",
        "AppButton.qml",
        "NavButton.qml",
        "WindowControlButton.qml",
        "ThemeChoice.qml",
        "AppCheckBox.qml",
        "AppComboBox.qml",
        "WorkbenchComboBox.qml",
        "AppSlider.qml",
        "AppRangeSlider.qml",
    ):
        source = (QML_DIR / filename).read_text(encoding="utf-8")
        assert "visualFocus" in source
        assert ".activeFocus" not in source
