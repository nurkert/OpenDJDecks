//! LIGHTING, roadmap 0.9.0: the stage as an editor (floor plan, fixture
//! programming, [32]), the editable light cue list and its lane in Prepare
//! ([33]), and venue profiles with named rig and show files ([35]).

use std::rc::Rc;

use od_core::{Beatgrid, TrackId};
use od_lighting::files::{FileKind, Files};
use od_lighting::looks;
use od_lighting::{DimmerCurve, FixtureRole, Placement, RigConfig, TrackCue};
use slint::{ComponentHandle, SharedString};

use crate::app::{App, handler};
use crate::lighting::{LightingRuntime, each, edit, refresh, rgb};
use crate::lighting_pads::pad_number;
use crate::{LightCueRow, LightFileRow, LightPlaceRow, LightTuneView};

// ── [32] Floor plan and fixture programming ─────────────────────────────

/// Rows of the floor plan, with the live colours of `looks`.
pub fn place_rows(patch: &od_lighting::Patch, looks: &[od_lighting::Look]) -> Vec<LightPlaceRow> {
    let pos = od_lighting::stage::positions(patch);
    patch
        .fixtures
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let look = looks.get(i).cloned().unwrap_or_default();
            LightPlaceRow {
                name: f.name.as_str().into(),
                group: f.group.as_str().into(),
                role: match f.def.role() {
                    FixtureRole::Wash => "wash",
                    FixtureRole::Mover => "mover",
                    FixtureRole::Bar => "bar",
                    FixtureRole::Effect => "effect",
                }
                .into(),
                x: pos[i].0,
                y: pos[i].1,
                rotation: f.place.map_or(0.0, |p| p.rotation),
                placed: f.place.is_some(),
                color: rgb(look.color),
                intensity: look.intensity.clamp(0.0, 1.0),
            }
        })
        .collect()
}

fn percent(v: f32) -> String {
    format!("{} %", (v * 100.0).round() as i32)
}

fn signed_percent(v: f32) -> String {
    let p = (v * 100.0).round() as i32;
    if p > 0 { format!("+{p} %") } else { format!("{p} %") }
}

pub(crate) fn curve_label(c: DimmerCurve) -> String {
    od_i18n::tr(match c {
        DimmerCurve::Linear => "LINEAR",
        DimmerCurve::Square => "SQUARE",
        DimmerCurve::Root => "ROOT",
        DimmerCurve::Soft => "SOFT",
    })
    .to_string()
}

/// The open fixture's place and programming.
pub fn tune_view(rt: &LightingRuntime) -> LightTuneView {
    let Some(f) = rt.selected.and_then(|i| rt.config.patch.fixtures.get(i)) else {
        return LightTuneView::default();
    };
    let t = &f.tune;
    let p = f.place.unwrap_or(Placement::new(0.5, 0.5));
    LightTuneView {
        name: f.name.as_str().into(),
        group: f.group.as_str().into(),
        rotation: format!("{}°", p.rotation.round() as i32).into(),
        height: format!("{:.1} m", p.height).into(),
        pan_from: percent(t.pan[0]).into(),
        pan_to: percent(t.pan[1]).into(),
        tilt_from: percent(t.tilt[0]).into(),
        tilt_to: percent(t.tilt[1]).into(),
        invert_pan: f.invert_pan,
        invert_tilt: t.invert_tilt,
        red: signed_percent(t.color[0]).into(),
        green: signed_percent(t.color[1]).into(),
        blue: signed_percent(t.color[2]).into(),
        curve: curve_label(t.curve).into(),
        mover: f.def.role() == FixtureRole::Mover,
        placed: f.place.is_some(),
    }
}

