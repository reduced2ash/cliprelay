from __future__ import annotations

import asyncio
import threading
import time
from pathlib import Path

import pytest
from watchdog.events import (
    FileClosedEvent,
    FileClosedNoWriteEvent,
    FileCreatedEvent,
    FileDeletedEvent,
    FileModifiedEvent,
    FileMovedEvent,
    FileOpenedEvent,
)

from cliprelay.controller import AppController, _WatchHandler
from cliprelay.database import Database
from cliprelay.media import ExportResult
from cliprelay.qt_models import FolderModel, HistoryModel, LibraryModel
from cliprelay.settings import Settings
from cliprelay.telegram import TelegramBotService, TelegramError


class MemorySecrets:
    backend = "memory"

    def __init__(self) -> None:
        self.values: dict[str, str] = {}
        self.get_calls = 0

    def get(self, key: str, default: str = "") -> str:
        self.get_calls += 1
        return self.values.get(key, default)

    def set(self, key: str, value: str) -> None:
        self.values[key] = value

    def delete(self, key: str) -> None:
        self.values.pop(key, None)


class Response:
    is_success = True

    def __init__(self, payload: dict) -> None:
        self.payload = payload

    def json(self) -> dict:
        return self.payload


class FakeHttpClient:
    calls: list[tuple[str, str]] = []

    def __init__(self, *args, **kwargs) -> None:
        pass

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        return False

    async def get(self, url: str):
        self.calls.append(("GET", url))
        return Response({"ok": True, "result": {"username": "relay_bot"}})

    async def post(self, url: str, data=None, files=None):
        self.calls.append(("POST", url))
        if url.endswith("/getChat"):
            return Response({"ok": True, "result": {"title": "Test channel"}})
        return Response({
            "ok": True,
            "result": {"message_id": 81, "chat": {"username": "test_channel"}},
        })


def test_watch_handler_only_emits_for_mutating_file_events(
    tmp_path: Path,
) -> None:
    source = tmp_path / "video.mp4"
    source.write_bytes(b"video")

    for passive_event in (
        FileOpenedEvent(str(source)),
        FileClosedEvent(str(source)),
        FileClosedNoWriteEvent(str(source)),
    ):
        emitted: list[bool] = []
        _WatchHandler(lambda: emitted.append(True)).on_any_event(
            passive_event
        )
        assert emitted == []

    for mutation_event in (
        FileCreatedEvent(str(source)),
        FileDeletedEvent(str(source)),
        FileModifiedEvent(str(source)),
        FileMovedEvent(str(source), str(tmp_path / "moved.mp4")),
    ):
        emitted = []
        _WatchHandler(lambda: emitted.append(True)).on_any_event(
            mutation_event
        )
        assert emitted == [True]


def test_watch_handler_ignores_metadata_only_modifications(
    tmp_path: Path,
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    library_root.mkdir()
    source = library_root / "video.mp4"
    source.write_bytes(b"video")
    database.upsert_media({
        "root_path": str(library_root),
        "path": str(source),
        "name": source.name,
        "relative_path": source.name,
        "folder": "",
        "size_bytes": source.stat().st_size,
        "mtime": source.stat().st_mtime,
    })
    settings = Settings(database)
    settings.set("library_root", str(library_root))
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        LibraryModel(database),
        FolderModel(database),
        HistoryModel(database),
    )
    emitted: list[bool] = []
    handler = _WatchHandler(
        lambda: emitted.append(True),
        controller._watch_event_requires_refresh,
    )

    handler.on_any_event(FileModifiedEvent(str(source)))
    assert emitted == []
    handler.on_any_event(FileCreatedEvent(str(source)))
    assert emitted == []

    source.write_bytes(b"substantive video update")
    handler.on_any_event(FileModifiedEvent(str(source)))
    assert emitted == [True]
    controller.shutdown()


