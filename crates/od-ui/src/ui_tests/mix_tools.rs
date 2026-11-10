//! For the mix: stems ready before the gig ([60]), effects for drops ([61])
//! and the key relation on the deck with the analysis confidence ([62]),
//! driven through the real window.

use super::harness::*;
use od_controller::{ControlAction, DeckAction, StemPart};
use od_core::{AnalysisConfidence, Beatgrid, DataSource, MusicalKey, TrackAnalysis, TrackId};
use od_engine::{DeckCommand, FxTail};
use slint::ComponentHandle;
use slint::platform::PointerEventButton;

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
}

fn snapshot(ui: &Ui) -> od_engine::EngineState {
    crate::engine::snapshot(&ui.app).expect("engine runs")
}

fn key(s: &str) -> MusicalKey {
    MusicalKey::parse(s).unwrap()
}

/// Adds a `secs` long 120 BPM track with `key`, analysed, with the given
/// confidence.
fn analysed(fx: &Fixture, title: &str, key: &str, confidence: Option<AnalysisConfidence>) -> TrackId {
    let path = fx.root.join(format!("music/{title}.wav"));
    write_wav(&path, 30.0, 330.0);
    let mut lib = fx.lib();
    let id = crate::workers::import_file(&mut lib, &path).unwrap();
    let mut tags = lib.track(id).unwrap().tags;
    tags.title = title.into();
    lib.set_tags(id, tags).unwrap();
    let a = TrackAnalysis {
        analysis_version: od_analysis::ANALYSIS_VERSION,
        beatgrid: Beatgrid::constant(120.0, 0.0, 1, 30.0),
        key: MusicalKey::parse(key),
        confidence,
        ..Default::default()
    };
    lib.store_analysis(id, &a, DataSource::Analysis).unwrap();
    id
}

fn load(ui: &Ui, deck: usize, id: TrackId) {
    crate::decks::load_track(&ui.app, deck, id, None);
    assert!(
        ui.wait_until(60.0, |u| crate::engine::snapshot(&u.app)
            .is_some_and(|s| s.decks[deck].track == Some(id))),
        "deck {} loads",
        deck + 1
    );
    ui.pump(200);
}

fn open_fx(ui: &Ui) {
    if !ui.app.ui.get_fx_panel() {
        ui.app.ui.invoke_toggle_fx_panel();
    }
    assert!(ui.wait_until(10.0, |u| u.has("fx.unit1.build16")), "FX panel with BUILD");
}

/// [61] BUILD 16 starts a build-up locked to the next 16-bar line (bars
/// left on the unit, the key lit), the same key stops it; TAIL steps off →
/// echo → reverb; the level shows in percent.
#[test]
fn build_up_tail_and_level_in_the_fx_panel() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    open_fx(&ui);
    ui.click("fx.unit1.build16");
    assert!(
        ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].build_bars == 16),
        "a 16-bar build-up runs"
    );
    let st = snapshot(&ui).tempo_fx[0];
    assert!(st.on && st.kind == od_engine::TempoFxKind::BuildUp);
    assert!(
        ui.wait_until(10.0, |u| u.value("fx.unit1.build16").starts_with('-')),
        "bars left on the key: {:?}",
        ui.value("fx.unit1.build16")
    );
    assert_eq!(ui.get("fx.unit1.build16").accessible_checked(), Some(true));
    ui.click("fx.unit1.build16");
    assert!(
        ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].build_bars == 0 && !snapshot(u).tempo_fx[0].on),
        "the lit key stops it"
    );
    // A controller can start one too.
    crate::controller::handle_action(
        &ui.app,
        ControlAction::TempoFx { unit: 2, action: od_controller::TempoFxAction::BuildUp(8) },
    );
    assert!(ui.wait_until(10.0, |u| snapshot(u).tempo_fx[1].build_bars == 8));

    assert_eq!(ui.value("fx.unit1.tail"), "TAIL OFF");
    ui.click("fx.unit1.tail");
    assert!(ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].tail == FxTail::Echo));
    ui.click("fx.unit1.tail");
    assert!(ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].tail == FxTail::Reverb));
    assert!(ui.wait_until(5.0, |u| u.value("fx.unit1.tail") == "TAIL REVERB"));

    // The level, large: a press near the right end of the bar sets it high.
    // (The percent text sits in the column the bar spans.)
    let text = Rect::of(&ui.get("fx.unit1.level"));
    let column = Rect::of(&ui.get("fx.unit1.tempo-fx"));
    ui.click_at(
        slint::LogicalPosition::new(column.x + 92.0 * 0.9, text.y + text.h / 2.0),
        PointerEventButton::Left,
    );
    assert!(
        ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].level > 0.7),
        "level from the bar: {}",
        snapshot(&ui).tempo_fx[0].level
    );
    assert!(ui.wait_until(5.0, |u| u.value("fx.unit1.level").ends_with('%')));
}

