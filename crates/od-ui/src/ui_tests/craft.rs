//! The craft of the controls (roadmap [45], [46], [49], [50]): the motion
//! timings and the reduced-motion switch, knobs with value arcs, centre
//! marks and a drag readout, fader caps and the tempo detent, meters with
//! peak hold, the calm waveform option and the calmer library table,
//! driven through the real window.

use super::harness::*;
use super::session::{prefs, prefs_el, stored};
use slint::ComponentHandle;
use slint::LogicalPosition;
use slint::platform::{PointerEventButton, WindowEvent};

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
}

fn load(ui: &Ui, deck: usize, t: usize) {
    crate::decks::load_track(&ui.app, deck, ui.fx.tracks[t].0, None);
    assert!(
        ui.wait_until(60.0, |u| crate::engine::snapshot(&u.app)
            .is_some_and(|s| s.decks[deck].track.is_some())),
        "deck {} loads",
        deck + 1
    );
}

/// Every element labelled `label` (part labels such as `knob.arc` are
/// outside the view scopes, so they are looked up directly).
fn all(ui: &Ui, label: &str) -> Vec<Rect> {
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui.app.ui, label)
        .map(|e| Rect::of(&e))
        .collect()
}

/// Elements labelled `label` whose centre lies inside `area`.
fn inside(ui: &Ui, label: &str, area: &Rect) -> usize {
    all(ui, label)
        .iter()
        .filter(|r| {
            let c = r.centre();
            c.x >= area.x - 1.0 && c.x <= area.right() + 1.0 && c.y >= area.y - 1.0 && c.y <= area.bottom() + 1.0
        })
        .count()
}

fn move_to(ui: &Ui, p: LogicalPosition) {
    ui.app.ui.window().dispatch_event(WindowEvent::PointerMoved { position: p });
    ui.pump(32);
}

/// [45] The four named timings run while motion is on and drop to 0 ms
/// with Reduce motion (Preferences › View); the switch is off on a fresh
/// install, since Slint does not report the system's setting.
#[test]
fn reduced_motion_stops_every_named_timing() {
    let ui = Ui::open((1440, 900));
    assert!(!crate::settings::Settings::default().view.reduced_motion, "off by default");
    let theme = ui.app.ui.global::<crate::Theme>();
    let timings = |t: &crate::Theme| [t.get_t_press(), t.get_t_toggle(), t.get_t_panel(), t.get_t_mode()];
    let on = timings(&theme);
    assert!(on.iter().all(|d| *d > 0), "timings while motion is on: {on:?}");
    assert!(on.windows(2).all(|w| w[0] < w[1]), "press < toggle < panel < mode: {on:?}");
    assert!(on[3] <= 300, "nothing slow: {on:?}");
    let w = prefs(&ui, "view");
    prefs_el(&w, "prefs.view.reduced-motion").mock_single_click(PointerEventButton::Left);
    ui.pump(100);
    assert!(stored(&ui).view.reduced_motion, "stored");
    assert_eq!(timings(&ui.app.ui.global::<crate::Theme>()), [0, 0, 0, 0], "reduced motion");
    assert!(!ui.app.ui.global::<crate::Theme>().get_motion());
    prefs_el(&w, "prefs.view.reduced-motion").mock_single_click(PointerEventButton::Left);
    ui.pump(100);
    assert_eq!(timings(&ui.app.ui.global::<crate::Theme>()), on, "back on");
}

