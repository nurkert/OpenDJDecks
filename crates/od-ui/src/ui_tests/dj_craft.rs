//! Roadmap 0.9.0, DJ: recording for release ([27]), sticks ([28]), the
//! queue, requests, do-not-play and auto-play ([29]), and search at
//! request speed, played tonight and bulk tags ([63]), through the real
//! window.

use super::harness::*;
use slint::{ComponentHandle, Model};

fn settings(fx: &Fixture, f: impl FnOnce(&mut crate::settings::Settings)) {
    let mut lib = fx.lib();
    let mut s = crate::settings::Settings::from_json(
        &lib.meta(crate::settings::META_SETTINGS).unwrap().unwrap(),
    );
    f(&mut s);
    lib.set_meta(crate::settings::META_SETTINGS, &s.to_json()).unwrap();
}

/// Loads the fixture's track `i` to deck `deck` (1-based) by a double-click
/// in Perform and waits for it.
pub(crate) fn load(ui: &Ui, i: usize) {
    let title = ui.fx.tracks[i].1.clone();
    ui.app.ui.invoke_select_section("library".into());
    ui.pump(100);
    let row = ui.by_value("browser.row", &title).unwrap().1;
    ui.double_click_el(&row);
    assert!(ui.wait_until(30.0, |u| (1..=4).any(|d| super::flows::deck_title(u, d) == title)));
}

fn rec(u: &Ui) -> crate::Rec<'_> {
    u.app.ui.global::<crate::Rec>()
}

fn em(u: &Ui) -> crate::Emergency<'_> {
    u.app.ui.global::<crate::Emergency>()
}

fn ed(u: &Ui) -> crate::RecEdit<'_> {
    u.app.ui.global::<crate::RecEdit>()
}

/// [27] A recording with a chapter marker and a voice track; loudness
/// while recording and on the finished recording; the tracklist edited
/// (renamed, moved, inserted, removed) and exported as text and CUE with
/// the chapters written into the file.
#[test]
fn recording_for_release_markers_voice_loudness_and_tracklist() {
    let fx = Fixture::new();
    settings(&fx, |s| s.recording.voice = true);
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_set_mode("perform".into());
    ui.pump(200);
    load(&ui, 0);
    ui.click("deck1.play");
    ui.app.ui.invoke_select_section("recordings".into());
    ui.pump(200);
    assert!(!ui.get("rec.mark").accessible_enabled().unwrap_or(true), "MARK waits for a recording");
    ui.click("rec.panel.toggle");
    assert!(ui.wait_until(10.0, |u| rec(u).get_recording()));
    // Loudness, true peak and clips while recording.
    assert!(
        ui.wait_until(20.0, |u| rec(u).get_integrated().ends_with("LUFS")
            && !rec(u).get_integrated().starts_with('—')),
        "integrated loudness shown: {}",
        rec(&ui).get_integrated()
    );
    for id in ["rec.loudness", "rec.short-term", "rec.true-peak", "rec.clips"] {
        assert!(ui.has(id), "{id}");
    }
    assert!(ui.value("rec.true-peak").ends_with("dBTP"));
    assert!(rec(&ui).get_voice_file().ends_with("voice.wav"), "voice track named");
    assert!(ui.wait_until(10.0, |u| u.app.recorder.borrow().recording().is_some_and(|r| r.elapsed() > 1.0)));
    ui.click("rec.mark");
    assert_eq!(rec(&ui).get_markers(), 1);
    ui.pump(1200);
    ui.click("rec.panel.toggle");
    assert!(ui.wait_until(10.0, |u| !rec(u).get_recording()));
    let dir = ui.fx.root.join("recordings");
    let wav = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "wav") && !p.to_string_lossy().contains("voice"))
        .expect("the mix");
    let voice = wav.with_file_name(format!("{} voice.wav", wav.file_stem().unwrap().to_string_lossy()));
    assert!(voice.exists(), "voice track beside the mix");
    assert_eq!(
        hound::WavReader::open(&wav).unwrap().duration(),
        hound::WavReader::open(&voice).unwrap().duration(),
        "sample-aligned"
    );
    let chapters = od_recorder::read_wav_chapters(&wav).unwrap();
    assert_eq!(chapters.len(), 2, "{chapters:?}");
    assert_eq!(chapters[1].title, "Marker 1");
    // The finished recording shows its loudness.
    ui.pump(200);
    assert!(ui.value("rec.list.loudness0").contains("LUFS"), "{}", ui.value("rec.list.loudness0"));
    // The tracklist editor (in Prepare, where the browser has the height).
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.app.ui.invoke_menu_action("view.layout.browser".into());
    ui.app.ui.invoke_select_section("recordings".into());
    ui.pump(200);
    ui.click("rec.list.tracklist0");
    assert!(ed(&ui).get_open());
    assert!(ui.has("rec.edit"));
    assert_eq!(ed(&ui).get_rows().row_count(), 2);
    ed(&ui).invoke_set_title(1, "Shout-out to the crowd".into());
    ed(&ui).invoke_set_artist(0, "Edited Artist".into());
    // A time that is not in the recording is refused with a message.
    ed(&ui).invoke_set_time(1, "99:00".into());
    assert!(!ed(&ui).get_error().is_empty());
    assert!(ui.has("rec.edit.error"));
    ed(&ui).invoke_set_time(1, "0:01".into());
    assert!(ed(&ui).get_error().is_empty());
    assert_eq!(ed(&ui).get_rows().row_data(1).unwrap().time, "0:01");
    ui.click("rec.edit.insert0");
    assert_eq!(ed(&ui).get_rows().row_count(), 3);
    ui.click("rec.edit.remove1");
    assert_eq!(ed(&ui).get_rows().row_count(), 2);
    assert_eq!(ed(&ui).get_rows().row_data(1).unwrap().title, "Shout-out to the crowd");
    ui.click("rec.edit.export-cue");
    let cue = std::fs::read_to_string(wav.with_extension("cue")).unwrap();
    assert!(cue.contains("Shout-out to the crowd") && cue.contains("Edited Artist"), "{cue}");
    ui.click("rec.edit.export-text");
    let text = std::fs::read_to_string(wav.with_extension("txt")).unwrap();
    assert!(text.contains("00:01 Shout-out to the crowd"), "{text}");
    assert!(
        ui.wait_until(10.0, |_| od_recorder::read_wav_chapters(&wav)
            .is_ok_and(|c| c.iter().any(|c| c.title == "Shout-out to the crowd"))),
        "chapters rewritten into the file"
    );
    // Kept in the library and the info file.
    let info = od_recorder::load_info(&wav).unwrap();
    assert_eq!(info.tracklist[1].title, "Shout-out to the crowd");
    assert!(info.loudness.integrated.is_some());
    ui.click("rec.edit.close");
    assert!(!ed(&ui).get_open());
    assert!(ui.has("rec.list.row0"));
}

