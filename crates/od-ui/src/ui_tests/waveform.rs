//! The zoomed waveform lanes in Perform: loop band, bar counter.

use super::flows::deck_views;
use super::harness::*;

fn perform_with_track(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
    crate::decks::load_track(&ui.app, 0, ui.fx.tracks[0].0, None);
    assert!(ui.wait_until(30.0, |u| deck_views(u).first().is_some_and(|d| d.loaded)), "deck 1 loads");
}

/// A running loop shows as an amber band over the lane, around the playhead.
#[test]
fn loop_shows_on_the_waveform() {
    let ui = Ui::open((1440, 900));
    perform_with_track(&ui);
    assert!(!ui.has("wave.loop1"), "no loop yet");
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::AutoLoop { beats: 4.0 });
    assert!(ui.wait_until(5.0, |u| u.has("wave.loop1")), "loop band shown");
    let band = Rect::of(&ui.get("wave.loop1"));
    let lane = Rect::of(&ui.get("wave.lane1"));
    let head = Rect::of(&ui.get("wave.playhead"));
    assert!(lane.contains(&band, 1.0), "inside the lane");
    assert!(band.w > 10.0 && band.x <= head.x + 1.0 && band.right() >= head.x, "around the playhead: {band:?}");
}

/// The bar counter is large, says what it counts, and a click picks the
/// next mode (stored like the preference).
#[test]
fn bar_counter_cycles_its_mode() {
    let ui = Ui::open((1440, 900));
    perform_with_track(&ui);
    assert!(ui.has("wave.counter1"));
    assert!(Rect::of(&ui.get("wave.counter1")).h >= 30.0, "readable from a distance");
    let mode = |u: &Ui| u.app.state.borrow().settings.view.bar_counter.clone();
    let before = mode(&ui);
    ui.click("wave.counter1");
    assert!(ui.wait_until(2.0, |u| mode(u) != before), "click picks the next mode");
    assert!(ui.wait_until(2.0, |u| deck_views(u)[0].counter_mode.as_str() == mode(u)));
}

/// Typing a BPM on the platter sets the tempo there (the range widens when
/// it must); a click on the tempo line steps the range.
#[test]
fn platter_takes_a_typed_bpm_and_steps_the_range() {
    let ui = Ui::open((1440, 900));
    perform_with_track(&ui);
    let bpm = |u: &Ui| crate::engine::snapshot(&u.app).unwrap().decks[0].bpm;
    assert!(ui.wait_until(10.0, |u| bpm(u) > 0.0), "analysed track has a BPM");
    let track = bpm(&ui);
    // The field opens on the BPM shown; Esc turns it back into the readout.
    ui.click("deck1.bpm-set");
    assert!(ui.wait_until(2.0, |u| u.has("deck1.bpm-input")), "field opens");
    assert_eq!(ui.value("deck1.bpm-input"), format!("{track:.2}"), "starts on the current BPM");
    ui.key(slint::platform::Key::Escape);
    assert!(ui.wait_until(2.0, |u| !u.has("deck1.bpm-input")), "Esc closes the field");
    ui.click("deck1.bpm-set");
    let target = (track * 1.12 * 100.0).round() / 100.0;
    ui.type_text(&format!("{target}"));
    ui.key(slint::platform::Key::Return);
    assert!(
        ui.wait_until(5.0, |u| (bpm(u) - target).abs() < 0.01),
        "deck plays at {target}, got {}",
        bpm(&ui)
    );
    let range = |u: &Ui| crate::engine::snapshot(&u.app).unwrap().decks[0].tempo_range;
    assert_eq!(range(&ui), od_engine::TempoRange::Sixteen, "+12 % needs ±16 %");
    ui.click("deck1.range-set");
    assert!(ui.wait_until(2.0, |u| range(u) == od_engine::TempoRange::Wide), "click steps the range");
}

/// The GRID key swaps the pads for the grid panel: the grid moves a beat
/// at the playhead, locks, and the gain correction is stored with the track.
#[test]
fn deck_grid_panel_edits_grid_and_gain() {
    let ui = Ui::open((1440, 900));
    perform_with_track(&ui);
    let id = ui.fx.tracks[0].0;
    let first = |u: &Ui| u.app.state.borrow().lib.beatgrid(id).unwrap().unwrap().beats[0].time;
    assert!(ui.wait_until(30.0, |u| u.app.state.borrow().lib.beatgrid(id).ok().flatten().is_some()));
    let before = first(&ui);
    assert!(ui.has("deck1.pad.A"));
    let places = |u: &Ui| [u.get("deck1.platter"), u.get("deck2.platter")].map(|e| Rect::of(&e).x);
    let before_places = places(&ui);
    ui.click("deck1.grid-panel");
    assert!(ui.wait_until(2.0, |u| u.has("deck1.grid.beat-later")), "panel opens");
    ui.pump(100);
    assert_eq!(places(&ui), before_places, "the decks keep their width and place");
    assert!(!ui.has("deck1.pad.A"), "in place of the pads");
    ui.click("deck1.grid.beat-later");
    assert!(ui.wait_until(2.0, |u| (first(u) - before).abs() > 0.1), "grid moved a beat");
    ui.click("deck1.grid.gain-up");
    assert!(ui.wait_until(2.0, |u| u.value("deck1.grid.gain") == "+0.5 dB"));
    assert_eq!(crate::deck_grid::stored_gain(&ui.app.state.borrow().lib, id), 0.5);
    ui.click("deck1.grid.lock");
    assert!(ui.wait_until(2.0, |u| u.app.state.borrow().lib.beatgrid(id).unwrap().unwrap().locked));
    let locked_at = first(&ui);
    ui.click("deck1.grid.beat-later");
    ui.pump(200);
    assert_eq!(first(&ui), locked_at, "a locked grid stays");
    ui.click("deck1.grid-panel");
    assert!(ui.wait_until(2.0, |u| u.has("deck1.pad.A")), "pads are back");
}