/// [61] XY: the unit turns into one surface; a press switches the idle
/// effect on with the parameter across and the level up, the release
/// switches it off again; XY again returns to the controls.
#[test]
fn xy_surface_plays_the_effect_while_held() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    open_fx(&ui);
    ui.click("fx.unit1.xy");
    assert!(ui.wait_until(10.0, |u| u.has("fx.unit1.xy-pad")), "the XY surface shows");
    assert!(!ui.has("fx.unit1.build16"), "in place of the controls");
    let pad = Rect::of(&ui.get("fx.unit1.xy-pad"));
    // Three quarters across, a quarter up.
    let p = slint::LogicalPosition::new(pad.x + pad.w * 0.75, pad.y + pad.h * 0.75);
    ui.press_at(p, PointerEventButton::Left);
    ui.pump(50);
    assert!(ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].on), "pressing plays it");
    let st = snapshot(&ui).tempo_fx[0];
    assert!((st.param - 0.5).abs() < 0.1, "param from across: {}", st.param);
    assert!((st.level - 0.25).abs() < 0.1, "level from up: {}", st.level);
    ui.release_at(p, PointerEventButton::Left);
    ui.pump(50);
    assert!(ui.wait_until(10.0, |u| !snapshot(u).tempo_fx[0].on), "letting go stops it");
    // An effect that was on stays on.
    crate::fx::toggle_on(&ui.app, 0);
    assert!(ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].on));
    ui.click_at(p, PointerEventButton::Left);
    ui.pump(200);
    assert!(snapshot(&ui).tempo_fx[0].on, "stays on after the XY press");
    ui.click("fx.unit1.xy");
    assert!(ui.wait_until(10.0, |u| u.has("fx.unit1.build16") && !u.has("fx.unit1.xy-pad")));
}

/// [61] ALL FX OFF (and the fx_panic mapping) stops every effect at once:
/// both units, their tails and the channel FX knobs.
#[test]
fn panic_key_stops_every_effect() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    open_fx(&ui);
    let on = |u: usize| crate::fx::send(&ui.app, u, od_engine::TempoFxCommand::SetOn(true));
    on(0);
    on(1);
    crate::engine::send(
        &ui.app,
        od_engine::Command::Channel(0, od_engine::ChannelCommand::SetChannelFxAmount(0.6)),
    );
    assert!(ui.wait_until(10.0, |u| {
        let s = snapshot(u);
        s.tempo_fx[0].on && s.tempo_fx[1].on && s.channels[0].channel_fx_amount > 0.5
    }));
    // The panic key sits inside the window, clear of the decks.
    let key = Rect::of(&ui.get("fx.panic"));
    let (w, h) = ui.size();
    assert!(key.x >= 0.0 && key.right() <= w && key.bottom() <= h, "inside the window: {key:?}");
    ui.click("fx.panic");
    assert!(
        ui.wait_until(10.0, |u| {
            let s = snapshot(u);
            s.tempo_fx.iter().all(|f| !f.on && !f.ringing) && s.channels[0].channel_fx_amount == 0.0
        }),
        "everything off"
    );
    assert!(ui.app.ui.get_status_text().contains("off"));
    // The mapping does the same.
    on(0);
    assert!(ui.wait_until(10.0, |u| snapshot(u).tempo_fx[0].on));
    crate::controller::handle_action(&ui.app, ControlAction::FxPanic);
    assert!(ui.wait_until(10.0, |u| !snapshot(u).tempo_fx[0].on));
}

