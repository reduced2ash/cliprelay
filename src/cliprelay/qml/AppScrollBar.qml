import QtQuick
import QtQuick.Controls
import "."

ScrollBar {
    id: control
    policy: ScrollBar.AsNeeded
    implicitWidth: 10
    implicitHeight: 10
    padding: 2
    minimumSize: 0.08
    visible: size < 0.999

    contentItem: Rectangle {
        implicitWidth: 5
        implicitHeight: 5
        radius: Math.min(width, height) / 2
        color: control.pressed || control.hovered ? Theme.muted : Theme.borderStrong
        opacity: control.active ? 0.82 : 0
        Behavior on color { ColorAnimation { duration: Theme.fastMotion } }
        Behavior on opacity { NumberAnimation { duration: Theme.stateMotion } }
    }
    background: Item { }
}
