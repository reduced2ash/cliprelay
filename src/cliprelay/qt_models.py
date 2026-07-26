from __future__ import annotations

import json
from typing import Any

from PySide6.QtCore import (
    QAbstractListModel,
    QModelIndex,
    Property,
    Qt,
    QUrl,
    Signal,
    Slot,
)

from .database import Database
from .utils import format_bytes, format_duration


def _url(path: str | None) -> str:
    return QUrl.fromLocalFile(path).toString() if path else ""


class DictListModel(QAbstractListModel):
    ROLE_NAMES: dict[int, bytes] = {}

    def __init__(self):
        super().__init__()
        self.rows: list[dict[str, Any]] = []

    def roleNames(self) -> dict[int, bytes]:
        return self.ROLE_NAMES

    def rowCount(self, parent: QModelIndex = QModelIndex()) -> int:
        return 0 if parent.isValid() else len(self.rows)

    def data(self, index: QModelIndex, role: int = Qt.ItemDataRole.DisplayRole) -> Any:
        if not index.isValid() or not 0 <= index.row() < len(self.rows):
            return None
        name = self.ROLE_NAMES.get(role)
        return self.rows[index.row()].get(name.decode()) if name else None

    def replace(self, rows: list[dict[str, Any]]) -> None:
        self.beginResetModel()
        self.rows = rows
        self.endResetModel()


