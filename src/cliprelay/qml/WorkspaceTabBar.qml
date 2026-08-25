pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    required property var appController
    required property var libraryPage

    property string renamingId: ""
    property string contextWorkspaceId: ""
    property int contextWorkspaceIndex: -1
    readonly property int tabCount: appController.workspaceCount
    readonly property real resolvedTabWidth: Math.max(
        132,
        Math.min(
            218,
            Math.floor(
                Math.max(132, tabViewport.width)
                / Math.max(1, Math.min(tabCount, 5))
            )
        )
    )

    color: Theme.surface
    implicitHeight: Theme.workspaceTabHeight
    clip: true

    function beginRename(workspaceId) {
        root.renamingId = String(workspaceId || "")
    }

    function finishRename(workspaceId, value) {
        if (root.renamingId !== workspaceId)
            return
        root.renamingId = ""
        root.appController.renameWorkspace(workspaceId, value)
    }

    function openContextMenu(workspaceId, workspaceIndex) {
        root.contextWorkspaceId = String(workspaceId || "")
        root.contextWorkspaceIndex = Number(workspaceIndex)
        tabContextMenu.popup()
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 1
        color: Theme.border
    }

    RowLayout {
        anchors.fill: parent
        anchors.topMargin: 1
        spacing: 0

        Item {
            id: tabViewport
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            ListView {
                id: tabList
                anchors.fill: parent
                orientation: ListView.Horizontal
                spacing: 0
                clip: true
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds
                model: root.appController.workspaceTabs
                currentIndex: {
                    const rows = root.appController.workspaceTabs
                    for (let index = 0; index < rows.length; ++index) {
                        if (rows[index].id
                                === root.appController.activeWorkspaceId)
                            return index
                    }
                    return 0
                }

                onCurrentIndexChanged: {
                    if (currentIndex >= 0)
                        positionViewAtIndex(currentIndex, ListView.Contain)
                }

                delegate: Rectangle {
                    id: tabDelegate
                    required property var modelData
                    required property int index

                    readonly property bool active:
                        modelData.id === root.appController.activeWorkspaceId
                    readonly property bool hovered: tabMouse.containsMouse
                    width: root.resolvedTabWidth
                    height: tabList.height
                    color: active
                        ? Theme.raised
                        : hovered ? Theme.hover : "transparent"

                    Rectangle {
                        visible: tabDelegate.active
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        height: 2
                        color: Theme.accent
                    }

                    Rectangle {
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: 1
                        color: Theme.border
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 5
                        spacing: 7

                        AppIcon {
                            Layout.preferredWidth: 14
                            Layout.preferredHeight: 14
                            name: tabDelegate.modelData.scanning
                                ? (tabDelegate.modelData.scanCancelling
                                    ? "square" : "refresh")
                                : "folder"
                            strokeWidth: 1.7
                            iconColor: tabDelegate.modelData.scanning
                                ? Theme.accentText
                                : tabDelegate.active
                                    ? Theme.accentText : Theme.muted
                        }

                        Item {
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            Text {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                visible:
                                    root.renamingId !== tabDelegate.modelData.id
                                text: tabDelegate.modelData.title
                                color: tabDelegate.active
                                    ? Theme.text : Theme.textSoft
                                font.pixelSize: 11
                                font.weight: tabDelegate.active
                                    ? Font.DemiBold : Font.Medium
                                elide: Text.ElideRight
                                verticalAlignment: Text.AlignVCenter
                            }

                            TextInput {
                                id: renameField
                                anchors.fill: parent
                                visible:
                                    root.renamingId === tabDelegate.modelData.id
                                text: tabDelegate.modelData.title
                                color: Theme.text
                                selectionColor: Theme.accent
                                selectedTextColor: Theme.accentContent
                                font.pixelSize: 11
                                font.weight: Font.DemiBold
                                verticalAlignment: TextInput.AlignVCenter
                                selectByMouse: true
                                maximumLength: 80
                                z: 4

                                onVisibleChanged: {
                                    if (visible) {
                                        text = tabDelegate.modelData.title
                                        Qt.callLater(function() {
                                            renameField.forceActiveFocus()
                                            renameField.selectAll()
                                        })
                                    }
                                }
                                Keys.onReturnPressed: root.finishRename(
                                    tabDelegate.modelData.id,
                                    text
                                )
                                Keys.onEnterPressed: root.finishRename(
                                    tabDelegate.modelData.id,
                                    text
                                )
                                Keys.onEscapePressed: root.renamingId = ""
                                onEditingFinished: {
                                    if (visible)
                                        root.finishRename(
                                            tabDelegate.modelData.id,
                                            text
                                        )
                                }
                            }
                        }

                        Item {
                            Layout.preferredWidth: 22
                            Layout.preferredHeight: 22

                            Rectangle {
                                anchors.fill: parent
                                radius: Theme.radiusWorkbench
                                color: closeMouse.containsMouse
                                    ? Theme.active : "transparent"
                                opacity: tabDelegate.active
                                    || tabDelegate.hovered ? 1 : 0

                                AppIcon {
                                    anchors.centerIn: parent
                                    width: 13
                                    height: 13
                                    name: "close"
                                    strokeWidth: 1.8
                                    iconColor: Theme.muted
                                }

                                MouseArea {
                                    id: closeMouse
                                    anchors.fill: parent
                                    enabled: parent.opacity > 0
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.libraryPage.closeWorkspace(
                                        tabDelegate.modelData.id
                                    )
                                }
                            }
                        }
                    }

                    MouseArea {
                        id: tabMouse
                        anchors.fill: parent
                        anchors.rightMargin: 27
                        acceptedButtons:
                            Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        enabled:
                            root.renamingId !== tabDelegate.modelData.id
                        onClicked: function(mouse) {
                            if (mouse.button === Qt.RightButton) {
                                root.openContextMenu(
                                    tabDelegate.modelData.id,
                                    tabDelegate.index
                                )
                            } else if (mouse.button === Qt.MiddleButton) {
                                root.libraryPage.closeWorkspace(
                                    tabDelegate.modelData.id
                                )
                            } else {
                                root.libraryPage.activateWorkspace(
                                    tabDelegate.modelData.id
                                )
                            }
                        }
                        onDoubleClicked: function(mouse) {
                            if (mouse.button === Qt.LeftButton)
                                root.beginRename(tabDelegate.modelData.id)
                        }
                    }

                    HoverHandler {
                        id: tabHover
                    }
                    ToolTip.visible:
                        tabHover.hovered && root.renamingId.length === 0
                    ToolTip.text: {
                        const selected = String(
                            tabDelegate.modelData.selectedMediaName || ""
                        )
                        const path = String(tabDelegate.modelData.root || "")
                        const scan = tabDelegate.modelData.scanning
                            ? (tabDelegate.modelData.scanCancelling
                                ? "Stopping scan\n" : "Scanning\n")
                            : ""
                        if (selected.length)
                            return scan + selected + "\n" + path
                        return scan + (path.length ? path : "No root folder")
                    }
                    ToolTip.delay: 600
                }

                ScrollBar.horizontal: AppScrollBar {
                    policy: ScrollBar.AsNeeded
                    height: 2
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: Theme.border
        }

        Rectangle {
            id: addWorkspaceButton
            Layout.preferredWidth: 34
            Layout.fillHeight: true
            color: addWorkspaceMouse.containsMouse
                ? Theme.hover : "transparent"

            AppIcon {
                anchors.centerIn: parent
                width: 15
                height: 15
                name: "plus"
                strokeWidth: 1.8
                iconColor: addWorkspaceMouse.containsMouse
                    ? Theme.text : Theme.muted
            }
            MouseArea {
                id: addWorkspaceMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.libraryPage.chooseNewWorkspaceFolder()
            }
            ToolTip.visible: addWorkspaceMouse.containsMouse
            ToolTip.text: "New workspace"
            ToolTip.delay: 450
        }

        Rectangle {
            id: workspaceMenuButton
            Layout.preferredWidth: 34
            Layout.fillHeight: true
            color: workspaceMenuMouse.containsMouse
                ? Theme.hover : "transparent"

            AppIcon {
                anchors.centerIn: parent
                width: 15
                height: 15
                name: "more"
                iconColor: workspaceMenuMouse.containsMouse
                    ? Theme.text : Theme.muted
            }
            MouseArea {
                id: workspaceMenuMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    root.contextWorkspaceId =
                        root.appController.activeWorkspaceId
                    root.contextWorkspaceIndex = tabList.currentIndex
                    tabContextMenu.popup()
                }
            }
            ToolTip.visible: workspaceMenuMouse.containsMouse
            ToolTip.text: "Workspace actions"
            ToolTip.delay: 450
        }
    }

    Menu {
        id: tabContextMenu
        width: 232
        padding: 5
        background: Rectangle {
            radius: Theme.radiusMd
            color: Theme.surfaceSoft
            border.width: 1
            border.color: Theme.borderStrong
        }

        AppMenuItem {
            text: "Rename workspace"
            iconName: "edit"
            onTriggered: root.beginRename(root.contextWorkspaceId)
        }
        AppMenuItem {
            text: "Duplicate workspace"
            iconName: "copy"
            onTriggered: root.libraryPage.duplicateWorkspace(
                root.contextWorkspaceId
            )
        }
        AppMenuItem {
            text: "Show root in file manager"
            iconName: "folder"
            enabled: {
                const rows = root.appController.workspaceTabs
                for (let index = 0; index < rows.length; ++index) {
                    if (rows[index].id === root.contextWorkspaceId)
                        return String(rows[index].root || "").length > 0
                }
                return false
            }
            onTriggered: root.appController.revealWorkspaceRoot(
                root.contextWorkspaceId
            )
        }
        AppMenuItem {
            text: root.appController.scanCancelling
                ? "Stopping scan…" : "Stop library scan"
            iconName: "square"
            visible: {
                const rows = root.appController.workspaceTabs
                for (let index = 0; index < rows.length; ++index) {
                    if (rows[index].id === root.contextWorkspaceId)
                        return Boolean(rows[index].scanning)
                }
                return false
            }
            enabled: !root.appController.scanCancelling
            onTriggered: root.appController.cancelScan()
        }

        MenuSeparator { }

        AppMenuItem {
            text: "Close workspace"
            iconName: "close"
            onTriggered: root.libraryPage.closeWorkspace(
                root.contextWorkspaceId
            )
        }
        AppMenuItem {
            text: "Close other workspaces"
            enabled: root.appController.workspaceCount > 1
            onTriggered: root.libraryPage.closeOtherWorkspaces(
                root.contextWorkspaceId
            )
        }
        AppMenuItem {
            text: "Close workspaces to the right"
            enabled: root.contextWorkspaceIndex >= 0
                && root.contextWorkspaceIndex
                    < root.appController.workspaceCount - 1
            onTriggered: root.libraryPage.closeWorkspacesToRight(
                root.contextWorkspaceId
            )
        }

        MenuSeparator { }

        AppMenuItem {
            text: "New workspace…"
            iconName: "plus"
            onTriggered: root.libraryPage.chooseNewWorkspaceFolder()
        }
        AppMenuItem {
            text: "Reopen closed workspace"
            iconName: "history"
            enabled: root.appController.closedWorkspaceCount > 0
            onTriggered: root.libraryPage.reopenClosedWorkspace()
        }
    }
}
