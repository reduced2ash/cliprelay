pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia
import "."

ColumnLayout {
    id: root

    property var player
    property real duration: 0
    property real trimStart: 0
    property real trimEnd: duration
    property string filmstripUrl: ""
    property string thumbnailUrl: ""
    property bool filmstripLoading: false
    property bool studioMode: false
    property bool controlsEnabled: true

    readonly property int frameCount: 12
    readonly property real minimumCutDuration: 0.05
    readonly property real startFraction: duration > 0
        ? clamp(trimStart / duration, 0, 1) : 0
    readonly property real endFraction: duration > 0
        ? clamp(trimEnd / duration, 0, 1) : 1
    readonly property real playheadSeconds: player
        ? Number(player.position || 0) / 1000 : 0
    readonly property real playheadFraction: duration > 0
        ? clamp(playheadSeconds / duration, 0, 1) : 0
    readonly property bool cutActive: duration > 0
        && (trimStart > minimumCutDuration
            || trimEnd < duration - minimumCutDuration)
    readonly property bool playing: player
        && player.playbackState === MediaPlayer.PlayingState

    signal seekRequested(real seconds)
    signal trimStartEdited(real seconds)
    signal trimEndEdited(real seconds)
    signal resetCutRequested()
    signal togglePlaybackRequested()

    spacing: studioMode ? 8 : 6

    function clamp(value, minimum, maximum) {
        return Math.max(minimum, Math.min(maximum, value))
    }

    function secondsAt(position, trackWidth) {
        if (duration <= 0 || trackWidth <= 0)
            return 0
        return clamp(position / trackWidth, 0, 1) * duration
    }

    function formatTime(seconds, precise) {
        var safe = Math.max(0, Number(seconds) || 0)
        var hours = Math.floor(safe / 3600)
        var minutes = Math.floor((safe % 3600) / 60)
        var wholeSeconds = Math.floor(safe % 60)
        var base = String(minutes).padStart(2, "0")
            + ":" + String(wholeSeconds).padStart(2, "0")
        if (hours > 0)
            base = String(hours).padStart(2, "0") + ":" + base
        if (precise) {
            var hundredths = Math.floor((safe - Math.floor(safe)) * 100)
            base += "." + String(hundredths).padStart(2, "0")
        }
        return base
    }

    function parseTime(value, fallback) {
        var parts = String(value).trim().split(":")
        var total = 0
        if (parts.length === 3) {
            total = Number(parts[0]) * 3600
                + Number(parts[1]) * 60 + Number(parts[2])
        } else if (parts.length === 2) {
            total = Number(parts[0]) * 60 + Number(parts[1])
        } else {
            total = Number(parts[0])
        }
        return isNaN(total) ? fallback : total
    }

    function seekBy(seconds) {
        root.seekRequested(
            clamp(root.playheadSeconds + seconds, 0, root.duration)
        )
    }

    RowLayout {
        id: transportRow
        Layout.fillWidth: true
        spacing: 6

        Text {
            Layout.preferredWidth: root.studioMode ? 78 : 70
            text: root.formatTime(root.playheadSeconds, true)
            color: Theme.textSoft
            font.pixelSize: Theme.textXs
            font.features: { "tnum": 1 }
        }

        Item { Layout.fillWidth: true }

        AppButton {
            Layout.preferredWidth: Theme.compactControl
            Layout.preferredHeight: Theme.compactControl
            text: "Back 5 seconds"
            iconName: "skipBack"
            compact: true
            kind: "ghost"
            toolTipText: "Back 5 seconds"
            enabled: root.controlsEnabled && root.duration > 0
            onClicked: root.seekBy(-5)
        }

        AppButton {
            Layout.preferredWidth: Theme.controlHeight
            Layout.preferredHeight: Theme.controlHeight
            text: root.playing ? "Pause" : "Play"
            iconName: root.playing ? "pause" : "play"
            compact: true
            kind: "ghost"
            toolTipText: text + "  ·  Space"
            enabled: root.controlsEnabled && root.duration > 0
            onClicked: root.togglePlaybackRequested()
        }

        AppButton {
            Layout.preferredWidth: Theme.compactControl
            Layout.preferredHeight: Theme.compactControl
            text: "Forward 5 seconds"
            iconName: "skipForward"
            compact: true
            kind: "ghost"
            toolTipText: "Forward 5 seconds"
            enabled: root.controlsEnabled && root.duration > 0
            onClicked: root.seekBy(5)
        }

        Item { Layout.fillWidth: true }

        Text {
            Layout.preferredWidth: root.studioMode ? 78 : 70
            horizontalAlignment: Text.AlignRight
            text: root.formatTime(root.duration, true)
            color: Theme.muted
            font.pixelSize: Theme.textXs
            font.features: { "tnum": 1 }
        }
    }

    Rectangle {
        id: timelineTrack
        Layout.fillWidth: true
        Layout.preferredHeight: root.studioMode ? 72 : 60
        radius: Theme.radiusSm
        color: Theme.mediaWell
        border.width: 1
        border.color: root.cutActive ? Theme.accent : Theme.borderStrong
        clip: true
        opacity: root.controlsEnabled ? 1 : 0.55

        Row {
            id: filmstripRow
            anchors.fill: parent
            visible: root.filmstripUrl.length > 0

            Repeater {
                model: root.frameCount

                delegate: Item {
                    id: frameCell
                    required property int index
                    width: filmstripRow.width / root.frameCount
                    height: filmstripRow.height
                    clip: true

                    Image {
                        anchors.fill: parent
                        source: root.filmstripUrl
                        sourceClipRect: Qt.rect(
                            frameCell.index * 160, 0, 160, 90
                        )
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                        cache: true
                    }

                    Rectangle {
                        anchors.right: parent.right
                        width: 1
                        height: parent.height
                        color: "#28FFFFFF"
                        visible: frameCell.index < root.frameCount - 1
                    }
                }
            }
        }

        Image {
            anchors.fill: parent
            anchors.margins: 1
            source: root.thumbnailUrl
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            cache: true
            opacity: 0.42
            visible: root.filmstripUrl.length === 0
                && status === Image.Ready
        }

        Row {
            anchors.fill: parent
            visible: root.filmstripUrl.length === 0
                && root.thumbnailUrl.length === 0

            Repeater {
                model: root.frameCount

                delegate: Rectangle {
                    required property int index
                    width: timelineTrack.width / root.frameCount
                    height: timelineTrack.height
                    color: index % 2 === 0 ? "#141820" : "#10131A"
                }
            }
        }

        Rectangle {
            anchors.centerIn: parent
            width: loadingLabel.implicitWidth + 20
            height: 26
            radius: Theme.radiusSm
            color: Theme.overlay
            opacity: 0.92
            visible: root.filmstripLoading
                && root.filmstripUrl.length === 0

            Text {
                id: loadingLabel
                anchors.centerIn: parent
                text: "Building filmstrip…"
                color: Theme.mediaMuted
                font.pixelSize: Theme.textXs
            }
        }

        Rectangle {
            z: 2
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: root.startFraction * parent.width
            color: "#B305070B"
            visible: width > 0
        }

        Rectangle {
            z: 2
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: (1 - root.endFraction) * parent.width
            color: "#B305070B"
            visible: width > 0
        }

        Rectangle {
            z: 3
            x: root.startFraction * timelineTrack.width
            width: Math.max(
                1,
                (root.endFraction - root.startFraction)
                    * timelineTrack.width
            )
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            color: "transparent"
            border.width: root.cutActive ? 2 : 0
            border.color: Theme.accent
        }

        MouseArea {
            z: 4
            anchors.fill: parent
            enabled: root.controlsEnabled && root.duration > 0
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onPressed: function(mouse) {
                root.seekRequested(
                    root.secondsAt(mouse.x, timelineTrack.width)
                )
            }
            onPositionChanged: function(mouse) {
                if (pressed) {
                    root.seekRequested(
                        root.secondsAt(mouse.x, timelineTrack.width)
                    )
                }
            }
            Accessible.role: Accessible.Slider
            Accessible.name: "Video playback position"
        }

        Rectangle {
            id: playhead
            z: 8
            x: root.clamp(
                root.playheadFraction * timelineTrack.width - width / 2,
                0,
                Math.max(0, timelineTrack.width - width)
            )
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 2
            color: "#F7F9FC"
            visible: root.duration > 0

            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: 3
                width: 7
                height: 7
                radius: 4
                color: parent.color
                border.width: 1
                border.color: "#66000000"
            }
        }

        Rectangle {
            id: inHandle
            z: 10
            x: root.clamp(
                root.startFraction * timelineTrack.width - width / 2,
                0,
                Math.max(0, timelineTrack.width - width)
            )
            width: 10
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            radius: 3
            color: Theme.accent
            border.width: 1
            border.color: Theme.accentContent
            visible: root.duration > 0
            activeFocusOnTab: true

            Rectangle {
                anchors.centerIn: parent
                width: 2
                height: Math.max(12, parent.height * 0.34)
                radius: 1
                color: Theme.accentContent
                opacity: 0.86
            }

            MouseArea {
                id: inHandleMouse
                anchors.centerIn: parent
                width: 28
                height: timelineTrack.height
                enabled: root.controlsEnabled
                hoverEnabled: true
                cursorShape: Qt.SizeHorCursor
                onPressed: {
                    inHandle.forceActiveFocus()
                    root.seekRequested(root.trimStart)
                }
                onPositionChanged: function(mouse) {
                    if (!pressed)
                        return
                    var point = mapToItem(
                        timelineTrack, mouse.x, mouse.y
                    )
                    var next = root.clamp(
                        root.secondsAt(point.x, timelineTrack.width),
                        0,
                        Math.max(
                            0,
                            root.trimEnd - root.minimumCutDuration
                        )
                    )
                    root.trimStartEdited(next)
                    root.seekRequested(next)
                }
            }

            Keys.onLeftPressed: function(event) {
                var step = event.modifiers & Qt.ShiftModifier ? 1 : 0.05
                root.trimStartEdited(root.clamp(
                    root.trimStart - step,
                    0,
                    root.trimEnd - root.minimumCutDuration
                ))
                event.accepted = true
            }
            Keys.onRightPressed: function(event) {
                var step = event.modifiers & Qt.ShiftModifier ? 1 : 0.05
                root.trimStartEdited(root.clamp(
                    root.trimStart + step,
                    0,
                    root.trimEnd - root.minimumCutDuration
                ))
                event.accepted = true
            }
            Accessible.role: Accessible.Slider
            Accessible.name: "Cut start"
            ToolTip.visible: inHandleMouse.containsMouse
                || inHandle.activeFocus
            ToolTip.text: "IN  " + root.formatTime(root.trimStart, true)
        }

        Rectangle {
            id: outHandle
            z: 10
            x: root.clamp(
                root.endFraction * timelineTrack.width - width / 2,
                0,
                Math.max(0, timelineTrack.width - width)
            )
            width: 10
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            radius: 3
            color: Theme.accent
            border.width: 1
            border.color: Theme.accentContent
            visible: root.duration > 0
            activeFocusOnTab: true

            Rectangle {
                anchors.centerIn: parent
                width: 2
                height: Math.max(12, parent.height * 0.34)
                radius: 1
                color: Theme.accentContent
                opacity: 0.86
            }

            MouseArea {
                id: outHandleMouse
                anchors.centerIn: parent
                width: 28
                height: timelineTrack.height
                enabled: root.controlsEnabled
                hoverEnabled: true
                cursorShape: Qt.SizeHorCursor
                onPressed: outHandle.forceActiveFocus()
                onPositionChanged: function(mouse) {
                    if (!pressed)
                        return
                    var point = mapToItem(
                        timelineTrack, mouse.x, mouse.y
                    )
                    root.trimEndEdited(root.clamp(
                        root.secondsAt(point.x, timelineTrack.width),
                        root.trimStart + root.minimumCutDuration,
                        root.duration
                    ))
                }
            }

            Keys.onLeftPressed: function(event) {
                var step = event.modifiers & Qt.ShiftModifier ? 1 : 0.05
                root.trimEndEdited(root.clamp(
                    root.trimEnd - step,
                    root.trimStart + root.minimumCutDuration,
                    root.duration
                ))
                event.accepted = true
            }
            Keys.onRightPressed: function(event) {
                var step = event.modifiers & Qt.ShiftModifier ? 1 : 0.05
                root.trimEndEdited(root.clamp(
                    root.trimEnd + step,
                    root.trimStart + root.minimumCutDuration,
                    root.duration
                ))
                event.accepted = true
            }
            Accessible.role: Accessible.Slider
            Accessible.name: "Cut end"
            ToolTip.visible: outHandleMouse.containsMouse
                || outHandle.activeFocus
            ToolTip.text: "OUT  " + root.formatTime(root.trimEnd, true)
        }
    }

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: 14

        Repeater {
            model: 5

            delegate: Text {
                id: tickLabel
                required property int index
                readonly property real fraction: index / 4
                x: root.clamp(
                    fraction * parent.width - width / 2,
                    0,
                    Math.max(0, parent.width - width)
                )
                text: root.formatTime(root.duration * fraction, false)
                color: Theme.mutedSoft
                font.pixelSize: 10
                font.features: { "tnum": 1 }
            }
        }
    }

    RowLayout {
        id: precisionRow
        Layout.fillWidth: true
        spacing: 7

        Text {
            text: "IN"
            color: Theme.muted
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
        }

        AppField {
            id: startField
            Layout.preferredWidth: root.studioMode ? 112 : 100
            implicitHeight: Theme.compactControl
            Accessible.name: "Cut start time"
            onEditingFinished: {
                var next = root.clamp(
                    root.parseTime(text, root.trimStart),
                    0,
                    Math.max(
                        0,
                        root.trimEnd - root.minimumCutDuration
                    )
                )
                root.trimStartEdited(next)
                root.seekRequested(next)
            }
            onAccepted: focus = false

            Binding {
                target: startField
                property: "text"
                value: root.formatTime(root.trimStart, true)
                when: !startField.activeFocus
            }
        }

        Text {
            text: "OUT"
            color: Theme.muted
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
        }

        AppField {
            id: endField
            Layout.preferredWidth: root.studioMode ? 112 : 100
            implicitHeight: Theme.compactControl
            Accessible.name: "Cut end time"
            onEditingFinished: root.trimEndEdited(root.clamp(
                root.parseTime(text, root.trimEnd),
                root.trimStart + root.minimumCutDuration,
                root.duration
            ))
            onAccepted: focus = false

            Binding {
                target: endField
                property: "text"
                value: root.formatTime(root.trimEnd, true)
                when: !endField.activeFocus
            }
        }

        Item { Layout.fillWidth: true }

        Text {
            visible: root.studioMode || root.cutActive
            text: (root.cutActive ? "CUT  " : "FULL  ")
                + root.formatTime(
                    Math.max(0, root.trimEnd - root.trimStart),
                    true
                )
            color: root.cutActive ? Theme.accentText : Theme.muted
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
        }

        AppButton {
            visible: root.cutActive
            text: "Reset cut"
            iconName: "refresh"
            compact: true
            kind: "ghost"
            toolTipText: "Reset cut to full video"
            onClicked: root.resetCutRequested()
        }
    }
}
