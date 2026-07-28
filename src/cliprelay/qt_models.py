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

    def refresh_request(
        self,
        preserve_loaded: bool = False,
    ) -> tuple[int, str, str, str, int]:
        self.generation += 1
        limit = (
            max(self.page_size, len(self.rows))
            if preserve_loaded
            else self.page_size
        )
        return (
            self.generation,
            self.search,
            self.folder,
            self.sort_mode,
            limit,
        )

    def fetch_refresh(
        self,
        request: tuple[int, str, str, str, int],
    ) -> tuple[int, list[dict[str, Any]], bool]:
        generation, search, folder, sort_mode, limit = request
        rows = self.database.list_media(
            search,
            folder,
            sort_mode,
            limit + 1,
            0,
        )
        return generation, rows[:limit], len(rows) > limit

    def apply_refresh(
        self,
        payload: tuple[int, list[dict[str, Any]], bool],
    ) -> bool:
        generation, rows, has_more = payload
        if generation != self.generation:
            return False
        self.has_more = has_more
        self.replace([self._map(row) for row in rows])
        return True

    def refresh(self, preserve_loaded: bool = False) -> None:
        self.apply_refresh(
            self.fetch_refresh(self.refresh_request(preserve_loaded))
        )

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

    @Slot(int, result=int)
    def indexOf(self, media_id: int) -> int:
        return self.find_index(media_id)

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
    FullPathRole = PathRole + 3
    DepthRole = PathRole + 4
    HasChildrenRole = PathRole + 5
    ExpandedRole = PathRole + 6
    KindRole = PathRole + 7
    ParentRole = PathRole + 8
    ROLE_NAMES = {
        PathRole: b"folderPath",
        NameRole: b"folderName",
        CountRole: b"videoCount",
        FullPathRole: b"folderFullPath",
        DepthRole: b"folderDepth",
        HasChildrenRole: b"folderHasChildren",
        ExpandedRole: b"folderExpanded",
        KindRole: b"folderKind",
        ParentRole: b"folderParent",
    }

    summaryChanged = Signal()

    def __init__(self, database: Database):
        super().__init__()
        self.database = database
        self._nodes: dict[str, dict[str, Any]] = {}
        self._children: dict[str, list[str]] = {}
        self._expanded: set[str] = set()
        self._initialized = False
        self._media_count = 0

    @Property(int, notify=summaryChanged)
    def totalCount(self) -> int:
        return len(self._nodes)

    @Property(int, notify=summaryChanged)
    def visibleCount(self) -> int:
        return max(0, len(self.rows) - 1)

    @staticmethod
    def _parent(path: str) -> str:
        return path.rpartition("/")[0]

    def _visible_rows(self) -> list[dict[str, Any]]:
        rows = [{
            "folderPath": "",
            "folderName": "All videos",
            "videoCount": self._media_count,
            "folderFullPath": "Entire library",
            "folderDepth": 0,
            "folderHasChildren": False,
            "folderExpanded": False,
            "folderKind": "all",
            "folderParent": "",
        }]

        def append_branch(parent: str, depth: int) -> None:
            for path in self._children.get(parent, []):
                node = self._nodes[path]
                rows.append({
                    **node,
                    "folderDepth": depth,
                    "folderHasChildren": bool(self._children.get(path)),
                    "folderExpanded": path in self._expanded,
                })
                if path in self._expanded:
                    append_branch(path, depth + 1)

        append_branch("", 0)
        return rows

    def _rebuild_visible(self) -> None:
        self.replace(self._visible_rows())
        self.summaryChanged.emit()

    def fetch_refresh(self) -> tuple[int, dict[str, dict[str, Any]], dict[str, list[str]]]:
        media_count = self.database.counts()["media"]
        nodes: dict[str, dict[str, Any]] = {}
        children: dict[str, list[str]] = {}
        for row in self.database.list_random_folders():
            path = str(row["folder"])
            if not path:
                continue
            parent = self._parent(path)
            nodes[path] = {
                "folderPath": path,
                "folderName": path.rpartition("/")[2],
                "videoCount": int(row["count"]),
                "folderFullPath": path,
                "folderKind": "folder",
                "folderParent": parent,
            }
            children.setdefault(parent, []).append(path)

        for paths in children.values():
            paths.sort(key=lambda path: nodes[path]["folderName"].casefold())
        return media_count, nodes, children

    def apply_refresh(
        self,
        payload: tuple[int, dict[str, dict[str, Any]], dict[str, list[str]]],
    ) -> None:
        media_count, nodes, children = payload
        previous_paths = set(self._nodes)
        self._media_count = media_count
        self._nodes = nodes
        self._children = children
        self._expanded.intersection_update(nodes)
        top_level_branches = {
            path
            for path in children.get("", [])
            if children.get(path)
        }
        auto_expand = (
            top_level_branches
            if len(top_level_branches) <= 32
            else set()
        )
        if not self._initialized:
            self._expanded.update(auto_expand)
            self._initialized = True
        else:
            self._expanded.update(auto_expand - previous_paths)
        self._rebuild_visible()

    def refresh(self) -> None:
        self.apply_refresh(self.fetch_refresh())

    def _index_without_expanding(self, folder: str) -> int:
        return next(
            (
                index
                for index, row in enumerate(self.rows)
                if row["folderPath"] == folder
            ),
            -1,
        )

    @Slot(str, result=int)
    def toggleExpanded(self, folder: str) -> int:
        if not self._children.get(folder):
            return self._index_without_expanding(folder)
        if folder in self._expanded:
            self._expanded.remove(folder)
        else:
            self._expanded.add(folder)
        self._rebuild_visible()
        return self._index_without_expanding(folder)

    @Slot()
    def collapseAll(self) -> None:
        if not self._expanded:
            return
        self._expanded.clear()
        self._rebuild_visible()

    @Slot(str, result=int)
    def expandTo(self, folder: str) -> int:
        ancestors: set[str] = set()
        parent = self._parent(folder)
        while parent:
            ancestors.add(parent)
            parent = self._parent(parent)
        if not ancestors.issubset(self._expanded):
            self._expanded.update(ancestors)
            self._rebuild_visible()
        return self._index_without_expanding(folder)

    @Slot(str, result=int)
    def indexOf(self, folder: str) -> int:
        return self._index_without_expanding(folder)

    @Slot(str, result=int)
    def parentIndex(self, folder: str) -> int:
        parent = self._parent(folder)
        return self._index_without_expanding(parent) if parent else 0

    def find_index(self, folder: str) -> int:
        return self.expandTo(folder)


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
        self.generation = 0

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

    def refresh_request(self) -> tuple[int, str, int]:
        self.generation += 1
        return self.generation, self.search, self.page_size

    def fetch_refresh(
        self,
        request: tuple[int, str, int],
    ) -> tuple[int, list[dict[str, Any]], bool]:
        generation, search, page_size = request
        rows = self.database.list_history(
            search,
            page_size + 1,
            0,
        )
        return generation, rows[:page_size], len(rows) > page_size

    def apply_refresh(
        self,
        payload: tuple[int, list[dict[str, Any]], bool],
    ) -> bool:
        generation, rows, has_more = payload
        if generation != self.generation:
            return False
        self.has_more = has_more
        self.replace([self._map(row) for row in rows])
        return True

    def refresh(self) -> None:
        self.apply_refresh(self.fetch_refresh(self.refresh_request()))

    def page_request(self) -> tuple[int, int, str, int] | None:
        if not self.has_more:
            return None
        return (
            self.generation,
            len(self.rows),
            self.search,
            self.page_size,
        )

    def fetch_page(
        self,
        request: tuple[int, int, str, int],
    ) -> tuple[int, int, list[dict[str, Any]], bool]:
        generation, offset, search, page_size = request
        rows = self.database.list_history(
            search,
            page_size + 1,
            offset,
        )
        return (
            generation,
            offset,
            rows[:page_size],
            len(rows) > page_size,
        )

    def append_fetched(
        self,
        payload: tuple[int, int, list[dict[str, Any]], bool],
    ) -> bool:
        generation, offset, rows, has_more = payload
        if generation != self.generation or offset != len(self.rows):
            return False
        self.has_more = has_more
        if not rows:
            return False
        mapped = [self._map(row) for row in rows]
        first = len(self.rows)
        self.beginInsertRows(
            QModelIndex(),
            first,
            first + len(mapped) - 1,
        )
        self.rows.extend(mapped)
        self.endInsertRows()
        return True

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