/// Changes one programming value of fixture `i` by `d` steps (pan and
/// tilt ranges and colour in 5 % steps, the curve through its choices,
/// the inverts on and off).
pub fn step_tune(patch: &mut od_lighting::Patch, i: usize, field: &str, d: i32) {
    let Some(f) = patch.fixtures.get_mut(i) else { return };
    let t = &mut f.tune;
    let step = 0.05 * d as f32;
    let round = |v: f32| (v * 20.0).round() / 20.0;
    match field {
        "pan-from" => t.pan[0] = round((t.pan[0] + step).clamp(0.0, t.pan[1] - 0.05)),
        "pan-to" => t.pan[1] = round((t.pan[1] + step).clamp(t.pan[0] + 0.05, 1.0)),
        "tilt-from" => t.tilt[0] = round((t.tilt[0] + step).clamp(0.0, t.tilt[1] - 0.05)),
        "tilt-to" => t.tilt[1] = round((t.tilt[1] + step).clamp(t.tilt[0] + 0.05, 1.0)),
        "red" | "green" | "blue" => {
            let c = ["red", "green", "blue"].iter().position(|x| *x == field).unwrap_or(0);
            t.color[c] = round((t.color[c] + step).clamp(-0.5, 0.5));
        }
        "curve" => {
            let all = DimmerCurve::ALL;
            let n = all.len() as i32;
            let at = all.iter().position(|c| *c == t.curve).unwrap_or(0) as i32;
            t.curve = all[(at + d).rem_euclid(n) as usize];
        }
        "invert-pan" => f.invert_pan = !f.invert_pan,
        "invert-tilt" => t.invert_tilt = !t.invert_tilt,
        _ => {}
    }
}

/// The placement of fixture `i`, made from its default spot when it has
/// none yet.
fn place_of(patch: &mut od_lighting::Patch, i: usize) -> Option<&mut Placement> {
    let (x, y) = *od_lighting::stage::positions(patch).get(i)?;
    let f = patch.fixtures.get_mut(i)?;
    Some(f.place.get_or_insert(Placement::new(x, y)))
}

// ── [33] The light cue list ─────────────────────────────────────────────

/// What a light cue can play: a cue of the stack or a pad with a look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plays {
    Cue(usize),
    Pad(u16),
}

/// Everything a light cue can play, in stepping order: the stack, then
/// the pads with a look or chase.
pub fn targets(s: &od_lighting::ShowSettings) -> Vec<Plays> {
    let mut pads: Vec<u16> = s.pads.iter().map(|p| p.pad).collect();
    pads.sort_unstable();
    (0..s.cues.len()).map(Plays::Cue).chain(pads.into_iter().map(Plays::Pad)).collect()
}

fn plays_of(c: &TrackCue) -> Plays {
    match c.pad {
        Some(p) => Plays::Pad(p),
        None => Plays::Cue(c.cue),
    }
}

/// A name cut to `n` characters with an ellipsis (the cue list's PLAYS
/// keeps its width; the lane's tooltip has the whole name).
fn short(name: &str, n: usize) -> String {
    if name.chars().count() <= n {
        name.to_string()
    } else {
        format!("{}…", name.chars().take(n - 1).collect::<String>().trim_end())
    }
}

/// [`plays_label`] with the name cut short.
pub fn plays_short(s: &od_lighting::ShowSettings, p: Plays) -> String {
    plays_with(s, p, |n| short(n, 10))
}

/// "CUE 2 DROP", "PAD 1.3 RED WASH", "CHASE 2.1 COLOUR RUN".
pub fn plays_label(s: &od_lighting::ShowSettings, p: Plays) -> String {
    plays_with(s, p, str::to_string)
}

fn plays_with(s: &od_lighting::ShowSettings, p: Plays, name: impl Fn(&str) -> String) -> String {
    match p {
        Plays::Cue(i) => match s.cues.get(i) {
            Some(c) => od_i18n::trf("CUE {} {}", &[&(i + 1), &name(&c.name).to_uppercase()]),
            None => od_i18n::trf("CUE {} MISSING", &[&(i + 1)]),
        },
        Plays::Pad(p) => match looks::find(&s.pads, p) {
            Some(l) if l.is_chase() => {
                od_i18n::trf("CHASE {} {}", &[&pad_number(p), &name(&l.name).to_uppercase()])
            }
            Some(l) => od_i18n::trf("PAD {} {}", &[&pad_number(p), &name(&l.name).to_uppercase()]),
            None => od_i18n::trf("PAD {} EMPTY", &[&pad_number(p)]),
        },
    }
}

