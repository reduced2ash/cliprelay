import QtQuick
import QtQuick.Controls
import "."

TextField {
    id: field
    property string iconName: ""
    implicitHeight: Theme.controlHeight
    leftPadding: iconName.length ? 40 : 13
    rightPadding: 13
    color: Theme.text
    placeholderTextColor: Theme.muted
    selectionColor: Theme.accent
    selectedTextColor: Theme.accentContent
    font.pixelSize: Theme.textSm
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.46

    background: Rectangle {
        radius: Theme.radiusSm
        color: Theme.raised
        border.width: field.activeFocus ? Theme.focusWidth : 1
        border.color: field.activeFocus ? Theme.accent : field.hovered ? Theme.borderStrong : Theme.border
        Behavior on border.color { ColorAnimation { duration: Theme.fastMotion } }
        AppIcon {
            visible: field.iconName.length > 0
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            width: 17
            height: 17
            name: field.iconName
            iconColor: field.activeFocus ? Theme.accent : Theme.muted
        }
    }
}
