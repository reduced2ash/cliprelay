from __future__ import annotations

import json
import random
import sqlite3
import threading
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterable, Iterator

from .utils import utc_now


EXACT_FOLDER_SCOPE_PREFIX = "\x1e"


SCHEMA = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS media_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root_path TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    folder TEXT NOT NULL DEFAULT '',
    duration REAL NOT NULL DEFAULT 0,
    width INTEGER NOT NULL DEFAULT 0,
    height INTEGER NOT NULL DEFAULT 0,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    video_codec TEXT NOT NULL DEFAULT '',
    audio_codec TEXT NOT NULL DEFAULT '',
    frame_rate REAL NOT NULL DEFAULT 0,
    mtime REAL NOT NULL DEFAULT 0,
    thumbnail_path TEXT,
    preview_path TEXT,
    timeline_path TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    valid INTEGER NOT NULL DEFAULT 1,
    seen INTEGER NOT NULL DEFAULT 0,
    posted_count INTEGER NOT NULL DEFAULT 0,
    indexed_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_media_root ON media_files(root_path);
CREATE INDEX IF NOT EXISTS idx_media_folder ON media_files(folder);
CREATE INDEX IF NOT EXISTS idx_media_valid_seen ON media_files(valid, seen);
CREATE INDEX IF NOT EXISTS idx_media_active_valid_folder
    ON media_files(active, valid, folder);
CREATE INDEX IF NOT EXISTS idx_media_active_valid_mtime
    ON media_files(active, valid, mtime DESC);
CREATE INDEX IF NOT EXISTS idx_media_active_valid_name
    ON media_files(active, valid, name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS exports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id INTEGER NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    trim_start REAL NOT NULL DEFAULT 0,
    trim_end REAL NOT NULL DEFAULT 0,
    preset TEXT NOT NULL,
    target_mb REAL NOT NULL DEFAULT 0,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    duration REAL NOT NULL DEFAULT 0,
    is_generated INTEGER NOT NULL DEFAULT 1,
    edit_spec TEXT NOT NULL DEFAULT '{}',
    cleanup_state TEXT NOT NULL DEFAULT 'kept'
);

CREATE TABLE IF NOT EXISTS posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id INTEGER NOT NULL REFERENCES media_files(id),
    export_id INTEGER REFERENCES exports(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    telegram_enabled INTEGER NOT NULL DEFAULT 0,
    x_enabled INTEGER NOT NULL DEFAULT 0,
    telegram_caption TEXT NOT NULL DEFAULT '',
    x_caption TEXT NOT NULL DEFAULT '',
    telegram_mode TEXT NOT NULL DEFAULT 'bot',
    telegram_destination TEXT NOT NULL DEFAULT '',
    telegram_status TEXT NOT NULL DEFAULT 'not_requested',
    telegram_message_id TEXT NOT NULL DEFAULT '',
    telegram_message_link TEXT NOT NULL DEFAULT '',
    x_status TEXT NOT NULL DEFAULT 'not_requested',
    x_url TEXT NOT NULL DEFAULT '',
    cleanup_policy TEXT NOT NULL DEFAULT 'keep',
    error TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS delivery_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    detail TEXT NOT NULL DEFAULT '',
    remote_id TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at DESC);