/// Fade steps in beats.
pub const FADES: [f32; 7] = [0.0, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

pub fn fade_label(beats: f32) -> String {
    if beats <= 0.0 {
        od_i18n::tr("AT ONCE").to_string()
    } else if beats < 1.0 {
        od_i18n::tr("½ BEAT").to_string()
    } else if beats == 1.0 {
        od_i18n::tr("1 BEAT").to_string()
    } else {
        od_i18n::trf("{} BEATS", &[&(beats as u32)])
    }
}

/// The track whose light cues the list shows: the one you hear (with its
/// playhead beat), else the one in the Prepare player (with the cursor's).
pub(crate) fn cue_track(app: &App) -> Option<(TrackId, Option<f64>, String)> {
    if let Some((id, beat, title)) = app.lighting.borrow().focus.clone() {
        return Some((id, Some(beat), title));
    }
    let st = app.state.borrow();
    let id = st.player.current?;
    let t = st.src().track(id).ok()?;
    let grid = st.src().analysis(id).ok()?.beatgrid;
    Some((id, grid.beat_position(st.player.cursor), t.tags.title))
}

/// The list of light cues of `track`, sorted by beat.
fn cues_mut(rt: &mut LightingRuntime, track: TrackId) -> &mut Vec<TrackCue> {
    let list = rt.config.settings.track_cues.entry(track.0).or_default();
    list.sort_by_key(|c| c.beat);
    list
}

/// Rows of the cue list for `track`.
pub fn cue_rows(rt: &LightingRuntime, track: Option<TrackId>) -> Vec<LightCueRow> {
    let s = &rt.config.settings;
    let Some(id) = track else { return Vec::new() };
    let mut list = s.track_cues.get(&id.0).cloned().unwrap_or_default();
    list.sort_by_key(|c| c.beat);
    list.iter()
        .map(|c| LightCueRow {
            bar: od_i18n::trf("BAR {}", &[&(c.beat / 4 + 1)]).into(),
            name: c.name.as_str().into(),
            plays: plays_short(s, plays_of(c)).into(),
            fade: fade_label(c.fade(&s.cues)).into(),
            active: rt.track_cue == Some((id.0, c.beat)),
        })
        .collect()
}

/// What a new light cue plays: the latched pad, else the cue the stack
/// shows (PRO), else the first cue of the stack, else the first pad.
fn default_target(rt: &LightingRuntime, stack_cue: usize) -> Option<Plays> {
    let s = &rt.config.settings;
    if let Some(p) = rt.pads.latched.filter(|p| looks::find(&s.pads, *p).is_some()) {
        return Some(Plays::Pad(p));
    }
    if !s.cues.is_empty() {
        return Some(Plays::Cue(stack_cue.min(s.cues.len() - 1)));
    }
    targets(s).first().copied()
}

fn set_plays(c: &mut TrackCue, p: Plays) {
    match p {
        Plays::Cue(i) => {
            c.cue = i;
            c.pad = None;
        }
        Plays::Pad(pad) => c.pad = Some(pad),
    }
}

/// Seconds of grid beat `beat` (past the last beat at the last tempo).
pub fn beat_time(grid: &Beatgrid, beat: u32) -> Option<f64> {
    if let Some(b) = grid.beats.get(beat as usize) {
        return Some(b.time);
    }
    let last = grid.beats.last()?;
    let extra = beat as f64 - (grid.beats.len() - 1) as f64;
    Some(last.time + extra * 60.0 / last.bpm.max(1.0))
}

/// The first beat of the bar nearest to `time`.
pub fn bar_at(grid: &Beatgrid, time: f64) -> Option<u32> {
    let b = grid.beat_position(time)?;
    Some(((b / 4.0).round().max(0.0) as u32) * 4)
}

/// Marks of the lane under the Prepare waveform.
pub fn lane_marks(
    s: &od_lighting::ShowSettings,
    track: TrackId,
    grid: &Beatgrid,
    duration: f64,
) -> Vec<crate::LightCueMark> {
    let mut list = s.track_cues.get(&track.0).cloned().unwrap_or_default();
    list.sort_by_key(|c| c.beat);
    list.iter()
        .filter_map(|c| {
            let t = beat_time(grid, c.beat)?;
            let plays = plays_label(s, plays_of(c));
            Some(crate::LightCueMark {
                pos: (t / duration.max(0.001)).clamp(0.0, 1.0) as f32,
                label: if c.name.is_empty() { plays.clone() } else { c.name.to_uppercase() }.into(),
                plays: plays.into(),
            })
        })
        .collect()
}

/// The lane of the Prepare player's track.
pub(crate) fn sync_lane(app: &App) {
    let marks = {
        let st = app.state.borrow();
        let rt = app.lighting.borrow();
        st.player.current.and_then(|id| {
            let t = st.src().track(id).ok()?;
            let grid = st.src().analysis(id).ok()?.beatgrid;
            Some(lane_marks(&rt.config.settings, id, &grid, t.duration))
        })
    }
    .unwrap_or_default();
    app.lighting.borrow().lane_rows.sync_eq(marks);
}

// ── [35] Venue profiles, rigs and shows ─────────────────────────────────

pub(crate) fn kind_of(k: &str) -> FileKind {
    match k {
        "rig" => FileKind::Rig,
        "show" => FileKind::Show,
        _ => FileKind::Venue,
    }
}

fn kind_id(k: FileKind) -> &'static str {
    match k {
        FileKind::Venue => "venue",
        FileKind::Rig => "rig",
        FileKind::Show => "show",
    }
}

