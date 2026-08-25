pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

WorkbenchButton {
    id: root

    required property var appController
    readonly property var tasks: {
        const rows = []
        if (appController.scanning) {
            rows.push({
                "title": appController.scanRootName.length
                    ? "Library scan  ·  " + appController.scanRootName
                    : "Library scan",
                "detail": appController.scanMessage || "Updating the index",
                "icon": "refresh",
                "progress": appController.scanProgress,
                "indeterminate": appController.scanProgress < 0,
                "cancellable": true
            })
        }
        if (appController.randomPicking) {
            rows.push({
                "title": "Random selection",
                "detail": "Choosing and validating a video",
                "icon": "shuffle",
                "progress": -1,
                "indeterminate": true
            })
        }
        if (appController.selectedMediaChecking) {
            rows.push({
                "title": "Selected video",
                "detail": "Checking the file before preparation",
                "icon": "media",
                "progress": -1,
                "indeterminate": true
            })
        }
        if (appController.selectedMediaTimelineLoading) {
            rows.push({
                "title": "Timeline filmstrip",
                "detail": "Preparing seek thumbnails",
                "icon": "activity",
                "progress": -1,
                "indeterminate": true
            })
        }
        if (appController.publishState.active) {
            rows.push({
                "title": "Preparing delivery",
                "detail": appController.publishState.stage
                    || "Processing the selected video",
                "icon": "send",
                "progress": Number(
                    appController.publishState.progress || 0
                ),
                "indeterminate": false
            })
        }
        return rows
    }
    readonly property int activeCount: tasks.length
    property double lastClosedAt: 0

    text: activeCount
        ? activeCount + (activeCount === 1 ? " active task" : " active tasks")
        : "Background activity"
    iconName: "activity"
    iconOnly: true
    kind: activityPopup.opened ? "secondary" : "ghost"
    toolTipText: activeCount
        ? text : "No background work  ·  View activity"
    onClicked: {
        if (activityPopup.opened) {
            activityPopup.close()
            return
        }
        if (Date.now() - lastClosedAt < 180)
            return
        activityPopup.open()
    }

    Rectangle {
        visible: root.activeCount > 0
        anchors.right: parent.right
        anchors.rightMargin: 4
        anchors.top: parent.top
        anchors.topMargin: 4
        width: 6
        height: 6
        radius: 3
        color: Theme.accent
        border.width: 1
        border.color: Theme.ink
    }

    Popup {
        id: activityPopup
        objectName: "activityPopup"
        x: root.width - width
        y: root.height + 5
        width: 326
        height: Math.min(
            286,
            activityColumn.implicitHeight + 2
        )
        padding: 0
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape
            | Popup.CloseOnPressOutsideParent
        onClosed: root.lastClosedAt = Date.now()

        background: Rectangle {
            color: Theme.surfaceSoft
            radius: Theme.radiusSm
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            id: activityColumn
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 36
                Layout.leftMargin: 10
                Layout.rightMargin: 7
                spacing: 8
                Text {
                    Layout.fillWidth: true
                    text: "BACKGROUND ACTIVITY"
                    color: Theme.textSoft
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.75
                }
                Text {
                    text: root.activeCount
                        ? root.activeCount.toLocaleString() + " ACTIVE"
                        : "IDLE"
                    color: root.activeCount
                        ? Theme.accentText : Theme.muted
                    font.pixelSize: 10
                    font.family: Theme.monoFamily
                    font.weight: Font.Medium
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Theme.border
            }

            ColumnLayout {
                visible: root.activeCount > 0
                Layout.fillWidth: true
                spacing: 0

                Repeater {
                    model: root.tasks
                    delegate: Item {
                        id: taskRow
                        required property int index
                        required property var modelData
                        Layout.fillWidth: true
                        Layout.preferredHeight: 58

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            spacing: 8
                            AppIcon {
                                Layout.preferredWidth: 16
                                Layout.preferredHeight: 16
                                name: taskRow.modelData.icon
                                strokeWidth: 1.7
                                iconColor: Theme.accentText
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.minimumWidth: 0
                                spacing: 2
                                Text {
                                    Layout.fillWidth: true
                                    text: taskRow.modelData.title
                                    color: Theme.text
                                    font.pixelSize: Theme.textWorkbench
                                    font.weight: Font.Medium
                                    elide: Text.ElideRight
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: taskRow.modelData.detail
                                    color: Theme.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideMiddle
                                }
                                AppProgressBar {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 3
                                    from: 0
                                    to: 1
                                    value: Math.max(
                                        0,
                                        Number(taskRow.modelData.progress || 0)
                                    )
                                    indeterminate:
                                        taskRow.modelData.indeterminate
                                }
                            }
                            WorkbenchButton {
                                visible: Boolean(
                                    taskRow.modelData.cancellable
                                )
                                Layout.preferredWidth: 28
                                Layout.preferredHeight: 28
                                text: root.appController.scanCancelling
                                    ? "Stopping scan" : "Stop scan"
                                iconName: "square"
                                iconOnly: true
                                kind: "ghost"
                                enabled:
                                    !root.appController.scanCancelling
                                toolTipText: text
                                onClicked:
                                    root.appController.cancelScan()
                            }
                        }

                        Rectangle {
                            visible: taskRow.index < root.activeCount - 1
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom
                            anchors.leftMargin: 34
                            height: 1
                            color: Theme.border
                        }
                    }
                }
            }

            RowLayout {
                visible: root.activeCount === 0
                Layout.fillWidth: true
                Layout.preferredHeight: 58
                Layout.leftMargin: 11
                Layout.rightMargin: 11
                spacing: 9
                AppIcon {
                    Layout.preferredWidth: 16
                    Layout.preferredHeight: 16
                    name: "check"
                    strokeWidth: 1.9
                    iconColor: Theme.success
                }
                Text {
                    Layout.fillWidth: true
                    text: "No background work. The library is ready."
                    color: Theme.muted
                    font.pixelSize: Theme.textWorkbench
                    wrapMode: Text.Wrap
                }
            }
        }
    }
}
