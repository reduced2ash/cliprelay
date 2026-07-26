pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
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
    signal chosen(int mediaId)
    signal playbackRequested(int mediaId)
    signal navigationRequested(int direction)

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

    Component.onCompleted: Qt.callLater(root.requestThumbnail)
    onMediaIdChanged: Qt.callLater(root.requestThumbnail)
    onThumbnailUrlChanged: Qt.callLater(root.requestThumbnail)

    implicitHeight: poster.height + metaColumn.implicitHeight + 10
    activeFocusOnTab: true
    scale: tileTap.pressed ? 0.99 : 1
    Accessible.role: Accessible.ListItem
    Accessible.name: name + ", " + durationLabel + ", " + resolution
    Behavior on scale { NumberAnimation { duration: Theme.quickMotion; easing.type: Easing.OutQuart } }

    Rectangle {
        id: poster
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Math.max(132, width * 0.58)
        radius: Theme.radiusMd
        color: Theme.mediaWell
        border.width: root.selected || root.activeFocus ? 2 : 1
        border.color: root.selected || root.activeFocus
            ? Theme.accent
            : (hover.hovered ? Theme.borderStrong : Theme.border)
        clip: true
        Behavior on border.color { ColorAnimation { duration: Theme.fastMotion } }

        Image {
            id: thumbnail
            anchors.fill: parent
            anchors.margins: 2
            source: root.thumbnailUrl
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            cache: true
            visible: !previewLoader.active
            onStatusChanged: {
                if (status === Image.Error) Qt.callLater(root.requestThumbnail)
            }

            Rectangle {
                anchors.fill: parent
                color: Theme.raised
                visible: thumbnail.status !== Image.Ready
                Column {
                    anchors.centerIn: parent
                    spacing: 8
                    AppIcon {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 23
                        height: 23
                        name: root.thumbnailState === "failed"
                            || thumbnail.status === Image.Error ? "warning" : "media"
                        strokeWidth: 1.6
                        iconColor: root.thumbnailState === "failed"
                            || thumbnail.status === Image.Error ? Theme.warning : Theme.mutedSoft
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: root.thumbnailState === "failed"
                            || thumbnail.status === Image.Error
                            ? "Thumbnail unavailable"
                            : root.thumbnailState === "generating"
                                ? "Creating thumbnail"
                                : root.thumbnailState === "ready"
                                    ? "Loading thumbnail"
                                    : "Thumbnail queued"
                        color: Theme.muted
                        font.pixelSize: Theme.textXs
                    }
                }
                AppProgressBar {
                    visible: root.thumbnailState === "generating"
                        || root.thumbnailState === "queued"
                    indeterminate: true
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 20
                    anchors.rightMargin: 20
                    anchors.bottomMargin: 14
                }
            }
        }

        Loader {
            id: previewLoader
            anchors.fill: parent
            anchors.margins: 2
            active: controller.settings.hover_previews
                && hover.hovered && root.previewUrl.length > 0
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
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 8
            width: durationText.implicitWidth + 12
            height: 24
            radius: 5
            color: Theme.overlay
            Text {
                id: durationText
                anchors.centerIn: parent
                text: root.durationLabel
                color: Theme.mediaText
                font.pixelSize: Theme.textXs
                font.family: Qt.platform.os === "windows" ? "Consolas" : "Menlo"
            }
        }

        Rectangle {
            visible: root.postedCount > 0
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.margins: 8
            width: postedText.implicitWidth + 32
            height: 24
            radius: 5
            color: Theme.successSoft
            Row {
                anchors.centerIn: parent
                spacing: 4
                AppIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 12
                    height: 12
                    name: "check"
                    strokeWidth: 2.2
                    iconColor: Theme.success
                }
                Text {
                    id: postedText
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Posted " + root.postedCount
                    color: Theme.success
                    font.pixelSize: Theme.textXs
                    font.weight: Font.DemiBold
                }
            }
        }

        HoverHandler { id: hover }
        TapHandler {
            id: tileTap
            onTapped: {
                root.forceActiveFocus()
                root.chosen(root.mediaId)
            }
        }
        Timer {
            interval: 350
            running: controller.settings.hover_previews
                && hover.hovered && root.previewUrl.length === 0
            onTriggered: controller.ensurePreview(root.mediaId)
        }
    }

    ColumnLayout {
        id: metaColumn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: poster.bottom
        anchors.topMargin: 8
        spacing: 3
        Text {
            text: root.name
            color: root.selected ? Theme.text : Theme.textSoft
            font.pixelSize: Theme.textSm
            font.weight: root.selected ? Font.DemiBold : Font.Medium
            elide: Text.ElideMiddle
            Layout.fillWidth: true
        }
        Text {
            text: root.sizeLabel + "  ·  " + root.resolution + (root.folder.length ? "  ·  " + root.folder : "")
            color: Theme.muted
            font.pixelSize: Theme.textXs
            elide: Text.ElideMiddle
            Layout.fillWidth: true
        }
    }

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