/// [46] Knobs: bipolar knobs (trim, EQ, CH FX) carry a centre mark, a
/// unipolar one (headphone level) does not; the value arc appears once a
/// bipolar knob leaves the centre; dragging shows the value above the ring
/// (the same words as the legend) and the readout goes on release.
#[test]
fn knobs_show_arc_centre_mark_and_a_readout_while_dragged() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    assert!(ui.wait_until(10.0, |u| u.has("mixer.ch1.eq-high")));
    let knob = Rect::of(&ui.get("mixer.ch1.eq-high"));
    for id in ["mixer.ch1.trim", "mixer.ch1.eq-high", "mixer.ch1.eq-mid", "mixer.ch1.eq-low", "fx.channel-fx.ch1"] {
        let r = Rect::of(&ui.get(id));
        assert_eq!(inside(&ui, "knob.centre", &r), 1, "{id}: centre mark");
    }
    let level = Rect::of(&ui.get("global.phones.level"));
    assert_eq!(inside(&ui, "knob.centre", &level), 0, "a level knob has no centre");
    assert_eq!(inside(&ui, "knob.arc", &level), 1, "a level knob lights its arc from the start");
    assert_eq!(inside(&ui, "knob.arc", &knob), 0, "no arc at the centre");
    assert!(!ui.has("control.readout"), "no readout at rest");

    let from = knob.centre();
    ui.press_at(from, PointerEventButton::Left);
    ui.pump(32);
    assert!(ui.wait_until(2.0, |u| u.has("control.readout")), "readout while held");
    for i in 1..=8 {
        move_to(&ui, LogicalPosition::new(from.x, from.y - 3.0 * i as f32));
    }
    assert!(ui.wait_until(2.0, |u| u.value("mixer.ch1.eq-high.value") != "HI"), "the knob turned");
    let readout = Rect::of(&ui.get("control.readout"));
    let shown = ui.value("control.readout");
    assert_eq!(shown, ui.value("mixer.ch1.eq-high.value"), "readout = legend value");
    assert!(readout.bottom() <= knob.y + 1.0, "above the ring: {readout:?} / {knob:?}");
    assert!(readout.h <= 16.5, "a small plate");
    ui.release_at(LogicalPosition::new(from.x, from.y - 24.0), PointerEventButton::Left);
    ui.pump(100);
    assert!(!ui.has("control.readout"), "gone on release");
    let knob = Rect::of(&ui.get("mixer.ch1.eq-high"));
    assert_eq!(inside(&ui, "knob.arc", &knob), 1, "the arc shows the turn");
    ui.double_click("mixer.ch1.eq-high");
    assert!(ui.wait_until(2.0, |u| inside(u, "knob.arc", &knob) == 0), "reset clears the arc");
}

/// [46] Faders: a cap with its centre line on every fader, a centre detent
/// on the tempo fader only, and the tempo shown beside the cap (towards
/// the deck) while it is held.
#[test]
fn faders_have_caps_a_tempo_detent_and_a_readout() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    load(&ui, 0, 0);
    let pitch = Rect::of(&ui.get("deck1.pitch"));
    let fader = Rect::of(&ui.get("mixer.ch1.fader"));
    assert_eq!(inside(&ui, "fader.detent", &pitch), 1, "tempo detent");
    assert_eq!(inside(&ui, "fader.detent", &fader), 0, "no detent on a channel fader");
    assert_eq!(inside(&ui, "fader.cap", &pitch), 1);
    assert_eq!(inside(&ui, "fader.cap", &fader), 1);
    let cap = all(&ui, "fader.cap").into_iter().find(|c| pitch.contains(c, 1.0)).unwrap();
    // At 0 % the cap sits on the detent.
    let detent = all(&ui, "fader.detent").into_iter().find(|c| pitch.contains(c, 1.0)).unwrap();
    assert!((cap.centre().y - detent.centre().y).abs() <= 1.5, "cap {cap:?} on the detent {detent:?}");

    let at = cap.centre();
    ui.press_at(at, PointerEventButton::Left);
    ui.pump(32);
    move_to(&ui, LogicalPosition::new(at.x, at.y + 12.0));
    assert!(ui.wait_until(2.0, |u| u.has("control.readout")), "readout while held");
    let readout = Rect::of(&ui.get("control.readout"));
    assert!(readout.right() <= pitch.x + 1.0, "deck 1: the readout opens towards the deck");
    // The engine reports the new tempo a frame or two later.
    assert!(
        ui.wait_until(5.0, |u| u.value("deck1.pitch").starts_with('+')
            && u.value("control.readout") == u.value("deck1.pitch")),
        "shows the tempo, down is faster: {} / {}",
        ui.value("control.readout"),
        ui.value("deck1.pitch")
    );
    ui.release_at(LogicalPosition::new(at.x, at.y + 12.0), PointerEventButton::Left);
    ui.pump(100);
    assert!(!ui.has("control.readout"));
}

