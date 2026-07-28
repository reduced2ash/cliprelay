pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import QtMultimedia
import "."

Item {
    id: root
    required property int mediaId
    required property string name
    required property string thumbnailUrl
    required property string thumbnailState
    required property string previewUrl
    required property string durationLabel
    required property string sizeLabel
    required property string resolution
    required property string folder
    required property int postedCount
    property bool selected: false
    property bool compact: false
    property bool previewActive: false
    readonly property bool mediaUnchecked:
        durationLabel === "Unchecked" || resolution === "Unchecked"
    readonly property string metadataLabel: {
        var fields = [root.sizeLabel]
        fields.push(root.mediaUnchecked ? "Unchecked" : root.resolution)
        if (!root.compact && root.folder.length > 0)
            fields.push(root.folder)
        return fields.join("  ·  ")
    }

    signal chosen(int mediaId)
    signal playbackRequested(int mediaId)
    signal navigationRequested(int direction)
    signal previewRequested(int mediaId)
    signal previewReleased(int mediaId)

    function requestThumbnail() {
        if (
            root.mediaId > 0
            && (
                root.thumbnailUrl.length === 0
                || thumbnail.status === Image.Error
            )
            && root.thumbnailState !== "failed"
        ) {
            controller.ensureThumbnail(root.mediaId)
        }
    }

    function releasePreview() {
        hoverDelay.stop()
        root.previewReleased(root.mediaId)
    }

    Component.onCompleted: Qt.callLater(root.requestThumbnail)
    Component.onDestruction: root.releasePreview()
    onMediaIdChanged: Qt.callLater(root.requestThumbnail)
    onThumbnailUrlChanged: Qt.callLater(root.requestThumbnail)

    implicitHeight: poster.height + (
        compact
            ? Theme.libraryTileChromeCompact
            : Theme.libraryTileChromeDefault
    )
    activeFocusOnTab: true
    Accessible.role: Accessible.ListItem
    Accessible.name: name + ", " + metadataLabel

    Rectangle {
        id: poster
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Math.max(96, Math.round(width * 9 / 16))
        radius: Theme.libraryTileRadius
        color: Theme.mediaWell
        border.width: root.selected || root.activeFocus
            ? Theme.focusWidth : 1
        border.color: root.selected || root.activeFocus
            ? Theme.accent
            : tileHover.hovered ? Theme.borderStrong : Theme.border
        clip: true
        Behavior on border.color {
            ColorAnimation { duration: Theme.quickMotion }
        }

        Image {
            id: thumbnail
            anchors.fill: parent
            anchors.margins: root.selected || root.activeFocus ? 2 : 1
            source: root.thumbnailUrl
            sourceSize.width: Math.max(
                1,
                Math.ceil(width * Screen.devicePixelRatio)
            )
            sourceSize.height: Math.max(
                1,
                Math.ceil(height * Screen.devicePixelRatio)
            )
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            cache: true
            visible: !previewLoader.active
            onStatusChanged: {
                if (status === Image.Error)
                    Qt.callLater(root.requestThumbnail)
            }

            AppIcon {
                anchors.centerIn: parent
                width: root.compact ? 18 : 22
                height: width
                visible: !previewLoader.active
                    && (
                        root.thumbnailState === "failed"
                        || thumbnail.status === Image.Error
                    )
                name: "warning"
                strokeWidth: 1.6
                iconColor: Theme.warning
            }

            AppProgressBar {
                visible: !previewLoader.active
                    && (
                        root.thumbnailState === "generating"
                        || root.thumbnailState === "queued"
                    )
                indeterminate: true
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.leftMargin: root.compact ? 12 : 20
                anchors.rightMargin: root.compact ? 12 : 20
                anchors.bottomMargin: root.compact ? 8 : 12
            }
        }

        Loader {
            id: previewLoader
            anchors.fill: parent
            anchors.margins: root.selected || root.activeFocus ? 2 : 1
            active: controller.settings.hover_previews
                && root.previewActive
                && root.previewUrl.length > 0
            sourceComponent: Component {
                Item {
                    MediaPlayer {
                        id: player
                        source: root.previewUrl
                        audioOutput: AudioOutput { muted: true }
                        videoOutput: previewOutput
                        loops: MediaPlayer.Infinite
                        Component.onCompleted: play()
                    }
                    VideoOutput {
                        id: previewOutput
                        anchors.fill: parent
                        fillMode: VideoOutput.PreserveAspectFit
                    }
                }
            }
        }

        Rectangle {
            visible: !root.mediaUnchecked
                && root.durationLabel.length > 0
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: root.compact ? 5 : 7
            width: durationText.implicitWidth + (root.compact ? 8 : 10)
            height: root.compact ? 18 : 20
            radius: Theme.radiusWorkbench
            color: Theme.overlay

            Text {
                id: durationText
                anchors.centerIn: parent
                text: root.durationLabel
                color: Theme.mediaText
                font.pixelSize: root.compact ? 10 : Theme.textXs
                font.family: Theme.monoFamily
            }
        }

        Rectangle {
            visible: root.selected
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: root.compact ? 5 : 7
            width: root.compact ? 20 : 22
            height: width
            radius: Theme.radiusWorkbench
            color: Theme.accent

            AppIcon {
                anchors.centerIn: parent
                width: 13
                height: 13
                name: "check"
                strokeWidth: 2.3
                iconColor: Theme.accentContent
            }
        }

        HoverHandler {
            id: tileHover
            onHoveredChanged: {
                if (
                    hovered
                    && controller.settings.hover_previews
                ) {
                    hoverDelay.restart()
                } else {
                    root.releasePreview()
                }
            }
        }
        TapHandler {
            onTapped: {
                root.forceActiveFocus()
                root.chosen(root.mediaId)
            }
        }
        Timer {
            id: hoverDelay
            interval: Theme.libraryPreviewDelay
            repeat: false
            onTriggered: {
                if (!tileHover.hovered)
                    return
                controller.ensurePreview(root.mediaId)
                root.previewRequested(root.mediaId)
            }
        }
    }

    ColumnLayout {
        id: metaColumn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: poster.bottom
        anchors.topMargin: root.compact ? 5 : 7
        spacing: root.compact ? 1 : 2

        Text {
            id: nameText
            text: root.name
            color: root.selected ? Theme.text : Theme.textSoft
            font.pixelSize: root.compact ? Theme.textXs : Theme.textSm
            font.weight: root.selected ? Font.DemiBold : Font.Medium
            elide: Text.ElideMiddle
            Layout.fillWidth: true
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 5

            AppIcon {
                visible: root.mediaUnchecked
                Layout.preferredWidth: 12
                Layout.preferredHeight: 12
                name: "activity"
                strokeWidth: 1.5
                iconColor: Theme.mutedSoft
            }
            Text {
                id: metadataText
                text: root.metadataLabel
                color: Theme.muted
                font.pixelSize: root.compact ? 11 : Theme.textXs
                elide: Text.ElideMiddle
                Layout.fillWidth: true
            }
            RowLayout {
                visible: root.postedCount > 0
                spacing: 2

                AppIcon {
                    Layout.preferredWidth: 11
                    Layout.preferredHeight: 11
                    name: "check"
                    strokeWidth: 2
                    iconColor: Theme.success
                }
                Text {
                    text: root.postedCount.toLocaleString()
                    color: Theme.success
                    font.pixelSize: root.compact ? 11 : Theme.textXs
                }
            }
        }
    }

    ToolTip.visible: tileHover.hovered
        && (nameText.truncated || metadataText.truncated)
    ToolTip.text: root.folder.length
        ? root.folder + "/" + root.name : root.name
    ToolTip.delay: 600

    Keys.onSpacePressed: function(event) {
        root.playbackRequested(root.mediaId)
        event.accepted = true
    }
    Keys.onLeftPressed: function(event) {
        root.navigationRequested(-1)
        event.accepted = true
    }
    Keys.onRightPressed: function(event) {
        root.navigationRequested(1)
        event.accepted = true
    }
    Keys.onReturnPressed: root.chosen(root.mediaId)
}
