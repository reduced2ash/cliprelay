pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import "."

Rectangle {
    id: root

    required property int index
    required property string folderPath
    required property string folderName
    required property string folderFullPath
    required property int videoCount
    required property int folderDepth
    required property bool folderHasChildren
    required property bool folderExpanded
    property bool selected: false

    signal activated()
    signal toggleRequested()
    signal moveFocusRequested(int delta)
    signal parentFocusRequested()

    readonly property int visualDepth: Math.min(folderDepth, 6)
    readonly property real branchX: 5 + visualDepth * 12

    width: ListView.view ? ListView.view.width : 190
    height: 34
    radius: 4
    clip: true
    activeFocusOnTab: true
    color: selected
        ? Theme.active
        : rowMouse.containsMouse || activeFocus
            ? Theme.hover
            : "transparent"
    border.width: activeFocus ? 1 : 0
    border.color: Theme.accent

    Accessible.role: Accessible.ListItem
    Accessible.name: folderName + ", " + videoCount
        + (videoCount === 1 ? " video" : " videos")
    Accessible.description: folderHasChildren
        ? (folderExpanded ? "Expanded folder" : "Collapsed folder")
        : "Folder"

    Behavior on color {
        ColorAnimation { duration: Theme.quickMotion }
    }

    Repeater {
        model: Math.max(0, root.visualDepth)
        Rectangle {
            required property int index
            x: 13 + index * 12
            y: 0
            width: 1
            height: root.height
            color: Theme.border
            opacity: 0.48
        }
    }

    Item {
        id: disclosureSlot
        x: root.branchX
        width: 20
        height: parent.height

        AppIcon {
            anchors.centerIn: parent
            width: 12
            height: 12
            visible: root.folderHasChildren
            name: root.folderExpanded ? "chevronDown" : "chevronRight"
            strokeWidth: 2
            iconColor: root.selected || rowMouse.containsMouse
                ? Theme.textSoft : Theme.mutedSoft
        }
    }

    AppIcon {
        id: folderIcon
        x: disclosureSlot.x + disclosureSlot.width + 1
        anchors.verticalCenter: parent.verticalCenter
        width: 15
        height: 15
        name: root.folderPath.length === 0 ? "library" : "folder"
        strokeWidth: 1.75
        iconColor: root.selected
            ? Theme.accentText
            : root.folderExpanded ? Theme.textSoft : Theme.muted
    }

    Text {
        id: folderLabel
        x: folderIcon.x + folderIcon.width + 7
        anchors.verticalCenter: parent.verticalCenter
        width: Math.max(0, itemCount.x - x - 8)
        text: root.folderName
        color: root.selected ? Theme.text : Theme.textSoft
        font.pixelSize: Theme.textXs
        font.weight: root.selected ? Font.DemiBold : Font.Medium
        elide: Text.ElideRight
        maximumLineCount: 1
    }

    Text {
        id: itemCount
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        text: root.videoCount.toLocaleString()
        color: root.selected ? Theme.textSoft : Theme.mutedSoft
        font.pixelSize: 10
        font.weight: Font.Medium
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: function(mouse) {
            root.forceActiveFocus()
            if (
                root.folderHasChildren
                && mouse.x >= disclosureSlot.x
                && mouse.x <= disclosureSlot.x + disclosureSlot.width
            ) {
                root.toggleRequested()
            } else {
                root.activated()
            }
        }
        onDoubleClicked: function(mouse) {
            var clickedDisclosure = mouse.x >= disclosureSlot.x
                && mouse.x <= disclosureSlot.x + disclosureSlot.width
            if (root.folderHasChildren && !clickedDisclosure)
                root.toggleRequested()
        }
    }

    ToolTip.visible: rowMouse.containsMouse
        && (folderLabel.truncated || root.folderFullPath !== root.folderName)
    ToolTip.text: root.folderFullPath
    ToolTip.delay: 500

    Keys.onSpacePressed: root.activated()
    Keys.onReturnPressed: root.activated()
    Keys.onEnterPressed: root.activated()
    Keys.onUpPressed: root.moveFocusRequested(-1)
    Keys.onDownPressed: root.moveFocusRequested(1)
    Keys.onRightPressed: {
        if (!root.folderHasChildren)
            return
        if (root.folderExpanded)
            root.moveFocusRequested(1)
        else
            root.toggleRequested()
    }
    Keys.onLeftPressed: {
        if (root.folderHasChildren && root.folderExpanded)
            root.toggleRequested()
        else
            root.parentFocusRequested()
    }
}
