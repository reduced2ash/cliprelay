import QtQuick
import "."

Item {
    id: root

    required property var hostWindow
    required property var windowController
    readonly property bool macStyle: Qt.platform.os === "osx"

    height: 38

    Rectangle {
        anchors.fill: parent
        color: Theme.ink
    }

    MouseArea {
        id: moveArea
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        preventStealing: true

        onPressed: function(mouse) {
            mouse.accepted = root.windowController.beginMove()
        }
        onPositionChanged: function(mouse) {
            if (pressed)
                root.windowController.updateMove()
        }
        onReleased: root.windowController.endMove()
        onCanceled: root.windowController.endMove()
        onDoubleClicked: function(mouse) {
            root.windowController.endMove()
            root.windowController.toggleZoom()
            mouse.accepted = true
        }
    }

    Text {
        anchors.centerIn: parent
        width: Math.min(280, Math.max(0, parent.width - 260))
        text: root.hostWindow.title
        color: root.windowController.active ? Theme.textSoft : Theme.mutedSoft
        font.pixelSize: Theme.textSm
        font.weight: Font.Medium
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    Row {
        visible: root.macStyle
        anchors.left: parent.left
        anchors.leftMargin: 7
        anchors.verticalCenter: parent.verticalCenter
        spacing: 0
        z: 2

        WindowControlButton {
            roleName: "close"
            macStyle: true
            windowActive: root.windowController.active
            onClicked: root.windowController.closeWindow()
        }
        WindowControlButton {
            roleName: "minimize"
            macStyle: true
            windowActive: root.windowController.active
            onClicked: root.windowController.minimizeWindow()
        }
        WindowControlButton {
            roleName: "zoom"
            macStyle: true
            windowActive: root.windowController.active
            windowMaximized: root.windowController.maximized
            windowFullscreen: root.windowController.fullscreen
            onClicked: root.windowController.performPrimaryZoom()
        }
    }

    Row {
        visible: !root.macStyle
        anchors.right: parent.right
        anchors.top: parent.top
        height: parent.height
        z: 2

        WindowControlButton {
            roleName: "minimize"
            windowActive: root.windowController.active
            onClicked: root.windowController.minimizeWindow()
        }
        WindowControlButton {
            roleName: "zoom"
            windowActive: root.windowController.active
            windowMaximized: root.windowController.maximized
            onClicked: root.windowController.toggleZoom()
        }
        WindowControlButton {
            roleName: "close"
            windowActive: root.windowController.active
            onClicked: root.windowController.closeWindow()
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.border
        opacity: root.windowController.active ? 0.86 : 0.55
    }
}
