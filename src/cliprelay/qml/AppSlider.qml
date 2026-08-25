import QtQuick
import QtQuick.Controls
import "."

Slider {
    id: control
    implicitHeight: 32
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    background: Rectangle {
        x: control.leftPadding
        y: Math.round((control.height - height) / 2)
        width: control.availableWidth
        height: 3
        radius: 1.5
        color: Theme.active

        Rectangle {
            width: control.visualPosition * parent.width
            height: parent.height
            radius: parent.radius
            color: Theme.accent
        }
    }

    handle: Rectangle {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: Math.round((control.height - height) / 2)
        width: control.pressed || control.visualFocus ? 17 : 15
        height: width
        radius: width / 2
        color: Theme.text
        border.width: control.visualFocus ? 3 : 2
        border.color: control.visualFocus ? Theme.accent : Theme.raised
        Behavior on width { NumberAnimation { duration: Theme.quickMotion } }
    }
}