def test_command_center_loads_overview_search_and_clear_states(
    tmp_path: Path,
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    nested = library_root / "Reference"
    nested.mkdir(parents=True)
    source = nested / "featured-clip.mp4"
    source.write_bytes(b"video")
    database.upsert_media({
        "root_path": str(library_root),
        "path": str(source),
        "name": source.name,
        "relative_path": "Reference/featured-clip.mp4",
        "folder": "Reference",
        "size_bytes": source.stat().st_size,
        "mtime": source.stat().st_mtime,
    })
    settings = Settings(database)
    settings.set("library_root", str(library_root))
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        LibraryModel(database),
        FolderModel(database),
        HistoryModel(database),
    )

    controller.requestCommandOverview()
    assert [row["kind"] for row in controller.commandSearchResults] == [
        "media",
        "folder",
    ]
    assert controller.commandSearchLoading is False

    controller.requestCommandSearch("featured")
    assert controller.commandSearchResults[0]["title"] == source.name

    controller.requestCommandSearch("Reference", "folders")
    assert [row["kind"] for row in controller.commandSearchResults] == [
        "folder",
    ]
    assert controller.commandSearchResults[0]["folderPath"] == "Reference"

    controller.requestCommandOverview("videos")
    assert controller.commandSearchResults
    assert all(
        row["kind"] == "media"
        for row in controller.commandSearchResults
    )

    controller.requestCommandSearch("")
    assert controller.commandSearchResults == []
    controller.shutdown()


@pytest.mark.asyncio
async def test_bot_validation_destination_and_send(monkeypatch, tmp_path: Path) -> None:
    from cliprelay import telegram as telegram_module

    monkeypatch.setattr(telegram_module.httpx, "AsyncClient", FakeHttpClient)
    secrets = MemorySecrets()
    service = TelegramBotService(secrets)
    bot = await service.validate("123:token")
    assert bot["username"] == "relay_bot"
    assert secrets.get("telegram_bot_token") == "123:token"
    assert (await service.validate_destination("@test"))["title"] == "Test channel"

    video = tmp_path / "prepared.mp4"
    video.write_bytes(b"video")
    progress: list[float] = []
    delivery = await service.send_video(video, "caption", "@test", lambda p, _: progress.append(p))
    assert delivery.message_id == "81"
    assert delivery.link == "https://t.me/test_channel/81"
    assert progress == [0.05, 1.0]


@pytest.mark.asyncio
async def test_bot_rejects_oversized_file_before_network(tmp_path: Path) -> None:
    secrets = MemorySecrets()
    secrets.set("telegram_bot_token", "token")
    service = TelegramBotService(secrets)
    video = tmp_path / "large.mp4"
    with video.open("wb") as stream:
        stream.truncate(50 * 1024 * 1024 + 1)
    with pytest.raises(TelegramError, match="exceeds"):
        await service.send_video(video, "", "@test")


class FailingBot:
    token = "token"

    async def send_video(self, *args, **kwargs):
        raise TelegramError("Bot is not allowed to post there")


class PreparedX:
    def __init__(self) -> None:
        self.calls: list[tuple[Path, str]] = []

    def prepare(self, path: Path, caption: str) -> None:
        self.calls.append((Path(path), caption))


class PreparedProcessor:
    def __init__(self, output: Path) -> None:
        self.output = output
        self.edits = None

    async def export(self, media, start, end, preset, target, progress=None, edits=None):
        self.edits = edits
        progress(1.0, "Export ready")
        return ExportResult(self.output, self.output.stat().st_size, end - start, True, preset)

    def cancel(self) -> None:
        pass


