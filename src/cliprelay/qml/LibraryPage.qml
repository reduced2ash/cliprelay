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
        objectName: "randomSourcePopup"
        parent: Overlay.overlay
        property bool selectedOnly: false
        property double lastClosedAt: 0
        readonly property real edgeInset: 12
        readonly property point triggerTop: parent
            ? randomSourceButton.mapToItem(parent, 0, 0)
            : Qt.point(0, 0)
        readonly property point triggerBottomRight: parent
            ? randomSourceButton.mapToItem(
                parent,
                randomSourceButton.width,
                randomSourceButton.height
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
            412,
            Math.max(0, (parent ? parent.width : root.width) - edgeInset * 2)
        )
        height: Math.min(
            492,
            Math.max(0, (parent ? parent.height : root.height) - edgeInset * 2)
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
            radius: Theme.radiusMd
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 44
                Layout.leftMargin: 12
                Layout.rightMargin: 7
                spacing: 8

                AppIcon {
                    Layout.preferredWidth: 16
                    Layout.preferredHeight: 16
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
                            + " SCOPE"
                            + (randomFolderModel.totalCount === 1 ? "" : "S")
                    color: controller.randomFolderOptionsLoading
                        ? Theme.warning : Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.6
                }
                AppButton {
                    text: "Close random sources"
                    iconName: "close"
                    kind: "ghost"
                    compact: true
                    Layout.preferredWidth: 40
                    Layout.preferredHeight: 36
                    onClicked: randomSourcePopup.close()
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 9
                Layout.rightMargin: 7
                Layout.topMargin: 7
                Layout.bottomMargin: 7
                spacing: 6
                AppField {
                    id: randomFolderSearch
                    Layout.fillWidth: true
                    Layout.preferredHeight: 36
                    iconName: "search"
                    placeholderText: "Filter name or path"
                    Accessible.name: "Search random source folders"
                    onTextChanged: randomFolderSearchTimer.restart()
                }
                AppButton {
                    visible: randomFolderSearch.text.length > 0
                    text: "Clear folder search"
                    iconName: "close"
                    kind: "ghost"
                    compact: true
                    Layout.preferredWidth: 40
                    Layout.preferredHeight: 36
                    onClicked: {
                        randomFolderSearch.text = ""
                        randomFolderSearch.forceActiveFocus()
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                id: allFoldersChoice
                Layout.preferredHeight: 40
                activeFocusOnTab: true
                Accessible.role: Accessible.CheckBox
                Accessible.name: "Select every folder for random selection"
                Accessible.checked: controller.allRandomFoldersSelected
                color: controller.allRandomFoldersSelected
                    ? Theme.accentSoft
                    : allFoldersHover.hovered || activeFocus
                        ? Theme.active : "transparent"
                border.width: activeFocus ? Theme.focusWidth : 0
                border.color: Theme.accent

                function selectAll() {
                    controller.selectAllRandomFolders()
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.leftMargin: 11
                    anchors.verticalCenter: parent.verticalCenter
                    width: 17
                    height: 17
                    radius: 4
                    color: controller.allRandomFoldersSelected
                        ? Theme.accent : Theme.raised
                    border.width: 1
                    border.color: controller.allRandomFoldersSelected
                        ? Theme.accent : Theme.borderStrong
                    AppIcon {
                        anchors.centerIn: parent
                        width: 13
                        height: 13
                        visible: controller.allRandomFoldersSelected
                        name: "check"
                        strokeWidth: 2.3
                        iconColor: Theme.accentContent
                    }
                }
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 38
                    anchors.verticalCenter: parent.verticalCenter
                    text: "All folders"
                    color: Theme.text
                    font.pixelSize: Theme.textSm
                    font.weight: Font.DemiBold
                }
                Text {
                    anchors.right: parent.right
                    anchors.rightMargin: 11
                    anchors.verticalCenter: parent.verticalCenter
                    text: controller.allRandomFoldersSelected
                        ? "ACTIVE"
                        : "SELECT ALL"
                    color: controller.allRandomFoldersSelected
                        ? Theme.accentText : Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.6
                }
                HoverHandler { id: allFoldersHover }
                TapHandler { onTapped: allFoldersChoice.selectAll() }
                Keys.onSpacePressed: allFoldersChoice.selectAll()
                Keys.onReturnPressed: allFoldersChoice.selectAll()
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 8
                Layout.rightMargin: 8
                Layout.topMargin: 4
                Layout.bottomMargin: 4
                spacing: 6

                AppButton {
                    text: "Selected only"
                    kind: randomSourcePopup.selectedOnly
                        ? "secondary" : "ghost"
                    Layout.preferredHeight: 32
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
                        + " / "
                        + randomFolderModel.totalCount.toLocaleString()
                    color: Theme.muted
                    font.pixelSize: 11
                }
                AppButton {
                    text: "Clear"
                    kind: "ghost"
                    enabled: controller.randomFolderSelectionCount > 0
                    Layout.preferredHeight: 32
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
                    spacing: 0
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
                        height: 48
                        clip: true
                        activeFocusOnTab: true
                        Accessible.role: Accessible.CheckBox
                        Accessible.name: folderName + ", " + folderDetail
                            + ", " + videoCount + " videos"
                        Accessible.checked: folderSelected
                        color: randomFolderHover.hovered || activeFocus
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
                            anchors.leftMargin: 11
                            anchors.verticalCenter: parent.verticalCenter
                            width: 17
                            height: 17
                            radius: 4
                            color: randomFolderChoice.folderSelected
                                ? Theme.accent : Theme.raised
                            border.width: 1
                            border.color: randomFolderChoice.folderSelected
                                ? Theme.accent : Theme.borderStrong
                            AppIcon {
                                anchors.centerIn: parent
                                width: 13
                                height: 13
                                visible: randomFolderChoice.folderSelected
                                name: "check"
                                strokeWidth: 2.3
                                iconColor: Theme.accentContent
                            }
                        }

                        Column {
                            anchors.left: folderIndicator.right
                            anchors.leftMargin: 9
                            anchors.right: folderCount.left
                            anchors.rightMargin: 10
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 1
                            Text {
                                id: folderNameLabel
                                width: parent.width
                                text: randomFolderChoice.folderName
                                color: Theme.text
                                font.pixelSize: Theme.textSm
                                font.weight: Font.DemiBold
                                elide: Text.ElideMiddle
                            }
                            Text {
                                id: folderDetailLabel
                                width: parent.width
                                text: randomFolderChoice.folderDetail
                                color: Theme.muted
                                font.pixelSize: 11
                                elide: Text.ElideMiddle
                            }
                        }

                        Text {
                            id: folderCount
                            anchors.right: parent.right
                            anchors.rightMargin: 11
                            anchors.verticalCenter: parent.verticalCenter
                            width: 62
                            horizontalAlignment: Text.AlignRight
                            text: randomFolderChoice.videoCount.toLocaleString()
                                + " VID"
                            color: randomFolderChoice.folderSelected
                                ? Theme.accentText : Theme.muted
                            font.pixelSize: 10
                            font.weight: randomFolderChoice.folderSelected
                                ? Font.DemiBold : Font.Normal
                            font.letterSpacing: 0.4
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
                        ToolTip.visible: randomFolderHover.hovered
                            && (folderNameLabel.truncated
                                || folderDetailLabel.truncated)
                        ToolTip.text: randomFolderChoice.folderPath.length
                            ? randomFolderChoice.folderPath
                            : "Files directly inside the library root"
                        ToolTip.delay: 500
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
                        text: "Indexing folder scopes…"
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
                        ? "Folder scopes appear as filenames are found."
                        : "Rescan the library to build folder scopes."
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
                        ? "No selected folders match this search."
                        : "No folders match this search."
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    wrapMode: Text.Wrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 11
                Layout.rightMargin: 7
                Layout.topMargin: 4
                Layout.bottomMargin: 4
                spacing: 8
                Text {
                    Layout.fillWidth: true
                    text: controller.randomFolderSelectionCount === 0
                        ? "No random sources selected"
                        : controller.allRandomFoldersSelected
                            ? "Whole library  ·  "
                                + randomFolderModel.totalCount.toLocaleString()
                                + " scopes"
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
                    Layout.preferredHeight: 32
                    Layout.preferredWidth: 82
                    Layout.maximumWidth: 82
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
                Layout.minimumWidth: 0
                text: (root.compactToolbar ? "" : "From: ")
                    + controller.randomFolderSummary
                iconName: randomSourcePopup.opened
                    ? "chevronUp" : "chevronDown"
                kind: controller.hasRandomFolderSelection
                    ? "secondary" : "ghost"
                toolTipText: "Random sources: "
                    + controller.randomFolderSummary
                enabled: Boolean(controller.settings.library_root)
                onClicked: root.toggleRandomSourcePopup()
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
                    && controller.hasRandomFolderSelection
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
