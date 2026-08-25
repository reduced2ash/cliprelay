from __future__ import annotations

import ctypes
import ctypes.util
import logging
import sys
from dataclasses import dataclass

from PySide6.QtCore import (
    QEvent,
    QObject,
    QPoint,
    QRect,
    QSize,
    Qt,
    QTimer,
    Property,
    Signal,
    Slot,
)
from PySide6.QtGui import QCursor, QGuiApplication, QWindow


LOGGER = logging.getLogger(__name__)
_WINDOW_CORNER_RADIUS = 10.0


@dataclass(frozen=True)
class _GeometryDrag:
    geometry: QRect
    cursor: QPoint
    edges: Qt.Edge = Qt.Edge(0)


def _resized_geometry(
    start: QRect,
    delta: QPoint,
    edges: Qt.Edge,
    minimum: QSize,
    maximum: QSize,
) -> QRect:
    """Return a constrained window rectangle for a manual edge resize."""

    x = start.x()
    y = start.y()
    width = start.width()
    height = start.height()

    if edges & Qt.Edge.LeftEdge:
        x += delta.x()
        width -= delta.x()
    elif edges & Qt.Edge.RightEdge:
        width += delta.x()

    if edges & Qt.Edge.TopEdge:
        y += delta.y()
        height -= delta.y()
    elif edges & Qt.Edge.BottomEdge:
        height += delta.y()

    minimum_width = max(1, minimum.width())
    minimum_height = max(1, minimum.height())
    maximum_width = max(minimum_width, maximum.width())
    maximum_height = max(minimum_height, maximum.height())
    constrained_width = min(max(width, minimum_width), maximum_width)
    constrained_height = min(max(height, minimum_height), maximum_height)

    if edges & Qt.Edge.LeftEdge:
        x += width - constrained_width
    if edges & Qt.Edge.TopEdge:
        y += height - constrained_height

    return QRect(x, y, constrained_width, constrained_height)


