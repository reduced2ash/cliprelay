import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Button {
    id: control

    property string kind: "ghost"
    property string iconName: ""
    property string trailingIconName: ""
    property bool iconOnly: false
    property string toolTipText: text

    implicitHeight: Theme.workbenchControlHeight
    implicitWidth: iconOnly
        ? Theme.workbenchControlHeight
        : Math.max(52, contentRow.implicitWidth + 18)
    leftPadding: iconOnly ? 6 : 9
    rightPadding: iconOnly ? 6 : 9
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.42
    scale: down ? 0.97 : 1

    Accessible.role: Accessible.Button
    Accessible.name: text
    Accessible.description: toolTipText

    contentItem: Item {
        AppIcon {
            visible: control.iconOnly && control.iconName.length > 0
            anchors.centerIn: parent
            width: 16
            height: 16
            name: control.iconName
            strokeWidth: 1.85
            iconColor: control.kind === "primary"
                ? Theme.accentContent
                : control.enabled
                    ? (control.hovered || control.visualFocus
                        ? Theme.text : Theme.textSoft)
                    : Theme.mutedSoft
        }

        RowLayout {
            id: contentRow
            visible: !control.iconOnly
            anchors.fill: parent
            spacing: 6

            AppIcon {
                visible: control.iconName.length > 0
                Layout.preferredWidth: 15
                Layout.preferredHeight: 15
                Layout.alignment: Qt.AlignVCenter
                name: control.iconName
                strokeWidth: 1.7
                iconColor: control.kind === "primary"
                    ? Theme.accentContent
                    : control.enabled
                        ? (control.hovered || control.visualFocus
                            ? Theme.text : Theme.textSoft)
                        : Theme.mutedSoft
            }
            Text {
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                Layout.alignment: Qt.AlignVCenter
                text: control.text
                color: control.kind === "primary"
                    ? Theme.accentContent : Theme.text
                font.pixelSize: Theme.textWorkbench
                font.weight: control.kind === "primary"
                    ? Font.DemiBold : Font.Medium
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }
            AppIcon {
                visible: control.trailingIconName.length > 0
                Layout.preferredWidth: 13
                Layout.preferredHeight: 13
                Layout.alignment: Qt.AlignVCenter
                name: control.trailingIconName
                strokeWidth: 1.8
                iconColor: control.kind === "primary"
                    ? Theme.accentContent : Theme.muted
            }
        }
    }

    background: Rectangle {
        radius: Theme.radiusWorkbench
        color: {
            if (control.kind === "primary")
                return control.down ? Theme.accentPressed : Theme.accent
            if (control.down)
                return Theme.active
            if (control.hovered)
                return Theme.hover
            return control.kind === "secondary"
                ? Theme.raised : "transparent"
        }
        border.width: control.visualFocus
            ? Theme.focusWidth
            : control.kind === "secondary" ? 1 : 0
        border.color: control.visualFocus ? Theme.accent : Theme.border
        Behavior on color {
            ColorAnimation { duration: Theme.fastMotion }
        }
    }

    HoverHandler { id: toolTipHover }
    ToolTip.visible: toolTipHover.hovered
        && (control.iconOnly
            || control.toolTipText !== control.text
            || !control.enabled)
    ToolTip.text: control.toolTipText
    ToolTip.delay: 420

    Behavior on scale {
        NumberAnimation {
            duration: Theme.quickMotion
            easing.type: Easing.OutQuart
        }
    }
}