/// [46] Meters hold their peak: a playing channel shows a hold tick at or
/// above its level, and so does the master meter in the top bar.
#[test]
fn meters_hold_their_peak() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    load(&ui, 0, 0);
    crate::engine::send(&ui.app, od_engine::Command::Channel(0, od_engine::ChannelCommand::SetFader(1.0)));
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Play);
    assert!(ui.wait_until(20.0, |u| u.has("mixer.ch1.peak-hold")), "channel hold tick");
    let hold: f32 = ui.value("mixer.ch1.peak-hold").parse().unwrap();
    let view = {
        use slint::Model;
        ui.app.ui.get_channels().row_data(0).unwrap()
    };
    assert!(hold > 0.0 && view.hold > 0.0, "hold {}", view.hold);
    assert!(view.hold + 0.05 >= view.level, "the hold sits at or above the level: {} / {}", view.hold, view.level);
    assert!(ui.wait_until(20.0, |u| u.has("topbar.master-hold")), "master hold tick");
    assert!(ui.app.ui.get_master_hold() >= ui.app.ui.get_master_meter() - 1e-3);
}

/// [46] The new control anatomy adds no size: knobs, faders and the mixer
/// column keep their widths in Perform at 1440 × 900 and 1280 × 800.
#[test]
fn control_anatomy_keeps_the_perform_sizes() {
    for (size, knob) in [((1440, 900), 32.0), ((1280, 800), 24.0)] {
        let ui = Ui::open(size);
        perform(&ui);
        assert!(ui.wait_until(10.0, |u| u.has("mixer.ch1.eq-high")));
        let r = |l: &str| Rect::of(&ui.get(l));
        assert!((r("mixer.ch1.eq-high").w - knob).abs() <= 0.5, "{size:?}: EQ knob {}", r("mixer.ch1.eq-high").w);
        assert!((r("deck1.pitch").w - 30.0).abs() <= 0.5, "{size:?}: tempo fader");
        assert!((r("mixer.ch1.fader").w - 30.0).abs() <= 0.5, "{size:?}: channel fader");
        let column = r("fx.channel-fx.type").w + 8.0;
        assert!(column <= 125.0, "{size:?}: mixer column {column}");
        for (a, b) in [("deck1.pitch", "mixer.ch1.stem"), ("mixer.ch2.cue", "deck2.pitch")] {
            assert!(r(a).right() <= r(b).x, "{size:?}: {a} left of {b}");
        }
    }
}

/// [50] The calm waveform option (Preferences › View) is off by default;
/// on, the playhead is a crisp 1 px line and cached tiles are redrawn
/// (faint beat lines, softer colours); off brings the standard back.
#[test]
fn calm_waveform_option_draws_a_crisp_playhead() {
    let ui = Ui::open((1440, 900));
    assert!(!crate::settings::Settings::default().view.calm_waveform, "the default stays as is");
    perform(&ui);
    load(&ui, 0, 0);
    assert!(ui.wait_until(10.0, |u| u.has("wave.playhead")));
    let width = |u: &Ui| Rect::of(&u.get("wave.playhead")).w;
    assert!((width(&ui) - 2.0).abs() < 0.1, "standard playhead 2 px");
    let w = prefs(&ui, "view");
    let before = crate::render::waveform_generation();
    prefs_el(&w, "prefs.view.calm-waveform").mock_single_click(PointerEventButton::Left);
    ui.pump(100);
    assert!(stored(&ui).view.calm_waveform, "stored");
    assert!(w.get_calm_waveform());
    assert!(crate::render::waveform_generation() > before, "cached waveforms are redrawn");
    assert!(ui.wait_until(2.0, |u| (width(u) - 1.0).abs() < 0.1), "crisp 1 px playhead");
    prefs_el(&w, "prefs.view.calm-waveform").mock_single_click(PointerEventButton::Left);
    ui.pump(100);
    assert!(!stored(&ui).view.calm_waveform);
    assert!(ui.wait_until(2.0, |u| (width(u) - 2.0).abs() < 0.1), "standard again");
}

/// [49] A track on a deck carries that deck's mark at the row's left edge,
/// its numeral as the value (figure alignment: `browser::columns` tests).
#[test]
fn library_rows_carry_their_deck_mark() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    let (id, title) = ui.fx.tracks[1].clone();
    crate::decks::load_track(&ui.app, 1, id, None);
    assert!(
        ui.wait_until(30.0, |u| u
            .with_prefix("browser.row")
            .iter()
            .any(|(l, v, _)| l.ends_with(".deck") && v == "2")),
        "the loaded row carries deck 2"
    );
    let (label, _, mark) = ui
        .with_prefix("browser.row")
        .into_iter()
        .find(|(l, v, _)| l.ends_with(".deck") && v == "2")
        .unwrap();
    let row_label = label.trim_end_matches(".deck").to_string();
    assert_eq!(ui.value(&row_label), title, "on its own row");
    let row = Rect::of(&ui.get(&row_label));
    let m = Rect::of(&mark);
    assert!(m.x <= row.x + 2.0 && m.right() <= row.x + 32.0, "at the left edge: {m:?} / {row:?}");
}

