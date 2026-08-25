import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root

    property int activeTab: 0
    property int editCount: 0
    property bool telegramReady: false

    signal tabSelected(int index)

    implicitHeight: Theme.prepareInspectorTabsHeight
    color: Theme.surfaceSoft

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 1
        color: Theme.border
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        TabButton {
            id: editTab
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 0
            Layout.preferredWidth: 1
            text: "Edit"
            checked: root.activeTab === 0
            hoverEnabled: true
            focusPolicy: Qt.StrongFocus
            Accessible.role: Accessible.PageTab
            Accessible.name: "Edit inspector"
            Accessible.selected: checked
            onClicked: root.tabSelected(0)

            contentItem: RowLayout {
                spacing: 7
                Item { Layout.fillWidth: true }
                AppIcon {
                    Layout.preferredWidth: 14
                    Layout.preferredHeight: 14
                    name: "crop"
                    iconColor: editTab.checked
                        ? Theme.accentText : Theme.muted
                }
                Text {
                    text: "Edit"
                    color: editTab.checked ? Theme.text : Theme.textSoft
                    font.pixelSize: Theme.textSm
                    font.weight: Font.DemiBold
                }
                Text {
                    text: root.editCount === 0
                        ? "No edits"
                        : root.editCount + (root.editCount === 1
                            ? " change" : " changes")
                    color: root.editCount > 0
                        ? Theme.accentText : Theme.muted
                    font.pixelSize: Theme.textXs
                    elide: Text.ElideRight
                }
                Item { Layout.fillWidth: true }
            }
            background: Rectangle {
                color: editTab.checked
                    ? Theme.active
                    : editTab.hovered ? Theme.hover : "transparent"
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: editTab.checked ? 2 : 1
                    color: editTab.checked ? Theme.accent : Theme.border
                }
            }
        }

        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 1
            color: Theme.border
        }

        TabButton {
            id: publishTab
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 0
            Layout.preferredWidth: 1
            text: "Publish"
            checked: root.activeTab === 1
            hoverEnabled: true
            focusPolicy: Qt.StrongFocus
            Accessible.role: Accessible.PageTab
            Accessible.name: "Publish inspector"
            Accessible.selected: checked
            onClicked: root.tabSelected(1)

            contentItem: RowLayout {
                spacing: 7
                Item { Layout.fillWidth: true }
                AppIcon {
                    Layout.preferredWidth: 14
                    Layout.preferredHeight: 14
                    name: "send"
                    iconColor: publishTab.checked
                        ? Theme.accentText : Theme.muted
                }
                Text {
                    text: "Publish"
                    color: publishTab.checked
                        ? Theme.text : Theme.textSoft
                    font.pixelSize: Theme.textSm
                    font.weight: Font.DemiBold
                }
                Text {
                    text: controller.publishState.active
                        ? "Working"
                        : (controller.publishState.error || "").length > 0
                            ? "Result"
                            : root.telegramReady ? "Ready" : "Needs setup"
                    color: controller.publishState.active
                        ? Theme.accentText
                        : root.telegramReady ? Theme.success : Theme.warning
                    font.pixelSize: Theme.textXs
                    elide: Text.ElideRight
                }
                Item { Layout.fillWidth: true }
            }
            background: Rectangle {
                color: publishTab.checked
                    ? Theme.active
                    : publishTab.hovered ? Theme.hover : "transparent"
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: publishTab.checked ? 2 : 1
                    color: publishTab.checked ? Theme.accent : Theme.border
                }
            }
        }
    }
}
