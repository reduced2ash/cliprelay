import QtQuick
import QtQuick.Window

Item {
    id: root

    required property var hostWindow
    readonly property real edgeSize: 7
    readonly property real cornerSize: 16
    readonly property bool windowed: hostWindow
        && hostWindow.visibility === Window.Windowed
    readonly property real moveLeftInset: Qt.platform.os === "osx" ? 78 : 8
    readonly property real moveRightInset:
        Qt.platform.os === "windows" ? 146 : 8

    z: 1000

    function beginResize(edges, mouse) {
        mouse.accepted = root.windowed
            && root.hostWindow.startSystemResize(edges)
    }

    // The extended client area removes the platform's normal draggable title
    // surface. Keep a narrow native move strip in the otherwise-empty top
    // gutter, clear of macOS traffic lights and Windows caption buttons.
    Item {
        x: root.moveLeftInset
        y: root.edgeSize
        width: Math.max(
            0,
            root.width - root.moveLeftInset - root.moveRightInset
        )
        height: 15
        visible: root.windowed

        DragHandler {
            target: null
            acceptedButtons: Qt.LeftButton
            onActiveChanged: {
                if (active)
                    root.hostWindow.startSystemMove()
            }
        }
    }

    MouseArea {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: root.cornerSize
        anchors.bottomMargin: root.cornerSize
        width: root.edgeSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeHorCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.LeftEdge, mouse)
        }
    }
    MouseArea {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: root.cornerSize
        anchors.bottomMargin: root.cornerSize
        width: root.edgeSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeHorCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.RightEdge, mouse)
        }
    }
    MouseArea {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: root.cornerSize
        anchors.rightMargin: root.cornerSize
        height: root.edgeSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeVerCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.TopEdge, mouse)
        }
    }
    MouseArea {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: root.cornerSize
        anchors.rightMargin: root.cornerSize
        height: root.edgeSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeVerCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.BottomEdge, mouse)
        }
    }

    MouseArea {
        anchors.left: parent.left
        anchors.top: parent.top
        width: root.cornerSize
        height: root.cornerSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeFDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.LeftEdge | Qt.TopEdge, mouse)
        }
    }
    MouseArea {
        anchors.right: parent.right
        anchors.top: parent.top
        width: root.cornerSize
        height: root.cornerSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeBDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.RightEdge | Qt.TopEdge, mouse)
        }
    }
    MouseArea {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        width: root.cornerSize
        height: root.cornerSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeBDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.LeftEdge | Qt.BottomEdge, mouse)
        }
    }
    MouseArea {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: root.cornerSize
        height: root.cornerSize
        visible: root.windowed
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.SizeFDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.RightEdge | Qt.BottomEdge, mouse)
        }
    }
}