"""


class Database:
    def __init__(self, path: str | Path):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = threading.RLock()
        self._active_root: str | None = None
        with self.connection() as connection:
            connection.executescript(SCHEMA)
            columns = {
                row["name"]
                for row in connection.execute("PRAGMA table_info(media_files)").fetchall()
            }
            if "active" not in columns:
                connection.execute(
                    "ALTER TABLE media_files ADD COLUMN active INTEGER NOT NULL DEFAULT 1"
                )
            if "timeline_path" not in columns:
                connection.execute(
                    "ALTER TABLE media_files ADD COLUMN timeline_path TEXT"
                )
            export_columns = {
                row["name"]
                for row in connection.execute("PRAGMA table_info(exports)").fetchall()
            }
            if "edit_spec" not in export_columns:
                connection.execute(
                    "ALTER TABLE exports ADD COLUMN edit_spec TEXT NOT NULL DEFAULT '{}'"
                )
            connection.execute(
                "CREATE INDEX IF NOT EXISTS idx_media_active_valid_seen "
                "ON media_files(active, valid, seen)"
            )

    @contextmanager
    def connection(self) -> Iterator[sqlite3.Connection]:
        connection = sqlite3.connect(self.path, timeout=30, check_same_thread=False)
        connection.row_factory = sqlite3.Row
        connection.execute("PRAGMA foreign_keys=ON")
        try:
            yield connection
            connection.commit()
        except Exception:
            connection.rollback()
            raise
        finally:
            connection.close()

    def get_setting(self, key: str, default: Any = None) -> Any:
        with self.connection() as connection:
            row = connection.execute("SELECT value FROM settings WHERE key = ?", (key,)).fetchone()
        if not row:
            return default
        try:
            return json.loads(row["value"])
        except json.JSONDecodeError:
            return default

    def get_settings(self) -> dict[str, Any]:
        with self.connection() as connection:
            rows = connection.execute("SELECT key, value FROM settings").fetchall()
        values: dict[str, Any] = {}
        for row in rows:
            try:
                values[str(row["key"])] = json.loads(row["value"])
            except json.JSONDecodeError:
                continue
        return values

    def set_setting(self, key: str, value: Any) -> None:
        payload = json.dumps(value, ensure_ascii=False)
        with self.connection() as connection:
            connection.execute(
                "INSERT INTO settings(key, value) VALUES(?, ?) "
                "ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                (key, payload),
            )

    def upsert_media(self, metadata: dict[str, Any]) -> int:
        now = utc_now()
        values = {
            "root_path": str(metadata["root_path"]),
            "path": str(metadata["path"]),
            "name": metadata["name"],
            "relative_path": metadata["relative_path"],
            "folder": metadata.get("folder", ""),
            "duration": float(metadata.get("duration", 0)),
            "width": int(metadata.get("width", 0)),
            "height": int(metadata.get("height", 0)),
            "size_bytes": int(metadata.get("size_bytes", 0)),
            "video_codec": metadata.get("video_codec", ""),
            "audio_codec": metadata.get("audio_codec", ""),
            "frame_rate": float(metadata.get("frame_rate", 0)),
            "mtime": float(metadata.get("mtime", 0)),
            "active": int(
                self._active_root is None
                or str(metadata["root_path"]) == self._active_root
            ),
            "indexed_at": now,
            "updated_at": now,
        }
        columns = ", ".join(values)
        placeholders = ", ".join(f":{key}" for key in values)
        updates = ", ".join(
            f"{key}=excluded.{key}" for key in values if key not in {"path", "indexed_at"}
        )
        with self.connection() as connection:
            connection.execute(
                f"INSERT INTO media_files({columns}) VALUES({placeholders}) "
                f"ON CONFLICT(path) DO UPDATE SET {updates}, valid=1",
                values,
            )
            row = connection.execute("SELECT id FROM media_files WHERE path = ?", (values["path"],)).fetchone()
        return int(row["id"])

    def upsert_manifest_batch(self, entries: Iterable[dict[str, Any]]) -> int:
        now = utc_now()
        payloads = []
        for entry in entries:
            root_path = str(entry["root_path"])
            payloads.append({
                "root_path": root_path,
                "path": str(entry["path"]),
                "name": str(entry["name"]),
                "relative_path": str(entry["relative_path"]),
                "folder": str(entry.get("folder", "")),
                "size_bytes": int(entry.get("size_bytes", 0)),
                "mtime": float(entry.get("mtime", 0)),
                "active": int(self._active_root is None or root_path == self._active_root),
                "indexed_at": now,
                "updated_at": now,
            })
        if not payloads:
            return 0
        changed = (
            "media_files.size_bytes != excluded.size_bytes "
            "OR ABS(media_files.mtime - excluded.mtime) > 0.001"
        )
        with self.connection() as connection:
            before = connection.total_changes
            connection.executemany(
                f"""INSERT INTO media_files(
                    root_path,path,name,relative_path,folder,size_bytes,mtime,active,indexed_at,updated_at
                ) VALUES(
                    :root_path,:path,:name,:relative_path,:folder,:size_bytes,:mtime,:active,
                    :indexed_at,:updated_at
                ) ON CONFLICT(path) DO UPDATE SET
                    root_path=excluded.root_path,
                    name=excluded.name,
                    relative_path=excluded.relative_path,
                    folder=excluded.folder,
                    duration=CASE WHEN {changed} THEN 0 ELSE media_files.duration END,
                    width=CASE WHEN {changed} THEN 0 ELSE media_files.width END,
                    height=CASE WHEN {changed} THEN 0 ELSE media_files.height END,
                    video_codec=CASE WHEN {changed} THEN '' ELSE media_files.video_codec END,
                    audio_codec=CASE WHEN {changed} THEN '' ELSE media_files.audio_codec END,
                    frame_rate=CASE WHEN {changed} THEN 0 ELSE media_files.frame_rate END,
                    thumbnail_path=CASE WHEN {changed} THEN NULL ELSE media_files.thumbnail_path END,
                    preview_path=CASE WHEN {changed} THEN NULL ELSE media_files.preview_path END,
                    timeline_path=CASE WHEN {changed} THEN NULL ELSE media_files.timeline_path END,
                    seen=CASE WHEN {changed} THEN 0 ELSE media_files.seen END,
                    size_bytes=excluded.size_bytes,
                    mtime=excluded.mtime,
                    active=excluded.active,
                    valid=1,
                    updated_at=excluded.updated_at
                WHERE media_files.root_path != excluded.root_path
                    OR media_files.name != excluded.name
                    OR media_files.relative_path != excluded.relative_path
                    OR media_files.folder != excluded.folder
                    OR {changed}
                    OR media_files.valid != 1
                    OR media_files.active != excluded.active""",
                payloads,
            )
            changed_rows = connection.total_changes - before
        return int(changed_rows)

    def invalidate_absent(self, root_path: str | Path, existing_paths: Iterable[str]) -> int:
        present = set(existing_paths)
        with self.connection() as connection:
            rows = connection.execute(
                "SELECT id, path FROM media_files WHERE root_path=? AND valid=1",
                (str(root_path),),
            ).fetchall()
            missing = [(int(row["id"]),) for row in rows if str(row["path"]) not in present]
            if missing:
                connection.executemany(
                    "UPDATE media_files SET valid=0 WHERE id=?", missing
                )
        return len(missing)

    def manifest_paths(self, root_path: str | Path) -> list[Path]:
        with self.connection() as connection:
            rows = connection.execute(
                "SELECT path FROM media_files WHERE root_path=? AND valid=1 ORDER BY path",
                (str(root_path),),
            ).fetchall()
        return [Path(str(row["path"])) for row in rows]

    def media_state_map(self, root_path: str | Path) -> dict[str, dict[str, Any]]:
        with self.connection() as connection:
            rows = connection.execute(
                """SELECT id,path,name,relative_path,folder,size_bytes,mtime,
                          duration,valid,thumbnail_path,preview_path,timeline_path
                   FROM media_files WHERE root_path=?""",
                (str(root_path),),
            ).fetchall()
        return {str(row["path"]): dict(row) for row in rows}

    def activate_root(self, root_path: str | Path | None) -> None:
        with self._lock:
            self.set_active_root(root_path)
            self.reconcile_active_root()

    def set_active_root(self, root_path: str | Path | None) -> None:
        with self._lock:
            self._active_root = (
                str(Path(root_path).expanduser().resolve())
                if root_path
                else None
            )

    def reconcile_active_root(self) -> None:
        with self._lock:
            with self.connection() as connection:
                if self._active_root is None:
                    connection.execute("UPDATE media_files SET active=0 WHERE active!=0")
                else:
                    connection.execute(
                        """UPDATE media_files
                           SET active=CASE WHEN root_path=? THEN 1 ELSE 0 END
                           WHERE active!=CASE WHEN root_path=? THEN 1 ELSE 0 END""",
                        (self._active_root, self._active_root),
                    )

    def invalidate_missing(self, root_path: str, existing_paths: Iterable[str]) -> None:
        paths = list(existing_paths)
        with self.connection() as connection:
            connection.execute("UPDATE media_files SET valid=0 WHERE root_path=?", (root_path,))
            for offset in range(0, len(paths), 500):
                chunk = paths[offset : offset + 500]
                placeholders = ",".join("?" for _ in chunk)
                connection.execute(
                    f"UPDATE media_files SET valid=1 WHERE root_path=? AND path IN ({placeholders})",
                    [root_path, *chunk],
                )

    def media_needs_probe(
        self,
        path: str,
        size_bytes: int,
        mtime: float,
        require_probe: bool = True,
    ) -> bool:
        with self.connection() as connection:
            row = connection.execute(
                "SELECT size_bytes, mtime, duration, valid FROM media_files WHERE path=?", (path,)
            ).fetchone()
        return (
            not row
            or not int(row["valid"])
            or int(row["size_bytes"]) != size_bytes
            or abs(float(row["mtime"]) - mtime) > 0.001
            or (require_probe and float(row["duration"]) <= 0)
        )

    def get_media_by_path(self, path: str | Path) -> dict[str, Any] | None:
        with self.connection() as connection:
            row = connection.execute(
                "SELECT * FROM media_files WHERE path=?", (str(path),)
            ).fetchone()
        return dict(row) if row else None

    def set_media_valid(self, media_id: int, valid: bool) -> None:
        with self.connection() as connection:
            connection.execute(
                "UPDATE media_files SET valid=?, updated_at=? WHERE id=?",
                (int(valid), utc_now(), media_id),
            )

    def set_path_valid(self, path: str | Path, valid: bool) -> None:
        with self.connection() as connection:
            connection.execute(
                "UPDATE media_files SET valid=?, updated_at=? WHERE path=?",
                (int(valid), utc_now(), str(path)),
            )

    def set_media_asset(self, media_id: int, kind: str, path: str) -> None:
        if kind not in {"thumbnail_path", "preview_path", "timeline_path"}:
            raise ValueError("Unsupported media asset kind")
        with self.connection() as connection:
            connection.execute(
                f"UPDATE media_files SET {kind}=?, updated_at=? WHERE id=?",
                (path, utc_now(), media_id),
            )

    def clear_media_asset(self, media_id: int, kind: str) -> None:
        if kind not in {"thumbnail_path", "preview_path", "timeline_path"}:
            raise ValueError("Unsupported media asset kind")
        with self.connection() as connection:
            connection.execute(
                f"UPDATE media_files SET {kind}=NULL, updated_at=? WHERE id=?",
                (utc_now(), media_id),
            )

    def get_media(self, media_id: int) -> dict[str, Any] | None:
        with self.connection() as connection:
            row = connection.execute("SELECT * FROM media_files WHERE id=?", (media_id,)).fetchone()
        return dict(row) if row else None

    @staticmethod
    def _media_order(sort_mode: str) -> str:
        return {
            "name": "name COLLATE NOCASE ASC, id ASC",
            "oldest": "mtime ASC, id ASC",
            "duration": "duration DESC, id ASC",
            "size": "size_bytes DESC, id ASC",
            "newest": "mtime DESC, id DESC",
        }.get(sort_mode, "mtime DESC, id DESC")

    @staticmethod
    def _media_filter(search: str, folder: str) -> tuple[list[str], list[Any]]:
        clauses = ["valid=1", "active=1"]
        values: list[Any] = []
        if search.strip():
            clauses.append("(name LIKE ? OR relative_path LIKE ?)")
            term = f"%{search.strip()}%"
            values.extend([term, term])
        if folder:
            escaped = (
                folder.replace("\\", "\\\\")
                .replace("%", "\\%")
                .replace("_", "\\_")
            )
            clauses.append("(folder=? OR folder LIKE ? ESCAPE '\\')")
            values.extend([folder, f"{escaped}/%"])
        return clauses, values

    def list_media(
        self,
        search: str = "",
        folder: str = "",
        sort_mode: str = "newest",
        limit: int = 5000,
        offset: int = 0,
    ) -> list[dict[str, Any]]:
        order = self._media_order(sort_mode)
        clauses, values = self._media_filter(search, folder)
        values.extend([limit, max(0, offset)])
        with self.connection() as connection:
            rows = connection.execute(
                f"SELECT * FROM media_files WHERE {' AND '.join(clauses)} "
                f"ORDER BY {order} LIMIT ? OFFSET ?",
                values,
            ).fetchall()
        return [dict(row) for row in rows]

    @staticmethod
    def _command_scope(value: str) -> str:
        scope = str(value or "all").strip().lower()
        return scope if scope in {"all", "videos", "folders"} else "all"

    @staticmethod
    def _merge_command_results(
        media_results: list[dict[str, Any]],
        folder_results: list[dict[str, Any]],
        limit: int,
        scope: str,
    ) -> list[dict[str, Any]]:
        if scope == "videos":
            return media_results[:limit]
        if scope == "folders":
            return folder_results[:limit]
        if limit == 1:
            return (media_results or folder_results)[:1]

        reserved_folders = min(3, max(1, limit // 3))
        folder_count = min(len(folder_results), reserved_folders)
        media_count = min(len(media_results), limit - folder_count)
        results = (
            media_results[:media_count]
            + folder_results[:folder_count]
        )
        remaining = limit - len(results)
        if remaining:
            extra_media = media_results[media_count:media_count + remaining]
            results.extend(extra_media)
            remaining -= len(extra_media)
        if remaining:
            results.extend(
                folder_results[folder_count:folder_count + remaining]
            )
        return results

    def search_suggestions(
        self,
        query: str,
        limit: int = 9,
        scope: str = "all",
    ) -> list[dict[str, Any]]:
        """Return a small, UI-ready set of media and folder matches.

        Command-center search deliberately stays inside SQLite. It never walks
        the filesystem or asks the media pipeline to inspect a file.
        """

        value = query.strip()
        capped_limit = max(1, min(int(limit), 12))
        normalized_scope = self._command_scope(scope)
        if not value:
            return []
        escaped = (
            value.replace("\\", "\\\\")
            .replace("%", "\\%")
            .replace("_", "\\_")
        )
        contains = f"%{escaped}%"
        prefix = f"{escaped}%"
        leaf_exact = f"%/{escaped}"
        leaf_prefix = f"%/{prefix}"
        media_rows: list[sqlite3.Row] = []
        folder_rows: list[sqlite3.Row] = []
        with self.connection() as connection:
            if normalized_scope != "folders":
                media_rows = connection.execute(
                    "SELECT id,name,relative_path,folder FROM media_files "
                    "WHERE valid=1 AND active=1 "
                    "AND (name LIKE ? ESCAPE '\\' "
                    "OR relative_path LIKE ? ESCAPE '\\') "
                    "ORDER BY CASE WHEN name LIKE ? ESCAPE '\\' "
                    "THEN 0 ELSE 1 END, name COLLATE NOCASE, id "
                    "LIMIT ?",
                    (contains, contains, prefix, capped_limit),
                ).fetchall()
            if normalized_scope != "videos":
                folder_rows = connection.execute(
                    "SELECT folder,COUNT(*) AS count FROM media_files "
                    "WHERE valid=1 AND active=1 AND folder != '' "
                    "AND folder LIKE ? ESCAPE '\\' "
                    "GROUP BY folder "
                    "ORDER BY CASE "
                    "WHEN folder = ? COLLATE NOCASE "
                    "OR folder LIKE ? ESCAPE '\\' THEN 0 "
                    "WHEN folder LIKE ? ESCAPE '\\' "
                    "OR folder LIKE ? ESCAPE '\\' THEN 1 "
                    "ELSE 2 END, folder COLLATE NOCASE "
                    "LIMIT ?",
                    (
                        contains,
                        value,
                        leaf_exact,
                        prefix,
                        leaf_prefix,
                        capped_limit,
                    ),
                ).fetchall()

        media_results: list[dict[str, Any]] = []
        for row in media_rows:
            relative_path = str(row["relative_path"] or row["name"])
            media_results.append(
                {
                    "kind": "media",
                    "mediaId": int(row["id"]),
                    "folderPath": str(row["folder"] or ""),
                    "title": str(row["name"]),
                    "detail": relative_path,
                    "icon": "media",
                    "count": 0,
                }
            )

        folder_results: list[dict[str, Any]] = []
        for row in folder_rows:
            folder = str(row["folder"])
            folder_results.append(
                {
                    "kind": "folder",
                    "mediaId": 0,
                    "folderPath": folder,
                    "title": folder.rsplit("/", 1)[-1],
                    "detail": folder,
                    "icon": "folder",
                    "count": int(row["count"]),
                }
            )
        return self._merge_command_results(
            media_results,
            folder_results,
            capped_limit,
            normalized_scope,
        )

    def command_center_overview(
        self,
        limit: int = 9,
        scope: str = "all",
    ) -> list[dict[str, Any]]:
        """Return a compact zero-query mix of recent media and folders."""

        capped_limit = max(3, min(int(limit), 12))
        normalized_scope = self._command_scope(scope)
        media_limit = (
            capped_limit
            if normalized_scope == "videos"
            else 0 if normalized_scope == "folders"
            else min(6, max(1, capped_limit - 2))
        )
        folder_limit = (
            capped_limit
            if normalized_scope == "folders"
            else 0 if normalized_scope == "videos"
            else capped_limit - media_limit
        )
        media_rows: list[sqlite3.Row] = []
        folder_rows: list[sqlite3.Row] = []
        with self.connection() as connection:
            if media_limit:
                media_rows = connection.execute(
                    "SELECT id,name,relative_path,folder FROM media_files "
                    "WHERE valid=1 AND active=1 "
                    "ORDER BY mtime DESC,id DESC LIMIT ?",
                    (media_limit,),
                ).fetchall()
            if folder_limit:
                folder_rows = connection.execute(
                    "SELECT folder,COUNT(*) AS count,MAX(mtime) AS latest "
                    "FROM media_files "
                    "WHERE valid=1 AND active=1 AND folder != '' "
                    "GROUP BY folder "
                    "ORDER BY latest DESC,folder COLLATE NOCASE LIMIT ?",
                    (folder_limit,),
                ).fetchall()

        media_results = [
            {
                "kind": "media",
                "mediaId": int(row["id"]),
                "folderPath": str(row["folder"] or ""),
                "title": str(row["name"]),
                "detail": str(row["relative_path"] or row["name"]),
                "icon": "media",
                "count": 0,
            }
            for row in media_rows
        ]
        folder_results = [
            {
                "kind": "folder",
                "mediaId": 0,
                "folderPath": str(row["folder"]),
                "title": str(row["folder"]).rsplit("/", 1)[-1],
                "detail": str(row["folder"]),
                "icon": "folder",
                "count": int(row["count"]),
            }
            for row in folder_rows
        ]
        return self._merge_command_results(
            media_results,
            folder_results,
            capped_limit,
            normalized_scope,
        )

    def navigation_neighbors(
        self,
        media_id: int,
        search: str = "",
        folder: str = "",
        sort_mode: str = "newest",
    ) -> dict[str, int | bool]:
        """Return the adjacent active videos in the same order as the library."""
        clauses, values = self._media_filter(search, folder)
        order = self._media_order(sort_mode)
        with self.connection() as connection:
            row = connection.execute(
                "WITH ordered AS ("
                "SELECT id, "
                f"LAG(id) OVER (ORDER BY {order}) AS previous_id, "
                f"LEAD(id) OVER (ORDER BY {order}) AS next_id "
                f"FROM media_files WHERE {' AND '.join(clauses)}"
                ") "
                "SELECT previous_id, next_id FROM ordered WHERE id=?",
                [*values, int(media_id)],
            ).fetchone()
        if not row:
            return {"found": False, "previousId": 0, "nextId": 0}
        return {
            "found": True,
            "previousId": int(row["previous_id"] or 0),
            "nextId": int(row["next_id"] or 0),
        }

    def list_folders(self) -> list[dict[str, Any]]:
        with self.connection() as connection:
            rows = connection.execute(
                "SELECT folder, COUNT(*) AS count FROM media_files "
                "WHERE valid=1 AND active=1 GROUP BY folder ORDER BY folder COLLATE NOCASE"
            ).fetchall()
        return [dict(row) for row in rows]

    def list_random_folders(self) -> list[dict[str, Any]]:
        """Return selectable folders with counts including their descendants."""
        direct_counts = {
            str(row["folder"]): int(row["count"])
            for row in self.list_folders()
        }
        subtree_counts: dict[str, int] = {}
        for folder, count in direct_counts.items():
            if not folder:
                continue
            parts = folder.split("/")
            for depth in range(1, len(parts) + 1):
                ancestor = "/".join(parts[:depth])
                subtree_counts[ancestor] = subtree_counts.get(ancestor, 0) + count

        folders: list[dict[str, Any]] = []
        root_count = direct_counts.get("", 0)
        if root_count:
            folders.append({
                "folder": "",
                "count": root_count,
                "direct_count": root_count,
            })
        folders.extend(
            {
                "folder": folder,
                "count": count,
                "direct_count": direct_counts.get(folder, 0),
            }
            for folder, count in sorted(
                subtree_counts.items(),
                key=lambda item: item[0].casefold(),
            )
        )
        return folders

    def list_explorer_folders(self) -> list[dict[str, Any]]:
        """Return folder-tree aggregates used by the Explorer.

        Counts and modification times include every descendant so a parent
        folder sorts according to the videos visible when that branch is
        selected, rather than only files placed directly inside it.
        """
        with self.connection() as connection:
            rows = connection.execute(
                "SELECT folder, COUNT(*) AS count, MAX(mtime) AS latest_mtime, "
                "MAX(indexed_at) AS latest_indexed "
                "FROM media_files WHERE valid=1 AND active=1 "
                "GROUP BY folder"
            ).fetchall()

        aggregates: dict[str, dict[str, int | float | str]] = {}
        for row in rows:
            folder = str(row["folder"])
            if not folder:
                continue
            count = int(row["count"])
            latest_mtime = float(row["latest_mtime"] or 0)
            latest_indexed = str(row["latest_indexed"] or "")
            parts = folder.split("/")
            for depth in range(1, len(parts) + 1):
                ancestor = "/".join(parts[:depth])
                aggregate = aggregates.setdefault(
                    ancestor,
                    {
                        "count": 0,
                        "latest_mtime": 0.0,
                        "latest_indexed": "",
                    },
                )
                aggregate["count"] = int(aggregate["count"]) + count
                aggregate["latest_mtime"] = max(
                    float(aggregate["latest_mtime"]),
                    latest_mtime,
                )
                aggregate["latest_indexed"] = max(
                    str(aggregate["latest_indexed"]),
                    latest_indexed,
                )

        return [
            {
                "folder": folder,
                "count": int(values["count"]),
                "latest_mtime": float(values["latest_mtime"]),
                "latest_indexed": str(values["latest_indexed"]),
            }
            for folder, values in sorted(
                aggregates.items(),
                key=lambda item: item[0].casefold(),
            )
        ]

    @staticmethod
    def _folder_scope(
        folders: Iterable[str] | None,
    ) -> tuple[str, list[str]]:
        normalized = list(dict.fromkeys(str(folder) for folder in (folders or [])))
        if not normalized:
            return "", []

        clauses: list[str] = []
        values: list[str] = []
        for folder in normalized:
            if folder.startswith(EXACT_FOLDER_SCOPE_PREFIX):
                clauses.append("folder=?")
                values.append(folder[len(EXACT_FOLDER_SCOPE_PREFIX):])
                continue
            if not folder:
                clauses.append("folder=''")
                continue
            escaped = (
                folder.replace("\\", "\\\\")
                .replace("%", "\\%")
                .replace("_", "\\_")
            )
            clauses.append("(folder=? OR folder LIKE ? ESCAPE '\\')")
            values.extend([folder, f"{escaped}/%"])
        return f" AND ({' OR '.join(clauses)})", values

    def random_media(
        self,
        avoid_seen: bool = True,
        require_checked: bool = False,
        folders: Iterable[str] | None = None,
    ) -> dict[str, Any] | None:
        with self.connection() as connection:
            checked = " AND duration>0" if require_checked else ""
            folder_scope, folder_values = self._folder_scope(folders)
            where = (
                f"valid=1 AND active=1 AND seen=0{checked}{folder_scope}"
                if avoid_seen
                else f"valid=1 AND active=1{checked}{folder_scope}"
            )
            count = int(
                connection.execute(
                    f"SELECT COUNT(*) AS c FROM media_files WHERE {where}",
                    folder_values,
                ).fetchone()["c"]
            )
            if count == 0 and avoid_seen:
                connection.execute(
                    f"UPDATE media_files SET seen=0 "
                    f"WHERE valid=1 AND active=1{checked}{folder_scope}",
                    folder_values,
                )
                where = f"valid=1 AND active=1{checked}{folder_scope}"
                count = int(
                    connection.execute(
                        f"SELECT COUNT(*) AS c FROM media_files WHERE {where}",
                        folder_values,
                    ).fetchone()["c"]
                )
            if count == 0:
                return None
            offset = random.randrange(count)
            chosen = connection.execute(
                f"SELECT id FROM media_files WHERE {where} "
                "ORDER BY id LIMIT 1 OFFSET ?",
                [*folder_values, offset],
            ).fetchone()
            if not chosen:
                return None
            media_id = int(chosen["id"])
            connection.execute("UPDATE media_files SET seen=1 WHERE id=?", (media_id,))
            row = connection.execute("SELECT * FROM media_files WHERE id=?", (media_id,)).fetchone()
        return dict(row)

    def seen_paths(self, root_path: str | Path) -> set[str]:
        with self.connection() as connection:
            rows = connection.execute(
                "SELECT path FROM media_files WHERE root_path=? AND valid=1 AND seen=1",
                (str(root_path),),
            ).fetchall()
        return {str(row["path"]) for row in rows}

    def mark_seen(self, media_id: int) -> None:
        with self.connection() as connection:
            connection.execute("UPDATE media_files SET seen=1 WHERE id=?", (media_id,))

    def reset_shuffle(
        self,
        root_path: str | Path | None = None,
        folders: Iterable[str] | None = None,
    ) -> None:
        with self.connection() as connection:
            folder_scope, folder_values = self._folder_scope(folders)
            if root_path:
                connection.execute(
                    f"UPDATE media_files SET seen=0 "
                    f"WHERE root_path=?{folder_scope}",
                    [str(root_path), *folder_values],
                )
            else:
                connection.execute(
                    f"UPDATE media_files SET seen=0 WHERE active=1{folder_scope}",
                    folder_values,
                )

    def create_export(self, values: dict[str, Any]) -> int:
        payload = {
            "media_id": values["media_id"],
            "path": str(values["path"]),
            "created_at": utc_now(),
            "trim_start": float(values.get("trim_start", 0)),
            "trim_end": float(values.get("trim_end", 0)),
            "preset": values.get("preset", "original"),
            "target_mb": float(values.get("target_mb", 0)),
            "size_bytes": int(values.get("size_bytes", 0)),
            "duration": float(values.get("duration", 0)),
            "is_generated": int(bool(values.get("is_generated", True))),
            "edit_spec": json.dumps(values.get("edit_spec") or {}, ensure_ascii=False),
            "cleanup_state": values.get("cleanup_state", "kept"),
        }
        with self.connection() as connection:
            cursor = connection.execute(
                """INSERT INTO exports(media_id,path,created_at,trim_start,trim_end,preset,target_mb,
                   size_bytes,duration,is_generated,edit_spec,cleanup_state)
                   VALUES(:media_id,:path,:created_at,:trim_start,:trim_end,:preset,:target_mb,
                   :size_bytes,:duration,:is_generated,:edit_spec,:cleanup_state)""",
                payload,
            )
        return int(cursor.lastrowid)

    def create_post(self, values: dict[str, Any]) -> int:
        now = utc_now()
        payload = {
            "media_id": values["media_id"],
            "export_id": values.get("export_id"),
            "created_at": now,
            "updated_at": now,
            "telegram_enabled": int(bool(values.get("telegram_enabled"))),
            "x_enabled": int(bool(values.get("x_enabled"))),
            "telegram_caption": values.get("telegram_caption", ""),
            "x_caption": values.get("x_caption", ""),
            "telegram_mode": values.get("telegram_mode", "bot"),
            "telegram_destination": values.get("telegram_destination", ""),
            "telegram_status": "queued" if values.get("telegram_enabled") else "not_requested",
            "x_status": "queued" if values.get("x_enabled") else "not_requested",
            "cleanup_policy": values.get("cleanup_policy", "keep"),
        }
        with self.connection() as connection:
            cursor = connection.execute(
                """INSERT INTO posts(media_id,export_id,created_at,updated_at,telegram_enabled,x_enabled,
                   telegram_caption,x_caption,telegram_mode,telegram_destination,telegram_status,x_status,
                   cleanup_policy)
                   VALUES(:media_id,:export_id,:created_at,:updated_at,:telegram_enabled,:x_enabled,
                   :telegram_caption,:x_caption,:telegram_mode,:telegram_destination,:telegram_status,
                   :x_status,:cleanup_policy)""",
                payload,
            )
        return int(cursor.lastrowid)

    def update_post(self, post_id: int, **values: Any) -> None:
        allowed = {
            "export_id", "telegram_status", "telegram_message_id", "telegram_message_link",
            "x_status", "x_url", "error", "cleanup_policy",
        }
        updates = {key: value for key, value in values.items() if key in allowed}
        if not updates:
            return
        updates["updated_at"] = utc_now()
        assignments = ", ".join(f"{key}=?" for key in updates)
        with self.connection() as connection:
            connection.execute(
                f"UPDATE posts SET {assignments} WHERE id=?", [*updates.values(), post_id]
            )

    def add_attempt(
        self,
        post_id: int,
        platform: str,
        status: str,
        detail: str = "",
        remote_id: str = "",
        finished: bool = True,
    ) -> int:
        with self.connection() as connection:
            cursor = connection.execute(
                """INSERT INTO delivery_attempts(post_id,platform,status,started_at,finished_at,detail,remote_id)
                   VALUES(?,?,?,?,?,?,?)""",
                (post_id, platform, status, utc_now(), utc_now() if finished else None, detail, remote_id),
            )
        return int(cursor.lastrowid)

    def get_post(self, post_id: int) -> dict[str, Any] | None:
        with self.connection() as connection:
            row = connection.execute(
                """SELECT posts.*, media_files.path AS source_path, media_files.name AS media_name,
                   media_files.thumbnail_path, media_files.duration AS source_duration,
                   exports.path AS export_path, exports.is_generated, exports.edit_spec,
                   exports.cleanup_state
                   FROM posts JOIN media_files ON media_files.id=posts.media_id
                   LEFT JOIN exports ON exports.id=posts.export_id WHERE posts.id=?""",
                (post_id,),
            ).fetchone()
        return dict(row) if row else None

    def list_history(
        self,
        search: str = "",
        limit: int = 1000,
        offset: int = 0,
    ) -> list[dict[str, Any]]:
        values: list[Any] = []
        clause = ""
        if search.strip():
            clause = "WHERE media_files.name LIKE ? OR posts.telegram_caption LIKE ? OR posts.x_caption LIKE ?"
            term = f"%{search.strip()}%"
            values.extend([term, term, term])
        values.extend([limit, max(0, offset)])
        with self.connection() as connection:
            rows = connection.execute(
                f"""SELECT posts.*, media_files.name AS media_name, media_files.path AS source_path,
                    media_files.thumbnail_path, exports.path AS export_path, exports.is_generated,
                    exports.edit_spec, exports.cleanup_state, exports.size_bytes AS export_size
                    FROM posts JOIN media_files ON media_files.id=posts.media_id
                    LEFT JOIN exports ON exports.id=posts.export_id {clause}
                    ORDER BY posts.created_at DESC LIMIT ? OFFSET ?""",
                values,
            ).fetchall()
        return [dict(row) for row in rows]

    def mark_export_cleanup(self, export_id: int, state: str) -> None:
        with self.connection() as connection:
            connection.execute("UPDATE exports SET cleanup_state=? WHERE id=?", (state, export_id))

    def increment_posted(self, media_id: int) -> None:
        with self.connection() as connection:
            connection.execute(
                "UPDATE media_files SET posted_count=posted_count+1 WHERE id=?", (media_id,)
            )

    def counts(self) -> dict[str, int]:
        with self.connection() as connection:
            media = connection.execute(
                "SELECT COUNT(*) AS c FROM media_files WHERE valid=1 AND active=1"
            ).fetchone()["c"]
            posts = connection.execute("SELECT COUNT(*) AS c FROM posts").fetchone()["c"]
            unseen = connection.execute(
                "SELECT COUNT(*) AS c FROM media_files WHERE valid=1 AND active=1 AND seen=0"
            ).fetchone()["c"]
        return {"media": int(media), "posts": int(posts), "unseen": int(unseen)}
