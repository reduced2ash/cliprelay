pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

ColumnLayout {
    id: root

    property var editor
    readonly property int maskCount: editor
        ? Number(editor.shapeCount || 0) : 0

    signal changed()

    spacing: 7

    Rectangle {
        id: maskFrame
        Layout.fillWidth: true
        Layout.preferredHeight: root.maskCount === 0
            ? 42 : Math.min(166, root.maskCount * 36)
        radius: Theme.radiusWorkbench
        color: Theme.raised
        border.width: 1
        border.color: Theme.border
        clip: true

        Text {
            visible: root.maskCount === 0
            anchors.fill: parent
            anchors.leftMargin: 11
            anchors.rightMargin: 11
            text: "No masks"
            color: Theme.muted
            font.pixelSize: Theme.textXs
            verticalAlignment: Text.AlignVCenter
        }

        ListView {
            id: maskList
            visible: root.maskCount > 0
            anchors.fill: parent
            model: root.editor ? root.editor.shapeModel : null
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar { }

            delegate: ItemDelegate {
                id: maskRow
                required property int index
                required property string shapeKind

                width: ListView.view ? ListView.view.width : 0
                height: 36
                hoverEnabled: true
                highlighted: root.editor
                    && root.editor.selectedShapeIndex === index
                onClicked: root.editor.selectedShapeIndex = index

                background: Rectangle {
                    color: maskRow.highlighted
                        ? Theme.active
                        : maskRow.hovered ? Theme.hover : "transparent"
                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: 1
                        color: Theme.border
                        visible: maskRow.index < root.maskCount - 1
                    }
                }

                contentItem: RowLayout {
                    spacing: 8

                    AppIcon {
                        Layout.preferredWidth: 14
                        Layout.preferredHeight: 14
                        name: maskRow.shapeKind === "Square"
                            ? "square" : "rectangle"
                        iconColor: maskRow.highlighted
                            ? Theme.accentText : Theme.muted
                    }
                    Text {
                        Layout.fillWidth: true
                        text: maskRow.shapeKind + " " + (maskRow.index + 1)
                        color: Theme.text
                        font.pixelSize: Theme.textSm
                        font.weight: maskRow.highlighted
                            ? Font.DemiBold : Font.Medium
                        elide: Text.ElideRight
                    }
                    AppButton {
                        visible: maskRow.highlighted
                        Layout.preferredWidth: 32
                        Layout.preferredHeight: 32
                        text: "Remove mask"
                        iconName: "trash"
                        kind: "danger"
                        compact: true
                        toolTipText: text
                        onClicked: {
                            root.editor.selectedShapeIndex = maskRow.index
                            root.editor.removeSelectedShape()
                            root.changed()
                        }
                    }
                }
            }
        }
    }

    AppButton {
        visible: root.maskCount > 1
        Layout.alignment: Qt.AlignRight
        text: "Clear all masks"
        iconName: "trash"
        kind: "ghost"
        onClicked: {
            root.editor.clearShapes()
            root.changed()
        }
    }
}
