from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from PySide6.QtCore import QObject, Property, QTimer, QUrl, Signal, Slot
from PySide6.QtGui import QDesktopServices
from watchdog.events import (
    EVENT_TYPE_CREATED,
    EVENT_TYPE_DELETED,
    EVENT_TYPE_MODIFIED,
    EVENT_TYPE_MOVED,
    FileSystemEvent,
    FileSystemEventHandler,
)
from watchdog.observers import Observer

from .cleanup import CleanupError, move_generated_to_trash
from .database import Database
from .media import (
    MediaError,
    MediaIndexer,
    MediaProcessor,
    ProcessingCancelled,
    normalize_edit_spec,
)
from .paths import ffmpeg_path, ffprobe_path, is_within
from .qt_models import FolderModel, HistoryModel, LibraryModel, RandomFolderModel
from .secrets import SecretStore
from .settings import Settings
from .telegram import (
    TelegramBotService,
    TelegramError,
    TelegramPasswordRequired,
    TelegramPersonalService,
)
from .utils import format_bytes, format_duration
from .x_assist import XAssistant


LOGGER = logging.getLogger(__name__)


@dataclass(frozen=True, slots=True)
class _NavigationState:
    folder: str
    search: str
    media_id: int


class _WatchHandler(FileSystemEventHandler):
    _MUTATING_EVENT_TYPES = frozenset({
        EVENT_TYPE_CREATED,
        EVENT_TYPE_DELETED,
        EVENT_TYPE_MODIFIED,
        EVENT_TYPE_MOVED,
    })

    def __init__(
        self,
        callback: Callable[[], None],
        should_refresh: Callable[[FileSystemEvent], bool] | None = None,
    ):
        super().__init__()
        self.callback = callback
        self.should_refresh = should_refresh
        self._last_emit = 0.0

    def on_any_event(self, event: FileSystemEvent) -> None:
        if (
            event.is_directory
            or event.event_type not in self._MUTATING_EVENT_TYPES
        ):
            return
        now = time.monotonic()
        if now - self._last_emit < 0.35:
            return
        if self.should_refresh and not self.should_refresh(event):
            return
        self._last_emit = now
        self.callback()


