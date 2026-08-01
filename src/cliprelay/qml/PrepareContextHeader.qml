import QtQuick
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    property string fileName: ""
    property bool edited: false

    signal openRequested()
    signal exitRequested()
    signal closeRequested()

    implicitHeight: Theme.prepareStudioHeaderHeight
    color: Theme.surface

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 7
        spacing: 6

        AppIcon {
            Layout.preferredWidth: 14
            Layout.preferredHeight: 14
            name: "edit"
            strokeWidth: 1.8
            iconColor: Theme.accentText
        }
        Text {
            text: "Prepare"
            color: Theme.text
            font.pixelSize: Theme.textWorkbench
            font.weight: Font.DemiBold
        }
        Text {
            visible: root.fileName.length > 0
            text: "/"
            color: Theme.mutedSoft
            font.pixelSize: Theme.textWorkbench
        }
        Text {
            Layout.fillWidth: true
            Layout.minimumWidth: 40
            text: root.fileName
            color: Theme.textSoft
            font.pixelSize: Theme.textWorkbench
            elide: Text.ElideMiddle
        }

        RowLayout {
            visible: root.edited
            spacing: 4

            AppIcon {
                Layout.preferredWidth: 13
                Layout.preferredHeight: 13
                name: "crop"
                strokeWidth: 1.8
                iconColor: Theme.accentText
            }
            Text {
                text: "Edited"
                color: Theme.accentText
                font.pixelSize: Theme.textWorkbench
                font.weight: Font.DemiBold
            }
        }

        Rectangle {
            visible: root.edited
            Layout.preferredWidth: 1
            Layout.preferredHeight: 18
            color: Theme.border
        }

        WorkbenchButton {
            text: "Open in default player"
            iconName: "external"
            iconOnly: true
            toolTipText: text
            Layout.preferredWidth: 30
            Layout.preferredHeight: 30
            onClicked: root.openRequested()
        }
        WorkbenchButton {
            text: "Exit full-screen editor"
            iconName: "contract"
            iconOnly: true
            toolTipText: text + "  ·  Escape"
            Layout.preferredWidth: 30
            Layout.preferredHeight: 30
            onClicked: root.exitRequested()
        }
        WorkbenchButton {
            text: "Close selected video"
            iconName: "close"
            iconOnly: true
            toolTipText: text
            Layout.preferredWidth: 30
            Layout.preferredHeight: 30
            onClicked: root.closeRequested()
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.border
    }
}
