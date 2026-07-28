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
    property string activeScope: "all"
    property bool syncingQuery: false
    property double lastPanelClosedAt: 0
    property string lastClosedScope: ""

    readonly property var scopeOptions: [
        { "id": "all", "label": "All" },
        { "id": "videos", "label": "Videos" },
        { "id": "folders", "label": "Folders" },
        { "id": "commands", "label": "Commands" }
    ]
    readonly property var quickActionIds: [
        "pick_random",
        "choose_folder",
        "rescan",
        "toggle_folders"
    ]
    readonly property bool commandPrefix:
        activeScope !== "commands"
        && leadingTrim(commandField.text).indexOf(">") === 0
    readonly property string effectiveScope:
        commandPrefix ? "commands" : activeScope
    readonly property bool commandMode: effectiveScope === "commands"
    readonly property string resultNeedle: {
        const text = root.leadingTrim(commandField.text)
        return (
            root.commandPrefix
                ? text.substring(1)
                : text
        ).trim().toLowerCase()
    }
    readonly property bool hasQuery: resultNeedle.length > 0
    readonly property bool popupOpen: resultsPopup.opened
    readonly property bool libraryResultsVisible:
        effectiveScope !== "commands"
    readonly property var commandRows: {
        const rows = []
        const actions = root.actionRegistry.actions
        for (let index = 0; index < actions.length; ++index) {
            const action = actions[index]
            const haystack = (
                action.label + " " + action.detail + " "
                + action.category + " " + action.keywords
            ).toLowerCase()
            if (root.resultNeedle.length
                    && haystack.indexOf(root.resultNeedle) < 0)
                continue
            rows.push(root.commandResult(action))
        }
        const categoryOrder = {
            "Library": 0,
            "Navigation": 1,
            "View": 2,
            "Sort": 3,
            "Theme": 4
        }
        rows.sort(function(left, right) {
            const leftOrder = categoryOrder[left.category] === undefined
                ? 99 : categoryOrder[left.category]
            const rightOrder = categoryOrder[right.category] === undefined
                ? 99 : categoryOrder[right.category]
            if (leftOrder !== rightOrder)
                return leftOrder - rightOrder
            return left.title.localeCompare(right.title)
        })
        return rows
    }
    readonly property var quickCommandRows: {
        const rows = []
        for (let index = 0; index < root.quickActionIds.length; ++index) {
            const action = root.actionRegistry.action(
                root.quickActionIds[index]
            )
            if (action.id === root.quickActionIds[index])
                rows.push(root.commandResult(action, "QUICK ACTIONS"))
        }
        return rows
    }
    readonly property var libraryRows: {
        const rows = []
        const sourceRows = root.appController.commandSearchResults
        for (let index = 0; index < sourceRows.length; ++index) {
            const source = sourceRows[index]
            if (root.effectiveScope === "videos"
                    && source.kind !== "media")
                continue
            if (root.effectiveScope === "folders"
                    && source.kind !== "folder")
                continue
            rows.push(root.libraryResult(source))
        }
        return rows
    }
    readonly property var visibleRows: {
        if (root.commandMode)
            return root.commandRows
        if (root.effectiveScope === "videos"
                || root.effectiveScope === "folders")
            return root.libraryRows
        if (root.hasQuery)
            return root.libraryRows.concat(root.commandRows)
        return root.quickCommandRows.concat(root.libraryRows)
    }
    readonly property int sectionCount: {
        let count = 0
        let previous = ""
        for (let index = 0; index < root.visibleRows.length; ++index) {
            const section = String(
                root.visibleRows[index].section || ""
            )
            if (section.length && section !== previous) {
                count += 1
                previous = section
            }
        }
        return count
    }
    readonly property real panelHeight: Math.min(
        430,
        Math.max(
            174,
            42
                + (root.visibleRows.length
                    ? Math.min(root.visibleRows.length, 8) * 46
                        + root.sectionCount * 24 + 8
                    : 94)
                + 31
        )
    )

    signal searchRequested(string query)
    signal mediaRequested(int mediaId)
    signal folderRequested(string folderPath)

    function leadingTrim(value) {
        return String(value || "").replace(/^\s+/, "")
    }

    function commandResult(action, sectionOverride) {
        return {
            "kind": "command",
            "title": action.label,
            "detail": action.detail,
            "icon": action.icon,
            "shortcut": action.shortcut,
            "actionId": action.id,
            "mediaId": 0,
            "folderPath": "",
            "count": 0,
            "enabled": action.enabled,
            "category": action.category,
            "section": sectionOverride || action.category.toUpperCase()
        }
    }

    function libraryResult(source) {
        const media = source.kind === "media"
        return {
            "kind": source.kind,
            "title": source.title,
            "detail": source.detail,
            "icon": source.icon || (media ? "media" : "folder"),
            "shortcut": "",
            "actionId": "",
            "mediaId": Number(source.mediaId || 0),
            "folderPath": String(source.folderPath || ""),
            "count": Number(source.count || 0),
            "enabled": true,
            "category": media ? "Videos" : "Folders",
            "section": media
                ? (root.hasQuery ? "VIDEOS" : "RECENT VIDEOS")
                : "FOLDERS"
        }
    }

    function setFieldText(value) {
        root.syncingQuery = true
        commandField.text = String(value || "")
        commandField.cursorPosition = commandField.length
        root.syncingQuery = false
    }

    function setExternalQuery(value) {
        const normalized = String(value || "")
        root.mediaQuery = normalized
        if (!root.commandMode)
            root.setFieldText(normalized)
    }

    function openPanel() {
        resultsList.currentIndex = root.firstEnabledIndex()
        resultsPopup.open()
    }

    function focusSearch() {
        root.activeScope = "all"
        root.setFieldText(root.mediaQuery)
        commandField.forceActiveFocus()
        commandField.selectAll()
        if (root.mediaQuery.trim().length)
            root.appController.requestCommandSearch(root.mediaQuery)
        else
            root.appController.requestCommandOverview()
        root.openPanel()
    }

    function focusCommands() {
        root.activeScope = "commands"
        root.setFieldText("")
        commandField.forceActiveFocus()
        root.openPanel()
    }

    function toggleCommands() {
        const justClosed = Date.now() - root.lastPanelClosedAt < 180
        if (justClosed) {
            if (root.lastClosedScope !== "commands")
                root.focusCommands()
            return
        }
        if (resultsPopup.opened && root.commandMode) {
            resultsPopup.close()
            commandField.focus = false
            return
        }
        root.focusCommands()
    }

    function selectScope(scope) {
        const normalized = String(scope || "all")
        if (normalized === "commands") {
            const carryQuery = root.commandPrefix
                ? root.resultNeedle : commandField.text
            root.activeScope = "commands"
            root.setFieldText(carryQuery)
        } else {
            root.activeScope = normalized
            root.setFieldText(root.mediaQuery)
            if (root.mediaQuery.trim().length)
                root.appController.requestCommandSearch(root.mediaQuery)
            else
                root.appController.requestCommandOverview()
        }
        commandField.forceActiveFocus()
        root.openPanel()
    }

    function clearActiveQuery() {
        searchTimer.stop()
        if (root.commandMode) {
            root.setFieldText("")
        } else {
            root.mediaQuery = ""
            root.setFieldText("")
            root.searchRequested("")
            root.appController.requestCommandOverview()
        }
        resultsList.currentIndex = root.firstEnabledIndex()
        root.openPanel()
    }

    function firstEnabledIndex() {
        for (let index = 0; index < root.visibleRows.length; ++index) {
            if (root.visibleRows[index].enabled !== false)
                return index
        }
        return -1
    }

    function moveCurrent(direction) {
        if (!root.visibleRows.length)
            return
        let index = resultsList.currentIndex
        if (index < 0)
            index = direction > 0 ? -1 : root.visibleRows.length
        while (true) {
            index += direction
            if (index < 0 || index >= root.visibleRows.length)
                return
            if (root.visibleRows[index].enabled !== false) {
                resultsList.currentIndex = index
                resultsList.positionViewAtIndex(index, ListView.Contain)
                return
            }
        }
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
        } else if (row.kind === "folder") {
            root.folderRequested(String(row.folderPath || ""))
        } else {
            root.mediaRequested(Number(row.mediaId || 0))
        }
        root.activeScope = "all"
        root.setFieldText(root.mediaQuery)
        resultsPopup.close()
        commandField.focus = false
    }

    function emptyTitle() {
        if (!root.appController.settings.library_root
                && root.effectiveScope !== "commands")
            return "Choose a library root to begin"
        if (root.effectiveScope === "commands")
            return "No matching command"
        if (root.effectiveScope === "folders")
            return root.hasQuery
                ? "No matching folder" : "No indexed folders"
        if (root.effectiveScope === "videos")
            return root.hasQuery
                ? "No matching video" : "No indexed videos"
        return "No matching result"
    }

    function emptyDetail() {
        if (!root.appController.settings.library_root
                && root.effectiveScope !== "commands")
            return "Use Choose library root below, or run it as a command."
        return root.hasQuery
            ? "Try a shorter filename, folder, or action."
            : "Your library index has no items for this scope."
    }

    onExternalMediaQueryChanged: setExternalQuery(externalMediaQuery)
    onVisibleRowsChanged: {
        resultsList.currentIndex = firstEnabledIndex()
        if (resultsList.currentIndex >= 0)
            resultsList.positionViewAtBeginning()
    }
    Component.onCompleted: setExternalQuery(externalMediaQuery)

    Timer {
        id: searchTimer
        interval: 120
        repeat: false
        onTriggered: {
            root.searchRequested(root.mediaQuery)
            if (root.mediaQuery.trim().length)
                root.appController.requestCommandSearch(root.mediaQuery)
            else
                root.appController.requestCommandOverview()
        }
    }

    TextField {
        id: commandField
        anchors.fill: parent
        leftPadding: 34
        rightPadding: clearButton.visible
            ? 38 : shortcutLabel.visible ? 48 : 12
        topPadding: 0
        bottomPadding: 0
        color: Theme.text
        placeholderText: root.commandMode
            ? "Run a command"
            : "Search videos, folders, and commands"
        placeholderTextColor: Theme.muted
        selectionColor: Theme.accent
        selectedTextColor: Theme.accentContent
        font.pixelSize: Theme.textWorkbench
        focusPolicy: Qt.StrongFocus
        selectByMouse: true
        Accessible.name: root.commandMode
            ? "Command palette"
            : "Search videos, folders, and commands"
        Accessible.description:
            "Search the indexed library or choose a visible result scope"

        onTextEdited: {
            if (root.syncingQuery)
                return
            if (root.commandMode) {
                resultsList.currentIndex = root.firstEnabledIndex()
                root.openPanel()
                return
            }
            root.mediaQuery = text
            searchTimer.restart()
            root.openPanel()
        }
        onActiveFocusChanged: {
            if (!activeFocus)
                return
            if (!root.commandMode
                    && root.mediaQuery.trim().length === 0)
                root.appController.requestCommandOverview()
            root.openPanel()
        }

        Keys.onPressed: function(event) {
            if (event.key === Qt.Key_Down) {
                root.moveCurrent(1)
                event.accepted = true
            } else if (event.key === Qt.Key_Up) {
                root.moveCurrent(-1)
                event.accepted = true
            } else if (event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                if (resultsPopup.opened)
                    root.activateCurrent()
                event.accepted = resultsPopup.opened
            } else if (event.key === Qt.Key_Escape) {
                if (resultsPopup.opened) {
                    resultsPopup.close()
                    commandField.focus = false
                } else if (root.commandMode) {
                    root.activeScope = "all"
                    root.setFieldText(root.mediaQuery)
                } else if (commandField.text.length) {
                    root.clearActiveQuery()
                    resultsPopup.close()
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

    WorkbenchButton {
        id: clearButton
        visible: commandField.activeFocus
            && commandField.text.length > 0
        anchors.right: parent.right
        anchors.rightMargin: 3
        anchors.verticalCenter: parent.verticalCenter
        width: 25
        height: 25
        text: root.commandMode ? "Clear command search" : "Clear search"
        iconName: "close"
        iconOnly: true
        kind: "ghost"
        onClicked: {
            root.clearActiveQuery()
            commandField.forceActiveFocus()
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
        parent: root
        popupType: Popup.Item
        x: 0
        y: root.height + 5
        width: Math.max(root.width, 520)
        height: root.panelHeight
        padding: 0
        modal: false
        focus: false
        closePolicy: Popup.CloseOnEscape
            | Popup.CloseOnPressOutsideParent
        z: 10000

        onClosed: {
            root.lastPanelClosedAt = Date.now()
            root.lastClosedScope = root.effectiveScope
            if (root.lastClosedScope === "commands") {
                root.activeScope = "all"
                root.setFieldText(root.mediaQuery)
            }
        }

        enter: Transition {
            NumberAnimation {
                property: "opacity"
                from: 0
                to: 1
                duration: Theme.fastMotion
                easing.type: Easing.OutQuart
            }
        }
        exit: Transition {
            NumberAnimation {
                property: "opacity"
                from: 1
                to: 0
                duration: Theme.quickMotion
                easing.type: Easing.OutQuart
            }
        }

        background: Rectangle {
            color: Theme.surfaceSoft
            radius: Theme.radiusSm
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 42
                Layout.leftMargin: 7
                Layout.rightMargin: 9
                spacing: 3

                Repeater {
                    model: root.scopeOptions
                    delegate: WorkbenchButton {
                        id: scopeButton
                        required property var modelData
                        readonly property bool selected:
                            root.effectiveScope === modelData.id
                        Layout.preferredHeight: 28
                        Layout.preferredWidth: Math.max(
                            52,
                            implicitWidth
                        )
                        text: modelData.label
                        kind: selected ? "secondary" : "ghost"
                        toolTipText: "Show " + modelData.label.toLowerCase()
                        onClicked: root.selectScope(modelData.id)

                        Rectangle {
                            visible: scopeButton.selected
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            height: 1
                            color: Theme.accent
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: root.appController.commandSearchLoading
                        && root.libraryResultsVisible
                        ? "SEARCHING"
                        : root.visibleRows.length.toLocaleString()
                            + " RESULT"
                            + (root.visibleRows.length === 1 ? "" : "S")
                    color: root.appController.commandSearchLoading
                        && root.libraryResultsVisible
                        ? Theme.warning : Theme.muted
                    font.pixelSize: 10
                    font.family: Theme.monoFamily
                    font.weight: Font.Medium
                }
            }

            AppProgressBar {
                visible: root.appController.commandSearchLoading
                    && root.libraryResultsVisible
                Layout.fillWidth: true
                Layout.preferredHeight: visible ? 2 : 0
                indeterminate: true
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
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
                    currentIndex: root.firstEnabledIndex()
                    boundsBehavior: Flickable.StopAtBounds
                    highlightMoveDuration: 0
                    reuseItems: true
                    keyNavigationEnabled: false
                    ScrollBar.vertical: AppScrollBar { }

                    section.property: "section"
                    section.criteria: ViewSection.FullString
                    section.delegate: Rectangle {
                        required property string section
                        width: ListView.view.width
                        height: 24
                        color: Theme.surfaceSoft
                        z: 2
                        Text {
                            anchors.left: parent.left
                            anchors.leftMargin: 9
                            anchors.verticalCenter: parent.verticalCenter
                            text: parent.section
                            color: Theme.mutedSoft
                            font.pixelSize: 9
                            font.weight: Font.DemiBold
                            font.letterSpacing: 0.7
                        }
                    }

                    delegate: ItemDelegate {
                        id: resultRow
                        required property int index
                        required property var modelData
                        width: ListView.view.width
                        height: 46
                        hoverEnabled: true
                        enabled: modelData.enabled !== false
                        opacity: enabled ? 1 : 0.42
                        highlighted: ListView.isCurrentItem
                            || hovered || visualFocus
                        focusPolicy: Qt.StrongFocus
                        Accessible.name: modelData.title
                        Accessible.description: modelData.detail
                        onClicked: {
                            resultsList.currentIndex = resultRow.index
                            root.activateCurrent()
                        }

                        contentItem: RowLayout {
                            spacing: 9

                            Item {
                                Layout.preferredWidth: 27
                                Layout.preferredHeight: 27
                                Layout.alignment: Qt.AlignVCenter

                                Rectangle {
                                    anchors.fill: parent
                                    radius: Theme.radiusWorkbench
                                    color: resultRow.highlighted
                                        ? Theme.accentSoft : Theme.raised
                                    border.width: 1
                                    border.color: resultRow.highlighted
                                        ? Theme.accent : Theme.border
                                }
                                AppIcon {
                                    anchors.centerIn: parent
                                    width: 15
                                    height: 15
                                    name: resultRow.modelData.icon || "media"
                                    strokeWidth: 1.7
                                    iconColor: resultRow.highlighted
                                        ? Theme.accentText : Theme.muted
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.minimumWidth: 0
                                Layout.alignment: Qt.AlignVCenter
                                spacing: 1
                                Text {
                                    Layout.fillWidth: true
                                    text: resultRow.modelData.title || ""
                                    color: Theme.text
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
                                Layout.alignment: Qt.AlignVCenter
                                text: resultRow.modelData.shortcut || ""
                                color: Theme.mutedSoft
                                font.pixelSize: 10
                                font.family: Theme.monoFamily
                            }

                            Text {
                                visible: resultRow.modelData.kind === "folder"
                                Layout.alignment: Qt.AlignVCenter
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
                            border.width: resultRow.visualFocus
                                ? Theme.focusWidth : 0
                            border.color: Theme.accent
                        }
                    }
                }

                Column {
                    visible: root.visibleRows.length === 0
                        && !root.appController.commandSearchLoading
                    anchors.centerIn: parent
                    width: parent.width - 36
                    spacing: 7

                    AppIcon {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 23
                        height: 23
                        name: root.commandMode ? "command" : "search"
                        strokeWidth: 1.65
                        iconColor: Theme.mutedSoft
                    }
                    Text {
                        width: parent.width
                        text: root.emptyTitle()
                        color: Theme.textSoft
                        font.pixelSize: Theme.textWorkbench
                        font.weight: Font.DemiBold
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: root.emptyDetail()
                        color: Theme.muted
                        font.pixelSize: 10
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 30
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                spacing: 12

                Text {
                    text: "↑↓  NAVIGATE"
                    color: Theme.mutedSoft
                    font.pixelSize: 9
                    font.family: Theme.monoFamily
                }
                Text {
                    text: "↵  OPEN"
                    color: Theme.mutedSoft
                    font.pixelSize: 9
                    font.family: Theme.monoFamily
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: "ESC  CLOSE"
                    color: Theme.mutedSoft
                    font.pixelSize: 9
                    font.family: Theme.monoFamily
                }
            }
        }
    }
}
