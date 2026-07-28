import QtQuick
import QtQuick.Controls
import "."

AbstractButton {
    id: root

    property string roleName: "close"
    property bool macStyle: false
    property bool windowActive: true
    property bool windowMaximized: false
    property bool windowFullscreen: false
    readonly property bool showGlyph: !macStyle || hovered || pressed
        || activeFocus
    readonly property color trafficColor: roleName === "close"
        ? "#FF5F57" : roleName === "minimize" ? "#FEBC2E" : "#28C840"
    readonly property string accessibleLabel: roleName === "close"
        ? "Close window" : roleName === "minimize"
            ? "Minimize window" : windowFullscreen
                ? "Exit full screen" : windowMaximized
                    ? "Restore window" : macStyle
                        ? "Enter full screen" : "Maximize window"

    implicitWidth: macStyle ? 34 : 46
    implicitHeight: 38
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    activeFocusOnTab: true

    Accessible.role: Accessible.Button
    Accessible.name: accessibleLabel

    ToolTip.visible: hovered
    ToolTip.text: accessibleLabel
    ToolTip.delay: 650

    background: Rectangle {
        color: {
            if (root.macStyle)
                return "transparent"
            if (root.roleName === "close" && root.hovered)
                return "#C8464B"
            if (root.pressed)
                return Theme.active
            return root.hovered || root.activeFocus
                ? Theme.hover : "transparent"
        }
        border.width: root.activeFocus ? 1 : 0
        border.color: Theme.accent

        Rectangle {
            visible: root.macStyle
            width: 13
            height: 13
            radius: 6.5
            anchors.centerIn: parent
            color: {
                if (!root.windowActive && !root.hovered)
                    return Theme.mutedSoft
                if (root.pressed)
                    return Qt.darker(root.trafficColor, 1.18)
                return root.trafficColor
            }
            border.width: 1
            border.color: Qt.rgba(0.08, 0.07, 0.08, 0.18)
        }

        Canvas {
            id: glyph
            anchors.centerIn: parent
            width: root.macStyle ? 10 : 16
            height: root.macStyle ? 10 : 16
            visible: root.showGlyph
            antialiasing: true

            Connections {
                target: root
                function onRoleNameChanged() { glyph.requestPaint() }
                function onShowGlyphChanged() { glyph.requestPaint() }
                function onWindowMaximizedChanged() { glyph.requestPaint() }
                function onWindowFullscreenChanged() { glyph.requestPaint() }
                function onHoveredChanged() { glyph.requestPaint() }
                function onPressedChanged() { glyph.requestPaint() }
            }

            onPaint: {
                const context = getContext("2d")
                context.clearRect(0, 0, width, height)
                context.save()
                context.scale(width / 16, height / 16)
                context.lineCap = "round"
                context.lineJoin = "round"
                context.lineWidth = root.macStyle ? 1.8 : 1.45
                context.strokeStyle = root.macStyle
                    ? Qt.rgba(0.08, 0.07, 0.08, 0.72)
                    : root.roleName === "close" && root.hovered
                        ? "#F7F9FC" : Theme.text

                if (root.roleName === "close") {
                    context.beginPath()
                    context.moveTo(4.5, 4.5)
                    context.lineTo(11.5, 11.5)
                    context.moveTo(11.5, 4.5)
                    context.lineTo(4.5, 11.5)
                    context.stroke()
                } else if (root.roleName === "minimize") {
                    context.beginPath()
                    context.moveTo(4, root.macStyle ? 8 : 10.5)
                    context.lineTo(12, root.macStyle ? 8 : 10.5)
                    context.stroke()
                } else if (
                    root.windowFullscreen
                    || (!root.macStyle && root.windowMaximized)
                ) {
                    context.beginPath()
                    context.moveTo(4, 7)
                    context.lineTo(7, 7)
                    context.lineTo(7, 4)
                    context.moveTo(12, 9)
                    context.lineTo(9, 9)
                    context.lineTo(9, 12)
                    context.stroke()
                } else if (root.macStyle) {
                    context.beginPath()
                    context.moveTo(4, 7)
                    context.lineTo(4, 4)
                    context.lineTo(7, 4)
                    context.moveTo(12, 9)
                    context.lineTo(12, 12)
                    context.lineTo(9, 12)
                    context.stroke()
                } else {
                    context.strokeRect(4, 4, 8, 8)
                }
                context.restore()
            }
        }
    }
}
