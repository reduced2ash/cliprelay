from __future__ import annotations

import json
from pathlib import Path

from cliprelay.database import Database
from cliprelay.qt_models import FolderModel, HistoryModel, LibraryModel, RandomFolderModel
from cliprelay.settings import Settings


def test_performance_settings_validate_and_default_safely(
    tmp_path: Path,
) -> None:
    settings = Settings(Database(tmp_path / "settings.sqlite3"))

    assert settings.get("performance_mode") == "automatic"
    assert settings.get("export_encoder") == "auto"

    settings.set("performance_mode", "maximum")
    settings.set("export_encoder", "hardware")
    assert settings.as_dict()["performance_mode"] == "maximum"
    assert settings.as_dict()["export_encoder"] == "hardware"

    settings.set("performance_mode", "turbo")
    settings.set("export_encoder", "mystery")
    assert settings.get("performance_mode") == "automatic"
    assert settings.get("export_encoder") == "auto"


def media_payload(path: Path, root: Path, name: str = "sample.mp4") -> dict:
    return {
        "root_path": str(root),
        "path": str(path),
        "name": name,
        "relative_path": name,
        "folder": "",
        "duration": 12.5,
        "width": 1280,
        "height": 720,
        "size_bytes": 1234,
        "video_codec": "h264",
        "audio_codec": "aac",
        "frame_rate": 30,
        "mtime": 100,
    }


def test_settings_media_shuffle_and_history(tmp_path: Path) -> None:
    database = Database(tmp_path / "cliprelay.sqlite3")
    database.set_setting("sort_mode", "name")
    assert database.get_setting("sort_mode") == "name"

    root = tmp_path / "library"
    root.mkdir()
    first = root / "sample.mp4"
    second = root / "second.mp4"
    first.write_bytes(b"one")
    second.write_bytes(b"two")
    first_id = database.upsert_media(media_payload(first, root))
    second_id = database.upsert_media(media_payload(second, root, "second.mp4"))
    assert database.counts() == {"media": 2, "posts": 0, "unseen": 2}

    picks = {database.random_media(True)["id"], database.random_media(True)["id"]}
    assert picks == {first_id, second_id}
    assert database.counts()["unseen"] == 0
    assert database.random_media(True) is not None

    export = tmp_path / "exports" / "prepared.mp4"
    export.parent.mkdir()
    export.write_bytes(b"prepared")
    export_id = database.create_export({
        "media_id": first_id,
        "path": export,
        "trim_start": 1,
        "trim_end": 4,
        "preset": "balanced",
        "size_bytes": export.stat().st_size,
        "duration": 3,
        "is_generated": True,
        "edit_spec": {
            "crop": {"x": 0.1, "y": 0, "width": 0.8, "height": 1},
            "overlays": [],
        },
    })
    post_id = database.create_post({
        "media_id": first_id,
        "telegram_enabled": True,
        "x_enabled": True,
        "telegram_caption": "telegram words",
        "x_caption": "x words",
        "telegram_destination": "@example",
    })
    database.update_post(post_id, export_id=export_id, telegram_status="sent", x_status="prepared")
    database.add_attempt(post_id, "telegram", "sent", "ok", "42")
    post = database.get_post(post_id)
    assert post and post["export_path"] == str(export)
    assert json.loads(post["edit_spec"])["crop"]["width"] == 0.8
    assert post["telegram_status"] == "sent"
    assert len(database.list_history("telegram")) == 1
    history_model = HistoryModel(database)
    history_model.refresh()
    assert history_model.rows[0]["edited"] is True
    assert database.counts()["posts"] == 1