/// [49] The lists column never draws over the track table: dragged to its
/// far left it stops where its content still fits (the three new-list
/// keys, a name and its count) and every element of the column stays
/// inside it; dragged far right it stops at its widest. A stored width
/// below the minimum is shown at the minimum. The player / browser
/// splitter keeps the browser and the part above apart in both
/// directions too. Prepare and Perform, desktop and laptop.
#[test]
fn browser_splitters_never_let_content_spill() {
    for (size, mode) in [((1440, 900), "prepare"), ((1280, 800), "perform")] {
        let ui = Ui::open(size);
        ui.app.ui.invoke_set_mode(mode.into());
        ui.pump(300);
        let check = |u: &Ui, when: &str| {
            let col = Rect::of(&u.get("browser.lists"));
            let table = Rect::of(&u.get("browser.list"));
            assert!(col.right() <= table.x + 0.5, "{size:?} {mode} {when}: column {col:?} meets table {table:?}");
            for prefix in ["browser.tree", "browser.rail"] {
                for (l, _, e) in u.with_prefix(prefix) {
                    let r = Rect::of(&e);
                    if r.w <= 0.0 || r.h <= 0.0 || r.y >= col.bottom() || r.bottom() <= col.y {
                        continue; // scrolled out of the tree's view
                    }
                    assert!(r.x >= col.x - 0.5 && r.right() <= col.right() + 0.5, "{size:?} {mode} {when}: {l} {r:?} outside {col:?}");
                    assert!(r.overlap(&table) <= 0.0, "{size:?} {mode} {when}: {l} over the table");
                }
            }
            col
        };
        let wide = check(&ui, "at rest");
        let split = Rect::of(&ui.get("browser.splitter")).centre();
        ui.drag_points(split, LogicalPosition::new(4.0, split.y));
        let narrow = check(&ui, "far left");
        assert!(narrow.w <= wide.w, "no wider after a drag left: {narrow:?}");
        let keys = ["browser.tree.new-set", "browser.tree.new-smart", "browser.tree.new-folder"].map(|k| Rect::of(&ui.get(k)));
        assert!(keys[2].right() <= narrow.right() + 0.5, "{size:?}: + FOLDER fits");
        // A second drag further left does not move it.
        let split = Rect::of(&ui.get("browser.splitter")).centre();
        ui.drag_points(split, LogicalPosition::new(4.0, split.y));
        assert!((Rect::of(&ui.get("browser.lists")).w - narrow.w).abs() <= 0.5, "stops at the content");
        let split = Rect::of(&ui.get("browser.splitter")).centre();
        ui.drag_points(split, LogicalPosition::new(size.0 as f32 - 4.0, split.y));
        let far = check(&ui, "far right");
        assert!(far.w > narrow.w + 100.0 && far.w <= 481.0, "widens up to 480 px: {far:?}");
        // A stored width under the minimum shows at the minimum.
        ui.app.ui.set_sidebar_width(60.0);
        ui.pump(200);
        let stored = check(&ui, "stored 60 px");
        assert!((stored.w - narrow.w).abs() <= 0.5, "{stored:?} vs {narrow:?}");

        // Player / browser: the browser keeps its height both ways.
        let up = Rect::of(&ui.get("browser.upper-splitter")).centre();
        ui.drag_points(up, LogicalPosition::new(up.x, 4.0));
        check(&ui, "upper split far up");
        let up = Rect::of(&ui.get("browser.upper-splitter")).centre();
        ui.drag_points(up, LogicalPosition::new(up.x, size.1 as f32 - 4.0));
        let col = check(&ui, "upper split far down");
        assert!(col.h >= 200.0, "{size:?} {mode}: the browser keeps its room: {col:?}");
        assert!(Rect::of(&ui.get("browser.upper-splitter")).bottom() <= col.y + 8.0, "splitter above the browser");
    }
}