/// [61] The FX panel's new lines fit: at 1440×900 and 1280×800 the panel
/// keeps BUILD, TAIL, XY and ALL FX OFF inside the window, nothing of it
/// overlaps another unit, and the decks keep their title and hot cue texts.
#[test]
fn fx_panel_fits_at_desktop_sizes() {
    for size in [(1440, 900), (1280, 800)] {
        let ui = Ui::open(size);
        perform(&ui);
        open_fx(&ui);
        ui.pump(300);
        let (w, h) = ui.size();
        for label in ["fx.unit1.build32", "fx.unit2.build32", "fx.unit1.tail", "fx.unit2.xy", "fx.panic", "fx.unit2.on"] {
            let r = Rect::of(&ui.get(label));
            assert!(
                r.x >= 0.0 && r.right() <= w + 0.5 && r.y >= 0.0 && r.bottom() <= h + 0.5 && r.w > 10.0,
                "{size:?}: {label} inside the window: {r:?}"
            );
        }
        let u1 = Rect::of(&ui.get("fx.unit1.xy"));
        let u2 = Rect::of(&ui.get("fx.unit2.assign.ch1"));
        assert!(u1.right() <= u2.x, "{size:?}: unit 1 ends before unit 2 starts");
        assert!(ui.has("deck1.title") && Rect::of(&ui.get("deck1.title")).h > 10.0);
    }
}

/// [62] The deck's key against the other playing deck: fits, then (key
/// shift a semitone up) a clash, spelled out for screen readers; nothing
/// while the other deck is paused.
#[test]
fn key_relation_against_the_playing_deck() {
    let fx = Fixture::new();
    let a = analysed(&fx, "Key One", "8A", None);
    let b = analysed(&fx, "Key Two", "9A", None);
    let ui = Ui::start(fx, (1440, 900), false);
    perform(&ui);
    load(&ui, 0, a);
    load(&ui, 1, b);
    assert!(!ui.has("deck1.key-relation"), "nothing while deck 2 is paused");
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::Play);
    crate::engine::deck_cmd(&ui.app, 1, DeckCommand::Play);
    assert!(
        ui.wait_until(10.0, |u| u.has("deck1.key-relation") && u.value("deck1.key-relation") == "fits"),
        "8A against 9A fits"
    );
    assert_eq!(ui.value("deck2.key-relation"), "fits");
    let words = ui.get("deck2.key-relation").accessible_description().unwrap_or_default().to_string();
    assert!(words.contains("deck 1") && words.contains("8A"), "spelled out: {words}");
    // Deck 2 a semitone up: 9A (Em) → Fm (4A), five steps from 8A.
    crate::decks::key_shift(&ui.app, 1, 1);
    assert!(
        ui.wait_until(10.0, |u| u.value("deck1.key-relation") == "clash"),
        "a semitone up clashes: {}",
        ui.value("deck1.key-relation")
    );
    assert_eq!(ui.value("deck2.key-relation"), "clash");
    // Five semitones down: 9A (Em) becomes 10A (Bm), two steps from 8A.
    crate::engine::deck_cmd(&ui.app, 1, DeckCommand::SetKeyShift(-5.0));
    assert!(
        ui.wait_until(10.0, |u| u.value("deck1.key-relation") == "energy"),
        "9A five semitones down is 10A: {}",
        ui.value("deck1.key-relation")
    );
    crate::engine::deck_cmd(&ui.app, 1, DeckCommand::Pause);
    assert!(ui.wait_until(10.0, |u| !u.has("deck1.key-relation")), "gone when deck 2 stops");
}