/// A finished recording in the library with its info file (loudness over
/// the limit, a voice track and a tracklist with long names); returns its
/// id. Made once per library.
pub(crate) fn finished_recording(ui: &Ui) -> i32 {
    let dir = ui.fx.root.join("recordings");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Mix 2026-11-07 21-30-05 at a venue with a rather long name.wav");
    let found: Option<i32> = ui
        .app
        .state
        .borrow()
        .lib
        .conn()
        .query_row("SELECT id FROM recording WHERE path = ?1", [path.display().to_string()], |r| r.get(0))
        .ok();
    if let Some(id) = found {
        return id;
    }
    let e = |o: f64, t: &str, a: &str, marker: bool| od_recorder::TracklistEntry {
        offset: o,
        track: None,
        title: t.into(),
        artist: a.into(),
        marker,
    };
    let tracklist = vec![
        e(0.0, "Opening Track With A Rather Long Title (Extended Club Mix)", "Some Artist & Another Artist", false),
        e(312.0, "Second Track", "Producer", false),
        e(640.5, "Shout-out to the crowd", "", true),
        e(905.0, "Third Track (Original Mix)", "A Very Long Artist Name Featuring Somebody Else", false),
    ];
    let voice = path.with_file_name("Mix 2026-11-07 21-30-05 at a venue with a rather long name voice.wav");
    std::fs::write(&voice, b"").unwrap();
    od_recorder::save_info(
        &path,
        &od_recorder::RecordingInfo {
            tracklist: tracklist.clone(),
            voice: Some(voice),
            loudness: od_recorder::Loudness {
                integrated: Some(-8.94),
                short_term: Some(-7.2),
                momentary: None,
                max_short_term: Some(-6.1),
                true_peak: Some(0.4),
                clips: 128,
            },
            duration: 3723.0,
        },
    )
    .unwrap();
    let st = ui.app.state.borrow();
    st.lib
        .conn()
        .execute(
            "INSERT INTO recording (path, started, duration, tracklist) VALUES (?1, '2026-11-07T21:30:05Z', 3723.0, ?2)",
            (path.display().to_string(), serde_json::to_string(&tracklist).unwrap()),
        )
        .unwrap();
    let id = st.lib.conn().last_insert_rowid() as i32;
    drop(st);
    crate::recorder::refresh_list(&ui.app);
    id
}