/// Rows of the files of `kind`.
pub fn file_rows(files: &Files, kind: FileKind, config: &RigConfig) -> Vec<LightFileRow> {
    files
        .list(kind)
        .into_iter()
        .map(|name| {
            let detail = match kind {
                FileKind::Venue => files
                    .venue(&name)
                    .map(|v| {
                        od_i18n::trf(
                            "RIG {} · SHOW {} · STROBE CAP {}",
                            &[&v.rig.to_uppercase(), &v.show.to_uppercase(), &v.safety.strobe_cap()],
                        )
                    })
                    .unwrap_or_default(),
                FileKind::Rig => files
                    .rig(&name)
                    .map(|r| {
                        od_i18n::trf(
                            "{} FIXTURES · {} OUTPUTS",
                            &[&r.patch.fixtures.len(), &r.outputs.len()],
                        )
                    })
                    .unwrap_or_default(),
                FileKind::Show => files
                    .show(&name)
                    .map(|s| {
                        let cues: usize = s.settings.track_cues.values().map(Vec::len).sum();
                        od_i18n::trf(
                            "{} LOOKS · {} CUES · {} LIGHT CUES",
                            &[&s.settings.pads.len(), &s.settings.cues.len(), &cues],
                        )
                    })
                    .unwrap_or_default(),
            };
            LightFileRow {
                current: config.in_use.get(kind) == name,
                name: name.into(),
                detail: detail.into(),
            }
        })
        .collect()
}

/// A files action on the `i`-th file of the kind shown.
fn file_action(app: &App, i: i32, act: &str) {
    if app.lighting.borrow().locked() {
        return;
    }
    let (kind, name) = {
        let rt = app.lighting.borrow();
        let kind = rt.files_kind;
        let Some(name) = rt.files.list(kind).get(i.max(0) as usize).cloned() else {
            return;
        };
        (kind, name)
    };
    edit(app, |rt| {
        let files = rt.files.clone();
        let r = match act {
            "load" => files.load(kind, &name, &mut rt.config).map(|_| {
                rt.selected = None;
                od_i18n::trf("Loaded {}", &[&name])
            }),
            "save" => files
                .save(kind, &name, &mut rt.config)
                .map(|_| od_i18n::trf("Saved {}", &[&name])),
            "duplicate" => files
                .duplicate(kind, &name)
                .map(|n| od_i18n::trf("Copied to {}", &[&n])),
            "delete" => files
                .delete(kind, &name, &mut rt.config)
                .map(|_| od_i18n::trf("Deleted {}", &[&name])),
            _ => return,
        };
        rt.files_note = r.unwrap_or_else(|e| e.to_string());
        rt.files_seen = None;
        if act == "load" {
            rt.reapply_outputs();
        }
    });
}

