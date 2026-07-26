import QtQuick
import QtQuick.Controls
import "."

RangeSlider {
    id: control
    implicitHeight: 34
    focusPolicy: Qt.StrongFocus

    background: Item {
        x: control.leftPadding
        y: Math.round((control.height - height) / 2)
        width: control.availableWidth
        height: 6

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Theme.active
        }
        Rectangle {
            x: control.first.visualPosition * parent.width
            width: Math.max(0, (control.second.visualPosition - control.first.visualPosition) * parent.width)
            height: parent.height
            radius: height / 2
            color: Theme.accent
        }
    }

    first.handle: Rectangle {
        x: control.leftPadding + control.first.visualPosition * (control.availableWidth - width)
        y: Math.round((control.height - height) / 2)
        width: control.first.pressed ? 20 : 18
        height: width
        radius: width / 2
        color: Theme.text
        border.width: 2
        border.color: control.activeFocus ? Theme.accent : Theme.raised
        Behavior on width { NumberAnimation { duration: Theme.quickMotion } }
    }

    second.handle: Rectangle {
        x: control.leftPadding + control.second.visualPosition * (control.availableWidth - width)
        y: Math.round((control.height - height) / 2)
        width: control.second.pressed ? 20 : 18
        height: width
        radius: width / 2
        color: Theme.text
        border.width: 2
        border.color: control.activeFocus ? Theme.accent : Theme.raised
        Behavior on width { NumberAnimation { duration: Theme.quickMotion } }
    }
}