/// [62] How sure the analysis is: the INFO tab shows key and grid in
/// percent with UNSURE, USE takes the likely key (your key, one undo step)
/// and the grid at the other tempo; an unsure key shows its percent on the
/// deck as well.
#[test]
fn confidence_and_the_one_button_fix() {
    let fx = Fixture::new();
    let id = analysed(
        &fx,
        "Unsure Tune",
        "8A",
        Some(AnalysisConfidence { key: 0.3, key_alt: Some(key("8B")), grid: 0.4, bpm_alt: Some(60.0) }),
    );
    let ui = Ui::start(fx, (1440, 900), false);
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.pump(200);
    let row = ui.by_value("browser.row", "Unsure Tune").expect("row").1;
    ui.click_el(&row);
    ui.pump(200);
    ui.click("player1.cue-list.tab.info");
    // Conditional rows inside the cue list's flickable are made when it is
    // laid out: asking for a geometry there does that.
    let info_shown = |u: &Ui| {
        let _ = u.find("player1.bpm-range").map(|e| Rect::of(&e));
        u.has("player1.key-sure")
    };
    assert!(ui.wait_until(10.0, info_shown), "INFO shows the confidence");
    assert_eq!(ui.value("player1.key-sure"), "30 %");
    assert_eq!(ui.get("player1.key-sure").accessible_description().as_deref(), Some("unsure"));
    assert_eq!(ui.value("player1.grid-sure"), "40 %");
    assert!(ui.has("player1.reanalyse"), "ANALYSE AGAIN offered");
    ui.click("player1.key-sure.use");
    assert!(
        ui.wait_until(10.0, |u| u.fx.lib().track(id).unwrap().key == Some(key("8B"))),
        "the likely key is now the track's"
    );
    assert!(
        ui.wait_until(5.0, |u| info_shown(u) && u.value("player1.key-sure") == "100 %"),
        "your key is sure"
    );
    assert!(ui.wait_until(5.0, |u| info_shown(u) && u.has("player1.grid-sure.use")));
    ui.click("player1.grid-sure.use");
    assert!(
        ui.wait_until(10.0, |u| u.fx.lib().beatgrid(id).ok().flatten().and_then(|g| g.first_bpm()).is_some_and(|b| (b - 60.0).abs() < 0.01)),
        "the grid at the other tempo"
    );
    // On a deck, an unsure key says so on its caption (a fresh track).
    let fx2 = Fixture::new();
    let other = analysed(
        &fx2,
        "Unsure Two",
        "5A",
        Some(AnalysisConfidence { key: 0.41, key_alt: None, grid: 0.9, bpm_alt: None }),
    );
    let ui2 = Ui::start(fx2, (1440, 900), false);
    perform(&ui2);
    load(&ui2, 0, other);
    assert!(ui2.wait_until(10.0, |u| super::flows::deck_views(u)[0].key_unsure == "41 %"));
    assert!(ui2.wait_until(5.0, |u| u.has("deck1.key-sure") && u.value("deck1.key-sure") == "41 %"));
    // Narrower than the key: the header keeps its widths.
    assert!(Rect::of(&ui2.get("deck1.key-sure")).w <= Rect::of(&ui2.get("deck1.key")).w + 0.5);
}

/// A track of its own audio (`pitch`), so no other test shares its stem
/// cache file.
fn own_track(fx: &Fixture, title: &str, pitch: f32) -> TrackId {
    let path = fx.root.join(format!("music/{title}.wav"));
    write_wav(&path, 9.0, pitch);
    let mut lib = fx.lib();
    let id = crate::workers::import_file(&mut lib, &path).unwrap();
    let mut tags = lib.track(id).unwrap().tags;
    tags.title = title.into();
    lib.set_tags(id, tags).unwrap();
    id
}