/// [28] One press copies the open playlist and the track on a deck to a
/// plugged-in stick (registered on the way, Club player USB with WAV and
/// OpenDJDecks), with its progress and result; the stick's formats are a
/// labelled element in the tree, on the USB screen and over its track
/// list, which also says per track whether the stick carries its stems.
#[test]
fn emergency_stick_formats_and_stems_on_the_stick() {
    let _volumes = volumes_lock();
    let ui = Ui::open((1440, 900));
    let stick = test_env().join("volumes").join("EMERGENCY");
    let _ = std::fs::remove_dir_all(&stick);
    // Nothing plugged in: the key says so and does nothing.
    ui.app.ui.invoke_select_section("devices".into());
    ui.pump(200);
    assert!(ui.has("devices.emergency"));
    assert!(!em(&ui).get_available());
    // A playlist open in the browser, a track on a deck.
    let pl = {
        let mut lib = ui.fx.lib();
        let pl = lib.create_node(None, od_library::NodeKind::Playlist, "Warm-up").unwrap();
        lib.add_to_node(pl, &[ui.fx.tracks[0].0, ui.fx.tracks[1].0], None).unwrap();
        pl
    };
    ui.app.refresh_library();
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    load(&ui, 2);
    ui.app.ui.invoke_select_node(pl.0 as i32);
    ui.pump(200);
    // The stick arrives.
    std::fs::create_dir_all(&stick).unwrap();
    ui.app.device_manager.send(od_devices::Command::Refresh);
    assert!(ui.wait_until(20.0, |u| em(u).get_available()), "a stick to copy to");
    // The DEVICES source of the lists column carries the key.
    ui.click("browser.rail.devices");
    ui.pump(300);
    assert!(ui.has("browser.tree.emergency"), "the key in the tree: {:?}", ui.with_prefix("browser.tree").iter().map(|x| x.0.clone()).collect::<Vec<_>>());
    ui.click("browser.tree.emergency");
    assert!(
        ui.wait_until(60.0, |u| em(u).get_stage() == "done" || em(u).get_stage() == "failed"),
        "{}",
        em(&ui).get_stage()
    );
    assert_eq!(em(&ui).get_stage(), "done", "{}", em(&ui).get_detail());
    assert!(em(&ui).get_detail().starts_with("3 tracks"), "{}", em(&ui).get_detail());
    // On the stick: both formats and the three tracks in a new playlist.
    assert!(stick.join("PIONEER/rekordbox/export.pdb").is_file(), "Club player USB");
    let s = od_format_native::Stick::open(&stick).unwrap().unwrap();
    assert_eq!(s.library.track_count().unwrap(), 3);
    let lib = ui.fx.lib();
    let node = lib.all_nodes().unwrap().into_iter().find(|n| n.name.starts_with("Emergency stick")).expect("its playlist");
    assert_eq!(lib.node_tracks(node.id).unwrap().len(), 3);
    let dev = lib.devices().unwrap().into_iter().find(|d| d.name == "EMERGENCY").expect("registered");
    assert!(dev.formats.club_usb && dev.formats.club_usb_compat && dev.formats.opendjdecks);
    // The result band on the USB screen (in Prepare, where the browser has
    // the height), the formats labelled.
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.app.ui.invoke_select_section("devices".into());
    ui.pump(300);
    assert!(ui.value("devices.emergency-state").contains("ready"), "{}", ui.value("devices.emergency-state"));
    assert_eq!(ui.value("devices.emergency.format.club"), "CLUB USB");
    assert_eq!(ui.value("devices.emergency.format.wav"), "WAV");
    assert!(ui.has("devices.emergency-eject"));
    ui.click("browser.rail.devices");
    ui.pump(200);
    assert_eq!(ui.value("browser.tree.emergency-result"), "READY ON EMERGENCY");
    assert_eq!(ui.value(&format!("devices.badge{}.native", dev.id.0)), "OPENDJDECKS");
    assert_eq!(ui.value(&format!("devices.badge{}.wav", dev.id.0)), "WAV");
    ui.click("devices.emergency-close");
    assert_eq!(em(&ui).get_stage(), "");
    // Browse the stick: its formats over the list, stems per track.
    ui.app.state.borrow_mut().change_settings(|s| {
        if !s.browser.columns.iter().any(|c| c == "stems") {
            s.browser.columns.push("stems".into());
        }
    });
    let hash = s.library.track(ui.fx.tracks[0].0).unwrap().content_hash;
    let name: String = hash.0.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    std::fs::create_dir_all(stick.join("OPENDJDECKS/stems")).unwrap();
    std::fs::write(stick.join("OPENDJDECKS/stems").join(format!("{name}.odstem")), b"stems").unwrap();
    ui.app.ui.invoke_select_section("library".into());
    let i = ui.app.state.borrow().browser.sticks.iter().position(|x| x.1 == stick).expect("browsable");
    ui.app.ui.invoke_select_node(crate::browser::stick_root_id(i));
    ui.pump(300);
    assert_eq!(ui.value("browser.stick.format.club"), "CLUB USB");
    assert_eq!(ui.value("browser.stick.format.native"), "OPENDJDECKS");
    assert_eq!(ui.value("browser.stick.stems"), "STEMS ON THE STICK 1 OF 3");
    let stems: Vec<String> = ui.with_prefix("browser.row").into_iter().filter(|(id, _, _)| id.ends_with(".stems")).map(|x| x.1).collect();
    assert_eq!(stems.iter().filter(|v| *v == "on-stick").count(), 1, "{stems:?}");
    assert_eq!(stems.iter().filter(|v| *v == "not-on-stick").count(), 2, "{stems:?}");
    ui.app.ui.invoke_select_node(-1);
    let _ = std::fs::remove_dir_all(&stick);
}

/// An emergency copy in `stage` ("copying", "done", "failed") to the
/// audit's stick, with long words, for the overflow audit.
pub(crate) fn fake_emergency(ui: &Ui, stage: &str) {
    use crate::emergency::{Emergency, Stage};
    let stick = ui
        .app
        .state
        .borrow()
        .devices
        .statuses
        .first()
        .map(|s| (s.device.id, s.device.name.clone()))
        .unwrap_or((od_core::DeviceId(1), "AUDIT STICK WITH A LONG NAME".into()));
    ui.app.state.borrow_mut().devices.emergency = Some(Emergency {
        mount: std::path::PathBuf::from("/Volumes/AUDIT"),
        stick: stick.1,
        device: Some(stick.0),
        node: od_library::NodeId(1),
        tracks: 48,
        stage: match stage {
            "copying" => Stage::Copying,
            "done" => Stage::Done,
            _ => Stage::Failed("not enough space on the stick: 3221225472 bytes needed, 104857600 bytes free".into()),
        },
        progress: 0.42,
        detail: match stage {
            "copying" => "Copying Opening Track With A Rather Long Title (Extended Club Mix).flac".into(),
            "done" => od_i18n::trf("{} tracks, ready to play", &[&48]) + &od_i18n::trf(" · {} missing files left out", &[&2]),
            _ => "not enough space on the stick: 3221225472 bytes needed, 104857600 bytes free".into(),
        },
        formats: od_library::DeviceFormats { club_usb: true, opendjdecks: true, club_usb_compat: true },
    });
    crate::emergency::project(&ui.app);
}

