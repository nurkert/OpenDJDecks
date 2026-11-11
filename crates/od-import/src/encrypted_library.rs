//! Reader for an encrypted library database (opt-in, read-only).
//!
//! The user picks the database file and supplies the key for their own
//! library; this crate contains no key, reads no key from any other program
//! and does not look for the database on disk.
//!
//! The file (and its `-wal` / `-shm` side files, if present) is copied to a
//! temporary directory, decrypted there by [`crate::sqlcipher`] and opened
//! read-only; the source is only ever read, and the temporary copy is removed
//! when the import ends, also on error. ANLZ files are read from
//! `share/PIONEER/USBANLZ/…` next to the database file.
//!
//! Table and column meanings follow the public documentation of the
//! pyrekordbox project (<https://github.com/dylanljones/pyrekordbox>):
//!
//! * `djmdContent.FolderPath` is the full path of the audio file;
//!   `AnalysisDataPath` is relative to `<database folder>/share`.
//! * `djmdContent.Rating` is 0..=5 (the XML export uses 0..=255; values
//!   above 5 are treated as that scale).
//! * `djmdContent.ColorID` references `djmdColor` ids 1..=8 (Pink, Red,
//!   Orange, Yellow, Green, Aqua, Blue, Purple), 0 = none.
//! * `djmdCue.Kind`: 0 = memory cue, otherwise the hot cue number. The
//!   numbers in use are 1, 2, 3, 5, 6, 7, 8, 9 for slots A..H (4 is not used
//!   for hot cues). Times in milliseconds (`InMsec`, `OutMsec`; `OutMsec` is
//!   -1 or ≤ `InMsec` unless the cue is a loop). `ColorTableIndex` is the
//!   hot cue colour code of the 62-colour palette also used in ANLZ `PCP2`
//!   entries; `Color` is -1 (none), a track colour id 1..=8, or 255 meaning
//!   "see ColorTableIndex".
//! * `djmdPlaylist.Attribute`: 0 playlist, 1 folder, 4 smart playlist
//!   (`SmartList` holds the rules as XML); `ParentID` is `root` at the top.
//! * `djmdHistory.Attribute`: 0 history list, 1 folder.
//! * Every table has `rb_local_deleted`; rows with 1 are soft-deleted.

use crate::model::{SmartFields, SrcLibrary, SrcNode, SrcTrack, anlz_family};
use crate::{ImportError, Result, smart, sqlcipher};
use od_core::{Cue, CueKind, DataSource, MusicalKey, TrackTags};
use od_format_clubusb::{HOT_CUE_PALETTE, TRACK_COLORS};
use rusqlite::types::Value;
use rusqlite::{Connection, OpenFlags, Row};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// The key a user types in for their own library. Never printed and wiped
/// from memory when dropped; there is deliberately no way to serialize it.
#[derive(Clone)]
pub struct LibraryKey(Zeroizing<String>);

impl LibraryKey {
    /// Takes the key over. Blanks around it (from pasting) are not part of it.
    pub fn new(key: String) -> Self {
        let key = Zeroizing::new(key);
        Self(Zeroizing::new(key.trim().to_string()))
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for LibraryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LibraryKey([redacted])")
    }
}

/// A temporary, decrypted, read-only copy of the database. Dropping it
/// removes the temporary directory.
pub(crate) struct LibraryCopy {
    pub conn: Connection,
    _dir: tempfile::TempDir,
}

/// Copies the database (and `-wal`/`-shm` if present) to a temporary
/// directory, decrypts the copy and opens it read-only. The source files are
/// only ever read. `key` may be `None` for a plain (unencrypted) database.
pub(crate) fn open_copy(file: &Path, key: Option<&LibraryKey>) -> Result<LibraryCopy> {
    open_copy_in(file, key, &std::env::temp_dir())
}

