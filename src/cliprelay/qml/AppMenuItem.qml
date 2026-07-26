import QtQuick
import QtQuick.Controls
import "."

MenuItem {
    id: control
    property string iconName: ""
    property bool dangerous: false

    implicitHeight: visible ? 40 : 0
    leftPadding: 11
    rightPadding: 11
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true

    contentItem: Row {
        spacing: 9
        AppIcon {
            visible: control.iconName.length > 0
            anchors.verticalCenter: parent.verticalCenter
            width: 16
            height: 16
            name: control.iconName
            iconColor: control.dangerous
                ? Theme.error
                : control.highlighted ? Theme.text : Theme.muted
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: control.text
            color: control.dangerous
                ? Theme.error
                : control.enabled ? Theme.text : Theme.muted
            font.pixelSize: Theme.textSm
            elide: Text.ElideRight
        }
    }

    background: Rectangle {
        radius: Theme.radiusSm
        color: control.highlighted
            ? (control.dangerous ? Theme.errorSoft : Theme.active)
            : "transparent"
    }
}