fn q(u: &Ui) -> crate::Queue<'_> {
    u.app.ui.global::<crate::Queue>()
}

fn queue_titles(u: &Ui) -> Vec<String> {
    q(u).get_rows().iter().map(|r| if r.title.is_empty() { r.request.to_string() } else { r.title.to_string() }).collect()
}

fn deck_playing(u: &Ui, d: usize) -> bool {
    crate::engine::snapshot(&u.app).is_some_and(|s| s.decks[d].playing)
}

/// [29] The queue in Perform: tracks added from the browser, moved and
/// removed; requests matched to the library (or tied to a track by hand);
/// the do-not-play tag (marked in the list and the queue, a warning with
/// UNLOAD when such a track is loaded, left out by auto-play); auto-play
/// starting the queue and mixing into the next track; and the dead-air
/// warning with LOAD NEXT.
#[test]
fn queue_requests_do_not_play_auto_play_and_dead_air() {
    let _volumes = volumes_lock();
    let ui = Ui::open((1440, 900));
    crate::engine::ensure_engine(&ui.app);
    // The list work in Prepare, where the browser has the height; the
    // queue is the same in Perform.
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.app.ui.invoke_menu_action("view.layout.browser".into());
    ui.app.ui.invoke_select_section("library".into());
    ui.app.ui.invoke_select_node(-1);
    ui.pump(200);
    ui.click("browser.panel.queue");
    assert!(ui.has("queue.panel"));
    let row_of = |u: &Ui, title: &str| {
        u.app.state.borrow().browser.rows.iter().position(|t| t.tags.title == title).unwrap() as i32
    };
    // ADD takes the selection; the queue keeps the order and moves.
    ui.app.ui.invoke_row_clicked(row_of(&ui, "Bravo Tune"), false, false);
    ui.app.ui.invoke_row_clicked(row_of(&ui, "Charlie Tune"), true, false);
    ui.pump(100);
    assert!(q(&ui).get_can_add());
    ui.click("queue.add");
    let mut added = queue_titles(&ui);
    assert_eq!(added.len(), 2);
    ui.click("queue.row1.up");
    added.reverse();
    assert_eq!(queue_titles(&ui), added);
    if added[0] != "Charlie Tune" {
        ui.click("queue.row0.down");
    }
    assert_eq!(queue_titles(&ui), ["Charlie Tune", "Bravo Tune"]);
    // Requests: one matches the library, one is noted until a track is
    // chosen for it.
    q(&ui).invoke_request("Test Artist - Alpha".into());
    q(&ui).invoke_request("Nobody Knows This One".into());
    let rows: Vec<crate::QueueRow> = q(&ui).get_rows().iter().collect();
    assert!(rows[2].matched && rows[2].title == "Alpha Tune", "{:?}", rows[2]);
    assert!(!rows[3].matched && rows[3].request == "Nobody Knows This One");
    assert_eq!(ui.value("queue.row3.detail"), "REQUEST · NOT IN THE LIBRARY");
    ui.app.ui.invoke_row_clicked(row_of(&ui, "Alpha Tune"), false, false);
    ui.pump(100);
    ui.click("queue.row3.match");
    assert!(q(&ui).get_rows().row_data(3).unwrap().matched);
    ui.click("queue.row3.remove");
    ui.click("queue.row2.remove");
    assert_eq!(queue_titles(&ui), ["Charlie Tune", "Bravo Tune"]);
    // Kept in the library.
    let saved = ui.fx.lib().meta(crate::queue::META_QUEUE).unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Vec<crate::queue::Item>>(&saved).unwrap().len(), 2);
    // Do not play: Bravo is tagged from the row menu.
    let bravo = row_of(&ui, "Bravo Tune");
    ui.app.ui.invoke_row_clicked(bravo, false, false);
    ui.app.ui.invoke_row_menu(bravo, "dnp".into());
    ui.pump(200);
    let bravo_id = ui.fx.tracks[1].0;
    assert!(crate::queue::is_dnp(&ui.app, bravo_id));
    assert!(ui.has(&format!("browser.row{}.dnp", row_of(&ui, "Bravo Tune"))), "marked in the list");
    let rows: Vec<crate::QueueRow> = q(&ui).get_rows().iter().collect();
    assert!(rows[1].dnp && rows[0].next && !rows[1].next);
    assert_eq!(ui.value("queue.row1.detail"), "DO NOT PLAY");
    // Loading it warns, UNLOAD takes it off again.
    ui.app.ui.invoke_set_mode("perform".into());
    assert!(ui.wait_until(30.0, |u| u.has("mixer.ch1.fader")));
    ui.app.ui.invoke_row_menu(row_of(&ui, "Bravo Tune"), "deck:2".into());
    assert!(ui.wait_until(20.0, |u| q(u).get_alert() == "dnp"), "warned on load");
    ui.pump(100);
    assert!(ui.value("app.set-alert.title").contains("Bravo Tune"));
    ui.click("app.set-alert.unload");
    assert!(ui.wait_until(10.0, |u| u.app.decks.borrow().loaded_id(1).is_none() && q(u).get_alert().is_empty()));
    // Auto-play: the queue starts by itself and mixes into the next track;
    // the do-not-play track is left out.
    q(&ui).invoke_set_mix(1);
    assert_eq!(ui.app.state.borrow().settings.queue.mix_beats, 8);
    ui.click("queue.auto");
    assert!(q(&ui).get_auto());
    let charlie = ui.fx.tracks[2].0;
    assert!(
        ui.wait_until(30.0, |u| (0..4).any(|d| u.app.decks.borrow().loaded_id(d) == Some(charlie) && deck_playing(u, d))),
        "the queue starts with Charlie"
    );
    let from = (0..4).find(|&d| ui.app.decks.borrow().loaded_id(d) == Some(charlie)).unwrap();
    // Alpha joins the queue and is cued on another deck at once.
    ui.app.ui.invoke_row_clicked(row_of(&ui, "Alpha Tune"), false, false);
    ui.pump(50);
    ui.click("queue.add");
    let alpha = ui.fx.tracks[0].0;
    assert!(ui.wait_until(20.0, |u| (0..4).any(|d| d != from && u.app.decks.borrow().loaded_id(d) == Some(alpha))), "cued");
    assert!(
        ui.wait_until(40.0, |u| (0..4).any(|d| d != from && u.app.decks.borrow().loaded_id(d) == Some(alpha) && deck_playing(u, d))),
        "mixed into Alpha before Charlie ends"
    );
    assert!(!(0..4).any(|d| ui.app.decks.borrow().loaded_id(d) == Some(bravo_id)), "Bravo is never loaded");
    // Dead air: auto-play off, Alpha (8 s) runs out with nothing cued (it
    // starts over, so the test has its 8 s).
    ui.click("queue.auto");
    assert!(!q(&ui).get_auto());
    assert!(ui.wait_until(20.0, |u| !deck_playing(u, from)), "Charlie has played out");
    let on = (0..4).find(|&d| ui.app.decks.borrow().loaded_id(d) == Some(alpha)).unwrap();
    crate::engine::deck_cmd(&ui.app, on, od_engine::DeckCommand::Seek { seconds: 0.0 });
    assert!(ui.wait_until(20.0, |u| q(u).get_alert() == "dead-air"), "dead air warned: {}", q(&ui).get_alert());
    ui.pump(100);
    assert!(ui.value("app.set-alert.title").starts_with("Dead air in"), "{}", ui.value("app.set-alert.title"));
    // A track in the queue: LOAD NEXT from the warning cues it.
    ui.app.ui.invoke_row_clicked(row_of(&ui, "Charlie Tune"), false, false);
    ui.pump(50);
    ui.click("queue.add");
    ui.pump(100);
    assert!(ui.wait_until(10.0, |u| u.has("app.set-alert.load")), "alert {:?} can-load {} queue {:?} decks {:?}", q(&ui).get_alert(), q(&ui).get_can_load(), queue_titles(&ui),
        (0..2).map(|d| (ui.app.decks.borrow().loaded_id(d), crate::engine::snapshot(&ui.app).map(|s| (s.decks[d].playing, s.decks[d].position, s.decks[d].duration)))).collect::<Vec<_>>());
    ui.click("app.set-alert.load");
    assert!(ui.wait_until(10.0, |u| q(u).get_alert().is_empty()), "cued: the warning goes");
}