fn with_stems(ui: &Ui, id: TrackId) {
    let track = ui.fx.lib().track(id).unwrap();
    let audio = od_decode::decode(&track.path, od_decode::Layout::Stereo).unwrap();
    let n = audio.samples.len();
    let stems = od_stems::Stems { sample_rate: audio.sample_rate, vocals: audio.samples, drums: vec![0.0; n], bass: vec![0.0; n] };
    ui.app.decks.borrow().stem_cache().store(&track.content_hash, &stems).unwrap();
}

fn without_stems(ui: &Ui, id: TrackId) {
    let track = ui.fx.lib().track(id).unwrap();
    let _ = ui.app.decks.borrow().stem_cache().remove(&track.content_hash);
    crate::stems_batch::forget_cached();
}

/// [60] Solo (right click, or held on a controller) quiets the other parts
/// and brings them back as they were; keep-only-this mutes them for good.
#[test]
fn stem_solo_and_keep_only_this() {
    let fx = Fixture::new();
    let id = own_track(&fx, "Solo Parts", 517.0);
    let ui = Ui::start(fx, (1440, 900), false);
    perform(&ui);
    with_stems(&ui, id);
    load(&ui, 0, id);
    assert!(ui.wait_until(60.0, |u| snapshot(u).decks[0].stems_available), "parts in");
    let levels = |u: &Ui| snapshot(u).decks[0].stem_levels;
    // Mute DRUMS first: the solo must give it back muted.
    crate::engine::deck_cmd(&ui.app, 0, DeckCommand::SetStemLevel { stem: od_engine::Stem::Drums, level: 0.0 });
    assert!(ui.wait_until(5.0, |u| levels(u)[1] == 0.0));
    ui.right_click_el(&ui.get("deck1.stem.vocal"));
    assert!(ui.wait_until(10.0, |u| levels(u) == [1.0, 0.0, 0.0, 0.0]), "only vocals: {:?}", levels(&ui));
    assert!(ui.wait_until(5.0, |u| u.value("deck1.stem.caption") == "SOLO"));
    assert_eq!(ui.get("deck1.stem.vocal").accessible_description().as_deref(), Some("solo"));
    ui.right_click_el(&ui.get("deck1.stem.vocal"));
    assert!(ui.wait_until(10.0, |u| levels(u) == [1.0, 0.0, 1.0, 1.0]), "back as before: {:?}", levels(&ui));
    assert!(ui.wait_until(5.0, |u| u.value("deck1.stem.caption") == "STEMS"));
    // Held on a controller: on while pressed, back on release.
    let solo = |on| ControlAction::Deck { deck: 0, action: DeckAction::StemSolo { stem: StemPart::Bass, on } };
    crate::controller::handle_action(&ui.app, solo(true));
    assert!(ui.wait_until(10.0, |u| levels(u) == [0.0, 0.0, 1.0, 0.0]));
    crate::controller::handle_action(&ui.app, solo(false));
    assert!(ui.wait_until(10.0, |u| levels(u) == [1.0, 0.0, 1.0, 1.0]));
    // Keep only INST (⇧-click): the others muted and they stay.
    ui.app.ui.global::<crate::DeckMix>().invoke_stem_keep(1, "inst".into());
    assert!(ui.wait_until(10.0, |u| levels(u) == [0.0, 0.0, 0.0, 1.0]), "only inst: {:?}", levels(&ui));
    assert!(ui.app.decks.borrow().stem_solo[0].is_none(), "no solo to undo");
    without_stems(&ui, id);
}

