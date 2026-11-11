//! Roadmap 0.9.0, lighting: SETUP and SHOW with a large BLACKOUT and the
//! output test one key away ([21]), the stage as a floor plan editor with
//! fixture programming ([32]), an editable light cue list with its lane in
//! Prepare ([33]) and venue profiles with named rig and show files ([35]),
//! through the real window. Outputs go to a test link, never the network.

use super::harness::*;
use od_core::{Beatgrid, DataSource, TrackAnalysis, TrackId};
use od_lighting::{DimmerCurve, FileKind, Mode, OutputKind};
use slint::{ComponentHandle, Model};

fn g(ui: &Ui) -> crate::Lighting<'_> {
    ui.app.ui.global::<crate::Lighting>()
}

fn type_into(ui: &Ui, label: &str, text: &str) {
    ui.click(label);
    ui.key(slint::platform::Key::End);
    for _ in 0..32 {
        ui.key(slint::platform::Key::Backspace);
    }
    ui.type_text(text);
    ui.key(slint::platform::Key::Return);
    ui.pump(50);
}

fn rig(ui: &Ui) -> od_lighting::RigConfig {
    od_lighting::RigConfig::load(&crate::lighting::rig_path(&ui.fx.library)).unwrap()
}

/// The LIGHTING view with outputs on a test link.
fn lighting(ui: &Ui) {
    ui.app
        .lighting
        .borrow()
        .use_opener(od_lighting::TestLink::default().opener());
    ui.click("topbar.mode.lighting");
    ui.pump(200);
}

/// Three wash pars and a mover.
fn patch(ui: &Ui) {
    let gl = g(ui);
    gl.invoke_add_fixtures(2, "Par".into(), "Wash".into(), "3".into(), "".into(), "".into());
    gl.invoke_add_fixtures(3, "Spot".into(), "Movers".into(), "1".into(), "".into(), "".into());
    assert!(ui.wait_until(5.0, |u| u.app.lighting.borrow().config.patch.fixtures.len() == 4));
}

/// [21] An empty rig opens on SETUP (rig, patch, outputs, safety, files;
/// the floor plan; the output test one key away). SHOW holds the stage,
/// pads and cues with AUTO, the masters and keys, and a large BLACKOUT that
/// stays in view however the column scrolls, also in a small window.
#[test]
fn setup_and_show_with_a_large_blackout_and_the_output_test() {
    let ui = Ui::open((1440, 900));
    lighting(&ui);
    assert_eq!(g(&ui).get_space(), "setup");
    for l in ["lighting.setup.rig", "lighting.setup.files", "lighting.centre.patch", "lighting.plan", "lighting.test"] {
        assert!(ui.has(l), "{l} in SETUP");
    }
    for l in ["lighting.blackout", "lighting.mode.auto", "lighting.centre.pads", "lighting.centre.cues"] {
        assert!(!ui.has(l), "{l} not in SETUP");
    }
    patch(&ui);
    // The output test: one key starts the test pattern, the same key stops it.
    ui.click("lighting.test");
    assert!(ui.wait_until(5.0, |u| {
        let rt = u.app.lighting.borrow();
        rt.test == Some(od_lighting::TestPattern::Each)
            && rt.runner.snapshot.lock().unwrap().looks.iter().filter(|l| l.intensity > 0.5).count() == 1
    }));
    assert!(ui.value("lighting.test").is_empty() || g(&ui).get_test() == "each");
    ui.click("lighting.test");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().test.is_none()));

    // SHOW.
    ui.click("lighting.space.show");
    ui.pump(100);
    for l in ["lighting.blackout", "lighting.mode.auto", "lighting.master", "lighting.flash", "lighting.centre.pads", "lighting.centre.cues", "lighting.stage"] {
        assert!(ui.has(l), "{l} in SHOW");
    }
    for l in ["lighting.rig.add", "lighting.test", "lighting.centre.patch"] {
        assert!(!ui.has(l), "{l} not in SHOW");
    }
    let big = Rect::of(&ui.get("lighting.blackout"));
    let flash = Rect::of(&ui.get("lighting.flash"));
    assert!(big.h >= 56.0 && big.h > 1.5 * flash.h && big.w >= flash.w, "BLACKOUT is the largest key: {big:?} {flash:?}");
    // Scrolled to the end of the AUTO column, BLACKOUT is still in view.
    ui.scroll("lighting.auto", -3000.0);
    let view = Rect::of(&ui.get("lighting.view"));
    assert!(view.contains(&Rect::of(&ui.get("lighting.blackout")), 0.5));
    ui.click("lighting.blackout");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().config.settings.blackout));
    // A small window: the same.
    ui.resize((900, 640));
    ui.pump(300);
    let view = Rect::of(&ui.get("lighting.view"));
    let big = Rect::of(&ui.get("lighting.blackout"));
    assert!(view.contains(&big, 0.5) && big.h >= 56.0, "{view:?} {big:?}");
    // Back to SETUP: the centre follows to a part SETUP has.
    ui.click("lighting.centre.cues");
    ui.click("lighting.space.setup");
    assert_eq!(g(&ui).get_centre(), "stage");
    assert!(ui.has("lighting.plan"));
}