/// A queue with a track, a matched and an unmatched request and a track
/// tagged do not play, for the overflow audit and the views.
pub(crate) fn fill_queue(ui: &Ui) {
    use crate::queue::{Item, STATE};
    let (a, b, c) = (ui.fx.tracks[0].0, ui.fx.tracks[1].0, ui.fx.tracks[2].0);
    {
        let mut lib = ui.app.state.borrow_mut();
        let mut tags = lib.lib.user_tags(c).unwrap_or_default();
        if !tags.iter().any(od_library::UserTag::is_do_not_play) {
            tags.push(od_library::UserTag::do_not_play());
            lib.lib.set_user_tags(c, tags).unwrap();
        }
    }
    STATE.with(|s| {
        s.borrow_mut().items = vec![
            Item { track: Some(a), request: String::new() },
            Item { track: Some(b), request: "Somebody's long request: the one with the vocal from last summer".into() },
            Item { track: None, request: "An Artist With A Very Long Name – A Title Nobody Has (Extended)".into() },
            Item { track: Some(c), request: String::new() },
        ];
    });
    crate::queue::project(&ui.app);
}

/// The set alert in `kind` ("dead-air" or "dnp") with long words.
pub(crate) fn fake_alert(ui: &Ui, kind: &str) {
    crate::queue::HOLD_ALERT.with(|h| h.set(!kind.is_empty()));
    let g = q(ui);
    g.set_alert(kind.into());
    if kind == "dead-air" {
        g.set_alert_title(od_i18n::trf("Dead air in {}", &[&"0:24"]).into());
        g.set_alert_detail(od_i18n::trf("Deck {} runs out and no deck is cued. LOAD NEXT takes the queue's next track.", &[&2]).into());
        g.set_can_load(true);
    } else {
        g.set_alert_title(od_i18n::trf("Do not play: {}", &[&"A Track With A Rather Long Title (Extended Club Mix)"]).into());
        g.set_alert_detail(od_i18n::trf("Deck {} holds a track tagged do not play.", &[&3]).into());
    }
}

