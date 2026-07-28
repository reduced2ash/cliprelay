import QtQuick
import QtQuick.Layouts
import "."

Item {
    id: root

    required property var hostWindow
    required property var windowController
    required property var appController
    required property var actionRegistry
    required property var libraryPage
    readonly property bool macStyle: Qt.platform.os === "osx"
    readonly property bool compact: width < 1120
    readonly property bool veryCompact: width < 1000
    readonly property alias randomSourceButtonItem: randomSourceButton
    readonly property bool commandCenterOpen: commandCenter.popupOpen

    height: Theme.workbenchTitleHeight

    function focusSearch() {
        commandCenter.focusSearch()
    }

    function focusCommands() {
        commandCenter.focusCommands()
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.ink
    }

    MouseArea {
        id: moveArea
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        preventStealing: true

        onPressed: function(mouse) {
            mouse.accepted = root.windowController.beginMove()
        }
        onPositionChanged: function(mouse) {
            if (pressed)
                root.windowController.updateMove()
        }
        onReleased: root.windowController.endMove()
        onCanceled: root.windowController.endMove()
        onDoubleClicked: function(mouse) {
            root.windowController.endMove()
            root.windowController.toggleZoom()
            mouse.accepted = true
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: root.macStyle ? 116 : 8
        anchors.rightMargin: root.macStyle ? 8 : 142
        spacing: 5
        opacity: root.windowController.active ? 1 : 0.76

        WorkbenchButton {
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            text: "Open command palette"
            iconName: "command"
            iconOnly: true
            kind: "primary"
            toolTipText: "Command palette  ·  "
                + (root.macStyle ? "⌘⇧P" : "Ctrl+Shift+P")
            onClicked: root.focusCommands()
        }

        Item { Layout.preferredWidth: 2 }

        WorkbenchButton {
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            text: root.actionRegistry.action("previous_video").label
            iconName: "chevronLeft"
            iconOnly: true
            toolTipText: "Previous video  ·  ←"
            kind: "ghost"
            enabled: root.actionRegistry.action("previous_video").enabled
            onClicked:
                root.actionRegistry.triggerAction("previous_video")
        }
        WorkbenchButton {
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            text: root.actionRegistry.action("next_video").label
            iconName: "chevronRight"
            iconOnly: true
            toolTipText: "Next video  ·  →"
            kind: "ghost"
            enabled: root.actionRegistry.action("next_video").enabled
            onClicked:
                root.actionRegistry.triggerAction("next_video")
        }

        Item {
            Layout.preferredWidth: root.compact ? 2 : 8
        }

        CommandCenter {
            id: commandCenter
            Layout.fillWidth: true
            Layout.minimumWidth: root.veryCompact ? 230 : 280
            Layout.maximumWidth: 860
            Layout.preferredHeight: 30
            appController: root.appController
            actionRegistry: root.actionRegistry
            externalMediaQuery: root.libraryPage.searchText
            onSearchRequested: function(query) {
                root.hostWindow.currentPage = 0
                root.libraryPage.setSearchText(query)
            }
            onMediaRequested: function(mediaId) {
                root.hostWindow.currentPage = 0
                root.libraryPage.selectSearchResult(mediaId)
            }
            onFolderRequested: function(folderPath) {
                root.hostWindow.currentPage = 0
                root.libraryPage.selectSearchFolder(folderPath)
            }
        }

        Item {
            Layout.preferredWidth: root.compact ? 2 : 7
        }

        WorkbenchActivityButton {
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            appController: root.appController
        }

        WorkbenchButton {
            id: randomSourceButton
            Layout.preferredWidth: root.veryCompact
                ? 116 : root.compact ? 132 : 150
            Layout.maximumWidth: Layout.preferredWidth
            Layout.minimumWidth: 76
            Layout.preferredHeight: 30
            text: root.appController.randomFolderSummary
            iconName: "folder"
            trailingIconName: root.libraryPage.randomSourcesOpen
                ? "chevronUp" : "chevronDown"
            kind: root.libraryPage.randomSourcesOpen
                || root.appController.hasRandomFolderSelection
                    ? "secondary" : "ghost"
            toolTipText: "Random sources: "
                + root.appController.randomFolderSummary
            enabled: Boolean(root.appController.settings.library_root)
            onClicked: {
                root.hostWindow.currentPage = 0
                Qt.callLater(root.libraryPage.toggleRandomSourcePopup)
            }
        }

        WorkbenchButton {
            visible: !root.compact
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            text: root.actionRegistry.action("reset_shuffle").label
            iconName: "refresh"
            iconOnly: true
            toolTipText: "Reset shuffle history"
            kind: "ghost"
            enabled: root.actionRegistry.action("reset_shuffle").enabled
            onClicked:
                root.actionRegistry.triggerAction("reset_shuffle")
        }

        WorkbenchButton {
            Layout.preferredWidth: root.veryCompact
                ? 96 : root.compact ? 112 : 128
            Layout.maximumWidth: Layout.preferredWidth
            Layout.minimumWidth: 72
            Layout.preferredHeight: 30
            text: root.appController.randomPicking
                ? "Picking…" : root.veryCompact ? "Random" : "Pick random"
            iconName: "shuffle"
            kind: "primary"
            toolTipText: "Pick random video  ·  R"
            enabled: root.actionRegistry.action("pick_random").enabled
            onClicked:
                root.actionRegistry.triggerAction("pick_random")
        }
    }

    Row {
        visible: root.macStyle
        anchors.left: parent.left
        anchors.leftMargin: 7
        anchors.verticalCenter: parent.verticalCenter
        spacing: 0
        z: 3

        WindowControlButton {
            roleName: "close"
            macStyle: true
            windowActive: root.windowController.active
            onClicked: root.windowController.closeWindow()
        }
        WindowControlButton {
            roleName: "minimize"
            macStyle: true
            windowActive: root.windowController.active
            onClicked: root.windowController.minimizeWindow()
        }
        WindowControlButton {
            roleName: "zoom"
            macStyle: true
            windowActive: root.windowController.active
            windowMaximized: root.windowController.maximized
            windowFullscreen: root.windowController.fullscreen
            onClicked: root.windowController.performPrimaryZoom()
        }
    }

    Row {
        visible: !root.macStyle
        anchors.right: parent.right
        anchors.top: parent.top
        height: parent.height
        z: 3

        WindowControlButton {
            roleName: "minimize"
            windowActive: root.windowController.active
            onClicked: root.windowController.minimizeWindow()
        }
        WindowControlButton {
            roleName: "zoom"
            windowActive: root.windowController.active
            windowMaximized: root.windowController.maximized
            onClicked: root.windowController.toggleZoom()
        }
        WindowControlButton {
            roleName: "close"
            windowActive: root.windowController.active
            onClicked: root.windowController.closeWindow()
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.border
        opacity: root.windowController.active ? 0.86 : 0.55
    }
}
