//! A reviewed import of a documented export file: [`plan`] reads the file
//! and the library and says what would carry over and what would not,
//! without writing anything; [`run`] then imports it. The caller takes the
//! library's [`od_library::Library::counts`] before and after and keeps a
//! snapshot for undo ([`od_library::Library::restore_in_place`]).
//!
//! Only files the user exported and picked are read: the library XML
//! format and the Music / iTunes `Library.xml` export. Nothing searches the
//! computer for other software's files.

use crate::itunes::{self, Plist};
use crate::xml::{self, XmlError, path_from_location};
use od_library::Library;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The documented export formats a reviewed import reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    /// The library XML format (rekordbox-compatible).
    LibraryXml,
    /// A Music / iTunes `Library.xml` export.
    MusicXml,
}

/// Why an item of the file does not carry over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeftOutReason {
    /// The audio file is not where the file says.
    FileMissing,
    /// The entry is not a file on this computer (a stream, a cloud item,
    /// no location at all).
    NotALocalFile,
    /// A video, podcast or other item that is not music.
    NotMusic,
    /// Cue points of a track that could not be read (count).
    UnreadableCues(usize),
    /// A list the other software makes itself (the whole library, built-in
    /// smart lists, hidden lists).
    BuiltInList,
    /// Entries of a list whose tracks do not carry over (count).
    ListEntries(usize),
}

/// One item that does not carry over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftOut {
    /// The track (artist – title, else the file) or list name.
    pub item: String,
    pub reason: LeftOutReason,
}

/// What an import would do, found before anything is written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportPlan {
    /// Tracks in the file.
    pub tracks_in_file: usize,
    /// Tracks that would be added.
    pub tracks_new: usize,
    /// Tracks already in the library (by path) that would take the file's
    /// tags, cues, grid and key.
    pub tracks_known: usize,
    pub playlists: usize,
    pub folders: usize,
    pub cues: usize,
    pub grids: usize,
    pub keys: usize,
    pub left_out: Vec<LeftOut>,
}

impl ImportPlan {
    /// Something to import at all.
    pub fn is_empty(&self) -> bool {
        self.tracks_new + self.tracks_known + self.playlists == 0
    }
}

/// What a reviewed import did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunReport {
    pub tracks: usize,
    pub playlists: usize,
    pub cues: usize,
}

/// A location that names a file on this computer.
fn local(location: &str) -> Option<PathBuf> {
    let l = location.trim();
    if l.starts_with("file://") || l.starts_with('/') || l.get(1..3) == Some(":\\") {
        Some(path_from_location(l))
    } else {
        None
    }
}

fn display_name(artist: &str, title: &str, file: Option<&Path>) -> String {
    match (artist.is_empty(), title.is_empty()) {
        (false, false) => format!("{artist} – {title}"),
        (true, false) => title.to_string(),
        _ => file
            .and_then(|f| f.file_name())
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| "?".into()),
    }
}

/// Where a track of the file stands: carried over (known to the library or
/// not) or left out.
enum Fate {
    Carried { known: bool },
    Left(LeftOutReason),
}

fn fate(lib: &Library, location: Option<&str>) -> Result<Fate, XmlError> {
    let Some(file) = location.and_then(local) else {
        return Ok(Fate::Left(LeftOutReason::NotALocalFile));
    };
    if !file.is_file() {
        return Ok(Fate::Left(LeftOutReason::FileMissing));
    }
    Ok(Fate::Carried {
        known: lib.track_by_path(&file)?.is_some(),
    })
}

/// Reads `path` (in `format`) and `lib` and says what an import would carry
/// over. Writes nothing.
pub fn plan(lib: &Library, path: &Path, format: ExportFormat) -> Result<ImportPlan, XmlError> {
    match format {
        ExportFormat::LibraryXml => plan_xml(lib, path),
        ExportFormat::MusicXml => plan_music(lib, path),
    }
}

/// Imports `path` (in `format`) into `lib`.
pub fn run(lib: &mut Library, path: &Path, format: ExportFormat) -> Result<RunReport, XmlError> {
    Ok(match format {
        ExportFormat::LibraryXml => {
            let r = xml::import_xml(lib, path)?;
            RunReport {
                tracks: r.tracks,
                playlists: r.playlists,
                cues: r.cues,
            }
        }
        ExportFormat::MusicXml => {
            let r = itunes::import_itunes(lib, path)?;
            RunReport {
                tracks: r.tracks,
                playlists: r.playlists,
                cues: 0,
            }
        }
    })
}

