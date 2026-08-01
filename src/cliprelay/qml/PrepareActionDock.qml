import QtQuick
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    property int activeTab: 0
    property var editor
    property bool selectedMediaChecking: false
    property bool telegramReady: false
    property string estimatedSize: ""
    property string activeAction: ""
    property bool lastSubmitXEnabled: true
    readonly property bool xReady:
        (controller.publishState.outputPath || "").length > 0
        && lastSubmitXEnabled
    readonly property bool operationActive:
        Boolean(controller.publishState.active)
    readonly property bool wideActions: width >= 540

    signal submitRequested(string action)
    signal resetEditsRequested()

    implicitHeight: activeTab === 0
        ? 52
        : selectedMediaChecking ? 52
        : operationActive ? 78
        : xReady ? 104
        : wideActions ? 78 : 118
    color: Theme.surfaceSoft

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 1
        color: Theme.border
    }

    RowLayout {
        visible: root.activeTab === 0
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        spacing: 8

        AppIcon {
            Layout.preferredWidth: 15
            Layout.preferredHeight: 15
            name: "copy"
            iconColor: Theme.success
        }
        Text {
            Layout.fillWidth: true
            text: root.editor && root.editor.hasEdits
                ? "Original unchanged · edits apply to a generated copy"
                : "Original unchanged"
            color: Theme.textSoft
            font.pixelSize: Theme.textXs
            elide: Text.ElideRight
        }
        AppButton {
            visible: root.editor && root.editor.hasEdits
            text: "Reset frame edits"
            iconName: "refresh"
            kind: "ghost"
            compact: root.width < 470
            toolTipText: text
            onClicked: root.resetEditsRequested()
        }
    }

    RowLayout {
        visible: root.activeTab === 1 && root.selectedMediaChecking
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        spacing: 8

        AppIcon {
            Layout.preferredWidth: 15
            Layout.preferredHeight: 15
            name: "info"
            iconColor: Theme.accentText
        }
        Text {
            Layout.fillWidth: true
            text: "Publishing unlocks when the selected video is ready"
            color: Theme.textSoft
            font.pixelSize: Theme.textXs
            elide: Text.ElideRight
        }
    }

    ColumnLayout {
        visible: root.activeTab === 1 && !root.selectedMediaChecking
            && root.operationActive
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        anchors.topMargin: 8
        anchors.bottomMargin: 8
        spacing: 5

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: controller.publishState.stage || "Preparing generated copy"
                color: Theme.textSoft
                font.pixelSize: Theme.textXs
                elide: Text.ElideRight
            }
            Text {
                text: Math.round(
                    Number(controller.publishState.progress || 0) * 100
                ) + "%"
                color: Theme.text
                font.pixelSize: Theme.textXs
                font.features: { "tnum": 1 }
            }
            AppButton {
                text: "Cancel"
                iconName: "close"
                kind: "danger"
                compact: true
                toolTipText: text
                Layout.preferredWidth: 34
                Layout.preferredHeight: 34
                onClicked: controller.cancelPublish()
            }
        }
        AppProgressBar {
            Layout.fillWidth: true
            from: 0
            to: 1
            value: Number(controller.publishState.progress || 0)
        }
    }

    ColumnLayout {
        visible: root.activeTab === 1 && !root.selectedMediaChecking
            && !root.operationActive
            && root.xReady
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        anchors.topMargin: 8
        anchors.bottomMargin: 8
        spacing: 7

        RowLayout {
            Layout.fillWidth: true
            spacing: 7

            AppIcon {
                Layout.preferredWidth: 15
                Layout.preferredHeight: 15
                name: "check"
                iconColor: Theme.success
            }
            Text {
                Layout.fillWidth: true
                text: "X handoff ready · generated copy available"
                color: Theme.textSoft
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 7

            AppButton {
                Layout.fillWidth: true
                text: "Copy video"
                iconName: "copy"
                compact: root.width < 430
                toolTipText: text
                onClicked: controller.copyVideoFile(
                    controller.publishState.outputPath
                )
            }
            AppButton {
                Layout.fillWidth: true
                text: "Drag video"
                iconName: "media"
                compact: root.width < 430
                toolTipText: text
                onPressed: controller.startFileDrag(
                    controller.publishState.outputPath
                )
            }
            AppButton {
                Layout.fillWidth: true
                text: "Show in folder"
                iconName: "folder"
                compact: root.width < 430
                toolTipText: text
                onClicked: controller.revealPath(
                    controller.publishState.outputPath
                )
            }
        }
    }

    ColumnLayout {
        visible: root.activeTab === 1 && !root.selectedMediaChecking
            && !root.operationActive
            && !root.xReady && !root.wideActions
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        anchors.topMargin: 6
        anchors.bottomMargin: 6
        spacing: 4

        Text {
            Layout.fillWidth: true
            text: (controller.publishState.error || "").length > 0
                ? "Failed · " + controller.publishState.error
                : (root.telegramReady ? "Telegram ready" : "Telegram needs setup")
                    + "  ·  X manual  ·  " + root.estimatedSize
            color: (controller.publishState.error || "").length > 0
                ? Theme.error
                : root.telegramReady ? Theme.textSoft : Theme.warning
            font.pixelSize: Theme.textXs
            elide: Text.ElideRight
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 7

            AppButton {
                Layout.fillWidth: true
                Layout.preferredHeight: 40
                text: "Prepare X"
                iconName: "x"
                enabled: !root.selectedMediaChecking
                onClicked: root.submitRequested("x")
            }
            AppButton {
                Layout.fillWidth: true
                Layout.preferredHeight: 40
                text: "Send Telegram"
                iconName: "send"
                enabled: !root.selectedMediaChecking && root.telegramReady
                onClicked: root.submitRequested("telegram")
            }
        }
        AppButton {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            kind: "primary"
            text: "Send + prepare X"
            iconName: "relay"
            enabled: !root.selectedMediaChecking && root.telegramReady
            onClicked: root.submitRequested("both")
        }
    }

    ColumnLayout {
        visible: root.activeTab === 1 && !root.selectedMediaChecking
            && !root.operationActive
            && !root.xReady && root.wideActions
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 10
        anchors.topMargin: 8
        anchors.bottomMargin: 8
        spacing: 6

        Text {
            Layout.fillWidth: true
            text: (controller.publishState.error || "").length > 0
                ? "Failed · " + controller.publishState.error
                : (root.telegramReady ? "Telegram ready" : "Telegram needs setup")
                    + "  ·  X manual  ·  " + root.estimatedSize
            color: (controller.publishState.error || "").length > 0
                ? Theme.error
                : root.telegramReady ? Theme.textSoft : Theme.warning
            font.pixelSize: Theme.textXs
            elide: Text.ElideRight
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 7

            AppButton {
                Layout.fillWidth: true
                text: "Prepare X"
                iconName: "x"
                enabled: !root.selectedMediaChecking
                onClicked: root.submitRequested("x")
            }
            AppButton {
                Layout.fillWidth: true
                text: "Send Telegram"
                iconName: "send"
                enabled: !root.selectedMediaChecking && root.telegramReady
                onClicked: root.submitRequested("telegram")
            }
            AppButton {
                Layout.fillWidth: true
                kind: "primary"
                text: "Send + prepare X"
                iconName: "relay"
                enabled: !root.selectedMediaChecking && root.telegramReady
                onClicked: root.submitRequested("both")
            }
        }
    }
}
