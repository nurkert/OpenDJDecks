//! The stick check before a gig (roadmap [53]): reads a synced stick back
//! and compares it with the library, track by track.
//!
//! Per track: the audio file is there and the decoder can open it, the cues
//! and the beatgrid on the stick are the library's. For the whole stick: the
//! playlists hold the library's tracks in the library's order, and no sync
//! was cut off. Both formats are read back when the stick carries them: the
//! OpenDJDecks library and the Club player USB export (`export.pdb` with the
//! analysis files). The stick is ready when nothing is off.
//!
//! What this cannot say: whether a given player model accepts the stick.
//! That needs the players (roadmap [H2]).

use crate::club_usb;
use crate::placement::PlacementMap;
use od_core::{Beatgrid, Cue, DeviceId, TrackId};
use od_format_clubusb::anlz::AnlzFile;
use od_format_clubusb::pdb::ExportDatabase;
use od_library::{Library, NodeKind};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// One aspect of one track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Ok,
    /// Not on the stick (or the file is not there).
    Missing,
    /// On the stick but not the library's (needs a sync).
    Differs,
    /// The file is there but cannot be opened as audio.
    Unreadable,
    /// Nothing to compare (the library has none).
    None,
}

impl Mark {
    pub fn is_ok(self) -> bool {
        matches!(self, Mark::Ok | Mark::None)
    }

    /// The worse of two marks.
    fn and(self, other: Mark) -> Mark {
        let rank = |m: Mark| match m {
            Mark::None => 0,
            Mark::Ok => 1,
            Mark::Differs => 2,
            Mark::Missing => 3,
            Mark::Unreadable => 4,
        };
        if rank(other) > rank(self) { other } else { self }
    }
}

/// The result for one track.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackCheck {
    pub track: TrackId,
    pub title: String,
    pub artist: String,
    pub file: Mark,
    pub cues: Mark,
    pub grid: Mark,
}

impl TrackCheck {
    pub fn ok(&self) -> bool {
        self.file.is_ok() && self.cues.is_ok() && self.grid.is_ok()
    }
}

/// A playlist whose content on the stick is not the library's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistProblem {
    pub name: String,
    pub mark: Mark,
}

/// The whole check.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StickCheck {
    pub tracks: Vec<TrackCheck>,
    pub playlists: usize,
    pub playlist_problems: Vec<PlaylistProblem>,
    /// A sync started and did not finish.
    pub interrupted: bool,
    /// Formats read back.
    pub native: bool,
    pub club_usb: bool,
    /// A format the stick should carry is not there or cannot be read.
    pub format_problems: Vec<String>,
}

impl StickCheck {
    pub fn ready(&self) -> bool {
        !self.interrupted
            && self.format_problems.is_empty()
            && self.playlist_problems.is_empty()
            && self.tracks.iter().all(TrackCheck::ok)
            && (self.native || self.club_usb)
    }

    pub fn tracks_ok(&self) -> usize {
        self.tracks.iter().filter(|t| t.ok()).count()
    }
}

fn readable(path: &Path, probe: bool) -> Mark {
    if !path.is_file() {
        return Mark::Missing;
    }
    if !probe {
        return Mark::Ok;
    }
    match od_decode::probe(path) {
        Ok(_) => Mark::Ok,
        Err(_) => Mark::Unreadable,
    }
}

fn same_grid(a: &Beatgrid, b: &Beatgrid) -> bool {
    a.beats.len() == b.beats.len()
        && a.beats
            .iter()
            .zip(&b.beats)
            .all(|(x, y)| (x.time - y.time).abs() < 0.002 && x.bar_position == y.bar_position)
}

fn grid_mark(library: Option<&Beatgrid>, stick: Option<&Beatgrid>) -> Mark {
    match (library, stick) {
        (None, _) => Mark::None,
        (Some(_), None) => Mark::Missing,
        (Some(l), Some(s)) if same_grid(l, s) => Mark::Ok,
        _ => Mark::Differs,
    }
}

fn cue_mark(library: &[Cue], stick: Option<&[Cue]>) -> Mark {
    match stick {
        None if library.is_empty() => Mark::None,
        None => Mark::Missing,
        Some(s) if club_usb::same_cues(library, s) => {
            if library.is_empty() {
                Mark::None
            } else {
                Mark::Ok
            }
        }
        Some(_) => Mark::Differs,
    }
}