/// A library of `n` generated tracks (no audio files: rows straight into
/// the database, as an import leaves them, with tempo and key), for the
/// search timings. Words come from a small vocabulary, so queries hit many
/// tracks as they would in a real collection.
pub(crate) fn big_library(n: usize) -> Fixture {
    let fx = Fixture::empty();
    let lib = fx.lib();
    const A: [&str; 24] = ["Deep", "Night", "Acid", "Velvet", "Solar", "Echo", "Midnight", "Golden", "Broken", "Silent", "Electric", "Lost", "Neon", "Paper", "Crystal", "Distant", "Hidden", "Liquid", "Wild", "Northern", "Southern", "Fading", "Rising", "Second"];
    const B: [&str; 24] = ["Groove", "Signal", "Garden", "Machine", "Dreams", "Motion", "Shadows", "Rhythm", "Waves", "Circuit", "Horizon", "Pulse", "Lights", "Fever", "Theory", "Ritual", "Voices", "Engine", "Bloom", "Static", "Desire", "Tension", "Memory", "Window"];
    const ARTISTS: [&str; 16] = ["Kollektiv Süd", "Mara Lenz", "The Night Shift", "DJ Okapi", "Ferro & Klang", "Aurélie Moreau", "Basement Theory", "Jun Takeda", "Hollow Pines", "Nils Fjord", "Sana Obi", "Rotterdam Rail", "Lumen", "Teo Blaze", "Kinetic Unit", "Paloma Ruiz"];
    const GENRES: [&str; 6] = ["Techno", "House", "Deep House", "Electro", "Minimal", "Disco"];
    let keys = ["1A", "2A", "3A", "4A", "5A", "6A", "7A", "8A", "9A", "10A", "11A", "12A", "1B", "5B", "8B", "11B"];
    let c = lib.conn();
    c.execute_batch("BEGIN").unwrap();
    {
        let mut st = c
            .prepare("INSERT INTO track (path, content_hash, file_size, format, duration, sample_rate, title, artist, album, genre, date_added, bpm, musical_key) VALUES (?1, ?2, ?3, 'mp3', ?4, 44100, ?5, ?6, ?7, ?8, '2026-01-01T00:00:00Z', ?9, ?10)")
            .unwrap();
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        let mut rnd = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for i in 0..n {
            let r = rnd();
            let title = format!(
                "{} {}{}",
                A[(r % 24) as usize],
                B[((r >> 8) % 24) as usize],
                if r % 5 == 0 { " (Extended Mix)" } else if r % 7 == 0 { " (Dub)" } else { "" }
            );
            let artist = ARTISTS[((r >> 16) % 16) as usize];
            let album = format!("{} {}", B[((r >> 24) % 24) as usize], (r >> 32) % 40);
            let genre = GENRES[((r >> 40) % 6) as usize];
            let bpm = 110.0 + ((r >> 44) % 300) as f64 / 10.0;
            st.execute(rusqlite::params![
                format!("/music/{i}.mp3"),
                format!("hash{i}"),
                8_000_000 + (i % 1000) as i64,
                240.0 + (r % 200) as f64,
                title,
                artist,
                album,
                genre,
                bpm,
                keys[((r >> 50) % 16) as usize],
            ])
            .unwrap();
        }
    }
    c.execute_batch("COMMIT").unwrap();
    drop(lib);
    fx
}

/// [63] Search at request speed: every keystroke of a few requests on a
/// library of 50 000 generated tracks, through the real window (the list
/// read, searched, sorted and projected into the table). Writes the
/// timings to target/search-speed.md. A debug build; the release build is
/// several times faster.
#[test]
#[ignore = "timing run on 50 000 tracks: cargo test -p od-ui --lib search_speed -- --ignored --nocapture"]
fn search_speed_on_50k_tracks() {
    let n: usize = std::env::var("OD_SEARCH_TRACKS").ok().and_then(|v| v.parse().ok()).unwrap_or(50_000);
    let fx = big_library(n);
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.app.ui.invoke_select_section("library".into());
    ui.app.ui.invoke_select_node(-1);
    ui.pump(200);
    assert_eq!(ui.app.state.borrow().browser.rows.len(), n);
    let mut report = format!("# Search timings on {n} generated tracks (debug build)\n\nPer keystroke: the whole search through the window, and of it the search itself (score, filter, order) and the table (rows projected into the list).\n\n| query | rows | ms | search | table |\n|---|---:|---:|---:|---:|\n");
    let mut all = Vec::new();
    for request in ["mara lenz rising", "deep groove", "nils fjrod", "ritual dub"] {
        let mut typed = String::new();
        for ch in request.chars() {
            typed.push(ch);
            let t = std::time::Instant::now();
            ui.app.ui.invoke_search(typed.clone().into());
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            let rows = ui.app.state.borrow().browser.rows.len();
            all.push(ms);
            // The same keystroke again, in its two parts.
            let t = std::time::Instant::now();
            ui.app.state.borrow_mut().load_rows_search();
            let search = t.elapsed().as_secs_f64() * 1000.0;
            let t = std::time::Instant::now();
            crate::browser::refresh_tracks(&ui.app);
            let table = t.elapsed().as_secs_f64() * 1000.0;
            report += &format!("| `{typed}` | {rows} | {ms:.1} | {search:.1} | {table:.1} |\n");
        }
        ui.app.ui.invoke_search("".into());
    }
    all.sort_by(f64::total_cmp);
    let median = all[all.len() / 2];
    let p95 = all[(all.len() * 95 / 100).min(all.len() - 1)];
    report += &format!("\nMedian {median:.1} ms, 95th percentile {p95:.1} ms, slowest {:.1} ms per keystroke.\n", all[all.len() - 1]);
    let path = super::views::workspace_root().join("target/search-speed.md");
    let _ = std::fs::write(&path, &report);
    println!("{report}");
}

