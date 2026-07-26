import QtQuick
import QtQuick.Controls
import "."

CheckBox {
    id: control
    property color detailColor: Theme.muted

    implicitWidth: Math.max(44, contentItem.implicitWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(44, contentItem.implicitHeight + topPadding + bottomPadding)
    leftPadding: 0
    rightPadding: 8
    topPadding: 8
    bottomPadding: 8
    spacing: 10
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.44

    Accessible.role: Accessible.CheckBox
    Accessible.name: text

    indicator: Rectangle {
        x: control.leftPadding
        y: Math.round((control.height - height) / 2)
        width: 21
        height: 21
        radius: 5
        color: control.checked
            ? (control.down ? Theme.accentPressed : Theme.accent)
            : control.hovered ? Theme.active : Theme.raised
        border.width: control.activeFocus ? 2 : 1
        border.color: control.activeFocus
            ? Theme.accent
            : control.checked ? Theme.accent : Theme.borderStrong

        AppIcon {
            anchors.centerIn: parent
            width: 15
            height: 15
            visible: control.checked
            name: "check"
            strokeWidth: 2.4
            iconColor: Theme.accentContent
        }
        Behavior on color { ColorAnimation { duration: Theme.fastMotion } }
    }

    contentItem: Text {
        leftPadding: control.indicator.width + control.spacing
        text: control.text
        color: control.enabled ? Theme.text : Theme.muted
        font.pixelSize: Theme.textSm
        font.weight: Font.Medium
        verticalAlignment: Text.AlignVCenter
        wrapMode: Text.Wrap
    }
}
