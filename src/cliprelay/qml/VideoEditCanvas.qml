pragma ComponentBehavior: Bound
import QtQuick
import QtMultimedia
import "."

Item {
    id: root

    required property VideoOutput videoOutput
    property real sourceWidth: 1
    property real sourceHeight: 1
    property bool cropEnabled: false
    property real cropX: 0
    property real cropY: 0
    property real cropWidth: 1
    property real cropHeight: 1
    property int selectedShapeIndex: -1
    property alias shapeModel: shapes
    readonly property int shapeCount: shapes.count
    readonly property bool hasEdits: cropEnabled || shapes.count > 0

    signal editsChanged()

    readonly property real videoX: videoOutput.x + videoOutput.contentRect.x
    readonly property real videoY: videoOutput.y + videoOutput.contentRect.y
    readonly property real videoWidth: Math.max(0, videoOutput.contentRect.width)
    readonly property real videoHeight: Math.max(0, videoOutput.contentRect.height)
    readonly property real cropFrameX: videoX + (cropEnabled ? cropX * videoWidth : 0)
    readonly property real cropFrameY: videoY + (cropEnabled ? cropY * videoHeight : 0)
    readonly property real cropFrameWidth: cropEnabled ? cropWidth * videoWidth : videoWidth
    readonly property real cropFrameHeight: cropEnabled ? cropHeight * videoHeight : videoHeight

    property real gestureStartX: 0
    property real gestureStartY: 0
    property real startCropX: 0
    property real startCropY: 0
    property real startCropWidth: 1
    property real startCropHeight: 1
    property int cropGestureMode: 4
    property int shapeGestureIndex: -1
    property real startShapeX: 0
    property real startShapeY: 0
    property real startShapeWidth: 0
    property real startShapeHeight: 0
    property bool resizingShape: false

    function bounded(value, minimum, maximum) {
        return Math.max(minimum, Math.min(maximum, value))
    }

    function reset() {
        cropEnabled = false
        cropX = 0
        cropY = 0
        cropWidth = 1
        cropHeight = 1
        selectedShapeIndex = -1
        shapes.clear()
        editsChanged()
    }

    function resetCrop() {
        cropX = 0
        cropY = 0
        cropWidth = 1
        cropHeight = 1
        editsChanged()
    }

    function setCropEnabled(enabled) {
        cropEnabled = enabled
        selectedShapeIndex = -1
        if (enabled && cropWidth >= 0.999 && cropHeight >= 0.999)
            applyCropAspect(0)
        else
            editsChanged()
    }

    function applyCropAspect(targetRatio) {
        cropEnabled = true
        if (!targetRatio || targetRatio <= 0) {
            if (cropWidth >= 0.999 && cropHeight >= 0.999) {
                cropX = 0.08
                cropY = 0.08
                cropWidth = 0.84
                cropHeight = 0.84
            }
            editsChanged()
            return
        }
        var sourceRatio = Math.max(0.01, sourceWidth) / Math.max(0.01, sourceHeight)
        var normalizedRatio = targetRatio / sourceRatio
        if (normalizedRatio <= 1) {
            cropWidth = normalizedRatio
            cropHeight = 1
        } else {
            cropWidth = 1
            cropHeight = 1 / normalizedRatio
        }
        cropX = (1 - cropWidth) / 2
        cropY = (1 - cropHeight) / 2
        editsChanged()
    }

    function addShape(square) {
        var frameAspect = Math.max(0.01, cropFrameWidth) / Math.max(0.01, cropFrameHeight)
        var shapeWidth = square ? (frameAspect >= 1 ? 0.24 / frameAspect : 0.24) : 0.28
        var shapeHeight = square ? (frameAspect >= 1 ? 0.24 : 0.24 * frameAspect) : 0.18
        shapes.append({
            shapeX: 0.5 - shapeWidth / 2,
            shapeY: 0.5 - shapeHeight / 2,
            shapeWidth: shapeWidth,
            shapeHeight: shapeHeight,
            shapeKind: square ? "Square" : "Rectangle"
        })
        selectedShapeIndex = shapes.count - 1
        editsChanged()
    }

    function removeSelectedShape() {
        if (selectedShapeIndex < 0 || selectedShapeIndex >= shapes.count)
            return
        shapes.remove(selectedShapeIndex)
        selectedShapeIndex = Math.min(selectedShapeIndex, shapes.count - 1)
        editsChanged()
    }

    function clearShapes() {
        shapes.clear()
        selectedShapeIndex = -1
        editsChanged()
    }

    function editSpec() {
        var overlays = []
        for (var index = 0; index < shapes.count; ++index) {
            var shape = shapes.get(index)
            overlays.push({
                type: "rectangle",
                x: shape.shapeX,
                y: shape.shapeY,
                width: shape.shapeWidth,
                height: shape.shapeHeight
            })
        }
        return {
            crop: cropEnabled ? {
                enabled: true,
                x: cropX,
                y: cropY,
                width: cropWidth,
                height: cropHeight
            } : null,
            overlays: overlays
        }
    }

    function loadEditSpec(spec) {
        root.reset()
        if (!spec)
            return
        var crop = spec.crop
        if (crop && Boolean(crop.enabled)) {
            cropEnabled = true
            cropX = bounded(Number(crop.x || 0), 0, 1)
            cropY = bounded(Number(crop.y || 0), 0, 1)
            cropWidth = bounded(Number(crop.width || 1), 0.04, 1 - cropX)
            cropHeight = bounded(Number(crop.height || 1), 0.04, 1 - cropY)
        }
        var overlays = spec.overlays || []
        for (var index = 0; index < overlays.length; ++index) {
            var overlay = overlays[index]
            if (!overlay || overlay.type !== "rectangle")
                continue
            var width = bounded(Number(overlay.width || 0.2), 0.04, 1)
            var height = bounded(Number(overlay.height || 0.2), 0.04, 1)
            shapes.append({
                shapeX: bounded(Number(overlay.x || 0), 0, 1 - width),
                shapeY: bounded(Number(overlay.y || 0), 0, 1 - height),
                shapeWidth: width,
                shapeHeight: height,
                shapeKind: Math.abs(width - height) < 0.015
                    ? "Square" : "Rectangle"
            })
        }
        selectedShapeIndex = -1
        editsChanged()
    }

    function beginCropGesture(sceneX, sceneY, mode) {
        gestureStartX = sceneX
        gestureStartY = sceneY
        startCropX = cropX
        startCropY = cropY
        startCropWidth = cropWidth
        startCropHeight = cropHeight
        cropGestureMode = mode
    }

    function updateCropGesture(sceneX, sceneY) {
        if (videoWidth <= 0 || videoHeight <= 0)
            return
        var dx = (sceneX - gestureStartX) / videoWidth
        var dy = (sceneY - gestureStartY) / videoHeight
        var minimum = 0.04
        if (cropGestureMode === 4) {
            cropX = bounded(startCropX + dx, 0, 1 - startCropWidth)
            cropY = bounded(startCropY + dy, 0, 1 - startCropHeight)
            editsChanged()
            return
        }
        var right = startCropX + startCropWidth
        var bottom = startCropY + startCropHeight
        if (cropGestureMode === 0 || cropGestureMode === 2) {
            cropX = bounded(startCropX + dx, 0, right - minimum)
            cropWidth = right - cropX
        } else {
            cropWidth = bounded(startCropWidth + dx, minimum, 1 - startCropX)
        }
        if (cropGestureMode === 0 || cropGestureMode === 1) {
            cropY = bounded(startCropY + dy, 0, bottom - minimum)
            cropHeight = bottom - cropY
        } else {
            cropHeight = bounded(startCropHeight + dy, minimum, 1 - startCropY)
        }
        editsChanged()
    }

    function beginShapeGesture(index, sceneX, sceneY, resize) {
        if (index < 0 || index >= shapes.count)
            return
        var shape = shapes.get(index)
        selectedShapeIndex = index
        shapeGestureIndex = index
        gestureStartX = sceneX
        gestureStartY = sceneY
        startShapeX = shape.shapeX
        startShapeY = shape.shapeY
        startShapeWidth = shape.shapeWidth
        startShapeHeight = shape.shapeHeight
        resizingShape = resize
    }

    function updateShapeGesture(sceneX, sceneY) {
        if (shapeGestureIndex < 0 || shapeGestureIndex >= shapes.count
                || cropFrameWidth <= 0 || cropFrameHeight <= 0)
            return
        var dx = (sceneX - gestureStartX) / cropFrameWidth
        var dy = (sceneY - gestureStartY) / cropFrameHeight
        if (resizingShape) {
            var currentShape = shapes.get(shapeGestureIndex)
            if (currentShape.shapeKind === "Square") {
                var startSide = startShapeWidth * cropFrameWidth
                var side = Math.max(
                    12,
                    Math.min(
                        startSide + Math.max(
                            dx * cropFrameWidth,
                            dy * cropFrameHeight
                        ),
                        (1 - startShapeX) * cropFrameWidth,
                        (1 - startShapeY) * cropFrameHeight
                    )
                )
                shapes.setProperty(
                    shapeGestureIndex, "shapeWidth", side / cropFrameWidth
                )
                shapes.setProperty(
                    shapeGestureIndex, "shapeHeight", side / cropFrameHeight
                )
            } else {
                shapes.setProperty(
                    shapeGestureIndex, "shapeWidth",
                    bounded(startShapeWidth + dx, 0.025, 1 - startShapeX)
                )
                shapes.setProperty(
                    shapeGestureIndex, "shapeHeight",
                    bounded(startShapeHeight + dy, 0.025, 1 - startShapeY)
                )
            }
        } else {
            shapes.setProperty(
                shapeGestureIndex, "shapeX",
                bounded(startShapeX + dx, 0, 1 - startShapeWidth)
            )
            shapes.setProperty(
                shapeGestureIndex, "shapeY",
                bounded(startShapeY + dy, 0, 1 - startShapeHeight)
            )
        }
        editsChanged()
    }

    ListModel { id: shapes }

    Item {
        id: shapeClip
        x: root.cropFrameX
        y: root.cropFrameY
        width: root.cropFrameWidth
        height: root.cropFrameHeight
        clip: true
        z: 7

        Repeater {
            model: shapes
            delegate: Rectangle {
                id: shapeDelegate
                required property int index
                required property real shapeX
                required property real shapeY
                required property real shapeWidth
                required property real shapeHeight
                x: shapeX * shapeClip.width
                y: shapeY * shapeClip.height
                width: shapeWidth * shapeClip.width
                height: shapeHeight * shapeClip.height
                color: "#050506"
                border.width: root.selectedShapeIndex === index ? 2 : 0
                border.color: Theme.accent

                MouseArea {
                    anchors.fill: parent
                    preventStealing: true
                    cursorShape: Qt.SizeAllCursor
                    onPressed: function(mouse) {
                        var point = mapToItem(root, mouse.x, mouse.y)
                        root.beginShapeGesture(shapeDelegate.index, point.x, point.y, false)
                    }
                    onPositionChanged: function(mouse) {
                        if (!pressed)
                            return
                        var point = mapToItem(root, mouse.x, mouse.y)
                        root.updateShapeGesture(point.x, point.y)
                    }
                }

                Rectangle {
                    visible: root.selectedShapeIndex === shapeDelegate.index
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.rightMargin: -8
                    anchors.bottomMargin: -8
                    width: 18
                    height: 18
                    radius: 4
                    color: Theme.accent
                    border.width: 2
                    border.color: Theme.accentContent
                    z: 2
                    MouseArea {
                        anchors.fill: parent
                        preventStealing: true
                        cursorShape: Qt.SizeFDiagCursor
                        onPressed: function(mouse) {
                            var point = mapToItem(root, mouse.x, mouse.y)
                            root.beginShapeGesture(shapeDelegate.index, point.x, point.y, true)
                        }
                        onPositionChanged: function(mouse) {
                            if (!pressed)
                                return
                            var point = mapToItem(root, mouse.x, mouse.y)
                            root.updateShapeGesture(point.x, point.y)
                        }
                    }
                }
            }
        }
    }

    Item {
        visible: root.cropEnabled
        anchors.fill: parent
        z: 6

        Rectangle {
            x: root.videoX
            y: root.videoY
            width: root.videoWidth
            height: Math.max(0, root.cropFrameY - root.videoY)
            color: "#9909080A"
        }
        Rectangle {
            x: root.videoX
            y: root.cropFrameY + root.cropFrameHeight
            width: root.videoWidth
            height: Math.max(0, root.videoY + root.videoHeight - y)
            color: "#9909080A"
        }
        Rectangle {
            x: root.videoX
            y: root.cropFrameY
            width: Math.max(0, root.cropFrameX - root.videoX)
            height: root.cropFrameHeight
            color: "#9909080A"
        }
        Rectangle {
            x: root.cropFrameX + root.cropFrameWidth
            y: root.cropFrameY
            width: Math.max(0, root.videoX + root.videoWidth - x)
            height: root.cropFrameHeight
            color: "#9909080A"
        }

        Rectangle {
            id: cropBorder
            x: root.cropFrameX
            y: root.cropFrameY
            width: root.cropFrameWidth
            height: root.cropFrameHeight
            color: "transparent"
            border.width: 2
            border.color: Theme.accent

            MouseArea {
                anchors.fill: parent
                anchors.margins: 12
                preventStealing: true
                cursorShape: Qt.SizeAllCursor
                onPressed: function(mouse) {
                    var point = mapToItem(root, mouse.x, mouse.y)
                    root.beginCropGesture(point.x, point.y, 4)
                }
                onPositionChanged: function(mouse) {
                    if (!pressed)
                        return
                    var point = mapToItem(root, mouse.x, mouse.y)
                    root.updateCropGesture(point.x, point.y)
                }
            }
        }

        Repeater {
            model: 4
            delegate: Rectangle {
                id: cropHandle
                required property int index
                width: 18
                height: 18
                radius: 4
                color: Theme.accent
                border.width: 2
                border.color: Theme.accentContent
                x: (index === 0 || index === 2)
                    ? root.cropFrameX - width / 2
                    : root.cropFrameX + root.cropFrameWidth - width / 2
                y: (index === 0 || index === 1)
                    ? root.cropFrameY - height / 2
                    : root.cropFrameY + root.cropFrameHeight - height / 2

                MouseArea {
                    anchors.fill: parent
                    preventStealing: true
                    cursorShape: cropHandle.index === 0 || cropHandle.index === 3
                        ? Qt.SizeFDiagCursor : Qt.SizeBDiagCursor
                    onPressed: function(mouse) {
                        var point = mapToItem(root, mouse.x, mouse.y)
                        root.beginCropGesture(point.x, point.y, cropHandle.index)
                    }
                    onPositionChanged: function(mouse) {
                        if (!pressed)
                            return
                        var point = mapToItem(root, mouse.x, mouse.y)
                        root.updateCropGesture(point.x, point.y)
                    }
                }
            }
        }
    }
}
