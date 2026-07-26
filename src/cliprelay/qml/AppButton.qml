import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Button {
    id: control
    property string kind: "secondary"
    property string iconName: ""
    property string leadingText: ""
    property bool compact: false
    property string toolTipText: text
    property bool iconOnly: compact

    implicitHeight: Theme.controlHeight
    implicitWidth: Math.max(iconOnly ? Theme.controlHeight : 104, contentRow.implicitWidth + (iconOnly ? 20 : 30))
    leftPadding: iconOnly ? 10 : 15
    rightPadding: iconOnly ? 10 : 15
    topPadding: 10
    bottomPadding: 10
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.46
    scale: down ? 0.985 : 1

    Accessible.role: Accessible.Button
    Accessible.name: text

    contentItem: RowLayout {
        id: contentRow
        spacing: 7
        Item { Layout.fillWidth: true; visible: control.iconOnly }
        AppIcon {
            visible: control.iconName.length > 0
            Layout.preferredWidth: control.iconOnly ? 20 : 17
            Layout.preferredHeight: control.iconOnly ? 20 : 17
            name: control.iconName
            strokeWidth: control.iconOnly ? 1.9 : 1.75
            iconColor: control.kind === "primary"
                ? Theme.accentContent
                : control.kind === "danger"
                    ? Theme.error
                    : control.hovered ? Theme.text : Theme.textSoft
            Layout.alignment: Qt.AlignVCenter
        }
        Text {
            visible: control.iconName.length === 0 && control.leadingText.length > 0
            text: control.leadingText
            color: control.kind === "primary"
                ? Theme.accentContent : Theme.textSoft
            font.pixelSize: 17
            Layout.alignment: Qt.AlignVCenter
        }
        Text {
            visible: !control.iconOnly
            text: control.text
            color: control.kind === "primary"
                ? Theme.accentContent
                : control.kind === "danger" ? Theme.error : Theme.text
            font.pixelSize: Theme.textSm
            font.weight: Font.DemiBold
            elide: Text.ElideRight
            Layout.alignment: Qt.AlignVCenter
        }
        Item { Layout.fillWidth: true; visible: control.iconOnly }
    }

    background: Rectangle {
        radius: control.iconOnly ? Theme.radiusMd : Theme.radiusSm
        color: {
            if (!control.enabled) return Theme.raised
            if (control.kind === "primary") return control.down ? Theme.accentPressed : Theme.accent
            if (control.kind === "danger") return control.hovered || control.down ? Theme.errorSoft : "transparent"
            if (control.kind === "ghost") return control.hovered || control.down ? Theme.active : "transparent"
            return control.hovered || control.down ? Theme.active : Theme.raised
        }
        border.width: control.activeFocus ? Theme.focusWidth : (control.kind === "secondary" ? 1 : 0)
        border.color: control.activeFocus ? Theme.accent : Theme.border
        Behavior on color { ColorAnimation { duration: Theme.fastMotion } }
    }

    ToolTip.visible: control.hovered
        && (control.iconOnly || control.toolTipText !== control.text)
    ToolTip.text: control.toolTipText
    ToolTip.delay: 450
    Behavior on scale { NumberAnimation { duration: Theme.quickMotion; easing.type: Easing.OutQuart } }
}