/// Synthetic kicks for the audio of track `id` (transients cache), once
/// the deck's own analysis of it is in (so it does not replace them).
fn kicks_on_deck(ui: &Ui, id: od_core::TrackId, kicks: impl Iterator<Item = f64>) {
    let hash = ui.fx.lib().track(id).unwrap().content_hash;
    assert!(ui.wait_until(20.0, |_| crate::transients::get(&hash).is_some()), "deck transients");
    crate::transients::insert(
        hash,
        kicks.map(|time| od_analysis::transients::Transient { time, strength: 1.0, low: true }).collect(),
    );
}

fn seek_deck(ui: &Ui, secs: f64) {
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: secs });
    assert!(ui.wait_until(5.0, |u| crate::engine::snapshot(&u.app)
        .is_some_and(|s| (s.decks[0].position - secs).abs() < 0.002)));
    ui.pump(50);
}

/// The GRID panel works at the playhead: the meter shows how far the grid
/// sits off the kicks, SET 1 HERE snaps the beat onto the kick you just
/// heard and makes it the 1, and two PINs on kicks solve the tempo; the
/// deck keeps its width.
#[test]
fn deck_grid_panel_meter_set_one_and_pins() {
    let ui = Ui::open((1440, 900));
    let id = ui.fx.tracks[0].0;
    ui.fx.lib().set_beatgrid(id, od_core::Beatgrid::constant(120.0, 0.0, 3, 8.0)).unwrap();
    perform_with_track(&ui);
    // Kicks 12 ms after the grid's beats.
    kicks_on_deck(&ui, id, (0..16).map(|k| 0.012 + k as f64 * 0.5));
    let places = |u: &Ui| [u.get("deck1.platter"), u.get("deck2.platter")].map(|e| Rect::of(&e).x);
    let before = places(&ui);
    ui.click("deck1.grid-panel");
    assert!(ui.wait_until(2.0, |u| u.has("deck1.grid.meter")), "panel with its meter");
    ui.pump(100);
    assert_eq!(places(&ui), before, "the deck keeps its width");
    seek_deck(&ui, 2.03);
    assert!(ui.wait_until(3.0, |u| u.value("deck1.grid.meter") == "+12 ms"), "meter: {}", ui.value("deck1.grid.meter"));
    assert_eq!(ui.value("deck1.grid.fit"), "72 %");
    // SET 1 HERE, 18 ms after the kick at 2.012 s.
    ui.click("deck1.grid.set-one");
    let grid = || ui.fx.lib().beatgrid(id).unwrap().unwrap();
    let g = grid();
    let i = g.nearest_beat(2.012).unwrap();
    assert!((g.beats[i].time - 2.012).abs() < 1e-6, "on the kick: {}", g.beats[i].time);
    assert_eq!(g.beats[i].bar_position, 1, "and the 1");
    assert!(ui.wait_until(3.0, |u| u.value("deck1.grid.meter") == "0 ms"), "meter: {}", ui.value("deck1.grid.meter"));
    // The music really runs at 121 BPM: PIN an early kick, then a late one.
    let p = 60.0 / 121.0;
    kicks_on_deck(&ui, id, (0..16).map(|k| 0.012 + k as f64 * p));
    seek_deck(&ui, 0.012 + p + 0.015);
    ui.click("deck1.grid.pin");
    assert_eq!(ui.value("deck1.grid.pin"), "1");
    seek_deck(&ui, 0.012 + 8.0 * p + 0.01);
    ui.click("deck1.grid.pin");
    assert_eq!(ui.value("deck1.grid.pin"), "2");
    let status = ui.app.ui.get_status_text().to_string();
    assert!(status.starts_with("Tempo 120.00 → 121.00 (2 pins"), "{status}");
    let g = grid();
    assert!((g.beats[3].bpm - 121.0).abs() < 1e-6, "{}", g.beats[3].bpm);
    for k in [1.0, 8.0] {
        assert!(g.beats.iter().any(|b| (b.time - (0.012 + k * p)).abs() < 1e-9), "pin {k}");
    }
    // The deck plays the new grid.
    assert_eq!(ui.app.decks.borrow().loaded(0).unwrap().grid, g);
}