class AppController(QObject):
    settingsChanged = Signal()
    selectedMediaChanged = Signal()
    scanStateChanged = Signal()
    randomStateChanged = Signal()
    randomFoldersChanged = Signal()
    timelineStateChanged = Signal()
    selectionNavigationChanged = Signal()
    navigationHistoryChanged = Signal()
    countsChanged = Signal()
    publishStateChanged = Signal()
    telegramStateChanged = Signal()
    telegramDialogsChanged = Signal()
    commandSearchChanged = Signal()
    navigationRequested = Signal(str)
    libraryRevealRequested = Signal(str, int, int)
    libraryNavigationRestored = Signal(str, str, int)
    librarySelectionRequested = Signal(int)
    toast = Signal(str, str)
    _filesystemChanged = Signal()
    _scanProgress = Signal(int, int, str)
    _manifestProgress = Signal(int, str)
    _mediaIndexed = Signal(int)

    def __init__(
        self,
        database: Database,
        settings: Settings,
        secrets: SecretStore,
        library_model: LibraryModel,
        folder_model: FolderModel,
        history_model: HistoryModel,
    ):
        super().__init__()
        self.database = database
        self.settings_store = settings
        self._settings_cache = settings.as_dict()
        self.secrets = secrets
        self.library_model = library_model
        self.folder_model = folder_model
        self.history_model = history_model
        self.random_folder_model = RandomFolderModel()
        self.indexer = MediaIndexer(database, self._settings_cache["export_dir"])
        self.processor = MediaProcessor(self._settings_cache["export_dir"])
        self.processor.set_encoder_mode(self._effective_export_encoder())
        self.bot = TelegramBotService(secrets)
        self.personal = TelegramPersonalService(secrets)
        self.x_assistant = XAssistant()
        self._bot_configured = bool(
            self._setting("telegram_bot_configured")
            or (
                self._setting("telegram_mode") == "bot"
                and self._setting("telegram_destination")
            )
        )
        self._personal_configured = bool(
            self._setting("telegram_personal_configured")
            or (
                self._setting("telegram_mode") == "personal"
                and self._setting("telegram_api_id")
                and self._setting("telegram_phone")
            )
        )
        self._ffmpeg_available = bool(self.indexer.ffmpeg)
        self._ffprobe_available = bool(self.indexer.ffprobe)
        self._selected_id = 0
        self._selected: dict[str, Any] = {}
        self._selected_checking = False
        self._selection_previous_id = 0
        self._selection_next_id = 0
        self._selection_navigation_generation = 0
        self._selection_navigation_task: asyncio.Task | None = None
        self._navigation_back: list[_NavigationState] = []
        self._navigation_forward: list[_NavigationState] = []
        self._navigation_restoring = False
        self._pending_navigation_focus_media_id = 0
        self._scanning = False
        self._random_picking = False
        self._random_folder_options_cache: list[dict[str, Any]] = []
        self._random_folder_options_dirty = True
        self._random_folder_options_loading = False
        self._random_folder_generation = 0
        self._random_folder_task: asyncio.Task | None = None
        self._command_search_results: list[dict[str, Any]] = []
        self._command_search_loading = False
        self._command_search_generation = 0
        self._command_search_task: asyncio.Task | None = None
        self._pending_refresh = False
        self._pending_index = False
        self._shutting_down = False
        self._thumbnail_jobs: dict[int, asyncio.Task] = {}
        maximum_performance = self._maximum_performance()
        self._thumbnail_semaphore = asyncio.Semaphore(
            4 if maximum_performance else 2
        )
        self._preview_jobs: dict[int, asyncio.Task] = {}
        self._preview_semaphore = asyncio.Semaphore(
            2 if maximum_performance else 1
        )
        self._timeline_task: asyncio.Task | None = None
        self._timeline_media_id = 0
        self._timeline_generation = 0
        self._timeline_loading = False
        self._timeline_semaphore = asyncio.Semaphore(1)
        self._load_more_task: asyncio.Task | None = None
        self._library_scan_task: asyncio.Task | None = None
        self._full_refresh_task: asyncio.Task | None = None
        self._library_model_refresh_task: asyncio.Task | None = None
        self._history_model_refresh_task: asyncio.Task | None = None
        self._counts_refresh_task: asyncio.Task | None = None
        self._shuffle_reset_task: asyncio.Task | None = None
        self._root_activation_task: asyncio.Task | None = None
        self._full_refresh_generation = 0
        self._root_activation_generation = 0
        try:
            asyncio.get_running_loop()
        except RuntimeError:
            # The packaged app constructs its controller before qasync starts.
            # Defer all startup SQLite/model queries until the event loop can
            # run them in worker threads. Unit embedders that construct the
            # controller inside a running loop retain immediate initialization.
            self._runtime_model_workers = True
        else:
            self._runtime_model_workers = False
        self._counts_cache: dict[str, int] = {"media": 0, "posts": 0, "unseen": 0}
        self._scan_progress = 0.0
        self._scan_message = ""
        self._publish_state: dict[str, Any] = {
            "active": False, "progress": 0.0, "stage": "", "postId": 0,
            "error": "", "outputEncoder": "", "hardwareAccelerated": False,
        }
        self._telegram_state: dict[str, Any] = {
            "bot": "configured" if self._bot_configured else "not configured",
            "personal": "signed in" if self._personal_configured else "not signed in",
            "message": "",
            "passwordRequired": False,
        }
        self._telegram_dialogs: list[dict[str, Any]] = []
        self._observer: Observer | None = None
        self._rescan_timer = QTimer(self)
        self._rescan_timer.setSingleShot(True)
        self._rescan_timer.setInterval(1400)
        self._rescan_timer.timeout.connect(self._refresh_manifest_only)
        self._auto_scan_timer = QTimer(self)
        self._auto_scan_timer.setSingleShot(True)
        self._auto_scan_timer.setInterval(80)
        self._auto_scan_timer.timeout.connect(self._refresh_automatic)
        self._startup_refresh_timer = QTimer(self)
        self._startup_refresh_timer.setSingleShot(True)
        self._startup_refresh_timer.setInterval(4000)
        self._startup_refresh_timer.timeout.connect(self._refresh_manifest_only)
        self._library_refresh_timer = QTimer(self)
        self._library_refresh_timer.setSingleShot(True)
        self._library_refresh_timer.setInterval(900)
        self._library_refresh_timer.timeout.connect(self._refresh_indexed_media)
        self._watcher_timer = QTimer(self)
        self._watcher_timer.setSingleShot(True)
        self._watcher_timer.setInterval(1000)
        self._watcher_timer.timeout.connect(self._start_watcher)
        self._filesystemChanged.connect(self._schedule_rescan)
        self._scanProgress.connect(self._on_scan_progress)
        self._manifestProgress.connect(self._on_manifest_progress)
        self._mediaIndexed.connect(self._on_media_indexed)
        root = self._setting("library_root")
        active_root = root if root and Path(root).is_dir() else None
        if self._runtime_model_workers:
            self.database.set_active_root(active_root)
            self.library_model.sort_mode = self._setting("sort_mode")
            QTimer.singleShot(0, self._start_initial_model_refresh)
        else:
            self.database.activate_root(active_root)
            self.refresh_all()
        if root and Path(root).is_dir():
            self._watcher_timer.start()
            self._startup_refresh_timer.start()

    @Property("QVariantMap", notify=settingsChanged)
    def settings(self) -> dict[str, Any]:
        values = dict(self._settings_cache)
        values.update({
            "botConfigured": self._bot_configured,
            "personalConfigured": self._personal_configured,
            "secretBackend": self.secrets.backend,
            "ffmpegAvailable": self._ffmpeg_available,
            "ffprobeAvailable": self._ffprobe_available,
        })
        return values

    def _setting(self, key: str, default: Any = None) -> Any:
        return self._settings_cache.get(key, default)

    def _maximum_performance(self) -> bool:
        return self._setting("performance_mode") == "maximum"

    def _effective_export_encoder(self) -> str:
        preference = str(self._setting("export_encoder", "auto"))
        if preference == "hardware":
            return "hardware"
        if preference == "software":
            return "software"
        return "hardware" if self._maximum_performance() else "software"

    def _store_setting(self, key: str, value: Any) -> None:
        self.settings_store.set(key, value)
        self._settings_cache[key] = self.settings_store.get(key, value)

    @Property("QVariantMap", notify=selectedMediaChanged)
    def selectedMedia(self) -> dict[str, Any]:
        return self._selected

    @Property(int, notify=selectedMediaChanged)
    def selectedMediaId(self) -> int:
        return self._selected_id

    @Property(bool, notify=selectedMediaChanged)
    def selectedMediaChecking(self) -> bool:
        return self._selected_checking

    @Property(bool, notify=timelineStateChanged)
    def selectedMediaTimelineLoading(self) -> bool:
        return self._timeline_loading

    @Property(bool, notify=selectionNavigationChanged)
    def canSelectPrevious(self) -> bool:
        return self._selection_previous_id > 0

    @Property(bool, notify=selectionNavigationChanged)
    def canSelectNext(self) -> bool:
        return self._selection_next_id > 0

    @Property(bool, notify=navigationHistoryChanged)
    def canNavigateBack(self) -> bool:
        return bool(self._navigation_back)

    @Property(bool, notify=navigationHistoryChanged)
    def canNavigateForward(self) -> bool:
        return bool(self._navigation_forward)

    @Property(bool, notify=scanStateChanged)
    def scanning(self) -> bool:
        return self._scanning

    @Property(bool, notify=randomStateChanged)
    def randomPicking(self) -> bool:
        return self._random_picking

    def _stored_random_folders(self) -> list[str]:
        value = self._setting("random_folders", [])
        if not isinstance(value, list):
            return []
        return list(
            dict.fromkeys(
                str(folder)
                for folder in value
                if isinstance(folder, str)
            )
        )

    def _random_folder_mode(self) -> str:
        return (
            "selected"
            if self._setting("random_folder_mode", "all") == "selected"
            else "all"
        )

    def _all_random_folder_paths(self) -> list[str]:
        return [
            str(row.get("folder") or "")
            for row in self._random_folder_options_cache
        ]

    def _selected_random_folders(self) -> list[str]:
        if self._random_folder_mode() == "all":
            return self._all_random_folder_paths()
        return self._stored_random_folders()

    def _random_folders(self) -> list[str]:
        if self._random_folder_mode() == "all":
            return []
        return self._stored_random_folders()

    @Property("QVariantList", notify=randomFoldersChanged)
    def randomFolderOptions(self) -> list[dict[str, Any]]:
        selected = set(self._selected_random_folders())
        return [
            {
                "folderPath": str(row["folder"]),
                "folderName": (
                    str(row["folder"]).replace("/", "  /  ")
                    if row["folder"]
                    else "Library root only"
                ),
                "videoCount": int(row["count"]),
                "selected": str(row["folder"]) in selected,
            }
            for row in self._random_folder_options_cache
        ]

    @Property(bool, notify=randomFoldersChanged)
    def randomFolderOptionsLoading(self) -> bool:
        return self._random_folder_options_loading

    @Property(str, notify=randomFoldersChanged)
    def randomFolderSummary(self) -> str:
        if self._random_folder_mode() == "all":
            return "All folders"
        folders = self._stored_random_folders()
        if not folders:
            return "No folders"
        all_paths = set(self._all_random_folder_paths())
        if all_paths and all_paths.issubset(folders):
            return "All folders"
        if len(folders) > 1:
            return f"{len(folders)} folders"
        if not folders[0]:
            return "Library root"
        return folders[0].rsplit("/", 1)[-1]

    @Property(int, notify=randomFoldersChanged)
    def randomFolderSelectionCount(self) -> int:
        return len(self._selected_random_folders())

    @Property(bool, notify=randomFoldersChanged)
    def allRandomFoldersSelected(self) -> bool:
        if self._random_folder_mode() == "all":
            return True
        all_paths = set(self._all_random_folder_paths())
        return bool(all_paths) and all_paths.issubset(
            self._stored_random_folders()
        )

    @Property(bool, notify=randomFoldersChanged)
    def hasRandomFolderSelection(self) -> bool:
        return (
            self._random_folder_mode() == "all"
            or bool(self._stored_random_folders())
        )

    @Property(float, notify=scanStateChanged)
    def scanProgress(self) -> float:
        return self._scan_progress

    @Property(str, notify=scanStateChanged)
    def scanMessage(self) -> str:
        return self._scan_message

    @Property("QVariantMap", notify=countsChanged)
    def counts(self) -> dict[str, int]:
        return dict(self._counts_cache)

    @Property("QVariantMap", notify=publishStateChanged)
    def publishState(self) -> dict[str, Any]:
        return self._publish_state

    @Property("QVariantMap", notify=telegramStateChanged)
    def telegramState(self) -> dict[str, Any]:
        return self._telegram_state

    @Property("QVariantList", notify=telegramDialogsChanged)
    def telegramDialogs(self) -> list[dict[str, Any]]:
        return self._telegram_dialogs

    @Property("QVariantList", notify=commandSearchChanged)
    def commandSearchResults(self) -> list[dict[str, Any]]:
        return list(self._command_search_results)

    @Property(bool, notify=commandSearchChanged)
    def commandSearchLoading(self) -> bool:
        return self._command_search_loading

    @Slot()
    def _start_initial_model_refresh(self) -> None:
        if self._can_use_model_workers():
            self._queue_root_activation(
                self._setting("library_root") or None
            )
        else:
            self.refresh_all()

    def _queue_root_activation(
        self,
        root: str | Path | None,
    ) -> None:
        self._root_activation_generation += 1
        generation = self._root_activation_generation
        if self._root_activation_task:
            self._root_activation_task.cancel()
        task = asyncio.create_task(
            self._activate_root_async(root, generation)
        )
        self._root_activation_task = task
        task.add_done_callback(
            lambda completed: self._simple_task_done(
                "_root_activation_task",
                completed,
            )
        )

    async def _activate_root_async(
        self,
        root: str | Path | None,
        generation: int,
    ) -> None:
        try:
            await asyncio.to_thread(
                self.database.activate_root,
                root,
            )
            if (
                self._shutting_down
                or generation != self._root_activation_generation
            ):
                return
            self.refresh_all()
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.warning("Could not activate library root: %s", exc)
            self.toast.emit("error", "The library database could not be updated.")

    def _can_use_model_workers(self) -> bool:
        if not self._runtime_model_workers or self._shutting_down:
            return False
        try:
            asyncio.get_running_loop()
        except RuntimeError:
            return False
        return True

    def refresh_all(self) -> None:
        self.library_model.sort_mode = self._setting("sort_mode")
        if self._can_use_model_workers():
            self._queue_full_model_refresh()
            return
        self.library_model.refresh()
        self._flush_navigation_focus()
        self.folder_model.refresh()
        self.history_model.refresh()
        self._refresh_counts()
        self._invalidate_random_folder_options()

    def _queue_full_model_refresh(self) -> None:
        self._full_refresh_generation += 1
        generation = self._full_refresh_generation
        if self._full_refresh_task:
            self._full_refresh_task.cancel()
        library_request = self.library_model.refresh_request()
        history_request = self.history_model.refresh_request()
        task = asyncio.create_task(
            self._refresh_all_models_async(
                generation,
                library_request,
                history_request,
            )
        )
        self._full_refresh_task = task
        task.add_done_callback(
            lambda completed, request=generation:
                self._model_refresh_done(
                    "_full_refresh_task",
                    request,
                    completed,
                )
        )

    async def _refresh_all_models_async(
        self,
        generation: int,
        library_request: tuple[int, str, str, str, int],
        history_request: tuple[int, str, int],
    ) -> None:
        try:
            library_payload, folder_payload, history_payload, counts = (
                await asyncio.gather(
                    asyncio.to_thread(
                        self.library_model.fetch_refresh,
                        library_request,
                    ),
                    asyncio.to_thread(self.folder_model.fetch_refresh),
                    asyncio.to_thread(
                        self.history_model.fetch_refresh,
                        history_request,
                    ),
                    asyncio.to_thread(self.database.counts),
                )
            )
            if (
                self._shutting_down
                or generation != self._full_refresh_generation
            ):
                return
            self.library_model.apply_refresh(library_payload)
            self._flush_navigation_focus()
            self.folder_model.apply_refresh(folder_payload)
            self.history_model.apply_refresh(history_payload)
            self._counts_cache = counts
            self.countsChanged.emit()
            self._invalidate_random_folder_options()
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.warning("Background model refresh failed: %s", exc)

    def _model_refresh_done(
        self,
        attribute: str,
        _generation: int,
        task: asyncio.Task,
    ) -> None:
        if getattr(self, attribute, None) is task:
            setattr(self, attribute, None)
        if not task.cancelled() and (error := task.exception()):
            LOGGER.debug("Model refresh task failed: %s", error)

    def _refresh_counts(self) -> None:
        if self._can_use_model_workers():
            if self._counts_refresh_task:
                self._counts_refresh_task.cancel()
            task = asyncio.create_task(self._refresh_counts_async())
            self._counts_refresh_task = task
            task.add_done_callback(
                lambda completed: self._simple_task_done(
                    "_counts_refresh_task",
                    completed,
                )
            )
            return
        self._counts_cache = self.database.counts()
        self.countsChanged.emit()

    async def _refresh_counts_async(self) -> None:
        counts = await asyncio.to_thread(self.database.counts)
        if self._shutting_down:
            return
        self._counts_cache = counts
        self.countsChanged.emit()

    def _simple_task_done(
        self,
        attribute: str,
        task: asyncio.Task,
    ) -> None:
        if getattr(self, attribute, None) is task:
            setattr(self, attribute, None)
        if not task.cancelled() and (error := task.exception()):
            LOGGER.debug("%s failed: %s", attribute, error)

    def _refresh_library_state(self, preserve_loaded: bool = True) -> None:
        if self._can_use_model_workers():
            self._queue_library_state_refresh(preserve_loaded)
            return
        self.library_model.refresh(preserve_loaded=preserve_loaded)
        self._flush_navigation_focus()
        self.folder_model.refresh()
        self._refresh_counts()
        self._invalidate_random_folder_options()

    def _queue_library_state_refresh(self, preserve_loaded: bool) -> None:
        self._full_refresh_generation += 1
        generation = self._full_refresh_generation
        if self._full_refresh_task:
            self._full_refresh_task.cancel()
        if self._library_model_refresh_task:
            self._library_model_refresh_task.cancel()
        request = self.library_model.refresh_request(preserve_loaded)
        task = asyncio.create_task(
            self._refresh_library_state_async(generation, request)
        )
        self._full_refresh_task = task
        task.add_done_callback(
            lambda completed, request_generation=generation:
                self._model_refresh_done(
                    "_full_refresh_task",
                    request_generation,
                    completed,
                )
        )

    async def _refresh_library_state_async(
        self,
        generation: int,
        request: tuple[int, str, str, str, int],
    ) -> None:
        try:
            library_payload, folder_payload, counts = await asyncio.gather(
                asyncio.to_thread(
                    self.library_model.fetch_refresh,
                    request,
                ),
                asyncio.to_thread(self.folder_model.fetch_refresh),
                asyncio.to_thread(self.database.counts),
            )
            if (
                self._shutting_down
                or generation != self._full_refresh_generation
            ):
                return
            self.library_model.apply_refresh(library_payload)
            self._flush_navigation_focus()
            self.folder_model.apply_refresh(folder_payload)
            self._counts_cache = counts
            self.countsChanged.emit()
            self._invalidate_random_folder_options()
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.warning("Background library refresh failed: %s", exc)

    def _invalidate_random_folder_options(self, clear: bool = False) -> None:
        self._random_folder_generation += 1
        self._random_folder_options_dirty = True
        if clear:
            self._random_folder_options_cache = []
            self.random_folder_model.clear()
        self.randomFoldersChanged.emit()

    def _map_selected(self, row: dict[str, Any] | None) -> dict[str, Any]:
        if not row:
            return {}
        path = Path(row["path"])
        thumb = row.get("thumbnail_path")
        preview = row.get("preview_path")
        timeline = row.get("timeline_path")
        checked = float(row["duration"]) > 0
        return {
            "mediaId": int(row["id"]), "path": str(path),
            "mediaUrl": QUrl.fromLocalFile(str(path)).toString(), "name": row["name"],
            "relativePath": row["relative_path"], "folder": row["folder"],
            "duration": float(row["duration"]),
            "durationLabel": format_duration(row["duration"]) if checked else "Unchecked",
            "width": int(row["width"]), "height": int(row["height"]),
            "resolution": f"{row['width']}×{row['height']}" if checked else "Unchecked",
            "sizeBytes": int(row["size_bytes"]), "sizeLabel": format_bytes(row["size_bytes"]),
            "codec": row["video_codec"], "audioCodec": row["audio_codec"],
            "frameRate": float(row["frame_rate"]),
            "thumbnailUrl": QUrl.fromLocalFile(thumb).toString() if thumb and Path(thumb).is_file() else "",
            "previewUrl": QUrl.fromLocalFile(preview).toString() if preview and Path(preview).is_file() else "",
            "timelineUrl": QUrl.fromLocalFile(timeline).toString() if timeline and Path(timeline).is_file() else "",
            "postedCount": int(row["posted_count"]), "seen": bool(row["seen"]),
        }

    def _set_selected(
        self,
        row: dict[str, Any] | None,
        checking: bool | None = None,
    ) -> None:
        previous_id = self._selected_id
        self._selected_id = int(row["id"]) if row else 0
        if self._selected_id != previous_id:
            self._cancel_timeline_generation()
        self._selected = self._map_selected(row)
        self._selected_checking = (
            bool(checking)
            if checking is not None
            else bool(row and float(row.get("duration") or 0) <= 0)
        )
        self.selectedMediaChanged.emit()
        if self._selected_id != previous_id:
            self.randomStateChanged.emit()
            self._queue_selection_neighbors()

    def _navigation_state(self) -> _NavigationState:
        return _NavigationState(
            folder=str(self.library_model.folder or ""),
            search=str(self.library_model.search or ""),
            media_id=max(0, int(self._selected_id)),
        )

    @staticmethod
    def _append_navigation_state(
        stack: list[_NavigationState],
        state: _NavigationState,
    ) -> bool:
        if stack and stack[-1] == state:
            return False
        stack.append(state)
        if len(stack) > 200:
            del stack[:-200]
        return True

    def _record_navigation_origin(self) -> None:
        if self._navigation_restoring:
            return
        changed = self._append_navigation_state(
            self._navigation_back,
            self._navigation_state(),
        )
        if self._navigation_forward:
            self._navigation_forward.clear()
            changed = True
        if changed:
            self.navigationHistoryChanged.emit()

    def _clear_navigation_history(self) -> None:
        changed = bool(self._navigation_back or self._navigation_forward)
        self._navigation_back.clear()
        self._navigation_forward.clear()
        self._pending_navigation_focus_media_id = 0
        if changed:
            self.navigationHistoryChanged.emit()

    def _resolve_navigation_state(
        self,
        state: _NavigationState,
        *,
        skip_missing_media: bool = False,
    ) -> _NavigationState | None:
        root = str(self._setting("library_root") or "")
        media_id = max(0, int(state.media_id))
        if media_id:
            row = self.database.get_media(media_id)
            row_root = str(row.get("root_path") or "") if row else ""
            available = bool(
                row
                and bool(row.get("active", 1))
                and bool(row.get("valid", 1))
                and Path(str(row.get("path") or "")).is_file()
                and root
                and Path(row_root).expanduser().resolve()
                == Path(root).expanduser().resolve()
            )
            if not available:
                if skip_missing_media:
                    return None
                media_id = 0

        folder = str(state.folder or "")
        if folder:
            folder_path = (
                Path(root).expanduser() / Path(folder)
                if root
                else Path()
            )
            if (
                not root
                or not is_within(folder_path, root)
                or not folder_path.is_dir()
            ):
                folder = ""

        return _NavigationState(
            folder=folder,
            search=str(state.search or ""),
            media_id=media_id,
        )

    def _flush_navigation_focus(self) -> None:
        media_id = self._pending_navigation_focus_media_id
        if media_id <= 0:
            return
        self._pending_navigation_focus_media_id = 0
        media_index = self.library_model.find_index(media_id)
        if media_index >= 0:
            self.librarySelectionRequested.emit(media_index)

    def _restore_navigation_state(
        self,
        state: _NavigationState,
    ) -> None:
        self._navigation_restoring = True
        try:
            self._cancel_load_more()
            self.library_model.folder = state.folder
            self.library_model.search = state.search
            self._select_media(state.media_id, record_navigation=False)
            folder_index = self.folder_model.find_index(state.folder)
            self.navigationRequested.emit("library")
            self.libraryNavigationRestored.emit(
                state.folder,
                state.search,
                folder_index,
            )
            self._pending_navigation_focus_media_id = state.media_id
            self._refresh_library_model_only()
            self._queue_selection_neighbors()
        finally:
            self._navigation_restoring = False

    def _navigate_history(
        self,
        source: list[_NavigationState],
        destination: list[_NavigationState],
    ) -> None:
        current = self._resolve_navigation_state(
            self._navigation_state()
        )
        if current is None:
            return
        changed = False
        while source:
            candidate = self._resolve_navigation_state(
                source.pop(),
                skip_missing_media=True,
            )
            changed = True
            if candidate is None:
                continue
            if candidate == current:
                continue
            self._append_navigation_state(destination, current)
            self._restore_navigation_state(candidate)
            self.navigationHistoryChanged.emit()
            return
        if changed:
            self.navigationHistoryChanged.emit()

    def _queue_selection_neighbors(self) -> None:
        self._selection_navigation_generation += 1
        generation = self._selection_navigation_generation
        if self._selection_navigation_task:
            self._selection_navigation_task.cancel()
            self._selection_navigation_task = None
        self._selection_previous_id = 0
        self._selection_next_id = 0
        self.selectionNavigationChanged.emit()
        if self._selected_id <= 0 or self._shutting_down:
            return
        media_id = self._selected_id
        search = self.library_model.search
        folder = self.library_model.folder
        sort_mode = self.library_model.sort_mode
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            result = self.database.navigation_neighbors(
                media_id,
                search,
                folder,
                sort_mode,
            )
            if not result["found"] and (search or folder):
                result = self.database.navigation_neighbors(
                    media_id,
                    "",
                    "",
                    sort_mode,
                )
            self._apply_selection_neighbors(media_id, generation, result)
            return
        self._selection_navigation_task = loop.create_task(
            self._load_selection_neighbors_async(
                media_id,
                generation,
                search,
                folder,
                sort_mode,
            )
        )

    def _apply_selection_neighbors(
        self,
        media_id: int,
        generation: int,
        result: dict[str, int | bool],
    ) -> None:
        if (
            generation != self._selection_navigation_generation
            or media_id != self._selected_id
        ):
            return
        self._selection_previous_id = int(result.get("previousId") or 0)
        self._selection_next_id = int(result.get("nextId") or 0)
        self.selectionNavigationChanged.emit()
        self._preload_selection_neighbors()

    def _preload_selection_neighbors(self) -> None:
        if not self._maximum_performance() or self._shutting_down:
            return
        for media_id in (
            self._selection_previous_id,
            self._selection_next_id,
        ):
            if media_id <= 0:
                continue
            self.ensureThumbnail(media_id)
            if bool(self._setting("hover_previews")):
                self.ensurePreview(media_id)

    async def _load_selection_neighbors_async(
        self,
        media_id: int,
        generation: int,
        search: str,
        folder: str,
        sort_mode: str,
    ) -> None:
        try:
            result = await asyncio.to_thread(
                self.database.navigation_neighbors,
                media_id,
                search,
                folder,
                sort_mode,
            )
            if not result["found"] and (search or folder):
                result = await asyncio.to_thread(
                    self.database.navigation_neighbors,
                    media_id,
                    "",
                    "",
                    sort_mode,
                )
            self._apply_selection_neighbors(media_id, generation, result)
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.debug("Could not load adjacent videos: %s", exc)
        finally:
            if generation == self._selection_navigation_generation:
                self._selection_navigation_task = None

    def _set_publish(self, **changes: Any) -> None:
        self._publish_state = {**self._publish_state, **changes}
        self.publishStateChanged.emit()

    def _set_scan(self, *, active: bool | None = None, progress: float | None = None, message: str | None = None) -> None:
        if active is not None:
            self._scanning = active
        if progress is not None:
            self._scan_progress = progress
        if message is not None:
            self._scan_message = message
        self.scanStateChanged.emit()

    def _set_telegram(self, **changes: Any) -> None:
        self._telegram_state = {**self._telegram_state, **changes}
        self.telegramStateChanged.emit()

    def _schedule_rescan(self) -> None:
        self._rescan_timer.start()

    @Slot()
    def _refresh_automatic(self) -> None:
        self._request_library_refresh(bool(self._setting("auto_index")))

    @Slot()
    def _refresh_manifest_only(self) -> None:
        self._request_library_refresh(False)

    @Slot(int, int, str)
    def _on_scan_progress(self, done: int, total: int, name: str) -> None:
        verifies = bool(self._setting("verify_during_index"))
        thumbnails = bool(self._setting("thumbnails_during_index"))
        if verifies and thumbnails:
            verb = "Processing"
        elif verifies:
            verb = "Checking"
        elif thumbnails:
            verb = "Creating thumbnail for"
        else:
            verb = "Adding"
        self._set_scan(
            progress=(done / total if total else 1),
            message=f"{verb} {done:,} of {total:,}: {name}",
        )

    @Slot(int, str)
    def _on_manifest_progress(self, discovered: int, name: str) -> None:
        detail = f": {name}" if name else ""
        self._set_scan(
            progress=-1.0,
            message=f"Found {discovered:,} video filenames{detail}",
        )

    @Slot(int)
    def _on_media_indexed(self, _media_id: int) -> None:
        self._library_refresh_timer.start()

    @Slot()
    def _refresh_indexed_media(self) -> None:
        self._refresh_library_state(preserve_loaded=True)

    def _stop_watcher(self, wait: bool = True) -> None:
        if self._observer:
            observer = self._observer
            self._observer = None
            observer.stop()
            if wait:
                observer.join(timeout=2)
            else:
                asyncio.create_task(
                    asyncio.to_thread(observer.join, 2)
                )

    def _start_watcher(self) -> None:
        self._stop_watcher()
        root = self._setting("library_root")
        if not root or not Path(root).is_dir():
            return
        try:
            observer = Observer()
            observer.schedule(
                _WatchHandler(
                    self._filesystemChanged.emit,
                    self._watch_event_requires_refresh,
                ),
                root,
                recursive=True,
            )
            observer.daemon = True
            observer.start()
            self._observer = observer
        except Exception as exc:
            LOGGER.warning("Could not start library watcher: %s", exc)

    def _watch_event_requires_refresh(
        self,
        event: FileSystemEvent,
    ) -> bool:
        # macOS FSEvents can report an extended-attribute-only update as
        # either modified or created when event flags are coalesced. If the
        # indexed content fingerprint is unchanged, neither event requires a
        # library refresh.
        if event.event_type not in {
            EVENT_TYPE_CREATED,
            EVENT_TYPE_MODIFIED,
        }:
            return True
        path = Path(event.src_path).expanduser().resolve()
        try:
            stat = path.stat()
        except OSError:
            return True
        indexed = self.database.get_media_by_path(path)
        if not indexed or not bool(indexed.get("valid", True)):
            return True
        return (
            int(indexed.get("size_bytes", -1)) != stat.st_size
            or abs(float(indexed.get("mtime", -1)) - stat.st_mtime) > 0.001
        )

    @Slot()
    def shutdown(self) -> None:
        self._shutting_down = True
        self._rescan_timer.stop()
        self._auto_scan_timer.stop()
        self._startup_refresh_timer.stop()
        self._library_refresh_timer.stop()
        self._watcher_timer.stop()
        self._stop_watcher()
        for task in self._thumbnail_jobs.values():
            task.cancel()
        self._thumbnail_jobs.clear()
        for task in self._preview_jobs.values():
            task.cancel()
        self._preview_jobs.clear()
        self._cancel_timeline_generation()
        if self._load_more_task:
            self._load_more_task.cancel()
            self._load_more_task = None
        if self._selection_navigation_task:
            self._selection_navigation_task.cancel()
            self._selection_navigation_task = None
        if self._random_folder_task:
            self._random_folder_task.cancel()
            self._random_folder_task = None
        if self._command_search_task:
            self._command_search_task.cancel()
            self._command_search_task = None
        for attribute in (
            "_full_refresh_task",
            "_library_model_refresh_task",
            "_history_model_refresh_task",
            "_counts_refresh_task",
            "_shuffle_reset_task",
            "_root_activation_task",
            "_library_scan_task",
        ):
            task = getattr(self, attribute, None)
            if task:
                task.cancel()
                setattr(self, attribute, None)
        self.processor.cancel()

    def _refresh_library_model_only(
        self,
        preserve_loaded: bool = False,
    ) -> None:
        if not self._can_use_model_workers():
            self.library_model.refresh(preserve_loaded=preserve_loaded)
            self._flush_navigation_focus()
            return
        if self._library_model_refresh_task:
            self._library_model_refresh_task.cancel()
        request = self.library_model.refresh_request(preserve_loaded)
        task = asyncio.create_task(
            self._refresh_library_model_async(request)
        )
        self._library_model_refresh_task = task
        task.add_done_callback(
            lambda completed, generation=request[0]:
                self._model_refresh_done(
                    "_library_model_refresh_task",
                    generation,
                    completed,
                )
        )

    async def _refresh_library_model_async(
        self,
        request: tuple[int, str, str, str, int],
    ) -> None:
        try:
            payload = await asyncio.to_thread(
                self.library_model.fetch_refresh,
                request,
            )
            if not self._shutting_down:
                if self.library_model.apply_refresh(payload):
                    self._flush_navigation_focus()
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.warning("Background library query failed: %s", exc)

    def _refresh_history_model(self) -> None:
        if not self._can_use_model_workers():
            self.history_model.refresh()
            return
        if self._history_model_refresh_task:
            self._history_model_refresh_task.cancel()
        request = self.history_model.refresh_request()
        task = asyncio.create_task(
            self._refresh_history_model_async(request)
        )
        self._history_model_refresh_task = task
        task.add_done_callback(
            lambda completed, generation=request[0]:
                self._model_refresh_done(
                    "_history_model_refresh_task",
                    generation,
                    completed,
                )
        )

    async def _refresh_history_model_async(
        self,
        request: tuple[int, str, int],
    ) -> None:
        try:
            payload = await asyncio.to_thread(
                self.history_model.fetch_refresh,
                request,
            )
            if not self._shutting_down:
                self.history_model.apply_refresh(payload)
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.warning("Background history query failed: %s", exc)

    @Slot(str, "QVariant")
    def setSetting(self, key: str, value: Any) -> None:
        try:
            if isinstance(value, QUrl):
                value = value.toLocalFile()
            previous_value = self._setting(key)
            self._store_setting(key, value)
            if key == "export_dir":
                self.processor.set_export_dir(str(value))
                self.indexer.export_dir = Path(str(value)).resolve()
            if key == "library_root":
                root = str(value)
                if str(previous_value or "") != root:
                    self._store_setting("random_folder_mode", "all")
                    self._store_setting("random_folders", [])
                    self._invalidate_random_folder_options(clear=True)
                    self._clear_navigation_history()
                active_root = (
                    root
                    if root and Path(root).is_dir()
                    else None
                )
                if self._can_use_model_workers():
                    self.database.set_active_root(active_root)
                    self._queue_root_activation(active_root)
                else:
                    self.database.activate_root(active_root)
                self._set_selected(None)
                self.library_model.folder = ""
                if not self._can_use_model_workers():
                    self.refresh_all()
                self._stop_watcher(wait=False)
                self._watcher_timer.start()
                self._auto_scan_timer.start()
                if not bool(self._setting("auto_index")):
                    self.toast.emit(
                        "success",
                        "Folder selected. Building the fast filename list now.",
                    )
            if key == "auto_index":
                if bool(value) and self._setting("library_root"):
                    self._auto_scan_timer.start()
            if key in {"performance_mode", "export_encoder"}:
                self.processor.set_encoder_mode(
                    self._effective_export_encoder()
                )
                if key == "performance_mode":
                    maximum = self._maximum_performance()
                    self._thumbnail_semaphore = asyncio.Semaphore(
                        4 if maximum else 2
                    )
                    self._preview_semaphore = asyncio.Semaphore(
                        2 if maximum else 1
                    )
                    if maximum and self._selected_id > 0:
                        self.ensureThumbnail(self._selected_id)
                        self.ensureTimeline(self._selected_id)
                        if bool(self._setting("hover_previews")):
                            self.ensurePreview(self._selected_id)
                        self._preload_selection_neighbors()
            if key == "sort_mode":
                self._cancel_load_more()
                self._pending_navigation_focus_media_id = 0
                self.library_model.sort_mode = str(value)
                self._refresh_library_model_only()
                self._queue_selection_neighbors()
            self.settingsChanged.emit()
        except Exception as exc:
            self.toast.emit("error", str(exc))

    @Slot(str)
    def setSearch(self, value: str) -> None:
        value = str(value or "")
        if value == self.library_model.search:
            return
        self._cancel_load_more()
        self._pending_navigation_focus_media_id = 0
        self.library_model.search = value
        self._refresh_library_model_only()
        self._queue_selection_neighbors()

    @Slot(str)
    def requestCommandSearch(self, value: str) -> None:
        query = value.strip()
        self._command_search_generation += 1
        generation = self._command_search_generation
        if self._command_search_task:
            self._command_search_task.cancel()
            self._command_search_task = None
        if not query:
            changed = (
                bool(self._command_search_results)
                or self._command_search_loading
            )
            self._command_search_results = []
            self._command_search_loading = False
            if changed:
                self.commandSearchChanged.emit()
            return

        self._command_search_loading = True
        self.commandSearchChanged.emit()
        if not self._can_use_model_workers():
            self._command_search_results = self.database.search_suggestions(
                query,
            )
            self._command_search_loading = False
            self.commandSearchChanged.emit()
            return

        task = asyncio.create_task(
            self._search_command_center_async(query, generation)
        )
        self._command_search_task = task

    async def _search_command_center_async(
        self,
        query: str,
        generation: int,
    ) -> None:
        try:
            results = await asyncio.to_thread(
                self.database.search_suggestions,
                query,
            )
            if (
                self._shutting_down
                or generation != self._command_search_generation
            ):
                return
            self._command_search_results = results
            self._command_search_loading = False
            self.commandSearchChanged.emit()
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            if generation != self._command_search_generation:
                return
            LOGGER.warning("Command-center search failed: %s", exc)
            self._command_search_results = []
            self._command_search_loading = False
            self.commandSearchChanged.emit()
        finally:
            if generation == self._command_search_generation:
                self._command_search_task = None

    @Slot(str)
    def setFolder(self, value: str) -> None:
        value = str(value or "")
        if value == self.library_model.folder:
            return
        self._record_navigation_origin()
        self._cancel_load_more()
        self._pending_navigation_focus_media_id = 0
        self.library_model.folder = value
        self._refresh_library_model_only()
        self._queue_selection_neighbors()

    @Slot(str, bool)
    def setRandomFolderEnabled(self, folder: str, enabled: bool) -> None:
        folders = self._selected_random_folders()
        if enabled and folder not in folders:
            folders.append(folder)
        elif not enabled and folder in folders:
            folders.remove(folder)
        all_paths = self._all_random_folder_paths()
        if all_paths and set(all_paths).issubset(folders):
            self._store_setting("random_folder_mode", "all")
            self._store_setting("random_folders", [])
        else:
            self._store_setting("random_folder_mode", "selected")
            self._store_setting("random_folders", folders)
        self.random_folder_model.set_selected(folder, enabled)
        self.settingsChanged.emit()
        self.randomFoldersChanged.emit()

    @Slot()
    def clearRandomFolders(self) -> None:
        if (
            self._random_folder_mode() == "selected"
            and not self._stored_random_folders()
        ):
            return
        self._store_setting("random_folder_mode", "selected")
        self._store_setting("random_folders", [])
        self.random_folder_model.sync_selected(set())
        self.settingsChanged.emit()
        self.randomFoldersChanged.emit()

    @Slot()
    def selectAllRandomFolders(self) -> None:
        if self._random_folder_mode() == "all":
            self.random_folder_model.sync_selected(
                set(self._all_random_folder_paths())
            )
            return
        self._store_setting("random_folder_mode", "all")
        self._store_setting("random_folders", [])
        self.random_folder_model.sync_selected(
            set(self._all_random_folder_paths())
        )
        self.settingsChanged.emit()
        self.randomFoldersChanged.emit()

    @Slot()
    def loadRandomFolderOptions(self) -> None:
        if (
            self._random_folder_options_loading
            or not self._random_folder_options_dirty
        ):
            return
        self._random_folder_options_loading = True
        generation = self._random_folder_generation
        self.randomFoldersChanged.emit()
        self._random_folder_task = asyncio.create_task(
            self._load_random_folder_options_async(generation)
        )

    async def _load_random_folder_options_async(self, generation: int) -> None:
        try:
            rows = await asyncio.to_thread(self.database.list_random_folders)
            if generation == self._random_folder_generation:
                self._random_folder_options_cache = rows
                self._random_folder_options_dirty = False
                self.random_folder_model.set_rows(
                    rows,
                    set(self._selected_random_folders()),
                )
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.warning("Could not load random folder options: %s", exc)
        finally:
            self._random_folder_options_loading = False
            self._random_folder_task = None
            self.randomFoldersChanged.emit()

    @Slot()
    def loadMoreMedia(self) -> None:
        if self._load_more_task or self._shutting_down:
            return
        request = self.library_model.page_request()
        if not request:
            return
        self._load_more_task = asyncio.create_task(
            self._load_more_media_async(request)
        )
        self._load_more_task.add_done_callback(self._load_more_done)

    def _cancel_load_more(self) -> None:
        if self._load_more_task:
            self._load_more_task.cancel()
            self._load_more_task = None

    def _load_more_done(self, task: asyncio.Task) -> None:
        if self._load_more_task is task:
            self._load_more_task = None
        if not task.cancelled() and (error := task.exception()):
            LOGGER.debug("Loading another library page failed: %s", error)

    async def _load_more_media_async(
        self,
        request: tuple[int, int, str, str, str, int],
    ) -> None:
        generation, offset, search, folder, sort_mode, page_size = request
        rows = await asyncio.to_thread(
            self.database.list_media,
            search,
            folder,
            sort_mode,
            page_size + 1,
            offset,
        )
        self.library_model.append_fetched(
            generation,
            offset,
            rows[:page_size],
            len(rows) > page_size,
        )

    @Slot(str)
    def setHistorySearch(self, value: str) -> None:
        self.history_model.search = value
        self._refresh_history_model()

    @Slot()
    def loadMoreHistory(self) -> None:
        if not self._can_use_model_workers():
            self.history_model.load_more()
            return
        if self._history_model_refresh_task:
            return
        request = self.history_model.page_request()
        if not request:
            return
        task = asyncio.create_task(
            self._load_more_history_async(request)
        )
        self._history_model_refresh_task = task
        task.add_done_callback(
            lambda completed, generation=request[0]:
                self._model_refresh_done(
                    "_history_model_refresh_task",
                    generation,
                    completed,
                )
        )

    async def _load_more_history_async(
        self,
        request: tuple[int, int, str, int],
    ) -> None:
        try:
            payload = await asyncio.to_thread(
                self.history_model.fetch_page,
                request,
            )
            if not self._shutting_down:
                self.history_model.append_fetched(payload)
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.warning("Background history page failed: %s", exc)

    def _select_media(
        self,
        media_id: int,
        *,
        record_navigation: bool,
    ) -> None:
        row = self.database.get_media(media_id)
        target_id = int(row["id"]) if row else 0
        if record_navigation and target_id != self._selected_id:
            self._record_navigation_origin()
        self._set_selected(row)
        if row and float(row.get("duration") or 0) <= 0:
            asyncio.create_task(self._verify_selection_async(media_id))
        elif row:
            self.ensureThumbnail(media_id)
            self.ensureTimeline(media_id)
            if (
                self._maximum_performance()
                and bool(self._setting("hover_previews"))
            ):
                self.ensurePreview(media_id)

    @Slot(int)
    def selectMedia(self, media_id: int) -> None:
        self._select_media(media_id, record_navigation=True)

    @Slot(int)
    def navigateSelection(self, direction: int) -> None:
        target_id = (
            self._selection_previous_id
            if int(direction) < 0
            else self._selection_next_id
        )
        if target_id <= 0:
            return
        self.selectMedia(target_id)
        media_index = self.library_model.find_index(target_id)
        if media_index >= 0:
            self.librarySelectionRequested.emit(media_index)

    @Slot()
    def navigateBack(self) -> None:
        self._navigate_history(
            self._navigation_back,
            self._navigation_forward,
        )

    @Slot()
    def navigateForward(self) -> None:
        self._navigate_history(
            self._navigation_forward,
            self._navigation_back,
        )

    def _reveal_selected_in_library(
        self,
        *,
        record_navigation: bool,
    ) -> None:
        row = self.database.get_media(self._selected_id)
        if not row:
            self.toast.emit(
                "error",
                "The selected video is no longer in the library.",
            )
            return
        folder = str(row.get("folder") or "")
        target = _NavigationState(
            folder=folder,
            search="",
            media_id=self._selected_id,
        )
        if record_navigation and target != self._navigation_state():
            self._record_navigation_origin()
        self._pending_navigation_focus_media_id = 0
        self.library_model.search = ""
        self.library_model.folder = folder
        self.library_model.refresh()
        media_index = self.library_model.find_index(self._selected_id)
        while media_index < 0 and self.library_model.has_more:
            self.library_model.load_more()
            media_index = self.library_model.find_index(self._selected_id)
        if media_index < 0:
            self.toast.emit(
                "error",
                "The selected video could not be shown in the library.",
            )
            return
        self._queue_selection_neighbors()
        self.navigationRequested.emit("library")
        self.libraryRevealRequested.emit(
            folder,
            media_index,
            self.folder_model.find_index(folder),
        )

    @Slot()
    def revealSelectedInLibrary(self) -> None:
        self._reveal_selected_in_library(
            record_navigation=True,
        )

    @Slot(int)
    def viewHistoryPost(self, post_id: int) -> None:
        post = self.database.get_post(post_id)
        if not post:
            self.toast.emit("error", "That history item is no longer available.")
            return
        media_id = int(post.get("media_id") or 0)
        row = self.database.get_media(media_id)
        if not row or not Path(str(row.get("path") or "")).is_file():
            self.toast.emit("error", "The source video for this history item is no longer available.")
            return
        current_root = str(self._setting("library_root") or "")
        if not current_root or (
            Path(current_root).expanduser().resolve()
            != Path(str(row.get("root_path") or "")).expanduser().resolve()
        ):
            self.toast.emit(
                "error",
                "Open this video's library folder before viewing it in Prepare.",
            )
            return
        if not bool(row.get("active", 1)) or not bool(row.get("valid", 1)):
            self.toast.emit("error", "This source video is not available in the current library.")
            return
        target = _NavigationState(
            folder=str(row.get("folder") or ""),
            search="",
            media_id=media_id,
        )
        if target != self._navigation_state():
            self._record_navigation_origin()
        self._select_media(media_id, record_navigation=False)
        self._reveal_selected_in_library(record_navigation=False)

    @Slot()
    def clearSelection(self) -> None:
        if self._selected_id > 0:
            self._record_navigation_origin()
        self._set_selected(None)

    @Slot()
    def pickRandom(self) -> None:
        if self._random_picking:
            return
        if not self._setting("library_root"):
            self.toast.emit("info", "Choose a library folder before picking a video.")
            return
        if not self.hasRandomFolderSelection:
            self.toast.emit("info", "Select at least one random source folder.")
            return
        asyncio.create_task(self._pick_random_cached_async())

    async def _pick_random_cached_async(self) -> None:
        if not self.hasRandomFolderSelection:
            self.toast.emit("info", "Select at least one random source folder.")
            return
        self._random_picking = True
        self.randomStateChanged.emit()
        try:
            fast = bool(self._setting("fast_random"))
            refresh_requested = False
            row = None
            while row is None:
                row = await asyncio.to_thread(
                    self.database.random_media,
                    bool(self._setting("avoid_repeats")),
                    require_checked=not fast,
                    folders=self._random_folders(),
                )
                if row:
                    break
                if self._scanning:
                    await asyncio.sleep(0.05)
                    continue
                if fast and not refresh_requested:
                    refresh_requested = True
                    self._request_library_refresh(False)
                    await asyncio.sleep(0.05)
                    continue
                break
            if not row:
                filtered = bool(self._random_folders())
                message = (
                    (
                        "No video filenames were found in the selected folders."
                        if filtered
                        else "No video filenames were found in this library."
                    )
                    if fast
                    else (
                        "No fully checked videos are available in the selected folders."
                        if filtered
                        else "No fully checked videos are available yet."
                    )
                )
                self.toast.emit("warning", message)
                return
            needs_check = float(row.get("duration") or 0) <= 0
            media_id = int(row["id"])
            if media_id != self._selected_id:
                self._record_navigation_origin()
            self._set_selected(row, checking=needs_check)
            if needs_check:
                asyncio.create_task(
                    self._verify_selection_async(
                        media_id,
                        retry_random=True,
                    )
                )
            else:
                self.ensureThumbnail(media_id)
                self.ensureTimeline(media_id)
        except Exception as exc:
            LOGGER.exception("Cached random selection failed")
            self.toast.emit("error", str(exc))
        finally:
            self._random_picking = False
            self.randomStateChanged.emit()

    async def _verify_selection_async(
        self,
        media_id: int,
        retry_random: bool = False,
    ) -> None:
        try:
            row = await asyncio.to_thread(self.indexer.ensure_metadata, media_id)
            if not row:
                if media_id == self._selected_id:
                    self._set_selected(None)
                self._refresh_library_state(preserve_loaded=True)
                if retry_random:
                    self.pickRandom()
                else:
                    self.toast.emit("warning", "That file is not a readable video.")
                return
            if media_id == self._selected_id:
                self._set_selected(row, checking=False)
            self.ensureThumbnail(media_id)
            if media_id == self._selected_id:
                self.ensureTimeline(media_id)
                if (
                    self._maximum_performance()
                    and bool(self._setting("hover_previews"))
                ):
                    self.ensurePreview(media_id)
        except Exception as exc:
            LOGGER.debug("Selected video validation failed for %s: %s", media_id, exc)
            if media_id == self._selected_id:
                self._set_selected(None)
                self.toast.emit("error", "ClipRelay could not check that video.")

    @Slot(int)
    def ensureThumbnail(self, media_id: int) -> None:
        if (
            media_id <= 0
            or self._shutting_down
            or media_id in self._thumbnail_jobs
        ):
            return
        self.library_model.set_thumbnail_state(media_id, "queued")
        task = asyncio.create_task(self._ensure_thumbnail_async(media_id))
        self._thumbnail_jobs[media_id] = task
        task.add_done_callback(
            lambda completed, selected_id=media_id: self._thumbnail_job_done(
                selected_id,
                completed,
            )
        )

    def _thumbnail_job_done(self, media_id: int, task: asyncio.Task) -> None:
        if self._thumbnail_jobs.get(media_id) is task:
            self._thumbnail_jobs.pop(media_id, None)
        if not task.cancelled() and (error := task.exception()):
            LOGGER.debug("Thumbnail task failed for %s: %s", media_id, error)

    async def _ensure_thumbnail_async(self, media_id: int) -> None:
        try:
            async with self._thumbnail_semaphore:
                if self._shutting_down:
                    return
                self.library_model.set_thumbnail_state(media_id, "generating")
                path = await asyncio.to_thread(
                    self.indexer.ensure_thumbnail,
                    media_id,
                )
            if path:
                self.library_model.update_asset(media_id, "thumbnailUrl", str(path))
                if media_id == self._selected_id:
                    row = await asyncio.to_thread(
                        self.database.get_media,
                        media_id,
                    )
                    self._set_selected(row)
            else:
                await asyncio.to_thread(
                    self.database.clear_media_asset,
                    media_id,
                    "thumbnail_path",
                )
                self.library_model.clear_thumbnail(media_id, "failed")
                if media_id == self._selected_id:
                    row = await asyncio.to_thread(
                        self.database.get_media,
                        media_id,
                    )
                    self._set_selected(row)
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.debug("Thumbnail generation failed for %s: %s", media_id, exc)
            await asyncio.to_thread(
                self.database.clear_media_asset,
                media_id,
                "thumbnail_path",
            )
            self.library_model.clear_thumbnail(media_id, "failed")

    def _cancel_timeline_generation(self) -> None:
        self._timeline_generation += 1
        if self._timeline_task:
            self._timeline_task.cancel()
            self._timeline_task = None
        self._timeline_media_id = 0
        if self._timeline_loading:
            self._timeline_loading = False
            self.timelineStateChanged.emit()

    @Slot(int)
    def ensureTimeline(self, media_id: int) -> None:
        if media_id <= 0 or self._shutting_down:
            return
        media = self.database.get_media(media_id)
        existing = (media or {}).get("timeline_path")
        if existing and Path(str(existing)).is_file():
            if media_id == self._selected_id:
                self._set_selected(media)
            return
        if (
            self._timeline_task
            and self._timeline_media_id == media_id
        ):
            return
        self._cancel_timeline_generation()
        self._timeline_generation += 1
        generation = self._timeline_generation
        self._timeline_media_id = media_id
        self._timeline_loading = True
        self.timelineStateChanged.emit()
        task = asyncio.create_task(
            self._ensure_timeline_async(media_id, generation)
        )
        self._timeline_task = task
        task.add_done_callback(
            lambda completed, selected_id=media_id, request=generation:
                self._timeline_job_done(
                    selected_id,
                    request,
                    completed,
                )
        )

    def _timeline_job_done(
        self,
        media_id: int,
        generation: int,
        task: asyncio.Task,
    ) -> None:
        if self._timeline_task is task:
            self._timeline_task = None
            self._timeline_media_id = 0
            self._timeline_loading = False
            self.timelineStateChanged.emit()
        if not task.cancelled() and (error := task.exception()):
            LOGGER.debug(
                "Timeline generation failed for %s (%s): %s",
                media_id,
                generation,
                error,
            )

    async def _ensure_timeline_async(
        self,
        media_id: int,
        generation: int,
    ) -> None:
        try:
            await asyncio.sleep(0.16)
            async with self._timeline_semaphore:
                if (
                    self._shutting_down
                    or generation != self._timeline_generation
                ):
                    return
                path = await asyncio.to_thread(
                    self.indexer.ensure_timeline,
                    media_id,
                )
            if (
                path
                and generation == self._timeline_generation
                and media_id == self._selected_id
            ):
                row = await asyncio.to_thread(
                    self.database.get_media,
                    media_id,
                )
                self._set_selected(row)
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            LOGGER.debug(
                "Timeline generation failed for %s: %s",
                media_id,
                exc,
            )

    @Slot()
    def resetShuffle(self) -> None:
        if not self.hasRandomFolderSelection:
            self.toast.emit("info", "Select at least one random source folder.")
            return
        if self._can_use_model_workers():
            if self._shuffle_reset_task:
                return
            task = asyncio.create_task(self._reset_shuffle_async())
            self._shuffle_reset_task = task
            task.add_done_callback(
                lambda completed: self._simple_task_done(
                    "_shuffle_reset_task",
                    completed,
                )
            )
            return
        self._reset_shuffle_now()

    def _reset_shuffle_now(self) -> None:
        self.database.reset_shuffle(
            self._setting("library_root") or None,
            folders=self._random_folders(),
        )
        self._finish_shuffle_reset()

    async def _reset_shuffle_async(self) -> None:
        root = self._setting("library_root") or None
        folders = list(self._random_folders())
        await asyncio.to_thread(
            self.database.reset_shuffle,
            root,
            folders=folders,
        )
        if not self._shutting_down:
            self._finish_shuffle_reset()

    def _finish_shuffle_reset(self) -> None:
        self._refresh_counts()
        message = (
            "Random history was reset for the selected folders."
            if self._random_folders()
            else "Random selection history was reset."
        )
        self.toast.emit("success", message)

    @Slot()
    def scanLibrary(self) -> None:
        self._request_library_refresh(True)

    def _request_library_refresh(self, include_index: bool) -> None:
        root = self._setting("library_root")
        if not root:
            if include_index:
                self.toast.emit("info", "Choose a library folder to begin indexing videos.")
            return
        if self._scanning:
            self._pending_refresh = True
            self._pending_index = self._pending_index or include_index
            return
        task = asyncio.create_task(
            self._refresh_library_async(str(root), include_index)
        )
        self._library_scan_task = task
        task.add_done_callback(
            lambda completed: self._simple_task_done(
                "_library_scan_task",
                completed,
            )
        )

    async def _refresh_library_async(self, root: str, include_index: bool) -> None:
        self._set_scan(
            active=True,
            progress=-1.0,
            message="Scanning folders for video filenames",
        )

        last_progress_emit = 0.0
        last_item_emit = 0.0

        def progress(done: int, total: int, name: str) -> None:
            nonlocal last_progress_emit
            now = time.monotonic()
            if done >= total or now - last_progress_emit >= 0.12:
                last_progress_emit = now
                self._scanProgress.emit(done, total, name)

        def item_ready(media_id: int) -> None:
            nonlocal last_item_emit
            now = time.monotonic()
            if now - last_item_emit >= 0.35:
                last_item_emit = now
                self._mediaIndexed.emit(media_id)

        try:
            await asyncio.to_thread(
                self.indexer.refresh_manifest,
                root,
                self._mediaIndexed.emit,
                progress=self._manifestProgress.emit,
                check_changes=bool(
                    include_index
                    and self._setting("verify_during_index")
                ),
            )
            self._refresh_library_state(preserve_loaded=True)
            current_root = self._setting("library_root")
            if not current_root or (
                Path(str(current_root)).expanduser().resolve()
                != Path(root).expanduser().resolve()
            ):
                return
            if self._pending_refresh:
                include_index = include_index or self._pending_index
                self._pending_refresh = False
                self._pending_index = False
            if not include_index:
                self._set_scan(
                    progress=1.0,
                    message=f"{self.database.counts()['media']} filenames ready",
                )
                return
            verify_media = bool(self._setting("verify_during_index"))
            deep_scan = bool(self._setting("deep_scan")) and verify_media
            candidates = None if deep_scan else self.database.manifest_paths(root)
            self._set_scan(progress=0.0, message="Checking library details")
            result = await asyncio.to_thread(
                self.indexer.scan,
                root,
                deep_scan,
                progress,
                verify_media=verify_media,
                generate_thumbnails=bool(self._setting("thumbnails_during_index")),
                item_ready=item_ready,
                candidates=candidates,
            )
            self._refresh_library_state(preserve_loaded=True)
            self._set_scan(progress=1.0, message=f"{self.database.counts()['media']} videos ready")
            if result.failed:
                self.toast.emit("warning", f"Indexed {result.indexed} videos; skipped {result.failed} unreadable files.")
        except Exception as exc:
            LOGGER.exception("Library scan failed")
            self.toast.emit("error", str(exc))
            self._set_scan(message="Scan failed")
        finally:
            self._set_scan(active=False)
            if self._shutting_down:
                self._pending_refresh = False
                self._pending_index = False
                return
            current_root = self._setting("library_root")
            root_changed = bool(current_root) and (
                Path(str(current_root)).expanduser().resolve()
                != Path(root).expanduser().resolve()
            )
            if root_changed:
                self._pending_refresh = False
                self._pending_index = False
                self._auto_scan_timer.start()
            elif self._pending_refresh:
                include_pending = self._pending_index
                self._pending_refresh = False
                self._pending_index = False
                QTimer.singleShot(
                    0,
                    lambda: self._request_library_refresh(include_pending),
                )

    @Slot(int)
    def ensurePreview(self, media_id: int) -> None:
        if (
            not bool(self._setting("hover_previews"))
            or media_id <= 0
            or media_id in self._preview_jobs
            or self._shutting_down
        ):
            return
        task = asyncio.create_task(self._ensure_preview_async(media_id))
        self._preview_jobs[media_id] = task
        task.add_done_callback(
            lambda completed, preview_id=media_id: self._preview_job_done(
                preview_id,
                completed,
            )
        )

    def _preview_job_done(self, media_id: int, task: asyncio.Task) -> None:
        if self._preview_jobs.get(media_id) is task:
            self._preview_jobs.pop(media_id, None)
        if not task.cancelled() and (error := task.exception()):
            LOGGER.debug("Preview task failed for %s: %s", media_id, error)

    async def _ensure_preview_async(self, media_id: int) -> None:
        try:
            async with self._preview_semaphore:
                if self._shutting_down:
                    return
                path = await asyncio.to_thread(
                    self.indexer.ensure_preview,
                    media_id,
                )
            if path:
                self.library_model.update_asset(media_id, "previewUrl", str(path))
                if media_id == self._selected_id:
                    row = await asyncio.to_thread(
                        self.database.get_media,
                        media_id,
                    )
                    self._set_selected(row)
        except Exception as exc:
            LOGGER.debug("Preview generation failed for %s: %s", media_id, exc)

    @Slot(float, float, str, float, result=str)
    def estimateOutputSize(self, trim_start: float, trim_end: float, preset: str, target_mb: float) -> str:
        if not self._selected:
            return ""
        duration = max(0.05, (trim_end or self._selected["duration"]) - trim_start)
        source_ratio = duration / max(0.05, self._selected["duration"])
        preset_limit = {
            "fit_bot": 49.0,
            "fit_x": float(self._setting("x_limit_mb") or 512) * 0.98,
            "fit_both": min(
                49.0,
                float(self._setting("x_limit_mb") or 512) * 0.98,
            ),
        }.get(preset)
        if preset_limit:
            return f"up to {preset_limit:.0f} MB"
        if preset == "custom" and target_mb > 0:
            return f"up to {target_mb:.0f} MB"
        if preset == "smallest":
            estimate = min(self._selected["sizeBytes"] * source_ratio * 0.28, duration * 750_000 / 8)
        elif preset == "balanced":
            estimate = min(self._selected["sizeBytes"] * source_ratio * 0.62, duration * 2_500_000 / 8)
        else:
            estimate = self._selected["sizeBytes"] * source_ratio
        return format_bytes(int(estimate))

    def _target_for(self, payload: dict[str, Any]) -> float:
        preset = payload.get("preset", "balanced")
        if preset == "fit_bot":
            return 49.0
        if preset == "fit_x":
            return float(self._setting("x_limit_mb")) * 0.98
        if preset == "fit_both":
            limits = []
            if payload.get("telegramEnabled"):
                limits.append(49.0 if payload.get("telegramMode") == "bot" else 1950.0)
            if payload.get("xEnabled"):
                limits.append(float(self._setting("x_limit_mb")) * 0.98)
            return min(limits) if limits else float(payload.get("targetMb") or 0)
        return float(payload.get("targetMb") or 0)

    @Slot("QVariantMap")
    def publish(self, payload: dict[str, Any]) -> None:
        if self._publish_state.get("active"):
            return
        asyncio.create_task(self._publish_async(dict(payload)))

    async def _publish_async(self, payload: dict[str, Any]) -> None:
        media_id = int(payload.get("mediaId") or self._selected_id)
        media = self.database.get_media(media_id)
        if not media:
            self.toast.emit("error", "Choose a video before preparing a post.")
            return
        if media_id == self._selected_id and self._selected_checking:
            self.toast.emit("info", "Wait for the selected video check to finish.")
            return
        telegram_enabled = bool(payload.get("telegramEnabled"))
        x_enabled = bool(payload.get("xEnabled"))
        if not telegram_enabled and not x_enabled:
            self.toast.emit("warning", "Choose Telegram, X, or both.")
            return
        if telegram_enabled and not str(payload.get("telegramDestination", "")).strip():
            self.toast.emit("warning", "Choose a Telegram destination.")
            return
        if (
            telegram_enabled
            and str(payload.get("telegramMode", "bot")) == "bot"
            and not self._bot_configured
            and isinstance(self.bot, TelegramBotService)
        ):
            self.toast.emit("warning", "Connect a Telegram bot in Settings before sending.")
            return
        if (
            telegram_enabled
            and str(payload.get("telegramMode", "bot")) == "personal"
            and not self._personal_configured
        ):
            self.toast.emit("warning", "Sign in to your personal Telegram account in Settings before sending.")
            return
        if float(media.get("duration") or 0) <= 0:
            media = await asyncio.to_thread(self.indexer.ensure_metadata, media_id)
            if not media:
                self._refresh_library_state(preserve_loaded=True)
                if media_id == self._selected_id:
                    self._set_selected(None)
                self.toast.emit("error", "The selected file is not a readable video.")
                return
            if media_id == self._selected_id:
                self._set_selected(media)
        if telegram_enabled:
            self._store_setting("telegram_mode", str(payload.get("telegramMode", "bot")))
            self._store_setting(
                "telegram_destination",
                str(payload.get("telegramDestination", "")).strip(),
            )
            self.settingsChanged.emit()
        post_id = self.database.create_post({
            "media_id": media_id,
            "telegram_enabled": telegram_enabled,
            "x_enabled": x_enabled,
            "telegram_caption": str(payload.get("telegramCaption", "")),
            "x_caption": str(payload.get("xCaption", "")),
            "telegram_mode": str(payload.get("telegramMode", "bot")),
            "telegram_destination": str(payload.get("telegramDestination", "")),
            "cleanup_policy": str(payload.get("cleanupPolicy", self._setting("cleanup_policy"))),
        })
        self._set_publish(
            active=True,
            progress=0.0,
            stage="Preparing video",
            postId=post_id,
            error="",
            outputEncoder="",
            hardwareAccelerated=False,
        )

        def progress(value: float, stage: str) -> None:
            self._set_publish(progress=value * 0.65, stage=stage)

        export_id = None
        result = None
        try:
            preset = str(payload.get("preset", "balanced"))
            target_mb = self._target_for(payload)
            edit_spec = normalize_edit_spec(payload.get("edits"))
            result = await self.processor.export(
                media,
                float(payload.get("trimStart") or 0),
                float(payload.get("trimEnd") or media["duration"]),
                preset,
                target_mb,
                progress=progress,
                edits=edit_spec,
            )
            export_id = self.database.create_export({
                "media_id": media_id, "path": str(result.path),
                "trim_start": float(payload.get("trimStart") or 0),
                "trim_end": float(payload.get("trimEnd") or media["duration"]),
                "preset": result.preset, "target_mb": target_mb,
                "size_bytes": result.size_bytes, "duration": result.duration,
                "is_generated": result.generated,
                "edit_spec": edit_spec,
            })
            self.database.update_post(post_id, export_id=export_id)
            self._set_publish(
                outputPath=str(result.path),
                outputSize=format_bytes(result.size_bytes),
                outputEncoder=result.encoder,
                hardwareAccelerated=result.hardware_accelerated,
            )

            delivery_errors: list[str] = []
            completed_any = False
            if telegram_enabled:
                self._set_publish(progress=0.68, stage="Sending to Telegram")
                self.database.add_attempt(post_id, "telegram", "started", finished=False)

                def tg_progress(value: float, stage: str) -> None:
                    self._set_publish(progress=0.68 + value * 0.27, stage=stage)

                try:
                    mode = str(payload.get("telegramMode", "bot"))
                    if mode == "personal":
                        api_id = int(self._setting("telegram_api_id") or 0)
                        delivery = await self.personal.send_video(
                            api_id, str(payload["telegramDestination"]), result.path,
                            str(payload.get("telegramCaption", "")), tg_progress,
                        )
                    else:
                        delivery = await self.bot.send_video(
                            result.path, str(payload.get("telegramCaption", "")),
                            str(payload["telegramDestination"]), tg_progress,
                        )
                    self.database.update_post(
                        post_id, telegram_status="sent", telegram_message_id=delivery.message_id,
                        telegram_message_link=delivery.link,
                    )
                    self.database.add_attempt(post_id, "telegram", "sent", delivery.detail, delivery.message_id)
                    completed_any = True
                except Exception as exc:
                    LOGGER.warning("Telegram delivery failed for post %s: %s", post_id, exc)
                    self.database.update_post(post_id, telegram_status="failed")
                    self.database.add_attempt(post_id, "telegram", "failed", str(exc))
                    delivery_errors.append(f"Telegram: {exc}")

            if x_enabled:
                self._set_publish(progress=0.96, stage="Opening X composer")
                try:
                    self.x_assistant.prepare(result.path, str(payload.get("xCaption", "")))
                    self.database.update_post(post_id, x_status="prepared")
                    self.database.add_attempt(post_id, "x", "prepared", "Caption prefilled and video copied")
                    completed_any = True
                except Exception as exc:
                    LOGGER.warning("X handoff failed for post %s: %s", post_id, exc)
                    self.database.update_post(post_id, x_status="failed")
                    self.database.add_attempt(post_id, "x", "failed", str(exc))
                    delivery_errors.append(f"X: {exc}")

            if completed_any:
                self.database.increment_posted(media_id)
            if delivery_errors:
                detail = " · ".join(delivery_errors)
                self.database.update_post(post_id, error=detail)
                self._set_publish(progress=1.0, stage="Completed with an issue", error=detail)
                self.toast.emit("warning", "One destination needs attention; the other completed steps were kept.")
            else:
                self._set_publish(progress=1.0, stage="Post prepared")
                self.toast.emit("success", "Telegram and X steps are ready." if telegram_enabled and x_enabled else "Post step completed.")
            await self._apply_cleanup(post_id)
        except ProcessingCancelled as exc:
            self.database.update_post(post_id, telegram_status="cancelled" if telegram_enabled else "not_requested", x_status="cancelled" if x_enabled else "not_requested", error=str(exc))
            self._set_publish(error=str(exc), stage="Cancelled")
            self.toast.emit("info", str(exc))
        except Exception as exc:
            LOGGER.exception("Publish workflow failed")
            post = self.database.get_post(post_id) or {}
            if telegram_enabled and post.get("telegram_status") not in {"sent"}:
                self.database.update_post(post_id, telegram_status="failed")
                self.database.add_attempt(post_id, "telegram", "failed", str(exc))
            if x_enabled and post.get("x_status") not in {"prepared", "posted"}:
                self.database.update_post(post_id, x_status="failed")
            self.database.update_post(post_id, error=str(exc))
            self._set_publish(error=str(exc), stage="Could not complete")
            self.toast.emit("error", str(exc))
        finally:
            self._refresh_history_model()
            self._refresh_library_model_only(preserve_loaded=True)
            self._refresh_counts()
            await asyncio.sleep(0.25)
            self._set_publish(active=False)

    @Slot()
    def cancelPublish(self) -> None:
        self.processor.cancel()

    async def _apply_cleanup(self, post_id: int) -> None:
        post = self.database.get_post(post_id)
        if not post or not post.get("export_id") or not post.get("is_generated"):
            return
        policy = post.get("cleanup_policy")
        telegram_done = post.get("telegram_status") in {"sent", "not_requested"}
        x_done = post.get("x_status") in {"posted", "not_requested"}
        should_trash = policy == "after_complete" and telegram_done and x_done
        if policy == "after_telegram" and post.get("x_status") == "not_requested" and telegram_done:
            should_trash = True
        if should_trash:
            move_generated_to_trash(post["export_path"], self._setting("export_dir"), True)
            self.database.mark_export_cleanup(int(post["export_id"]), "trashed")

    @Slot(int, str)
    def markXPosted(self, post_id: int, url: str = "") -> None:
        self.database.update_post(post_id, x_status="posted", x_url=url.strip())
        self.database.add_attempt(post_id, "x", "posted", "Confirmed by user", url.strip())
        asyncio.create_task(self._apply_cleanup(post_id))
        self._refresh_history_model()
        self.toast.emit("success", "X post marked as posted.")

    @Slot(int)
    def prepareXAgain(self, post_id: int) -> None:
        post = self.database.get_post(post_id)
        if not post:
            return
        path = post.get("export_path") or post.get("source_path")
        if path and Path(path).is_file():
            self.x_assistant.prepare(path, post.get("x_caption", ""))
            self.database.update_post(post_id, x_status="prepared")
            self._refresh_history_model()
        else:
            self.toast.emit("error", "The video used for this post is no longer available.")

    @Slot(int)
    def retryTelegram(self, post_id: int) -> None:
        asyncio.create_task(self._retry_telegram_async(post_id))

    async def _retry_telegram_async(self, post_id: int) -> None:
        post = self.database.get_post(post_id)
        if not post:
            return
        path = post.get("export_path") or post.get("source_path")
        if not path or not Path(path).is_file():
            self.toast.emit("error", "The video used for this post is no longer available.")
            return
        self._set_publish(active=True, progress=0.0, stage="Retrying Telegram", postId=post_id, error="")
        try:
            if post["telegram_mode"] == "personal":
                delivery = await self.personal.send_video(
                    int(self._setting("telegram_api_id") or 0),
                    post["telegram_destination"], path, post["telegram_caption"],
                    lambda p, s: self._set_publish(progress=p, stage=s),
                )
            else:
                delivery = await self.bot.send_video(
                    path, post["telegram_caption"], post["telegram_destination"],
                    lambda p, s: self._set_publish(progress=p, stage=s),
                )
            self.database.update_post(post_id, telegram_status="sent", telegram_message_id=delivery.message_id, telegram_message_link=delivery.link, error="")
            self.database.add_attempt(post_id, "telegram", "sent", "Retry succeeded", delivery.message_id)
            self.toast.emit("success", "Telegram retry succeeded.")
            await self._apply_cleanup(post_id)
        except Exception as exc:
            self.database.update_post(post_id, telegram_status="failed", error=str(exc))
            self.database.add_attempt(post_id, "telegram", "failed", str(exc))
            self.toast.emit("error", str(exc))
        finally:
            self._refresh_history_model()
            self._set_publish(active=False)

    @Slot(int)
    def trashExport(self, post_id: int) -> None:
        post = self.database.get_post(post_id)
        if not post or not post.get("export_id"):
            return
        try:
            move_generated_to_trash(
                post["export_path"], self._setting("export_dir"), bool(post["is_generated"])
            )
            self.database.mark_export_cleanup(int(post["export_id"]), "trashed")
            self._refresh_history_model()
            self.toast.emit("success", "Generated video moved to Trash.")
        except CleanupError as exc:
            self.toast.emit("error", str(exc))

    @Slot(str)
    def revealPath(self, path: str) -> None:
        if path and Path(path).exists():
            self.x_assistant.reveal(path)
        else:
            self.toast.emit("error", "That file is no longer available.")

    @Slot()
    def openSelectedVideo(self) -> None:
        path = str(self._selected.get("path") or "")
        if not path or not Path(path).is_file():
            self.toast.emit("error", "The selected video is no longer available.")
            return
        if not QDesktopServices.openUrl(QUrl.fromLocalFile(path)):
            self.toast.emit("error", "The default video player could not open this file.")

    @Slot(str)
    def openUrl(self, url: str) -> None:
        if url:
            QDesktopServices.openUrl(QUrl(url))

    @Slot(str)
    def copyVideoFile(self, path: str) -> None:
        if path and Path(path).is_file():
            self.x_assistant.copy_file(path)
            self.toast.emit("success", "Video copied. Paste it into the X composer.")

    @Slot(str)
    def startFileDrag(self, path: str) -> None:
        if path and Path(path).is_file():
            self.x_assistant.start_drag(self, path)

    @Slot(str)
    def validateBotToken(self, token: str) -> None:
        asyncio.create_task(self._validate_bot_async(token.strip()))

    async def _validate_bot_async(self, token: str) -> None:
        self._set_telegram(message="Checking bot token")
        try:
            bot = await self.bot.validate(token)
            self._bot_configured = True
            self._store_setting("telegram_bot_configured", True)
            self._set_telegram(bot=f"@{bot.get('username', 'bot')}", message="Bot connected")
            self.settingsChanged.emit()
            self.toast.emit("success", "Telegram bot connected.")
        except Exception as exc:
            self._set_telegram(message=str(exc))
            self.toast.emit("error", str(exc))

    @Slot(str)
    def validateBotDestination(self, destination: str) -> None:
        asyncio.create_task(self._validate_destination_async(destination.strip()))

    @Slot()
    def disconnectBot(self) -> None:
        asyncio.create_task(self._disconnect_bot_async())

    async def _disconnect_bot_async(self) -> None:
        await asyncio.to_thread(
            self.secrets.delete,
            "telegram_bot_token",
        )
        self._bot_configured = False
        self._store_setting("telegram_bot_configured", False)
        self._set_telegram(bot="not configured", message="Bot token removed")
        self.settingsChanged.emit()
        self.toast.emit("success", "Telegram bot disconnected.")

    async def _validate_destination_async(self, destination: str) -> None:
        try:
            chat = await self.bot.validate_destination(destination)
            self._store_setting("telegram_destination", destination)
            self.settingsChanged.emit()
            self.toast.emit("success", f"Telegram destination ready: {chat.get('title') or destination}")
        except Exception as exc:
            self.toast.emit("error", str(exc))

    @Slot(str, str, str)
    def beginPersonalLogin(self, api_id: str, api_hash: str, phone: str) -> None:
        asyncio.create_task(self._begin_personal_async(api_id, api_hash, phone))

    async def _begin_personal_async(self, api_id: str, api_hash: str, phone: str) -> None:
        try:
            numeric_id = int(api_id)
            self._set_telegram(message="Requesting Telegram sign-in code", passwordRequired=False)
            await self.personal.begin_login(numeric_id, api_hash.strip(), phone.strip())
            self._store_setting("telegram_api_id", str(numeric_id))
            self._store_setting("telegram_phone", phone.strip())
            self.settingsChanged.emit()
            self._set_telegram(message="Enter the code Telegram sent")
        except Exception as exc:
            self._set_telegram(message=str(exc))
            self.toast.emit("error", str(exc))

    @Slot(str, str)
    def completePersonalLogin(self, code: str, password: str) -> None:
        asyncio.create_task(self._complete_personal_async(code.strip(), password))

    async def _complete_personal_async(self, code: str, password: str) -> None:
        try:
            display = await self.personal.complete_login(code, password)
            self._personal_configured = True
            self._store_setting("telegram_personal_configured", True)
            self._set_telegram(personal=display, message="Personal account connected", passwordRequired=False)
            self.settingsChanged.emit()
            self.toast.emit("success", f"Signed in as {display}.")
            await self._load_dialogs_async()
        except TelegramPasswordRequired as exc:
            self._set_telegram(message=str(exc), passwordRequired=True)
        except Exception as exc:
            self._set_telegram(message=str(exc))
            self.toast.emit("error", str(exc))

    @Slot()
    def loadTelegramDialogs(self) -> None:
        asyncio.create_task(self._load_dialogs_async())

    async def _load_dialogs_async(self) -> None:
        try:
            self._telegram_dialogs = await self.personal.dialogs(
                int(self._setting("telegram_api_id") or 0)
            )
            self.telegramDialogsChanged.emit()
        except Exception as exc:
            self.toast.emit("error", str(exc))

    @Slot()
    def signOutPersonal(self) -> None:
        asyncio.create_task(self._sign_out_personal_async())

    async def _sign_out_personal_async(self) -> None:
        await self.personal.sign_out(int(self._setting("telegram_api_id") or 0))
        self._personal_configured = False
        self._store_setting("telegram_personal_configured", False)
        self._telegram_dialogs = []
        self.telegramDialogsChanged.emit()
        self._set_telegram(personal="not signed in", message="Personal Telegram session removed")
        self.settingsChanged.emit()

    @Slot(result="QVariantMap")
    def diagnostics(self) -> dict[str, Any]:
        return {
            "ffmpeg": str(ffmpeg_path() or "Not found"),
            "ffprobe": str(ffprobe_path() or "Not found"),
            "database": str(self.database.path),
            "secretBackend": self.secrets.backend,
            "libraryInsideExports": bool(
                self._setting("library_root")
                and is_within(self._setting("library_root"), self._setting("export_dir"))
            ),
        }
