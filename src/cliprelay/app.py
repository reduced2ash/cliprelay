from __future__ import annotations

import argparse
import asyncio
import logging
import os
import sys
from pathlib import Path

from PySide6.QtCore import (
    QCoreApplication,
    QEvent,
    QMetaObject,
    QObject,
    QTimer,
    Qt,
    QUrl,
)
from PySide6.QtGui import QGuiApplication, QIcon, QSurfaceFormat
from PySide6.QtQml import QQmlApplicationEngine, QQmlEngine
from PySide6.QtQuick import QQuickItem, QQuickWindow, QSGRendererInterface
from PySide6.QtWidgets import QApplication
from qasync import QEventLoop

from . import __version__
from .controller import AppController
from .database import Database
from .native_window import NativeWindowController
from .paths import database_path, log_dir
from .performance import PerformanceMonitor
from .qt_models import FolderModel, HistoryModel, LibraryModel
from .secrets import SecretStore
from .settings import Settings


_MINIMUM_MOUSE_WHEEL_LINES = 14


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="ClipRelay desktop application")
    parser.add_argument(
        "--version",
        action="version",
        version=f"ClipRelay {__version__}",
    )
    parser.add_argument("--data-dir", type=Path, help="Use a separate data directory")
    parser.add_argument("--library", type=Path, help="Open and index this library folder")
    parser.add_argument("--log-level", default="INFO", choices=["DEBUG", "INFO", "WARNING", "ERROR"])
    parser.add_argument("--screenshot", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--page", choices=["library", "history", "settings"], default="library", help=argparse.SUPPRESS)
    parser.add_argument("--window-width", type=int, help=argparse.SUPPRESS)
    parser.add_argument("--window-height", type=int, help=argparse.SUPPRESS)
    parser.add_argument("--scroll-end", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument(
        "--random-sources",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--command-center",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    parser.add_argument("--prepare-fullscreen", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument(
        "--prepare-tab",
        choices=["edit", "publish"],
        default="edit",
        help=argparse.SUPPRESS,
    )
    return parser


def _configure_logging(level: str) -> None:
    target = log_dir() / "cliprelay.log"
    logging.basicConfig(
        level=getattr(logging, level),
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
        handlers=[logging.FileHandler(target, encoding="utf-8"), logging.StreamHandler()],
    )


def _apply_color_scheme(app: QApplication, theme_mode: str) -> None:
    scheme = (
        Qt.ColorScheme.Light
        if theme_mode == "full_white"
        else Qt.ColorScheme.Dark
    )
    hints = app.styleHints()
    if hints.colorScheme() != scheme:
        hints.setColorScheme(scheme)


def _configure_mouse_wheel_scrolling(app: QApplication) -> None:
    if sys.platform != "darwin":
        return
    hints = app.styleHints()
    if hints.wheelScrollLines() < _MINIMUM_MOUSE_WHEEL_LINES:
        hints.setWheelScrollLines(_MINIMUM_MOUSE_WHEEL_LINES)


def _configure_rendering(performance_mode: str) -> None:
    surface_format = QSurfaceFormat.defaultFormat()
    surface_format.setSwapInterval(1)
    QSurfaceFormat.setDefaultFormat(surface_format)
    if performance_mode != "maximum":
        return
    if "QSG_RENDER_LOOP" not in os.environ:
        os.environ["QSG_RENDER_LOOP"] = "threaded"
    if sys.platform == "darwin":
        QQuickWindow.setGraphicsApi(
            QSGRendererInterface.GraphicsApi.Metal
        )
        os.environ.setdefault(
            "QT_FFMPEG_DECODING_HW_DEVICE_TYPES",
            "videotoolbox",
        )


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
    QCoreApplication.setOrganizationName("ClipRelay")
    QCoreApplication.setApplicationName("ClipRelay")
    QGuiApplication.setDesktopFileName("cliprelay")
    _configure_logging(args.log_level)

    db_file = (args.data_dir / "cliprelay.sqlite3") if args.data_dir else database_path()
    database = Database(db_file)
    settings = Settings(database)
    if args.library:
        settings.set("library_root", str(args.library))
    _configure_rendering(str(settings.get("performance_mode", "automatic")))

    app = QApplication(sys.argv[:1])
    app.setApplicationDisplayName("ClipRelay")
    app.setWindowIcon(QIcon(str(Path(__file__).resolve().parent / "assets" / "cliprelay.svg")))
    _configure_mouse_wheel_scrolling(app)
    loop = QEventLoop(app)
    asyncio.set_event_loop(loop)

    _apply_color_scheme(app, str(settings.get("theme_mode", "relay")))
    secrets = SecretStore(args.data_dir if args.data_dir else None)
    library_model = LibraryModel(database)
    folder_model = FolderModel(database)
    history_model = HistoryModel(database)
    controller = AppController(database, settings, secrets, library_model, folder_model, history_model)
    performance_monitor = PerformanceMonitor(
        controller.processor,
        lambda: controller.settings,
    )
    native_window = NativeWindowController(app)
    controller.settingsChanged.connect(
        lambda: _apply_color_scheme(
            app, str(controller.settings.get("theme_mode", "relay"))
        )
    )

    engine = QQmlApplicationEngine()
    QQmlEngine.setObjectOwnership(controller, QQmlEngine.ObjectOwnership.CppOwnership)
    context = engine.rootContext()
    context.setContextProperty("controller", controller)
    context.setContextProperty("libraryModel", library_model)
    context.setContextProperty("folderModel", folder_model)
    context.setContextProperty(
        "randomFolderModel",
        controller.random_folder_model,
    )
    context.setContextProperty("historyModel", history_model)
    context.setContextProperty("performanceMonitor", performance_monitor)
    context.setContextProperty("nativeWindow", native_window)
    qml_file = Path(__file__).resolve().parent / "qml" / "Main.qml"
    engine.load(QUrl.fromLocalFile(str(qml_file)))
    if not engine.rootObjects():
        controller.shutdown()
        return 1
    root_window = engine.rootObjects()[0]
    if isinstance(root_window, QQuickWindow):
        root_window.setPersistentGraphics(True)
        root_window.setPersistentSceneGraph(True)
        performance_monitor.attach_window(root_window)
        native_window.attach_window(root_window)
    controller.settingsChanged.connect(performance_monitor.settingsChanged)
    if args.window_width:
        root_window.setProperty("width", max(940, args.window_width))
    if args.window_height:
        root_window.setProperty("height", max(660, args.window_height))

    app.aboutToQuit.connect(controller.shutdown)
    app.aboutToQuit.connect(performance_monitor.shutdown)
    app.aboutToQuit.connect(native_window.shutdown)
    app.aboutToQuit.connect(loop.stop)
    if args.screenshot:
        async def capture() -> None:
            for _ in range(200):
                if not controller.scanning and controller.counts["media"] > 0:
                    break
                await asyncio.sleep(0.1)
            if library_model.rows:
                controller.selectMedia(int(library_model.rows[0]["mediaId"]))
            root_window.setProperty(
                "currentPage", {"library": 0, "history": 1, "settings": 2}[args.page]
            )
            if args.random_sources and args.page == "library":
                random_sources = root_window.findChild(
                    QObject,
                    "randomSourcePopup",
                )
                if random_sources:
                    QMetaObject.invokeMethod(random_sources, "open")
            if args.command_center:
                command_center = root_window.findChild(
                    QObject,
                    "commandCenter",
                )
                if command_center:
                    QMetaObject.invokeMethod(
                        command_center,
                        "focusCommands",
                    )
            if args.page == "library":
                library_page = root_window.findChild(QObject, "libraryPage")
                if library_page and args.prepare_fullscreen:
                    library_page.setProperty("prepareFullscreen", True)
                prepare_panel = root_window.findChild(QObject, "preparePanel")
                if prepare_panel:
                    prepare_panel.setProperty(
                        "studioTab",
                        1 if args.prepare_tab == "publish" else 0,
                    )
            await asyncio.sleep(1.2)
            if args.scroll_end:
                flickable_name = (
                    "settingsFlickable"
                    if args.page == "settings"
                    else "prepareFlickable"
                )
                flickable = root_window.findChild(QObject, flickable_name)
                if flickable:
                    content_height = float(flickable.property("contentHeight") or 0)
                    viewport_height = float(flickable.property("height") or 0)
                    flickable.setProperty(
                        "contentY",
                        max(0.0, content_height - viewport_height),
                    )
                    await asyncio.sleep(0.35)
            args.screenshot.parent.mkdir(parents=True, exist_ok=True)
            window = root_window
            screen = window.screen() or app.primaryScreen()
            if screen:
                sg = screen.availableGeometry()
                window.setGeometry(sg)
            window.raise_()
            window.requestActivate()
            window.setColor(Qt.GlobalColor.black)

            await asyncio.sleep(0.5)
            for _ in range(100):
                if controller.thumbnailJobCount == 0:
                    break
                await asyncio.sleep(0.15)
            await asyncio.sleep(0.5)
            captured = False
            if isinstance(window, QQuickWindow):
                image = window.grabWindow()
                if not image.isNull():
                    captured = image.save(str(args.screenshot))
            if not captured:
                content_item = window.property("contentItem")
                if isinstance(content_item, QQuickItem):
                    grab_pointer = content_item.grabToImage()
                    grab_result = grab_pointer.data()
                    grab_ready = loop.create_future()

                    def mark_grab_ready() -> None:
                        if not grab_ready.done():
                            grab_ready.set_result(None)

                    grab_result.ready.connect(mark_grab_ready)
                    try:
                        await asyncio.wait_for(grab_ready, timeout=2)
                        captured = grab_result.saveToFile(
                            str(args.screenshot)
                        )
                    except TimeoutError:
                        pass
            if not captured and screen:
                image = screen.grabWindow(int(window.winId()))
                if image.isNull():
                    frame = window.frameGeometry()
                    image = screen.grabWindow(
                        0,
                        int(frame.x()),
                        int(frame.y()),
                        int(frame.width()),
                        int(frame.height()),
                    )
                image.save(str(args.screenshot))
            controller.shutdown()
            performance_monitor.shutdown()
            await asyncio.sleep(0)
            loop.stop()

        asyncio.ensure_future(capture())

    with loop:
        loop.run_forever()
    controller.shutdown()
    performance_monitor.shutdown()
    native_window.shutdown()
    for root_object in engine.rootObjects():
        root_object.deleteLater()
    engine.deleteLater()
    QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)
    app.processEvents()
    return 0