/// [32] Fixtures are dragged on the floor plan (snapping to the grid),
/// turned, raised and regrouped, and programmed (pan and tilt range,
/// invert, colour calibration, dimmer curve); the positions order chases
/// from left to right; all of it is saved with the rig.
#[test]
fn floor_plan_places_turns_and_tunes_fixtures() {
    let ui = Ui::open((1440, 900));
    lighting(&ui);
    patch(&ui);
    assert!(ui.wait_until(3.0, |u| u.has("lighting.plan.fixture3")));
    let plan = Rect::of(&ui.get("lighting.plan"));
    // The first par to the right edge, near the front.
    let to = slint::LogicalPosition::new(plan.x + plan.w * 0.93, plan.y + plan.h * 0.7);
    ui.drag_el(&ui.get("lighting.plan.fixture0"), to);
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().config.patch.fixtures[0].place.is_some()));
    let p = ui.app.lighting.borrow().config.patch.fixtures[0].place.unwrap();
    assert!(p.x > 0.8, "{p:?}");
    assert!(((p.x * 20.0).round() - p.x * 20.0).abs() < 1e-3, "on the grid: {p:?}");
    assert_eq!(g(&ui).get_selected_fixture(), 0, "the dragged fixture is open");
    // Its chase place: last from the left.
    assert_eq!(
        od_lighting::stage::stage_order(&ui.app.lighting.borrow().config.patch),
        // The mover's default spot (the middle of the back truss) ties
        // with the middle par: patch order decides.
        vec![1, 3, 2, 0]
    );
    // Turn, raise, regroup.
    ui.click("lighting.fixture.rotate.up");
    ui.click("lighting.fixture.rotate.up");
    ui.click("lighting.fixture.height.up");
    type_into(&ui, "lighting.fixture.group", "Front");
    assert!(ui.wait_until(3.0, |u| {
        let rt = u.app.lighting.borrow();
        let f = &rt.config.patch.fixtures[0];
        f.place.is_some_and(|p| p.rotation == 30.0 && p.height == 3.5) && f.group == "Front"
    }));
    assert_eq!(g(&ui).get_tune().rotation, "30°");
    // The mover: range, invert, colour, curve.
    ui.click("lighting.plan.fixture3");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.fixture.pan-from")));
    ui.click("lighting.fixture.pan-from.up");
    ui.click("lighting.fixture.tilt-to.down");
    ui.click("lighting.fixture.invert-tilt");
    let red = ui.scroll_into_view("lighting.plan.inspector", |u| u.find("lighting.fixture.red.down"));
    ui.click_el(&red.expect("colour calibration"));
    let curve = ui.scroll_into_view("lighting.plan.inspector", |u| u.find("lighting.fixture.curve.up"));
    ui.click_el(&curve.expect("dimmer curve"));
    assert!(ui.wait_until(3.0, |u| {
        let t = u.app.lighting.borrow().config.patch.fixtures[3].tune;
        t.pan == [0.05, 1.0] && t.tilt == [0.0, 0.95] && t.invert_tilt && t.color[0] == -0.05 && t.curve == DimmerCurve::Square
    }), "{:?}", ui.app.lighting.borrow().config.patch.fixtures[3].tune);
    // Saved with the rig; the default spot comes back on request.
    let saved = rig(&ui);
    assert_eq!(saved.patch.fixtures[0].group, "Front");
    assert_eq!(saved.patch.fixtures[3].tune.curve, DimmerCurve::Square);
    ui.click("lighting.plan.fixture0");
    let back = ui.scroll_into_view("lighting.plan.inspector", |u| u.find("lighting.fixture.unplace"));
    ui.click_el(&back.expect("DEFAULT SPOT"));
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().config.patch.fixtures[0].place.is_none()));
    // The setup PIN locks the floor plan too.
    {
        let mut rt = ui.app.lighting.borrow_mut();
        rt.config.safety.pin = "2468".into();
        rt.unlocked = false;
    }
    g(&ui).invoke_rotate_fixture(3, 1);
    ui.pump(50);
    assert!(ui.app.lighting.borrow().config.patch.fixtures[3].place.is_none(), "locked");
}

