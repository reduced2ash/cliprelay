import QtQuick
import QtQuick.Layouts
import "."

Item {
    id: root

    property bool studioMode: false
    property bool compactMode: false
    property real trimStart: 0
    property real trimEnd: Number(controller.selectedMedia.duration || 0)
    property alias editor: videoEditor.editor
    property alias player: videoEditor.player

    signal trimStartEdited(real seconds)
    signal trimEndEdited(real seconds)
    signal editsChanged()

    implicitHeight: videoEditor.implicitHeight
        + sourceStrip.implicitHeight + stageLayout.spacing

    function reset() {
        videoEditor.reset()
    }

    function startAutoplay() {
        videoEditor.startAutoplay()
    }

    function togglePlayback() {
        videoEditor.togglePlayback()
    }

    function editSpec() {
        return videoEditor.editSpec()
    }

    function loadEditSpec(spec) {
        videoEditor.loadEditSpec(spec)
    }

    ColumnLayout {
        id: stageLayout
        anchors.fill: parent
        anchors.leftMargin: root.studioMode ? 16 : 12
        anchors.rightMargin: root.studioMode ? 16 : 12
        anchors.topMargin: root.studioMode
            ? 14 : root.compactMode ? 6 : 8
        anchors.bottomMargin: 0
        spacing: root.studioMode ? 10 : root.compactMode ? 4 : 8

        PrepareVideoEditor {
            id: videoEditor
            Layout.fillWidth: true
            Layout.fillHeight: true
            studioMode: root.studioMode
            compactMode: root.compactMode
            trimStart: root.trimStart
            trimEnd: root.trimEnd
            onTrimStartEdited: function(seconds) {
                root.trimStartEdited(seconds)
            }
            onTrimEndEdited: function(seconds) {
                root.trimEndEdited(seconds)
            }
        }

        PrepareSourceStrip {
            id: sourceStrip
            Layout.fillWidth: true
            Layout.preferredHeight: implicitHeight
            studioMode: root.studioMode
            compactMode: root.compactMode
        }
    }

    Connections {
        target: videoEditor.editor

        function onEditsChanged() {
            root.editsChanged()
        }
    }
}