/// Imports a venue file (venue, rig and show) and loads it.
pub fn import(app: &App, path: &std::path::Path) {
    if app.lighting.borrow().locked() {
        return;
    }
    edit(app, |rt| {
        let files = rt.files.clone();
        rt.files_note = match files.import(path, &mut rt.config) {
            Ok(name) => od_i18n::trf("Imported and loaded {}", &[&name]),
            Err(e) => e.to_string(),
        };
        rt.files_seen = None;
        rt.reapply_outputs();
    });
}

pub(crate) fn sync(app: &App) {
    let rt = app.lighting.borrow();
    let looks = rt
        .runner
        .snapshot
        .lock()
        .map(|s| s.looks.clone())
        .unwrap_or_default();
    rt.place_rows.sync_eq(place_rows(&rt.config.patch, &looks));
    let track = cue_track(app);
    rt.cue_rows.sync_eq(cue_rows(&rt, track.as_ref().map(|t| t.0)));
    // The files are read again only when the kind shown, the names in
    // use or the files themselves changed (not on every fader move).
    let seen = Some((rt.files_kind, rt.config.in_use.clone()));
    if rt.files_seen != seen {
        rt.file_rows.sync_eq(file_rows(&rt.files, rt.files_kind, &rt.config));
        drop(rt);
        app.lighting.borrow_mut().files_seen = seen;
        return sync(app);
    }
    let tune = tune_view(&rt);
    let can_add = track.as_ref().is_some_and(|t| t.1.is_some())
        && !targets(&rt.config.settings).is_empty();
    let title: SharedString = track.map(|t| t.2).unwrap_or_default().into();
    let selected = rt.selected.map_or(-1, |i| i as i32);
    let venue: SharedString = rt.config.in_use.venue.as_str().into();
    let kind = kind_id(rt.files_kind);
    let note: SharedString = rt.files_note.as_str().into();
    each(app, |g| {
        g.set_tune(tune.clone());
        g.set_selected_fixture(selected);
        g.set_cue_title(title.clone());
        g.set_cue_can_add(can_add);
        g.set_venue(venue.clone());
        g.set_files_kind(kind.into());
        g.set_files_note(note.clone());
    });
    drop(rt);
    sync_lane(app);
}

/// Live colours on the floor plan (SETUP is shown).
pub(crate) fn sync_live(app: &App) {
    let rt = app.lighting.borrow();
    let looks = rt
        .runner
        .snapshot
        .lock()
        .map(|s| s.looks.clone())
        .unwrap_or_default();
    rt.place_rows.sync(place_rows(&rt.config.patch, &looks), |a, b| a == b);
    let track = cue_track(app).map(|t| t.0);
    let rows = cue_rows(&rt, track);
    if rows.iter().map(|r| r.active).ne((0..rt.cue_rows.len()).map(|i| rt.cue_rows.get(i).is_some_and(|r| r.active))) {
        rt.cue_rows.sync_eq(rows);
    }
}

/// An edit of the floor plan (setup: not while the PIN locks it).
fn plan(app: &App, f: impl FnOnce(&mut LightingRuntime)) {
    if app.lighting.borrow().locked() {
        return;
    }
    edit(app, f);
}

/// An edit of the cue list of the cue track.
fn cue_edit(app: &App, f: impl FnOnce(&mut LightingRuntime, TrackId, Option<f64>)) {
    let Some((id, beat, _)) = cue_track(app) else { return };
    edit(app, |rt| {
        f(rt, id, beat);
        if rt.config.settings.track_cues.get(&id.0).is_some_and(Vec::is_empty) {
            rt.config.settings.track_cues.remove(&id.0);
        }
    });
}

