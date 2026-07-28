pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    objectName: "preparePanel"

    property bool studioMode: false
    property real trimStart: 0
    property real trimEnd: Number(controller.selectedMedia.duration || 0)
    property bool sameCaption: true
    property int loadedMediaId: 0
    property string activeAction: ""
    property bool lastSubmitXEnabled: true
    property int studioTab: 0
    property bool restoringDraft: false
    property bool telegramConnected: telegramMode.currentIndex === 0
        ? Boolean(controller.settings.botConfigured)
        : Boolean(controller.settings.personalConfigured)
    property bool telegramReady: telegramConnected
        && destinationField.text.trim().length > 0

    signal fullScreenExitRequested()
    signal closeRequested()

    color: root.studioMode ? Theme.ink : Theme.surface

    function submit(action) {
        var presetCodes = [
            "original", "balanced", "fit_bot", "fit_x",
            "fit_both", "smallest", "custom"
        ]
        var sendTelegram = action !== "x"
        var prepareX = action !== "telegram"
        root.activeAction = action
        root.lastSubmitXEnabled = prepareX
        controller.publish({
            mediaId: controller.selectedMediaId,
            trimStart: root.trimStart,
            trimEnd: root.trimEnd,
            preset: presetCodes[compressionBox.currentIndex],
            targetMb: Number(targetField.text) || 0,
            telegramEnabled: sendTelegram,
            xEnabled: prepareX,
            telegramMode: telegramMode.currentIndex === 0 ? "bot" : "personal",
            telegramDestination: destinationField.text.trim(),
            telegramCaption: captionArea.text,
            xCaption: root.sameCaption ? captionArea.text : xCaptionArea.text,
            edits: videoEditor.editSpec(),
            cleanupPolicy: [
                "keep", "after_complete", "after_telegram"
            ][cleanupBox.currentIndex]
        })
    }

    function resetScrollPositions() {
        Qt.callLater(function() {
            if (regularScroll.contentItem)
                regularScroll.contentItem.contentY = 0
            if (studioEditScroll.contentItem)
                studioEditScroll.contentItem.contentY = 0
            if (studioPublishScroll.contentItem)
                studioPublishScroll.contentItem.contentY = 0
        })
    }

    function togglePlayback() {
        videoEditor.togglePlayback()
    }

    function captureDraft() {
        if (Number(controller.selectedMediaId || 0) <= 0)
            return {}
        return {
            mediaId: Number(controller.selectedMediaId || 0),
            trimStart: root.trimStart,
            trimEnd: root.trimEnd,
            sameCaption: root.sameCaption,
            studioTab: root.studioTab,
            telegramModeIndex: telegramMode.currentIndex,
            destination: destinationField.text,
            caption: captionArea.text,
            xCaption: xCaptionArea.text,
            compressionIndex: compressionBox.currentIndex,
            targetSize: targetField.text,
            cleanupIndex: cleanupBox.currentIndex,
            edits: videoEditor.editSpec()
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
        root.sameCaption = draft.sameCaption === undefined
            ? true : Boolean(draft.sameCaption)
        root.studioTab = Math.max(
            0,
            Math.min(1, Number(draft.studioTab || 0))
        )
        telegramMode.currentIndex = Math.max(
            0,
            Math.min(1, Number(draft.telegramModeIndex || 0))
        )
        destinationField.text = String(draft.destination || "")
        captionArea.text = String(draft.caption || "")
        xCaptionArea.text = String(draft.xCaption || "")
        compressionBox.currentIndex = Math.max(
            0,
            Math.min(6, Number(draft.compressionIndex || 0))
        )
        targetField.text = String(draft.targetSize || "")
        cleanupBox.currentIndex = Math.max(
            0,
            Math.min(2, Number(draft.cleanupIndex || 0))
        )
        videoEditor.loadEditSpec(draft.edits || {})
        cropPreset.currentIndex = videoEditor.editor.cropEnabled ? 0 : 1
        root.restoringDraft = false
        Qt.callLater(videoEditor.startAutoplay)
    }

    onStudioModeChanged: root.resetScrollPositions()

    Timer {
        interval: 900
        repeat: true
        running: Number(controller.selectedMediaId || 0) > 0
        onTriggered: {
            if (!root.restoringDraft)
                controller.saveActiveWorkspaceDraft(root.captureDraft())
        }
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
                root.loadedMediaId = nextId
                root.trimStart = 0
                root.trimEnd = Number(controller.selectedMedia.duration || 0)
                videoEditor.reset()
                cropPreset.currentIndex = 0
                root.studioTab = 0
                root.resetScrollPositions()
            } else if (root.trimEnd <= 0) {
                root.trimEnd = Number(controller.selectedMedia.duration || 0)
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

    Rectangle {
        id: prepareHeader
        visible: root.studioMode
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.studioMode ? 70 : 0
        z: 20
        color: Theme.surfaceSoft

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 24
            anchors.rightMargin: 20
            spacing: 7

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    text: "Full-screen editor"
                    color: Theme.text
                    font.pixelSize: Theme.textTitle
                    font.weight: Font.DemiBold
                    Layout.fillWidth: true
                }
                Text {
                    text: controller.selectedMedia.name || ""
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
            }

            StatusPill {
                visible: videoEditor.editor.hasEdits
                status: "accent"
                text: "Edited copy"
            }
            AppButton {
                text: "Open in default player"
                iconName: "external"
                kind: "ghost"
                compact: true
                onClicked: controller.openSelectedVideo()
            }
            AppButton {
                text: "Back to library"
                iconName: "chevronLeft"
                kind: "secondary"
                onClicked: root.fullScreenExitRequested()
            }
            AppButton {
                text: "Close selected video"
                iconName: "close"
                kind: "ghost"
                compact: true
                onClicked: root.closeRequested()
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

    StackLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: root.studioMode ? prepareHeader.bottom : parent.top
        anchors.bottom: parent.bottom
        currentIndex: root.studioMode ? 1 : 0

        ScrollView {
            id: regularScroll
            objectName: "prepareScroll"
            Component.onCompleted: contentItem.objectName = "prepareFlickable"
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AppScrollBar { }
            contentWidth: availableWidth

            GridLayout {
                id: regularColumn
                width: regularScroll.availableWidth
                columns: 1
                rowSpacing: 0
                columnSpacing: 0
            }
        }

        Item {
            id: studioWorkspace

            RowLayout {
                anchors.fill: parent
                spacing: 0

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Theme.ink

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 24
                        anchors.rightMargin: 24
                        anchors.topMargin: 20
                        anchors.bottomMargin: 20
                        spacing: 14

                        GridLayout {
                            id: studioCanvasColumn
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            columns: 1
                            rowSpacing: 10
                            columnSpacing: 0
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 1
                            color: Theme.border
                        }

                        GridLayout {
                            id: studioTimelineColumn
                            Layout.fillWidth: true
                            columns: 1
                            rowSpacing: 12
                            columnSpacing: 0
                        }
                    }
                }

                Rectangle {
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    color: Theme.border
                }

                Rectangle {
                    Layout.fillHeight: true
                    Layout.preferredWidth: Math.min(
                        520,
                        Math.max(400, root.width * 0.34)
                    )
                    color: Theme.surface

                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0

                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.leftMargin: 20
                            Layout.rightMargin: 20
                            Layout.topMargin: 16
                            Layout.bottomMargin: 14
                            spacing: 10

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 1
                                    Text {
                                        text: root.studioTab === 0
                                            ? "Edit tools" : "Delivery"
                                        color: Theme.text
                                        font.pixelSize: Theme.textSection
                                        font.weight: Font.DemiBold
                                    }
                                    Text {
                                        text: root.studioTab === 0
                                            ? "Changes affect only the generated copy"
                                            : "Compress, caption, and relay"
                                        color: Theme.muted
                                        font.pixelSize: Theme.textXs
                                    }
                                }
                                StatusPill {
                                    visible: root.studioTab === 0
                                        && videoEditor.editor.hasEdits
                                    status: "success"
                                    text: "Active"
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                AppButton {
                                    Layout.fillWidth: true
                                    text: "Edit"
                                    iconName: "crop"
                                    kind: root.studioTab === 0
                                        ? "secondary" : "ghost"
                                    onClicked: root.studioTab = 0
                                }
                                AppButton {
                                    Layout.fillWidth: true
                                    text: "Publish"
                                    iconName: "send"
                                    kind: root.studioTab === 1
                                        ? "secondary" : "ghost"
                                    onClicked: root.studioTab = 1
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 1
                            color: Theme.border
                        }

                        StackLayout {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            currentIndex: root.studioTab

                            ScrollView {
                                id: studioEditScroll
                                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                                ScrollBar.vertical: AppScrollBar { }
                                contentWidth: availableWidth

                                GridLayout {
                                    id: studioEditColumn
                                    width: studioEditScroll.availableWidth
                                    columns: 1
                                    rowSpacing: 0
                                    columnSpacing: 0
                                }
                            }

                            ScrollView {
                                id: studioPublishScroll
                                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                                ScrollBar.vertical: AppScrollBar { }
                                contentWidth: availableWidth

                                GridLayout {
                                    id: studioPublishColumn
                                    width: studioPublishScroll.availableWidth
                                    columns: 1
                                    rowSpacing: 0
                                    columnSpacing: 0
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Rectangle {
        id: checkingBanner
        parent: root.studioMode ? studioCanvasColumn : regularColumn
        visible: controller.selectedMediaChecking
        Layout.fillWidth: true
        Layout.row: 0
        Layout.column: 0
        Layout.preferredHeight: visible ? 42 : 0
        Layout.leftMargin: root.studioMode ? 0 : 22
        Layout.rightMargin: root.studioMode ? 0 : 22
        Layout.topMargin: visible ? (root.studioMode ? 0 : 16) : 0
        radius: Theme.radiusSm
        color: Theme.active
        border.width: 1
        border.color: Theme.border

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            spacing: 9

            AppIcon {
                Layout.preferredWidth: 17
                Layout.preferredHeight: 17
                name: "info"
                iconColor: Theme.accent
            }
            Text {
                text: "Checking this video before editing and delivery"
                color: Theme.text
                font.pixelSize: Theme.textXs
                Layout.fillWidth: true
            }
            AppProgressBar {
                Layout.preferredWidth: 54
                indeterminate: true
            }
        }
    }

    PrepareVideoEditor {
        id: videoEditor
        parent: root.studioMode ? studioCanvasColumn : regularColumn
        Layout.fillWidth: true
        Layout.fillHeight: root.studioMode
        Layout.row: 1
        Layout.column: 0
        Layout.leftMargin: root.studioMode ? 0 : 22
        Layout.rightMargin: root.studioMode ? 0 : 22
        Layout.topMargin: root.studioMode
            ? (checkingBanner.visible ? 0 : 0)
            : (checkingBanner.visible ? 8 : 16)
        Layout.bottomMargin: root.studioMode ? 0 : 0
        studioMode: root.studioMode
        trimStart: root.trimStart
        trimEnd: root.trimEnd
        onTrimStartEdited: function(seconds) {
            root.trimStart = seconds
        }
        onTrimEndEdited: function(seconds) {
            root.trimEnd = seconds
        }
    }

    ColumnLayout {
        id: mediaDetailsSection
        parent: root.studioMode ? studioTimelineColumn : regularColumn
        Layout.fillWidth: true
        Layout.row: root.studioMode ? 0 : 2
        Layout.column: 0
        Layout.leftMargin: root.studioMode ? 0 : 22
        Layout.rightMargin: root.studioMode ? 0 : 22
        Layout.topMargin: root.studioMode ? 0 : 20
        spacing: 8
        enabled: !controller.selectedMediaChecking

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                Text {
                    text: controller.selectedMedia.name || ""
                    color: Theme.text
                    font.pixelSize: root.studioMode
                        ? Theme.textSection : Theme.textBase
                    font.weight: Font.DemiBold
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
                Text {
                    text: [
                        controller.selectedMedia.sizeLabel || "",
                        controller.selectedMedia.durationLabel || "",
                        Number(controller.selectedMedia.width || 0) > 0
                            ? Number(controller.selectedMedia.width)
                                + "×"
                                + Number(controller.selectedMedia.height || 0)
                            : ""
                    ].filter(function(value) { return value.length > 0 }).join("  ·  ")
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    Layout.fillWidth: true
                }
            }

            AppButton {
                text: "Reveal in library"
                iconName: "target"
                kind: "ghost"
                compact: !root.studioMode
                onClicked: controller.revealSelectedInLibrary()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            AppIcon {
                Layout.preferredWidth: 15
                Layout.preferredHeight: 15
                name: "folder"
                iconColor: Theme.muted
            }
            Text {
                id: sourcePathText
                text: controller.selectedMedia.path || ""
                color: Theme.textSoft
                font.pixelSize: Theme.textXs
                elide: Text.ElideMiddle
                Layout.fillWidth: true
                ToolTip.visible: sourcePathHover.hovered
                ToolTip.text: text
                HoverHandler { id: sourcePathHover }
            }
        }
    }

    ColumnLayout {
        id: frameSection
        parent: root.studioMode ? studioEditColumn : regularColumn
        Layout.fillWidth: true
        Layout.row: root.studioMode ? 0 : 3
        Layout.column: 0
        Layout.leftMargin: root.studioMode ? 20 : 22
        Layout.rightMargin: root.studioMode ? 20 : 22
        Layout.topMargin: root.studioMode ? 20 : 18
        Layout.bottomMargin: root.studioMode ? 24 : 0
        spacing: 12
        enabled: !controller.selectedMediaChecking

        Rectangle {
            visible: !root.studioMode
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text {
                    text: "FRAME EDITS"
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    font.letterSpacing: 1.2
                }
                Text {
                    visible: root.studioMode
                    text: "Crop the image or cover part of the frame."
                    color: Theme.textSoft
                    font.pixelSize: Theme.textXs
                }
            }
            StatusPill {
                visible: videoEditor.editor.hasEdits
                status: "success"
                text: "Generated copy only"
            }
        }

        Text {
            visible: !root.studioMode
            text: "Crop the frame or place solid black masks. Drag a selection to move it, or drag its handle to resize it."
            color: Theme.muted
            font.pixelSize: Theme.textXs
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 8

            AppCheckBox {
                id: cropCheck
                text: "Enable crop"
                checked: videoEditor.editor.cropEnabled
                onToggled: {
                    videoEditor.editor.selectedShapeIndex = -1
                    videoEditor.editor.setCropEnabled(checked)
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                AppComboBox {
                    id: cropPreset
                    Layout.fillWidth: true
                    enabled: videoEditor.editor.cropEnabled
                    model: [
                        "Free crop", "Original frame", "Square (1:1)",
                        "Landscape (16:9)", "Portrait (9:16)"
                    ]
                    Accessible.name: "Crop aspect"
                    onActivated: {
                        videoEditor.editor.selectedShapeIndex = -1
                        if (currentIndex === 0)
                            videoEditor.editor.applyCropAspect(0)
                        else if (currentIndex === 1)
                            videoEditor.editor.resetCrop()
                        else if (currentIndex === 2)
                            videoEditor.editor.applyCropAspect(1)
                        else if (currentIndex === 3)
                            videoEditor.editor.applyCropAspect(16 / 9)
                        else
                            videoEditor.editor.applyCropAspect(9 / 16)
                    }
                }
                AppButton {
                    text: "Reset crop"
                    iconName: "refresh"
                    kind: "ghost"
                    compact: root.studioMode
                    enabled: videoEditor.editor.cropEnabled
                    onClicked: {
                        videoEditor.editor.selectedShapeIndex = -1
                        videoEditor.editor.resetCrop()
                        cropPreset.currentIndex = 1
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: "BLACK MASKS"
                color: Theme.muted
                font.pixelSize: Theme.textXs
                font.letterSpacing: 1.1
            }
            Text {
                text: "Place masks directly on the video, then drag to position and resize."
                color: Theme.muted
                font.pixelSize: Theme.textXs
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                AppButton {
                    text: "Add rectangle"
                    iconName: "rectangle"
                    Layout.fillWidth: true
                    onClicked: videoEditor.editor.addShape(false)
                }
                AppButton {
                    text: "Add square"
                    iconName: "square"
                    Layout.fillWidth: true
                    onClicked: videoEditor.editor.addShape(true)
                }
            }
            RowLayout {
                visible: videoEditor.editor.shapeCount > 0
                Layout.fillWidth: true
                spacing: 8

                AppComboBox {
                    id: shapePicker
                    Layout.fillWidth: true
                    model: videoEditor.editor.shapeModel
                    textRole: "shapeKind"
                    currentIndex: videoEditor.editor.selectedShapeIndex
                    Accessible.name: "Black shape selection"
                    onActivated: videoEditor.editor.selectedShapeIndex = currentIndex

                    delegate: ItemDelegate {
                        required property int index
                        required property string shapeKind
                        width: ListView.view
                            ? ListView.view.width : implicitWidth
                        text: shapeKind + " " + (index + 1)
                    }
                    contentItem: Text {
                        leftPadding: 8
                        text: videoEditor.editor.selectedShapeIndex >= 0
                            ? videoEditor.editor.shapeModel.get(
                                videoEditor.editor.selectedShapeIndex
                            ).shapeKind
                                + " "
                                + (videoEditor.editor.selectedShapeIndex + 1)
                            : "Choose a black mask"
                        color: Theme.text
                        font.pixelSize: Theme.textSm
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }
                }
                AppButton {
                    text: "Remove selected"
                    iconName: "trash"
                    kind: "danger"
                    compact: root.studioMode
                    enabled: videoEditor.editor.selectedShapeIndex >= 0
                    onClicked: videoEditor.editor.removeSelectedShape()
                }
            }
            AppButton {
                visible: videoEditor.editor.shapeCount > 0
                text: "Clear all masks"
                iconName: "trash"
                kind: "ghost"
                Layout.fillWidth: true
                onClicked: videoEditor.editor.clearShapes()
            }
            Text {
                visible: videoEditor.editor.shapeCount > 0
                text: videoEditor.editor.selectedShapeIndex >= 0
                    ? "The selected mask has an accent outline and resize handle."
                    : "Choose a mask above to move or resize it."
                color: Theme.muted
                font.pixelSize: Theme.textXs
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }
        }

        Item {
            visible: root.studioMode
            Layout.fillHeight: true
        }
    }

    ColumnLayout {
        id: publishSection
        parent: root.studioMode ? studioPublishColumn : regularColumn
        Layout.fillWidth: true
        Layout.row: root.studioMode ? 0 : 5
        Layout.column: 0
        Layout.leftMargin: root.studioMode ? 20 : 22
        Layout.rightMargin: root.studioMode ? 20 : 22
        Layout.topMargin: root.studioMode ? 20 : 18
        Layout.bottomMargin: 24
        spacing: 14
        enabled: !controller.selectedMediaChecking

        Rectangle {
            visible: !root.studioMode
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text {
                    text: "OUTPUT"
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    font.letterSpacing: 1.2
                }
                Text {
                    visible: root.studioMode
                    text: "Choose a destination-aware file size."
                    color: Theme.textSoft
                    font.pixelSize: Theme.textXs
                }
            }
            StatusPill {
                status: "neutral"
                text: controller.estimateOutputSize(
                    root.trimStart,
                    root.trimEnd,
                    [
                        "original", "balanced", "fit_bot", "fit_x",
                        "fit_both", "smallest", "custom"
                    ][compressionBox.currentIndex],
                    Number(targetField.text) || 0
                )
            }
        }

        AppComboBox {
            id: compressionBox
            Layout.fillWidth: true
            model: [
                "Original when possible", "Balanced", "Fit Telegram bot",
                "Fit X", "Fit both", "Smallest practical", "Custom size"
            ]
            currentIndex: 4
            Accessible.name: "Compression preset"
        }
        RowLayout {
            visible: compressionBox.currentIndex === 6
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: "Target file size"
                color: Theme.muted
                font.pixelSize: Theme.textXs
                Layout.fillWidth: true
            }
            AppField {
                id: targetField
                Layout.preferredWidth: 110
                placeholderText: "MB"
                inputMethodHints: Qt.ImhFormattedNumbersOnly
                Accessible.name: "Target megabytes"
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: "TELEGRAM"
                color: Theme.muted
                font.pixelSize: Theme.textXs
                font.letterSpacing: 1.2
                Layout.fillWidth: true
            }
            StatusPill {
                status: root.telegramReady ? "success" : "warning"
                text: root.telegramReady ? "Ready" : "Needs setup"
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            AppComboBox {
                id: telegramMode
                Layout.preferredWidth: 132
                model: ["Bot", "Personal"]
                currentIndex: controller.settings.telegram_mode === "personal"
                    ? 1 : 0
                onActivated: controller.setSetting(
                    "telegram_mode",
                    currentIndex === 0 ? "bot" : "personal"
                )
                Accessible.name: "Telegram account type"
            }
            AppField {
                id: destinationField
                Layout.fillWidth: true
                text: controller.settings.telegram_destination || ""
                placeholderText: telegramMode.currentIndex === 0
                    ? "@channel or chat ID" : "Username or chat ID"
                Accessible.name: "Telegram destination"
            }
        }
        Text {
            text: !root.telegramConnected
                ? (telegramMode.currentIndex === 0
                    ? "Connect a bot in Settings before sending."
                    : "Sign in under Settings before sending.")
                : destinationField.text.trim().length === 0
                    ? "Enter a Telegram destination before sending."
                    : telegramMode.currentIndex === 0
                        ? "Bot connected and ready."
                        : "Personal account connected and ready."
            color: root.telegramReady ? Theme.success : Theme.warning
            font.pixelSize: Theme.textXs
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Text {
            visible: (root.trimEnd - root.trimStart)
                > Number(controller.settings.x_duration_seconds || 140)
            text: "This cut is longer than your "
                + Number(controller.settings.x_duration_seconds || 140)
                + "-second X limit. Trim it, or change the limit in Settings if your account supports longer uploads."
            color: Theme.warning
            font.pixelSize: Theme.textXs
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Text {
            text: "CAPTION"
            color: Theme.muted
            font.pixelSize: Theme.textXs
            font.letterSpacing: 1.2
        }
        AppTextArea {
            id: captionArea
            Layout.fillWidth: true
            Layout.preferredHeight: root.studioMode ? 112 : 94
            color: Theme.text
            placeholderText: "Telegram message or shared caption"
            placeholderTextColor: Theme.muted
            wrapMode: TextEdit.Wrap
        }
        Text {
            text: "Telegram: " + captionArea.length + " / "
                + (telegramMode.currentIndex === 0 ? "1,024" : "4,096")
                + (root.sameCaption
                    ? "  ·  X: " + captionArea.length + " / 280" : "")
            color: captionArea.length
                > (telegramMode.currentIndex === 0 ? 1024 : 4096)
                || (root.sameCaption && captionArea.length > 280)
                ? Theme.warning : Theme.muted
            font.pixelSize: Theme.textXs
            Layout.alignment: Qt.AlignRight
        }
        AppCheckBox {
            text: "Use the same text for Telegram and X"
            checked: root.sameCaption
            onToggled: root.sameCaption = checked
        }
        AppTextArea {
            id: xCaptionArea
            visible: !root.sameCaption
            Layout.fillWidth: true
            Layout.preferredHeight: 96
            color: Theme.text
            placeholderText: "Separate X post text"
            placeholderTextColor: Theme.muted
            wrapMode: TextEdit.Wrap
        }
        Text {
            visible: xCaptionArea.visible
            text: "X: " + xCaptionArea.length + " / 280"
            color: xCaptionArea.length > 280
                ? Theme.warning : Theme.muted
            font.pixelSize: Theme.textXs
            Layout.alignment: Qt.AlignRight
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: "Generated file"
                color: Theme.muted
                font.pixelSize: Theme.textXs
                Layout.fillWidth: true
            }
            AppComboBox {
                id: cleanupBox
                Layout.preferredWidth: 176
                model: [
                    "Keep", "Trash when complete", "Trash after Telegram"
                ]
                currentIndex: controller.settings.cleanup_policy
                    === "after_complete"
                    ? 1
                    : controller.settings.cleanup_policy === "after_telegram"
                        ? 2 : 0
                onActivated: controller.setSetting(
                    "cleanup_policy",
                    [
                        "keep", "after_complete", "after_telegram"
                    ][currentIndex]
                )
                Accessible.name: "Generated file cleanup"
            }
        }

        ColumnLayout {
            visible: controller.publishState.active
                || (controller.publishState.stage || "").length > 0
            Layout.fillWidth: true
            spacing: 6

            AppProgressBar {
                Layout.fillWidth: true
                from: 0
                to: 1
                value: Number(controller.publishState.progress || 0)
            }
            RowLayout {
                Layout.fillWidth: true
                Text {
                    text: controller.publishState.stage || ""
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    Layout.fillWidth: true
                }
                Text {
                    text: Math.round(
                        Number(controller.publishState.progress || 0) * 100
                    ) + "%"
                    color: Theme.text
                    font.pixelSize: Theme.textXs
                }
            }
            Text {
                visible: (controller.publishState.error || "").length > 0
                text: controller.publishState.error || ""
                color: Theme.error
                wrapMode: Text.Wrap
                Layout.fillWidth: true
                font.pixelSize: Theme.textXs
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            AppButton {
                Layout.fillWidth: true
                iconName: "x"
                text: controller.publishState.active
                    && root.activeAction === "x"
                    ? "Working…" : "Prepare X"
                enabled: !controller.selectedMediaChecking
                    && !controller.publishState.active
                onClicked: root.submit("x")
            }
            AppButton {
                Layout.fillWidth: true
                iconName: "send"
                text: controller.publishState.active
                    && root.activeAction === "telegram"
                    ? "Working…" : "Send Telegram"
                enabled: !controller.selectedMediaChecking
                    && !controller.publishState.active
                    && root.telegramReady
                onClicked: root.submit("telegram")
            }
        }
        AppButton {
            Layout.fillWidth: true
            kind: "primary"
            iconName: "relay"
            text: controller.publishState.active
                && root.activeAction === "both"
                ? "Working…" : "Send + prepare X"
            enabled: !controller.selectedMediaChecking
                && !controller.publishState.active
                && root.telegramReady
            onClicked: root.submit("both")
        }
        AppButton {
            visible: controller.publishState.active
            Layout.fillWidth: true
            iconName: "close"
            text: "Cancel"
            onClicked: controller.cancelPublish()
        }

        Rectangle {
            visible: (controller.publishState.outputPath || "").length > 0
                && root.lastSubmitXEnabled
            Layout.fillWidth: true
            Layout.preferredHeight: xReadyColumn.implicitHeight + 28
            radius: Theme.radiusMd
            color: Theme.active
            border.width: 1
            border.color: Theme.accent

            ColumnLayout {
                id: xReadyColumn
                anchors.fill: parent
                anchors.margins: 14
                spacing: 8

                Text {
                    text: "X handoff is ready"
                    color: Theme.text
                    font.pixelSize: Theme.textSm
                    font.weight: Font.DemiBold
                }
                Text {
                    text: "The composer is open with your text. Paste or drag the video into it."
                    color: Theme.muted
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                    font.pixelSize: Theme.textXs
                }
                RowLayout {
                    Layout.fillWidth: true
                    AppButton {
                        text: "Copy video"
                        iconName: "copy"
                        Layout.fillWidth: true
                        onClicked: controller.copyVideoFile(
                            controller.publishState.outputPath
                        )
                    }
                    AppButton {
                        text: "Drag video"
                        iconName: "media"
                        Layout.fillWidth: true
                        onPressed: controller.startFileDrag(
                            controller.publishState.outputPath
                        )
                    }
                }
                AppButton {
                    text: "Show in folder"
                    iconName: "folder"
                    kind: "ghost"
                    Layout.fillWidth: true
                    onClicked: controller.revealPath(
                        controller.publishState.outputPath
                    )
                }
            }
        }
    }
}