/// [`open_copy`] with the temporary directory created under `parent`.
pub(crate) fn open_copy_in(
    file: &Path,
    key: Option<&LibraryKey>,
    parent: &Path,
) -> Result<LibraryCopy> {
    if !file.is_file() {
        return Err(ImportError::NotFound(file.to_path_buf()));
    }
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "library.db".into());
    let dir = tempfile::Builder::new()
        .prefix("opendjdecks-library-import")
        .tempdir_in(parent)?;
    let copy = dir.path().join(&name);
    std::fs::copy(file, &copy)?;
    for suffix in ["-wal", "-shm"] {
        let mut side = file.as_os_str().to_owned();
        side.push(suffix);
        let side = PathBuf::from(side);
        if side.is_file() {
            std::fs::copy(&side, dir.path().join(format!("{name}{suffix}")))?;
        }
    }
    let head = {
        use std::io::Read;
        let mut b = [0u8; 16];
        let n = std::fs::File::open(&copy)?.read(&mut b)?;
        b[..n].to_vec()
    };
    let open_path = if sqlcipher::is_plain_sqlite(&head) {
        copy
    } else {
        let key = key
            .filter(|k| !k.is_empty())
            .ok_or(ImportError::KeyRequired)?;
        let plain = dir.path().join("plain.db");
        sqlcipher::decrypt_file(&copy, &plain, key.as_bytes())?;
        plain
    };
    let conn = Connection::open_with_flags(
        &open_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    // Fail early with a clear error if this is not a library database.
    conn.query_row("SELECT count(*) FROM djmdContent", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|e| ImportError::Invalid(format!("not a library database: {e}")))?;
    Ok(LibraryCopy { conn, _dir: dir })
}

fn text(r: &Row, i: usize) -> rusqlite::Result<String> {
    Ok(match r.get::<_, Value>(i)? {
        Value::Text(s) => s,
        Value::Integer(n) => n.to_string(),
        Value::Real(f) => f.to_string(),
        _ => String::new(),
    })
}

fn int(r: &Row, i: usize) -> rusqlite::Result<i64> {
    Ok(match r.get::<_, Value>(i)? {
        Value::Integer(n) => n,
        Value::Real(f) => f as i64,
        Value::Text(s) => s.trim().parse().unwrap_or(0),
        _ => 0,
    })
}

