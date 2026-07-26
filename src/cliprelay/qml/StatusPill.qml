import QtQuick
import "."

Rectangle {
    id: root
    property string status: "neutral"
    property alias text: label.text
    property color statusColor: status === "success" ? Theme.success
        : status === "warning" ? Theme.warning
        : status === "error" ? Theme.error
        : status === "accent" ? Theme.accentText : Theme.muted
    property color statusBackground: status === "success" ? Theme.successSoft
        : status === "warning" ? Theme.warningSoft
        : status === "error" ? Theme.errorSoft
        : status === "accent" ? Theme.accentSoft : Theme.raised

    implicitWidth: label.implicitWidth + 24
    implicitHeight: 28
    radius: Theme.radiusSm
    color: root.statusBackground
    border.width: 0

    Row {
        anchors.centerIn: parent
        spacing: 5
        AppIcon {
            width: 13
            height: 13
            name: root.status === "error" || root.status === "warning"
                ? "warning" : root.status === "neutral" ? "info" : "check"
            strokeWidth: 2
            iconColor: root.statusColor
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            id: label
            color: root.status === "neutral" ? Theme.textSoft : root.statusColor
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
