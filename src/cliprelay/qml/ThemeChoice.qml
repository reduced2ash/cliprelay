import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Button {
    id: control

    property string title: ""
    property string description: ""
    property bool selected: false
    property color previewInk: "#151416"
    property color previewSurface: "#1D1B1E"
    property color previewRaised: "#262328"
    property color previewTextColor: "#F1ECE8"
    property color previewAccent: "#F07858"
    property color previewBorder: "#3A343B"

    implicitWidth: 230
    implicitHeight: 92
    leftPadding: 11
    rightPadding: 11
    topPadding: 11
    bottomPadding: 11
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    scale: down ? 0.985 : 1

    Accessible.role: Accessible.RadioButton
    Accessible.name: title + " theme"
    Accessible.checked: selected

    contentItem: RowLayout {
        spacing: 11

        Rectangle {
            Layout.preferredWidth: 62
            Layout.preferredHeight: 58
            Layout.alignment: Qt.AlignVCenter
            radius: 8
            color: control.previewInk
            border.width: 1
            border.color: control.previewBorder
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: 15
                color: control.previewSurface
            }
            Rectangle {
                x: 22
                y: 12
                width: 29
                height: 5
                radius: 2
                color: control.previewTextColor
                opacity: 0.88
            }
            Rectangle {
                x: 22
                y: 22
                width: 23
                height: 4
                radius: 2
                color: control.previewTextColor
                opacity: 0.42
            }
            Rectangle {
                x: 22
                y: 35
                width: 31
                height: 13
                radius: 4
                color: control.previewRaised
                border.width: 1
                border.color: control.previewBorder
                Rectangle {
                    anchors.left: parent.left
                    anchors.leftMargin: 4
                    anchors.verticalCenter: parent.verticalCenter
                    width: 15
                    height: 4
                    radius: 2
                    color: control.previewAccent
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 3

            Text {
                Layout.fillWidth: true
                text: control.title
                color: Theme.text
                font.pixelSize: Theme.textSm
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                text: control.description
                color: control.selected ? Theme.textSoft : Theme.muted
                font.pixelSize: Theme.textXs
                elide: Text.ElideRight
            }
        }

        Rectangle {
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
            Layout.alignment: Qt.AlignTop
            radius: 10
            color: control.selected ? Theme.accent : "transparent"
            border.width: control.selected ? 0 : 1
            border.color: Theme.borderStrong

            AppIcon {
                anchors.centerIn: parent
                width: 13
                height: 13
                visible: control.selected
                name: "check"
                strokeWidth: 2.3
                iconColor: Theme.accentContent
            }
        }
    }

    background: Rectangle {
        radius: Theme.radiusMd
        color: control.selected
            ? Theme.accentSoft
            : control.hovered ? Theme.hover : Theme.surfaceSoft
        border.width: control.visualFocus || control.selected ? 2 : 1
        border.color: control.visualFocus || control.selected
            ? Theme.accent : control.hovered ? Theme.borderStrong : Theme.border
        Behavior on color { ColorAnimation { duration: Theme.fastMotion } }
        Behavior on border.color { ColorAnimation { duration: Theme.fastMotion } }
    }

    Behavior on scale {
        NumberAnimation {
            duration: Theme.quickMotion
            easing.type: Easing.OutQuart
        }
    }
}