/// The library's playlists for the device as (id, name, tracks), playlists
/// only (folders hold no tracks).
fn expected_lists(
    lib: &Library,
    device: DeviceId,
    on_device: &HashSet<TrackId>,
) -> od_library::Result<Vec<(i64, String, Vec<TrackId>)>> {
    Ok(crate::sync::device_tree(lib, device, on_device)?
        .into_iter()
        .filter(|(n, _)| n.kind != NodeKind::Folder)
        .map(|(n, e)| (n.id.0, n.name, e.into_iter().map(|e| e.track).collect()))
        .collect())
}

/// Checks the stick of `device` mounted at `root` against `lib`.
pub fn check_stick(lib: &Library, device: DeviceId, root: &Path) -> od_library::Result<StickCheck> {
    compare(lib, device, root, true)
}

/// What the stick holds that is not the library's, by the same comparison
/// as [`check_stick`] (without opening the audio files): the tracks whose
/// cues or beatgrid differ or are missing, and whether a playlist or a
/// format differs. A sync writes these even when the change log has nothing
/// for them (a stick written from another state of the library, a library
/// restored from a backup), so that the check and the sync agree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StickDrift {
    pub tracks: HashSet<TrackId>,
    pub playlists: bool,
    pub formats: bool,
}

pub fn stick_drift(lib: &Library, device: DeviceId, root: &Path) -> od_library::Result<StickDrift> {
    let c = compare(lib, device, root, false)?;
    let off = |m: Mark| matches!(m, Mark::Differs | Mark::Missing);
    Ok(StickDrift {
        tracks: c
            .tracks
            .iter()
            .filter(|t| off(t.cues) || off(t.grid))
            .map(|t| t.track)
            .collect(),
        playlists: !c.playlist_problems.is_empty(),
        formats: !c.format_problems.is_empty(),
    })
}

