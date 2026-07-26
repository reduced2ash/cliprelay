pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia
import "."

ColumnLayout {
    id: root

    property bool studioMode: false
    property real trimStart: 0
    property real trimEnd: Number(controller.selectedMedia.duration || 0)
    property alias editor: editCanvas
    property alias player: previewPlayer

    spacing: 6
    implicitHeight: videoFrame.Layout.preferredHeight + playbackBar.Layout.preferredHeight + spacing

    function timeLabel(seconds) {
        var safe = Math.max(0, Number(seconds) || 0)
        var hours = Math.floor(safe / 3600)
        var minutes = Math.floor((safe % 3600) / 60)
        var secs = Math.floor(safe % 60)
        var base = String(minutes).padStart(2, "0") + ":" + String(secs).padStart(2, "0")
        return hours > 0 ? String(hours).padStart(2, "0") + ":" + base : base
    }

    function reset() {
        previewPlayer.stop()
        previewPlayer.position = 0
        editCanvas.reset()
        Qt.callLater(root.startAutoplay)
    }

    function startAutoplay() {
        if (String(previewPlayer.source).length === 0)
            return
        previewPlayer.position = Math.max(0, root.trimStart) * 1000
        previewPlayer.play()
    }

    function editSpec() {
        return editCanvas.editSpec()
    }

    function togglePlayback() {
        if (controller.selectedMediaChecking)
            return
        if (previewPlayer.playbackState === MediaPlayer.PlayingState)
            previewPlayer.pause()
        else
            previewPlayer.play()
    }

    Rectangle {
        id: videoFrame
        Layout.fillWidth: true
        Layout.fillHeight: root.studioMode
        Layout.minimumHeight: root.studioMode ? 220 : 190
        Layout.preferredHeight: root.studioMode
            ? Math.max(300, Math.min(620, width * 0.5625))
            : Math.max(190, width * 0.54)
        radius: root.studioMode ? Theme.radiusLg : Theme.radiusMd
        color: Theme.mediaWell
        border.width: 1
        border.color: editCanvas.hasEdits ? Theme.accent : Theme.borderStrong
        clip: true

        MediaPlayer {
            id: previewPlayer
            source: controller.selectedMedia.mediaUrl || ""
            autoPlay: true
            loops: MediaPlayer.Infinite
            videoOutput: previewOutput
            audioOutput: AudioOutput { volume: 0.65 }
            onPositionChanged: function(position) {
                if (root.trimEnd > 0 && position / 1000 >= root.trimEnd)
                    previewPlayer.position = root.trimStart * 1000
            }
            onMediaStatusChanged: {
                if (mediaStatus === MediaPlayer.EndOfMedia)
                    root.startAutoplay()
            }
        }

        VideoOutput {
            id: previewOutput
            anchors.fill: parent
            anchors.margins: 1
            fillMode: VideoOutput.PreserveAspectFit
        }

        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            z: 1
            color: Theme.mediaWell
            visible: previewPlayer.playbackState === MediaPlayer.StoppedState
                && prepareThumbnail.status !== Image.Ready

            Column {
                anchors.centerIn: parent
                spacing: 8
                AppIcon {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: root.studioMode ? 32 : 25
                    height: width
                    name: "media"
                    iconColor: Theme.mediaMuted
                }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: controller.selectedMediaChecking
                        ? "Checking video"
                        : "Preparing preview"
                    color: Theme.mediaMuted
                    font.pixelSize: Theme.textXs
                }
            }
        }

        Image {
            id: prepareThumbnail
            anchors.fill: parent
            anchors.margins: 1
            z: 2
            source: controller.selectedMedia.thumbnailUrl || ""
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            cache: true
            visible: previewPlayer.playbackState === MediaPlayer.StoppedState
                && status === Image.Ready
        }

        VideoEditCanvas {
            id: editCanvas
            anchors.fill: parent
            anchors.margins: 1
            z: 5
            videoOutput: previewOutput
            sourceWidth: Number(controller.selectedMedia.width || 1)
            sourceHeight: Number(controller.selectedMedia.height || 1)
        }

        Rectangle {
            visible: root.studioMode
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.margins: 14
            z: 10
            height: 28
            width: canvasState.implicitWidth + 20
            radius: Theme.radiusSm
            color: Theme.overlay
            opacity: 0.94

            Row {
                id: canvasState
                anchors.centerIn: parent
                spacing: 6
                AppIcon {
                    width: 13
                    height: 13
                    anchors.verticalCenter: parent.verticalCenter
                    name: editCanvas.hasEdits ? "crop" : "media"
                    iconColor: editCanvas.hasEdits
                        ? Theme.accent : Theme.mediaMuted
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: editCanvas.hasEdits ? "Frame edits active" : "Original frame"
                    color: Theme.mediaText
                    font.pixelSize: Theme.textXs
                    font.weight: Font.DemiBold
                }
            }
        }
    }

    Item {
        id: playbackBar
        Layout.fillWidth: true
        Layout.preferredHeight: Theme.controlHeight

        RowLayout {
            anchors.fill: parent
            spacing: 8

            AppButton {
                text: previewPlayer.playbackState === MediaPlayer.PlayingState
                    ? "Pause" : "Play"
                iconName: previewPlayer.playbackState === MediaPlayer.PlayingState
                    ? "pause" : "play"
                compact: true
                kind: "ghost"
                toolTipText: text + "  ·  Space"
                enabled: !controller.selectedMediaChecking
                onClicked: root.togglePlayback()
            }
            AppSlider {
                Layout.fillWidth: true
                from: 0
                to: Math.max(1, Number(controller.selectedMedia.duration || 0) * 1000)
                value: previewPlayer.position
                enabled: !controller.selectedMediaChecking
                    && Number(controller.selectedMedia.duration || 0) > 0
                onMoved: previewPlayer.position = value
                Accessible.name: "Video playback position"
            }
            Text {
                text: root.timeLabel(previewPlayer.position / 1000)
                color: Theme.textSoft
                font.pixelSize: Theme.textXs
            }
            Text {
                text: "/"
                color: Theme.mutedSoft
                font.pixelSize: Theme.textXs
            }
            Text {
                text: root.timeLabel(controller.selectedMedia.duration || 0)
                color: Theme.muted
                font.pixelSize: Theme.textXs
            }
        }
    }
}
