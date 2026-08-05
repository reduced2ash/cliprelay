pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    color: Theme.ink
    property bool showFolders: true
    property string currentFolder: ""
    property string searchText: ""
    property bool syncingReveal: false
    property Item fullscreenHost
    property Item randomSourceTrigger
    property bool prepareFullscreen: false
    property int activePreviewMediaId: 0
    property bool creatingWorkspace: false
    property bool commandShortcutsEnabled: !randomSourcePopup.opened
    readonly property bool randomSourcesOpen: randomSourcePopup.opened
    readonly property bool compactLibrary:
        String(controller.settings.library_density || "default")
            === "compact"
    readonly property string explorerSortMode:
        String(controller.settings.folder_sort_mode || "name_asc")
    readonly property bool prepareDocked:
        Number(controller.selectedMediaId || 0) > 0
            && !root.prepareFullscreen
    readonly property bool prepareHasEdits: preparePanel.hasEdits
    readonly property bool prepareCutActive: preparePanel.cutActive
    readonly property real prepareDockWidth:
        controller.settings.prepare_expanded
            ? Math.min(
                680,
                Math.max(
                    Math.min(420, Math.max(360, root.width * 0.34)),
                    root.width - (root.showFolders ? 210 : 0) - 460
                )
            )
            : Math.min(420, Math.max(360, root.width * 0.34))
    objectName: "libraryPage"

    signal searchFocusRequested()

    function focusSearch() {
        root.searchFocusRequested()
    }

    function setSearchText(value) {
        const normalized = String(value || "")
        if (root.searchText === normalized)
            return
        root.searchText = normalized
        if (!root.syncingReveal)
            controller.setSearch(normalized)
    }

    function toggleFolders() {
        root.showFolders = !root.showFolders
    }

    function chooseLibraryFolder() {
        root.creatingWorkspace = false
        libraryDialog.title = "Choose your video library"
        libraryDialog.open()
    }

    function chooseNewWorkspaceFolder() {
        root.captureWorkspaceDraft()
        root.creatingWorkspace = true
        libraryDialog.title = "Open a folder in a new workspace"
        libraryDialog.open()
    }

    function captureWorkspaceDraft() {
        controller.saveActiveWorkspaceDraft(preparePanel.captureDraft())
    }

    function activateWorkspace(workspaceId) {
        root.captureWorkspaceDraft()
        controller.activateWorkspace(workspaceId)
    }

    function closeWorkspace(workspaceId) {
        root.captureWorkspaceDraft()
        controller.closeWorkspace(workspaceId)
    }

    function duplicateWorkspace(workspaceId) {
        root.captureWorkspaceDraft()
        controller.duplicateWorkspace(workspaceId)
    }

    function closeOtherWorkspaces(workspaceId) {
        root.captureWorkspaceDraft()
        controller.closeOtherWorkspaces(workspaceId)
    }

    function closeWorkspacesToRight(workspaceId) {
        root.captureWorkspaceDraft()
        controller.closeWorkspacesToRight(workspaceId)
    }

    function reopenClosedWorkspace() {
        root.captureWorkspaceDraft()
        controller.reopenClosedWorkspace()
    }

    function setSortMode(mode) {
        controller.setSetting("sort_mode", mode)
    }

    function setFolderSortMode(mode) {
        controller.setSetting("folder_sort_mode", mode)
    }

    function selectSearchResult(mediaId) {
        controller.selectMedia(mediaId)
        Qt.callLater(function() {
            const mediaIndex = libraryModel.indexOf(mediaId)
            if (mediaIndex >= 0)
                libraryGrid.positionViewAtIndex(
                    mediaIndex,
                    GridView.Center
                )
        })
    }

    function selectSearchFolder(folderPath) {
        root.syncingReveal = true
        root.searchText = ""
        root.currentFolder = String(folderPath || "")
        root.showFolders = true
        controller.requestCommandSearch("")
        controller.setSearch("")
        controller.setFolder(root.currentFolder)
        const folderIndex = folderModel.expandTo(root.currentFolder)
        root.syncingReveal = false
        Qt.callLater(function() {
            if (folderIndex < 0)
                return
            folderList.currentIndex = folderIndex
            folderList.positionViewAtIndex(
                folderIndex,
                ListView.Center
            )
        })
    }

    function togglePlayback() {
        if (Number(controller.selectedMediaId || 0) > 0)
            preparePanel.togglePlayback()
    }

    function openSelectedInDefaultPlayer() {
        if (Number(controller.selectedMediaId || 0) > 0)
            controller.openSelectedVideo()
    }

    function togglePrepareWidth() {
        if (!root.prepareDocked)
            return
        controller.setSetting(
            "prepare_expanded",
            !Boolean(controller.settings.prepare_expanded)
        )
    }

    function openPrepareFullscreen() {
        if (Number(controller.selectedMediaId || 0) > 0)
            root.prepareFullscreen = true
    }

    function focusPrepareTab(index) {
        if (Number(controller.selectedMediaId || 0) <= 0)
            return
        preparePanel.studioTab = Math.max(0, Math.min(1, index))
    }

    function resetPrepareCut() {
        if (Number(controller.selectedMediaId || 0) > 0)
            preparePanel.resetCut()
    }

    function revealPreparedVideo() {
        if (Number(controller.selectedMediaId || 0) > 0)
            controller.revealSelectedInLibrary()
    }

    function closePrepare() {
        root.prepareFullscreen = false
        controller.clearSelection()
    }

    function playMedia(mediaId) {
        if (Number(controller.selectedMediaId || 0) !== Number(mediaId))
            controller.selectMedia(mediaId)
        Qt.callLater(root.togglePlayback)
    }

    function navigateSelection(direction) {
        controller.navigateSelection(direction)
    }

    function focusFolderIndex(targetIndex) {
        if (folderList.count <= 0)
            return
        var clampedIndex = Math.max(
            0,
            Math.min(folderList.count - 1, targetIndex)
        )
        folderList.currentIndex = clampedIndex
        folderList.positionViewAtIndex(clampedIndex, ListView.Contain)
        Qt.callLater(function() {
            var item = folderList.itemAtIndex(clampedIndex)
            if (item)
                item.forceActiveFocus()
        })
    }

    function toggleFolderBranch(folderPath) {
        var folderIndex = folderModel.toggleExpanded(folderPath)
        root.focusFolderIndex(folderIndex)
    }

    function toggleRandomSourcePopup() {
        if (randomSourcePopup.opened) {
            randomSourcePopup.close()
            return
        }
        // A click on the trigger is also an outside click for Popup. Suppress
        // the trailing Button click when that press just closed the popup.
        if (Date.now() - randomSourcePopup.lastClosedAt < 180)
            return
        randomSourcePopup.open()
    }

    function focusRandomFolderIndex(targetIndex) {
        if (randomFolderList.count <= 0)
            return
        const clampedIndex = Math.max(
            0,
            Math.min(randomFolderList.count - 1, targetIndex)
        )
        randomFolderList.currentIndex = clampedIndex
        randomFolderList.positionViewAtIndex(
            clampedIndex,
            ListView.Contain
        )
        Qt.callLater(function() {
            const item = randomFolderList.itemAtIndex(clampedIndex)
            if (item)
                item.forceActiveFocus()
        })
    }

    function toggleRandomFolderBranch(folderPath) {
        const folderIndex = randomFolderModel.toggleExpanded(folderPath)
        root.focusRandomFolderIndex(folderIndex)
    }

    Connections {
        target: controller
        function onLibraryRevealRequested(folder, mediaIndex, folderIndex) {
            root.activePreviewMediaId = 0
            root.syncingReveal = true
            root.showFolders = true
            root.currentFolder = folder
            root.searchText = ""
            controller.requestCommandSearch("")
            root.syncingReveal = false
            Qt.callLater(function() {
                if (folderIndex >= 0) {
                    folderList.currentIndex = folderIndex
                    folderList.positionViewAtIndex(folderIndex, ListView.Center)
                }
                libraryGrid.positionViewAtIndex(mediaIndex, GridView.Center)
                libraryGrid.forceActiveFocus()
            })
        }
        function onLibraryNavigationRestored(folder, search, folderIndex) {
            root.activePreviewMediaId = 0
            root.syncingReveal = true
            root.showFolders = true
            root.currentFolder = folder
            root.searchText = search
            controller.requestCommandSearch("")
            root.syncingReveal = false
            Qt.callLater(function() {
                if (folderIndex >= 0) {
                    folderList.currentIndex = folderIndex
                    folderList.positionViewAtIndex(
                        folderIndex,
                        ListView.Contain
                    )
                }
            })
        }
        function onSelectedMediaChanged() {
            if (Number(controller.selectedMediaId || 0) <= 0)
                root.prepareFullscreen = false
        }
        function onSettingsChanged() {
            if (!controller.settings.hover_previews)
                root.activePreviewMediaId = 0
        }
        function onLibrarySelectionRequested(mediaIndex) {
            if (mediaIndex >= 0) {
                libraryGrid.positionViewAtIndex(mediaIndex, GridView.Visible)
            }
        }
    }

    onVisibleChanged: {
        if (!visible)
            activePreviewMediaId = 0
    }
    onCompactLibraryChanged: {
        activePreviewMediaId = 0
        Qt.callLater(function() {
            const selectedIndex = libraryModel.indexOf(
                Number(controller.selectedMediaId || 0)
            )
            if (selectedIndex >= 0) {
                libraryGrid.positionViewAtIndex(
                    selectedIndex,
                    GridView.Contain
                )
            }
        })
    }
    onExplorerSortModeChanged: Qt.callLater(function() {
        const folderIndex = folderModel.indexOf(root.currentFolder)
        if (folderIndex < 0)
            return
        folderList.currentIndex = folderIndex
        folderList.positionViewAtIndex(folderIndex, ListView.Contain)
    })

    Component.onCompleted: {
        root.currentFolder = String(controller.activeWorkspace.folder || "")
        root.searchText = String(controller.activeWorkspace.search || "")
    }

    FolderDialog {
        id: libraryDialog
        title: "Choose your video library"
        onAccepted: {
            root.currentFolder = ""
            root.searchText = ""
            if (root.creatingWorkspace)
                controller.createWorkspace(selectedFolder)
            else
                controller.setSetting("library_root", selectedFolder)
            root.creatingWorkspace = false
        }
        onRejected: root.creatingWorkspace = false
    }

    Popup {
        id: randomSourcePopup
        objectName: "randomSourcePopup"
        parent: Overlay.overlay
        property bool selectedOnly: false
        property double lastClosedAt: 0
        readonly property real edgeInset: 12
        readonly property point triggerTop: parent
            && root.randomSourceTrigger
            ? root.randomSourceTrigger.mapToItem(parent, 0, 0)
            : Qt.point(0, 0)
        readonly property point triggerBottomRight: parent
            && root.randomSourceTrigger
            ? root.randomSourceTrigger.mapToItem(
                parent,
                root.randomSourceTrigger.width,
                root.randomSourceTrigger.height
            )
            : Qt.point(0, 0)
        readonly property real belowY: triggerBottomRight.y + 6
        readonly property real aboveY: triggerTop.y - height - 6

        x: parent
            ? Math.max(
                edgeInset,
                Math.min(
                    parent.width - width - edgeInset,
                    triggerBottomRight.x - width
                )
            )
            : edgeInset
        y: parent
            ? (belowY + height <= parent.height - edgeInset
                ? belowY
                : Math.max(
                    edgeInset,
                    Math.min(
                        parent.height - height - edgeInset,
                        aboveY
                    )
                ))
            : edgeInset
        z: 100
        width: Math.min(
            456,
            Math.max(
                0,
                (parent ? parent.width : root.width) - edgeInset * 2
            )
        )
        height: Math.min(
            548,
            Math.max(
                0,
                (parent ? parent.height : root.height) - edgeInset * 2
            )
        )
        padding: 0
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        onClosed: lastClosedAt = Date.now()
        onOpened: {
            controller.loadRandomFolderOptions()
            randomFolderModel.setSelectedOnly(selectedOnly)
            randomFolderModel.setFilter(randomFolderSearch.text)
            Qt.callLater(function() {
                randomFolderSearch.forceActiveFocus()
            })
        }

        Timer {
            id: randomFolderSearchTimer
            interval: 120
            onTriggered: randomFolderModel.setFilter(randomFolderSearch.text)
        }

        background: Rectangle {
            color: Theme.surface
            radius: Theme.radiusSm
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 38
                Layout.leftMargin: 10
                Layout.rightMargin: 5
                spacing: 7

                AppIcon {
                    Layout.preferredWidth: 15
                    Layout.preferredHeight: 15
                    name: "folders"
                    strokeWidth: 1.8
                    iconColor: Theme.accentText
                }
                Text {
                    Layout.fillWidth: true
                    text: "RANDOM SOURCES"
                    color: Theme.textSoft
                    font.pixelSize: Theme.textXs
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.8
                }
                Text {
                    text: controller.randomFolderOptionsLoading
                        ? "INDEXING"
                        : randomFolderModel.totalCount.toLocaleString()
                            + " NODES"
                    color: controller.randomFolderOptionsLoading
                        ? Theme.warning : Theme.mutedSoft
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.5
                }
                AppButton {
                    text: "Collapse source tree"
                    iconName: "contract"
                    kind: "ghost"
                    compact: true
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    onClicked: randomFolderModel.collapseAll()
                }
                AppButton {
                    text: "Expand source tree"
                    iconName: "expand"
                    kind: "ghost"
                    compact: true
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    onClicked: randomFolderModel.expandAll()
                }
                AppButton {
                    text: "Close random sources"
                    iconName: "close"
                    kind: "ghost"
                    compact: true
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    onClicked: randomSourcePopup.close()
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 7
                Layout.rightMargin: 6
                Layout.topMargin: 6
                Layout.bottomMargin: 6
                spacing: 4

                AppField {
                    id: randomFolderSearch
                    Layout.fillWidth: true
                    Layout.preferredHeight: 32
                    iconName: "search"
                    placeholderText: "Filter source tree"
                    Accessible.name: "Search random source folders"
                    onTextChanged: randomFolderSearchTimer.restart()
                }
                AppButton {
                    visible: randomFolderSearch.text.length > 0
                    text: "Clear folder search"
                    iconName: "close"
                    kind: "ghost"
                    compact: true
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                    onClicked: {
                        randomFolderSearch.text = ""
                        randomFolderSearch.forceActiveFocus()
                    }
                }
            }

            Rectangle {
                id: allFoldersChoice
                Layout.fillWidth: true
                Layout.preferredHeight: 34
                activeFocusOnTab: true
                Accessible.role: Accessible.CheckBox
                Accessible.name: "Use the entire library for random selection"
                Accessible.checked: controller.allRandomFoldersSelected
                color: controller.allRandomFoldersSelected
                    ? Theme.accentSoft
                    : allFoldersHover.hovered || activeFocus
                        ? Theme.hover : "transparent"

                function toggleAll() {
                    if (controller.allRandomFoldersSelected)
                        controller.clearRandomFolders()
                    else
                        controller.selectAllRandomFolders()
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 15
                    height: 15
                    radius: 3
                    color: controller.allRandomFoldersSelected
                        ? Theme.accent : Theme.raised
                    border.width: 1
                    border.color: controller.allRandomFoldersSelected
                        ? Theme.accent : Theme.borderStrong
                    AppIcon {
                        anchors.centerIn: parent
                        width: 11
                        height: 11
                        visible: controller.allRandomFoldersSelected
                        name: "check"
                        strokeWidth: 2.4
                        iconColor: Theme.accentContent
                    }
                }
                AppIcon {
                    anchors.left: parent.left
                    anchors.leftMargin: 32
                    anchors.verticalCenter: parent.verticalCenter
                    width: 14
                    height: 14
                    name: "library"
                    strokeWidth: 1.7
                    iconColor: controller.allRandomFoldersSelected
                        ? Theme.accentText : Theme.muted
                }
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 52
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Entire library"
                    color: Theme.text
                    font.pixelSize: Theme.textXs
                    font.weight: Font.DemiBold
                }
                Text {
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    text: controller.allRandomFoldersSelected
                        ? "SELECTED" : "SELECT ALL"
                    color: controller.allRandomFoldersSelected
                        ? Theme.accentText : Theme.mutedSoft
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.5
                }
                HoverHandler { id: allFoldersHover }
                TapHandler { onTapped: allFoldersChoice.toggleAll() }
                Keys.onSpacePressed: allFoldersChoice.toggleAll()
                Keys.onReturnPressed: allFoldersChoice.toggleAll()
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 6
                Layout.rightMargin: 6
                Layout.topMargin: 3
                Layout.bottomMargin: 3
                spacing: 5

                AppButton {
                    text: "Selected only"
                    kind: randomSourcePopup.selectedOnly
                        ? "secondary" : "ghost"
                    iconOnly: false
                    Layout.preferredHeight: 28
                    Layout.preferredWidth: 104
                    onClicked: {
                        randomSourcePopup.selectedOnly =
                            !randomSourcePopup.selectedOnly
                        randomFolderModel.setSelectedOnly(
                            randomSourcePopup.selectedOnly
                        )
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: randomFolderModel.visibleCount.toLocaleString()
                        + " VISIBLE"
                    color: Theme.mutedSoft
                    font.pixelSize: 10
                    font.weight: Font.Medium
                    font.letterSpacing: 0.45
                }
                AppButton {
                    text: "Clear"
                    kind: "ghost"
                    iconOnly: false
                    enabled: controller.randomFolderSelectionCount > 0
                    Layout.preferredHeight: 28
                    Layout.preferredWidth: 64
                    onClicked: controller.clearRandomFolders()
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Theme.border
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: randomFolderList
                    anchors.fill: parent
                    anchors.topMargin: 3
                    anchors.bottomMargin: 3
                    clip: true
                    pixelAligned: true
                    synchronousDrag: true
                    reuseItems: true
                    spacing: 0
                    model: randomFolderModel
                    boundsBehavior: Flickable.StopAtBounds
                    keyNavigationEnabled: false
                    ScrollBar.vertical: AppScrollBar { }

                    delegate: RandomSourceTreeRow {
                        id: randomFolderRow
                        width: ListView.view.width
                        onSelectionRequested: function(enabled) {
                            controller.setRandomFolderEnabled(
                                randomFolderRow.folderPath,
                                enabled
                            )
                        }
                        onToggleRequested:
                            root.toggleRandomFolderBranch(
                                randomFolderRow.folderPath
                            )
                        onMoveFocusRequested:
                            root.focusRandomFolderIndex(
                                randomFolderRow.index + delta
                            )
                        onParentFocusRequested: {
                            const parentIndex =
                                randomFolderModel.parentIndex(
                                    randomFolderRow.folderPath
                                )
                            if (parentIndex >= 0)
                                root.focusRandomFolderIndex(parentIndex)
                        }
                    }
                }

                Column {
                    visible: controller.randomFolderOptionsLoading
                        && randomFolderModel.totalCount === 0
                    anchors.centerIn: parent
                    width: Math.min(220, parent.width - 48)
                    spacing: 8
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Building source tree…"
                        color: Theme.muted
                        font.pixelSize: Theme.textXs
                    }
                    AppProgressBar {
                        width: parent.width
                        indeterminate: true
                    }
                }

                Text {
                    visible: !controller.randomFolderOptionsLoading
                        && randomFolderModel.totalCount === 0
                    anchors.centerIn: parent
                    width: parent.width - 48
                    text: controller.scanning
                        ? "Source folders appear as videos are indexed."
                        : "Rescan the library to build the source tree."
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    wrapMode: Text.Wrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    visible: !controller.randomFolderOptionsLoading
                        && randomFolderModel.totalCount > 0
                        && randomFolderModel.visibleCount === 0
                    anchors.centerIn: parent
                    width: parent.width - 48
                    text: randomSourcePopup.selectedOnly
                        ? "No selected source folders match."
                        : "No source folders match."
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    wrapMode: Text.Wrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 10
                Layout.rightMargin: 6
                Layout.topMargin: 4
                Layout.bottomMargin: 4
                spacing: 8

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Text {
                        text: controller.randomFolderSelectionCount === 0
                            ? "NO SOURCES SELECTED"
                            : controller.allRandomFoldersSelected
                                ? "ENTIRE LIBRARY"
                                : controller.randomFolderSelectionCount
                                    .toLocaleString()
                                    + " SOURCE FOLDERS"
                        color: controller.randomFolderSelectionCount === 0
                            ? Theme.warning : Theme.textSoft
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.55
                    }
                    Text {
                        text: "Parent checks include every nested folder"
                        color: Theme.mutedSoft
                        font.pixelSize: 10
                    }
                }
                AppButton {
                    text: "Done"
                    kind: "secondary"
                    iconOnly: false
                    Layout.preferredHeight: 30
                    Layout.preferredWidth: 72
                    onClicked: randomSourcePopup.close()
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Rectangle {
                visible: root.showFolders && controller.settings.library_root
                Layout.fillHeight: true
                Layout.preferredWidth: 204
                color: Theme.surfaceSoft

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 0

                    ListView {
                        id: folderList
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.leftMargin: 4
                        Layout.rightMargin: 4
                        Layout.topMargin: 4
                        Layout.bottomMargin: 4
                        spacing: 0
                        clip: true
                        pixelAligned: true
                        synchronousDrag: true
                        reuseItems: true
                        boundsBehavior: Flickable.StopAtBounds
                        model: folderModel
                        delegate: FolderTreeRow {
                            id: folderRow
                            selected: root.currentFolder === folderPath

                            function chooseFolder() {
                                folderList.currentIndex = folderRow.index
                                root.currentFolder = folderRow.folderPath
                                controller.setFolder(folderRow.folderPath)
                            }

                            onActivated: chooseFolder()
                            onToggleRequested: {
                                root.toggleFolderBranch(folderRow.folderPath)
                            }
                            onMoveFocusRequested: function(delta) {
                                root.focusFolderIndex(folderRow.index + delta)
                            }
                            onParentFocusRequested: {
                                root.focusFolderIndex(
                                    folderModel.parentIndex(folderRow.folderPath)
                                )
                            }
                        }

                        ScrollBar.vertical: AppScrollBar { }
                    }

                }
            }

            Rectangle { visible: root.showFolders && controller.settings.library_root; Layout.fillHeight: true; width: 1; color: Theme.border }

            ColumnLayout {
                id: libraryColumn
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                ColumnLayout {
                    visible: controller.activeWorkspaceScanning
                    Layout.fillWidth: true
                    Layout.leftMargin: 20; Layout.rightMargin: 20; Layout.bottomMargin: 10
                    spacing: 4
                    AppProgressBar {
                        Layout.fillWidth: true
                        from: 0
                        to: 1
                        indeterminate: controller.scanProgress < 0
                        value: Math.max(0, controller.scanProgress)
                    }
                    Text { text: controller.scanMessage; color: Theme.muted; font.pixelSize: Theme.textXs }
                }

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    AppEmptyState {
                        visible: !controller.settings.library_root
                        anchors.centerIn: parent
                        width: Math.min(480, parent.width - 48)
                        iconName: "folders"
                        title: "Bring your video folders together"
                        body: "Choose one top-level folder. ClipRelay finds videos inside every nested folder without moving your originals."
                        actionText: "Choose video library"
                        onAction: libraryDialog.open()
                    }

                    AppEmptyState {
                        visible: controller.settings.library_root
                            && !controller.activeWorkspaceScanning
                            && libraryGrid.count === 0
                        anchors.centerIn: parent
                        width: Math.min(420, parent.width - 48)
                        iconName: root.searchText.length ? "search" : "media"
                        title: root.searchText.length ? "No matching videos" : "No videos found"
                        body: root.searchText.length
                            ? "Try a broader search or switch folders."
                            : (controller.settings.auto_index
                                ? "Rescan the library, or enable deep format detection for uncommon files."
                                : "Background indexing is off. Pick randomly now, or rescan to build the visual library.")
                    }

                    GridView {
                        id: libraryGrid
                        visible: controller.settings.library_root && count > 0
                        anchors.fill: parent
                        anchors.leftMargin: Theme.libraryGridInset
                        anchors.rightMargin: Theme.libraryGridInset
                        anchors.topMargin: 8
                        anchors.bottomMargin: 8
                        clip: true
                        model: libraryModel
                        readonly property bool compactDensity:
                            root.compactLibrary
                        readonly property int tileGap: compactDensity
                            ? Theme.libraryGridGapCompact
                            : Theme.libraryGridGapDefault
                        readonly property int minimumTileWidth: compactDensity
                            ? Theme.libraryTileMinCompact
                            : Theme.libraryTileMinDefault
                        property int columns: Math.max(
                            1,
                            Math.floor(
                                (width + tileGap)
                                / (minimumTileWidth + tileGap)
                            )
                        )
                        readonly property real tileContentWidth: Math.max(
                            0,
                            cellWidth - tileGap
                        )
                        readonly property real tilePosterHeight: Math.max(
                            96,
                            Math.round(tileContentWidth * 9 / 16)
                        )
                        readonly property real tileChromeHeight: compactDensity
                            ? Theme.libraryTileChromeCompact
                            : Theme.libraryTileChromeDefault
                        cellWidth: width / columns
                        cellHeight: Math.ceil(
                            tilePosterHeight + tileChromeHeight + tileGap
                        )
                        cacheBuffer: cellHeight * (
                            controller.settings.performance_mode === "maximum"
                                ? 3 : 2
                        )
                        pixelAligned: true
                        synchronousDrag: true
                        maximumFlickVelocity: 4000
                        reuseItems: true
                        boundsBehavior: Flickable.StopAtBounds
                        onAtYEndChanged: {
                            if (atYEnd) controller.loadMoreMedia()
                        }
                        ScrollBar.vertical: AppScrollBar { }
                        delegate: Item {
                            id: tileCell
                            required property int mediaId
                            required property string name
                            required property string thumbnailUrl
                            required property string thumbnailState
                            required property string previewUrl
                            required property string durationLabel
                            required property string sizeLabel
                            required property string resolution
                            required property string folder
                            required property int postedCount
                            width: libraryGrid.cellWidth
                            height: libraryGrid.cellHeight
                            clip: true
                            MediaTile {
                                anchors.fill: parent
                                anchors.rightMargin: libraryGrid.tileGap
                                anchors.bottomMargin: libraryGrid.tileGap
                                mediaId: tileCell.mediaId
                                name: tileCell.name
                                thumbnailUrl: tileCell.thumbnailUrl
                                thumbnailState: tileCell.thumbnailState
                                previewUrl: tileCell.previewUrl
                                durationLabel: tileCell.durationLabel
                                sizeLabel: tileCell.sizeLabel
                                resolution: tileCell.resolution
                                folder: root.showFolders ? "" : tileCell.folder
                                postedCount: tileCell.postedCount
                                selected: controller.selectedMediaId === tileCell.mediaId
                                compact: libraryGrid.compactDensity
                                previewActive:
                                    root.activePreviewMediaId === tileCell.mediaId
                                onChosen: controller.selectMedia(mediaId)
                                onPlaybackRequested: root.playMedia(mediaId)
                                onNavigationRequested:
                                    controller.navigateSelection(direction)
                                onPreviewRequested: function(mediaId) {
                                    root.activePreviewMediaId = mediaId
                                }
                                onPreviewReleased: function(mediaId) {
                                    if (root.activePreviewMediaId === mediaId)
                                        root.activePreviewMediaId = 0
                                }
                            }
                        }
                    }
                }

            }

            Rectangle {
                visible: root.prepareDocked
                Layout.fillHeight: true
                width: 1
                color: Theme.border
            }

            Item {
                id: prepareSlot
                visible: root.prepareDocked
                Layout.fillHeight: true
                Layout.preferredWidth: root.prepareDockWidth
                Layout.minimumWidth: 360
                Layout.maximumWidth: 680
            }
        }
    }

    PreparePanel {
        id: preparePanel
        parent: root.prepareFullscreen && root.fullscreenHost
            ? root.fullscreenHost
            : prepareSlot
        anchors.fill: parent
        visible: controller.selectedMediaId > 0
        z: root.prepareFullscreen ? 80 : 0
        studioMode: root.prepareFullscreen
        onFullScreenExitRequested: root.prepareFullscreen = false
        onCloseRequested: root.closePrepare()
    }
}
