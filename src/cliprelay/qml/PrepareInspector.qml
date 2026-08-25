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

    StackLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        currentIndex: root.activeTab

        ScrollView {
            id: editScroll
            objectName: "prepareEditScroll"
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AppScrollBar { }
            Component.onCompleted: {
                contentItem.pixelAligned = true
                contentItem.synchronousDrag = true
                contentItem.maximumFlickVelocity = 4000
            }
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
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AppScrollBar { }
            Component.onCompleted: {
                contentItem.objectName = "prepareFlickable"
                contentItem.pixelAligned = true
                contentItem.synchronousDrag = true
                contentItem.maximumFlickVelocity = 4000
            }
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
        visible: root.activeTab === 1
        Layout.fillWidth: true
        Layout.preferredHeight: visible ? implicitHeight : 0
        activeTab: root.activeTab
        selectedMediaChecking: controller.selectedMediaChecking
        telegramReady: root.telegramReady
        estimatedSize: root.estimatedSize
        activeAction: root.activeAction
        lastSubmitXEnabled: root.lastSubmitXEnabled
        onSubmitRequested: function(action) {
            root.submitRequested(action)
        }
    }

    PrepareInspectorTabs {
        Layout.fillWidth: true
        Layout.preferredHeight: implicitHeight
        activeTab: root.activeTab
        editCount: root.editCount
        telegramReady: root.telegramReady
        onTabSelected: function(index) {
            root.selectTab(index)
        }
    }
}
