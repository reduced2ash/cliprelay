import QtQuick
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    objectName: "preparePanel"

    property bool studioMode: false
    property real trimStart: 0
    property real trimEnd: Number(controller.selectedMedia.duration || 0)
    property int loadedMediaId: 0
    property int activeInspectorTab: 0
    property int studioTab: activeInspectorTab
    property string activeAction: ""
    property bool lastSubmitXEnabled: true
    property bool restoringDraft: false
    property real studioInspectorWidth: 460
    readonly property bool hasEdits: prepareStage.editor.hasEdits
    readonly property bool cutActive:
        Number(controller.selectedMedia.duration || 0) > 0
        && (root.trimStart > 0.05
            || root.trimEnd
                < Number(controller.selectedMedia.duration || 0) - 0.05)
    readonly property bool compactDockedStage:
        !root.studioMode && root.height < 620
    readonly property real selectedMediaAspect: Math.max(
        0.1,
        Number(controller.selectedMedia.width || 16)
            / Math.max(1, Number(controller.selectedMedia.height || 9))
    )
    readonly property real dockedIdealFrameHeight: Math.max(
        170,
        Math.min(
            480,
            Math.max(1, root.width - 24) / root.selectedMediaAspect
        )
    )
    readonly property real resolvedStudioInspectorWidth: Math.max(
        360,
        Math.min(
            root.studioInspectorWidth,
            Math.min(620, Math.max(360, root.width - 420))
        )
    )
    readonly property real dockedStageHeight: root.compactDockedStage
        ? 315
        : Math.max(
            360,
            Math.min(
                root.dockedIdealFrameHeight + 230,
                Math.max(360, root.height - 210)
            )
        )

    signal fullScreenExitRequested()
    signal closeRequested()

    color: root.studioMode ? Theme.ink : Theme.surface

    onStudioTabChanged: {
        if (root.activeInspectorTab !== root.studioTab)
            root.activeInspectorTab = root.studioTab
    }
    onActiveInspectorTabChanged: {
        if (root.studioTab !== root.activeInspectorTab)
            root.studioTab = root.activeInspectorTab
    }

    function scheduleDraftSave() {
        if (root.restoringDraft
                || Number(controller.selectedMediaId || 0) <= 0)
            return
        draftSaveTimer.restart()
    }

    function saveDraftNow() {
        if (root.restoringDraft
                || Number(controller.selectedMediaId || 0) <= 0)
            return
        draftSaveTimer.stop()
        controller.saveActiveWorkspaceDraft(root.captureDraft())
    }

    function submit(action) {
        var delivery = prepareInspector.publishValues()
        var sendTelegram = action !== "x"
        var prepareX = action !== "telegram"
        root.activeAction = action
        root.lastSubmitXEnabled = prepareX
        controller.publish({
            mediaId: controller.selectedMediaId,
            trimStart: root.trimStart,
            trimEnd: root.trimEnd,
            preset: delivery.preset,
            targetMb: delivery.targetMb,
            telegramEnabled: sendTelegram,
            xEnabled: prepareX,
            telegramMode: delivery.telegramMode,
            telegramDestination: delivery.telegramDestination,
            telegramCaption: delivery.telegramCaption,
            xCaption: delivery.xCaption,
            edits: prepareStage.editSpec(),
            cleanupPolicy: delivery.cleanupPolicy
        })
    }

    function togglePlayback() {
        prepareStage.togglePlayback()
    }

    function resetCut() {
        root.trimStart = 0
        root.trimEnd = Number(controller.selectedMedia.duration || 0)
        prepareStage.player.position = 0
        root.scheduleDraftSave()
    }

    function captureDraft() {
        if (Number(controller.selectedMediaId || 0) <= 0)
            return {}
        var publishDraft = prepareInspector.capturePublishState()
        var scrollDraft = prepareInspector.captureScrollState()
        return {
            mediaId: Number(controller.selectedMediaId || 0),
            trimStart: root.trimStart,
            trimEnd: root.trimEnd,
            sameCaption: publishDraft.sameCaption,
            studioTab: root.activeInspectorTab,
            inspectorTab: root.activeInspectorTab,
            telegramModeIndex: publishDraft.telegramModeIndex,
            destination: publishDraft.destination,
            caption: publishDraft.caption,
            xCaption: publishDraft.xCaption,
            compressionIndex: publishDraft.compressionIndex,
            targetSize: publishDraft.targetSize,
            cleanupIndex: publishDraft.cleanupIndex,
            editScrollY: scrollDraft.editScrollY,
            publishScrollY: scrollDraft.publishScrollY,
            studioInspectorWidth: root.studioInspectorWidth,
            edits: prepareStage.editSpec()
        }
    }

    function restoreDraft(draft) {
        if (!draft
                || Number(draft.mediaId || 0)
                    !== Number(controller.selectedMediaId || 0))
            return
        root.restoringDraft = true
        var duration = Math.max(
            0,
            Number(controller.selectedMedia.duration || 0)
        )
        root.trimStart = Math.max(
            0,
            Math.min(duration, Number(draft.trimStart || 0))
        )
        root.trimEnd = Math.max(
            root.trimStart,
            Math.min(duration, Number(draft.trimEnd || duration))
        )
        root.activeInspectorTab = Math.max(
            0,
            Math.min(1, Number(
                draft.inspectorTab === undefined
                    ? draft.studioTab || 0 : draft.inspectorTab
            ))
        )
        root.studioInspectorWidth = Math.max(
            360,
            Math.min(620, Number(draft.studioInspectorWidth || 460))
        )
        prepareStage.loadEditSpec(draft.edits || {})
        prepareInspector.restorePublishState(draft)
        prepareInspector.restoreScrollState(draft)
        root.restoringDraft = false
        Qt.callLater(prepareStage.startAutoplay)
    }

    Timer {
        id: draftSaveTimer
        interval: 360
        repeat: false
        onTriggered: root.saveDraftNow()
    }

    Shortcut {
        sequence: "Escape"
        enabled: root.studioMode
        onActivated: root.fullScreenExitRequested()
    }

    Connections {
        target: controller

        function onSelectedMediaChanged() {
            var nextId = Number(controller.selectedMediaId || 0)
            if (nextId !== root.loadedMediaId) {
                draftSaveTimer.stop()
                root.restoringDraft = true
                root.loadedMediaId = nextId
                root.trimStart = 0
                root.trimEnd = Number(
                    controller.selectedMedia.duration || 0
                )
                root.activeInspectorTab = 0
                root.activeAction = ""
                prepareStage.reset()
                prepareInspector.resetScrollPositions()
                root.restoringDraft = false
            } else if (root.trimEnd <= 0) {
                root.trimEnd = Number(
                    controller.selectedMedia.duration || 0
                )
            }
        }

        function onPublishStateChanged() {
            if (!controller.publishState.active)
                root.activeAction = ""
        }

        function onWorkspaceDraftRestoreRequested(draft) {
            root.restoreDraft(draft)
        }
    }

    Component.onCompleted: {
        root.loadedMediaId = Number(controller.selectedMediaId || 0)
        root.restoreDraft(controller.activeWorkspaceDraft)
    }

    PrepareContextHeader {
        id: studioHeader
        visible: root.studioMode
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.studioMode
            ? Theme.prepareStudioHeaderHeight : 0
        z: 20
        fileName: controller.selectedMedia.name || ""
        edited: prepareStage.editor.hasEdits
        onOpenRequested: controller.openSelectedVideo()
        onExitRequested: root.fullScreenExitRequested()
        onCloseRequested: root.closeRequested()
    }

    StackLayout {
        id: layoutStack
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: root.studioMode ? studioHeader.bottom : parent.top
        anchors.bottom: parent.bottom
        currentIndex: root.studioMode ? 1 : 0

        ColumnLayout {
            id: dockedWorkspace
            spacing: 0

            Item {
                id: dockedStatusHost
                Layout.fillWidth: true
                Layout.preferredHeight: statusStrip.visible
                    ? statusStrip.implicitHeight : 0
            }

            Item {
                id: dockedStageHost
                Layout.fillWidth: true
                Layout.preferredHeight: root.dockedStageHeight
                Layout.minimumHeight: root.compactDockedStage ? 315 : 360
            }

            Item {
                id: dockedInspectorHost
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 120
            }
        }

        ColumnLayout {
            id: studioWorkspace
            spacing: 0

            Item {
                id: studioStatusHost
                Layout.fillWidth: true
                Layout.preferredHeight: statusStrip.visible
                    ? statusStrip.implicitHeight : 0
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                Item {
                    id: studioStageHost
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumWidth: 400
                }

                Item {
                    id: inspectorDivider
                    Layout.fillHeight: true
                    Layout.preferredWidth: 9

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 1
                        height: parent.height
                        color: dividerMouse.containsMouse
                            || dividerMouse.pressed
                            ? Theme.accent : Theme.border
                    }

                    MouseArea {
                        id: dividerMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.SplitHCursor
                        property real pressSceneX: 0
                        property real pressWidth: 0

                        onPressed: function(mouse) {
                            pressSceneX = mapToItem(
                                root, mouse.x, mouse.y
                            ).x
                            pressWidth = root.studioInspectorWidth
                        }
                        onPositionChanged: function(mouse) {
                            if (!pressed)
                                return
                            var sceneX = mapToItem(
                                root, mouse.x, mouse.y
                            ).x
                            root.studioInspectorWidth = Math.max(
                                360,
                                Math.min(
                                    620,
                                    pressWidth - (sceneX - pressSceneX)
                                )
                            )
                        }
                        onReleased: root.scheduleDraftSave()
                        onDoubleClicked: {
                            root.studioInspectorWidth = 460
                            root.scheduleDraftSave()
                        }
                        Accessible.role: Accessible.Splitter
                        Accessible.name: "Resize Prepare inspector"
                    }
                }

                Item {
                    id: studioInspectorHost
                    Layout.fillHeight: true
                    Layout.preferredWidth:
                        root.resolvedStudioInspectorWidth
                    Layout.minimumWidth: 360
                    Layout.maximumWidth: 620
                }
            }
        }
    }

    PrepareStatusStrip {
        id: statusStrip
        parent: root.studioMode
            ? studioStatusHost : dockedStatusHost
        anchors.fill: parent
        checking: controller.selectedMediaChecking
    }

    PrepareStage {
        id: prepareStage
        parent: root.studioMode ? studioStageHost : dockedStageHost
        anchors.fill: parent
        studioMode: root.studioMode
        compactMode: root.compactDockedStage
        trimStart: root.trimStart
        trimEnd: root.trimEnd
        onTrimStartEdited: function(seconds) {
            root.trimStart = seconds
            root.scheduleDraftSave()
        }
        onTrimEndEdited: function(seconds) {
            root.trimEnd = seconds
            root.scheduleDraftSave()
        }
        onEditsChanged: root.scheduleDraftSave()
    }

    PrepareInspector {
        id: prepareInspector
        parent: root.studioMode
            ? studioInspectorHost : dockedInspectorHost
        anchors.fill: parent
        studioMode: root.studioMode
        editor: prepareStage.editor
        trimStart: root.trimStart
        trimEnd: root.trimEnd
        activeTab: root.activeInspectorTab
        activeAction: root.activeAction
        lastSubmitXEnabled: root.lastSubmitXEnabled
        onActiveTabEdited: function(index) {
            root.activeInspectorTab = index
            root.scheduleDraftSave()
        }
        onDraftChanged: root.scheduleDraftSave()
        onSubmitRequested: function(action) {
            root.submit(action)
        }
    }
}