class LibraryModel(DictListModel):
    IdRole = Qt.ItemDataRole.UserRole + 1
    PathRole = IdRole + 1
    UrlRole = IdRole + 2
    NameRole = IdRole + 3
    FolderRole = IdRole + 4
    DurationRole = IdRole + 5
    DurationLabelRole = IdRole + 6
    SizeRole = IdRole + 7
    SizeLabelRole = IdRole + 8
    ResolutionRole = IdRole + 9
    CodecRole = IdRole + 10
    ThumbnailRole = IdRole + 11
    PreviewRole = IdRole + 12
    SeenRole = IdRole + 13
    PostedRole = IdRole + 14
    RelativeRole = IdRole + 15
    ThumbnailStateRole = IdRole + 16

    ROLE_NAMES = {
        IdRole: b"mediaId", PathRole: b"path", UrlRole: b"mediaUrl", NameRole: b"name",
        FolderRole: b"folder", DurationRole: b"duration", DurationLabelRole: b"durationLabel",
        SizeRole: b"sizeBytes", SizeLabelRole: b"sizeLabel", ResolutionRole: b"resolution",
        CodecRole: b"codec", ThumbnailRole: b"thumbnailUrl", PreviewRole: b"previewUrl",
        SeenRole: b"seen", PostedRole: b"postedCount", RelativeRole: b"relativePath",
        ThumbnailStateRole: b"thumbnailState",
    }

    def __init__(self, database: Database):
        super().__init__()
        self.database = database
        self.search = ""
        self.folder = ""
        self.sort_mode = "newest"
        self.page_size = 240
        self.has_more = False
        self.generation = 0
        self._thumbnail_states: dict[int, str] = {}

    def _map(self, row: dict[str, Any]) -> dict[str, Any]:
        checked = float(row["duration"]) > 0
        media_id = int(row["id"])
        thumbnail_path = row.get("thumbnail_path")
        thumbnail_state = (
            "ready"
            if thumbnail_path
            else self._thumbnail_states.get(media_id, "idle")
        )
        if not thumbnail_path and thumbnail_state == "ready":
            thumbnail_state = "idle"
        return {
            "mediaId": media_id, "path": row["path"], "mediaUrl": _url(row["path"]),
            "name": row["name"], "folder": row["folder"], "relativePath": row["relative_path"],
            "duration": float(row["duration"]),
            "durationLabel": format_duration(row["duration"]) if checked else "Unchecked",
            "sizeBytes": int(row["size_bytes"]), "sizeLabel": format_bytes(row["size_bytes"]),
            "resolution": f"{row['width']}×{row['height']}" if checked else "Unchecked",
            "codec": row["video_codec"],
            "thumbnailUrl": _url(thumbnail_path),
            "thumbnailState": thumbnail_state,
            "previewUrl": _url(row.get("preview_path")),
            "seen": bool(row["seen"]), "postedCount": int(row["posted_count"]),
        }

    def _page(self, offset: int, limit: int) -> tuple[list[dict[str, Any]], bool]:
        rows = self.database.list_media(
            self.search,
            self.folder,
            self.sort_mode,
            limit + 1,
            offset,
        )
        return rows[:limit], len(rows) > limit

    def refresh(self, preserve_loaded: bool = False) -> None:
        self.generation += 1
        limit = max(self.page_size, len(self.rows)) if preserve_loaded else self.page_size
        rows, self.has_more = self._page(0, limit)
        self.replace([self._map(row) for row in rows])

    def page_request(self) -> tuple[int, int, str, str, str, int] | None:
        if not self.has_more:
            return None
        return (
            self.generation,
            len(self.rows),
            self.search,
            self.folder,
            self.sort_mode,
            self.page_size,
        )

    def append_fetched(
        self,
        generation: int,
        offset: int,
        rows: list[dict[str, Any]],
        has_more: bool,
    ) -> bool:
        if generation != self.generation or offset != len(self.rows):
            return False
        self.has_more = has_more
        if not rows:
            return False
        mapped = [self._map(row) for row in rows]
        first = len(self.rows)
        self.beginInsertRows(QModelIndex(), first, first + len(mapped) - 1)
        self.rows.extend(mapped)
        self.endInsertRows()
        return True

    def load_more(self) -> bool:
        if not self.has_more:
            return False
        rows, self.has_more = self._page(len(self.rows), self.page_size)
        if not rows:
            return False
        mapped = [self._map(row) for row in rows]
        first = len(self.rows)
        self.beginInsertRows(QModelIndex(), first, first + len(mapped) - 1)
        self.rows.extend(mapped)
        self.endInsertRows()
        return True

    def find_index(self, media_id: int) -> int:
        return next((index for index, row in enumerate(self.rows) if row["mediaId"] == media_id), -1)

    def update_asset(self, media_id: int, role_name: str, path: str) -> None:
        index = self.find_index(media_id)
        if index < 0:
            return
        role = self.PreviewRole if role_name == "previewUrl" else self.ThumbnailRole
        self.rows[index][role_name] = _url(path)
        roles = [role]
        if role_name == "thumbnailUrl":
            self._thumbnail_states[media_id] = "ready"
            self.rows[index]["thumbnailState"] = "ready"
            roles.append(self.ThumbnailStateRole)
        model_index = self.index(index, 0)
        self.dataChanged.emit(model_index, model_index, roles)

    def set_thumbnail_state(self, media_id: int, state: str) -> None:
        self._thumbnail_states[media_id] = state
        index = self.find_index(media_id)
        if index < 0:
            return
        self.rows[index]["thumbnailState"] = state
        model_index = self.index(index, 0)
        self.dataChanged.emit(
            model_index,
            model_index,
            [self.ThumbnailStateRole],
        )

    def clear_thumbnail(self, media_id: int, state: str = "idle") -> None:
        self._thumbnail_states[media_id] = state
        index = self.find_index(media_id)
        if index < 0:
            return
        self.rows[index]["thumbnailUrl"] = ""
        self.rows[index]["thumbnailState"] = state
        model_index = self.index(index, 0)
        self.dataChanged.emit(
            model_index,
            model_index,
            [self.ThumbnailRole, self.ThumbnailStateRole],
        )


class FolderModel(DictListModel):
    PathRole = Qt.ItemDataRole.UserRole + 1
    NameRole = PathRole + 1
    CountRole = PathRole + 2
    ROLE_NAMES = {PathRole: b"folderPath", NameRole: b"folderName", CountRole: b"videoCount"}

    def __init__(self, database: Database):
        super().__init__()
        self.database = database

    def refresh(self) -> None:
        folders = [{"folderPath": "", "folderName": "All folders", "videoCount": self.database.counts()["media"]}]
        folders.extend(
            {
                "folderPath": row["folder"],
                "folderName": row["folder"].replace("/", "  /  ") if row["folder"] else "Library root",
                "videoCount": int(row["count"]),
            }
            for row in self.database.list_folders()
        )
        self.replace(folders)

    def find_index(self, folder: str) -> int:
        return next(
            (
                index
                for index, row in enumerate(self.rows)
                if row["folderPath"] == folder
            ),
            -1,
        )


