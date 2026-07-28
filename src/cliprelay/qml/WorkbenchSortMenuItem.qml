pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Button {
    id: control

    property bool selected: false

    implicitWidth: 252
    implicitHeight: 31
    leftPadding: 8
    rightPadding: 8
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    Accessible.role: Accessible.MenuItem
    Accessible.name: text
    Accessible.checked: selected

    contentItem: RowLayout {
        spacing: 7

        Item {
            Layout.preferredWidth: 14
            Layout.preferredHeight: 14

            AppIcon {
                visible: control.selected
                anchors.centerIn: parent
                width: 13
                height: 13
                name: "check"
                strokeWidth: 2.2
                iconColor: Theme.accentText
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            text: control.text
            color: control.selected
                ? Theme.accentText
                : control.enabled ? Theme.textSoft : Theme.muted
            font.pixelSize: Theme.textWorkbench
            font.weight: control.selected ? Font.DemiBold : Font.Normal
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
    }

    background: Rectangle {
        radius: Theme.radiusWorkbench
        color: control.down
            ? Theme.active
            : control.hovered || control.visualFocus
                ? Theme.hover
                : control.selected ? Theme.accentSoft : "transparent"
        border.width: control.visualFocus ? Theme.focusWidth : 0
        border.color: Theme.accent

        Behavior on color {
            ColorAnimation { duration: Theme.fastMotion }
        }
    }
}