def test_random_media_can_be_limited_to_folder_subtrees(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    root = tmp_path / "library"
    root.mkdir()

    def add_video(relative_path: str) -> int:
        path = root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(relative_path.encode())
        payload = media_payload(path, root, path.name)
        payload["relative_path"] = relative_path
        payload["folder"] = (
            "" if path.parent == root else path.parent.relative_to(root).as_posix()
        )
        return database.upsert_media(payload)

    root_id = add_video("root.mp4")
    direct_id = add_video("group/direct.mp4")
    nested_id = add_video("group/deeper/nested.mp4")
    outside_id = add_video("other/outside.mp4")

    assert database.list_random_folders() == [
        {"folder": "", "count": 1},
        {"folder": "group", "count": 2},
        {"folder": "group/deeper", "count": 1},
        {"folder": "other", "count": 1},
    ]

    subtree_picks = {
        database.random_media(True, folders=["group"])["id"]
        for _ in range(2)
    }
    assert subtree_picks == {direct_id, nested_id}

    database.mark_seen(outside_id)
    assert database.random_media(True, folders=["group"])["id"] in subtree_picks
    assert database.get_media(outside_id)["seen"] == 1
    assert database.random_media(False, folders=[""])["id"] == root_id


def test_folder_model_builds_a_compact_expandable_tree(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    root = tmp_path / "library"
    root.mkdir()

    def add_video(relative_path: str) -> None:
        path = root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(relative_path.encode())
        payload = media_payload(path, root, path.name)
        payload.update(
            relative_path=relative_path,
            folder=path.parent.relative_to(root).as_posix(),
        )
        database.upsert_media(payload)

    add_video("studio/direct.mp4")
    add_video("studio/shoots/day-one/first.mp4")
    add_video("studio/shoots/day-two/second.mp4")
    add_video("exports/final.mp4")

    assert {
        row["name"]
        for row in database.list_media(folder="studio")
    } == {"direct.mp4", "first.mp4", "second.mp4"}

    model = FolderModel(database)
    reset_events: list[bool] = []
    model.modelReset.connect(lambda: reset_events.append(True))
    model.refresh()

    assert model.totalCount == 5
    assert model.visibleCount == 3
    assert [row["folderPath"] for row in model.rows] == [
        "",
        "exports",
        "studio",
        "studio/shoots",
    ]
    studio = model.rows[2]
    assert studio["folderName"] == "studio"
    assert studio["videoCount"] == 3
    assert studio["folderDepth"] == 0
    assert studio["folderHasChildren"] is True
    assert studio["folderExpanded"] is True
    shoots = model.rows[3]
    assert shoots["folderDepth"] == 1
    assert shoots["folderHasChildren"] is True
    assert shoots["folderExpanded"] is False

    reset_count = len(reset_events)
    model.toggleExpanded("studio/shoots")
    assert len(reset_events) == reset_count + 1
    assert model.visibleCount == 5
    assert model.rows[4]["folderPath"] == "studio/shoots/day-one"
    assert model.rows[4]["folderDepth"] == 2

    model.toggleExpanded("studio")
    assert model.visibleCount == 2
    assert model.find_index("studio/shoots/day-two") >= 0
    assert "studio/shoots/day-two" in {
        row["folderPath"] for row in model.rows
    }

    model.collapseAll()
    assert model.visibleCount == 2
    assert [row["folderPath"] for row in model.rows] == [
        "",
        "exports",
        "studio",
    ]


def test_invalidate_missing_preserves_present_files(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    root = tmp_path / "library"
    root.mkdir()
    keep = root / "keep.mp4"
    gone = root / "gone.mp4"
    keep.write_bytes(b"1")
    gone.write_bytes(b"2")
    database.upsert_media(media_payload(keep, root, "keep.mp4"))
    database.upsert_media(media_payload(gone, root, "gone.mp4"))
    database.invalidate_missing(str(root), [str(keep)])
    rows = database.list_media()
    assert [row["name"] for row in rows] == ["keep.mp4"]


def test_active_library_is_immediate_and_preserves_other_roots(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    first_root = tmp_path / "first"
    second_root = tmp_path / "second"
    first_root.mkdir()
    second_root.mkdir()
    first = first_root / "first.mp4"
    second = second_root / "second.mp4"
    first.write_bytes(b"1")
    second.write_bytes(b"2")
    database.upsert_media(media_payload(first, first_root, first.name))
    database.upsert_media(media_payload(second, second_root, second.name))

    database.activate_root(first_root)
    assert [row["name"] for row in database.list_media()] == ["first.mp4"]
    database.activate_root(second_root)
    assert [row["name"] for row in database.list_media()] == ["second.mp4"]
    database.activate_root(first_root)
    assert [row["name"] for row in database.list_media()] == ["first.mp4"]


def test_lightweight_rows_require_probe_only_when_requested(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    root = tmp_path / "library"
    root.mkdir()
    video = root / "unchecked.mp4"
    video.write_bytes(b"video")
    stat = video.stat()
    payload = media_payload(video, root, video.name)
    payload.update(duration=0, width=0, height=0, size_bytes=stat.st_size, mtime=stat.st_mtime)
    database.upsert_media(payload)

    assert not database.media_needs_probe(
        str(video), stat.st_size, stat.st_mtime, require_probe=False
    )
    assert database.media_needs_probe(
        str(video), stat.st_size, stat.st_mtime, require_probe=True
    )


def test_large_library_model_is_paginated_without_filesystem_access(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    root = tmp_path / "library"
    database.activate_root(root)
    entries = [
        {
            "root_path": str(root),
            "path": str(root / f"folder-{index // 100}" / f"clip-{index:04}.mp4"),
            "name": f"clip-{index:04}.mp4",
            "relative_path": f"folder-{index // 100}/clip-{index:04}.mp4",
            "folder": f"folder-{index // 100}",
            "size_bytes": 1000 + index,
            "mtime": float(index),
        }
        for index in range(1000)
    ]
    assert database.upsert_manifest_batch(entries) == 1000
    model = LibraryModel(database)

    model.refresh()
    assert len(model.rows) == 240
    assert model.has_more
    assert model.rows[0]["mediaUrl"].startswith("file:")
    assert model.load_more()
    assert len(model.rows) == 480


def test_command_search_is_bounded_and_uses_indexed_media_and_folders(
    tmp_path: Path,
) -> None:
    database = Database(tmp_path / "db.sqlite3")
    root = tmp_path / "library"
    root.mkdir()

    for relative_path in (
        "Cosplay/Makima Nurse/portrait.mp4",
        "Cosplay/Makima Nurse/detail.mov",
        "Travel/alpine_route.mp4",
    ):
        path = root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(relative_path.encode())
        payload = media_payload(path, root, path.name)
        payload.update(
            relative_path=relative_path,
            folder=path.parent.relative_to(root).as_posix(),
        )
        database.upsert_media(payload)

    results = database.search_suggestions("makima nurse", limit=4)
    assert len(results) <= 4
    assert [row["kind"] for row in results] == [
        "media",
        "media",
        "folder",
    ]
    assert {row["title"] for row in results[:2]} == {
        "portrait.mp4",
        "detail.mov",
    }
    assert results[-1]["folderPath"] == "Cosplay/Makima Nurse"
    assert database.search_suggestions("%", limit=4) == []

    model = LibraryModel(database)
    model.search = "Makima"
    model.refresh()
    media_id = int(model.rows[0]["mediaId"])
    assert model.indexOf(media_id) >= 0


def test_media_neighbors_follow_library_order_and_filters(tmp_path: Path) -> None:
    database = Database(tmp_path / "db.sqlite3")
    root = tmp_path / "library"
    root.mkdir()

    media_ids: list[int] = []
    for index, (name, mtime, folder) in enumerate([
        ("newest.mp4", 300, "one"),
        ("middle.mp4", 200, "one"),
        ("oldest.mp4", 100, "two"),
    ]):
        path = root / folder / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(str(index).encode())
        payload = media_payload(path, root, name)
        payload.update(
            relative_path=f"{folder}/{name}",
            folder=folder,
            mtime=mtime,
        )
        media_ids.append(database.upsert_media(payload))

    neighbors = database.navigation_neighbors(
        media_ids[1],
        sort_mode="newest",
    )
    assert neighbors == {
        "found": True,
        "previousId": media_ids[0],
        "nextId": media_ids[2],
    }

    filtered = database.navigation_neighbors(
        media_ids[1],
        folder="one",
        sort_mode="newest",
    )
    assert filtered == {
        "found": True,
        "previousId": media_ids[0],
        "nextId": 0,
    }
    assert database.navigation_neighbors(
        media_ids[2],
        folder="one",
        sort_mode="newest",
    )["found"] is False


def test_random_folder_model_updates_selection_without_resetting() -> None:
    model = RandomFolderModel()
    reset_events: list[bool] = []
    model.modelReset.connect(lambda: reset_events.append(True))
    model.set_rows([
        {"folder": "events/2025", "count": 8},
        {"folder": "events/2026", "count": 11},
        {"folder": "personal", "count": 3},
    ], {"events/2025"})
    reset_count = len(reset_events)

    model.set_selected("events/2026", True)

    assert len(reset_events) == reset_count
    assert model.rows[1]["folderSelected"] is True
    model.setFilter("2026")
    assert model.visibleCount == 1
    assert model.rows[0]["folderPath"] == "events/2026"
    model.setSelectedOnly(True)
    assert model.visibleCount == 1


def test_large_library_safe_defaults(tmp_path: Path) -> None:
    settings = Settings(Database(tmp_path / "db.sqlite3"))
    assert settings.get("auto_index") is False
    assert settings.get("thumbnails_during_index") is False
    assert settings.get("ui_scale") == 1.0
    assert settings.get("theme_mode") == "relay"
    assert settings.get("random_folder_mode") == "all"


def test_theme_setting_is_persistent_and_rejects_unknown_modes(tmp_path: Path) -> None:
    settings = Settings(Database(tmp_path / "db.sqlite3"))

    settings.set("theme_mode", "pitch_black")
    assert settings.get("theme_mode") == "pitch_black"

    settings.set("theme_mode", "full_white")
    assert settings.get("theme_mode") == "full_white"

    settings.set("theme_mode", "surprise_me")
    assert settings.get("theme_mode") == "relay"


def test_random_folder_mode_rejects_unknown_values(tmp_path: Path) -> None:
    settings = Settings(Database(tmp_path / "db.sqlite3"))

    settings.set("random_folder_mode", "selected")
    assert settings.get("random_folder_mode") == "selected"

    settings.set("random_folder_mode", "surprise")
    assert settings.get("random_folder_mode") == "all"
