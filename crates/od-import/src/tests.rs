use super::*;
use od_core::{
    Beat, Cue, CueColor, CueKind, MusicalKey, Phrase, PhraseKind, PhraseMood, TrackTags, Waveform,
};
use od_format_clubusb::HOT_CUE_PALETTE;
use od_format_clubusb::anlz::{AnlzInput, anlz_dat_path_for, build_anlz};
use od_format_clubusb::pdb::{ExportDatabase, ExportHistoryPlaylist, ExportPlaylist, ExportTrack};

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