/// [63] The kept search base gives the same rows, in the same order, as a
/// full reload, for queries with typos, filters and other sorts; anything
/// but the search drops it.
#[test]
fn search_base_matches_a_full_reload() {
    let fx = big_library(3000);
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_select_section("library".into());
    ui.app.ui.invoke_select_node(-1);
    ui.pump(100);
    let ids = |u: &Ui| u.app.state.borrow().browser.rows.iter().map(|t| t.id).collect::<Vec<_>>();
    for (q, ranked, sort, filter) in [
        ("deep groove", true, "date_added", false),
        ("nils fjrod", true, "date_added", false),
        ("ritual", false, "title", false),
        ("mara", false, "bpm", true),
        ("", false, "artist", true),
        ("xyzzy nothing", true, "date_added", false),
    ] {
        {
            let mut st = ui.app.state.borrow_mut();
            st.browser.sort = sort.into();
            st.browser.search = q.into();
            st.browser.search_ranked = ranked;
            if filter {
                st.browser.filter = Default::default();
            }
        }
        if filter {
            ui.app.ui.invoke_filter_edit("bpm-min".into(), "120".into());
            ui.app.ui.invoke_filter_edit("bpm-max".into(), "130".into());
        }
        ui.app.state.borrow_mut().load_rows();
        let full = ids(&ui);
        assert!(ui.app.state.borrow().browser.search_base.is_none(), "a full reload drops the base");
        ui.app.state.borrow_mut().load_rows_search();
        assert!(ui.app.state.borrow().browser.search_base.is_some());
        assert_eq!(ids(&ui), full, "{q:?} sorted by {sort}, filter {filter}");
        ui.app.ui.invoke_filter_edit("clear".into(), "".into());
    }
}

/// [63] Tags edited on many tracks at once (shared values shown, MIXED
/// where they differ, one undo step), do not play from the info panel,
/// and the played-tonight mark with its time, column and filter.
#[test]
fn bulk_tags_and_played_tonight() {
    let _volumes = volumes_lock();
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.app.ui.invoke_menu_action("view.layout.browser".into());
    ui.app.ui.invoke_select_section("library".into());
    ui.app.ui.invoke_select_node(-1);
    ui.app.ui.set_info_open(true);
    ui.app.ui.invoke_row_clicked(0, false, false);
    ui.app.ui.invoke_row_clicked(1, true, false);
    ui.pump(200);
    let info = ui.app.ui.get_track_info();
    assert_eq!(info.count, 2);
    assert!(info.mixed_title && !info.mixed_artist, "titles differ, the artist is shared");
    assert_eq!(info.artist, "Test Artist");
    assert!(ui.has("info.bulk"));
    let ids: Vec<od_core::TrackId> = ui.app.state.borrow().selected_ids();
    ui.app.ui.invoke_info_edit("genre".into(), "Techno".into());
    ui.pump(100);
    let lib = ui.fx.lib();
    assert!(ids.iter().all(|id| lib.track(*id).unwrap().tags.genre == "Techno"), "set on both");
    assert!(!ui.app.ui.get_track_info().mixed_genre);
    // One undo takes both back.
    ui.app.undo();
    ui.pump(100);
    assert!(ids.iter().all(|id| lib.track(*id).unwrap().tags.genre.is_empty()), "one undo step");
    // Do not play for the selection from the panel.
    ui.click("info.dnp");
    ui.pump(100);
    assert!(ids.iter().all(|id| crate::queue::is_dnp(&ui.app, *id)));
    assert!(ui.app.ui.get_track_info().dnp);
    ui.click("info.dnp");
    ui.pump(100);
    assert!(ids.iter().all(|id| !crate::queue::is_dnp(&ui.app, *id)));
    // Played tonight: the tick with its time, the TONIGHT column, the filter.
    ui.app.state.borrow_mut().change_settings(|s| {
        if !s.browser.columns.iter().any(|c| c == "tonight") {
            s.browser.columns.push("tonight".into());
        }
    });
    let played = ui.fx.tracks[2].0;
    crate::recorder::mark_played_now(&ui.app, played, "22:41");
    ui.pump(100);
    let row = ui.app.state.borrow().browser.rows.iter().position(|t| t.id == played).unwrap();
    let r = ui.app.browser.tracks.get(row).unwrap();
    assert!(r.played && r.played_at == "22:41");
    assert!(ui.with_prefix("browser.row.played").iter().any(|(_, v, _)| v == "22:41"), "the tick says when");
    assert!(ui.has("browser.header.tonight"), "the TONIGHT column");
    let before = ui.app.state.borrow().browser.rows.len();
    ui.app.ui.set_filter_open(true);
    ui.pump(100);
    ui.click("browser.filter.unplayed");
    ui.pump(100);
    let after: Vec<od_core::TrackId> = ui.app.state.borrow().browser.rows.iter().map(|t| t.id).collect();
    assert_eq!(after.len(), before - 1);
    assert!(!after.contains(&played), "played tonight is left out");
    assert!(ui.app.ui.get_filter().chips.iter().any(|c| c.field == "unplayed"));
    ui.app.ui.invoke_filter_edit("unplayed".into(), "".into());
    ui.pump(100);
    assert_eq!(ui.app.state.borrow().browser.rows.len(), before);
}

