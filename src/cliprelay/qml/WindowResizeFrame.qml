import QtQuick

Item {
    id: root

    required property var windowController
    readonly property real edgeSize: 6
    readonly property real cornerSize: 15

    z: 10000
    visible: windowController.resizable

    function beginResize(edges, mouse) {
        mouse.accepted = windowController.beginResize(edges)
    }

    function updateResize(area) {
        if (area.pressed)
            windowController.updateResize()
    }

    function finishResize() {
        windowController.endResize()
    }

    MouseArea {
        id: leftEdge
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: root.cornerSize
        anchors.bottomMargin: root.cornerSize
        width: root.edgeSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeHorCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.LeftEdge, mouse)
        }
        onPositionChanged: root.updateResize(leftEdge)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }
    MouseArea {
        id: rightEdge
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: root.cornerSize
        anchors.bottomMargin: root.cornerSize
        width: root.edgeSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeHorCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.RightEdge, mouse)
        }
        onPositionChanged: root.updateResize(rightEdge)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }
    MouseArea {
        id: topEdge
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: root.cornerSize
        anchors.rightMargin: root.cornerSize
        height: root.edgeSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeVerCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.TopEdge, mouse)
        }
        onPositionChanged: root.updateResize(topEdge)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }
    MouseArea {
        id: bottomEdge
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: root.cornerSize
        anchors.rightMargin: root.cornerSize
        height: root.edgeSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeVerCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.BottomEdge, mouse)
        }
        onPositionChanged: root.updateResize(bottomEdge)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }

    MouseArea {
        id: topLeftCorner
        anchors.left: parent.left
        anchors.top: parent.top
        width: root.cornerSize
        height: root.cornerSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeFDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.LeftEdge | Qt.TopEdge, mouse)
        }
        onPositionChanged: root.updateResize(topLeftCorner)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }
    MouseArea {
        id: topRightCorner
        anchors.right: parent.right
        anchors.top: parent.top
        width: root.cornerSize
        height: root.cornerSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeBDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.RightEdge | Qt.TopEdge, mouse)
        }
        onPositionChanged: root.updateResize(topRightCorner)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }
    MouseArea {
        id: bottomLeftCorner
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        width: root.cornerSize
        height: root.cornerSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeBDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.LeftEdge | Qt.BottomEdge, mouse)
        }
        onPositionChanged: root.updateResize(bottomLeftCorner)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }
    MouseArea {
        id: bottomRightCorner
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: root.cornerSize
        height: root.cornerSize
        acceptedButtons: Qt.LeftButton
        preventStealing: true
        cursorShape: Qt.SizeFDiagCursor
        onPressed: function(mouse) {
            root.beginResize(Qt.RightEdge | Qt.BottomEdge, mouse)
        }
        onPositionChanged: root.updateResize(bottomRightCorner)
        onReleased: root.finishResize()
        onCanceled: root.finishResize()
    }
}
