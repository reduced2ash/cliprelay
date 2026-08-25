pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import "."

ComboBox {
    id: control

    implicitHeight: Theme.workbenchControlHeight
    implicitWidth: Math.max(
        94,
        implicitContentWidth + leftPadding + rightPadding
    )
    leftPadding: 9
    rightPadding: 28
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.42

    Accessible.role: Accessible.ComboBox

    delegate: ItemDelegate {
        id: option
        required property int index
        width: ListView.view ? ListView.view.width : control.width
        height: 32
        text: control.textAt(index)
        highlighted: control.highlightedIndex === index
        hoverEnabled: true

        contentItem: Text {
            text: option.text
            color: option.highlighted || option.hovered
                ? Theme.text : Theme.textSoft
            font.pixelSize: Theme.textWorkbench
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
            leftPadding: 7
            rightPadding: 7
        }
        background: Rectangle {
            radius: Theme.radiusWorkbench
            color: option.highlighted || option.hovered
                ? Theme.active : "transparent"
        }
    }

    indicator: AppIcon {
        x: control.width - width - 8
        y: Math.round((control.height - height) / 2)
        width: 13
        height: 13
        name: control.popup.visible ? "chevronUp" : "chevronDown"
        strokeWidth: 1.8
        iconColor: control.visualFocus || control.popup.visible
            ? Theme.accent : Theme.muted
    }

    contentItem: Text {
        text: control.displayText
        color: control.enabled ? Theme.text : Theme.muted
        font.pixelSize: Theme.textWorkbench
        font.weight: Font.Medium
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: Theme.radiusWorkbench
        color: control.hovered || control.popup.visible
            ? Theme.hover : Theme.raised
        border.width: control.visualFocus || control.popup.visible ? 2 : 1
        border.color: control.visualFocus || control.popup.visible
            ? Theme.accent : Theme.border
        Behavior on color {
            ColorAnimation { duration: Theme.fastMotion }
        }
    }

    popup: Popup {
        y: control.height + 4
        width: control.width
        implicitHeight: Math.min(contentItem.implicitHeight + 6, 224)
        padding: 3
        closePolicy: Popup.CloseOnEscape
            | Popup.CloseOnPressOutsideParent

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
            radius: Theme.radiusSm
            color: Theme.surfaceSoft
            border.width: 1
            border.color: Theme.borderStrong
        }
    }
}
