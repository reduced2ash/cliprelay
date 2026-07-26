from __future__ import annotations

import argparse
import asyncio
import logging
import os
import sys
from pathlib import Path

from PySide6.QtCore import QCoreApplication, QEvent, QObject, QTimer, Qt, QUrl
from PySide6.QtGui import QGuiApplication, QIcon
from PySide6.QtQml import QQmlApplicationEngine, QQmlEngine
from PySide6.QtWidgets import QApplication
from qasync import QEventLoop

from . import __version__
from .controller import AppController
from .database import Database
from .paths import database_path, log_dir
from .qt_models import FolderModel, HistoryModel, LibraryModel
from .secrets import SecretStore
from .settings import Settings


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


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
    QCoreApplication.setOrganizationName("ClipRelay")
    QCoreApplication.setApplicationName("ClipRelay")
    QGuiApplication.setDesktopFileName("cliprelay")
    _configure_logging(args.log_level)

    app = QApplication(sys.argv[:1])
    app.setApplicationDisplayName("ClipRelay")
    app.setWindowIcon(QIcon(str(Path(__file__).resolve().parent / "assets" / "cliprelay.svg")))
    loop = QEventLoop(app)
    asyncio.set_event_loop(loop)

    db_file = (args.data_dir / "cliprelay.sqlite3") if args.data_dir else database_path()
    database = Database(db_file)
    settings = Settings(database)
    if args.library:
        settings.set("library_root", str(args.library))
    _apply_color_scheme(app, str(settings.get("theme_mode", "relay")))
    secrets = SecretStore(args.data_dir if args.data_dir else None)
    library_model = LibraryModel(database)
    folder_model = FolderModel(database)
    history_model = HistoryModel(database)
    controller = AppController(database, settings, secrets, library_model, folder_model, history_model)
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
    context.setContextProperty("historyModel", history_model)
    qml_file = Path(__file__).resolve().parent / "qml" / "Main.qml"
    engine.load(QUrl.fromLocalFile(str(qml_file)))
    if not engine.rootObjects():
        controller.shutdown()
        return 1
    root_window = engine.rootObjects()[0]
    if args.window_width:
        root_window.setProperty("width", max(940, args.window_width))
    if args.window_height:
        root_window.setProperty("height", max(660, args.window_height))

    app.aboutToQuit.connect(controller.shutdown)
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
            if args.prepare_fullscreen and args.page == "library":
                library_page = root_window.findChild(QObject, "libraryPage")
                if library_page:
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
            window.raise_()
            window.requestActivate()
            await asyncio.sleep(0.25)
            screen = window.screen() or app.primaryScreen()
            if screen:
                frame = window.frameGeometry()
                screen.grabWindow(
                    0,
                    int(frame.x()),
                    int(frame.y()),
                    int(frame.width()),
                    int(frame.height()),
                ).save(str(args.screenshot))
            loop.stop()

        asyncio.ensure_future(capture())

    with loop:
        loop.run_forever()
    for root_object in engine.rootObjects():
        root_object.deleteLater()
    engine.deleteLater()
    QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)
    app.processEvents()
    return 0