fn has_table(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

/// Source rating to 0..=5 stars.
pub(crate) fn rating(v: i64) -> u8 {
    if v <= 5 {
        v.max(0) as u8
    } else {
        ((v.min(255) + 25) / 51) as u8
    }
}

fn track_color(id: i64) -> Option<od_core::CueColor> {
    (1..=8)
        .contains(&id)
        .then(|| TRACK_COLORS[id as usize - 1].1)
}

fn hot_palette(code: i64) -> Option<od_core::CueColor> {
    (1..=HOT_CUE_PALETTE.len() as i64)
        .contains(&code)
        .then(|| HOT_CUE_PALETTE[code as usize - 1])
}

/// Hot cue slot of a `djmdCue.Kind` value.
pub(crate) fn hot_slot(kind: i64) -> Option<u8> {
    match kind {
        1..=3 => Some(kind as u8 - 1),
        5..=9 => Some(kind as u8 - 2),
        _ => None,
    }
}

/// Reads everything needed for an import. `folder` holds the database file
/// (ANLZ paths are resolved against `folder/share`).
pub(crate) fn read(conn: &Connection, folder: &Path, with_smart: bool) -> Result<SrcLibrary> {
    let mut lib = SrcLibrary::default();
    let share = folder.join("share");
    const NOT_DELETED: &str = "coalesce(rb_local_deleted, 0) = 0";

    // Tracks.
    let sql = concat!(
        "SELECT c.ID, c.FolderPath, c.Title, a.Name, al.Name, g.Name, l.Name, rm.Name, co.Name,
                c.Commnt, c.ReleaseYear, c.TrackNo, c.Rating, c.ColorID, c.BPM, c.Length,
                c.SampleRate, c.BitRate, c.BitDepth, c.FileSize, k.ScaleName, c.AnalysisDataPath,
                c.StockDate, c.created_at, c.ReleaseDate, c.DJPlayCount, c.FileNameL, oa.Name, c.ImagePath
         FROM djmdContent c
         LEFT JOIN djmdArtist a ON a.ID = c.ArtistID
         LEFT JOIN djmdAlbum al ON al.ID = c.AlbumID
         LEFT JOIN djmdGenre g ON g.ID = c.GenreID
         LEFT JOIN djmdLabel l ON l.ID = c.LabelID
         LEFT JOIN djmdArtist rm ON rm.ID = c.RemixerID
         LEFT JOIN djmdArtist co ON co.ID = c.ComposerID
         LEFT JOIN djmdArtist oa ON oa.ID = c.OrgArtistID
         LEFT JOIN djmdKey k ON k.ID = c.KeyID
         WHERE coalesce(c.rb_local_deleted, 0) = 0
         ORDER BY c.rowid"
    );
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |r| {
        let id = text(r, 0)?;
        let path = text(r, 1)?;
        let year = int(r, 10)?;
        let track_no = int(r, 11)?;
        let bpm = int(r, 14)?;
        let key_name = text(r, 20)?;
        let anlz = text(r, 21)?;
        let image = text(r, 28)?;
        Ok(SrcTrack {
            src_id: id,
            path: PathBuf::from(&path),
            tags: TrackTags {
                title: text(r, 2)?,
                artist: text(r, 3)?,
                album: text(r, 4)?,
                genre: text(r, 5)?,
                label: text(r, 6)?,
                remixer: text(r, 7)?,
                composer: text(r, 8)?,
                comment: text(r, 9)?,
                year: (1..=9999).contains(&year).then_some(year as u16),
                track_number: (track_no > 0).then_some(track_no as u32),
                rating: rating(int(r, 12)?),
                color: track_color(int(r, 13)?),
            },
            duration: int(r, 15)? as f64,
            sample_rate: int(r, 16)?.max(0) as u32,
            bit_rate: Some(int(r, 17)?).filter(|b| *b > 0).map(|b| b as u32),
            bits_per_sample: Some(int(r, 18)?).filter(|b| *b > 0).map(|b| b as u32),
            file_size: int(r, 19)?.max(0) as u64,
            bpm: (bpm > 0).then(|| bpm as f64 / 100.0),
            key: MusicalKey::parse(&key_name),
            db_cues: Some(Vec::new()),
            anlz: if anlz.is_empty() {
                Vec::new()
            } else {
                anlz_family(share.join(anlz.trim_start_matches(['/', '\\'])))
            },
            artwork: (!image.is_empty()).then(|| share.join(image.trim_start_matches(['/', '\\']))),
            smart: SmartFields {
                original_artist: text(r, 27)?,
                key_name,
                file_name: text(r, 26)?,
                stock_date: text(r, 22)?,
                date_created: text(r, 23)?,
                release_date: text(r, 24)?,
                play_count: int(r, 25)?,
                color_id: int(r, 13)?,
            },
        })
    })?;
    for t in rows {
        let t = t?;
        if t.path.as_os_str().is_empty() {
            lib.skipped.push((
                t.tags.title.clone(),
                "no file path (streaming or cloud track)".into(),
            ));
            continue;
        }
        lib.tracks.push(t);
    }
    drop(stmt);
    let index: HashMap<String, usize> = lib
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.src_id.clone(), i))
        .collect();

    // Cues.
    let mut stmt = conn.prepare(&format!(
        "SELECT ContentID, InMsec, OutMsec, Kind, Color, ColorTableIndex, Comment FROM djmdCue
         WHERE {NOT_DELETED} ORDER BY ContentID, Kind, InMsec"
    ))?;
    let mut unknown_kinds = 0usize;
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        let Some(&ti) = index.get(&text(r, 0)?) else {
            continue;
        };
        let kind = int(r, 3)?;
        let (in_ms, out_ms) = (int(r, 1)?, int(r, 2)?);
        let (color, table) = (int(r, 4)?, int(r, 5)?);
        let cue_kind = match kind {
            0 => CueKind::Memory,
            k => match hot_slot(k) {
                Some(s) => CueKind::Hot(s),
                None => {
                    unknown_kinds += 1;
                    continue;
                }
            },
        };
        let color = match cue_kind {
            CueKind::Hot(_) => hot_palette(table),
            CueKind::Memory => track_color(color).or_else(|| hot_palette(table)),
        };
        let cue = Cue {
            kind: cue_kind,
            start: in_ms.max(0) as f64 / 1000.0,
            loop_end: (out_ms > in_ms && out_ms > 0).then(|| out_ms as f64 / 1000.0),
            label: text(r, 6)?,
            color,
            source: DataSource::Imported,
            active: false,
            trigger: Default::default(),
        };
        lib.tracks[ti]
            .db_cues
            .get_or_insert_with(Vec::new)
            .push(cue);
        lib.db_cue_count += 1;
    }
    drop(rows);
    drop(stmt);
    if unknown_kinds > 0 {
        lib.notes.push(format!(
            "{unknown_kinds} cue(s) of an unknown kind were not imported"
        ));
    }

    // Playlists.
    let mut entries: HashMap<String, Vec<String>> = HashMap::new();
    {
        let mut stmt = conn.prepare(&format!(
            "SELECT PlaylistID, ContentID FROM djmdSongPlaylist WHERE {NOT_DELETED}
             ORDER BY PlaylistID, TrackNo, rowid"
        ))?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let content = text(r, 1)?;
            if index.contains_key(&content) {
                entries.entry(text(r, 0)?).or_default().push(content);
            }
        }
    }
    {
        let mut stmt = conn.prepare(&format!(
            "SELECT ID, Name, Attribute, ParentID, Seq, SmartList FROM djmdPlaylist WHERE {NOT_DELETED}"
        ))?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let id = text(r, 0)?;
            let name = text(r, 1)?;
            let attribute = int(r, 2)?;
            let parent = text(r, 3)?;
            let mut node = SrcNode {
                tracks: entries.remove(&id).unwrap_or_default(),
                id,
                parent: (!parent.is_empty() && parent != "root").then_some(parent),
                name,
                folder: attribute == 1,
                seq: int(r, 4)?,
                was_smart: false,
            };
            match attribute {
                0 | 1 => {}
                4 => {
                    node.was_smart = true;
                    if with_smart {
                        match smart::evaluate(&text(r, 5)?, &lib.tracks) {
                            Ok(ids) => node.tracks = ids,
                            Err(reason) => {
                                lib.skipped
                                    .push((format!("smart playlist \"{}\"", node.name), reason));
                                continue;
                            }
                        }
                    }
                }
                other => {
                    lib.skipped.push((
                        format!("playlist \"{}\"", node.name),
                        format!("unknown type {other}"),
                    ));
                    continue;
                }
            }
            lib.playlists.push(node);
        }
    }

    // History.
    if has_table(conn, "djmdHistory") && has_table(conn, "djmdSongHistory") {
        let mut entries: HashMap<String, Vec<String>> = HashMap::new();
        let mut stmt = conn.prepare(&format!(
            "SELECT HistoryID, ContentID FROM djmdSongHistory WHERE {NOT_DELETED}
             ORDER BY HistoryID, TrackNo, rowid"
        ))?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let content = text(r, 1)?;
            if index.contains_key(&content) {
                entries.entry(text(r, 0)?).or_default().push(content);
            }
        }
        drop(rows);
        drop(stmt);
        let mut stmt = conn.prepare(&format!(
            "SELECT ID, Name, Attribute, ParentID, Seq FROM djmdHistory WHERE {NOT_DELETED}"
        ))?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let id = text(r, 0)?;
            let parent = text(r, 3)?;
            lib.history.push(SrcNode {
                tracks: entries.remove(&id).unwrap_or_default(),
                id,
                parent: (!parent.is_empty() && parent != "root").then_some(parent),
                name: text(r, 1)?,
                folder: int(r, 2)? == 1,
                seq: int(r, 4)?,
                was_smart: false,
            });
        }
    }
    Ok(lib)
}