class _MacWindowBridge:
    """Small, dependency-free bridge for borderless NSWindow presentation."""

    def __init__(self) -> None:
        self._runtime: ctypes.CDLL | None = None
        if sys.platform != "darwin":
            return
        library = ctypes.util.find_library("objc")
        if not library:
            return
        try:
            self._runtime = ctypes.CDLL(library)
            self._runtime.sel_registerName.restype = ctypes.c_void_p
            self._runtime.sel_registerName.argtypes = [ctypes.c_char_p]
            self._runtime.objc_getClass.restype = ctypes.c_void_p
            self._runtime.objc_getClass.argtypes = [ctypes.c_char_p]
        except (AttributeError, OSError):
            self._runtime = None

    @property
    def available(self) -> bool:
        return self._runtime is not None

    def _selector(self, name: str) -> int:
        assert self._runtime is not None
        return int(self._runtime.sel_registerName(name.encode("ascii")))

    def _class(self, name: str) -> int:
        assert self._runtime is not None
        return int(self._runtime.objc_getClass(name.encode("ascii")))

    def _send(
        self,
        receiver: int,
        selector: str,
        restype: type[ctypes._SimpleCData] | None = None,
        arguments: tuple[object, ...] = (),
        argument_types: tuple[type[ctypes._SimpleCData], ...] = (),
    ) -> object:
        assert self._runtime is not None
        function = self._runtime.objc_msgSend
        function.restype = restype
        function.argtypes = [
            ctypes.c_void_p,
            ctypes.c_void_p,
            *argument_types,
        ]
        return function(
            ctypes.c_void_p(receiver),
            ctypes.c_void_p(self._selector(selector)),
            *arguments,
        )

    def _pointer(self, receiver: int, selector: str) -> int:
        value = self._send(receiver, selector, ctypes.c_void_p)
        return int(value or 0)

    def _set_bool(self, receiver: int, selector: str, value: bool) -> None:
        self._send(
            receiver,
            selector,
            None,
            (ctypes.c_bool(value),),
            (ctypes.c_bool,),
        )

    def _set_double(self, receiver: int, selector: str, value: float) -> None:
        self._send(
            receiver,
            selector,
            None,
            (ctypes.c_double(value),),
            (ctypes.c_double,),
        )

    def _set_pointer(self, receiver: int, selector: str, value: int) -> None:
        self._send(
            receiver,
            selector,
            None,
            (ctypes.c_void_p(value),),
            (ctypes.c_void_p,),
        )

    def _ns_window(self, window: QWindow) -> int:
        view = int(window.winId())
        return self._pointer(view, "window") if view else 0

    def configure(self, window: QWindow) -> None:
        if not self.available:
            return
        try:
            ns_window = self._ns_window(window)
            if not ns_window:
                return
            self._set_bool(ns_window, "setHasShadow:", True)
            self._set_bool(ns_window, "setMovableByWindowBackground:", False)
            self._set_bool(ns_window, "setOpaque:", False)
            clear_color = self._pointer(self._class("NSColor"), "clearColor")
            if clear_color:
                self._set_pointer(
                    ns_window,
                    "setBackgroundColor:",
                    clear_color,
                )
            self.update_shape(window)
        except (AttributeError, OSError, TypeError, ValueError):
            LOGGER.exception("Could not configure the custom macOS window")

    def update_shape(self, window: QWindow) -> None:
        if not self.available:
            return
        try:
            ns_window = self._ns_window(window)
            content_view = (
                self._pointer(ns_window, "contentView")
                if ns_window
                else 0
            )
            if not content_view:
                return
            self._set_bool(content_view, "setWantsLayer:", True)
            layer = self._pointer(content_view, "layer")
            if not layer:
                return
            states = window.windowStates()
            rounded = not bool(
                states
                & (
                    Qt.WindowState.WindowMaximized
                    | Qt.WindowState.WindowFullScreen
                )
            )
            self._set_double(
                layer,
                "setCornerRadius:",
                _WINDOW_CORNER_RADIUS if rounded else 0.0,
            )
            self._set_bool(layer, "setMasksToBounds:", rounded)
            self._send(ns_window, "invalidateShadow", None)
        except (AttributeError, OSError, TypeError, ValueError):
            LOGGER.exception("Could not update the custom macOS window shape")