class RandomFolderModel(DictListModel):
    PathRole = Qt.ItemDataRole.UserRole + 1
    NameRole = PathRole + 1
    DetailRole = PathRole + 2
    CountRole = PathRole + 3
    SelectedRole = PathRole + 4
    ROLE_NAMES = {
        PathRole: b"folderPath",
        NameRole: b"folderName",
        DetailRole: b"folderDetail",
        CountRole: b"videoCount",
        SelectedRole: b"folderSelected",
    }

    summaryChanged = Signal()

    def __init__(self):
        super().__init__()
        self._all_rows: list[dict[str, Any]] = []
        self._row_by_path: dict[str, dict[str, Any]] = {}
        self._visible_index: dict[str, int] = {}
        self._filter = ""
        self._selected_only = False

    @Property(int, notify=summaryChanged)
    def totalCount(self) -> int:
        return len(self._all_rows)

    @Property(int, notify=summaryChanged)
    def visibleCount(self) -> int:
        return len(self.rows)

    @Property(bool, notify=summaryChanged)
    def selectedOnly(self) -> bool:
        return self._selected_only

    def _matches(self, row: dict[str, Any]) -> bool:
        if self._selected_only and not row["folderSelected"]:
            return False
        if not self._filter:
            return True
        return self._filter in row["_searchText"]

    def _apply_filter(self) -> None:
        self.beginResetModel()
        self.rows = [row for row in self._all_rows if self._matches(row)]
        self._visible_index = {
            str(row["folderPath"]): index
            for index, row in enumerate(self.rows)
        }
        self.endResetModel()
        self.summaryChanged.emit()

    def set_rows(
        self,
        rows: list[dict[str, Any]],
        selected: set[str] | None = None,
    ) -> None:
        selected_paths = selected or set()
        mapped: list[dict[str, Any]] = []
        for source in rows:
            path = str(source.get("folder") or "")
            if path:
                name = path.rsplit("/", 1)[-1]
                parent = path.rsplit("/", 1)[0] if "/" in path else ""
                detail = (
                    parent.replace("/", "  /  ")
                    if parent
                    else "Top level"
                )
            else:
                name = "Library root only"
                detail = "Files directly inside the chosen library"
            mapped.append({
                "folderPath": path,
                "folderName": name,
                "folderDetail": detail,
                "videoCount": int(source.get("count") or 0),
                "folderSelected": path in selected_paths,
                "_searchText": f"{name}\n{path}".casefold(),
            })
        self._all_rows = mapped
        self._row_by_path = {
            str(row["folderPath"]): row
            for row in self._all_rows
        }
        self._apply_filter()

    @Slot()
    def clear(self) -> None:
        self._all_rows = []
        self._row_by_path = {}
        self._apply_filter()

    @Slot(str)
    def setFilter(self, value: str) -> None:
        normalized = str(value).strip().casefold()
        if normalized == self._filter:
            return
        self._filter = normalized
        self._apply_filter()

    @Slot(bool)
    def setSelectedOnly(self, enabled: bool) -> None:
        enabled = bool(enabled)
        if enabled == self._selected_only:
            return
        self._selected_only = enabled
        self._apply_filter()

    def set_selected(self, folder: str, enabled: bool) -> None:
        row = self._row_by_path.get(folder)
        if not row or bool(row["folderSelected"]) == bool(enabled):
            return
        row["folderSelected"] = bool(enabled)
        if self._selected_only:
            self._apply_filter()
            return
        visible_index = self._visible_index.get(folder, -1)
        if visible_index >= 0:
            model_index = self.index(visible_index, 0)
            self.dataChanged.emit(
                model_index,
                model_index,
                [self.SelectedRole],
            )

    def sync_selected(self, selected: set[str]) -> None:
        changed_paths: list[str] = []
        for row in self._all_rows:
            next_value = str(row["folderPath"]) in selected
            if bool(row["folderSelected"]) != next_value:
                row["folderSelected"] = next_value
                changed_paths.append(str(row["folderPath"]))
        if not changed_paths:
            return
        if self._selected_only:
            self._apply_filter()
            return
        changed_indexes = sorted(
            self._visible_index[path]
            for path in changed_paths
            if path in self._visible_index
        )
        if not changed_indexes:
            return
        range_start = changed_indexes[0]
        range_end = changed_indexes[0]
        for changed_index in changed_indexes[1:]:
            if changed_index == range_end + 1:
                range_end = changed_index
                continue
            self.dataChanged.emit(
                self.index(range_start, 0),
                self.index(range_end, 0),
                [self.SelectedRole],
            )
            range_start = changed_index
            range_end = changed_index
        self.dataChanged.emit(
            self.index(range_start, 0),
            self.index(range_end, 0),
            [self.SelectedRole],
        )


