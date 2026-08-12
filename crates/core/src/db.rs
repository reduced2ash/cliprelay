//! SQLite persistence layer, ported 1:1 from `database.py`.

use crate::utils::utc_now;
use anyhow::{Context, Result};
use rusqlite::{Connection, Row};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::Mutex;

/// Prefix marking an *exact* folder scope in `random_media` folder lists
/// (`\x1e` — unit separator, matching the Python port).
pub const EXACT_FOLDER_SCOPE_PREFIX: &str = "\u{1e}";

const SCHEMA: &str = r#"
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
"#;

#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    pub root_path: String,
    pub path: String,
    pub name: String,
    pub relative_path: String,
    pub folder: String,
    pub duration: f64,
    pub width: i64,
    pub height: i64,
    pub size_bytes: i64,
    pub video_codec: String,
    pub audio_codec: String,
    pub frame_rate: f64,
    pub mtime: f64,
}

#[derive(Debug, Clone)]
pub struct ManifestEntry {
    pub root_path: String,
    pub path: String,
    pub name: String,
    pub relative_path: String,
    pub folder: String,
    pub size_bytes: i64,
    pub mtime: f64,
}

#[derive(Debug, Clone, Default)]
pub struct MediaRow {
    pub id: i64,
    pub root_path: String,
    pub path: String,
    pub name: String,
    pub relative_path: String,
    pub folder: String,
    pub duration: f64,
    pub width: i64,
    pub height: i64,
    pub size_bytes: i64,
    pub video_codec: String,
    pub audio_codec: String,
    pub frame_rate: f64,
    pub mtime: f64,
    pub thumbnail_path: Option<String>,
    pub preview_path: Option<String>,
    pub timeline_path: Option<String>,
    pub active: bool,
    pub valid: bool,
    pub seen: bool,
    pub posted_count: i64,
    pub indexed_at: String,
    pub updated_at: String,
}

impl MediaRow {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            root_path: row.get("root_path")?,
            path: row.get("path")?,
            name: row.get("name")?,
            relative_path: row.get("relative_path")?,
            folder: row.get("folder")?,
            duration: row.get("duration")?,
            width: row.get("width")?,
            height: row.get("height")?,
            size_bytes: row.get("size_bytes")?,
            video_codec: row.get("video_codec")?,
            audio_codec: row.get("audio_codec")?,
            frame_rate: row.get("frame_rate")?,
            mtime: row.get("mtime")?,
            thumbnail_path: row.get("thumbnail_path")?,
            preview_path: row.get("preview_path")?,
            timeline_path: row.get("timeline_path")?,
            active: bool_from_int(row.get("active")?),
            valid: bool_from_int(row.get("valid")?),
            seen: bool_from_int(row.get("seen")?),
            posted_count: row.get("posted_count")?,
            indexed_at: row.get("indexed_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub kind: &'static str,
    pub media_id: i64,
    pub folder_path: String,
    pub title: String,
    pub detail: String,
    pub icon: &'static str,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct FolderCount {
    pub folder: String,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct RandomFolder {
    pub folder: String,
    pub count: i64,
    pub direct_count: i64,
}

#[derive(Debug, Clone)]
pub struct ExplorerFolder {
    pub folder: String,
    pub count: i64,
    pub latest_mtime: f64,
    pub latest_indexed: String,
}

#[derive(Debug, Clone)]
pub struct ExportRow {
    pub id: i64,
    pub media_id: i64,
    pub path: String,
    pub created_at: String,
    pub trim_start: f64,
    pub trim_end: f64,
    pub preset: String,
    pub target_mb: f64,
    pub size_bytes: i64,
    pub duration: f64,
    pub is_generated: bool,
    pub edit_spec: String,
    pub cleanup_state: String,
}

#[derive(Debug, Clone)]
pub struct ExportValues {
    pub media_id: i64,
    pub path: String,
    pub trim_start: f64,
    pub trim_end: f64,
    pub preset: String,
    pub target_mb: f64,
    pub size_bytes: i64,
    pub duration: f64,
    pub is_generated: bool,
    pub edit_spec: serde_json::Value,
    pub cleanup_state: String,
}

#[derive(Debug, Clone, Default)]
pub struct PostRow {
    pub id: i64,
    pub media_id: i64,
    pub export_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub telegram_enabled: bool,
    pub x_enabled: bool,
    pub telegram_caption: String,
    pub x_caption: String,
    pub telegram_mode: String,
    pub telegram_destination: String,
    pub telegram_status: String,
    pub telegram_message_id: String,
    pub telegram_message_link: String,
    pub x_status: String,
    pub x_url: String,
    pub cleanup_policy: String,
    pub error: String,
    // Joined fields:
    pub source_path: Option<String>,
    pub media_name: Option<String>,
    pub thumbnail_path: Option<String>,
    pub source_duration: Option<f64>,
    pub export_path: Option<String>,
    pub is_generated: Option<bool>,
    pub edit_spec: Option<String>,
    pub cleanup_state: Option<String>,
    pub export_size: Option<i64>,
}

impl PostRow {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            media_id: row.get("media_id")?,
            export_id: row.get("export_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            telegram_enabled: bool_from_int(row.get("telegram_enabled")?),
            x_enabled: bool_from_int(row.get("x_enabled")?),
            telegram_caption: row.get("telegram_caption")?,
            x_caption: row.get("x_caption")?,
            telegram_mode: row.get("telegram_mode")?,
            telegram_destination: row.get("telegram_destination")?,
            telegram_status: row.get("telegram_status")?,
            telegram_message_id: row.get("telegram_message_id")?,
            telegram_message_link: row.get("telegram_message_link")?,
            x_status: row.get("x_status")?,
            x_url: row.get("x_url")?,
            cleanup_policy: row.get("cleanup_policy")?,
            error: row.get("error")?,
            source_path: row.get("source_path").ok().flatten(),
            media_name: row.get("media_name").ok().flatten(),
            thumbnail_path: row.get("thumbnail_path").ok().flatten(),
            source_duration: row.get("source_duration").ok().flatten(),
            export_path: row.get("export_path").ok().flatten(),
            is_generated: row.get("is_generated").ok().flatten().map(bool_from_int),
            edit_spec: row.get("edit_spec").ok().flatten(),
            cleanup_state: row.get("cleanup_state").ok().flatten(),
            export_size: row.get("export_size").ok().flatten(),
        })
    }
}

fn bool_from_int(value: i64) -> bool {
    value != 0
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn folder_scope(folders: &[String]) -> (String, Vec<String>) {
    let mut seen = Vec::new();
    for folder in folders {
        if !seen.contains(folder) {
            seen.push(folder.clone());
        }
    }
    if seen.is_empty() {
        return (String::new(), Vec::new());
    }
    let mut clauses: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();
    for folder in seen {
        if let Some(exact) = folder.strip_prefix(EXACT_FOLDER_SCOPE_PREFIX) {
            clauses.push("folder=?".to_string());
            values.push(exact.to_string());
            continue;
        }
        if folder.is_empty() {
            clauses.push("folder=''".to_string());
            continue;
        }
        let escaped = escape_like(&folder);
        clauses.push("(folder=? OR folder LIKE ? ESCAPE '\\')".to_string());
        values.push(folder.clone());
        values.push(format!("{escaped}/%"));
    }
    (format!(" AND ({})", clauses.join(" OR ")), values)
}

fn media_order(sort_mode: &str) -> &'static str {
    match sort_mode {
        "name" => "name COLLATE NOCASE ASC, id ASC",
        "oldest" => "mtime ASC, id ASC",
        "duration" => "duration DESC, id ASC",
        "size" => "size_bytes DESC, id ASC",
        "newest" => "mtime DESC, id DESC",
        _ => "mtime DESC, id DESC",
    }
}

