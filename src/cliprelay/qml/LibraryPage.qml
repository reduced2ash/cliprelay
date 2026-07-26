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
    property bool syncingReveal: false
    property bool compactToolbar: width < 1100
    property Item fullscreenHost
    property bool prepareFullscreen: false
    property bool commandShortcutsEnabled: !randomSourcePopup.opened
    objectName: "libraryPage"

    function focusSearch() {
        searchField.forceActiveFocus()
        searchField.selectAll()
    }

    function togglePlayback() {
        if (Number(controller.selectedMediaId || 0) > 0)
            preparePanel.togglePlayback()
    }

    function playMedia(mediaId) {
        if (Number(controller.selectedMediaId || 0) !== Number(mediaId))
            controller.selectMedia(mediaId)
        Qt.callLater(root.togglePlayback)
    }

    function navigateSelection(direction) {
        controller.navigateSelection(direction)
    }

    Connections {
        target: controller
        function onLibraryRevealRequested(folder, mediaIndex, folderIndex) {
            root.syncingReveal = true
            root.showFolders = true
            root.currentFolder = folder
            searchTimer.stop()
            searchField.text = ""
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
        function onSelectedMediaChanged() {
            if (Number(controller.selectedMediaId || 0) <= 0)
                root.prepareFullscreen = false
        }
        function onLibrarySelectionRequested(mediaIndex) {
            if (mediaIndex >= 0) {
                libraryGrid.positionViewAtIndex(mediaIndex, GridView.Visible)
            }
        }
    }

    FolderDialog {
        id: libraryDialog
        title: "Choose your video library"
        onAccepted: controller.setSetting("library_root", selectedFolder)
    }

    Popup {
        id: randomSourcePopup
        parent: root
        property bool selectedOnly: false
        x: Math.max(
            16,
            Math.min(
                root.width - width - 16,
                randomSourceButton.mapToItem(root, 0, 0).x
            )
        )
        y: 70
        z: 100
        width: Math.min(520, root.width - 32)
        height: Math.min(640, root.height - 92)
        padding: 0
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
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
            radius: Theme.radiusLg
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 16
                Layout.rightMargin: 16
                Layout.topMargin: 16
                Layout.bottomMargin: 14
                spacing: 12

                Rectangle {
                    Layout.preferredWidth: 38
                    Layout.preferredHeight: 38
                    radius: Theme.radiusSm
                    color: Theme.accentSoft
                    AppIcon {
                        anchors.centerIn: parent
                        width: 19
                        height: 19
                        name: "folders"
                        strokeWidth: 1.8
                        iconColor: Theme.accentText
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        text: "Random sources"
                        color: Theme.text
                        font.pixelSize: Theme.textSection
                        font.weight: Font.DemiBold
                    }
                    Text {
                        text: "Choose one or more folders. Nested folders are included."
                        color: Theme.muted
                        font.pixelSize: Theme.textXs
                    }
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 12
                Layout.rightMargin: 10
                Layout.topMargin: 10
                Layout.bottomMargin: 10
                spacing: 6
                AppField {
                    id: randomFolderSearch
                    Layout.fillWidth: true
                    iconName: "search"
                    placeholderText: "Search by folder name or path"
                    Accessible.name: "Search random source folders"
                    onTextChanged: randomFolderSearchTimer.restart()
                }
                AppButton {
                    visible: randomFolderSearch.text.length > 0
                    text: "Clear folder search"
                    iconName: "close"
                    kind: "ghost"
                    compact: true
                    onClicked: {
                        randomFolderSearch.text = ""
                        randomFolderSearch.forceActiveFocus()
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 54
                color: allFoldersChoice.hovered || allFoldersChoice.activeFocus
                    ? Theme.active
                    : "transparent"

                AppCheckBox {
                    id: allFoldersChoice
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    text: "All folders"
                    checked: controller.randomFolderSelectionCount === 0
                    focusPolicy: Qt.StrongFocus
                    Accessible.name: "Use all folders for random selection"
                    onClicked: controller.clearRandomFolders()
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 12
                Layout.rightMargin: 10
                Layout.topMargin: 6
                Layout.bottomMargin: 6
                spacing: 8

                AppButton {
                    text: "Selected only"
                    iconName: randomSourcePopup.selectedOnly ? "check" : ""
                    kind: randomSourcePopup.selectedOnly
                        ? "secondary" : "ghost"
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
                        + " of "
                        + randomFolderModel.totalCount.toLocaleString()
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                }
                AppButton {
                    text: "Clear selected"
                    kind: "ghost"
                    enabled: controller.randomFolderSelectionCount > 0
                    onClicked: controller.clearRandomFolders()
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: randomFolderList
                    anchors.fill: parent
                    anchors.topMargin: 4
                    anchors.bottomMargin: 4
                    clip: true
                    reuseItems: true
                    spacing: 2
                    model: randomFolderModel
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: AppScrollBar { }

                    delegate: Rectangle {
                        id: randomFolderChoice
                        required property string folderPath
                        required property string folderName
                        required property string folderDetail
                        required property int videoCount
                        required property bool folderSelected
                        width: ListView.view.width
                        height: 62
                        activeFocusOnTab: true
                        Accessible.role: Accessible.CheckBox
                        Accessible.name: folderName + ", " + folderDetail
                            + ", " + videoCount + " videos"
                        Accessible.checked: folderSelected
                        radius: Theme.radiusSm
                        color: folderSelected
                            ? Theme.accentSoft
                            : randomFolderHover.hovered || activeFocus
                                ? Theme.active : "transparent"
                        border.width: activeFocus ? Theme.focusWidth : 0
                        border.color: Theme.accent

                        function toggleFolder() {
                            controller.setRandomFolderEnabled(
                                randomFolderChoice.folderPath,
                                !randomFolderChoice.folderSelected
                            )
                        }

                        Rectangle {
                            id: folderIndicator
                            anchors.left: parent.left
                            anchors.leftMargin: 16
                            anchors.verticalCenter: parent.verticalCenter
                            width: 21
                            height: 21
                            radius: 5
                            color: randomFolderChoice.folderSelected
                                ? Theme.accent : Theme.raised
                            border.width: 1
                            border.color: randomFolderChoice.folderSelected
                                ? Theme.accent : Theme.borderStrong
                            AppIcon {
                                anchors.centerIn: parent
                                width: 15
                                height: 15
                                visible: randomFolderChoice.folderSelected
                                name: "check"
                                strokeWidth: 2.4
                                iconColor: Theme.accentContent
                            }
                        }

                        Column {
                            anchors.left: folderIndicator.right
                            anchors.leftMargin: 12
                            anchors.right: folderCount.left
                            anchors.rightMargin: 14
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 3
                            Text {
                                width: parent.width
                                text: randomFolderChoice.folderName
                                color: Theme.text
                                font.pixelSize: Theme.textSm
                                font.weight: Font.DemiBold
                                elide: Text.ElideMiddle
                            }
                            Text {
                                width: parent.width
                                text: randomFolderChoice.folderDetail
                                color: Theme.muted
                                font.pixelSize: Theme.textXs
                                elide: Text.ElideMiddle
                            }
                        }

                        Text {
                            id: folderCount
                            anchors.right: parent.right
                            anchors.rightMargin: 16
                            anchors.verticalCenter: parent.verticalCenter
                            text: randomFolderChoice.videoCount.toLocaleString()
                                + (randomFolderChoice.videoCount === 1
                                    ? " video" : " videos")
                            color: randomFolderChoice.folderSelected
                                ? Theme.accentText : Theme.muted
                            font.pixelSize: Theme.textXs
                            font.weight: randomFolderChoice.folderSelected
                                ? Font.DemiBold : Font.Normal
                        }

                        HoverHandler { id: randomFolderHover }
                        TapHandler {
                            onTapped: {
                                randomFolderChoice.forceActiveFocus()
                                randomFolderChoice.toggleFolder()
                            }
                        }
                        Keys.onSpacePressed:
                            randomFolderChoice.toggleFolder()
                        Keys.onReturnPressed:
                            randomFolderChoice.toggleFolder()
                    }
                }

                Column {
                    visible: controller.randomFolderOptionsLoading
                        && randomFolderModel.totalCount === 0
                    anchors.centerIn: parent
                    width: Math.min(220, parent.width - 48)
                    spacing: 10
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Loading folder sources…"
                        color: Theme.muted
                        font.pixelSize: Theme.textSm
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
                        ? "Folders will appear as filenames are found."
                        : "Rescan the library to build the folder list."
                    color: Theme.muted
                    font.pixelSize: Theme.textSm
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
                        ? "No selected folders match this search."
                        : "No folders match this search."
                    color: Theme.muted
                    font.pixelSize: Theme.textSm
                    wrapMode: Text.Wrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 16
                Layout.rightMargin: 10
                Layout.topMargin: 7
                Layout.bottomMargin: 7
                spacing: 8
                Text {
                    Layout.fillWidth: true
                    text: controller.randomFolderSelectionCount === 0
                        ? "Using the whole library"
                        : controller.randomFolderSelectionCount
                            + (controller.randomFolderSelectionCount === 1
                                ? " folder selected"
                                : " folders selected")
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                }
                AppButton {
                    text: "Done"
                    kind: "secondary"
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
            Layout.preferredHeight: 82
            Layout.leftMargin: 26; Layout.rightMargin: 24
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text { text: "Video library"; color: Theme.text; font.pixelSize: Theme.textTitle; font.weight: Font.DemiBold }
                Text {
                    text: controller.settings.library_root ? controller.settings.library_root : "Choose a folder to begin"
                    color: Theme.muted; font.pixelSize: Theme.textXs; elide: Text.ElideMiddle; Layout.fillWidth: true
                }
            }
            AppButton {
                text: root.showFolders ? "Hide folders" : "Show folders"
                iconName: "folders"
                compact: root.compactToolbar
                toolTipText: text
                kind: root.showFolders ? "secondary" : "ghost"
                onClicked: root.showFolders = !root.showFolders
            }
            AppButton {
                text: "Choose folder"
                iconName: "folder"
                compact: root.compactToolbar
                toolTipText: text
                kind: "ghost"
                onClicked: libraryDialog.open()
            }
            AppButton {
                text: "Previous video"
                iconName: "chevronLeft"
                compact: true
                toolTipText: "Previous video  ·  ←"
                kind: "ghost"
                enabled: controller.canSelectPrevious
                onClicked: controller.navigateSelection(-1)
            }
            AppButton {
                text: "Next video"
                iconName: "chevronRight"
                compact: true
                toolTipText: "Next video  ·  →"
                kind: "ghost"
                enabled: controller.canSelectNext
                onClicked: controller.navigateSelection(1)
            }
            AppButton {
                id: randomSourceButton
                Layout.preferredWidth: root.compactToolbar ? 146 : 180
                Layout.maximumWidth: root.compactToolbar ? 146 : 180
                text: (root.compactToolbar ? "" : "From: ")
                    + controller.randomFolderSummary
                iconName: "chevronDown"
                kind: controller.randomFolderSelectionCount > 0 ? "secondary" : "ghost"
                enabled: Boolean(controller.settings.library_root)
                onClicked: randomSourcePopup.open()
            }
            AppButton {
                text: "Previous random"
                iconName: "history"
                compact: true
                toolTipText: "Previous random  ·  Shift+R"
                kind: "ghost"
                enabled: controller.canPickPreviousRandom
                onClicked: controller.pickPreviousRandom()
            }
            AppButton {
                text: controller.randomPicking ? "Picking…" : "Pick random"
                iconName: "shuffle"
                kind: "primary"
                compact: root.width < 900
                toolTipText: "Pick random  ·  R"
                enabled: !controller.randomPicking
                    && (controller.settings.fast_random
                        ? Boolean(controller.settings.library_root)
                        : controller.counts.media > 0)
                onClicked: controller.pickRandom()
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Rectangle {
                visible: root.showFolders && controller.settings.library_root
                Layout.fillHeight: true
                Layout.preferredWidth: 210
                color: Theme.surface

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 8
                    Text { text: "FOLDERS"; color: Theme.muted; font.pixelSize: Theme.textXs; font.letterSpacing: 1.2; Layout.leftMargin: 6; Layout.topMargin: 5 }
                    ListView {
                        id: folderList
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 3
                        clip: true
                        model: folderModel
                        delegate: Rectangle {
                            id: folderRow
                            required property string folderPath
                            required property string folderName
                            required property int videoCount
                            width: ListView.view.width
                            height: 42
                            activeFocusOnTab: true
                            Accessible.role: Accessible.ListItem
                            Accessible.name: folderName + ", " + videoCount + " videos"
                            radius: Theme.radiusSm
                            color: root.currentFolder === folderPath ? Theme.active : (folderHover.hovered || activeFocus ? Theme.raised : "transparent")
                            border.width: activeFocus ? 2 : 0
                            border.color: Theme.accent
                            function chooseFolder() {
                                root.currentFolder = folderRow.folderPath
                                controller.setFolder(folderRow.folderPath)
                            }
                            RowLayout {
                                anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 9
                                AppIcon {
                                    Layout.preferredWidth: 16
                                    Layout.preferredHeight: 16
                                    name: "folder"
                                    strokeWidth: 1.7
                                    iconColor: root.currentFolder === folderRow.folderPath
                                        ? Theme.accent : Theme.muted
                                }
                                Text { text: folderRow.folderName; color: Theme.text; font.pixelSize: Theme.textSm; elide: Text.ElideMiddle; Layout.fillWidth: true }
                                Text { text: folderRow.videoCount; color: Theme.muted; font.pixelSize: Theme.textXs }
                            }
                            HoverHandler { id: folderHover }
                            TapHandler {
                                onTapped: {
                                    folderRow.forceActiveFocus()
                                    folderRow.chooseFolder()
                                }
                            }
                            Keys.onSpacePressed: folderRow.chooseFolder()
                            Keys.onReturnPressed: folderRow.chooseFolder()
                        }
                    }
                    AppButton {
                        text: "Reset shuffle"
                        iconName: "refresh"
                        kind: "ghost"
                        Layout.fillWidth: true
                        onClicked: controller.resetShuffle()
                    }
                }
            }

            Rectangle { visible: root.showFolders && controller.settings.library_root; Layout.fillHeight: true; width: 1; color: Theme.border }

            ColumnLayout {
                id: libraryColumn
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                GridLayout {
                    id: libraryToolbar
                    property bool compact: libraryColumn.width < 520
                    visible: controller.settings.library_root
                    Layout.fillWidth: true
                    Layout.preferredHeight: compact ? 108 : 64
                    Layout.leftMargin: 20; Layout.rightMargin: 20
                    columns: compact ? 2 : 3
                    columnSpacing: 10
                    rowSpacing: 8
                    AppField {
                        id: searchField
                        Layout.fillWidth: true
                        Layout.columnSpan: libraryToolbar.compact ? 2 : 1
                        iconName: "search"
                        placeholderText: "Search filenames and folders"
                        Accessible.name: "Search video library"
                        onTextChanged: {
                            if (!root.syncingReveal) searchTimer.restart()
                        }
                    }
                    AppComboBox {
                        id: sortBox
                        Layout.fillWidth: libraryToolbar.compact
                        Layout.preferredWidth: libraryToolbar.compact ? 120 : 138
                        model: ["Newest", "Oldest", "Name", "Duration", "Size"]
                        currentIndex: ["newest", "oldest", "name", "duration", "size"].indexOf(controller.settings.sort_mode)
                        onActivated: controller.setSetting("sort_mode", ["newest", "oldest", "name", "duration", "size"][currentIndex])
                        Accessible.name: "Sort videos"
                    }
                    AppButton {
                        text: "Rescan"
                        iconName: "refresh"
                        kind: "ghost"
                        Layout.fillWidth: libraryToolbar.compact
                        enabled: !controller.scanning
                        onClicked: controller.scanLibrary()
                    }
                }
                Timer { id: searchTimer; interval: 180; onTriggered: controller.setSearch(searchField.text) }

                ColumnLayout {
                    visible: controller.scanning
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
                        visible: controller.settings.library_root && !controller.scanning && libraryGrid.count === 0
                        anchors.centerIn: parent
                        width: Math.min(420, parent.width - 48)
                        iconName: searchField.text.length ? "search" : "media"
                        title: searchField.text.length ? "No matching videos" : "No videos found"
                        body: searchField.text.length
                            ? "Try a broader search or switch folders."
                            : (controller.settings.auto_index
                                ? "Rescan the library, or enable deep format detection for uncommon files."
                                : "Background indexing is off. Pick randomly now, or rescan to build the visual library.")
                    }

                    GridView {
                        id: libraryGrid
                        visible: controller.settings.library_root && count > 0
                        anchors.fill: parent
                        anchors.leftMargin: 20; anchors.rightMargin: 14; anchors.topMargin: 4; anchors.bottomMargin: 12
                        clip: true
                        model: libraryModel
                        property int columns: Math.max(1, Math.floor(width / 236))
                        cellWidth: width / columns
                        cellHeight: 220
                        cacheBuffer: cellHeight * 2
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
                            MediaTile {
                                anchors.fill: parent
                                anchors.rightMargin: 12
                                anchors.bottomMargin: 8
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
                                onChosen: controller.selectMedia(mediaId)
                                onPlaybackRequested: root.playMedia(mediaId)
                                onNavigationRequested:
                                    controller.navigateSelection(direction)
                            }
                        }
                    }
                }
            }

            Rectangle {
                visible: controller.selectedMediaId > 0
                    && !root.prepareFullscreen
                Layout.fillHeight: true
                width: 1
                color: Theme.border
            }

            Item {
                id: prepareSlot
                visible: controller.selectedMediaId > 0
                    && !root.prepareFullscreen
                Layout.fillHeight: true
                Layout.preferredWidth: controller.settings.prepare_expanded
                    ? Math.min(
                        680,
                        Math.max(
                            Math.min(420, Math.max(360, root.width * 0.34)),
                            root.width - (root.showFolders ? 210 : 0) - 460
                        )
                    )
                    : Math.min(420, Math.max(360, root.width * 0.34))
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
        onFullScreenRequested: root.prepareFullscreen = true
        onFullScreenExitRequested: root.prepareFullscreen = false
        onCloseRequested: {
            root.prepareFullscreen = false
            controller.clearSelection()
        }
    }
}