/// A 32 s, 120 BPM track with a constant grid (16 bars).
fn analysed_track(fx: &Fixture, title: &str) -> TrackId {
    let path = fx.root.join(format!("music/{title}.wav"));
    write_wav(&path, 32.0, 330.0);
    let mut lib = fx.lib();
    let id = crate::workers::import_file(&mut lib, &path).unwrap();
    let mut tags = lib.track(id).unwrap().tags;
    tags.title = title.into();
    lib.set_tags(id, tags).unwrap();
    let a = TrackAnalysis {
        analysis_version: od_analysis::ANALYSIS_VERSION,
        beatgrid: Beatgrid::constant(120.0, 0.0, 1, 32.0),
        ..Default::default()
    };
    lib.store_analysis(id, &a, DataSource::Analysis).unwrap();
    id
}

/// [33] The light cues of a track: added at the playhead, moved by bars,
/// renamed, pointed at a stack cue, a pad's look or a chase, given a fade,
/// deleted; in Prepare they sit on a lane under the waveform and drag to
/// another bar.
#[test]
fn light_cue_list_and_its_lane_in_prepare() {
    let fx = Fixture::new();
    let id = analysed_track(&fx, "Cue Lane");
    let ui = Ui::start(fx, (1440, 900), false);
    {
        let mut rt = ui.app.lighting.borrow_mut();
        let _ = od_lighting::add_fixtures(&mut rt.config.patch, 2, 2, None, None, "Par", "Wash");
        let s = &mut rt.config.settings;
        s.cues.push(od_lighting::LightCue { name: "Blue".into(), fade_beats: 2.0, ..Default::default() });
        s.pads.push(od_lighting::PadLook::new(0, "Red wash"));
        let mut chase = od_lighting::PadLook::new(1, "Run");
        chase.steps = vec![0];
        s.pads.push(chase);
    }
    // The track in the Prepare player, the cursor at 9 s (bar 5).
    {
        let mut st = ui.app.state.borrow_mut();
        st.player.current = Some(id);
        st.player.cursor = 9.0;
    }
    crate::player::refresh_detail(&ui.app);
    assert!(!ui.has("player1.light-cues"), "no lane without light cues");
    lighting(&ui);
    ui.click("lighting.space.show");
    ui.click("lighting.centre.cues");
    assert!(ui.wait_until(3.0, |u| g(u).get_cue_title() == "Cue Lane"));
    ui.click("lighting.cues.add");
    let cues = |u: &Ui| u.app.lighting.borrow().config.settings.track_cues.get(&id.0).cloned().unwrap_or_default();
    assert!(ui.wait_until(3.0, |u| cues(u).len() == 1));
    assert_eq!(cues(&ui)[0].beat, 16, "on the bar the cursor is in");
    assert!(ui.value("lighting.cues.row0").contains("CUE 1 BLUE"), "{}", ui.value("lighting.cues.row0"));
    assert!(ui.value("lighting.cues.row0").contains("2 BEATS"), "the stack cue's fade");
    // Plays: the next is the pad's look, then the chase.
    ui.click("lighting.cues.row0.plays.next");
    assert!(ui.wait_until(3.0, |u| cues(u)[0].pad == Some(0)));
    ui.click("lighting.cues.row0.plays.next");
    assert!(ui.wait_until(3.0, |u| cues(u)[0].pad == Some(1)));
    assert!(ui.value("lighting.cues.row0").contains("CHASE 1.2 RUN"));
    ui.click("lighting.cues.row0.fade.up");
    assert!(ui.wait_until(3.0, |u| cues(u)[0].fade_beats == Some(2.0)), "{:?}", cues(&ui));
    type_into(&ui, "lighting.cues.row0.name", "Drop");
    ui.click("lighting.cues.row0.later");
    assert!(ui.wait_until(3.0, |u| cues(u)[0].beat == 20 && cues(u)[0].name == "Drop"));
    // Saved with the show.
    assert_eq!(rig(&ui).settings.track_cues[&id.0][0].name, "Drop");

    // The lane in Prepare.
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.pump(200);
    assert!(ui.wait_until(3.0, |u| u.has("player1.light-cue0")));
    assert!(ui.value("player1.light-cue0").starts_with("DROP"), "{}", ui.value("player1.light-cue0"));
    let lane = Rect::of(&ui.get("player1.light-cues"));
    let mark = Rect::of(&ui.get("player1.light-cue0"));
    // At 10 s of 32.
    assert!((mark.x + 1.0 - (lane.x + lane.w * 10.0 / 32.0)).abs() < 2.0, "{mark:?} {lane:?}");
    // Dragged to about 24 s: it lands on the bar there (beat 48).
    let to = slint::LogicalPosition::new(lane.x + lane.w * 0.74, mark.y + mark.h / 2.0);
    ui.drag_points(slint::LogicalPosition::new(mark.x + 6.0, mark.y + mark.h / 2.0), to);
    assert!(ui.wait_until(3.0, |u| cues(u)[0].beat == 48), "{:?}", cues(&ui));
    // Delete in the list.
    ui.app.ui.invoke_set_mode("lighting".into());
    ui.pump(200);
    ui.click("lighting.cues.row0.delete");
    assert!(ui.wait_until(3.0, |u| cues(u).is_empty()));
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.pump(200);
    assert!(!ui.has("player1.light-cues"), "the lane goes with the last cue");
}

