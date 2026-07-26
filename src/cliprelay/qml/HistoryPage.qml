pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    color: Theme.ink

    function prettyDate(value) {
        var date = new Date(value)
        if (isNaN(date.getTime())) return value
        return date.toLocaleString(Qt.locale(), "MMM d, yyyy · h:mm AP")
    }

    function prettyStatus(value) {
        return String(value || "").replace(/_/g, " ")
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 82
            Layout.leftMargin: 26; Layout.rightMargin: 24
            ColumnLayout {
                Layout.fillWidth: true; spacing: 2
                Text { text: "Relay history"; color: Theme.text; font.pixelSize: Theme.textTitle; font.weight: Font.DemiBold }
                Text { text: "Every post prepared through this app stays visible here"; color: Theme.muted; font.pixelSize: Theme.textXs }
            }
            AppField {
                id: historySearch
                Layout.preferredWidth: 300
                iconName: "search"
                placeholderText: "Search history"
                onTextChanged: historyTimer.restart()
                Accessible.name: "Search post history"
            }
            Timer { id: historyTimer; interval: 180; onTriggered: controller.setHistorySearch(historySearch.text) }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            AppEmptyState {
                visible: historyList.count === 0
                anchors.centerIn: parent
                width: Math.min(420, parent.width - 48)
                iconName: historySearch.text.length ? "search" : "history"
                title: historySearch.text.length ? "No matching relays" : "Nothing relayed yet"
                body: historySearch.text.length
                    ? "Try another filename or caption."
                    : "Telegram sends and X handoffs will appear here with their exact status and generated file."
                actionText: historySearch.text.length ? "" : "Open library"
                onAction: controller.navigationRequested("library")
            }

            ListView {
                id: historyList
                width: Math.min(1180, parent.width - 40)
                height: parent.height - 32
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.verticalCenter: parent.verticalCenter
                clip: true
                spacing: 0
                model: historyModel
                reuseItems: true
                ScrollBar.vertical: AppScrollBar { }
                onAtYEndChanged: {
                    if (atYEnd) controller.loadMoreHistory()
                }
                delegate: Rectangle {
                    id: historyRow
                    required property int postId
                    required property string createdAt
                    required property string mediaName
                    required property string thumbnailUrl
                    required property string telegramStatus
                    required property string xStatus
                    required property string caption
                    required property string exportPath
                    required property string sourcePath
                    required property string errorText
                    required property bool canTrash
                    required property string statusSummary
                    required property string xUrl
                    required property string telegramUrl
                    required property bool edited
                    width: ListView.view.width
                    height: Math.max(132, content.implicitHeight + 28)
                    color: rowHover.hovered ? Theme.surface : "transparent"
                    border.width: 0

                    RowLayout {
                        id: content
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 16
                        Rectangle {
                            Layout.preferredWidth: 150
                            Layout.preferredHeight: 92
                            radius: Theme.radiusMd
                            color: Theme.surface
                            clip: true
                            Image {
                                anchors.fill: parent; anchors.margins: 1
                                source: historyRow.thumbnailUrl
                                fillMode: Image.PreserveAspectFit
                                asynchronous: true
                            }
                            AppIcon {
                                visible: historyRow.thumbnailUrl.length === 0
                                anchors.centerIn: parent
                                width: 25
                                height: 25
                                name: "media"
                                iconColor: Theme.mutedSoft
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignVCenter
                            spacing: 6
                            RowLayout {
                                Layout.fillWidth: true
                                Text { text: historyRow.mediaName; color: Theme.text; font.pixelSize: Theme.textBase; font.weight: Font.DemiBold; elide: Text.ElideMiddle; Layout.fillWidth: true }
                                Text { text: root.prettyDate(historyRow.createdAt); color: Theme.muted; font.pixelSize: Theme.textXs }
                            }
                            Text { text: historyRow.caption.length ? historyRow.caption : "No caption"; color: historyRow.caption.length ? Theme.text : Theme.muted; font.pixelSize: Theme.textSm; elide: Text.ElideRight; Layout.fillWidth: true }
                            RowLayout {
                                spacing: 8
                                StatusPill {
                                    visible: historyRow.edited
                                    text: "Edited copy"
                                    status: "accent"
                                }
                                StatusPill {
                                    visible: historyRow.telegramStatus !== "not_requested"
                                    text: "Telegram " + root.prettyStatus(historyRow.telegramStatus)
                                    status: historyRow.telegramStatus === "sent" ? "success" : historyRow.telegramStatus === "failed" ? "error" : "warning"
                                }
                                StatusPill {
                                    visible: historyRow.xStatus !== "not_requested"
                                    text: "X " + root.prettyStatus(historyRow.xStatus)
                                    status: historyRow.xStatus === "posted" ? "success" : historyRow.xStatus === "failed" ? "error" : "accent"
                                }
                            }
                            Text { visible: historyRow.errorText.length > 0; text: historyRow.errorText; color: Theme.error; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                        }
                        ColumnLayout {
                            Layout.preferredWidth: 166
                            Layout.maximumWidth: 166
                            Layout.alignment: Qt.AlignVCenter
                            spacing: 6
                            AppButton {
                                text: "View"
                                iconName: "play"
                                Layout.fillWidth: true
                                onClicked: controller.viewHistoryPost(historyRow.postId)
                            }
                            AppButton { visible: historyRow.telegramStatus === "failed"; text: "Retry Telegram"; Layout.fillWidth: true; onClicked: controller.retryTelegram(historyRow.postId) }
                            AppButton { visible: historyRow.xStatus === "prepared"; text: "Mark X posted"; kind: "primary"; Layout.fillWidth: true; onClicked: controller.markXPosted(historyRow.postId, "") }
                            AppButton {
                                text: "More actions"
                                iconName: "more"
                                kind: "ghost"
                                Layout.fillWidth: true
                                onClicked: actionMenu.open()
                            }
                            Menu {
                                id: actionMenu
                                width: 224
                                padding: 5
                                background: Rectangle {
                                    radius: Theme.radiusMd
                                    color: Theme.surfaceSoft
                                    border.width: 1
                                    border.color: Theme.borderStrong
                                }
                                AppMenuItem {
                                    visible: historyRow.xStatus === "prepared" || historyRow.xStatus === "failed"
                                    text: "Prepare X again"
                                    iconName: "x"
                                    onTriggered: controller.prepareXAgain(historyRow.postId)
                                }
                                AppMenuItem {
                                    visible: historyRow.telegramUrl.length > 0
                                    text: "Open Telegram post"
                                    iconName: "external"
                                    onTriggered: controller.openUrl(historyRow.telegramUrl)
                                }
                                AppMenuItem {
                                    visible: historyRow.xUrl.length > 0
                                    text: "Open X post"
                                    iconName: "external"
                                    onTriggered: controller.openUrl(historyRow.xUrl)
                                }
                                AppMenuItem {
                                    text: "Show video in folder"
                                    iconName: "folder"
                                    onTriggered: controller.revealPath(historyRow.exportPath.length ? historyRow.exportPath : historyRow.sourcePath)
                                }
                                AppMenuItem {
                                    visible: historyRow.canTrash
                                    text: "Move generated video to Trash"
                                    iconName: "trash"
                                    dangerous: true
                                    onTriggered: controller.trashExport(historyRow.postId)
                                }
                            }
                        }
                    }
                    Rectangle { anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; height: 1; color: Theme.border }
                    HoverHandler { id: rowHover }
                }
            }
        }
    }
}