/// Every new control of block F is a Tab stop with the focus ring, has a
/// readable name (never a test id) and is named in the help overlay: the
/// recordings page while recording, the tracklist editor, the USB screen
/// with an emergency result, the queue with requests and a do-not-play
/// track, the set alert, the stick's formats and the info panel on two
/// tracks.
#[test]
fn new_controls_are_tab_stops_named_and_in_the_help() {
    let _volumes = volumes_lock();
    let ui = Ui::start(Fixture::new(), (1440, 900), false);
    crate::engine::ensure_engine(&ui.app);
    let prefixes = ["rec.", "devices.emergency", "queue.", "app.set-alert", "info.", "browser.panel.queue", "browser.filter.unplayed"];
    let views: Vec<(&str, fn(&Ui))> = vec![
        ("recordings (recording)", |u| {
            u.app.ui.invoke_set_mode("prepare".into());
            u.app.ui.invoke_menu_action("view.layout.browser".into());
            finished_recording(u);
            u.app.ui.invoke_select_section("recordings".into());
            if !rec(u).get_recording() {
                u.app.ui.invoke_toggle_recording();
            }
            u.wait_until(5.0, |u| rec(u).get_recording());
            u.pump(300);
        }),
        ("tracklist editor", |u| {
            if rec(u).get_recording() {
                u.app.ui.invoke_toggle_recording();
            }
            let id = finished_recording(u);
            ed(u).invoke_edit(id);
            u.pump(200);
        }),
        ("usb emergency", |u| {
            ed(u).invoke_close();
            fake_emergency(u, "done");
            u.app.ui.invoke_select_section("devices".into());
            u.pump(200);
        }),
        ("queue and alert", |u| {
            u.app.state.borrow_mut().devices.emergency = None;
            u.app.ui.invoke_select_section("library".into());
            u.app.ui.invoke_set_mode("perform".into());
            u.wait_until(30.0, |u| u.has("mixer.ch1.fader"));
            fill_queue(u);
            u.app.ui.set_queue_open(true);
            fake_alert(u, "dnp");
            u.pump(300);
        }),
        ("info bulk and filter", |u| {
            fake_alert(u, "");
            u.app.ui.set_queue_open(false);
            u.app.ui.invoke_set_mode("prepare".into());
            u.app.ui.invoke_select_node(-1);
            u.app.ui.set_info_open(true);
            u.app.ui.set_filter_open(true);
            u.app.ui.invoke_row_clicked(0, false, false);
            u.app.ui.invoke_row_clicked(1, true, false);
            u.pump(300);
        }),
    ];
    let mut report = Vec::new();
    for (name, setup) in views {
        setup(&ui);
        let ours: Vec<super::access::Control> = super::access::controls(ui.app.ui.window())
            .into_iter()
            .filter(|c| prefixes.iter().any(|p| c.id.starts_with(p)))
            .collect();
        assert!(!ours.is_empty(), "{name}: the new controls are on screen");
        for b in super::access::bad_names(ui.app.ui.window()) {
            report.push(format!("{name}: {b}"));
        }
        // The help names each of them.
        crate::help::toggle(&ui.app);
        ui.pump(200);
        let items: Vec<crate::HelpItem> = ui.app.ui.global::<crate::Help>().get_items().iter().collect();
        for c in &ours {
            if !items.iter().any(|i| i.name.as_str() == c.name) {
                report.push(format!("{name}: not in the help: {}", c.show()));
            }
        }
        crate::help::toggle(&ui.app);
        ui.pump(100);
        // Then Tab from the window round to the start again (it scrolls
        // the panels, so it comes after the help).
        ui.app.ui.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Escape.into() });
        ui.app.ui.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: slint::platform::Key::Escape.into() });
        ui.pump(50);
        let (missed, ringless) = super::access::tab_walk(&ui);
        for m in missed.iter().chain(&ringless).filter(|m| prefixes.iter().any(|p| m.contains(&format!("`{p}")))) {
            report.push(format!("{name}: tab: {m}"));
        }
    }
    assert!(report.is_empty(), "{} problems:\n{}", report.len(), report.join("\n"));
}
