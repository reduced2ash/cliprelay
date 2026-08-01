import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    required property var actionRegistry
    required property var appController
    required property var folderTreeModel
    required property var libraryPage
    property int activityWidth: 68
    property int explorerWidth: 204
    property real prepareWidth: 0
    property int currentPage: 0
    property bool activityCollapsed: true
    property bool showFolders: true
    property bool showPrepare: false
    property string currentFolder: ""
    property string libraryRoot: ""
    readonly property bool libraryPageActive: currentPage === 0
    readonly property string pageName: currentPage === 1
        ? "History" : currentPage === 2 ? "Settings" : "Library"
    readonly property string pageIcon: currentPage === 1
        ? "history" : currentPage === 2 ? "settings" : "library"
    readonly property string pageDetail: currentPage === 1
        ? "Recorded relays and delivery outcomes"
        : currentPage === 2
            ? "Application preferences and integrations"
            : ""
    readonly property bool hasLibrary: libraryRoot.length > 0
    readonly property bool showExplorer:
        libraryPageActive && hasLibrary && showFolders
    readonly property bool showPrepareContext:
        libraryPageActive && showPrepare && prepareWidth > 0
    readonly property bool compactActions:
        libraryContextSlot.width < 680
    readonly property bool narrowActions:
        libraryContextSlot.width < 500
    readonly property string libraryName: {
        if (!libraryRoot.length)
            return "No library selected"
        const normalized = libraryRoot.replace(/\\/g, "/")
        const parts = normalized.split("/")
        return parts.length ? parts[parts.length - 1] : libraryRoot
    }
    readonly property string locationName: {
        if (!currentFolder.length)
            return libraryName
        const normalized = currentFolder.replace(/\\/g, "/")
        const parts = normalized.split("/")
        return parts.length ? parts[parts.length - 1] : currentFolder
    }

    height: Theme.workbenchContextHeight
    color: Theme.surface

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: root.activityWidth
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: root.activityCollapsed ? 0 : 12
                anchors.rightMargin: root.activityCollapsed ? 0 : 10
                spacing: 8
                Item {
                    visible: root.activityCollapsed
                    Layout.fillWidth: true
                }
                AppIcon {
                    Layout.preferredWidth: 16
                    Layout.preferredHeight: 16
                    name: root.pageIcon
                    strokeWidth: 1.75
                    iconColor: Theme.accentText
                }
                Text {
                    visible: !root.activityCollapsed
                    Layout.fillWidth: true
                    text: root.pageName.toUpperCase()
                    color: Theme.textSoft
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.8
                }
                Item {
                    visible: root.activityCollapsed
                    Layout.fillWidth: true
                }
            }
        }

        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 1
            color: Theme.border
        }

        Rectangle {
            visible: root.showExplorer
            Layout.fillHeight: true
            Layout.preferredWidth: root.showExplorer
                ? root.explorerWidth : 0
            color: Theme.surfaceSoft

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 5
                spacing: 7
                AppIcon {
                    Layout.preferredWidth: 15
                    Layout.preferredHeight: 15
                    name: "folders"
                    strokeWidth: 1.75
                    iconColor: Theme.muted
                }
                Text {
                    Layout.fillWidth: true
                    text: "EXPLORER"
                    color: Theme.textSoft
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.8
                }
                Text {
                    text: root.folderTreeModel.totalCount.toLocaleString()
                    color: Theme.mutedSoft
                    font.pixelSize: 10
                    font.family: Theme.monoFamily
                    font.weight: Font.Medium
                }
                WorkbenchButton {
                    text: "Collapse all folders"
                    iconName: "chevronUp"
                    iconOnly: true
                    toolTipText: text
                    kind: "ghost"
                    Layout.preferredWidth: 28
                    Layout.preferredHeight: 28
                    onClicked: root.folderTreeModel.collapseAll()
                }
            }
        }

        Rectangle {
            visible: root.showExplorer
            Layout.fillHeight: true
            Layout.preferredWidth: root.showExplorer ? 1 : 0
            color: Theme.border
        }

        Item {
            id: libraryContextSlot
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            RowLayout {
                visible: root.libraryPageActive
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 9
                spacing: 7

                AppIcon {
                    Layout.preferredWidth: 15
                    Layout.preferredHeight: 15
                    name: "folder"
                    strokeWidth: 1.7
                    iconColor: root.hasLibrary
                        ? Theme.accentText : Theme.muted
                }
                Text {
                    visible: !root.narrowActions
                    text: "Video library"
                    color: Theme.accentText
                    font.pixelSize: Theme.textWorkbench
                    font.weight: Font.DemiBold
                }
                Text {
                    visible: !root.narrowActions && root.hasLibrary
                    text: "/"
                    color: Theme.mutedSoft
                    font.pixelSize: Theme.textWorkbench
                    font.family: Theme.monoFamily
                }
                Text {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 42
                    text: root.locationName
                    color: root.hasLibrary ? Theme.textSoft : Theme.muted
                    font.pixelSize: Theme.textWorkbench
                    font.family: Theme.monoFamily
                    elide: Text.ElideMiddle
                }

                WorkbenchButton {
                    visible: root.hasLibrary
                    text: root.showFolders
                        ? "Hide folders" : "Show folders"
                    iconName: "panel"
                    iconOnly: root.compactActions
                    toolTipText: text
                    kind: "ghost"
                    enabled: root.actionRegistry.action(
                        "toggle_folders"
                    ).enabled
                    onClicked:
                        root.actionRegistry.triggerAction("toggle_folders")
                }
                WorkbenchButton {
                    text: "Choose root"
                    iconName: "folder"
                    iconOnly: root.compactActions
                    toolTipText: "Choose library root"
                    kind: "ghost"
                    onClicked:
                        root.actionRegistry.triggerAction("choose_folder")
                }

                Rectangle {
                    visible: !root.narrowActions
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 22
                    color: Theme.border
                }

                LibrarySortControl {
                    visible: root.hasLibrary
                    Layout.preferredWidth: root.narrowActions ? 96 : 112
                    actionRegistry: root.actionRegistry
                    appController: root.appController
                }

                Rectangle {
                    visible: !root.narrowActions && root.hasLibrary
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 22
                    color: Theme.border
                }

                WorkbenchButton {
                    visible: root.hasLibrary
                    text: root.appController.activeWorkspaceScanning
                        ? (root.appController.scanCancelling
                            ? "Stopping…" : "Stop scan")
                        : "Rescan"
                    iconName: root.appController.activeWorkspaceScanning
                        ? "square" : "refresh"
                    iconOnly: root.compactActions
                    toolTipText:
                        root.appController.activeWorkspaceScanning
                            ? root.appController.scanMessage
                            : root.appController.scanning
                                ? "Stop the other library scan and scan this root"
                                : "Rescan library"
                    kind: "ghost"
                    enabled: root.appController.activeWorkspaceScanning
                        ? !root.appController.scanCancelling
                        : root.actionRegistry.action("rescan").enabled
                    onClicked: {
                        if (root.appController.activeWorkspaceScanning)
                            root.appController.cancelScan()
                        else
                            root.actionRegistry.triggerAction("rescan")
                    }
                }
            }

            RowLayout {
                visible: !root.libraryPageActive
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 7

                AppIcon {
                    Layout.preferredWidth: 15
                    Layout.preferredHeight: 15
                    name: root.pageIcon
                    strokeWidth: 1.7
                    iconColor: Theme.accentText
                }
                Text {
                    text: root.pageName
                    color: Theme.accentText
                    font.pixelSize: Theme.textWorkbench
                    font.weight: Font.DemiBold
                }
                Text {
                    text: "/"
                    color: Theme.mutedSoft
                    font.pixelSize: Theme.textWorkbench
                    font.family: Theme.monoFamily
                }
                Text {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    text: root.pageDetail
                    color: Theme.muted
                    font.pixelSize: Theme.textWorkbench
                    elide: Text.ElideRight
                }
            }
        }

        Rectangle {
            visible: root.showPrepareContext
            Layout.fillHeight: true
            Layout.preferredWidth: root.showPrepareContext ? 1 : 0
            color: Theme.border
        }

        Rectangle {
            visible: root.showPrepareContext
            Layout.fillHeight: true
            Layout.preferredWidth: root.showPrepareContext
                ? root.prepareWidth : 0
            Layout.minimumWidth: root.showPrepareContext ? 360 : 0
            Layout.maximumWidth: 680
            color: Theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 7
                spacing: 3

                AppIcon {
                    Layout.preferredWidth: 15
                    Layout.preferredHeight: 15
                    name: "edit"
                    strokeWidth: 1.7
                    iconColor: Theme.accentText
                }
                Text {
                    text: "Prepare"
                    color: Theme.text
                    font.pixelSize: Theme.textWorkbench
                    font.weight: Font.DemiBold
                }
                Text {
                    visible: root.prepareWidth >= 470
                    text: "/"
                    color: Theme.mutedSoft
                    font.pixelSize: Theme.textWorkbench
                }
                Text {
                    visible: root.prepareWidth >= 470
                    Layout.fillWidth: true
                    Layout.minimumWidth: 36
                    text: root.appController.selectedMedia.name || ""
                    color: Theme.textSoft
                    font.pixelSize: Theme.textWorkbench
                    font.weight: Font.Medium
                    elide: Text.ElideMiddle
                }
                Item {
                    visible: root.prepareWidth < 470
                    Layout.fillWidth: true
                }
                AppIcon {
                    visible: root.libraryPage.prepareHasEdits
                    Layout.preferredWidth: 14
                    Layout.preferredHeight: 14
                    name: "crop"
                    strokeWidth: 1.8
                    iconColor: Theme.accentText
                    ToolTip.visible: editedHover.hovered
                    ToolTip.text: "Frame edits active"
                    HoverHandler { id: editedHover }
                }
                WorkbenchButton {
                    text: "Open in default player"
                    iconName: "external"
                    iconOnly: true
                    toolTipText: text
                    kind: "ghost"
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    onClicked:
                        root.libraryPage.openSelectedInDefaultPlayer()
                }
                WorkbenchButton {
                    text: root.appController.settings.prepare_expanded
                        ? "Narrow Prepare" : "Widen Prepare"
                    iconName: root.appController.settings.prepare_expanded
                        ? "contract" : "expand"
                    iconOnly: true
                    toolTipText: text
                    kind: "ghost"
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    onClicked: root.libraryPage.togglePrepareWidth()
                }
                WorkbenchButton {
                    text: "Open full-screen editor"
                    iconName: "maximize"
                    iconOnly: true
                    toolTipText: text
                    kind: "ghost"
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    onClicked: root.libraryPage.openPrepareFullscreen()
                }
                WorkbenchButton {
                    text: "Close selected video"
                    iconName: "close"
                    iconOnly: true
                    toolTipText: text
                    kind: "ghost"
                    Layout.preferredWidth: 30
                    Layout.preferredHeight: 30
                    onClicked: root.libraryPage.closePrepare()
                }
            }
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.border
    }
}