/// [46] Club: the four keys beside the jog wrap it. Each key reaches to the
/// centre line (half a key gap short), so the corners are filled; a press
/// on the legend acts, a press on the jog never reaches a key, and the
/// keys are at least as wide as the old rectangles. Desktop and laptop.
#[test]
fn club_keys_wrap_the_jog() {
    for size in [(1440, 900), (1280, 800)] {
        let ui = Ui::open(size);
        perform(&ui);
        slint::ComponentHandle::global::<crate::Club>(&ui.app.ui).invoke_set_enabled(true);
        ui.pump(300);
        assert!(ui.wait_until(5.0, |u| u.has("deck1.club.jog")));
        let jog = Rect::of(&ui.get("deck1.club.jog"));
        let cx = jog.centre().x;
        let ring = jog.w / 2.0 + 5.0;
        for (key, left) in [("search-back", true), ("call-prev", true), ("search-fwd", false), ("call-next", false)] {
            let k = Rect::of(&ui.get(&format!("deck1.club.{key}")));
            let inner = if left { k.right() } else { k.x };
            assert!((inner - cx).abs() <= 2.5, "{size:?} {key}: inner edge {inner} at the centre line {cx}");
            // The old key ended a group gap before the jog's square.
            let old = (jog.x - 5.0 - 6.0) - if left { k.x } else { 2.0 * cx - k.right() };
            let reach = (cx - ring - 4.0) - if left { k.x } else { 2.0 * cx - k.right() };
            assert!(reach + 0.5 >= old, "{size:?} {key}: no smaller than before ({reach} vs {old})");
        }
        // The jog's centre belongs to the jog: pressing it calls nothing.
        let before = crate::engine::snapshot(&ui.app).map(|s| s.decks[0].position);
        ui.click_at(jog.centre(), slint::platform::PointerEventButton::Left);
        ui.pump(200);
        assert_eq!(crate::engine::snapshot(&ui.app).map(|s| s.decks[0].position), before, "{size:?}: the jog is not a key");
    }
}

/// [50] The waveform zone's view keys stand in one column (zoom in, zoom
/// out, centre, then the two height keys), each whole and inside the
/// zone's height; the lanes start right after the column. When the zone is
/// too short (four decks, collapsed) the height keys move behind ⋯. The
/// GRID badge sits inside the numeral column, clear of its divider.
#[test]
fn waveform_view_keys_stand_in_one_column() {
    let order = ["wave.zoom-in", "wave.zoom-out", "wave.zoom-reset", "wave.collapse", "wave.expand"];
    for (size, decks) in [((1440, 900), 2), ((1280, 800), 2), ((1440, 900), 4), ((1280, 800), 4)] {
        let ui = Ui::open(size);
        perform(&ui);
        if decks == 4 {
            ui.app.ui.invoke_set_deck_count(4);
            ui.pump(300);
        }
        load(&ui, 0, 0);
        assert!(ui.wait_until(10.0, |u| u.has("wave.lane1") && u.has("wave.zoom-in")));
        let lanes: Vec<Rect> = (1..=decks).map(|d| Rect::of(&ui.get(&format!("wave.lane{d}")))).collect();
        let (top, bottom) = (lanes[0].y, lanes[decks - 1].bottom());
        let shown: Vec<Rect> = order.iter().filter(|l| ui.has(l)).map(|l| Rect::of(&ui.get(l))).collect();
        assert!(shown.len() >= 2, "{size:?} x{decks}: the zoom keys stay");
        if shown.len() < 5 {
            assert!(ui.has("wave.more"), "{size:?} x{decks}: the rest behind ⋯");
        }
        // One column when all five fit, else two columns in the same order.
        let columns = if shown.len() == 5 && shown.iter().all(|k| (k.x - shown[0].x).abs() < 0.5) { 1 } else { 2 };
        let right = shown.iter().map(|k| k.right()).fold(0.0f32, f32::max);
        for (i, k) in shown.iter().enumerate() {
            if columns == 1 {
                assert!((k.x - shown[0].x).abs() < 0.5, "{size:?} x{decks}: one column");
            }
            assert!(k.y >= top - 0.5 && k.bottom() <= bottom + 0.5 + 26.0, "{size:?} x{decks}: key {i} inside the zone");
            if i > 0 {
                assert!(k.y >= shown[i - 1].y - 0.5, "{size:?} x{decks}: in order");
                for o in &shown[..i] {
                    assert!(k.overlap(o) <= 0.0, "{size:?} x{decks}: no overlap");
                }
            }
            assert!(lanes[0].x >= right - 0.5 && lanes[0].x - right <= 1.0, "{size:?} x{decks}: lanes start after the keys");
        }
        if ui.has("wave.grid-edit1") {
            let g = Rect::of(&ui.get("wave.grid-edit1"));
            assert!(g.x >= lanes[0].x && g.right() <= lanes[0].x + 44.0 - 2.0, "{size:?}: GRID clear of the divider: {g:?}");
        }
    }
}

