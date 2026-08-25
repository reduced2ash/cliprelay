pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Button {
    id: root

    required property var actionRegistry
    required property var appController

    readonly property var videoOptions: [
        { "mode": "newest", "label": "Newest first", "shortLabel": "Newest" },
        { "mode": "oldest", "label": "Oldest first", "shortLabel": "Oldest" },
        { "mode": "name", "label": "Name", "shortLabel": "Name" },
        { "mode": "duration", "label": "Longest duration", "shortLabel": "Duration" },
        { "mode": "size", "label": "Largest size", "shortLabel": "Size" }
    ]
    readonly property var folderOptions: [
        { "mode": "name_asc", "label": "Alphabetical A–Z" },
        { "mode": "name_desc", "label": "Alphabetical Z–A" },
        { "mode": "added_recent", "label": "Recently added" },
        { "mode": "added_old", "label": "Least recently added" },
        { "mode": "recent", "label": "Recently updated" },
        { "mode": "stale", "label": "Least recently updated" },
        { "mode": "count_desc", "label": "Most videos" },
        { "mode": "count_asc", "label": "Fewest videos" }
    ]
    readonly property string currentVideoLabel: {
        const currentMode = String(
            root.appController.settings.sort_mode || "newest"
        )
        for (let index = 0; index < videoOptions.length; ++index) {
            if (videoOptions[index].mode === currentMode)
                return videoOptions[index].shortLabel
        }
        return "Newest"
    }

    implicitWidth: 112
    implicitHeight: Theme.workbenchControlHeight
    leftPadding: 9
    rightPadding: 8
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    Accessible.role: Accessible.ComboBox
    Accessible.name: "Sort videos and Explorer folders"
    Accessible.description: sortPopup.opened ? "Expanded" : "Collapsed"

    function focusCurrentVideoSort() {
        const currentMode = String(
            root.appController.settings.sort_mode || "newest"
        )
        for (let index = 0; index < videoRepeater.count; ++index) {
            if (root.videoOptions[index].mode === currentMode) {
                const item = videoRepeater.itemAt(index)
                if (item)
                    item.forceActiveFocus()
                return
            }
        }
    }

    onClicked: {
        if (sortPopup.opened)
            sortPopup.close()
        else
            sortPopup.open()
    }

    contentItem: RowLayout {
        spacing: 6

        Text {
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            text: root.currentVideoLabel
            color: Theme.text
            font.pixelSize: Theme.textWorkbench
            font.weight: Font.Medium
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        AppIcon {
            Layout.preferredWidth: 13
            Layout.preferredHeight: 13
            name: sortPopup.opened ? "chevronUp" : "chevronDown"
            strokeWidth: 1.8
            iconColor: sortPopup.opened || root.visualFocus
                ? Theme.accentText : Theme.muted
        }
    }

    background: Rectangle {
        radius: Theme.radiusWorkbench
        color: root.down
            ? Theme.active
            : root.hovered || sortPopup.opened
                ? Theme.hover : Theme.raised
        border.width: root.visualFocus || sortPopup.opened ? 2 : 1
        border.color: root.visualFocus || sortPopup.opened
            ? Theme.accent : Theme.border

        Behavior on color {
            ColorAnimation { duration: Theme.fastMotion }
        }
    }

    Popup {
        id: sortPopup

        x: root.width - width
        y: root.height + 4
        z: 120
        width: 268
        implicitHeight: sortContent.implicitHeight + topPadding + bottomPadding
        topPadding: 4
        bottomPadding: 4
        leftPadding: 4
        rightPadding: 4
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape
            | Popup.CloseOnPressOutsideParent
        onOpened: Qt.callLater(root.focusCurrentVideoSort)

        background: Rectangle {
            radius: Theme.radiusSm
            color: Theme.surfaceSoft
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: Column {
            id: sortContent
            width: sortPopup.availableWidth
            spacing: 0

            Item {
                width: parent.width
                height: 24

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: "VIDEOS"
                    color: Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }
            }

            Repeater {
                id: videoRepeater
                model: root.videoOptions

                delegate: WorkbenchSortMenuItem {
                    required property var modelData
                    width: sortContent.width
                    text: modelData.label
                    selected: String(
                        root.appController.settings.sort_mode
                    ) === modelData.mode
                    onClicked: {
                        if (!selected) {
                            root.actionRegistry.triggerAction(
                                "sort_" + modelData.mode
                            )
                        }
                        sortPopup.close()
                    }
                }
            }

            Rectangle {
                width: parent.width
                height: 1
                color: Theme.border
            }

            Item {
                width: parent.width
                height: 28

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: "EXPLORER FOLDERS"
                    color: Theme.muted
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.7
                }
            }

            Repeater {
                model: root.folderOptions

                delegate: WorkbenchSortMenuItem {
                    required property var modelData
                    width: sortContent.width
                    text: modelData.label
                    selected: String(
                        root.appController.settings.folder_sort_mode
                            || "name_asc"
                    ) === modelData.mode
                    onClicked: {
                        if (!selected) {
                            root.actionRegistry.triggerAction(
                                "folder_sort_" + modelData.mode
                            )
                        }
                        sortPopup.close()
                    }
                }
            }
        }
    }
}