fn plan_xml(lib: &Library, path: &Path) -> Result<ImportPlan, XmlError> {
    let doc = xml::read_doc(path)?;
    let mut p = ImportPlan {
        tracks_in_file: doc.tracks.len(),
        ..Default::default()
    };
    let mut carried: HashSet<String> = HashSet::new();
    for t in &doc.tracks {
        let location = t.get("Location");
        let name = display_name(
            &t.get("Artist"),
            &t.get("Name"),
            local(&location).as_deref(),
        );
        match fate(lib, (!location.is_empty()).then_some(location.as_str()))? {
            Fate::Left(reason) => p.left_out.push(LeftOut { item: name, reason }),
            Fate::Carried { known } => {
                if known {
                    p.tracks_known += 1;
                } else {
                    p.tracks_new += 1;
                }
                let (cues, lost) = t.cues();
                p.cues += cues.len();
                if lost > 0 {
                    p.left_out.push(LeftOut {
                        item: name,
                        reason: LeftOutReason::UnreadableCues(lost),
                    });
                }
                p.grids += usize::from(t.grid().is_some());
                p.keys += usize::from(t.key().is_some());
                carried.insert(t.get("TrackID"));
            }
        }
    }
    for (_, name, folder, keys) in doc.nodes.iter().skip(1) {
        if *folder {
            p.folders += 1;
            continue;
        }
        p.playlists += 1;
        let lost = keys.iter().filter(|k| !carried.contains(*k)).count();
        if lost > 0 {
            p.left_out.push(LeftOut {
                item: name.clone(),
                reason: LeftOutReason::ListEntries(lost),
            });
        }
    }
    Ok(p)
}

