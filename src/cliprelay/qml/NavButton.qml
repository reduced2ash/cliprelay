import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Button {
    id: control
    property bool selected: false
    property string iconName: ""
    property bool collapsed: false
    property string toolTipText: text

    implicitHeight: 48
    implicitWidth: collapsed ? 48 : 180
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    leftPadding: 0
    rightPadding: 0
    topPadding: 0
    bottomPadding: 0
    opacity: enabled ? 1 : 0.45

    Accessible.role: Accessible.Button
    Accessible.name: text

    contentItem: Item {
        AppIcon {
            visible: control.collapsed
            anchors.centerIn: parent
            width: 23
            height: 23
            name: control.iconName
            strokeWidth: 1.9
            iconColor: control.selected
                ? Theme.accent
                : control.hovered ? Theme.text : Theme.muted
        }

        RowLayout {
            visible: !control.collapsed
            anchors.fill: parent
            anchors.leftMargin: 13
            anchors.rightMargin: 13
            spacing: 11
            AppIcon {
                Layout.preferredWidth: 21
                Layout.preferredHeight: 21
                name: control.iconName
                iconColor: control.selected
                    ? Theme.accent
                    : control.hovered ? Theme.text : Theme.muted
            }
            Text {
                text: control.text
                color: control.selected || control.hovered ? Theme.text : Theme.muted
                font.pixelSize: Theme.textBase
                font.weight: control.selected ? Font.DemiBold : Font.Normal
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }
    }

    background: Rectangle {
        radius: control.collapsed ? 12 : Theme.radiusMd
        color: control.selected ? Theme.active : (control.hovered ? Theme.hover : "transparent")
        border.width: control.activeFocus ? 2 : 0
        border.color: Theme.accent
        Behavior on color { ColorAnimation { duration: Theme.fastMotion } }
    }

    ToolTip.visible: control.collapsed && control.hovered
    ToolTip.text: control.toolTipText
    ToolTip.delay: 450
}
