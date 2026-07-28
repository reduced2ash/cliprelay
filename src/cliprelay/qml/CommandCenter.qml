pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Item {
    id: root
    objectName: "commandCenter"

    required property var appController
    required property var actionRegistry
    property string externalMediaQuery: ""
    property string mediaQuery: ""
    property bool syncingQuery: false
    readonly property bool commandMode:
        leadingTrim(commandField.text).indexOf(">") === 0
    readonly property string commandNeedle: commandMode
        ? leadingTrim(commandField.text).substring(1).trim().toLowerCase()
        : ""
    readonly property var commandRows: {
        const rows = []
        const actions = root.actionRegistry.actions
        for (let index = 0; index < actions.length; ++index) {
            const action = actions[index]
            const haystack = (
                action.label + " " + action.detail + " "
                + action.category + " " + action.keywords
            ).toLowerCase()
            if (root.commandNeedle.length
                    && haystack.indexOf(root.commandNeedle) < 0)
                continue
            rows.push({
                "kind": "command",
                "title": action.label,
                "detail": action.category,
                "icon": action.icon,
                "shortcut": action.shortcut,
                "actionId": action.id,
                "mediaId": 0,
                "folderPath": "",
                "count": 0,
                "enabled": action.enabled
            })
        }
        return rows
    }
    readonly property var visibleRows: commandMode
        ? commandRows : appController.commandSearchResults
    readonly property bool hasQuery: commandMode
        || commandField.text.trim().length > 0
    readonly property bool popupOpen: resultsPopup.opened

    signal searchRequested(string query)
    signal mediaRequested(int mediaId)
    signal folderRequested(string folderPath)

    function leadingTrim(value) {
        return String(value || "").replace(/^\s+/, "")
    }

    function setExternalQuery(value) {
        const normalized = String(value || "")
        root.mediaQuery = normalized
        if (!root.commandMode) {
            root.syncingQuery = true
            commandField.text = normalized
            root.syncingQuery = false
        }
    }

    function focusSearch() {
        root.syncingQuery = true
        commandField.text = root.mediaQuery
        root.syncingQuery = false
        commandField.forceActiveFocus()
        commandField.selectAll()
        if (commandField.text.trim().length)
            resultsPopup.open()
    }

    function focusCommands() {
        root.syncingQuery = true
        commandField.text = "> "
        root.syncingQuery = false
        commandField.forceActiveFocus()
        commandField.cursorPosition = commandField.length
        resultsList.currentIndex = root.commandRows.length ? 0 : -1
        resultsPopup.open()
    }

    function scheduleSearch() {
        searchTimer.restart()
        if (commandField.text.trim().length)
            resultsPopup.open()
        else
            resultsPopup.close()
    }

    function activateCurrent() {
        const index = resultsList.currentIndex
        if (index < 0 || index >= root.visibleRows.length)
            return
        const row = root.visibleRows[index]
        if (row.enabled === false)
            return
        if (row.kind === "command") {
            root.actionRegistry.triggerAction(row.actionId)
            root.syncingQuery = true
            commandField.text = root.mediaQuery
            root.syncingQuery = false
        } else if (row.kind === "folder") {
            root.folderRequested(String(row.folderPath || ""))
        } else {
            root.mediaRequested(Number(row.mediaId || 0))
        }
        resultsPopup.close()
        commandField.focus = false
    }

    onExternalMediaQueryChanged: setExternalQuery(externalMediaQuery)
    onVisibleRowsChanged: {
        resultsList.currentIndex = visibleRows.length ? 0 : -1
    }
    Component.onCompleted: setExternalQuery(externalMediaQuery)

    Timer {
        id: searchTimer
        interval: 120
        repeat: false
        onTriggered: {
            root.searchRequested(root.mediaQuery)
            root.appController.requestCommandSearch(root.mediaQuery)
        }
    }

    TextField {
        id: commandField
        anchors.fill: parent
        leftPadding: 34
        rightPadding: shortcutLabel.visible ? 48 : 12
        topPadding: 0
        bottomPadding: 0
        color: Theme.text
        placeholderText: "Search videos, folders, and commands"
        placeholderTextColor: Theme.muted
        selectionColor: Theme.accent
        selectedTextColor: Theme.accentContent
        font.pixelSize: Theme.textWorkbench
        focusPolicy: Qt.StrongFocus
        selectByMouse: true
        Accessible.name: "Search videos, folders, and commands"
        Accessible.description:
            "Type a filename or folder, or begin with greater-than for commands"

        onTextEdited: {
            if (root.syncingQuery)
                return
            if (root.commandMode) {
                resultsList.currentIndex = root.commandRows.length ? 0 : -1
                resultsPopup.open()
                return
            }
            root.mediaQuery = text
            root.scheduleSearch()
        }
        onActiveFocusChanged: {
            if (activeFocus && root.hasQuery)
                resultsPopup.open()
        }

        Keys.onPressed: function(event) {
            if (event.key === Qt.Key_Down) {
                if (!resultsPopup.opened && root.hasQuery)
                    resultsPopup.open()
                if (root.visibleRows.length) {
                    resultsList.currentIndex = Math.min(
                        root.visibleRows.length - 1,
                        resultsList.currentIndex + 1
                    )
                }
                event.accepted = true
            } else if (event.key === Qt.Key_Up) {
                if (root.visibleRows.length) {
                    resultsList.currentIndex = Math.max(
                        0,
                        resultsList.currentIndex - 1
                    )
                }
                event.accepted = true
            } else if (event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                if (resultsPopup.opened)
                    root.activateCurrent()
                event.accepted = resultsPopup.opened
            } else if (event.key === Qt.Key_Escape) {
                if (resultsPopup.opened) {
                    resultsPopup.close()
                } else if (root.commandMode) {
                    root.syncingQuery = true
                    commandField.text = root.mediaQuery
                    root.syncingQuery = false
                } else if (commandField.text.length) {
                    root.mediaQuery = ""
                    commandField.text = ""
                    searchTimer.stop()
                    root.searchRequested("")
                    root.appController.requestCommandSearch("")
                } else {
                    commandField.focus = false
                }
                event.accepted = true
            }
        }

        background: Rectangle {
            radius: Theme.radiusWorkbench
            color: commandField.activeFocus
                ? Theme.active : Theme.raised
            border.width: commandField.activeFocus
                || resultsPopup.opened ? 2 : 1
            border.color: commandField.activeFocus
                || resultsPopup.opened ? Theme.accent : Theme.border
            Behavior on color {
                ColorAnimation { duration: Theme.fastMotion }
            }
            Behavior on border.color {
                ColorAnimation { duration: Theme.fastMotion }
            }
            AppIcon {
                anchors.left: parent.left
                anchors.leftMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                width: 15
                height: 15
                name: root.commandMode ? "command" : "search"
                strokeWidth: 1.75
                iconColor: commandField.activeFocus
                    ? Theme.accent : Theme.muted
            }
        }
    }

    Text {
        id: shortcutLabel
        visible: !commandField.activeFocus
            && commandField.text.length === 0
            && root.width >= 320
        anchors.right: parent.right
        anchors.rightMargin: 10
        anchors.verticalCenter: parent.verticalCenter
        text: Qt.platform.os === "osx" ? "⌘K" : "Ctrl K"
        color: Theme.mutedSoft
        font.pixelSize: 10
        font.family: Theme.monoFamily
    }

    Popup {
        id: resultsPopup
        objectName: "commandCenterPopup"
        parent: Overlay.overlay
        readonly property real edgeInset: 8
        readonly property point origin: parent
            ? root.mapToItem(parent, 0, 0) : Qt.point(0, 0)
        x: parent
            ? Math.max(
                edgeInset,
                Math.min(
                    parent.width - width - edgeInset,
                    origin.x
                )
            ) : edgeInset
        y: parent
            ? (origin.y + root.height + 5 + height
                    <= parent.height - edgeInset
                ? origin.y + root.height + 5
                : Math.max(edgeInset, origin.y - height - 5))
            : edgeInset
        width: Math.min(
            Math.max(root.width, 420),
            Math.max(0, (parent ? parent.width : root.width) - edgeInset * 2)
        )
        height: Math.min(
            330,
            Math.max(
                54,
                resultsHeader.height
                    + (root.visibleRows.length
                        ? Math.min(root.visibleRows.length, 8) * 42 + 8
                        : 58)
            )
        )
        padding: 0
        modal: false
        focus: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        z: 10000

        background: Rectangle {
            color: Theme.surfaceSoft
            radius: Theme.radiusSm
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 0

            RowLayout {
                id: resultsHeader
                Layout.fillWidth: true
                Layout.preferredHeight: 34
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                spacing: 8
                Text {
                    Layout.fillWidth: true
                    text: root.commandMode ? "COMMANDS" : "LIBRARY"
                    color: Theme.textSoft
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.8
                }
                Text {
                    text: root.commandMode
                        ? root.visibleRows.length.toLocaleString()
                        : root.appController.commandSearchLoading
                            ? "SEARCHING"
                            : root.visibleRows.length.toLocaleString()
                                + " MATCH"
                                + (root.visibleRows.length === 1 ? "" : "ES")
                    color: root.appController.commandSearchLoading
                        && !root.commandMode ? Theme.warning : Theme.muted
                    font.pixelSize: 10
                    font.family: Theme.monoFamily
                    font.weight: Font.Medium
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
                    id: resultsList
                    anchors.fill: parent
                    anchors.margins: 4
                    clip: true
                    spacing: 0
                    model: root.visibleRows
                    currentIndex: root.visibleRows.length ? 0 : -1
                    boundsBehavior: Flickable.StopAtBounds
                    highlightMoveDuration: 0
                    ScrollBar.vertical: AppScrollBar { }

                    delegate: ItemDelegate {
                        id: resultRow
                        required property int index
                        required property var modelData
                        width: ListView.view.width
                        height: 42
                        hoverEnabled: true
                        enabled: modelData.enabled !== false
                        highlighted: ListView.isCurrentItem
                            || hovered || activeFocus
                        focusPolicy: Qt.StrongFocus
                        Accessible.name: modelData.title
                        Accessible.description: modelData.detail
                        onClicked: {
                            resultsList.currentIndex = resultRow.index
                            root.activateCurrent()
                        }

                        contentItem: RowLayout {
                            spacing: 8
                            AppIcon {
                                Layout.preferredWidth: 16
                                Layout.preferredHeight: 16
                                name: resultRow.modelData.icon || "media"
                                strokeWidth: 1.7
                                iconColor: resultRow.highlighted
                                    ? Theme.accentText : Theme.muted
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.minimumWidth: 0
                                spacing: 0
                                Text {
                                    Layout.fillWidth: true
                                    text: resultRow.modelData.title || ""
                                    color: resultRow.enabled
                                        ? Theme.text : Theme.muted
                                    font.pixelSize: Theme.textWorkbench
                                    font.weight: Font.Medium
                                    elide: Text.ElideMiddle
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: resultRow.modelData.detail || ""
                                    color: Theme.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideMiddle
                                }
                            }
                            Text {
                                visible: String(
                                    resultRow.modelData.shortcut || ""
                                ).length > 0
                                text: resultRow.modelData.shortcut || ""
                                color: Theme.mutedSoft
                                font.pixelSize: 10
                                font.family: Theme.monoFamily
                            }
                            Text {
                                visible: resultRow.modelData.kind === "folder"
                                text: Number(
                                    resultRow.modelData.count || 0
                                ).toLocaleString()
                                color: Theme.mutedSoft
                                font.pixelSize: 10
                                font.family: Theme.monoFamily
                            }
                        }

                        background: Rectangle {
                            radius: Theme.radiusWorkbench
                            color: resultRow.highlighted
                                ? Theme.active : "transparent"
                            border.width: resultRow.activeFocus
                                ? Theme.focusWidth : 0
                            border.color: Theme.accent
                        }
                    }
                }

                Text {
                    visible: root.visibleRows.length === 0
                        && !root.appController.commandSearchLoading
                    anchors.centerIn: parent
                    width: parent.width - 28
                    text: root.commandMode
                        ? "No command matches this phrase."
                        : "No indexed video or folder matches."
                    color: Theme.muted
                    font.pixelSize: Theme.textWorkbench
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                }

                Row {
                    visible: !root.commandMode
                        && root.appController.commandSearchLoading
                    anchors.centerIn: parent
                    spacing: 8
                    AppProgressBar {
                        width: 96
                        height: 4
                        indeterminate: true
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        text: "Searching the index…"
                        color: Theme.muted
                        font.pixelSize: Theme.textWorkbench
                    }
                }
            }
        }
    }
}
