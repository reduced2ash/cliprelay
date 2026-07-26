pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import "."

ComboBox {
    id: control

    implicitWidth: Math.max(140, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Theme.controlHeight
    leftPadding: 13
    rightPadding: 38
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.46

    Accessible.role: Accessible.ComboBox

    delegate: ItemDelegate {
        id: option
        required property int index
        width: ListView.view ? ListView.view.width : control.width
        height: 40
        text: control.textAt(index)
        highlighted: control.highlightedIndex === index
        hoverEnabled: true

        contentItem: Text {
            text: option.text
            color: option.highlighted || option.hovered ? Theme.text : Theme.textSoft
            font.pixelSize: Theme.textSm
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
            leftPadding: 9
            rightPadding: 9
        }
        background: Rectangle {
            radius: Theme.radiusSm
            color: option.highlighted || option.hovered ? Theme.active : "transparent"
        }
    }

    indicator: AppIcon {
        x: control.width - width - 12
        y: Math.round((control.height - height) / 2)
        width: 17
        height: 17
        name: control.popup.visible ? "chevronUp" : "chevronDown"
        strokeWidth: 1.9
        iconColor: control.activeFocus || control.popup.visible ? Theme.accent : Theme.muted
    }

    contentItem: Text {
        leftPadding: 0
        rightPadding: 0
        text: control.displayText
        color: control.enabled ? Theme.text : Theme.muted
        font.pixelSize: Theme.textSm
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: Theme.radiusSm
        color: Theme.raised
        border.width: control.activeFocus || control.popup.visible ? 2 : 1
        border.color: control.activeFocus || control.popup.visible
            ? Theme.accent
            : control.hovered ? Theme.borderStrong : Theme.border
        Behavior on border.color { ColorAnimation { duration: Theme.fastMotion } }
    }

    popup: Popup {
        y: control.height + 4
        width: control.width
        implicitHeight: Math.min(contentItem.implicitHeight + 8, 320)
        padding: 4
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: control.delegateModel
            currentIndex: control.highlightedIndex
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar { }
        }
        background: Rectangle {
            radius: Theme.radiusMd
            color: Theme.surfaceSoft
            border.width: 1
            border.color: Theme.borderStrong
        }
    }
}
