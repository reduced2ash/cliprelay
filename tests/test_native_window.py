from __future__ import annotations

from PySide6.QtCore import QPoint, QRect, QSize, Qt

from cliprelay.native_window import _resized_geometry


MAXIMUM = QSize(16_777_215, 16_777_215)


def test_manual_resize_moves_each_requested_edge() -> None:
    start = QRect(100, 120, 800, 600)

    assert _resized_geometry(
        start,
        QPoint(40, 25),
        Qt.Edge.LeftEdge | Qt.Edge.TopEdge,
        QSize(300, 200),
        MAXIMUM,
    ) == QRect(140, 145, 760, 575)
    assert _resized_geometry(
        start,
        QPoint(40, 25),
        Qt.Edge.RightEdge | Qt.Edge.BottomEdge,
        QSize(300, 200),
        MAXIMUM,
    ) == QRect(100, 120, 840, 625)


def test_manual_resize_keeps_anchored_edge_when_constrained() -> None:
    start = QRect(100, 120, 800, 600)

    assert _resized_geometry(
        start,
        QPoint(700, 500),
        Qt.Edge.LeftEdge | Qt.Edge.TopEdge,
        QSize(420, 360),
        MAXIMUM,
    ) == QRect(480, 360, 420, 360)
    assert _resized_geometry(
        start,
        QPoint(-700, -500),
        Qt.Edge.RightEdge | Qt.Edge.BottomEdge,
        QSize(420, 360),
        MAXIMUM,
    ) == QRect(100, 120, 420, 360)


def test_manual_resize_honors_maximum_size() -> None:
    start = QRect(100, 120, 800, 600)

    assert _resized_geometry(
        start,
        QPoint(-900, -800),
        Qt.Edge.LeftEdge | Qt.Edge.TopEdge,
        QSize(300, 200),
        QSize(1_000, 700),
    ) == QRect(-100, 20, 1_000, 700)
