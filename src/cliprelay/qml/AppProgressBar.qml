import QtQuick
import QtQuick.Controls
import "."

ProgressBar {
    id: control
    implicitHeight: 7

    background: Rectangle {
        implicitWidth: 160
        implicitHeight: 5
        y: Math.round((control.height - height) / 2)
        radius: height / 2
        color: Theme.raised
    }

    contentItem: Item {
        clip: true
        Rectangle {
            id: movingBar
            width: control.indeterminate
                ? Math.max(34, parent.width * 0.28)
                : parent.width * control.visualPosition
            height: 5
            y: Math.round((parent.height - height) / 2)
            radius: height / 2
            color: Theme.accent
            x: control.indeterminate
                ? (travel * (parent.width + width) - width)
                : 0
            property real travel: 0
            NumberAnimation on travel {
                id: indeterminateAnimation
                running: control.visible && control.indeterminate
                from: 0
                to: 1
                duration: 1050
                loops: Animation.Infinite
                easing.type: Easing.InOutQuad
            }
        }
    }
}