/// [50] The playhead of the Perform lanes stands on the window's centre,
/// the mixer column's axis, whatever the width of the keys at the left;
/// the bar counter stays at the playhead.
#[test]
fn lane_playhead_sits_on_the_window_centre() {
    for size in [(1440, 900), (1280, 800)] {
        let ui = Ui::open(size);
        perform(&ui);
        load(&ui, 0, 0);
        load(&ui, 1, 1);
        assert!(ui.wait_until(10.0, |u| u.has("wave.playhead") && u.has("mixer.ch1.stem")));
        let head = Rect::of(&ui.get("wave.playhead")).centre().x;
        let centre = size.0 as f32 / 2.0;
        let mixer = (Rect::of(&ui.get("mixer.ch1.stem")).x + Rect::of(&ui.get("mixer.ch2.cue")).right()) / 2.0;
        assert!((head - centre).abs() <= 1.0, "{size:?}: playhead {head} vs window centre {centre}");
        assert!((head - mixer).abs() <= 1.0, "{size:?}: playhead {head} vs mixer centre {mixer}");
        let counter = Rect::of(&ui.get("wave.counter1"));
        assert!((counter.x - head).abs() <= 8.0, "{size:?}: counter at the playhead");
    }
}

/// Drags lane `deck` (1-based) by `dx` px in small steps, `hold` frames
/// per step, and returns before release (the caller releases).
fn drag_lane(ui: &Ui, deck: usize, dx: f32, steps: usize) -> LogicalPosition {
    let lane = Rect::of(&ui.get(&format!("wave.lane{deck}")));
    let from = LogicalPosition::new(lane.x + lane.w * 0.7, lane.y + lane.h / 2.0);
    ui.press_at(from, PointerEventButton::Left);
    ui.pump(16);
    let mut at = from;
    for i in 1..=steps {
        at = LogicalPosition::new(from.x + dx * i as f32 / steps as f32, from.y);
        move_to(ui, at);
    }
    at
}

/// [50] A paused deck scrubs: dragging the lane moves the track 1:1 with
/// the pointer at the current zoom (dragging left moves later), a readout
/// shows the offset at the playhead, and the track stays where it is put.
/// A click without movement does not move it.
#[test]
fn dragging_a_paused_lane_scrubs() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    load(&ui, 0, 0);
    let pos = |u: &Ui| crate::engine::snapshot(&u.app).unwrap().decks[0].position;
    assert!(ui.wait_until(10.0, |u| u.has("wave.lane1") && !crate::engine::snapshot(&u.app).unwrap().decks[0].playing));
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Seek { seconds: 4.0 });
    assert!(ui.wait_until(5.0, |u| (pos(u) - 4.0).abs() < 0.01));
    let lane = Rect::of(&ui.get("wave.lane1"));
    ui.click_at(LogicalPosition::new(lane.x + lane.w * 0.7, lane.y + lane.h / 2.0), PointerEventButton::Left);
    ui.pump(200);
    assert!((pos(&ui) - 4.0).abs() < 0.01, "a click does not scrub");
    let span = {
        use slint::Model;
        ui.app.ui.get_lane_motion().row_data(0).unwrap().span as f64
    };
    let area = lane.w as f64 - 44.0;
    let at = drag_lane(&ui, 1, -120.0, 12);
    assert!(ui.wait_until(2.0, |u| u.has("wave.drag1")), "readout while dragging");
    assert!(ui.value("wave.drag1").ends_with(" s"), "{}", ui.value("wave.drag1"));
    ui.release_at(at, PointerEventButton::Left);
    ui.pump(200);
    let want = 4.0 + 120.0 / area * span;
    assert!(ui.wait_until(5.0, |u| (pos(u) - want).abs() < 0.05), "scrubbed to {want:.2}, at {:.2}", pos(&ui));
    ui.pump(500);
    assert!((pos(&ui) - want).abs() < 0.05, "stays where it was put");
    assert!(!ui.has("wave.drag1"));
}