class NativeWindowController(QObject):
    """Expose dependable frameless-window behavior to QML."""

    activeChanged = Signal()
    windowStateChanged = Signal()

    def __init__(self, parent: QObject | None = None) -> None:
        super().__init__(parent)
        self._window: QWindow | None = None
        self._active = True
        self._maximized = False
        self._fullscreen = False
        self._pre_fullscreen_maximized = False
        self._move_drag: _GeometryDrag | None = None
        self._resize_drag: _GeometryDrag | None = None
        self._mac_bridge = _MacWindowBridge()

    def attach_window(self, window: QWindow) -> None:
        if self._window is window:
            return
        self.detach_window()
        self._window = window
        window.installEventFilter(self)
        self._sync_state()
        QTimer.singleShot(0, lambda: self._configure_native_window(window))

    def detach_window(self) -> None:
        if self._window is not None:
            self._window.removeEventFilter(self)
        self._window = None
        self._move_drag = None
        self._resize_drag = None

    def shutdown(self) -> None:
        self.detach_window()

    def _configure_native_window(self, window: QWindow) -> None:
        if self._window is window:
            self._mac_bridge.configure(window)

    def eventFilter(self, watched: QObject, event: QEvent) -> bool:
        if watched is self._window and event.type() in {
            QEvent.Type.WindowActivate,
            QEvent.Type.WindowDeactivate,
            QEvent.Type.WindowStateChange,
            QEvent.Type.Show,
        }:
            QTimer.singleShot(0, self._sync_state)
        return super().eventFilter(watched, event)

    def _sync_state(self) -> None:
        window = self._window
        if window is None:
            return
        active = window.isActive()
        states = window.windowStates()
        maximized = bool(states & Qt.WindowState.WindowMaximized)
        fullscreen = bool(states & Qt.WindowState.WindowFullScreen)
        if active != self._active:
            self._active = active
            self.activeChanged.emit()
        if (
            maximized != self._maximized
            or fullscreen != self._fullscreen
        ):
            self._maximized = maximized
            self._fullscreen = fullscreen
            self.windowStateChanged.emit()
            self._mac_bridge.update_shape(window)

    @Property(bool, notify=activeChanged)
    def active(self) -> bool:
        return self._active

    @Property(bool, notify=windowStateChanged)
    def maximized(self) -> bool:
        return self._maximized

    @Property(bool, notify=windowStateChanged)
    def fullscreen(self) -> bool:
        return self._fullscreen

    @Property(bool, notify=windowStateChanged)
    def resizable(self) -> bool:
        return not self._maximized and not self._fullscreen

    @Property(bool, constant=True)
    def macos(self) -> bool:
        return sys.platform == "darwin"

    @Slot(result=bool)
    def beginMove(self) -> bool:
        window = self._window
        if window is None or self._fullscreen:
            return False
        self._move_drag = None
        if window.startSystemMove():
            return True
        self._move_drag = _GeometryDrag(
            QRect(window.geometry()),
            QPoint(QCursor.pos()),
        )
        return True

    @Slot()
    def updateMove(self) -> None:
        if self._window is None or self._move_drag is None:
            return
        delta = QCursor.pos() - self._move_drag.cursor
        start = self._move_drag.geometry
        self._window.setPosition(start.x() + delta.x(), start.y() + delta.y())

    @Slot()
    def endMove(self) -> None:
        self._move_drag = None

    @Slot(int, result=bool)
    def beginResize(self, edge_value: int) -> bool:
        window = self._window
        if window is None or not self.resizable:
            return False
        edges = Qt.Edge(edge_value)
        self._resize_drag = None
        if sys.platform != "darwin" and window.startSystemResize(edges):
            return True
        self._resize_drag = _GeometryDrag(
            QRect(window.geometry()),
            QPoint(QCursor.pos()),
            edges,
        )
        return True

    @Slot()
    def updateResize(self) -> None:
        window = self._window
        drag = self._resize_drag
        if window is None or drag is None:
            return
        geometry = _resized_geometry(
            drag.geometry,
            QCursor.pos() - drag.cursor,
            drag.edges,
            window.minimumSize(),
            window.maximumSize(),
        )
        window.setGeometry(geometry)

    @Slot()
    def endResize(self) -> None:
        self._resize_drag = None

    @Slot()
    def closeWindow(self) -> None:
        if self._window is not None:
            self._window.close()

    @Slot()
    def minimizeWindow(self) -> None:
        if self._window is not None:
            self._window.showMinimized()

    @Slot()
    def toggleZoom(self) -> None:
        window = self._window
        if window is None or self._fullscreen:
            return
        if self._maximized:
            window.showNormal()
        else:
            window.showMaximized()
        QTimer.singleShot(0, self._sync_state)

    @Slot()
    def toggleFullscreen(self) -> None:
        window = self._window
        if window is None:
            return
        if self._fullscreen:
            if self._pre_fullscreen_maximized:
                window.showMaximized()
            else:
                window.showNormal()
        else:
            self._pre_fullscreen_maximized = self._maximized
            window.showFullScreen()
        QTimer.singleShot(0, self._sync_state)

    @Slot()
    def performPrimaryZoom(self) -> None:
        modifiers = QGuiApplication.keyboardModifiers()
        if self._mac_bridge.available and not (
            modifiers & Qt.KeyboardModifier.AltModifier
        ):
            self.toggleFullscreen()
        else:
            self.toggleZoom()