/// The label of the file row named `name`.
fn file_row(ui: &Ui, name: &str) -> Option<String> {
    ui.with_prefix("lighting.files.row")
        .into_iter()
        .find(|(_, v, _)| v == name || v.strip_suffix(" in use") == Some(name))
        .map(|(l, _, _)| l)
}

/// [35] The first start makes the rig a Default venue; venues, rigs and
/// shows are saved, loaded, renamed, duplicated and deleted as named
/// files; a venue brings its rig, show and safety limits; one file goes
/// out and comes back in.
#[test]
fn venue_profiles_rigs_and_shows_as_named_files() {
    let ui = Ui::open((1440, 900));
    lighting(&ui);
    assert_eq!(ui.value("lighting.venue"), "Default", "migrated");
    ui.click("lighting.setup.files");
    assert!(ui.wait_until(3.0, |u| u.has("lighting.files.row0")));
    assert!(ui.value("lighting.files.row0").contains("in use"));
    // A rig for the club, its show and limits, saved as the venue "Club".
    patch(&ui);
    {
        let mut rt = ui.app.lighting.borrow_mut();
        rt.config.settings.mode = Mode::Pro;
        rt.config.safety.strobe_hz = 2.0;
        rt.config.outputs.push((0, OutputKind::Sacn { universe: 1 }));
    }
    type_into(&ui, "lighting.files.name", "Club");
    assert!(ui.wait_until(3.0, |u| u.value("lighting.venue") == "Club"));
    let files = ui.app.lighting.borrow().files.clone();
    assert_eq!(files.list(FileKind::Venue), vec!["Club", "Default"]);
    // Arriving at the other venue: Default (an empty rig).
    ui.click("lighting.files.row1");
    ui.click("lighting.files.load");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().config.patch.fixtures.is_empty()));
    assert_eq!(ui.app.lighting.borrow().config.safety.strobe_hz, 3.0);
    // Back at the club: rig, show and limits.
    ui.click("lighting.files.row0");
    ui.click("lighting.files.load");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().config.patch.fixtures.len() == 4));
    {
        let rt = ui.app.lighting.borrow();
        assert_eq!((rt.config.settings.mode, rt.config.safety.strobe_hz), (Mode::Pro, 2.0));
        assert_eq!(rt.config.outputs.len(), 1);
    }
    // Shows: save the show alone under a new name, rename it.
    ui.click("lighting.files.show");
    type_into(&ui, "lighting.files.name", "Friday");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().config.in_use.show == "Friday"));
    let row = file_row(&ui, "Friday").expect("Friday listed");
    ui.click(&row);
    ui.click("lighting.files.name");
    ui.type_text("Saturday");
    ui.click("lighting.files.rename");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().config.in_use.show == "Saturday"));
    // Venues: duplicate and delete.
    ui.click("lighting.files.venue");
    ui.click("lighting.files.row0");
    ui.click("lighting.files.duplicate");
    assert!(ui.wait_until(3.0, |u| u.app.lighting.borrow().files.list(FileKind::Venue).contains(&"Club 2".to_string())));
    let row = file_row(&ui, "Club 2").expect("the copy listed");
    ui.click(&row);
    ui.click("lighting.files.delete");
    assert!(ui.wait_until(3.0, |u| !u.app.lighting.borrow().files.list(FileKind::Venue).contains(&"Club 2".to_string())));
    // One file out, and in again (as a second venue, the same rig and show).
    let out = ui.fx.root.join("club.json");
    crate::lighting_stage::export(&ui.app, &out);
    crate::lighting_stage::import(&ui.app, &out);
    assert!(ui.wait_until(3.0, |u| u.value("lighting.venue").starts_with("Club ")));
    assert_eq!(ui.app.lighting.borrow().config.patch.fixtures.len(), 4);
    // Locked with the PIN, files cannot be loaded.
    {
        let mut rt = ui.app.lighting.borrow_mut();
        rt.config.safety.pin = "2468".into();
        rt.unlocked = false;
    }
    g(&ui).invoke_file_load(1);
    ui.pump(50);
    assert!(ui.value("lighting.venue").starts_with("Club "), "locked");
    // The working file names the venue in use (the app and the MCP server
    // keep reading lighting.json).
    assert!(rig(&ui).in_use.venue.starts_with("Club"));
    let _ = g(&ui).get_file_rows().row_count();
}