/// [50] A playing deck never jumps when its lane is dragged: the drag's
/// speed bends the tempo (the deck runs faster while dragged towards the
/// future), the position stays continuous, and on release the deck plays
/// at its tempo again.
#[test]
fn dragging_a_playing_lane_nudges() {
    let ui = Ui::open((1440, 900));
    perform(&ui);
    load(&ui, 0, 0);
    crate::engine::deck_cmd(&ui.app, 0, od_engine::DeckCommand::Play);
    let snap = |u: &Ui| crate::engine::snapshot(&u.app).unwrap().decks[0];
    assert!(ui.wait_until(10.0, |u| snap(u).playing && snap(u).position > 0.2));
    let tempo = snap(&ui).speed;
    let before = snap(&ui).position;
    let at = drag_lane(&ui, 1, -60.0, 15);
    assert!(ui.wait_until(2.0, |u| u.has("wave.drag1")));
    assert!(ui.value("wave.drag1").ends_with('%'), "{}", ui.value("wave.drag1"));
    let during = snap(&ui).position;
    assert!(during >= before - 0.01 && during - before < 2.0, "no jump: {before:.2} → {during:.2}");
    ui.release_at(at, PointerEventButton::Left);
    ui.pump(100);
    assert!(
        ui.wait_until(5.0, |u| (snap(u).speed - tempo).abs() < 1e-3),
        "back to its tempo: {} vs {tempo}",
        snap(&ui).speed
    );
    assert!(snap(&ui).position >= during - 0.01, "still no jump back");
}

/// [46] The deck's centre: the platter, its setup-key block and SYNC / CUE
/// / PLAY share one horizontal axis; the keys hug the platter (their boxes
/// start at the ring, one key gap from it at the axis) and the two decks
/// mirror each other. At 1440 × 900 and 1280 × 800 nothing gets smaller.
#[test]
fn deck_centre_groups_round_the_platter() {
    for (size, platter, key) in [((1440, 900), 166.0, 24.0), ((1280, 800), 136.0, 24.0), ((1440, 1200), 166.0, 24.0), ((2560, 1440), 166.0, 24.0)] {
        let ui = Ui::open(size);
        perform(&ui);
        load(&ui, 0, 0);
        ui.pump(300);
        let r = |l: &str| Rect::of(&ui.get(l));
        for d in [1, 2] {
            let p = r(&format!("deck{d}.platter"));
            assert!(p.w + 0.5 >= platter, "{size:?} deck {d}: platter {}", p.w);
            let axis = p.centre().y;
            let keys: Vec<Rect> = ["slip", "quantize", "keylock", "tempo-mode"].iter().map(|k| r(&format!("deck{d}.{k}"))).collect();
            let block = (keys[0].y + keys[3].bottom()) / 2.0;
            assert!((block - axis).abs() <= 1.5, "{size:?} deck {d}: key block on the axis ({block} vs {axis})");
            let transport = (r(&format!("deck{d}.beat-sync")).y + r(&format!("deck{d}.play")).bottom()) / 2.0;
            assert!((transport - axis).abs() <= 2.0, "{size:?} deck {d}: transport on the axis ({transport} vs {axis})");
            for (i, k) in keys.iter().enumerate() {
                assert!(k.h + 0.5 >= key, "{size:?} deck {d}: key {i} {}", k.h);
                if i > 0 {
                    assert!((k.y - keys[i - 1].bottom() - 4.0).abs() <= 0.5, "{size:?} deck {d}: a fixed key gap");
                }
                // Deck 1 has its keys right of the platter, deck 2 left.
                if d == 1 {
                    assert!(k.x >= p.centre().x && k.right() > p.right(), "{size:?}: deck 1 keys right of the platter");
                } else {
                    assert!(k.right() <= p.centre().x && k.x < p.x, "{size:?}: deck 2 keys left of the platter");
                }
            }
            // The keys' click centre is on the key, not on the platter.
            let k = &keys[1];
            let c = k.centre();
            let dist = ((c.x - p.centre().x).powi(2) + (c.y - axis).powi(2)).sqrt();
            assert!(dist > p.w / 2.0 + 4.0, "{size:?} deck {d}: key centre clear of the ring");
        }
        // Mirror: the decks' platters sit symmetric about the window centre.
        let (p1, p2) = (r("deck1.platter"), r("deck2.platter"));
        let centre = size.0 as f32 / 2.0;
        assert!(((centre - p1.centre().x) - (p2.centre().x - centre)).abs() <= 5.0, "{size:?}: decks mirror: {} vs {}", centre - p1.centre().x, p2.centre().x - centre);
    }
}
