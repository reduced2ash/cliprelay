import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    property bool studioMode: false
    property bool compactMode: false

    implicitHeight: root.compactMode ? 44 : Theme.prepareSourceHeight
    color: "transparent"

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 1
        color: Theme.border
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 2
        anchors.rightMargin: 2
        anchors.topMargin: 5
        anchors.bottomMargin: 5
        spacing: 8

        Text {
            Layout.preferredWidth: Math.max(
                92, Math.min(230, root.width * 0.28)
            )
            Layout.minimumWidth: 72
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
            visible: !root.compactMode && root.width >= 560
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

        Rectangle {
            visible: !root.compactMode && root.width >= 560
            Layout.preferredWidth: 1
            Layout.preferredHeight: 18
            color: Theme.border
        }

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
            Layout.minimumWidth: 40
            text: controller.selectedMedia.path || ""
            color: Theme.muted
            font.pixelSize: Theme.textXs
            elide: Text.ElideMiddle
            ToolTip.visible: pathHover.hovered && truncated
            ToolTip.text: text
            HoverHandler { id: pathHover }
        }
        AppButton {
            text: "Reveal in library"
            iconName: "target"
            kind: "ghost"
            compact: root.compactMode || root.width < 760
            toolTipText: text
            Layout.preferredHeight: Theme.compactControl
            onClicked: controller.revealSelectedInLibrary()
        }
    }
}
