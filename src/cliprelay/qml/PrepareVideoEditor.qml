pragma ComponentBehavior: Bound
import QtQuick
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

    signal trimStartEdited(real seconds)
    signal trimEndEdited(real seconds)

    spacing: 6
    implicitHeight: root.studioMode
        ? 0
        : videoFrame.Layout.preferredHeight
            + playbackTimeline.implicitHeight + spacing

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
        implicitHeight: root.studioMode
            ? 0 : Math.max(190, width * 0.54)
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

    VideoTimeline {
        id: playbackTimeline
        Layout.fillWidth: true
        player: previewPlayer
        duration: Number(controller.selectedMedia.duration || 0)
        trimStart: root.trimStart
        trimEnd: root.trimEnd
        filmstripUrl: controller.selectedMedia.timelineUrl || ""
        thumbnailUrl: controller.selectedMedia.thumbnailUrl || ""
        filmstripLoading: controller.selectedMediaTimelineLoading
        studioMode: root.studioMode
        controlsEnabled: !controller.selectedMediaChecking
        onTogglePlaybackRequested: root.togglePlayback()
        onSeekRequested: function(seconds) {
            previewPlayer.position = seconds * 1000
        }
        onTrimStartEdited: function(seconds) {
            root.trimStartEdited(seconds)
            previewPlayer.position = seconds * 1000
        }
        onTrimEndEdited: function(seconds) {
            root.trimEndEdited(seconds)
        }
        onResetCutRequested: {
            root.trimStartEdited(0)
            root.trimEndEdited(
                Number(controller.selectedMedia.duration || 0)
            )
            previewPlayer.position = 0
        }
    }
}
