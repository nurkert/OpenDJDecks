//! Library backups and whole-library archives.
//!
//! Backups are complete SQLite copies made with the online backup API, so
//! they are consistent even while the library is in use. Rotation keeps the
//! newest [`KEEP_RECENT`] backups plus one per day for [`KEEP_DAYS`] days.

use crate::{Library, LibraryError, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const KEEP_RECENT: usize = 10;
pub const KEEP_DAYS: u64 = 14;

impl Library {
    /// Writes a consistent copy of the library to `dest`.
    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = dest.with_extension("partial");
        {
            let mut dst = Connection::open(&tmp)?;
            let backup = rusqlite::backup::Backup::new(&self.conn, &mut dst)?;
            backup.run_to_completion(256, Duration::from_millis(0), None)?;
        }
        std::fs::rename(&tmp, dest)?;
        Ok(())
    }

    /// Puts the open library back to `snapshot` (a copy made with
    /// [`Self::backup_to`] or [`Self::backup_now`]) in place, without closing
    /// it: every change since the snapshot is gone. Used to undo a library
    /// import.
    pub fn restore_in_place(&mut self, snapshot: &Path) -> Result<()> {
        let src = Connection::open_with_flags(
            snapshot,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let backup = rusqlite::backup::Backup::new(&src, &mut self.conn)?;
        backup.run_to_completion(256, Duration::from_millis(0), None)?;
        Ok(())
    }

    /// How many tracks, lists and analysis results the library holds (the
    /// before and after counts of an import).
    pub fn counts(&self) -> Result<LibraryCounts> {
        let n = |sql: &str| -> Result<u64> {
            Ok(self.conn.query_row(sql, [], |r| r.get::<_, i64>(0))? as u64)
        };
        Ok(LibraryCounts {
            tracks: n("SELECT count(*) FROM track")?,
            playlists: n("SELECT count(*) FROM node WHERE kind <> 'folder'")?,
            folders: n("SELECT count(*) FROM node WHERE kind = 'folder'")?,
            cues: n("SELECT count(*) FROM cue")?,
            grids: n("SELECT count(*) FROM analysis WHERE grid IS NOT NULL")?,
            keys: n("SELECT count(*) FROM track WHERE musical_key IS NOT NULL AND musical_key <> ''")?,
        })
    }

    /// Creates a timestamped backup in the backup directory and rotates old
    /// ones. Returns the backup path.
    pub fn backup_now(&self, reason: &str) -> Result<PathBuf> {
        let dir = self.backup_dir()?;
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let reason: String = reason
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let path = dir.join(format!("library-{stamp}-{reason}.sqlite"));
        self.backup_to(&path)?;
        rotate(&dir)?;
        Ok(path)
    }

    fn backup_dir(&self) -> Result<PathBuf> {
        let p = self.path().ok_or_else(|| {
            LibraryError::Invalid("in-memory libraries have no backup directory".into())
        })?;
        Ok(p.parent().unwrap_or(Path::new(".")).join("backups"))
    }

    /// Existing backups, newest first.
    pub fn backups(&self) -> Result<Vec<PathBuf>> {
        list_backups(&self.backup_dir()?)
    }

    /// Replaces the library at `library_path` with `backup`. The current
    /// library is backed up first. The library must not be open.
    pub fn restore(backup: &Path, library_path: &Path) -> Result<Library> {
        if library_path.exists() {
            let current = Library::open(library_path)?;
            current.backup_now("before-restore")?;
            drop(current);
        }
        for ext in ["sqlite-wal", "sqlite-shm"] {
            let _ = std::fs::remove_file(library_path.with_extension(ext));
        }
        std::fs::copy(backup, library_path)?;
        Library::open(library_path)
    }
}

/// What a library holds, by kind ([`Library::counts`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LibraryCounts {
    pub tracks: u64,
    /// Playlists, smart lists and sets.
    pub playlists: u64,
    pub folders: u64,
    pub cues: u64,
    pub grids: u64,
    pub keys: u64,
}

pub(crate) fn backup_before_migration(lib: &Library) -> Result<()> {
    lib.backup_now("before-migration").map(|_| ())
}

fn list_backups(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "sqlite"))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    Ok(files.into_iter().map(|(_, p)| p).collect())
}

fn rotate(dir: &Path) -> Result<()> {
    let now = SystemTime::now();
    let mut kept_days = std::collections::HashSet::new();
    for (i, p) in list_backups(dir)?.into_iter().enumerate() {
        let modified = std::fs::metadata(&p)?.modified()?;
        let age = now.duration_since(modified).unwrap_or_default();
        let day = age.as_secs() / 86_400;
        let keep = i < KEEP_RECENT || (day < KEEP_DAYS && kept_days.insert(day));
        if keep {
            kept_days.insert(day);
        } else {
            std::fs::remove_file(&p)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::track;

    #[test]
    fn restore_in_place_undoes_everything_since_the_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");
        let mut lib = Library::open(&path).unwrap();
        track(&mut lib, "A", None, None);
        let before = lib.counts().unwrap();
        assert_eq!(before.tracks, 1);
        let snap = dir.path().join("undo.snapshot");
        lib.backup_to(&snap).unwrap();
        track(&mut lib, "B", None, None);
        lib.create_node(None, crate::NodeKind::Playlist, "New").unwrap();
        let after = lib.counts().unwrap();
        assert_eq!((after.tracks, after.playlists), (2, before.playlists + 1));
        lib.restore_in_place(&snap).unwrap();
        assert_eq!(lib.counts().unwrap(), before);
        // A second connection sees the restored library too.
        assert_eq!(Library::open(&path).unwrap().counts().unwrap(), before);
    }

    #[test]
    fn backup_rotate_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");
        let mut lib = Library::open(&path).unwrap();
        track(&mut lib, "A", None, None);
        let first = lib.backup_now("test").unwrap();
        for _ in 0..12 {
            lib.backup_now("more").unwrap();
        }
        // All backups are from today: 10 recent ones are kept.
        assert_eq!(lib.backups().unwrap().len(), KEEP_RECENT);
        track(&mut lib, "B", None, None);
        let latest = lib.backups().unwrap()[0].clone();
        drop(lib);
        let lib = Library::restore(&latest, &path).unwrap();
        assert_eq!(lib.track_count().unwrap(), 1);
        let _ = first;
    }
}
