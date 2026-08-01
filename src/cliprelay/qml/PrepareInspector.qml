import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

ColumnLayout {
    id: root

    property bool studioMode: false
    property var editor
    property real trimStart: 0
    property real trimEnd: 0
    property int activeTab: 0
    property string activeAction: ""
    property bool lastSubmitXEnabled: true
    readonly property int editCount: editor
        ? (editor.cropEnabled ? 1 : 0) + Number(editor.shapeCount || 0)
        : 0
    readonly property bool telegramReady: publishTools.telegramReady
    readonly property string estimatedSize: publishTools.estimatedSize

    signal activeTabEdited(int index)
    signal draftChanged()
    signal submitRequested(string action)

    spacing: 0

    function selectTab(index) {
        var next = Math.max(0, Math.min(1, Number(index || 0)))
        if (root.activeTab === next)
            return
        root.activeTabEdited(next)
        root.draftChanged()
    }

    function capturePublishState() {
        return publishTools.captureState()
    }

    function captureScrollState() {
        return {
            editScrollY: editScroll.contentItem
                ? Number(editScroll.contentItem["contentY"] || 0) : 0,
            publishScrollY: publishScroll.contentItem
                ? Number(publishScroll.contentItem["contentY"] || 0) : 0
        }
    }

    function restorePublishState(draft) {
        publishTools.restoreState(draft || {})
        editTools.syncFromEditor()
    }

    function restoreScrollState(draft) {
        Qt.callLater(function() {
            if (editScroll.contentItem)
                editScroll.contentItem["contentY"] = Math.max(
                    0, Number(draft.editScrollY || 0)
                )
            if (publishScroll.contentItem)
                publishScroll.contentItem["contentY"] = Math.max(
                    0, Number(draft.publishScrollY || 0)
                )
        })
    }

    function publishValues() {
        return publishTools.deliveryValues()
    }

    function resetScrollPositions() {
        Qt.callLater(function() {
            if (editScroll.contentItem)
                editScroll.contentItem["contentY"] = 0
            if (publishScroll.contentItem)
                publishScroll.contentItem["contentY"] = 0
        })
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Theme.prepareInspectorTabsHeight
        color: Theme.surfaceSoft

        RowLayout {
            anchors.fill: parent
            spacing: 0

            TabButton {
                id: editTab
                Layout.fillWidth: true
                Layout.fillHeight: true
                text: "Edit"
                checked: root.activeTab === 0
                hoverEnabled: true
                focusPolicy: Qt.StrongFocus
                Accessible.role: Accessible.PageTab
                Accessible.name: "Edit inspector"
                Accessible.selected: checked
                onClicked: root.selectTab(0)

                contentItem: RowLayout {
                    spacing: 7
                    Item { Layout.fillWidth: true }
                    AppIcon {
                        Layout.preferredWidth: 14
                        Layout.preferredHeight: 14
                        name: "crop"
                        iconColor: editTab.checked
                            ? Theme.accentText : Theme.muted
                    }
                    Text {
                        text: "Edit"
                        color: editTab.checked ? Theme.text : Theme.textSoft
                        font.pixelSize: Theme.textSm
                        font.weight: Font.DemiBold
                    }
                    Text {
                        text: root.editCount === 0
                            ? "No edits"
                            : root.editCount + (root.editCount === 1
                                ? " change" : " changes")
                        color: root.editCount > 0
                            ? Theme.accentText : Theme.muted
                        font.pixelSize: Theme.textXs
                    }
                    Item { Layout.fillWidth: true }
                }
                background: Rectangle {
                    color: editTab.checked
                        ? Theme.active
                        : editTab.hovered ? Theme.hover : "transparent"
                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: editTab.checked ? 2 : 1
                        color: editTab.checked ? Theme.accent : Theme.border
                    }
                }
            }

            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Theme.border
            }

            TabButton {
                id: publishTab
                Layout.fillWidth: true
                Layout.fillHeight: true
                text: "Publish"
                checked: root.activeTab === 1
                hoverEnabled: true
                focusPolicy: Qt.StrongFocus
                Accessible.role: Accessible.PageTab
                Accessible.name: "Publish inspector"
                Accessible.selected: checked
                onClicked: root.selectTab(1)

                contentItem: RowLayout {
                    spacing: 7
                    Item { Layout.fillWidth: true }
                    AppIcon {
                        Layout.preferredWidth: 14
                        Layout.preferredHeight: 14
                        name: "send"
                        iconColor: publishTab.checked
                            ? Theme.accentText : Theme.muted
                    }
                    Text {
                        text: "Publish"
                        color: publishTab.checked
                            ? Theme.text : Theme.textSoft
                        font.pixelSize: Theme.textSm
                        font.weight: Font.DemiBold
                    }
                    Text {
                        text: controller.publishState.active
                            ? "Working"
                            : (controller.publishState.error || "").length > 0
                                ? "Result"
                                : root.telegramReady ? "Ready" : "Needs setup"
                        color: controller.publishState.active
                            ? Theme.accentText
                            : root.telegramReady ? Theme.success : Theme.warning
                        font.pixelSize: Theme.textXs
                    }
                    Item { Layout.fillWidth: true }
                }
                background: Rectangle {
                    color: publishTab.checked
                        ? Theme.active
                        : publishTab.hovered ? Theme.hover : "transparent"
                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: publishTab.checked ? 2 : 1
                        color: publishTab.checked ? Theme.accent : Theme.border
                    }
                }
            }
        }
    }

    StackLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        currentIndex: root.activeTab

        ScrollView {
            id: editScroll
            objectName: "prepareEditScroll"
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AppScrollBar { }
            contentWidth: availableWidth

            ColumnLayout {
                width: editScroll.availableWidth
                spacing: 0

                PrepareEditInspector {
                    id: editTools
                    Layout.fillWidth: true
                    Layout.leftMargin: 14
                    Layout.rightMargin: 14
                    Layout.topMargin: 14
                    Layout.bottomMargin: 16
                    studioMode: root.studioMode
                    editor: root.editor
                    enabled: !controller.selectedMediaChecking
                    opacity: enabled ? 1 : 0.5
                    onChanged: root.draftChanged()
                }
            }
        }

        ScrollView {
            id: publishScroll
            objectName: "preparePublishScroll"
            Component.onCompleted:
                contentItem.objectName = "prepareFlickable"
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AppScrollBar { }
            contentWidth: availableWidth

            ColumnLayout {
                width: publishScroll.availableWidth
                spacing: 0

                PreparePublishInspector {
                    id: publishTools
                    Layout.fillWidth: true
                    Layout.leftMargin: 14
                    Layout.rightMargin: 14
                    Layout.topMargin: 14
                    Layout.bottomMargin: 16
                    studioMode: root.studioMode
                    trimStart: root.trimStart
                    trimEnd: root.trimEnd
                    enabled: !controller.selectedMediaChecking
                    opacity: enabled ? 1 : 0.5
                    onChanged: root.draftChanged()
                }
            }
        }
    }

    PrepareActionDock {
        Layout.fillWidth: true
        Layout.preferredHeight: implicitHeight
        activeTab: root.activeTab
        editor: root.editor
        selectedMediaChecking: controller.selectedMediaChecking
        telegramReady: root.telegramReady
        estimatedSize: root.estimatedSize
        activeAction: root.activeAction
        lastSubmitXEnabled: root.lastSubmitXEnabled
        onSubmitRequested: function(action) {
            root.submitRequested(action)
        }
        onResetEditsRequested: {
            root.editor.reset()
            editTools.syncFromEditor()
            root.draftChanged()
        }
    }
}