@pytest.mark.asyncio
async def test_both_destinations_keep_x_handoff_when_telegram_fails(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    source = tmp_path / "source.mp4"
    source.write_bytes(b"source")
    media_id = database.upsert_media({
        "root_path": str(tmp_path), "path": str(source), "name": source.name,
        "relative_path": source.name, "folder": "", "duration": 5,
        "width": 640, "height": 360, "size_bytes": source.stat().st_size,
        "video_codec": "h264", "audio_codec": "aac", "frame_rate": 30, "mtime": 1,
    })
    output = tmp_path / "prepared.mp4"
    output.write_bytes(b"prepared")
    settings = Settings(database)
    settings.set("export_dir", str(tmp_path))
    secrets = MemorySecrets()
    library = LibraryModel(database)
    controller = AppController(
        database, settings, secrets, library, FolderModel(database), HistoryModel(database)
    )
    controller.bot = FailingBot()
    controller.processor = PreparedProcessor(output)
    prepared_x = PreparedX()
    controller.x_assistant = prepared_x

    await controller._publish_async({
        "mediaId": media_id,
        "trimStart": 0,
        "trimEnd": 5,
        "preset": "balanced",
        "telegramEnabled": True,
        "xEnabled": True,
        "telegramMode": "bot",
        "telegramDestination": "@test",
        "telegramCaption": "telegram caption",
        "xCaption": "x caption",
        "edits": {
            "crop": {"enabled": True, "x": 0.1, "y": 0.1, "width": 0.8, "height": 0.8},
            "overlays": [{"x": 0.2, "y": 0.2, "width": 0.25, "height": 0.1}],
        },
        "cleanupPolicy": "keep",
    })

    post = database.list_history()[0]
    assert post["telegram_status"] == "failed"
    assert post["x_status"] == "prepared"
    assert "Telegram:" in post["error"]
    assert prepared_x.calls == [(output, "x caption")]
    assert controller.processor.edits["crop"]["width"] == pytest.approx(0.8)
    assert len(controller.processor.edits["overlays"]) == 1
    assert database.get_media(media_id)["posted_count"] == 1
    controller.shutdown()


@pytest.mark.asyncio
async def test_random_uses_cached_database_without_walking_library(
    monkeypatch, tmp_path: Path
) -> None:
    from cliprelay import controller as controller_module

    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    library_root.mkdir()
    source = library_root / "cached.mp4"
    source.write_bytes(b"source")
    media_id = database.upsert_media({
        "root_path": str(library_root), "path": str(source), "name": source.name,
        "relative_path": source.name, "folder": "", "duration": 5,
        "width": 640, "height": 360, "size_bytes": source.stat().st_size,
        "video_codec": "h264", "audio_codec": "aac", "frame_rate": 30,
        "mtime": source.stat().st_mtime,
    })
    settings = Settings(database)
    settings.set("library_root", str(library_root))
    settings.set("auto_index", False)
    secrets = MemorySecrets()
    controller = AppController(
        database,
        settings,
        secrets,
        LibraryModel(database),
        FolderModel(database),
        HistoryModel(database),
    )
    assert secrets.get_calls == 0
    assert controller._startup_refresh_timer.interval() == 4_000
    assert controller._startup_refresh_timer.isSingleShot()
    assert not controller._auto_scan_timer.isActive()
    secret_reads = secrets.get_calls
    assert controller.settings["botConfigured"] is False
    assert controller.settings["personalConfigured"] is False
    assert secrets.get_calls == secret_reads
    monkeypatch.setattr(
        controller.indexer,
        "discover",
        lambda *_: pytest.fail("Random must not walk the folder tree"),
    )
    main_thread = threading.get_ident()
    original_random = database.random_media

    def random_off_ui_thread(*args, **kwargs):
        assert threading.get_ident() != main_thread
        return original_random(*args, **kwargs)

    monkeypatch.setattr(database, "random_media", random_off_ui_thread)
    monkeypatch.setattr(controller.indexer, "ensure_thumbnail", lambda *_: None)

    await controller._pick_random_cached_async()

    assert controller.selectedMediaId == media_id
    assert not controller.selectedMediaChecking
    opened: list[str] = []
    monkeypatch.setattr(
        controller_module.QDesktopServices,
        "openUrl",
        lambda url: opened.append(url.toLocalFile()) or True,
    )
    controller.openSelectedVideo()
    assert len(opened) == 1
    assert Path(opened[0]) == source
    controller.shutdown()


@pytest.mark.asyncio
async def test_library_page_fetch_runs_off_ui_thread(
    monkeypatch, tmp_path: Path
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    settings = Settings(database)
    library = LibraryModel(database)
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        library,
        FolderModel(database),
        HistoryModel(database),
    )
    library.has_more = True
    main_thread = threading.get_ident()
    called = False

    def fetch_page(*_args, **_kwargs):
        nonlocal called
        called = True
        assert threading.get_ident() != main_thread
        return []

    monkeypatch.setattr(database, "list_media", fetch_page)
    controller.loadMoreMedia()
    for _ in range(100):
        if controller._load_more_task is None:
            break
        await asyncio.sleep(0.01)

    assert called
    assert controller._load_more_task is None
    controller.shutdown()


@pytest.mark.asyncio
async def test_filtered_library_refresh_runs_off_ui_thread(
    monkeypatch,
    tmp_path: Path,
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library = LibraryModel(database)
    controller = AppController(
        database,
        Settings(database),
        MemorySecrets(),
        library,
        FolderModel(database),
        HistoryModel(database),
    )
    controller._runtime_model_workers = True
    main_thread = threading.get_ident()
    queried = False

    def list_media(*_args, **_kwargs):
        nonlocal queried
        queried = True
        assert threading.get_ident() != main_thread
        return []

    monkeypatch.setattr(database, "list_media", list_media)
    controller.setSearch("large library query")
    for _ in range(100):
        if controller._library_model_refresh_task is None:
            break
        await asyncio.sleep(0.01)

    assert queried
    assert controller._library_model_refresh_task is None
    controller.shutdown()


@pytest.mark.asyncio
async def test_maximum_performance_prefers_hardware_export(
    tmp_path: Path,
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    settings = Settings(database)
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        LibraryModel(database),
        FolderModel(database),
        HistoryModel(database),
    )

    assert controller.processor.encoder_mode == "software"
    controller.setSetting("performance_mode", "maximum")
    assert controller.processor.encoder_mode == "hardware"
    assert controller._thumbnail_semaphore._value == 4
    assert controller._preview_semaphore._value == 2

    controller.setSetting("export_encoder", "software")
    assert controller.processor.encoder_mode == "software"
    controller.setSetting("export_encoder", "hardware")
    assert controller.processor.encoder_mode == "hardware"
    controller.shutdown()


@pytest.mark.asyncio
async def test_random_folder_filter_uses_selected_subtree(
    monkeypatch, tmp_path: Path
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    chosen = library_root / "chosen" / "deeper" / "selected.mp4"
    outside = library_root / "outside" / "other.mp4"
    chosen.parent.mkdir(parents=True)
    outside.parent.mkdir(parents=True)
    chosen.write_bytes(b"chosen")
    outside.write_bytes(b"outside")

    chosen_id = database.upsert_media({
        "root_path": str(library_root),
        "path": str(chosen),
        "name": chosen.name,
        "relative_path": "chosen/deeper/selected.mp4",
        "folder": "chosen/deeper",
        "duration": 5,
        "width": 640,
        "height": 360,
        "size_bytes": chosen.stat().st_size,
        "video_codec": "h264",
        "audio_codec": "aac",
        "frame_rate": 30,
        "mtime": chosen.stat().st_mtime,
    })
    database.upsert_media({
        "root_path": str(library_root),
        "path": str(outside),
        "name": outside.name,
        "relative_path": "outside/other.mp4",
        "folder": "outside",
        "duration": 5,
        "width": 640,
        "height": 360,
        "size_bytes": outside.stat().st_size,
        "video_codec": "h264",
        "audio_codec": "aac",
        "frame_rate": 30,
        "mtime": outside.stat().st_mtime,
    })
    settings = Settings(database)
    settings.set("library_root", str(library_root))
    settings.set("auto_index", False)
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        LibraryModel(database),
        FolderModel(database),
        HistoryModel(database),
    )
    monkeypatch.setattr(controller.indexer, "ensure_thumbnail", lambda *_: None)

    controller.setRandomFolderEnabled("chosen", True)
    assert controller.randomFolderSummary == "chosen"
    assert controller.randomFolderSelectionCount == 1
    assert settings.get("random_folders") == ["chosen"]
    controller.loadRandomFolderOptions()
    while controller.randomFolderOptionsLoading:
        await asyncio.sleep(0.01)
    assert next(
        row for row in controller.randomFolderOptions
        if row["folderPath"] == "chosen"
    )["videoCount"] == 1

    await controller._pick_random_cached_async()

    assert controller.selectedMediaId == chosen_id
    controller.clearRandomFolders()
    assert controller.randomFolderSummary == "No folders"
    assert controller.randomFolderSelectionCount == 0
    assert controller.hasRandomFolderSelection is False
    assert all(
        row["selected"] is False
        for row in controller.randomFolderOptions
    )

    controller.selectAllRandomFolders()
    assert controller.randomFolderSummary == "All folders"
    assert controller.allRandomFoldersSelected is True
    assert controller.randomFolderSelectionCount == len(
        controller.randomFolderOptions
    )
    assert all(
        row["selected"] is True
        for row in controller.randomFolderOptions
    )
    assert settings.get("random_folder_mode") == "all"
    assert settings.get("random_folders") == []
    controller.shutdown()


@pytest.mark.asyncio
async def test_selection_arrows_feed_unified_navigation_history(
    monkeypatch, tmp_path: Path
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    library_root.mkdir()
    media_ids: list[int] = []
    for index, (name, mtime) in enumerate([
        ("newest.mp4", 300),
        ("middle.mp4", 200),
        ("oldest.mp4", 100),
    ]):
        source = library_root / name
        source.write_bytes(str(index).encode())
        media_ids.append(database.upsert_media({
            "root_path": str(library_root),
            "path": str(source),
            "name": name,
            "relative_path": name,
            "folder": "",
            "duration": 5,
            "width": 640,
            "height": 360,
            "size_bytes": source.stat().st_size,
            "video_codec": "h264",
            "audio_codec": "aac",
            "frame_rate": 30,
            "mtime": mtime,
        }))
    settings = Settings(database)
    settings.set("library_root", str(library_root))
    settings.set("auto_index", False)
    library = LibraryModel(database)
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        library,
        FolderModel(database),
        HistoryModel(database),
    )
    monkeypatch.setattr(controller, "ensureThumbnail", lambda *_: None)
    monkeypatch.setattr(controller, "ensureTimeline", lambda *_: None)

    controller.selectMedia(media_ids[1])
    for _ in range(100):
        if controller._selection_navigation_task is None:
            break
        await asyncio.sleep(0.01)
    assert controller.canSelectPrevious
    assert controller.canSelectNext

    controller.navigateSelection(-1)
    assert controller.selectedMediaId == media_ids[0]
    assert controller.canNavigateBack

    controller.navigateBack()
    assert controller.selectedMediaId == media_ids[1]
    assert controller.canNavigateForward

    controller.navigateForward()
    assert controller.selectedMediaId == media_ids[0]

    random_rows = [
        database.get_media(media_ids[1]),
        database.get_media(media_ids[2]),
    ]
    monkeypatch.setattr(
        database,
        "random_media",
        lambda *_args, **_kwargs: random_rows.pop(0),
    )
    await controller._pick_random_cached_async()
    await controller._pick_random_cached_async()
    assert controller.selectedMediaId == media_ids[2]

    controller.navigateBack()
    assert controller.selectedMediaId == media_ids[1]

    controller.selectMedia(media_ids[0])
    assert controller.canNavigateForward is False
    controller.shutdown()


@pytest.mark.asyncio
async def test_navigation_history_restores_mixed_folder_and_video_states(
    monkeypatch, tmp_path: Path
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    folder_a = library_root / "folder-a"
    folder_b = library_root / "folder-b"
    folder_a.mkdir(parents=True)
    folder_b.mkdir()

    media_ids: dict[str, int] = {}
    for folder, name, mtime in [
        ("folder-a", "a.mp4", 200),
        ("folder-b", "b.mp4", 100),
    ]:
        source = library_root / folder / name
        source.write_bytes(name.encode())
        media_ids[folder] = database.upsert_media({
            "root_path": str(library_root),
            "path": str(source),
            "name": name,
            "relative_path": f"{folder}/{name}",
            "folder": folder,
            "duration": 5,
            "width": 640,
            "height": 360,
            "size_bytes": source.stat().st_size,
            "video_codec": "h264",
            "audio_codec": "aac",
            "frame_rate": 30,
            "mtime": mtime,
        })

    settings = Settings(database)
    settings.set("library_root", str(library_root))
    settings.set("auto_index", False)
    library = LibraryModel(database)
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        library,
        FolderModel(database),
        HistoryModel(database),
    )
    monkeypatch.setattr(controller, "ensureThumbnail", lambda *_: None)
    monkeypatch.setattr(controller, "ensureTimeline", lambda *_: None)

    restored: list[tuple[str, str, int]] = []
    controller.libraryNavigationRestored.connect(
        lambda folder, search, folder_index: restored.append(
            (folder, search, folder_index)
        )
    )

    controller.setFolder("folder-a")
    controller.selectMedia(media_ids["folder-a"])
    monkeypatch.setattr(
        database,
        "random_media",
        lambda *_args, **_kwargs: database.get_media(
            media_ids["folder-b"]
        ),
    )
    await controller._pick_random_cached_async()
    controller.setFolder("folder-b")

    assert library.folder == "folder-b"
    assert controller.selectedMediaId == media_ids["folder-b"]

    controller.navigateBack()
    assert library.folder == "folder-a"
    assert controller.selectedMediaId == media_ids["folder-b"]
    assert restored[-1][0] == "folder-a"

    controller.navigateBack()
    assert library.folder == "folder-a"
    assert controller.selectedMediaId == media_ids["folder-a"]

    controller.navigateForward()
    assert library.folder == "folder-a"
    assert controller.selectedMediaId == media_ids["folder-b"]

    controller.navigateForward()
    assert library.folder == "folder-b"
    assert controller.selectedMediaId == media_ids["folder-b"]
    assert controller.canNavigateForward is False

    replacement_root = tmp_path / "replacement"
    replacement_root.mkdir()
    controller.setSetting("library_root", str(replacement_root))
    assert controller.canNavigateBack is False
    assert controller.canNavigateForward is False
    controller.shutdown()


@pytest.mark.asyncio
async def test_select_generates_thumbnail_and_reveals_nested_file(
    monkeypatch, tmp_path: Path
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    nested = library_root / "nested" / "clips"
    nested.mkdir(parents=True)
    source = nested / "chosen.mp4"
    source.write_bytes(b"source")
    media_id = database.upsert_media({
        "root_path": str(library_root),
        "path": str(source),
        "name": source.name,
        "relative_path": "nested/clips/chosen.mp4",
        "folder": "nested/clips",
        "duration": 8,
        "width": 1280,
        "height": 720,
        "size_bytes": source.stat().st_size,
        "video_codec": "h264",
        "audio_codec": "aac",
        "frame_rate": 30,
        "mtime": source.stat().st_mtime,
    })
    settings = Settings(database)
    settings.set("library_root", str(library_root))
    library = LibraryModel(database)
    folders = FolderModel(database)
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        library,
        folders,
        HistoryModel(database),
    )
    generated: list[int] = []
    timeline_generated: list[int] = []
    thumbnail = tmp_path / "chosen.jpg"
    timeline = tmp_path / "chosen-timeline.jpg"

    def record_thumbnail(selected_id: int) -> Path:
        generated.append(selected_id)
        thumbnail.write_bytes(b"thumbnail")
        database.set_media_asset(selected_id, "thumbnail_path", str(thumbnail))
        return thumbnail

    def record_timeline(selected_id: int) -> Path:
        timeline_generated.append(selected_id)
        timeline.write_bytes(b"timeline")
        database.set_media_asset(
            selected_id,
            "timeline_path",
            str(timeline),
        )
        return timeline

    monkeypatch.setattr(controller.indexer, "ensure_thumbnail", record_thumbnail)
    monkeypatch.setattr(controller.indexer, "ensure_timeline", record_timeline)
    controller.selectMedia(media_id)
    for _ in range(50):
        if not controller._thumbnail_jobs and controller._timeline_task is None:
            break
        await asyncio.sleep(0.01)
    assert generated == [media_id]
    assert timeline_generated == [media_id]
    assert library.rows[0]["thumbnailState"] == "ready"
    assert library.rows[0]["thumbnailUrl"].startswith("file:")
    assert controller.selectedMedia["timelineUrl"].startswith("file:")

    library.search = "does not match"
    library.folder = "elsewhere"
    revealed: list[tuple[str, int, int]] = []
    controller.libraryRevealRequested.connect(
        lambda folder, media_index, folder_index: revealed.append(
            (folder, media_index, folder_index)
        )
    )
    controller.revealSelectedInLibrary()

    assert library.search == ""
    assert library.folder == "nested/clips"
    assert library.rows[0]["mediaId"] == media_id
    assert revealed == [("nested/clips", 0, folders.find_index("nested/clips"))]

    post_id = database.create_post({
        "media_id": media_id,
        "telegram_enabled": True,
        "x_enabled": False,
        "telegram_caption": "from history",
    })
    navigated: list[str] = []
    controller.navigationRequested.connect(navigated.append)
    controller.clearSelection()
    library.search = "hidden"
    library.folder = "elsewhere"
    revealed.clear()

    controller.viewHistoryPost(post_id)

    assert controller.selectedMediaId == media_id
    assert library.search == ""
    assert library.folder == "nested/clips"
    assert navigated == ["library"]
    assert revealed == [("nested/clips", 0, folders.find_index("nested/clips"))]
    controller.shutdown()


@pytest.mark.asyncio
async def test_visible_thumbnail_queue_limits_parallel_ffmpeg_work(
    monkeypatch, tmp_path: Path
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    library_root = tmp_path / "library"
    library_root.mkdir()
    media_ids: list[int] = []
    for index in range(6):
        source = library_root / f"clip-{index}.mp4"
        source.write_bytes(b"source")
        media_ids.append(database.upsert_media({
            "root_path": str(library_root),
            "path": str(source),
            "name": source.name,
            "relative_path": source.name,
            "folder": "",
            "duration": 8,
            "width": 1280,
            "height": 720,
            "size_bytes": source.stat().st_size,
            "video_codec": "h264",
            "audio_codec": "aac",
            "frame_rate": 30,
            "mtime": source.stat().st_mtime,
        }))
    settings = Settings(database)
    settings.set("library_root", str(library_root))
    library = LibraryModel(database)
    controller = AppController(
        database,
        settings,
        MemorySecrets(),
        library,
        FolderModel(database),
        HistoryModel(database),
    )
    state_lock = threading.Lock()
    active = 0
    maximum_active = 0

    def create_thumbnail(media_id: int) -> Path:
        nonlocal active, maximum_active
        with state_lock:
            active += 1
            maximum_active = max(maximum_active, active)
        time.sleep(0.03)
        target = tmp_path / f"{media_id}.jpg"
        target.write_bytes(b"thumbnail")
        database.set_media_asset(media_id, "thumbnail_path", str(target))
        with state_lock:
            active -= 1
        return target

    monkeypatch.setattr(controller.indexer, "ensure_thumbnail", create_thumbnail)
    for media_id in media_ids:
        controller.ensureThumbnail(media_id)
    for _ in range(100):
        if not controller._thumbnail_jobs:
            break
        await asyncio.sleep(0.01)

    assert maximum_active == 2
    assert all(row["thumbnailState"] == "ready" for row in library.rows)
    controller.shutdown()