fn compare(lib: &Library, device: DeviceId, root: &Path, probe: bool) -> od_library::Result<StickCheck> {
    let dev = lib.device(device)?;
    let wanted = lib.device_tracks(device)?;
    let wanted_ids: HashSet<TrackId> = wanted.iter().map(|t| t.id).collect();
    let expected = expected_lists(lib, device, &wanted_ids)?;
    let mut out = StickCheck {
        interrupted: matches!(od_format_native::interrupted_sync(root), Ok(Some(_))),
        playlists: expected.len(),
        ..Default::default()
    };
    let mut rows: Vec<TrackCheck> = wanted
        .iter()
        .map(|t| TrackCheck {
            track: t.id,
            title: if t.tags.title.is_empty() {
                t.path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                t.tags.title.clone()
            },
            artist: t.tags.artist.clone(),
            file: Mark::None,
            cues: Mark::None,
            grid: Mark::None,
        })
        .collect();
    let mut problems: HashMap<String, Mark> = HashMap::new();
    let mut note = |name: &str, mark: Mark| {
        let e = problems.entry(name.to_string()).or_insert(Mark::Ok);
        *e = e.and(mark);
    };

    // The OpenDJDecks library on the stick.
    if dev.formats.opendjdecks {
        match od_format_native::Stick::open(root) {
            Ok(Some(stick)) => {
                out.native = true;
                for row in &mut rows {
                    let Ok(rec) = stick.library.track(row.track) else {
                        row.file = row.file.and(Mark::Missing);
                        continue;
                    };
                    row.file = row.file.and(readable(&stick.resolve(&rec.path), probe));
                    let lib_cues = lib.cues(row.track)?;
                    let stick_cues = stick.library.cues(row.track).ok();
                    row.cues = row.cues.and(cue_mark(&lib_cues, stick_cues.as_deref()));
                    let lg = lib.beatgrid(row.track)?;
                    let sg = stick.library.beatgrid(row.track).ok().flatten();
                    row.grid = row.grid.and(grid_mark(lg.as_ref(), sg.as_ref()));
                }
                for (id, name, tracks) in &expected {
                    let id = od_library::NodeId(*id);
                    match stick.library.entries(id) {
                        Ok(e) if stick.library.node(id).is_ok() => {
                            let on: Vec<TrackId> = e.into_iter().map(|e| e.track).collect();
                            if &on != tracks {
                                note(name, Mark::Differs);
                            }
                        }
                        _ => note(name, Mark::Missing),
                    }
                }
            }
            Ok(None) => out.format_problems.push("OpenDJDecks".into()),
            Err(_) => out.format_problems.push("OpenDJDecks".into()),
        }
    }

    // The Club player USB export.
    if dev.formats.club_usb {
        match std::fs::read(root.join(club_usb::PDB_PATH))
            .ok()
            .and_then(|b| ExportDatabase::parse(&b).ok())
        {
            Some(db) => {
                out.club_usb = true;
                let state = club_usb::written_analysis(root);
                let placement = PlacementMap::load(root);
                // Export row id → library track, through the analysis path.
                let by_dat: HashMap<&str, TrackId> =
                    state.iter().map(|(id, dat)| (dat.as_str(), *id)).collect();
                let mut pdb_to_lib: HashMap<u32, TrackId> = HashMap::new();
                let mut exported: HashMap<TrackId, &od_format_clubusb::pdb::ExportTrack> =
                    HashMap::new();
                for t in &db.tracks {
                    if let Some(id) = by_dat.get(t.anlz_path.as_str()) {
                        pdb_to_lib.insert(t.id, *id);
                        exported.insert(*id, t);
                    }
                }
                for row in &mut rows {
                    let Some(t) = exported.get(&row.track) else {
                        row.file = row.file.and(Mark::Missing);
                        continue;
                    };
                    // The audio the players get: the WAV copy where there is one.
                    let audio = placement
                        .tracks
                        .get(&row.track)
                        .and_then(|p| p.wav.as_ref())
                        .filter(|w| w.path == t.device_path)
                        .map(|w| w.path.clone())
                        .unwrap_or_else(|| t.device_path.clone());
                    row.file = row.file.and(readable(&root.join(audio.trim_start_matches('/')), probe));
                    let dat = std::fs::read(root.join(t.anlz_path.trim_start_matches('/')))
                        .ok()
                        .and_then(|b| AnlzFile::parse(&b).ok());
                    let ext = std::fs::read(
                        root.join(club_usb::ext_path(&t.anlz_path).trim_start_matches('/')),
                    )
                    .ok()
                    .and_then(|b| AnlzFile::parse(&b).ok());
                    let lib_cues = lib.cues(row.track)?;
                    let stick_cues = ext.as_ref().or(dat.as_ref()).map(|f| f.cues());
                    row.cues = row.cues.and(cue_mark(&lib_cues, stick_cues.as_deref()));
                    let lg = lib.beatgrid(row.track)?;
                    let sg = dat.as_ref().and_then(|f| f.beatgrid());
                    row.grid = row.grid.and(grid_mark(lg.as_ref(), sg.as_ref()));
                }
                for (_, name, tracks) in &expected {
                    match db.playlists.iter().find(|p| !p.is_folder && &p.name == name) {
                        Some(p) => {
                            let on: Vec<TrackId> = p
                                .track_ids
                                .iter()
                                .filter_map(|i| pdb_to_lib.get(i).copied())
                                .collect();
                            if &on != tracks {
                                note(name, Mark::Differs);
                            }
                        }
                        None => note(name, Mark::Missing),
                    }
                }
            }
            None => out.format_problems.push("Club player USB".into()),
        }
    }

    let mut list_problems: Vec<PlaylistProblem> = problems
        .into_iter()
        .filter(|(_, m)| !m.is_ok())
        .map(|(name, mark)| PlaylistProblem { name, mark })
        .collect();
    list_problems.sort_by(|a, b| a.name.cmp(&b.name));
    out.playlist_problems = list_problems;
    // Problems first, then by title.
    rows.sort_by(|a, b| a.ok().cmp(&b.ok()).then_with(|| a.title.cmp(&b.title)));
    out.tracks = rows;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{FormatWriter, NativeWriter, SyncOptions, sync_device};
    use crate::ClubUsbWriter;
    use od_core::{AudioFormat, TrackTags};
    use od_library::{DeviceFormats, NewTrack};
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;

    struct Fx {
        _dir: tempfile::TempDir,
        lib: Library,
        stick: PathBuf,
        dev: DeviceId,
        pl: od_library::NodeId,
        ids: Vec<TrackId>,
    }

    fn wav(path: &Path) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..44_100 {
            let v = ((i as f32 * 0.05).sin() * 8000.0) as i16;
            w.write_sample(v).unwrap();
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
    }

    fn setup(formats: DeviceFormats) -> Fx {
        let dir = tempfile::tempdir().unwrap();
        let stick = dir.path().join("stick");
        std::fs::create_dir_all(&stick).unwrap();
        let mut lib = Library::open(&dir.path().join("lib.sqlite")).unwrap();
        let mut ids = Vec::new();
        for name in ["Alpha", "Beta"] {
            let file = dir.path().join(format!("{name}.wav"));
            wav(&file);
            let id = lib
                .add_track(&NewTrack {
                    content_hash: od_decode::content_hash(&file).unwrap(),
                    file_size: std::fs::metadata(&file).unwrap().len(),
                    path: file,
                    format: Some(AudioFormat::Wav),
                    duration: 1.0,
                    sample_rate: 44_100,
                    channels: 2,
                    bit_rate: None,
                    bits_per_sample: Some(16),
                    tags: TrackTags {
                        title: name.into(),
                        artist: "Someone".into(),
                        ..Default::default()
                    },
                    artwork: None,
                })
                .unwrap();
            lib.set_beatgrid(id, Beatgrid::constant(124.0, 0.05, 1, 1.0))
                .unwrap();
            lib.set_cues(id, vec![Cue::hot(0, 0.5)]).unwrap();
            ids.push(id);
        }
        let pl = lib.create_node(None, NodeKind::Playlist, "Gig").unwrap();
        lib.add_to_node(pl, &ids, None).unwrap();
        let dev = lib.register_device("u", "S", formats).unwrap();
        lib.set_device_nodes(dev, &[pl]).unwrap();
        let mut f = Fx {
            _dir: dir,
            lib,
            stick,
            dev,
            pl,
            ids,
        };
        sync(&mut f);
        f
    }

    fn sync(f: &mut Fx) {
        let mut writers: Vec<Box<dyn FormatWriter + Send>> = vec![
            Box::new(NativeWriter { stems: None }),
            Box::new(ClubUsbWriter::default()),
        ];
        sync_device(
            &mut f.lib,
            f.dev,
            &f.stick.clone(),
            &mut writers,
            &SyncOptions::default(),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    }

    fn both() -> DeviceFormats {
        DeviceFormats {
            club_usb: true,
            opendjdecks: true,
            club_usb_compat: false,
        }
    }

    #[test]
    fn a_freshly_synced_stick_is_ready() {
        for formats in [both(), DeviceFormats::default()] {
            let f = setup(formats);
            let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
            assert!(c.ready(), "{c:#?}");
            assert_eq!(c.tracks.len(), 2);
            assert_eq!(c.tracks_ok(), 2);
            assert_eq!(c.playlists, 1);
            assert!(c.native);
            assert_eq!(c.club_usb, formats.club_usb);
            assert!(c.tracks.iter().all(|t| t.cues == Mark::Ok && t.grid == Mark::Ok));
        }
    }

    #[test]
    fn a_missing_file_is_named_on_its_track() {
        let f = setup(both());
        let rec = od_format_native::Stick::open(&f.stick)
            .unwrap()
            .unwrap()
            .library
            .track(f.ids[1])
            .unwrap();
        std::fs::remove_file(f.stick.join(rec.path.strip_prefix("/").unwrap_or(&rec.path)))
            .unwrap();
        let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
        assert!(!c.ready());
        assert_eq!(c.tracks[0].title, "Beta", "problems come first");
        assert_eq!(c.tracks[0].file, Mark::Missing);
        assert!(c.tracks[1].ok());
    }

    #[test]
    fn a_damaged_file_is_unreadable() {
        let f = setup(DeviceFormats::default());
        let rec = od_format_native::Stick::open(&f.stick)
            .unwrap()
            .unwrap()
            .library
            .track(f.ids[0])
            .unwrap();
        std::fs::write(
            f.stick.join(rec.path.strip_prefix("/").unwrap_or(&rec.path)),
            b"not audio at all",
        )
        .unwrap();
        let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
        assert_eq!(c.tracks[0].file, Mark::Unreadable);
        assert!(!c.ready());
    }

    #[test]
    fn library_changes_since_the_sync_show_until_the_next_sync() {
        let mut f = setup(both());
        f.lib.set_cues(f.ids[0], vec![Cue::hot(0, 0.7)]).unwrap();
        f.lib
            .set_beatgrid(f.ids[1], Beatgrid::constant(128.0, 0.05, 1, 1.0))
            .unwrap();
        f.lib.remove_from_node(f.pl, &[1]).unwrap();
        let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
        assert!(!c.ready());
        let alpha = c.tracks.iter().find(|t| t.title == "Alpha").unwrap();
        assert_eq!(alpha.cues, Mark::Differs);
        assert_eq!(
            c.playlist_problems,
            vec![PlaylistProblem {
                name: "Gig".into(),
                mark: Mark::Differs
            }]
        );
        sync(&mut f);
        let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
        assert!(c.ready(), "{c:#?}");
        assert_eq!(c.tracks.len(), 1, "Beta left the playlist and the stick");
    }

    /// The demo stick's bug: the check named a track's cues as changed, SYNC
    /// wrote nothing ("0 copied, 0 updated, 0 removed") and the next check
    /// named them again, because the sync only went by the change log. Here
    /// the library is restored from a backup taken before a cue edit was
    /// synced, so the stick holds cues the library no longer has and the
    /// change log has nothing for them.
    #[test]
    fn the_sync_writes_what_the_check_calls_changed() {
        for formats in [both(), DeviceFormats::default()] {
            let mut f = setup(formats);
            let backup = f._dir.path().join("before-edit.sqlite");
            f.lib.backup_to(&backup).unwrap();
            f.lib
                .set_cues(f.ids[0], vec![Cue::hot(0, 0.5), Cue::hot(3, 0.9)])
                .unwrap();
            sync(&mut f);
            assert!(check_stick(&f.lib, f.dev, &f.stick).unwrap().ready());
            f.lib.restore_in_place(&backup).unwrap();
            assert_eq!(
                f.lib.pending_changes(f.dev).unwrap().count,
                0,
                "the log knows nothing"
            );

            let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
            let alpha = c.tracks.iter().find(|t| t.title == "Alpha").unwrap();
            assert_eq!(alpha.cues, Mark::Differs, "{c:#?}");
            assert!(!c.ready());
            let drift = stick_drift(&f.lib, f.dev, &f.stick).unwrap();
            assert_eq!(drift.tracks, HashSet::from([f.ids[0]]));

            let mut writers: Vec<Box<dyn FormatWriter + Send>> = vec![
                Box::new(NativeWriter { stems: None }),
                Box::new(ClubUsbWriter::default()),
            ];
            let report = sync_device(
                &mut f.lib,
                f.dev,
                &f.stick.clone(),
                &mut writers,
                &SyncOptions::default(),
                &AtomicBool::new(false),
                |_| {},
            )
            .unwrap();
            assert!(!report.full);
            assert_eq!(report.updated, 1, "SYNC writes the track the check named");
            let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
            assert!(c.ready(), "{c:#?}");
            assert!(
                stick_drift(&f.lib, f.dev, &f.stick)
                    .unwrap()
                    .tracks
                    .is_empty()
            );
        }
    }

    #[test]
    fn an_interrupted_sync_is_not_ready() {
        let f = setup(DeviceFormats::default());
        od_format_native::begin_sync(
            &f.stick,
            &od_format_native::SyncJournal {
                started_at: "x".into(),
                full: false,
            },
        )
        .unwrap();
        let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
        assert!(c.interrupted && !c.ready());
    }

    #[test]
    fn a_stick_without_its_formats_is_not_ready() {
        let f = setup(both());
        std::fs::remove_file(f.stick.join(club_usb::PDB_PATH)).unwrap();
        let c = check_stick(&f.lib, f.dev, &f.stick).unwrap();
        assert_eq!(c.format_problems, vec!["Club player USB".to_string()]);
        assert!(!c.ready());
        let empty = tempfile::tempdir().unwrap();
        let c = check_stick(&f.lib, f.dev, empty.path()).unwrap();
        assert!(!c.ready());
    }
}