fn media_filter(search: &str, folder: &str) -> (Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut clauses: Vec<String> = vec!["valid=1".into(), "active=1".into()];
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if !search.trim().is_empty() {
        clauses.push("(name LIKE ? OR relative_path LIKE ?)".into());
        let term = format!("%{}%", search.trim());
        values.push(Box::new(term.clone()));
        values.push(Box::new(term));
    }
    if !folder.is_empty() {
        let escaped = escape_like(folder);
        clauses.push("(folder=? OR folder LIKE ? ESCAPE '\\')".into());
        values.push(Box::new(folder.to_string()));
        values.push(Box::new(format!("{escaped}/%")));
    }
    (clauses, values)
}

fn command_scope(value: &str) -> &'static str {
    let scope = value.trim().to_ascii_lowercase();
    match scope.as_str() {
        "all" | "videos" | "folders" => {
            if scope == "videos" { "videos" } else if scope == "folders" { "folders" } else { "all" }
        }
        _ => "all",
    }
}

/// Merge media + folder search results with the Python port's exact
/// budget rules.
fn merge_command_results(
    media_results: Vec<CommandResult>,
    folder_results: Vec<CommandResult>,
    limit: usize,
    scope: &str,
) -> Vec<CommandResult> {
    if scope == "videos" {
        return media_results.into_iter().take(limit).collect();
    }
    if scope == "folders" {
        return folder_results.into_iter().take(limit).collect();
    }
    if limit == 1 {
        if !media_results.is_empty() {
            return media_results.into_iter().take(1).collect();
        }
        return folder_results.into_iter().take(1).collect();
    }
    let reserved_folders = (limit / 3).clamp(1, 3);
    let folder_count = folder_results.len().min(reserved_folders);
    let media_count = media_results.len().min(limit - folder_count);
    let mut results: Vec<CommandResult> = Vec::new();
    results.extend(media_results.iter().take(media_count).cloned());
    results.extend(folder_results.iter().take(folder_count).cloned());
    let mut remaining = limit.saturating_sub(results.len());
    if remaining > 0 {
        let extra = media_results[media_count..]
            .iter()
            .take(remaining)
            .cloned()
            .collect::<Vec<_>>();
        let extra_count = extra.len();
        results.extend(extra);
        remaining = remaining.saturating_sub(extra_count);
    }
    if remaining > 0 {
        results.extend(
            folder_results[folder_count..]
                .iter()
                .take(remaining)
                .cloned(),
        );
    }
    results
}

#[derive(Debug, Clone)]
pub struct MediaState {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub relative_path: String,
    pub folder: String,
    pub size_bytes: i64,
    pub mtime: f64,
    pub duration: f64,
    pub valid: bool,
    pub thumbnail_path: Option<String>,
    pub preview_path: Option<String>,
    pub timeline_path: Option<String>,
}