class HistoryModel(DictListModel):
    IdRole = Qt.ItemDataRole.UserRole + 1
    CreatedRole = IdRole + 1
    MediaNameRole = IdRole + 2
    ThumbnailRole = IdRole + 3
    TelegramRole = IdRole + 4
    XRole = IdRole + 5
    CaptionRole = IdRole + 6
    ExportRole = IdRole + 7
    SourceRole = IdRole + 8
    ErrorRole = IdRole + 9
    CanTrashRole = IdRole + 10
    SummaryRole = IdRole + 11
    XUrlRole = IdRole + 12
    TelegramUrlRole = IdRole + 13
    EditedRole = IdRole + 14
    ROLE_NAMES = {
        IdRole: b"postId", CreatedRole: b"createdAt", MediaNameRole: b"mediaName",
        ThumbnailRole: b"thumbnailUrl", TelegramRole: b"telegramStatus", XRole: b"xStatus",
        CaptionRole: b"caption", ExportRole: b"exportPath", SourceRole: b"sourcePath",
        ErrorRole: b"errorText", CanTrashRole: b"canTrash", SummaryRole: b"statusSummary",
        XUrlRole: b"xUrl", TelegramUrlRole: b"telegramUrl",
        EditedRole: b"edited",
    }

    def __init__(self, database: Database):
        super().__init__()
        self.database = database
        self.search = ""
        self.page_size = 200
        self.has_more = False

    def _map(self, row: dict[str, Any]) -> dict[str, Any]:
        telegram = row["telegram_status"]
        x_status = row["x_status"]
        summaries = []
        if telegram != "not_requested":
            summaries.append(f"Telegram: {telegram.replace('_', ' ')}")
        if x_status != "not_requested":
            summaries.append(f"X: {x_status.replace('_', ' ')}")
        try:
            edit_spec = json.loads(row.get("edit_spec") or "{}")
        except (TypeError, json.JSONDecodeError):
            edit_spec = {}
        edited = bool(edit_spec.get("crop") or edit_spec.get("overlays"))
        return {
            "postId": int(row["id"]), "createdAt": row["created_at"],
            "mediaName": row["media_name"], "thumbnailUrl": _url(row.get("thumbnail_path")),
            "telegramStatus": telegram, "xStatus": x_status,
            "caption": row["telegram_caption"] or row["x_caption"],
            "exportPath": row.get("export_path") or "", "sourcePath": row["source_path"],
            "errorText": row["error"],
            "canTrash": bool(row.get("is_generated")) and row.get("cleanup_state") != "trashed",
            "statusSummary": " · ".join(summaries) or "Draft", "xUrl": row["x_url"],
            "telegramUrl": row["telegram_message_link"],
            "edited": edited,
        }

    def refresh(self) -> None:
        rows = self.database.list_history(
            self.search,
            self.page_size + 1,
            0,
        )
        self.has_more = len(rows) > self.page_size
        self.replace([self._map(row) for row in rows[:self.page_size]])

    def load_more(self) -> bool:
        if not self.has_more:
            return False
        rows = self.database.list_history(
            self.search,
            self.page_size + 1,
            len(self.rows),
        )
        self.has_more = len(rows) > self.page_size
        mapped = [self._map(row) for row in rows[:self.page_size]]
        if not mapped:
            return False
        first = len(self.rows)
        self.beginInsertRows(QModelIndex(), first, first + len(mapped) - 1)
        self.rows.extend(mapped)
        self.endInsertRows()
        return True
