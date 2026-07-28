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

    contentItem: RowLayout {
        id: contentRow
        spacing: 6

        Item {
            visible: control.iconOnly
            Layout.fillWidth: true
        }
        AppIcon {
            visible: control.iconName.length > 0
            Layout.preferredWidth: control.iconOnly ? 16 : 15
            Layout.preferredHeight: control.iconOnly ? 16 : 15
            Layout.alignment: Qt.AlignVCenter
            name: control.iconName
            strokeWidth: control.iconOnly ? 1.85 : 1.7
            iconColor: control.kind === "primary"
                ? Theme.accentContent
                : control.enabled
                    ? (control.hovered || control.activeFocus
                        ? Theme.text : Theme.textSoft)
                    : Theme.mutedSoft
        }
        Text {
            visible: !control.iconOnly
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
            visible: !control.iconOnly
                && control.trailingIconName.length > 0
            Layout.preferredWidth: 13
            Layout.preferredHeight: 13
            Layout.alignment: Qt.AlignVCenter
            name: control.trailingIconName
            strokeWidth: 1.8
            iconColor: control.kind === "primary"
                ? Theme.accentContent : Theme.muted
        }
        Item {
            visible: control.iconOnly
            Layout.fillWidth: true
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
        border.width: control.activeFocus
            ? Theme.focusWidth
            : control.kind === "secondary" ? 1 : 0
        border.color: control.activeFocus ? Theme.accent : Theme.border
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