pub struct Database {
    conn: Mutex<Connection>,
    active_root: Mutex<Option<String>>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating database directory {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening database {}", path.display()))?;
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        conn.execute_batch(SCHEMA)?;
        // Migration parity with the Python app: columns that older databases
        // may lack.
        let columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(media_files)")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
            rows.collect::<Result<_, _>>()?
        };
        if !columns.iter().any(|c| c == "active") {
            conn.execute_batch("ALTER TABLE media_files ADD COLUMN active INTEGER NOT NULL DEFAULT 1")?;
        }
        if !columns.iter().any(|c| c == "timeline_path") {
            conn.execute_batch("ALTER TABLE media_files ADD COLUMN timeline_path TEXT")?;
        }
        let export_columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(exports)")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
            rows.collect::<Result<_, _>>()?
        };
        if !export_columns.iter().any(|c| c == "edit_spec") {
            conn.execute_batch("ALTER TABLE exports ADD COLUMN edit_spec TEXT NOT NULL DEFAULT '{}'")?;
        }
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_media_active_valid_seen \
             ON media_files(active, valid, seen)",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
            active_root: Mutex::new(None),
        })
    }

    // ---- settings -----------------------------------------------------

    pub fn get_setting_raw(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query([key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn get_settings(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    // ---- media manifest ----------------------------------------------

    pub fn upsert_media(&self, metadata: &MediaMetadata) -> Result<i64> {
        let now = utc_now();
        let active_root = self.active_root.lock().clone();
        let active = active_root.is_none() || Some(&metadata.root_path) == active_root.as_ref();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO media_files(root_path,path,name,relative_path,folder,duration,width,height,\
             size_bytes,video_codec,audio_codec,frame_rate,mtime,active,indexed_at,updated_at) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16) \
             ON CONFLICT(path) DO UPDATE SET \
             root_path=excluded.root_path, name=excluded.name, \
             relative_path=excluded.relative_path, folder=excluded.folder, \
             duration=excluded.duration, width=excluded.width, height=excluded.height, \
             size_bytes=excluded.size_bytes, video_codec=excluded.video_codec, \
             audio_codec=excluded.audio_codec, frame_rate=excluded.frame_rate, \
             mtime=excluded.mtime, active=excluded.active, updated_at=excluded.updated_at, \
             valid=1",
            rusqlite::params![
                metadata.root_path,
                metadata.path,
                metadata.name,
                metadata.relative_path,
                metadata.folder,
                metadata.duration,
                metadata.width,
                metadata.height,
                metadata.size_bytes,
                metadata.video_codec,
                metadata.audio_codec,
                metadata.frame_rate,
                metadata.mtime,
                active as i64,
                now,
                now,
            ],
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM media_files WHERE path = ?1",
            [&metadata.path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Batch upsert of manifest entries; returns the number of rows that
    /// actually changed (used to decide whether a re-probe is needed).
    pub fn upsert_manifest_batch(&self, entries: &[ManifestEntry]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }
        let now = utc_now();
        let active_root = self.active_root.lock().clone();
        let conn = self.conn.lock();
        let before = conn.total_changes() as i64;
        let mut stmt = conn.prepare(
            "INSERT INTO media_files(root_path,path,name,relative_path,folder,size_bytes,mtime,\
             active,indexed_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) \
             ON CONFLICT(path) DO UPDATE SET \
             root_path=excluded.root_path, name=excluded.name, \
             relative_path=excluded.relative_path, folder=excluded.folder, \
             duration=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN 0 ELSE media_files.duration END, \
             width=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN 0 ELSE media_files.width END, \
             height=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN 0 ELSE media_files.height END, \
             video_codec=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN '' ELSE media_files.video_codec END, \
             audio_codec=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN '' ELSE media_files.audio_codec END, \
             frame_rate=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN 0 ELSE media_files.frame_rate END, \
             thumbnail_path=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN NULL ELSE media_files.thumbnail_path END, \
             preview_path=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN NULL ELSE media_files.preview_path END, \
             timeline_path=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN NULL ELSE media_files.timeline_path END, \
             seen=CASE WHEN media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 THEN 0 ELSE media_files.seen END, \
             size_bytes=excluded.size_bytes, mtime=excluded.mtime, \
             active=excluded.active, valid=1, updated_at=excluded.updated_at \
             WHERE media_files.root_path != excluded.root_path \
             OR media_files.name != excluded.name \
             OR media_files.relative_path != excluded.relative_path \
             OR media_files.folder != excluded.folder \
             OR media_files.size_bytes != excluded.size_bytes \
             OR ABS(media_files.mtime - excluded.mtime) > 0.001 \
             OR media_files.valid != 1 OR media_files.active != excluded.active",
        )?;
        for entry in entries {
            let active = active_root.is_none() || Some(&entry.root_path) == active_root.as_ref();
            stmt.execute(rusqlite::params![
                entry.root_path,
                entry.path,
                entry.name,
                entry.relative_path,
                entry.folder,
                entry.size_bytes,
                entry.mtime,
                active as i64,
                now,
                now,
            ])?;
        }
        let changed = conn.total_changes() as i64 - before;
        Ok(changed as usize)
    }

    /// Mark rows whose path is not in `existing_paths` as invalid.
    pub fn invalidate_absent(&self, root_path: &str, existing_paths: &[String]) -> Result<usize> {
        let present: std::collections::HashSet<&str> = existing_paths.iter().map(String::as_str).collect();
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, path FROM media_files WHERE root_path=?1 AND valid=1")?;
        let rows = stmt
            .query_map([root_path], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        let missing: Vec<i64> = rows
            .into_iter()
            .filter(|(_, path)| !present.contains(path.as_str()))
            .map(|(id, _)| id)
            .collect();
        if !missing.is_empty() {
            let mut update = conn.prepare("UPDATE media_files SET valid=0 WHERE id=?1")?;
            for id in &missing {
                update.execute([id])?;
            }
        }
        Ok(missing.len())
    }

    pub fn manifest_paths(&self, root_path: &str) -> Result<Vec<PathBuf>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT path FROM media_files WHERE root_path=?1 AND valid=1 ORDER BY path",
        )?;
        let rows = stmt.query_map([root_path], |row| row.get::<_, String>(0))?;
        Ok(rows
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(PathBuf::from)
            .collect())
    }

    pub fn media_state_map(&self, root_path: &str) -> Result<HashMap<String, MediaState>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id,path,name,relative_path,folder,size_bytes,mtime,duration,valid,\
             thumbnail_path,preview_path,timeline_path FROM media_files WHERE root_path=?1",
        )?;
        let rows = stmt.query_map([root_path], |row| {
            Ok((
                row.get::<_, String>("path")?,
                MediaState {
                    id: row.get("id")?,
                    path: row.get("path")?,
                    name: row.get("name")?,
                    relative_path: row.get("relative_path")?,
                    folder: row.get("folder")?,
                    size_bytes: row.get("size_bytes")?,
                    mtime: row.get("mtime")?,
                    duration: row.get("duration")?,
                    valid: bool_from_int(row.get("valid")?),
                    thumbnail_path: row.get("thumbnail_path")?,
                    preview_path: row.get("preview_path")?,
                    timeline_path: row.get("timeline_path")?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn activate_root(&self, root_path: Option<&str>) -> Result<()> {
        let mut active_root = self.active_root.lock();
        *active_root = root_path.map(normalize_path);
        drop(active_root);
        self.reconcile_active_root()
    }

    pub fn reconcile_active_root(&self) -> Result<()> {
        let active_root = self.active_root.lock().clone();
        let conn = self.conn.lock();
        match active_root {
            None => {
                conn.execute("UPDATE media_files SET active=0 WHERE active!=0", [])?;
            }
            Some(root) => {
                conn.execute(
                    "UPDATE media_files SET active=CASE WHEN root_path=?1 THEN 1 ELSE 0 END \
                     WHERE active!=CASE WHEN root_path=?1 THEN 1 ELSE 0 END",
                    [&root],
                )?;
            }
        }
        Ok(())
    }

    /// Mark every row under `root_path` invalid, then re-validate the given
    /// paths (in chunks of 500).
    pub fn invalidate_missing(&self, root_path: &str, existing_paths: &[String]) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE media_files SET valid=0 WHERE root_path=?1",
            [root_path],
        )?;
        for chunk in existing_paths.chunks(500) {
            let placeholders: Vec<String> = (2..=chunk.len() + 1).map(|i| format!("?{i}")).collect();
            let mut sql = format!(
                "UPDATE media_files SET valid=1 WHERE root_path=?1 AND path IN ({})",
                placeholders.join(",")
            );
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> =
                vec![Box::new(root_path.to_string())];
            params.extend(chunk.iter().map(|p| Box::new(p.clone()) as Box<dyn rusqlite::types::ToSql>));
            let mut stmt = conn.prepare(&mut sql)?;
            stmt.execute(rusqlite::params_from_iter(params.iter()))?;
        }
        Ok(())
    }

    pub fn media_needs_probe(
        &self,
        path: &str,
        size_bytes: i64,
        mtime: f64,
        require_probe: bool,
    ) -> Result<bool> {
        let conn = self.conn.lock();
        let row = conn.query_row(
            "SELECT size_bytes, mtime, duration, valid FROM media_files WHERE path=?1",
            [path],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    bool_from_int(row.get::<_, i64>(3)?),
                ))
            },
        );
        let (size, mtime_db, duration, valid) = match row {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(true),
            Err(e) => return Err(e.into()),
        };
        Ok(!valid
            || size != size_bytes
            || (mtime_db - mtime).abs() > 0.001
            || (require_probe && duration <= 0.0))
    }

    pub fn get_media_by_path(&self, path: &str) -> Result<Option<MediaRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT * FROM media_files WHERE path=?1")?;
        let mut rows = stmt.query([path])?;
        match rows.next()? {
            Some(row) => Ok(Some(MediaRow::from_row(row)?)),
            None => Ok(None),
        }
    }

    pub fn set_media_valid(&self, media_id: i64, valid: bool) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE media_files SET valid=?1, updated_at=?2 WHERE id=?3",
            rusqlite::params![valid as i64, utc_now(), media_id],
        )?;
        Ok(())
    }

    pub fn set_path_valid(&self, path: &str, valid: bool) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE media_files SET valid=?1, updated_at=?2 WHERE path=?3",
            rusqlite::params![valid as i64, utc_now(), path],
        )?;
        Ok(())
    }

    pub fn set_media_asset(&self, media_id: i64, kind: &str, path: &str) -> Result<()> {
        if !matches!(kind, "thumbnail_path" | "preview_path" | "timeline_path") {
            anyhow::bail!("Unsupported media asset kind");
        }
        let conn = self.conn.lock();
        conn.execute(
            &format!("UPDATE media_files SET {kind}=?1, updated_at=?2 WHERE id=?3"),
            rusqlite::params![path, utc_now(), media_id],
        )?;
        Ok(())
    }

    pub fn clear_media_asset(&self, media_id: i64, kind: &str) -> Result<()> {
        if !matches!(kind, "thumbnail_path" | "preview_path" | "timeline_path") {
            anyhow::bail!("Unsupported media asset kind");
        }
        let conn = self.conn.lock();
        conn.execute(
            &format!("UPDATE media_files SET {kind}=NULL, updated_at=?1 WHERE id=?2"),
            rusqlite::params![utc_now(), media_id],
        )?;
        Ok(())
    }

    pub fn get_media(&self, media_id: i64) -> Result<Option<MediaRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT * FROM media_files WHERE id=?1")?;
        let mut rows = stmt.query([media_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(MediaRow::from_row(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_media(
        &self,
        search: &str,
        folder: &str,
        sort_mode: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MediaRow>> {
        let order = media_order(sort_mode);
        let (clauses, mut values) = media_filter(search, folder);
        let conn = self.conn.lock();
        let mut sql = format!(
            "SELECT * FROM media_files WHERE {} ORDER BY {} LIMIT ?{} OFFSET ?{}",
            clauses.join(" AND "),
            order,
            values.len() + 1,
            values.len() + 2
        );
        let mut stmt = conn.prepare(&mut sql)?;
        values.push(Box::new(limit));
        values.push(Box::new(offset.max(0)));
        let rows = stmt.query_map(rusqlite::params_from_iter(values.iter()), |row| {
            MediaRow::from_row(row)
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    // ---- command center ----------------------------------------------

    pub fn search_suggestions(&self, query: &str, limit: usize, scope: &str) -> Result<Vec<CommandResult>> {
        let value = query.trim().to_string();
        let capped = limit.clamp(1, 12);
        let normalized = command_scope(scope);
        if value.is_empty() {
            return Ok(Vec::new());
        }
        let escaped = escape_like(&value);
        let contains = format!("%{escaped}%");
        let prefix = format!("{escaped}%");
        let leaf_exact = format!("%/{escaped}");
        let leaf_prefix = format!("%/{prefix}");
        let conn = self.conn.lock();
        let mut media_rows: Vec<(i64, String, String, String)> = Vec::new();
        if normalized != "folders" {
            let mut stmt = conn.prepare(
                "SELECT id,name,relative_path,folder FROM media_files \
                 WHERE valid=1 AND active=1 \
                 AND (name LIKE ?1 ESCAPE '\\' OR relative_path LIKE ?2 ESCAPE '\\') \
                 ORDER BY CASE WHEN name LIKE ?3 ESCAPE '\\' THEN 0 ELSE 1 END, \
                 name COLLATE NOCASE, id LIMIT ?4",
            )?;
            let mut rows = stmt.query(rusqlite::params![contains, contains, prefix, capped as i64])?;
            while let Some(row) = rows.next()? {
                media_rows.push((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ));
            }
        }
        let mut folder_rows: Vec<(String, i64)> = Vec::new();
        if normalized != "videos" {
            let mut stmt = conn.prepare(
                "SELECT folder,COUNT(*) AS count FROM media_files \
                 WHERE valid=1 AND active=1 AND folder != '' \
                 AND folder LIKE ?1 ESCAPE '\\' \
                 GROUP BY folder \
                 ORDER BY CASE \
                 WHEN folder = ?2 COLLATE NOCASE OR folder LIKE ?3 ESCAPE '\\' THEN 0 \
                 WHEN folder LIKE ?4 ESCAPE '\\' OR folder LIKE ?5 ESCAPE '\\' THEN 1 \
                 ELSE 2 END, folder COLLATE NOCASE LIMIT ?6",
            )?;
            let mut rows = stmt.query(rusqlite::params![
                contains, value, leaf_exact, prefix, leaf_prefix, capped as i64
            ])?;
            while let Some(row) = rows.next()? {
                folder_rows.push((row.get(0)?, row.get(1)?));
            }
        }
        drop(conn);
        let media_results: Vec<CommandResult> = media_rows
            .into_iter()
            .map(|(id, name, relative, folder)| CommandResult {
                kind: "media",
                media_id: id,
                folder_path: folder,
                title: name.clone(),
                detail: if relative.is_empty() { name } else { relative },
                icon: "media",
                count: 0,
            })
            .collect();
        let folder_results: Vec<CommandResult> = folder_rows
            .into_iter()
            .map(|(folder, count)| CommandResult {
                kind: "folder",
                media_id: 0,
                folder_path: folder.clone(),
                title: folder.rsplit('/').next().unwrap_or("").to_string(),
                detail: folder,
                icon: "folder",
                count,
            })
            .collect();
        Ok(merge_command_results(media_results, folder_results, capped, normalized))
    }

    pub fn command_center_overview(&self, limit: usize, scope: &str) -> Result<Vec<CommandResult>> {
        let capped = limit.clamp(3, 12);
        let normalized = command_scope(scope);
        let media_limit: usize = if normalized == "videos" {
            capped
        } else if normalized == "folders" {
            0
        } else {
            (capped.saturating_sub(2)).clamp(1, 6)
        };
        let folder_limit: usize = if normalized == "folders" {
            capped
        } else if normalized == "videos" {
            0
        } else {
            capped - media_limit
        };
        let conn = self.conn.lock();
        let mut media_rows: Vec<(i64, String, String, String)> = Vec::new();
        if media_limit > 0 {
            let mut stmt = conn.prepare(
                "SELECT id,name,relative_path,folder FROM media_files \
                 WHERE valid=1 AND active=1 ORDER BY mtime DESC,id DESC LIMIT ?1",
            )?;
            let mut rows = stmt.query([media_limit as i64])?;
            while let Some(row) = rows.next()? {
                media_rows.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
            }
        }
        let mut folder_rows: Vec<(String, i64)> = Vec::new();
        if folder_limit > 0 {
            let mut stmt = conn.prepare(
                "SELECT folder,COUNT(*) AS count,MAX(mtime) AS latest FROM media_files \
                 WHERE valid=1 AND active=1 AND folder != '' \
                 GROUP BY folder ORDER BY latest DESC,folder COLLATE NOCASE LIMIT ?1",
            )?;
            let mut rows = stmt.query([folder_limit as i64])?;
            while let Some(row) = rows.next()? {
                folder_rows.push((row.get(0)?, row.get(1)?));
            }
        }
        drop(conn);
        let media_results: Vec<CommandResult> = media_rows
            .into_iter()
            .map(|(id, name, relative, folder)| CommandResult {
                kind: "media",
                media_id: id,
                folder_path: folder,
                title: name.clone(),
                detail: if relative.is_empty() { name } else { relative },
                icon: "media",
                count: 0,
            })
            .collect();
        let folder_results: Vec<CommandResult> = folder_rows
            .into_iter()
            .map(|(folder, count)| CommandResult {
                kind: "folder",
                media_id: 0,
                folder_path: folder.clone(),
                title: folder.rsplit('/').next().unwrap_or("").to_string(),
                detail: folder,
                icon: "folder",
                count,
            })
            .collect();
        Ok(merge_command_results(media_results, folder_results, capped, normalized))
    }

    pub fn navigation_neighbors(
        &self,
        media_id: i64,
        search: &str,
        folder: &str,
        sort_mode: &str,
    ) -> Result<(bool, i64, i64)> {
        let order = media_order(sort_mode);
        let (clauses, mut values) = media_filter(search, folder);
        let conn = self.conn.lock();
        let sql = format!(
            "WITH ordered AS (SELECT id, LAG(id) OVER (ORDER BY {order}) AS previous_id, \
             LEAD(id) OVER (ORDER BY {order}) AS next_id \
             FROM media_files WHERE {}) SELECT previous_id, next_id FROM ordered WHERE id=?{}",
            clauses.join(" AND "),
            values.len() + 1
        );
        let mut stmt = conn.prepare(&sql)?;
        values.push(Box::new(media_id));
        let mut rows = stmt.query(rusqlite::params_from_iter(values.iter()))?;
        match rows.next()? {
            Some(row) => Ok((true, row.get::<_, Option<i64>>(0)?.unwrap_or(0), row.get::<_, Option<i64>>(1)?.unwrap_or(0))),
            None => Ok((false, 0, 0)),
        }
    }

    // ---- folders -------------------------------------------------------

    pub fn list_folders(&self) -> Result<Vec<FolderCount>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT folder, COUNT(*) AS count FROM media_files \
             WHERE valid=1 AND active=1 GROUP BY folder ORDER BY folder COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FolderCount {
                folder: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn list_random_folders(&self) -> Result<Vec<RandomFolder>> {
        let direct = self.list_folders()?;
        let mut subtree_counts: HashMap<String, i64> = HashMap::new();
        for folder_count in &direct {
            if folder_count.folder.is_empty() {
                continue;
            }
            let parts: Vec<&str> = folder_count.folder.split('/').collect();
            for depth in 1..=parts.len() {
                let ancestor = parts[..depth].join("/");
                *subtree_counts.entry(ancestor).or_insert(0) += folder_count.count;
            }
        }
        let mut folders: Vec<RandomFolder> = Vec::new();
        let direct_counts: HashMap<&str, i64> = direct
            .iter()
            .map(|f| (f.folder.as_str(), f.count))
            .collect();
        if let Some(root_count) = direct_counts.get("") {
            folders.push(RandomFolder {
                folder: String::new(),
                count: *root_count,
                direct_count: *root_count,
            });
        }
        let mut subtree: Vec<(String, i64)> = subtree_counts.into_iter().collect();
        subtree.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        folders.extend(subtree.into_iter().map(|(folder, count)| RandomFolder {
            direct_count: direct_counts.get(folder.as_str()).copied().unwrap_or(0),
            folder,
            count,
        }));
        Ok(folders)
    }

    pub fn list_explorer_folders(&self) -> Result<Vec<ExplorerFolder>> {
        let rows = {
            let conn = self.conn.lock();
            let mut stmt = conn.prepare(
                "SELECT folder, COUNT(*) AS count, MAX(mtime) AS latest_mtime, \
                 MAX(indexed_at) AS latest_indexed FROM media_files WHERE valid=1 AND active=1 \
                 GROUP BY folder",
            )?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };

        #[derive(Default)]
        struct Aggregate {
            count: i64,
            latest_mtime: f64,
            latest_indexed: String,
        }
        let mut aggregates: HashMap<String, Aggregate> = HashMap::new();
        for (folder, count, latest_mtime, latest_indexed) in rows {
            if folder.is_empty() {
                continue;
            }
            let parts: Vec<&str> = folder.split('/').collect();
            for depth in 1..=parts.len() {
                let ancestor = parts[..depth].join("/");
                let aggregate = aggregates.entry(ancestor).or_default();
                aggregate.count += count;
                aggregate.latest_mtime = aggregate.latest_mtime.max(latest_mtime);
                if latest_indexed > aggregate.latest_indexed {
                    aggregate.latest_indexed.clone_from(&latest_indexed);
                }
            }
        }
        let mut folders: Vec<ExplorerFolder> = aggregates
            .into_iter()
            .map(|(folder, values)| ExplorerFolder {
                folder,
                count: values.count,
                latest_mtime: values.latest_mtime,
                latest_indexed: values.latest_indexed,
            })
            .collect();
        folders.sort_by(|a, b| a.folder.to_lowercase().cmp(&b.folder.to_lowercase()));
        Ok(folders)
    }

    // ---- random --------------------------------------------------------

    pub fn random_media(
        &self,
        avoid_seen: bool,
        require_checked: bool,
        folders: &[String],
    ) -> Result<Option<MediaRow>> {
        let checked = if require_checked { " AND duration>0" } else { "" };
        let (folder_scope, folder_values) = folder_scope(folders);
        let conn = self.conn.lock();
        let mut where_clause = if avoid_seen {
            format!("valid=1 AND active=1 AND seen=0{checked}{folder_scope}")
        } else {
            format!("valid=1 AND active=1{checked}{folder_scope}")
        };
        let mut count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) AS c FROM media_files WHERE {where_clause}"),
            rusqlite::params_from_iter(folder_values.iter()),
            |row| row.get(0),
        )?;
        if count == 0 && avoid_seen {
            conn.execute(
                &format!("UPDATE media_files SET seen=0 WHERE valid=1 AND active=1{checked}{folder_scope}"),
                rusqlite::params_from_iter(folder_values.iter()),
            )?;
            where_clause = format!("valid=1 AND active=1{checked}{folder_scope}");
            count = conn.query_row(
                &format!("SELECT COUNT(*) AS c FROM media_files WHERE {where_clause}"),
                rusqlite::params_from_iter(folder_values.iter()),
                |row| row.get(0),
            )?;
        }
        if count == 0 {
            return Ok(None);
        }
        use rand::Rng;
        let offset: i64 = rand::thread_rng().gen_range(0..count);
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> =
            folder_values.iter().map(|v| Box::new(v.clone()) as Box<dyn rusqlite::types::ToSql>).collect();
        params.push(Box::new(offset));
        let media_id: i64 = conn.query_row(
            &format!("SELECT id FROM media_files WHERE {where_clause} ORDER BY id LIMIT 1 OFFSET ?{}", params.len()),
            rusqlite::params_from_iter(params.iter()),
            |row| row.get(0),
        )?;
        conn.execute("UPDATE media_files SET seen=1 WHERE id=?1", [media_id])?;
        let mut stmt = conn.prepare("SELECT * FROM media_files WHERE id=?1")?;
        let mut rows = stmt.query([media_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(MediaRow::from_row(row)?)),
            None => Ok(None),
        }
    }

    pub fn seen_paths(&self, root_path: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT path FROM media_files WHERE root_path=?1 AND valid=1 AND seen=1",
        )?;
        let rows = stmt.query_map([root_path], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn mark_seen(&self, media_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("UPDATE media_files SET seen=1 WHERE id=?1", [media_id])?;
        Ok(())
    }

    pub fn reset_shuffle(&self, root_path: Option<&str>, folders: &[String]) -> Result<()> {
        let (folder_scope, folder_values) = folder_scope(folders);
        let conn = self.conn.lock();
        if let Some(root) = root_path {
            let sql = format!("UPDATE media_files SET seen=0 WHERE root_path=?{}{folder_scope}", folder_values.len() + 1);
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> =
                folder_values.iter().map(|v| Box::new(v.clone()) as Box<dyn rusqlite::types::ToSql>).collect();
            params.push(Box::new(root.to_string()));
            conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        } else {
            let sql = format!("UPDATE media_files SET seen=0 WHERE active=1{folder_scope}");
            conn.execute(&sql, rusqlite::params_from_iter(folder_values.iter()))?;
        }
        Ok(())
    }

    // ---- exports -------------------------------------------------------

    pub fn create_export(&self, values: &ExportValues) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO exports(media_id,path,created_at,trim_start,trim_end,preset,target_mb,\
             size_bytes,duration,is_generated,edit_spec,cleanup_state) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            rusqlite::params![
                values.media_id,
                values.path,
                utc_now(),
                values.trim_start,
                values.trim_end,
                values.preset,
                values.target_mb,
                values.size_bytes,
                values.duration,
                values.is_generated as i64,
                serde_json::to_string(&values.edit_spec).unwrap_or_else(|_| "{}".to_string()),
                values.cleanup_state,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_export(&self, export_id: i64) -> Result<Option<ExportRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT * FROM exports WHERE id=?1")?;
        let mut rows = stmt.query([export_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(ExportRow {
                id: row.get("id")?,
                media_id: row.get("media_id")?,
                path: row.get("path")?,
                created_at: row.get("created_at")?,
                trim_start: row.get("trim_start")?,
                trim_end: row.get("trim_end")?,
                preset: row.get("preset")?,
                target_mb: row.get("target_mb")?,
                size_bytes: row.get("size_bytes")?,
                duration: row.get("duration")?,
                is_generated: bool_from_int(row.get("is_generated")?),
                edit_spec: row.get("edit_spec")?,
                cleanup_state: row.get("cleanup_state")?,
            })),
            None => Ok(None),
        }
    }

    pub fn mark_export_cleanup(&self, export_id: i64, state: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE exports SET cleanup_state=?1 WHERE id=?2",
            rusqlite::params![state, export_id],
        )?;
        Ok(())
    }

    // ---- posts ----------------------------------------------------------

    pub fn create_post(&self, values: &PostValues) -> Result<i64> {
        let now = utc_now();
        let telegram_status = if values.telegram_enabled { "queued" } else { "not_requested" };
        let x_status = if values.x_enabled { "queued" } else { "not_requested" };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO posts(media_id,export_id,created_at,updated_at,telegram_enabled,x_enabled,\
             telegram_caption,x_caption,telegram_mode,telegram_destination,telegram_status,x_status,\
             cleanup_policy) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            rusqlite::params![
                values.media_id,
                values.export_id,
                now,
                now,
                values.telegram_enabled as i64,
                values.x_enabled as i64,
                values.telegram_caption,
                values.x_caption,
                values.telegram_mode,
                values.telegram_destination,
                telegram_status,
                x_status,
                values.cleanup_policy,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_post(&self, post_id: i64, values: &PostUpdate) -> Result<()> {
        let mut assignments: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        macro_rules! field {
            ($key:literal, $value:expr) => {
                assignments.push(format!("{}=?{}", $key, params.len() + 1));
                params.push(Box::new($value));
            };
        }
        if let Some(v) = &values.export_id {
            field!("export_id", v);
        }
        if let Some(v) = &values.telegram_status {
            field!("telegram_status", v);
        }
        if let Some(v) = &values.telegram_message_id {
            field!("telegram_message_id", v);
        }
        if let Some(v) = &values.telegram_message_link {
            field!("telegram_message_link", v);
        }
        if let Some(v) = &values.x_status {
            field!("x_status", v);
        }
        if let Some(v) = &values.x_url {
            field!("x_url", v);
        }
        if let Some(v) = &values.error {
            field!("error", v);
        }
        if let Some(v) = &values.cleanup_policy {
            field!("cleanup_policy", v);
        }
        if assignments.is_empty() {
            return Ok(());
        }
        assignments.push(format!("updated_at=?{}", params.len() + 1));
        params.push(Box::new(utc_now()));
        params.push(Box::new(post_id));
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&format!(
            "UPDATE posts SET {} WHERE id=?{}",
            assignments.join(", "),
            params.len()
        ))?;
        stmt.execute(rusqlite::params_from_iter(params.iter()))?;
        Ok(())
    }

    pub fn add_attempt(
        &self,
        post_id: i64,
        platform: &str,
        status: &str,
        detail: &str,
        remote_id: &str,
        finished: bool,
    ) -> Result<i64> {
        let now = utc_now();
        let finished_at: Option<String> = if finished { Some(now.clone()) } else { None };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO delivery_attempts(post_id,platform,status,started_at,finished_at,detail,remote_id) \
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![post_id, platform, status, now, finished_at, detail, remote_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_post(&self, post_id: i64) -> Result<Option<PostRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT posts.*, media_files.path AS source_path, media_files.name AS media_name, \
             media_files.thumbnail_path, media_files.duration AS source_duration, \
             exports.path AS export_path, exports.is_generated, exports.edit_spec, \
             exports.cleanup_state FROM posts JOIN media_files ON media_files.id=posts.media_id \
             LEFT JOIN exports ON exports.id=posts.export_id WHERE posts.id=?1",
        )?;
        let mut rows = stmt.query([post_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(PostRow::from_row(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_history(&self, search: &str, limit: i64, offset: i64) -> Result<Vec<PostRow>> {
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut clause = String::new();
        if !search.trim().is_empty() {
            clause = "WHERE media_files.name LIKE ?1 ESCAPE '\\' OR posts.telegram_caption LIKE ?2 ESCAPE '\\' OR posts.x_caption LIKE ?3 ESCAPE '\\'".to_string();
            let escaped = escape_like(search.trim());
            let term = format!("%{escaped}%");
            values.push(Box::new(term.clone()));
            values.push(Box::new(term.clone()));
            values.push(Box::new(term));
        }
        let conn = self.conn.lock();
        let mut sql = format!(
            "SELECT posts.*, media_files.name AS media_name, media_files.path AS source_path, \
             media_files.thumbnail_path, exports.path AS export_path, exports.is_generated, \
             exports.edit_spec, exports.cleanup_state, exports.size_bytes AS export_size \
             FROM posts JOIN media_files ON media_files.id=posts.media_id \
             LEFT JOIN exports ON exports.id=posts.export_id {clause} \
             ORDER BY posts.created_at DESC LIMIT ?{} OFFSET ?{}",
            values.len() + 1,
            values.len() + 2
        );
        let mut stmt = conn.prepare(&mut sql)?;
        values.push(Box::new(limit));
        values.push(Box::new(offset.max(0)));
        let rows = stmt.query_map(rusqlite::params_from_iter(values.iter()), |row| {
            PostRow::from_row(row)
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn increment_posted(&self, media_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE media_files SET posted_count=posted_count+1 WHERE id=?1",
            [media_id],
        )?;
        Ok(())
    }

    pub fn counts(&self) -> Result<(i64, i64, i64)> {
        let conn = self.conn.lock();
        let media: i64 = conn.query_row(
            "SELECT COUNT(*) AS c FROM media_files WHERE valid=1 AND active=1",
            [],
            |row| row.get(0),
        )?;
        let posts: i64 = conn.query_row("SELECT COUNT(*) AS c FROM posts", [], |row| row.get(0))?;
        let unseen: i64 = conn.query_row(
            "SELECT COUNT(*) AS c FROM media_files WHERE valid=1 AND active=1 AND seen=0",
            [],
            |row| row.get(0),
        )?;
        Ok((media, posts, unseen))
    }
}

#[derive(Debug, Clone, Default)]
pub struct PostValues {
    pub media_id: i64,
    pub export_id: Option<i64>,
    pub telegram_enabled: bool,
    pub x_enabled: bool,
    pub telegram_caption: String,
    pub x_caption: String,
    pub telegram_mode: String,
    pub telegram_destination: String,
    pub cleanup_policy: String,
}

#[derive(Debug, Clone, Default)]
pub struct PostUpdate {
    pub export_id: Option<i64>,
    pub telegram_status: Option<String>,
    pub telegram_message_id: Option<String>,
    pub telegram_message_link: Option<String>,
    pub x_status: Option<String>,
    pub x_url: Option<String>,
    pub error: Option<String>,
    pub cleanup_policy: Option<String>,
}

fn normalize_path(path: &str) -> String {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    // Resolve symlinks like the scanner does (/tmp -> /private/tmp on
    // macOS); otherwise the active-root reconciliation and the row-level
    // root comparisons never match the stored (resolved) root_path.
    absolute
        .canonicalize()
        .unwrap_or(absolute)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_tmp() -> Database {
        let dir = tempfile::tempdir().unwrap();
        Database::open(dir.path().join("test.sqlite3")).unwrap()
    }

    fn entry(root: &str, path: &str, folder: &str) -> ManifestEntry {
        ManifestEntry {
            root_path: root.to_string(),
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap().to_string(),
            relative_path: path.to_string(),
            folder: folder.to_string(),
            size_bytes: 100,
            mtime: 1.0,
        }
    }

    #[test]
    fn command_center_search_scopes_and_ranking() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries = vec![
            entry(root, "/tmp/root/clip one.mp4", ""),
            entry(root, "/tmp/root/sub/deep clip.mp4", "sub"),
            entry(root, "/tmp/root/other.mp4", ""),
        ];
        db.upsert_manifest_batch(&entries).unwrap();
        db.activate_root(Some(root)).unwrap();

        // Video scope: matches the name and prefers the prefix match.
        let videos = db.search_suggestions("clip", 12, "videos").unwrap();
        let names: Vec<String> = videos.iter().filter(|r| r.kind == "media").map(|r| r.title.clone()).collect();
        assert!(names.iter().any(|n| n == "clip one.mp4"), "got {names:?}");
        assert!(names.iter().any(|n| n == "deep clip.mp4"), "got {names:?}");
        let first = videos.iter().find(|r| r.kind == "media").unwrap();
        assert_eq!(first.title, "clip one.mp4", "prefix match ranks first");

        // Folder scope: the nested folder is found by leaf match.
        let folders = db.search_suggestions("sub", 12, "folders").unwrap();
        assert!(folders.iter().any(|r| r.kind == "folder" && r.title == "sub"), "got {folders:?}");

        // Empty query yields no results.
        assert!(db.search_suggestions("", 12, "all").unwrap().is_empty());

        // Overview returns media + folders for the active root.
        let overview = db.command_center_overview(12, "all").unwrap();
        assert_eq!(overview.iter().filter(|r| r.kind == "media").count(), 3);
        assert_eq!(overview.iter().filter(|r| r.kind == "folder").count(), 1);
    }

    #[test]
    fn manifest_batch_upsert_and_changed_counts() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries = vec![
            entry(root, "/tmp/root/a.mp4", ""),
            entry(root, "/tmp/root/sub/b.mp4", "sub"),
        ];
        assert_eq!(db.upsert_manifest_batch(&entries).unwrap(), 2);
        // Same entries again -> no change.
        assert_eq!(db.upsert_manifest_batch(&entries).unwrap(), 0);
        // Changed mtime -> change.
        let mut changed = entries.clone();
        changed[0].mtime = 2.0;
        assert_eq!(db.upsert_manifest_batch(&changed).unwrap(), 1);
        // Empty batch.
        assert_eq!(db.upsert_manifest_batch(&[]).unwrap(), 0);
    }

    #[test]
    fn invalidate_absent_marks_missing() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries = vec![
            entry(root, "/tmp/root/a.mp4", ""),
            entry(root, "/tmp/root/b.mp4", ""),
        ];
        db.upsert_manifest_batch(&entries).unwrap();
        let removed = db
            .invalidate_absent(root, &["/tmp/root/a.mp4".to_string()])
            .unwrap();
        assert_eq!(removed, 1);
        let row = db.get_media_by_path("/tmp/root/b.mp4").unwrap().unwrap();
        assert!(!row.valid);
    }

    #[test]
    fn settings_roundtrip() {
        let db = open_tmp();
        db.set_setting("theme_mode", "\"relay\"").unwrap();
        assert_eq!(db.get_setting_raw("theme_mode").unwrap().unwrap(), "\"relay\"");
        assert_eq!(db.get_setting_raw("missing").unwrap(), None);
        let all = db.get_settings().unwrap();
        assert_eq!(all.get("theme_mode").unwrap(), "\"relay\"");
    }

    #[test]
    fn media_probe_decision() {
        let db = open_tmp();
        let root = "/tmp/root";
        db.upsert_manifest_batch(&[entry(root, "/tmp/root/a.mp4", "")])
            .unwrap();
        // Duration is 0 until probed -> needs probe.
        assert!(db
            .media_needs_probe("/tmp/root/a.mp4", 100, 1.0, true)
            .unwrap());
        // Mark probed via upsert_media.
        let metadata = MediaMetadata {
            root_path: root.into(),
            path: "/tmp/root/a.mp4".into(),
            name: "a.mp4".into(),
            relative_path: "a.mp4".into(),
            folder: String::new(),
            duration: 30.0,
            width: 1920,
            height: 1080,
            size_bytes: 100,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            frame_rate: 30.0,
            mtime: 1.0,
        };
        db.upsert_media(&metadata).unwrap();
        assert!(!db
            .media_needs_probe("/tmp/root/a.mp4", 100, 1.0, true)
            .unwrap());
        // Size change -> needs probe.
        assert!(db
            .media_needs_probe("/tmp/root/a.mp4", 101, 1.0, true)
            .unwrap());
        // Without require_probe, duration 0 is fine.
        let db2 = open_tmp();
        db2.upsert_manifest_batch(&[entry(root, "/tmp/root/a.mp4", "")])
            .unwrap();
        assert!(!db2
            .media_needs_probe("/tmp/root/a.mp4", 100, 1.0, false)
            .unwrap());
    }

    #[test]
    fn random_avoids_seen_then_resets() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries: Vec<ManifestEntry> = (0..5)
            .map(|i| entry(root, &format!("/tmp/root/{i}.mp4"), ""))
            .collect();
        db.upsert_manifest_batch(&entries).unwrap();
        let mut picked = std::collections::HashSet::new();
        for _ in 0..5 {
            let row = db.random_media(true, false, &[]).unwrap().unwrap();
            assert!(picked.insert(row.id), "no repeats before pool exhaustion");
        }
        // Pool exhausted -> reset happens and a row comes back.
        assert!(db.random_media(true, false, &[]).unwrap().is_some());
    }

    #[test]
    fn random_folder_scopes() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries = vec![
            entry(root, "/tmp/root/a.mp4", "A"),
            entry(root, "/tmp/root/b.mp4", "B"),
            entry(root, "/tmp/root/c.mp4", "A/sub"),
        ];
        db.upsert_manifest_batch(&entries).unwrap();
        // Scope "A" matches A and A/sub (3? no: a.mp4 + c.mp4).
        for _ in 0..10 {
            let row = db.random_media(true, false, &["A".to_string()]).unwrap().unwrap();
            assert!(row.folder == "A" || row.folder == "A/sub");
        }
        // Exact scope.
        for _ in 0..10 {
            let row = db
                .random_media(true, false, &[format!("{EXACT_FOLDER_SCOPE_PREFIX}A")])
                .unwrap()
                .unwrap();
            assert_eq!(row.folder, "A");
        }
        // Folder "B" only.
        let row = db.random_media(true, false, &["B".to_string()]).unwrap().unwrap();
        assert_eq!(row.folder, "B");
    }

    #[test]
    fn history_search_escapes_wildcards() {
        let db = open_tmp();
        let root = "/tmp/root";
        db.upsert_manifest_batch(&[entry(root, "/tmp/root/clip.mp4", "")]).unwrap();
        db.activate_root(Some(root)).unwrap();
        let media_id = db.list_media("", "", "newest", 1, 0).unwrap()[0].id;
        // Two posts: one whose caption contains a literal percent.
        db.create_post(&PostValues {
            media_id,
            export_id: None,
            telegram_enabled: true,
            x_enabled: false,
            telegram_caption: "discount 50% off".into(),
            x_caption: String::new(),
            telegram_mode: "bot".into(),
            telegram_destination: "@c".into(),
            cleanup_policy: "keep".into(),
        }).unwrap();
        db.create_post(&PostValues {
            media_id,
            export_id: None,
            telegram_enabled: true,
            x_enabled: false,
            telegram_caption: "plain caption".into(),
            x_caption: String::new(),
            telegram_mode: "bot".into(),
            telegram_destination: "@c".into(),
            cleanup_policy: "keep".into(),
        }).unwrap();
        // Literal % matches only the caption containing it.
        let hits = db.list_history("50%", 10, 0).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].telegram_caption, "discount 50% off");
        // The underscore is literal too: no caption contains "discount_5"
        // verbatim, so the query must not wildcard-match "discount 50".
        let hits = db.list_history("discount_5", 10, 0).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn posts_and_history_flow() {
        let db = open_tmp();
        let root = "/tmp/root";
        db.upsert_manifest_batch(&[entry(root, "/tmp/root/a.mp4", "")])
            .unwrap();
        let media = db.get_media_by_path("/tmp/root/a.mp4").unwrap().unwrap();
        let export_id = db
            .create_export(&ExportValues {
                media_id: media.id,
                path: "/tmp/export/a.mp4".into(),
                trim_start: 0.0,
                trim_end: 10.0,
                preset: "original".into(),
                target_mb: 0.0,
                size_bytes: 1234,
                duration: 10.0,
                is_generated: true,
                edit_spec: serde_json::json!({"trim_start": 0.0, "trim_end": 10.0}),
                cleanup_state: "kept".into(),
            })
            .unwrap();
        let post_id = db
            .create_post(&PostValues {
                media_id: media.id,
                export_id: Some(export_id),
                telegram_enabled: true,
                x_enabled: true,
                telegram_caption: "hello".into(),
                x_caption: "world".into(),
                telegram_mode: "bot".into(),
                telegram_destination: "@channel".into(),
                cleanup_policy: "keep".into(),
            })
            .unwrap();
        let post = db.get_post(post_id).unwrap().unwrap();
        assert_eq!(post.telegram_status, "queued");
        assert_eq!(post.x_status, "queued");
        assert_eq!(post.source_path.as_deref(), Some("/tmp/root/a.mp4"));
        db.update_post(
            post_id,
            &PostUpdate {
                telegram_status: Some("sent".into()),
                telegram_message_id: Some("42".into()),
                telegram_message_link: Some("https://t.me/c/1/42".into()),
                x_status: Some("prepared".into()),
                ..Default::default()
            },
        )
        .unwrap();
        db.add_attempt(post_id, "telegram", "sent", "Sent through bot", "42", true)
            .unwrap();
        let history = db.list_history("", 100, 0).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].telegram_status, "sent");
        assert_eq!(history[0].export_path.as_deref(), Some("/tmp/export/a.mp4"));
        let (media_count, posts_count, unseen) = db.counts().unwrap();
        assert_eq!(media_count, 1);
        assert_eq!(posts_count, 1);
        assert_eq!(unseen, 1);
        // posted_count increments
        db.increment_posted(media.id);
        let media = db.get_media(media.id).unwrap().unwrap();
        assert_eq!(media.posted_count, 1);
    }

    #[test]
    fn navigation_neighbors_follow_sort() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries = vec![
            entry(root, "/tmp/root/b.mp4", ""),
            entry(root, "/tmp/root/a.mp4", ""),
            entry(root, "/tmp/root/c.mp4", ""),
        ];
        db.upsert_manifest_batch(&entries).unwrap();
        db.activate_root(Some(root)).unwrap();
        let ids: Vec<i64> = db
            .list_media("", "", "name", 10, 0)
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        // Name sort: a, b, c.
        let a = ids[0];
        let b = ids[1];
        let c = ids[2];
        let (found, prev, next) = db.navigation_neighbors(b, "", "", "name").unwrap();
        assert!(found);
        assert_eq!(prev, a);
        assert_eq!(next, c);
        // Boundaries: a has no previous, c has no next.
        let (_, prev, _) = db.navigation_neighbors(a, "", "", "name").unwrap();
        assert_eq!(prev, 0);
        let (_, _, next) = db.navigation_neighbors(c, "", "", "name").unwrap();
        assert_eq!(next, 0);
        // Missing media reports not found.
        let (found, _, _) = db.navigation_neighbors(9999, "", "", "name").unwrap();
        assert!(!found);
    }

    #[test]
    fn folder_lists_and_search() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries = vec![
            entry(root, "/tmp/root/vacation/clip1.mp4", "vacation"),
            entry(root, "/tmp/root/vacation/clip2.mp4", "vacation"),
            entry(root, "/tmp/root/work/talk.mp4", "work"),
        ];
        db.upsert_manifest_batch(&entries).unwrap();
        let random_folders = db.list_random_folders().unwrap();
        let vacation = random_folders
            .iter()
            .find(|f| f.folder == "vacation")
            .unwrap();
        assert_eq!(vacation.count, 2);
        let explorer = db.list_explorer_folders().unwrap();
        assert_eq!(explorer.len(), 2);
        let suggestions = db.search_suggestions("clip", 9, "all").unwrap();
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.iter().all(|s| s.kind == "media"));
        let suggestions = db.search_suggestions("vac", 9, "all").unwrap();
        assert!(suggestions.iter().any(|s| s.kind == "folder"));
        let overview = db.command_center_overview(9, "all").unwrap();
        assert!(!overview.is_empty());
    }

    #[test]
    fn list_media_sort_and_filter() {
        let db = open_tmp();
        let root = "/tmp/root";
        let entries = vec![
            entry(root, "/tmp/root/b.mp4", "x"),
            entry(root, "/tmp/root/a.mp4", "y"),
        ];
        db.upsert_manifest_batch(&entries).unwrap();
        let by_name = db.list_media("", "", "name", 100, 0).unwrap();
        assert_eq!(by_name[0].name, "a.mp4");
        assert_eq!(by_name[1].name, "b.mp4");
        let filtered = db.list_media("a.mp4", "", "name", 100, 0).unwrap();
        assert_eq!(filtered.len(), 1);
        let folder = db.list_media("", "x", "name", 100, 0).unwrap();
        assert_eq!(folder.len(), 1);
        assert_eq!(folder[0].name, "b.mp4");
    }

    #[test]
    fn pagination_is_stable_across_offsets() {
        let db = open_tmp();
        let root = "/tmp/root";
        // 25 entries with identical mtimes and names that collide after
        // the sort tiebreaker, to exercise the id-ordering.
        let entries: Vec<ManifestEntry> = (0..25)
            .map(|i| entry(root, &format!("/tmp/root/v{i:02}.mp4"), ""))
            .collect();
        db.upsert_manifest_batch(&entries).unwrap();
        let page_size = 10usize;
        let mut seen: Vec<i64> = Vec::new();
        let mut offset = 0usize;
        loop {
            let rows = db
                .list_media("", "", "newest", page_size as i64, offset as i64)
                .unwrap();
            if rows.is_empty() {
                break;
            }
            for row in &rows {
                assert!(
                    !seen.contains(&row.id),
                    "duplicate row {} at offset {offset}",
                    row.id
                );
                seen.push(row.id);
            }
            offset += rows.len();
        }
        assert_eq!(seen.len(), 25, "every row appears exactly once");
        // Same guarantee under the name sort.
        let mut seen: Vec<i64> = Vec::new();
        let mut offset = 0usize;
        loop {
            let rows = db
                .list_media("", "", "name", 7, offset as i64)
                .unwrap();
            if rows.is_empty() {
                break;
            }
            for row in &rows {
                assert!(!seen.contains(&row.id), "duplicate under name sort");
                seen.push(row.id);
            }
            offset += rows.len();
        }
        assert_eq!(seen.len(), 25);
    }

    #[test]
    fn activate_root_reconciles() {
        let db = open_tmp();
        let root_a = "/tmp/rootA";
        let root_b = "/tmp/rootB";
        db.upsert_manifest_batch(&[entry(root_a, "/tmp/rootA/a.mp4", "")])
            .unwrap();
        db.upsert_manifest_batch(&[entry(root_b, "/tmp/rootB/b.mp4", "")])
            .unwrap();
        db.activate_root(Some(root_a)).unwrap();
        let all = db.list_media("", "", "name", 100, 0).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].root_path, root_a);
        // With no active root, every row is deactivated (Python parity).
        db.activate_root(None).unwrap();
        let all = db.list_media("", "", "name", 100, 0).unwrap();
        assert_eq!(all.len(), 0);
        // Reactivating restores the correct scope.
        db.activate_root(Some(root_b)).unwrap();
        let all = db.list_media("", "", "name", 100, 0).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].root_path, root_b);
    }
}
