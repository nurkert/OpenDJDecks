use super::*;
use od_core::{
    Beat, Cue, CueColor, CueKind, MusicalKey, Phrase, PhraseKind, PhraseMood, TrackTags, Waveform,
};
use od_format_clubusb::anlz::{AnlzInput, anlz_dat_path_for, build_anlz};
use od_format_clubusb::pdb::{ExportDatabase, ExportHistoryPlaylist, ExportPlaylist, ExportTrack};
use od_format_clubusb::{HOT_CUE_PALETTE, TRACK_COLORS};
use rusqlite::{Connection, params};

/// Writes a short 16-bit mono WAV whose content depends on `seed`.
fn write_wav(path: &Path, seed: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let rate = 44_100u32;
    let frames = rate * 2;
    let mut data = Vec::with_capacity(frames as usize * 2);
    for i in 0..frames {
        let v = (((i * (seed + 3)) % 200) as i16 - 100) * 50;
        data.extend_from_slice(&v.to_le_bytes());
    }
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(&data);
    std::fs::write(path, b).unwrap();
}

fn grid(bpm: f64, n: usize) -> Beatgrid {
    Beatgrid {
        beats: (0..n)
            .map(|i| Beat {
                time: 0.05 + i as f64 * 60.0 / bpm,
                bpm,
                bar_position: (i % 4) as u8 + 1,
            })
            .collect(),
        locked: false,
    }
}

/// Writes `.DAT`/`.EXT`/`.2EX` next to `dat`.
fn write_anlz(dat: &Path, device_path: &str, g: &Beatgrid, cues: &[Cue]) {
    let wf = Waveform {
        columns_per_second: Waveform::DETAIL_RATE,
        columns: vec![
            od_core::WaveformColumn {
                low: 100,
                mid: 50,
                high: 20,
                peak: 100
            };
            300
        ],
    };
    let phrases = [
        Phrase {
            start_beat: 0,
            end_beat: 2,
            kind: PhraseKind::Intro,
        },
        Phrase {
            start_beat: 2,
            end_beat: 4,
            kind: PhraseKind::Outro,
        },
    ];
    let set = build_anlz(&AnlzInput {
        device_path,
        duration: 2.0,
        beatgrid: g,
        cues,
        waveform: &wf,
        phrases: &phrases,
        phrase_mood: PhraseMood::High,
        is_mp3_vbr_index: None,
    });
    std::fs::create_dir_all(dat.parent().unwrap()).unwrap();
    std::fs::write(dat, set.dat).unwrap();
    std::fs::write(dat.with_extension("EXT"), set.ext).unwrap();
    std::fs::write(dat.with_extension("2EX"), set.ex2).unwrap();
}

const LIBRARY_FILE: &str = "library.db";
/// An obviously fake passphrase, only used to encrypt the test database.
const TEST_PASSPHRASE: &str = "obviously-fake-test-passphrase";

fn plain_source(dir: &Path) -> Source {
    Source::EncryptedLibrary {
        file: dir.join(LIBRARY_FILE),
        key: None,
    }
}

fn encrypted_source(dir: &Path, key: &str) -> Source {
    Source::EncryptedLibrary {
        file: dir.join(LIBRARY_FILE),
        key: Some(LibraryKey::new(key.into())),
    }
}

const SCHEMA: &str = "
CREATE TABLE djmdContent (ID VARCHAR(255) PRIMARY KEY, FolderPath, FileNameL, Title, ArtistID, AlbumID,
  GenreID, LabelID, RemixerID, ComposerID, OrgArtistID, KeyID, Commnt, ReleaseYear INTEGER, TrackNo INTEGER,
  Rating INTEGER, ColorID, BPM INTEGER, Length INTEGER, SampleRate INTEGER, BitRate INTEGER, BitDepth INTEGER,
  FileSize INTEGER, AnalysisDataPath, StockDate, created_at, ReleaseDate, DJPlayCount INTEGER, ImagePath,
  rb_local_deleted TINYINT(1) DEFAULT 0);
