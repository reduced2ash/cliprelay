import QtQuick
import QtQuick.Layouts
import "."

ColumnLayout {
    id: root
    property string iconName: "media"
    property string title: ""
    property string body: ""
    property string actionText: ""
    signal action()

    width: 440
    spacing: 11

    Rectangle {
        Layout.alignment: Qt.AlignHCenter
        Layout.preferredWidth: 58
        Layout.preferredHeight: 58
        radius: 16
        color: Theme.active
        AppIcon {
            anchors.centerIn: parent
            width: 27
            height: 27
            name: root.iconName
            strokeWidth: 1.8
            iconColor: Theme.accent
        }
    }
    Text {
        text: root.title
        color: Theme.text
        font.pixelSize: Theme.textTitle
        font.weight: Font.DemiBold
        horizontalAlignment: Text.AlignHCenter
        Layout.fillWidth: true
    }
    Text {
        text: root.body
        color: Theme.muted
        font.pixelSize: Theme.textSm
        lineHeight: 1.25
        wrapMode: Text.Wrap
        horizontalAlignment: Text.AlignHCenter
        Layout.fillWidth: true
    }
    AppButton {
        visible: root.actionText.length > 0
        text: root.actionText
        iconName: "chevronRight"
        kind: "primary"
        Layout.alignment: Qt.AlignHCenter
        Layout.topMargin: 3
        onClicked: root.action()
    }
}