/// [60] Stems ahead for a whole list: the row menu queues every track of
/// the list; the STEMS column shows READY for parts in the cache, the
/// batch's state for the others, and a deck shows QUEUED on its keys.
#[test]
fn stems_ahead_for_the_list_and_the_stems_column() {
    let fx = Fixture::new();
    let a = own_track(&fx, "Ready Parts", 523.0);
    let b = own_track(&fx, "Queued Parts", 529.0);
    let c = own_track(&fx, "Deck Parts", 541.0);
    let ui = Ui::start(fx, (1440, 900), false);
    crate::stems_model::assume_model_installed(&ui.app);
    with_stems(&ui, a);
    // The STEMS column on (column menu).
    let cols = crate::browser::toggle_column(&crate::browser::sanitize_columns(&[]), "stems");
    ui.app.state.borrow_mut().change_settings(|s| s.browser.columns = cols.clone());
    crate::browser::apply_columns(&ui.app);
    crate::browser::refresh_tracks(&ui.app);
    assert!(ui.wait_until(10.0, |u| u.has("browser.header.stems")), "STEMS column shown");
    let row_of = |u: &Ui, title: &str| {
        u.with_prefix("browser.row")
            .into_iter()
            .find(|(l, v, _)| v == title && !l["browser.row".len()..].contains('.'))
            .map(|(l, _, _)| l)
    };
    let label_a = row_of(&ui, "Ready Parts").expect("row of Ready Parts");
    assert!(
        ui.wait_until(10.0, |u| u.find(&format!("{label_a}.stems")).is_some_and(|e| e.accessible_value().as_deref() == Some("ready"))),
        "parts in the cache read READY"
    );
    // Whole list from the row menu: the two without parts are queued (the
    // one with parts is skipped).
    let rows: Vec<_> = ui.with_prefix("browser.row").into_iter().filter(|(l, _, _)| !l["browser.row".len()..].contains('.')).collect();
    ui.right_click_el(&rows[0].2);
    assert!(ui.click_menu_item("Separate stems for the whole list"), "row menu offers the whole list");
    let n = crate::stems_batch::count();
    // Every track but the one with parts (fixture tracks other tests
    // separate may count as done).
    assert!(n.total >= 3 && n.total <= 5, "the tracks without parts: {n:?}");
    // Headless runs have no real model: the jobs end as failed, and the
    // column says so; the batch completes.
    assert!(ui.wait_until(60.0, |_| crate::stems_batch::count().finished()), "{:?}", crate::stems_batch::count());
    let label_b = row_of(&ui, "Queued Parts").expect("row of Queued Parts");
    assert!(
        ui.wait_until(10.0, |u| u.find(&format!("{label_b}.stems")).and_then(|e| e.accessible_value()).is_some_and(|v| v == "failed" || v == "ready")),
        "the column follows the batch"
    );
    // A queued track on a deck: QUEUED on its keys.
    crate::stems_batch::stage(&[], None, &[c]);
    perform(&ui);
    load(&ui, 1, c);
    assert!(ui.wait_until(10.0, |u| u.value("deck2.stem.caption") == "QUEUED"), "{}", ui.value("deck2.stem.caption"));
    assert_eq!(ui.value("deck2.stem.vocal"), "queued");
    without_stems(&ui, a);
}

/// [60] A track without parts loaded where parts were in use warns: on the
/// status line and as NO STEMS on the deck, until the parts arrive.
#[test]
fn loading_without_stems_where_they_were_in_use_warns() {
    let fx = Fixture::new();
    let a = own_track(&fx, "Used Parts", 547.0);
    let b = own_track(&fx, "No Parts", 557.0);
    let c = own_track(&fx, "Other No Parts", 563.0);
    let ui = Ui::start(fx, (1440, 900), false);
    perform(&ui);
    with_stems(&ui, a);
    load(&ui, 0, a);
    assert!(ui.wait_until(60.0, |u| snapshot(u).decks[0].stems_available));
    // The next track on the same deck has no parts.
    load(&ui, 0, b);
    assert!(
        ui.wait_until(10.0, |u| u.app.ui.get_status_text().contains("No stems")),
        "status warns: {}",
        ui.app.ui.get_status_text()
    );
    assert!(ui.wait_until(10.0, |u| u.value("deck1.stem.caption") == "NO STEMS"));
    // A track with no stems where none were in use: no warning.
    load(&ui, 1, c);
    ui.pump(300);
    assert_ne!(ui.value("deck2.stem.caption"), "NO STEMS");
    without_stems(&ui, a);
}

