pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import QtMultimedia
import "."

ColumnLayout {
    id: root

    property bool studioMode: false
    property bool compactMode: false
    property real trimStart: 0
    property real trimEnd: Number(controller.selectedMedia.duration || 0)
    property alias editor: editCanvas
    property alias player: previewPlayer

    signal trimStartEdited(real seconds)
    signal trimEndEdited(real seconds)

    spacing: root.compactMode ? 4 : 6
    implicitHeight: root.studioMode
        ? 0
        : (root.compactMode
            ? 115 : Math.max(170, root.width * 0.54))
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

    function loadEditSpec(spec) {
        editCanvas.loadEditSpec(spec)
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
        Layout.fillHeight: true
        Layout.minimumHeight: root.studioMode
            ? 180 : root.compactMode ? 96 : 130
        Layout.preferredHeight: root.studioMode
            ? 420 : root.compactMode
                ? 115 : Math.max(170, root.width * 0.54)
        implicitHeight: root.studioMode
            ? 420 : root.compactMode
                ? 115 : Math.max(170, root.width * 0.54)
        radius: Theme.radiusWorkbench
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
        compactMode: root.compactMode
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
