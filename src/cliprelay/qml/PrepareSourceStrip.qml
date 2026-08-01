import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    property bool studioMode: false
    property bool compactMode: false

    implicitHeight: root.compactMode ? 56 : Theme.prepareSourceHeight
    color: "transparent"

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 1
        color: Theme.border
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: root.compactMode ? 4 : 8
        anchors.bottomMargin: root.compactMode ? 4 : 7
        spacing: root.compactMode ? 2 : 3

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                Layout.fillWidth: true
                Layout.minimumWidth: 48
                text: controller.selectedMedia.name || ""
                color: Theme.text
                font.pixelSize: Theme.textSm
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
                ToolTip.visible: nameHover.hovered && truncated
                ToolTip.text: text
                HoverHandler { id: nameHover }
            }
            Text {
                visible: !root.compactMode && root.width >= 340
                text: [
                    controller.selectedMedia.sizeLabel || "",
                    controller.selectedMedia.durationLabel || "",
                    Number(controller.selectedMedia.width || 0) > 0
                        ? Number(controller.selectedMedia.width)
                            + "×"
                            + Number(controller.selectedMedia.height || 0)
                        : ""
                ].filter(function(value) {
                    return value.length > 0
                }).join("  ·  ")
                color: Theme.muted
                font.pixelSize: Theme.textXs
                font.features: { "tnum": 1 }
            }
            AppButton {
                text: "Reveal in library"
                iconName: "target"
                kind: "ghost"
                compact: root.width < 560
                toolTipText: text
                Layout.preferredWidth: root.compactMode ? 32 : implicitWidth
                Layout.preferredHeight: root.compactMode
                    ? 32 : Theme.compactControl
                onClicked: controller.revealSelectedInLibrary()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 7

            AppIcon {
                Layout.preferredWidth: 14
                Layout.preferredHeight: 14
                name: "folder"
                strokeWidth: 1.7
                iconColor: Theme.muted
            }
            Text {
                id: sourcePath
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                text: controller.selectedMedia.path || ""
                color: Theme.muted
                font.pixelSize: Theme.textXs
                elide: Text.ElideMiddle
                ToolTip.visible: pathHover.hovered && truncated
                ToolTip.text: text
                HoverHandler { id: pathHover }
            }
        }
    }
}