CREATE TABLE djmdArtist (ID PRIMARY KEY, Name, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdAlbum (ID PRIMARY KEY, Name, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdGenre (ID PRIMARY KEY, Name, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdLabel (ID PRIMARY KEY, Name, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdKey (ID PRIMARY KEY, ScaleName, Seq, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdCue (ID PRIMARY KEY, ContentID, InMsec INTEGER, OutMsec INTEGER, Kind INTEGER, Color INTEGER,
  ColorTableIndex INTEGER, Comment, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdPlaylist (ID PRIMARY KEY, Seq INTEGER, Name, Attribute INTEGER, ParentID, SmartList,
  rb_local_deleted DEFAULT 0);
CREATE TABLE djmdSongPlaylist (ID PRIMARY KEY, PlaylistID, ContentID, TrackNo INTEGER, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdHistory (ID PRIMARY KEY, Seq INTEGER, Name, Attribute INTEGER, ParentID, rb_local_deleted DEFAULT 0);
CREATE TABLE djmdSongHistory (ID PRIMARY KEY, HistoryID, ContentID, TrackNo INTEGER, rb_local_deleted DEFAULT 0);
";

/// Builds a synthetic library database in `dir` with music in `music`.
/// Returns the connection-free paths of the three track files.
fn build_library(rb: &Path, music: &Path, reserve: bool) -> Vec<PathBuf> {
    let files: Vec<PathBuf> = ["one.wav", "two.wav", "gone.wav", "deleted.wav"]
        .iter()
        .map(|n| music.join(n))
        .collect();
    write_wav(&files[0], 1);
    write_wav(&files[1], 2);
    write_wav(&files[3], 4);
    // files[2] intentionally missing.
    let anlz_rel = "/PIONEER/USBANLZ/abc/0001-uuid/ANLZ0000.DAT";
    write_anlz(
        &rb.join("share").join(&anlz_rel[1..]),
        "/one.wav",
        &grid(120.0, 4),
        &[Cue::hot(0, 9.0)],
    );

    std::fs::create_dir_all(rb).unwrap();
    let path = rb.join(if reserve { "plain.db" } else { LIBRARY_FILE });
    let conn = Connection::open(&path).unwrap();
    if reserve {
        let mut n: std::ffi::c_int = 80;
        let rc = unsafe {
            rusqlite::ffi::sqlite3_file_control(
                conn.handle(),
                c"main".as_ptr(),
                rusqlite::ffi::SQLITE_FCNTL_RESERVE_BYTES,
                &mut n as *mut _ as *mut std::ffi::c_void,
            )
        };
        assert_eq!(rc, 0);
    }
    conn.execute_batch(SCHEMA).unwrap();
    conn.execute_batch(
        "INSERT INTO djmdArtist VALUES ('a1','Alpha',0),('a2','Remixa',0);
         INSERT INTO djmdAlbum VALUES ('al1','First LP',0);
         INSERT INTO djmdGenre VALUES ('g1','House',0),('g2','Techno',0);
         INSERT INTO djmdLabel VALUES ('l1','Label X',0);
         INSERT INTO djmdKey VALUES ('k1','8A',1,0),('k2','5B',2,0);",
    )
    .unwrap();
    let add = |id: &str,
               f: &Path,
               title: &str,
               genre: &str,
               key: &str,
               bpm: i64,
               anlz: &str,
               deleted: i64| {
        conn.execute(
            "INSERT INTO djmdContent (ID, FolderPath, FileNameL, Title, ArtistID, AlbumID, GenreID, LabelID,
               RemixerID, KeyID, Commnt, ReleaseYear, TrackNo, Rating, ColorID, BPM, Length, SampleRate,
               FileSize, AnalysisDataPath, StockDate, created_at, DJPlayCount, rb_local_deleted)
             VALUES (?1, ?2, ?3, ?4, 'a1', 'al1', ?5, 'l1', 'a2', ?6, 'nice', 2019, 7, 3, '2', ?7, 2, 44100,
               1000, ?8, '2026-01-02', '2026-01-02 10:00:00', 4, ?9)",
            params![id, f.to_str().unwrap(), f.file_name().unwrap().to_str().unwrap(), title, genre, key, bpm, anlz, deleted],
        )
        .unwrap();
    };
    add("1", &files[0], "One", "g1", "k1", 12000, anlz_rel, 0);
    add("2", &files[1], "Two", "g2", "k2", 12800, "", 0);
    add("3", &files[2], "Gone", "g1", "", 0, "", 0);
    add("4", &files[3], "Deleted", "g1", "", 0, "", 1);
    // Track "One" has a cover in the library's artwork folder (thumbnail plus
    // the larger `_m` variant).
    let art = rb.join("share/PIONEER/Artwork/00001");
    std::fs::create_dir_all(&art).unwrap();
    std::fs::write(art.join("a1.jpg"), b"small").unwrap();
    std::fs::write(art.join("a1_m.jpg"), b"\xFF\xD8medium").unwrap();
    conn.execute(
        "UPDATE djmdContent SET ImagePath = '/PIONEER/Artwork/00001/a1.jpg' WHERE ID = '1'",
        [],
    )
    .unwrap();
    conn.execute_batch(
        "INSERT INTO djmdCue VALUES ('c1','1',500,-1,1,255,36,'Drop',0);
         INSERT INTO djmdCue VALUES ('c2','1',1000,1500,5,255,43,'',0);
         INSERT INTO djmdCue VALUES ('c3','1',250,-1,0,-1,NULL,'mem',0);
         INSERT INTO djmdCue VALUES ('c4','1',100,-1,2,-1,0,'',1);
         INSERT INTO djmdCue VALUES ('c5','2',100,-1,9,-1,0,'',0);
         INSERT INTO djmdPlaylist VALUES ('f1',1,'Folder',1,'root',NULL,0);
         INSERT INTO djmdPlaylist VALUES ('p1',2,'Second',0,'f1',NULL,0);
         INSERT INTO djmdPlaylist VALUES ('p2',1,'First',0,'f1',NULL,0);
         INSERT INTO djmdPlaylist VALUES ('p3',2,'Top',0,'root',NULL,0);
         INSERT INTO djmdPlaylist VALUES ('s1',3,'Smart House',4,'root',
           '<NODE Id=\"-123\" LogicalOperator=\"1\" AutomaticUpdate=\"0\"><CONDITION PropertyName=\"genre\" Operator=\"8\" ValueUnit=\"\" ValueLeft=\"hous\" ValueRight=\"\"/></NODE>',0);
         INSERT INTO djmdPlaylist VALUES ('s2',4,'Smart MyTag',4,'root',
           '<NODE Id=\"-124\" LogicalOperator=\"1\" AutomaticUpdate=\"0\"><CONDITION PropertyName=\"myTag\" Operator=\"8\" ValueUnit=\"\" ValueLeft=\"5\" ValueRight=\"\"/></NODE>',0);
         INSERT INTO djmdPlaylist VALUES ('d1',5,'Deleted list',0,'root',NULL,1);
         INSERT INTO djmdSongPlaylist VALUES ('e1','p3','2',1,0),('e2','p3','1',2,0),('e3','p3','3',3,0),
           ('e4','p3','4',4,0),('e5','p3','1',5,1),('e6','p2','1',1,0),('e7','d1','1',1,0);
         INSERT INTO djmdHistory VALUES ('h0',1,'2026',1,'root',0),('h1',1,'HISTORY 001',0,'h0',0);
         INSERT INTO djmdSongHistory VALUES ('x1','h1','1',1,0),('x2','h1','2',2,0);",
    )
    .unwrap();
    if reserve {
        let enc = {
            drop(conn);
            let plain = std::fs::read(&path).unwrap();
            assert_eq!(plain[20], 80, "reserve bytes not applied");
            sqlcipher::encrypt_bytes(&plain, TEST_PASSPHRASE.as_bytes(), [7; 16], 4096)
        };
        std::fs::remove_file(&path).unwrap();
        std::fs::write(rb.join(LIBRARY_FILE), enc).unwrap();
    }
    files
}

fn import_all(source: &Source, lib: &mut Library, opts: &ImportOptions) -> ImportReport {
    import(source, lib, opts, |_| true).unwrap()
}

fn find_node(lib: &Library, parent: Option<NodeId>, name: &str) -> od_library::Node {
    lib.children(parent)
        .unwrap()
        .into_iter()
        .find(|n| n.name == name)
        .unwrap_or_else(|| panic!("no node {name}"))
}

#[test]
fn library_database_import() {
    let tmp = tempfile::tempdir().unwrap();
    let rb = tmp.path().join("library");
    let files = build_library(&rb, &tmp.path().join("music"), false);
    let source = plain_source(&rb);

    let p = preview(&source).unwrap();
    assert_eq!(
        p,
        ImportPreview {
            tracks: 3,
            playlists: 4,
            smart_playlists: 1,
            folders: 1,
            history_playlists: 1,
            cues: 4,
            missing_files: 1
        }
    );

    let mut lib = Library::open_in_memory().unwrap();
    let opts = ImportOptions {
        include_history: true,
        analyse_missing: true,
        ..Default::default()
    };
    let r = import_all(&source, &mut lib, &opts);
    assert_eq!((r.tracks_added, r.tracks_existing), (3, 0));
    assert_eq!(r.missing_files, vec![files[2].clone()]);
    assert_eq!(r.grids, 1);
    assert_eq!(r.keys, 2);
    assert_eq!(r.cues, 4);
    assert!(
        r.skipped
            .iter()
            .any(|(what, why)| what.contains("Smart MyTag") && why.contains("myTag")),
        "{r:?}"
    );
    assert!(r.notes.iter().any(|n| n.contains("smart playlist")));
    assert_eq!(lib.track_count().unwrap(), 3);

    // Track one: tags, grid, key, cues, phrases.
    let one = lib.track_by_path(&files[0]).unwrap().unwrap();
    assert_eq!(one.tags.title, "One");
    assert_eq!(one.tags.artist, "Alpha");
    assert_eq!(one.tags.album, "First LP");
    assert_eq!(one.tags.genre, "House");
    assert_eq!(one.tags.label, "Label X");
    assert_eq!(one.tags.remixer, "Remixa");
    assert_eq!(one.tags.comment, "nice");
    assert_eq!(one.tags.year, Some(2019));
    assert_eq!(one.tags.track_number, Some(7));
    assert_eq!(one.tags.rating, 3);
    assert_eq!(one.tags.color, Some(TRACK_COLORS[1].1));
    // The cover comes from the library's artwork folder, larger variant first.
    let (art, media) = lib
        .artwork(one.id)
        .unwrap()
        .expect("cover from PIONEER/Artwork");
    assert_eq!(
        (art.as_slice(), media.as_str()),
        (b"\xFF\xD8medium".as_slice(), "image/jpeg")
    );
    let (g, src) = lib.beatgrid_with_source(one.id).unwrap().unwrap();
    assert_eq!(src, DataSource::Imported);
    assert_eq!(g.beats.len(), 4);
    assert!((g.beats[0].time - 0.05).abs() < 1e-3);
    assert_eq!(
        lib.key_with_source(one.id).unwrap(),
        Some((MusicalKey::parse("8A").unwrap(), DataSource::Imported))
    );
    let an = lib.analysis(one.id).unwrap();
    // The fixture's two phrases span one bar; readers get them normalised
    // into one section (fragments under four bars merge, see
    // od_core::normalize_phrases).
    assert_eq!(an.phrases.len(), 1);
    assert!(!an.waveform.columns.is_empty());
    let cues = lib.cues(one.id).unwrap();
    assert_eq!(cues.len(), 3, "{cues:?}");
    let a = cues.iter().find(|c| c.kind == CueKind::Hot(0)).unwrap();
    assert_eq!(
        (a.start, a.label.as_str(), a.color),
        (0.5, "Drop", Some(HOT_CUE_PALETTE[35]))
    );
    assert_eq!(a.source, DataSource::Imported);
    let d = cues.iter().find(|c| c.kind == CueKind::Hot(3)).unwrap();
    assert_eq!(
        (d.start, d.loop_end, d.color),
        (1.0, Some(1.5), Some(HOT_CUE_PALETTE[42]))
    );
    let m = cues.iter().find(|c| c.kind == CueKind::Memory).unwrap();
    assert_eq!((m.start, m.label.as_str(), m.color), (0.25, "mem", None));

    // Track two: no ANLZ -> provisional grid from the BPM column (analysis priority).
    let two = lib.track_by_path(&files[1]).unwrap().unwrap();
    assert_eq!(two.bpm, Some(128.0));
    assert_eq!(
        lib.beatgrid_with_source(two.id).unwrap().unwrap().1,
        DataSource::Analysis
    );
    assert_eq!(lib.cues(two.id).unwrap()[0].kind, CueKind::Hot(7));
    assert_eq!(r.needs_analysis, vec![two.id]);
    // Missing track is kept but flagged.
    assert!(lib.track_by_path(&files[2]).unwrap().unwrap().missing);
    assert!(lib.track_by_path(&files[3]).unwrap().is_none());

    // Playlist tree.
    let root = lib.node(r.root_folder.unwrap()).unwrap();
    assert!(root.name.starts_with("Library import 20"));
    let names: Vec<String> = lib
        .children(Some(root.id))
        .unwrap()
        .into_iter()
        .map(|n| n.name)
        .collect();
    assert_eq!(names, ["Folder", "Top", "Smart House", HISTORY_FOLDER]);
    let folder = find_node(&lib, Some(root.id), "Folder");
    let inner: Vec<String> = lib
        .children(Some(folder.id))
        .unwrap()
        .into_iter()
        .map(|n| n.name)
        .collect();
    assert_eq!(inner, ["First", "Second"]);
    let top = find_node(&lib, Some(root.id), "Top");
    let gone = lib.track_by_path(&files[2]).unwrap().unwrap().id;
    let top_tracks: Vec<TrackId> = lib
        .entries(top.id)
        .unwrap()
        .into_iter()
        .map(|e| e.track)
        .collect();
    assert_eq!(top_tracks, vec![two.id, one.id, gone]);
    let smart = find_node(&lib, Some(root.id), "Smart House");
    assert_eq!(smart.kind, NodeKind::Playlist);
    let smart_tracks: Vec<TrackId> = lib
        .entries(smart.id)
        .unwrap()
        .into_iter()
        .map(|e| e.track)
        .collect();
    assert_eq!(smart_tracks, vec![one.id, gone]);
    let hist = find_node(&lib, Some(root.id), HISTORY_FOLDER);
    let year = find_node(&lib, Some(hist.id), "2026");
    let h1 = find_node(&lib, Some(year.id), "HISTORY 001");
    assert_eq!(lib.entries(h1.id).unwrap().len(), 2);
    assert_eq!((r.playlists, r.folders), (5, 4));

    // Manual edits made after the import win on a re-run.
    lib.set_key(one.id, MusicalKey::parse("1B")).unwrap();
    lib.set_cues(one.id, vec![Cue::hot(0, 0.75)]).unwrap();
    let r2 = import_all(&source, &mut lib, &opts);
    assert_eq!((r2.tracks_added, r2.tracks_existing), (0, 3));
    assert_eq!(lib.track_count().unwrap(), 3);
    assert_eq!(
        lib.key_with_source(one.id).unwrap().unwrap().1,
        DataSource::Manual
    );
    let cues = lib.cues(one.id).unwrap();
    let a = cues.iter().find(|c| c.kind == CueKind::Hot(0)).unwrap();
    assert_eq!((a.start, a.source), (0.75, DataSource::Manual));
    assert_eq!(cues.len(), 3);
    // Identical playlists are not created twice.
    assert_eq!(r2.root_folder, r.root_folder);
    assert!(
        r2.notes
            .iter()
            .any(|n| n.contains("identical import folder"))
    );
    assert_eq!(lib.children(None).unwrap().len(), 1);
}

#[test]
fn changed_source_creates_second_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let rb = tmp.path().join("library");
    build_library(&rb, &tmp.path().join("music"), false);
    let source = plain_source(&rb);
    let mut lib = Library::open_in_memory().unwrap();
    import_all(&source, &mut lib, &ImportOptions::default());
    let conn = Connection::open(rb.join(LIBRARY_FILE)).unwrap();
    conn.execute(
        "INSERT INTO djmdPlaylist VALUES ('p9',9,'New',0,'root',NULL,0)",
        [],
    )
    .unwrap();
    drop(conn);
    let r = import_all(&source, &mut lib, &ImportOptions::default());
    let roots: Vec<String> = lib
        .children(None)
        .unwrap()
        .into_iter()
        .map(|n| n.name)
        .collect();
    assert_eq!(roots.len(), 2);
    assert!(roots[1].ends_with("(2)"), "{roots:?}");
    assert_eq!(r.tracks_added, 0);
}

#[test]
fn encrypted_library_import() {
    let tmp = tempfile::tempdir().unwrap();
    let rb = tmp.path().join("library");
    build_library(&rb, &tmp.path().join("music"), true);
    let before = std::fs::read(rb.join(LIBRARY_FILE)).unwrap();
    assert!(!sqlcipher::is_plain_sqlite(&before));
    let source = encrypted_source(&rb, TEST_PASSPHRASE);
    let p = preview(&source).unwrap();
    assert_eq!((p.tracks, p.cues, p.folders, p.missing_files), (3, 4, 1, 1));
    let mut lib = Library::open_in_memory().unwrap();
    let r = import_all(&source, &mut lib, &ImportOptions::default());
    assert_eq!(r.tracks_added, 3);
    // The source is untouched and no side files were created.
    assert_eq!(std::fs::read(rb.join(LIBRARY_FILE)).unwrap(), before);
    let side = |s: &str| rb.join(format!("{LIBRARY_FILE}{s}"));
    assert!(!side("-wal").exists() && !side("-shm").exists());
}

#[test]
fn wrong_key_is_rejected_with_a_plain_message() {
    let tmp = tempfile::tempdir().unwrap();
    let rb = tmp.path().join("library");
    build_library(&rb, &tmp.path().join("music"), true);
    let before = std::fs::read(rb.join(LIBRARY_FILE)).unwrap();
    let err = preview(&encrypted_source(&rb, "not-the-key")).unwrap_err();
    assert!(matches!(err, ImportError::KeyNotAccepted), "{err:?}");
    assert!(err.to_string().contains("key was not accepted"), "{err}");
    assert!(
        !err.to_string().contains("not-the-key"),
        "the key must not appear in an error"
    );
    assert!(matches!(
        sqlcipher::decrypt_bytes(&before, None, b"wrong"),
        Err(sqlcipher::CipherError::BadKey)
    ));
    // A source without any key at all asks for it instead.
    assert!(matches!(
        preview(&plain_source(&rb)),
        Err(ImportError::KeyRequired)
    ));
    assert!(matches!(
        preview(&encrypted_source(&rb, "   ")),
        Err(ImportError::KeyRequired)
    ));
    assert_eq!(std::fs::read(rb.join(LIBRARY_FILE)).unwrap(), before);
}

#[test]
fn temporary_copy_is_removed_after_success_and_error() {
    let tmp = tempfile::tempdir().unwrap();
    let rb = tmp.path().join("library");
    build_library(&rb, &tmp.path().join("music"), true);
    let parent = tmp.path().join("tmp");
    std::fs::create_dir_all(&parent).unwrap();
    let file = rb.join(LIBRARY_FILE);
    let empty = || std::fs::read_dir(&parent).unwrap().next().is_none();

    let good = LibraryKey::new(TEST_PASSPHRASE.into());
    let copy = encrypted_library::open_copy_in(&file, Some(&good), &parent).unwrap();
    assert!(
        !empty(),
        "the decrypted copy lives in the temporary directory"
    );
    drop(copy);
    assert!(empty(), "temporary copy left behind after success");

    let bad = LibraryKey::new("not-the-key".into());
    assert!(matches!(
        encrypted_library::open_copy_in(&file, Some(&bad), &parent),
        Err(ImportError::KeyNotAccepted)
    ));
    assert!(empty(), "temporary copy left behind after a wrong key");

    // Readable, but not a library database.
    std::fs::remove_file(&file).unwrap();
    let conn = Connection::open(&file).unwrap();
    conn.execute_batch("CREATE TABLE unrelated (id INTEGER);")
        .unwrap();
    drop(conn);
    assert!(matches!(
        encrypted_library::open_copy_in(&file, None, &parent),
        Err(ImportError::Invalid(_))
    ));
    assert!(empty(), "temporary copy left behind after an error");
}

#[test]
fn key_is_never_printed() {
    let key = LibraryKey::new("  fake-secret\n".into());
    assert_eq!(key.as_bytes(), b"fake-secret", "pasted blanks are dropped");
    assert!(!format!("{key:?}").contains("fake-secret"));
    let source = Source::EncryptedLibrary {
        file: "x.db".into(),
        key: Some(key),
    };
    assert!(!format!("{source:?}").contains("fake-secret"));
}

#[test]
fn rating_and_slots() {
    assert_eq!(encrypted_library::rating(3), 3);
    assert_eq!(encrypted_library::rating(255), 5);
    assert_eq!(encrypted_library::rating(102), 2);
    let slots: Vec<Option<u8>> = (0..=10).map(encrypted_library::hot_slot).collect();
    assert_eq!(
        slots,
        [
            None,
            Some(0),
            Some(1),
            Some(2),
            None,
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            None
        ]
    );
}

#[test]
fn stick_import_with_copy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("stick");
    let dest = tmp.path().join("music");
    let paths = ["/Contents/Alpha/First LP/a.wav", "/Contents/Beta/b.wav"];
    let mut tracks = Vec::new();
    for (i, dp) in paths.iter().enumerate() {
        let file = club_stick::on_stick(&root, dp);
        write_wav(&file, 10 + i as u32);
        let anlz = anlz_dat_path_for(dp);
        let mut hot = Cue::hot(2, 0.5);
        hot.color = Some(HOT_CUE_PALETTE[5]);
        hot.label = "Hi".into();
        let mut mem = Cue::memory(1.0);
        mem.loop_end = Some(1.5);
        write_anlz(
            &club_stick::on_stick(&root, &anlz),
            dp,
            &grid(124.0, 4),
            &[hot, mem],
        );
        tracks.push(ExportTrack {
            id: i as u32 + 1,
            device_path: dp.to_string(),
            tags: TrackTags {
                title: format!("T{i}"),
                artist: if i == 0 {
                    "Alpha".into()
                } else {
                    "Beta".into()
                },
                rating: 4,
                color: Some(CueColor::rgb(0x10, 0xe0, 0x20)),
                ..Default::default()
            },
            duration: 2.0,
            sample_rate: 44_100,
            bit_rate_kbps: 705,
            bits_per_sample: 16,
            file_size: std::fs::metadata(&file).unwrap().len(),
            format: od_core::AudioFormat::Wav,
            bpm: Some(124.0),
            key: MusicalKey::parse("3A"),
            artwork_path: None,
            anlz_path: anlz,
            date_added: "2026-01-01".into(),
            play_count: 0,
        });
    }
    let db = ExportDatabase {
        tracks,
        playlists: vec![
            ExportPlaylist {
                id: 1,
                parent_id: 0,
                name: "Gigs".into(),
                is_folder: true,
                sort_order: 0,
                track_ids: vec![],
            },
            ExportPlaylist {
                id: 2,
                parent_id: 1,
                name: "Club".into(),
                is_folder: false,
                sort_order: 0,
                track_ids: vec![2, 1],
            },
        ],
        history: vec![],
        export_date: "2026-01-01".into(),
    };
    let pdb = club_stick::pdb_path(&root);
    std::fs::create_dir_all(pdb.parent().unwrap()).unwrap();
    std::fs::write(&pdb, db.to_pdb_bytes()).unwrap();

    let source = Source::ClubStick { root: root.clone() };
    let p = preview(&source).unwrap();
    assert_eq!(
        (p.tracks, p.playlists, p.folders, p.cues, p.missing_files),
        (2, 1, 1, 4, 0)
    );

    let mut lib = Library::open_in_memory().unwrap();
    let opts = ImportOptions {
        copy_audio_to: Some(dest.clone()),
        ..Default::default()
    };
    let r = import_all(&source, &mut lib, &opts);
    assert_eq!(r.tracks_added, 2);
    assert_eq!((r.grids, r.keys, r.cues), (2, 2, 4));
    let all = lib.tracks(od_library::SortKey::Title, false).unwrap();
    assert!(
        all.iter()
            .all(|t| t.path.starts_with(&dest) && t.path.is_file()),
        "{all:?}"
    );
    assert_eq!(all[0].path, dest.join("Alpha").join("a.wav"));
    assert_eq!(all[0].tags.rating, 4);
    let cues = lib.cues(all[0].id).unwrap();
    let hot = cues.iter().find(|c| c.kind == CueKind::Hot(2)).unwrap();
    assert_eq!(
        (hot.label.as_str(), hot.color),
        ("Hi", Some(HOT_CUE_PALETTE[5]))
    );
    assert!(
        cues.iter()
            .any(|c| c.kind == CueKind::Memory && c.loop_end == Some(1.5))
    );
    let root_node = r.root_folder.unwrap();
    let gigs = find_node(&lib, Some(root_node), "Gigs");
    let club = find_node(&lib, Some(gigs.id), "Club");
    let order: Vec<TrackId> = lib
        .entries(club.id)
        .unwrap()
        .into_iter()
        .map(|e| e.track)
        .collect();
    assert_eq!(order, vec![all[1].id, all[0].id]);

    // Re-import: tracks are found by content hash, nothing is copied twice.
    let r2 = import_all(&source, &mut lib, &opts);
    assert_eq!((r2.tracks_added, r2.tracks_existing), (0, 2));
    assert_eq!(lib.track_count().unwrap(), 2);
    assert_eq!(std::fs::read_dir(dest.join("Alpha")).unwrap().count(), 1);
    assert_eq!(lib.children(None).unwrap().len(), 1);
}

#[test]
fn cancel_stops_import() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("stick");
    write_stick(&root);
    let mut lib = Library::open_in_memory().unwrap();
    let res = import(
        &Source::ClubStick { root },
        &mut lib,
        &ImportOptions::default(),
        |p| p.phase != ImportPhase::Playlists,
    );
    assert!(matches!(res, Err(ImportError::Cancelled)));
    assert!(lib.children(None).unwrap().len() <= 1);
}

#[test]
fn stick_history_is_imported_into_its_own_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("stick");
    write_stick(&root);
    let mut lib = Library::open_in_memory().unwrap();
    let opts = ImportOptions {
        include_history: true,
        ..Default::default()
    };
    let r = import_all(&Source::ClubStick { root }, &mut lib, &opts);
    let hist = find_node(&lib, r.root_folder, HISTORY_FOLDER);
    let names: Vec<String> = lib
        .children(Some(hist.id))
        .unwrap()
        .into_iter()
        .map(|n| n.name)
        .collect();
    assert_eq!(names, ["HISTORY 001"]);
}

/// A small stick with one track, one playlist and one history list.
fn write_stick(root: &Path) {
    let dp = "/Contents/a.wav";
    let file = club_stick::on_stick(root, dp);
    write_wav(&file, 3);
    let anlz = anlz_dat_path_for(dp);
    write_anlz(
        &club_stick::on_stick(root, &anlz),
        dp,
        &grid(120.0, 4),
        &[Cue::hot(0, 0.5)],
    );
    let db = ExportDatabase {
        tracks: vec![ExportTrack {
            id: 1,
            device_path: dp.into(),
            tags: TrackTags {
                title: "A".into(),
                ..Default::default()
            },
            duration: 2.0,
            sample_rate: 44_100,
            bit_rate_kbps: 705,
            bits_per_sample: 16,
            file_size: std::fs::metadata(&file).unwrap().len(),
            format: od_core::AudioFormat::Wav,
            bpm: Some(120.0),
            key: None,
            artwork_path: None,
            anlz_path: anlz,
            date_added: "2026-01-01".into(),
            play_count: 0,
        }],
        playlists: vec![ExportPlaylist {
            id: 1,
            parent_id: 0,
            name: "List".into(),
            is_folder: false,
            sort_order: 0,
            track_ids: vec![1],
        }],
        history: vec![ExportHistoryPlaylist {
            id: 7,
            name: "HISTORY 001".into(),
            track_ids: vec![1],
        }],
        export_date: "2026-01-01".into(),
    };
    let pdb = club_stick::pdb_path(root);
    std::fs::create_dir_all(pdb.parent().unwrap()).unwrap();
    std::fs::write(&pdb, db.to_pdb_bytes()).unwrap();
}
