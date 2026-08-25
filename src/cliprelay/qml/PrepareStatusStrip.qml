import QtQuick
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    property bool checking: false

    visible: checking
    implicitHeight: visible ? 34 : 0
    color: Theme.accentSoft

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        spacing: 8

        AppIcon {
            Layout.preferredWidth: 15
            Layout.preferredHeight: 15
            name: "info"
            iconColor: Theme.accentText
        }
        Text {
            Layout.fillWidth: true
            text: "Checking selected video"
            color: Theme.textSoft
            font.pixelSize: Theme.textXs
            font.weight: Font.Medium
        }
        AppProgressBar {
            Layout.preferredWidth: 58
            indeterminate: true
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
