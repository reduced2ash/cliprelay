pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import "."

Rectangle {
    id: root

    required property int index
    required property string folderPath
    required property string folderName
    required property string folderDetail
    required property int videoCount
    required property int directVideoCount
    required property int folderSelectionState
    required property int folderDepth
    required property bool folderHasChildren
    required property bool folderExpanded

    signal selectionRequested(bool enabled)
    signal toggleRequested()
    signal moveFocusRequested(int delta)
    signal parentFocusRequested()

    readonly property bool fullySelected: folderSelectionState === 2
    readonly property bool partiallySelected: folderSelectionState === 1
    readonly property int visualDepth: Math.min(folderDepth, 8)
    readonly property real branchX: 5 + visualDepth * 12

    width: ListView.view ? ListView.view.width : 380
    height: 32
    radius: 3
    clip: true
    activeFocusOnTab: true
    color: rowMouse.containsMouse || activeFocus
        ? Theme.hover : "transparent"

    Accessible.role: Accessible.CheckBox
    Accessible.name: folderName + ", " + videoCount
        + (videoCount === 1 ? " video" : " videos")
    Accessible.description: partiallySelected
        ? "Some nested source folders selected"
        : fullySelected
            ? "Folder and nested source folders selected"
            : "Folder and nested source folders not selected"
    Accessible.checked: fullySelected

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
            opacity: 0.42
        }
    }

    Item {
        id: disclosureSlot
        x: root.branchX
        width: 18
        height: parent.height

        AppIcon {
            anchors.centerIn: parent
            width: 11
            height: 11
            visible: root.folderHasChildren
            name: root.folderExpanded ? "chevronDown" : "chevronRight"
            strokeWidth: 2
            iconColor: rowMouse.containsMouse
                ? Theme.textSoft : Theme.mutedSoft
        }
    }

    Rectangle {
        id: selectionBox
        x: disclosureSlot.x + disclosureSlot.width + 1
        anchors.verticalCenter: parent.verticalCenter
        width: 15
        height: 15
        radius: 3
        color: root.fullySelected
            ? Theme.accent
            : root.partiallySelected ? Theme.accentSoft : Theme.raised
        border.width: 1
        border.color: root.folderSelectionState > 0
            ? Theme.accent : Theme.borderStrong

        AppIcon {
            anchors.centerIn: parent
            width: 11
            height: 11
            visible: root.fullySelected
            name: "check"
            strokeWidth: 2.4
            iconColor: Theme.accentContent
        }

        Rectangle {
            anchors.centerIn: parent
            width: 7
            height: 1.5
            radius: 1
            visible: root.partiallySelected
            color: Theme.accentText
        }
    }

    AppIcon {
        id: folderIcon
        x: selectionBox.x + selectionBox.width + 7
        anchors.verticalCenter: parent.verticalCenter
        width: 14
        height: 14
        name: root.folderPath.length === 0 ? "library" : "folder"
        strokeWidth: 1.7
        iconColor: root.folderSelectionState > 0
            ? Theme.accentText
            : root.folderExpanded ? Theme.textSoft : Theme.muted
    }

    Text {
        id: folderLabel
        x: folderIcon.x + folderIcon.width + 6
        anchors.verticalCenter: parent.verticalCenter
        width: Math.max(0, itemCount.x - x - 8)
        text: root.folderName
        color: root.folderSelectionState > 0
            ? Theme.text : Theme.textSoft
        font.pixelSize: Theme.textXs
        font.weight: root.folderSelectionState > 0
            ? Font.DemiBold : Font.Medium
        elide: Text.ElideRight
        maximumLineCount: 1
    }

    Text {
        id: itemCount
        anchors.right: parent.right
        anchors.rightMargin: 9
        anchors.verticalCenter: parent.verticalCenter
        width: 46
        horizontalAlignment: Text.AlignRight
        text: root.videoCount.toLocaleString()
        color: root.folderSelectionState > 0
            ? Theme.accentText : Theme.mutedSoft
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
                root.selectionRequested(!root.fullySelected)
            }
        }
        onDoubleClicked: function(mouse) {
            const clickedDisclosure = mouse.x >= disclosureSlot.x
                && mouse.x <= disclosureSlot.x + disclosureSlot.width
            if (root.folderHasChildren && !clickedDisclosure)
                root.toggleRequested()
        }
    }

    ToolTip.visible: rowMouse.containsMouse
        && (folderLabel.truncated || root.folderDetail.length > 0)
    ToolTip.text: root.folderPath.length
        ? root.folderPath + "\n"
            + root.videoCount.toLocaleString() + " videos in subtree"
        : "Files directly inside the library root"
    ToolTip.delay: 500

    Keys.onSpacePressed:
        root.selectionRequested(!root.fullySelected)
    Keys.onReturnPressed:
        root.selectionRequested(!root.fullySelected)
    Keys.onEnterPressed:
        root.selectionRequested(!root.fullySelected)
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