fn plan_music(lib: &Library, path: &Path) -> Result<ImportPlan, XmlError> {
    let root = itunes::parse_plist(&std::fs::read_to_string(path)?)?;
    let mut p = ImportPlan::default();
    let mut carried: HashSet<i64> = HashSet::new();
    if let Some(Plist::Dict(tracks)) = root.get("Tracks") {
        p.tracks_in_file = tracks.len();
        for (_, t) in tracks {
            let location = t.str("Location");
            let name = display_name(
                t.str("Artist").unwrap_or_default(),
                t.str("Name").unwrap_or_default(),
                location.and_then(local).as_deref(),
            );
            if t.str("Kind").is_some_and(|k| k.contains("video"))
                || t.flag("Movie")
                || t.flag("Podcast")
            {
                p.left_out.push(LeftOut {
                    item: name,
                    reason: LeftOutReason::NotMusic,
                });
                continue;
            }
            match fate(lib, location)? {
                Fate::Left(reason) => p.left_out.push(LeftOut { item: name, reason }),
                Fate::Carried { known } => {
                    if known {
                        p.tracks_known += 1;
                    } else {
                        p.tracks_new += 1;
                    }
                    if let Some(id) = t.int("Track ID") {
                        carried.insert(id);
                    }
                }
            }
        }
    }
    if let Some(Plist::Array(lists)) = root.get("Playlists") {
        for l in lists {
            let name = l.str("Name").unwrap_or("Playlist").to_string();
            if l.flag("Master")
                || l.get("Distinguished Kind").is_some()
                || matches!(l.get("Visible"), Some(Plist::Bool(false)))
            {
                p.left_out.push(LeftOut {
                    item: name,
                    reason: LeftOutReason::BuiltInList,
                });
                continue;
            }
            if l.flag("Folder") {
                p.folders += 1;
                continue;
            }
            p.playlists += 1;
            let lost = match l.get("Playlist Items") {
                Some(Plist::Array(items)) => items
                    .iter()
                    .filter_map(|i| i.int("Track ID"))
                    .filter(|i| !carried.contains(i))
                    .count(),
                _ => 0,
            };
            if lost > 0 {
                p.left_out.push(LeftOut {
                    item: name,
                    reason: LeftOutReason::ListEntries(lost),
                });
            }
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_library::Library;

    fn wav(path: &Path, seed: u8) {
        std::fs::write(path, [b'R', b'I', b'F', b'F', seed, seed, seed]).unwrap();
    }

    /// A library XML file: a new track with a cue, a grid and a key; a track
    /// the library already has; a missing file; a stream; a track with an
    /// unreadable cue; a playlist holding a carried and a missing track.
    fn library_xml(dir: &Path) -> (PathBuf, PathBuf) {
        let (new, known) = (dir.join("new.wav"), dir.join("known.wav"));
        let odd = dir.join("odd.wav");
        wav(&new, 1);
        wav(&known, 2);
        wav(&odd, 3);
        let loc = |p: &Path| xml::location(p);
        let gone = loc(&dir.join("gone.wav"));
        let text = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<DJ_PLAYLISTS Version="1.0.0">
  <COLLECTION Entries="5">
    <TRACK TrackID="1" Name="Fresh" Artist="Ann" TotalTime="200" Tonality="Am" Location="{}">
      <TEMPO Inizio="0.1" Bpm="124.00" Metro="4/4" Battito="1"/>
      <POSITION_MARK Name="" Type="0" Start="1.000" Num="0"/>
    </TRACK>
    <TRACK TrackID="2" Name="Known" Artist="Bo" Location="{}"/>
    <TRACK TrackID="3" Name="Lost" Artist="Cy" Location="{gone}"/>
    <TRACK TrackID="4" Name="Radio" Artist="Di" Location="https://example.invalid/stream"/>
    <TRACK TrackID="5" Name="Odd" Artist="Ed" Location="{}">
      <POSITION_MARK Name="broken" Type="0" Num="1"/>
    </TRACK>
  </COLLECTION>
  <PLAYLISTS>
    <NODE Type="0" Name="ROOT" Count="1">
      <NODE Type="0" Name="Gigs" Count="1">
        <NODE Name="Friday" Type="1" KeyType="0" Entries="2">
          <TRACK Key="1"/>
          <TRACK Key="3"/>
        </NODE>
      </NODE>
    </NODE>
  </PLAYLISTS>
</DJ_PLAYLISTS>"#,
            loc(&new),
            loc(&known),
            loc(&odd),
        );
        let file = dir.join("library.xml");
        std::fs::write(&file, text).unwrap();
        (file, known)
    }

    fn add(lib: &mut Library, path: &Path) {
        lib.add_track(&od_library::NewTrack {
            content_hash: od_decode::content_hash(path).unwrap(),
            file_size: 7,
            path: path.to_path_buf(),
            format: None,
            duration: 1.0,
            sample_rate: 44_100,
            channels: 2,
            bit_rate: None,
            bits_per_sample: None,
            tags: Default::default(),
            artwork: None,
        })
        .unwrap();
    }

    #[test]
    fn library_xml_preview_then_import_then_undo() {
        let dir = tempfile::tempdir().unwrap();
        let (file, known) = library_xml(dir.path());
        let db = dir.path().join("library.sqlite");
        let mut lib = Library::open(&db).unwrap();
        add(&mut lib, &known);
        let before = lib.counts().unwrap();

        let p = plan(&lib, &file, ExportFormat::LibraryXml).unwrap();
        assert_eq!(lib.counts().unwrap(), before, "the preview writes nothing");
        assert_eq!(p.tracks_in_file, 5);
        assert_eq!((p.tracks_new, p.tracks_known), (2, 1));
        assert_eq!((p.playlists, p.folders), (1, 1));
        assert_eq!((p.cues, p.grids, p.keys), (1, 1, 1));
        let reasons: Vec<(&str, &LeftOutReason)> =
            p.left_out.iter().map(|l| (l.item.as_str(), &l.reason)).collect();
        assert_eq!(
            reasons,
            vec![
                ("Cy – Lost", &LeftOutReason::FileMissing),
                ("Di – Radio", &LeftOutReason::NotALocalFile),
                ("Ed – Odd", &LeftOutReason::UnreadableCues(1)),
                ("Friday", &LeftOutReason::ListEntries(1)),
            ]
        );

        let snapshot = dir.path().join("undo.snapshot");
        lib.backup_to(&snapshot).unwrap();
        let r = run(&mut lib, &file, ExportFormat::LibraryXml).unwrap();
        assert_eq!((r.tracks, r.playlists, r.cues), (3, 1, 1));
        let after = lib.counts().unwrap();
        assert_eq!(after.tracks, before.tracks + p.tracks_new as u64);
        assert_eq!(after.cues, before.cues + p.cues as u64);
        assert_eq!(after.grids, before.grids + p.grids as u64);
        assert_eq!(after.keys, before.keys + p.keys as u64);
        assert_eq!(after.playlists, before.playlists + p.playlists as u64);

        lib.restore_in_place(&snapshot).unwrap();
        assert_eq!(lib.counts().unwrap(), before, "undo puts everything back");
    }

    #[test]
    fn music_xml_preview_names_what_stays_behind() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.wav"), dir.path().join("b.wav"));
        wav(&a, 1);
        wav(&b, 2);
        let text = crate::itunes::tests::library_xml(&a, &b, &dir.path().join("gone.wav"));
        let file = dir.path().join("Library.xml");
        std::fs::write(&file, text).unwrap();
        let lib = Library::open_in_memory().unwrap();
        let p = plan(&lib, &file, ExportFormat::MusicXml).unwrap();
        assert_eq!((p.tracks_in_file, p.tracks_new, p.tracks_known), (3, 2, 0));
        assert_eq!((p.playlists, p.folders), (2, 1));
        assert_eq!((p.cues, p.grids, p.keys), (0, 0, 0));
        let reasons: Vec<(&str, &LeftOutReason)> =
            p.left_out.iter().map(|l| (l.item.as_str(), &l.reason)).collect();
        assert_eq!(
            reasons,
            vec![
                ("Gone", &LeftOutReason::FileMissing),
                ("Library", &LeftOutReason::BuiltInList),
                ("Music", &LeftOutReason::BuiltInList),
                ("Warm-up", &LeftOutReason::ListEntries(1)),
            ]
        );
    }

    #[test]
    fn only_local_locations_count_as_files() {
        assert!(local("file://localhost/Music/a.mp3").is_some());
        assert!(local("/Music/a.mp3").is_some());
        assert!(local("https://example.invalid/a.mp3").is_none());
        assert!(local("").is_none());
    }
}