pub(crate) fn wire(app: &Rc<App>, g: &crate::Lighting<'_>) {
    {
        let rt = app.lighting.borrow();
        g.set_places(rt.place_rows.handle());
        g.set_cue_rows(rt.cue_rows.handle());
        g.set_file_rows(rt.file_rows.handle());
    }
    // [32] Floor plan.
    g.on_select_fixture(handler!(app, |a, i: i32| {
        {
            let mut rt = a.lighting.borrow_mut();
            rt.selected = usize::try_from(i).ok().filter(|i| *i < rt.config.patch.fixtures.len());
        }
        refresh(&a);
    }));
    g.on_place_fixture(handler!(app, |a, i: i32, x: f32, y: f32| plan(&a, |rt| {
        let i = i.max(0) as usize;
        if let Some(p) = place_of(&mut rt.config.patch, i) {
            p.x = x.clamp(0.0, 1.0);
            p.y = y.clamp(0.0, 1.0);
        }
        rt.selected = Some(i);
    })));
    g.on_unplace_fixture(handler!(app, |a, i: i32| plan(&a, |rt| {
        if let Some(f) = rt.config.patch.fixtures.get_mut(i.max(0) as usize) {
            f.place = None;
        }
    })));
    g.on_rotate_fixture(handler!(app, |a, i: i32, d: i32| plan(&a, |rt| {
        if let Some(p) = place_of(&mut rt.config.patch, i.max(0) as usize) {
            p.rotate(15.0 * d as f32);
        }
    })));
    g.on_step_height(handler!(app, |a, i: i32, d: i32| plan(&a, |rt| {
        if let Some(p) = place_of(&mut rt.config.patch, i.max(0) as usize) {
            p.height = (p.height + 0.5 * d as f32).clamp(0.0, 20.0);
        }
    })));
    g.on_set_group(handler!(app, |a, i: i32, name: SharedString| plan(&a, |rt| {
        let name = name.trim();
        if let Some(f) = rt.config.patch.fixtures.get_mut(i.max(0) as usize)
            && !name.is_empty()
        {
            f.group = name.to_string();
        }
    })));
    g.on_step_tune(handler!(app, |a, i: i32, field: SharedString, d: i32| plan(&a, |rt| {
        step_tune(&mut rt.config.patch, i.max(0) as usize, &field, d);
    })));
    // [33] Cue list.
    g.on_cue_add(handler!(app, |a| {
        let stack = a.ui.global::<crate::Lighting>().get_active_cue().max(0) as usize;
        cue_edit(&a, |rt, id, beat| {
            let Some(beat) = beat else { return };
            let Some(plays) = default_target(rt, stack) else { return };
            let bar = (beat.max(0.0) / 4.0).floor() as u32 * 4;
            let list = cues_mut(rt, id);
            list.retain(|c| c.beat != bar);
            let mut c = TrackCue::new(bar, 0);
            set_plays(&mut c, plays);
            list.push(c);
            list.sort_by_key(|c| c.beat);
        });
    }));
    g.on_cue_move(handler!(app, |a, i: i32, d: i32| cue_edit(&a, |rt, id, _| {
        let list = cues_mut(rt, id);
        let Some(beat) = list.get(i.max(0) as usize).map(|c| c.beat) else { return };
        let to = (beat as i64 + 4 * d as i64).max(0) as u32;
        if list.iter().any(|c| c.beat == to) {
            return;
        }
        list[i as usize].beat = to;
        list.sort_by_key(|c| c.beat);
    })));
    g.on_cue_rename(handler!(app, |a, i: i32, name: SharedString| cue_edit(&a, |rt, id, _| {
        if let Some(c) = cues_mut(rt, id).get_mut(i.max(0) as usize) {
            c.name = name.trim().to_string();
        }
    })));
    g.on_cue_plays(handler!(app, |a, i: i32, d: i32| cue_edit(&a, |rt, id, _| {
        let all = targets(&rt.config.settings);
        if all.is_empty() {
            return;
        }
        if let Some(c) = cues_mut(rt, id).get_mut(i.max(0) as usize) {
            let n = all.len() as i32;
            let at = all.iter().position(|p| *p == plays_of(c)).map_or(-1, |x| x as i32);
            let next = if at < 0 { 0 } else { (at + d).rem_euclid(n) };
            set_plays(c, all[next as usize]);
        }
    })));
    g.on_cue_fade(handler!(app, |a, i: i32, d: i32| cue_edit(&a, |rt, id, _| {
        let stack = rt.config.settings.cues.clone();
        if let Some(c) = cues_mut(rt, id).get_mut(i.max(0) as usize) {
            let now = c.fade(&stack);
            let at = FADES.iter().position(|f| *f >= now).unwrap_or(FADES.len() - 1) as i32;
            let next = (at + d).clamp(0, FADES.len() as i32 - 1) as usize;
            c.fade_beats = Some(FADES[next]);
        }
    })));
    g.on_cue_remove(handler!(app, |a, i: i32| cue_edit(&a, |rt, id, _| {
        let list = cues_mut(rt, id);
        if (i.max(0) as usize) < list.len() {
            list.remove(i as usize);
        }
    })));
    // [35] Files.
    g.on_choose_files_kind(handler!(app, |a, k: SharedString| {
        {
            let mut rt = a.lighting.borrow_mut();
            rt.files_kind = kind_of(&k);
            rt.files_note.clear();
        }
        refresh(&a);
    }));
    g.on_file_load(handler!(app, |a, i: i32| file_action(&a, i, "load")));
    g.on_file_save(handler!(app, |a, i: i32| file_action(&a, i, "save")));
    g.on_file_duplicate(handler!(app, |a, i: i32| file_action(&a, i, "duplicate")));
    g.on_file_delete(handler!(app, |a, i: i32| file_action(&a, i, "delete")));
    g.on_file_save_as(handler!(app, |a, name: SharedString| {
        if a.lighting.borrow().locked() {
            return;
        }
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        edit(&a, |rt| {
            let (files, kind) = (rt.files.clone(), rt.files_kind);
            rt.files_note = if files.exists(kind, &name) {
                od_i18n::trf("{} exists already: choose it and press SAVE", &[&name])
            } else {
                match files.save(kind, &name, &mut rt.config) {
                    Ok(()) => od_i18n::trf("Saved {}", &[&name]),
                    Err(e) => e.to_string(),
                }
            };
            rt.files_seen = None;
        });
    }));
    g.on_file_rename(handler!(app, |a, i: i32, new: SharedString| {
        if a.lighting.borrow().locked() {
            return;
        }
        edit(&a, |rt| {
            let (files, kind) = (rt.files.clone(), rt.files_kind);
            let Some(old) = files.list(kind).get(i.max(0) as usize).cloned() else { return };
            rt.files_note = match files.rename(kind, &old, &new, &mut rt.config) {
                Ok(()) => od_i18n::trf("Renamed to {}", &[&new.trim()]),
                Err(e) => e.to_string(),
            };
            rt.files_seen = None;
        });
    }));
    g.on_file_import(handler!(app, |a| {
        let Some(path) = rfd::FileDialog::new()
            .set_title(od_i18n::tr("Lighting file (venue, rig and show)"))
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        import(&a, &path);
    }));
    g.on_file_export(handler!(app, |a| {
        let name = {
            let rt = a.lighting.borrow();
            let v = rt.config.in_use.venue.clone();
            if v.is_empty() { "Lighting".to_string() } else { v }
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(od_i18n::tr("Lighting file (venue, rig and show)"))
            .add_filter("JSON", &["json"])
            .set_file_name(format!("{name}.json"))
            .save_file()
        else {
            return;
        };
        export(&a, &path);
    }));
}

/// Writes the venue, rig and show in use to one file.
pub fn export(app: &App, path: &std::path::Path) {
    let note = match Files::export(&app.lighting.borrow().config, path) {
        Ok(()) => od_i18n::trf("Exported to {}", &[&path.display()]),
        Err(e) => e.to_string(),
    };
    {
        let mut rt = app.lighting.borrow_mut();
        rt.files_note = note;
        rt.files_seen = None;
    }
    refresh(app);
}

/// Prepare: the lane's callbacks.
pub(crate) fn wire_lane(app: &Rc<App>) {
    let lane = app.ui.global::<crate::LightCueLane>();
    lane.set_marks(app.lighting.borrow().lane_rows.handle());
    // Where a drag lands: the nearest bar, as a fraction.
    let weak = Rc::downgrade(app);
    lane.on_snap(move |f: f32| -> f32 {
        let Some(a) = weak.upgrade() else { return f };
        let st = a.state.borrow();
        let Some(id) = st.player.current else { return f };
        let (Ok(t), Ok(an)) = (st.src().track(id), st.src().analysis(id)) else { return f };
        let d = t.duration.max(0.001);
        bar_at(&an.beatgrid, f.clamp(0.0, 1.0) as f64 * d)
            .and_then(|b| beat_time(&an.beatgrid, b))
            .map_or(f, |s| (s / d).clamp(0.0, 1.0) as f32)
    });
    lane.on_move(handler!(app, |a, i: i32, f: f32| {
        let target = {
            let st = a.state.borrow();
            st.player.current.and_then(|id| {
                let t = st.src().track(id).ok()?;
                let grid = st.src().analysis(id).ok()?.beatgrid;
                Some((id, bar_at(&grid, f.clamp(0.0, 1.0) as f64 * t.duration)?))
            })
        };
        let Some((id, bar)) = target else { return };
        edit(&a, |rt| {
            let list = cues_mut(rt, id);
            let i = i.max(0) as usize;
            if i < list.len() && !list.iter().enumerate().any(|(j, c)| j != i && c.beat == bar) {
                list[i].beat = bar;
                list.sort_by_key(|c| c.beat);
            }
        });
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_labels_fades_and_bars() {
        let mut s = od_lighting::ShowSettings::default();
        s.cues.push(od_lighting::LightCue { name: "Drop".into(), ..Default::default() });
        let mut chase = od_lighting::PadLook::new(9, "Run");
        chase.steps = vec![0];
        s.pads.push(chase);
        s.pads.push(od_lighting::PadLook::new(0, "Red"));
        // The stack first, then the pads by number.
        assert_eq!(targets(&s), vec![Plays::Cue(0), Plays::Pad(0), Plays::Pad(9)]);
        assert_eq!(plays_label(&s, Plays::Cue(0)), "CUE 1 DROP");
        assert_eq!(plays_label(&s, Plays::Pad(0)), "PAD 1.1 RED");
        assert_eq!(plays_label(&s, Plays::Pad(9)), "CHASE 2.2 RUN");
        assert_eq!(plays_label(&s, Plays::Pad(3)), "PAD 1.4 EMPTY");
        assert_eq!(
            [0.0, 0.5, 1.0, 8.0].map(fade_label),
            ["AT ONCE", "½ BEAT", "1 BEAT", "8 BEATS"].map(String::from)
        );
        let grid = Beatgrid::constant(120.0, 0.0, 1, 32.0);
        assert_eq!(bar_at(&grid, 9.0), Some(20), "9 s is beat 18: the nearest bar starts at 20");
        assert_eq!(beat_time(&grid, 16), Some(8.0));
        assert!(beat_time(&grid, 200).unwrap() > 32.0, "past the end at the last tempo");
    }

    #[test]
    fn tuning_steps_stay_in_range() {
        let mut patch = od_lighting::Patch::default();
        let def = od_lighting::FixtureDef::builtin("moving-head").unwrap();
        patch.add_many(&def, 1, Some(0), Some(1), "Spot", "Movers").unwrap();
        for _ in 0..30 {
            step_tune(&mut patch, 0, "pan-from", 1);
            step_tune(&mut patch, 0, "red", 1);
        }
        let t = patch.fixtures[0].tune;
        assert_eq!(t.pan, [0.95, 1.0], "from stays below to");
        assert_eq!(t.color[0], 0.5, "calibration at most +50 %");
        step_tune(&mut patch, 0, "curve", -1);
        assert_eq!(patch.fixtures[0].tune.curve, DimmerCurve::Soft, "round");
        step_tune(&mut patch, 0, "invert-pan", 1);
        assert!(patch.fixtures[0].invert_pan);
        let v = place_rows(&patch, &[]);
        assert_eq!((v[0].role.as_str(), v[0].placed), ("mover", false));
    }
}
