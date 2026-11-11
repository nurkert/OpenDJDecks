//! Beatgrid editing (EDIT GRID in the Prepare player, the GRID panel of a
//! Perform deck).
//!
//! Principles: moving the grid and changing its tempo are separate; tempo
//! changes pivot on what you look at (the Prepare cursor, the Perform
//! playhead) and keep tempo markers; pins say "this beat sits exactly
//! here" (one pin moves the grid, two pins solve the tempo); edits snap to
//! the track's transients (kicks first) unless SNAP is off; the evidence
//! (transients, beat ticks coloured by error, the drift strip, FIT) is
//! always in view.
//!
//! A Prepare session edits a working copy: every change is an undo step of
//! the session (UNDO / REDO), decks playing the track follow at once, and
//! DONE writes the result to the library as one undo step; CANCEL drops it.
//! The edit operations themselves are pure ([`apply_op`]) and shared with
//! the Perform panel ([`crate::deck_grid`]).

use crate::app::{App, handler};
use crate::{decks, engine, format, player, render};
use od_analysis::transients::{self as tr, BeatError, Transient};
use od_core::{Beatgrid, TrackId};
use od_engine::DeckCommand;
use slint::{ComponentHandle, SharedString};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Snap window of SET 1, PIN and a double-click (seconds).
pub(crate) const SNAP_WIDE: f64 = 0.060;
/// Snap window when a drag of the grid ends (seconds).
pub(crate) const SNAP_DRAG: f64 = 0.015;
/// Metronome level of the click (linear).
pub(crate) const CLICK_LEVEL: f32 = 0.55;

// ── Pure edit operations ────────────────────────────────────────────────

/// One edit of a grid.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Op {
    /// Move the grid by seconds.
    Shift(f64),
    /// Move the grid by beats (at the pivot's tempo).
    Beats(f64),
    /// The beat at this time (snapped) becomes the 1 and sits exactly there.
    SetOne(f64),
    /// Tempo of the pivot's segment ± BPM.
    Step(f64),
    /// Tempo of the pivot's segment × factor.
    Scale(f64),
    /// Tempo of the pivot's segment set.
    Bpm(f64),
    /// A pin at this time (snapped).
    Pin(f64),
    ClearPins,
    /// Slip and tempo correction from the transients.
    AutoFix,
    /// A new tempo from the beat at the pivot on, measured from the
    /// transients after it.
    TempoChange,
    /// Moves the grid by the median offset of these times (taps, snapped
    /// to transients) to their nearest beats.
    Phase(Vec<f64>),
    Lock,
}

/// What an operation works with besides the grid.
pub(crate) struct Cx<'a> {
    /// The cursor (Prepare) or playhead (Perform), seconds.
    pub pivot: f64,
    pub duration: f64,
    pub transients: Option<&'a [Transient]>,
    pub snap: bool,
}

impl Cx<'_> {
    /// `t` snapped to the strongest transient within `window` (if on).
    pub(crate) fn snapped(&self, t: f64, window: f64) -> f64 {
        match self.transients.filter(|_| self.snap) {
            Some(list) => tr::snap(list, t, window).unwrap_or(t),
            None => t,
        }
    }
}

/// The result of an operation.
#[derive(Clone, Debug)]
pub(crate) struct Outcome {
    pub grid: Beatgrid,
    pub pins: Vec<f64>,
    /// Status line (English, translated on display).
    pub status: String,
    /// Where to look next (AUTO-FIX finding a tempo jump).
    pub look_at: Option<f64>,
}

/// Tempo of the beat nearest `t`.
pub(crate) fn bpm_at(g: &Beatgrid, t: f64) -> Option<f64> {
    g.nearest_beat(t).map(|i| g.beats[i].bpm)
}

/// One beat length at `t` in seconds (0 without a grid).
pub(crate) fn beat_len(g: &Beatgrid, t: f64) -> f64 {
    bpm_at(g, t).filter(|b| *b > 0.0).map_or(0.0, |b| 60.0 / b)
}

/// Run of beats with the tempo of beat `i` (its tempo segment).
fn segment(g: &Beatgrid, i: usize) -> std::ops::Range<usize> {
    let same = |a: usize| (g.beats[a].bpm - g.beats[i].bpm).abs() < 1e-9;
    let mut s = i;
    while s > 0 && same(s - 1) {
        s -= 1;
    }
    let mut e = i + 1;
    while e < g.beats.len() && same(e) {
        e += 1;
    }
    s..e
}

/// "127.98".
pub(crate) fn bpm_text(bpm: f64) -> String {
    format!("{bpm:.2}")
}

/// "+4 ms", "−12 ms", "0 ms".
pub(crate) fn ms_text(secs: f64) -> String {
    let ms = (secs * 1000.0).round() as i64;
    match ms {
        0 => "0 ms".into(),
        m if m > 0 => format!("+{m} ms"),
        m => format!("−{} ms", -m),
    }
}

/// Error class of a beat: 0 within 5 ms, 1 within 15 ms, 2 beyond.
pub(crate) fn tone_of(err: f64) -> i32 {
    let e = err.abs();
    if e <= 0.005 {
        0
    } else if e <= 0.015 {
        1
    } else {
        2
    }
}

/// A grid through `pins` (times): the beat counts between them come from
/// `g`, which must already sit near them; bars and the lock carry over.
fn solve_pins(g: &Beatgrid, pins: &[f64], duration: f64) -> Option<Beatgrid> {
    let numbered: Vec<(f64, i64)> = pins
        .iter()
        .filter_map(|&t| g.beat_position(t).map(|b| (t, b.round() as i64)))
        .collect();
    let lowest = numbered.iter().min_by_key(|p| p.1)?;
    let bar = g
        .beats
        .get(lowest.1.max(0) as usize)
        .map(|b| {
            // A beat before the grid's first: count back from it.
            if lowest.1 < 0 {
                ((b.bar_position as i64 - 1 + lowest.1).rem_euclid(4) + 1) as u8
            } else {
                b.bar_position
            }
        })
        .unwrap_or(1);
    let bpm = bpm_at(g, lowest.0).unwrap_or(120.0);
    let mut out = Beatgrid::through_pins(&numbered, bar, bpm, duration)?;
    out.locked = g.locked;
    Some(out)
}

/// Least-squares line `y = a + b·x` through points.
fn lsq(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    let n = points.len() as f64;
    if points.len() < 2 {
        return None;
    }
    let mx = points.iter().map(|p| p.0).sum::<f64>() / n;
    let my = points.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let b = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    Some((my - b * mx, b))
}

/// The tempo of the music from `start` on: a beat train from there at
/// `period` is fitted to the kicks over a growing stretch (3, 6, 12, …
/// beats), so it locks on even when the tempo differs a lot from the
/// grid's. Returns the fitted first beat and the BPM.
pub(crate) fn measure_tempo(list: &[Transient], start: f64, period: f64, stop: f64) -> Option<(f64, f64)> {
    let (mut s, mut p) = (start, period);
    let mut used = 0;
    for n in [3usize, 6, 12, 24, 48, 96, 128] {
        let pts: Vec<(f64, f64)> = (0..n)
            .map(|k| (k, s + k as f64 * p))
            .take_while(|(_, t)| *t < stop)
            .filter_map(|(k, t)| tr::snap(list, t, SNAP_WIDE).map(|x| (k as f64, x - t)))
            .collect();
        if pts.len() < 3 {
            break;
        }
        let (a, b) = lsq(&pts)?;
        s += a;
        p += b;
        used = pts.len();
        if !(p > 0.0) {
            return None;
        }
    }
    (used >= 8).then(|| (s, 60.0 / p))
}

/// Applies `op` to `grid` (with the session's `pins`). `Err` carries the
/// reason it could not be done (English).
pub(crate) fn apply_op(grid: &Beatgrid, pins: &[f64], op: &Op, cx: &Cx) -> Result<Outcome, String> {
    if grid.locked && *op != Op::Lock {
        return Err("The beatgrid is locked — unlock it to edit".into());
    }
    if grid.beats.is_empty() && !matches!(op, Op::Bpm(_) | Op::Lock) {
        return Err("This track has no beatgrid yet — analyse it first".into());
    }
    let mut g = grid.clone();
    let mut pins = pins.to_vec();
    let mut look_at = None;
    let end = cx.duration.max(grid.beats.last().map_or(0.0, |b| b.time) + 1.0);
    let pivot_bpm = bpm_at(&g, cx.pivot).unwrap_or(120.0);
    let status = match op {
        Op::Shift(s) => {
            g.shift(*s);
            od_i18n::trf("Grid moved {}", &[&ms_text(*s)])
        }
        Op::Beats(n) => {
            g.shift(n * beat_len(&g, cx.pivot));
            if *n < 0.0 {
                "Grid moved one beat earlier".into()
            } else {
                "Grid moved one beat later".into()
            }
        }
        Op::SetOne(at) => {
            let t = cx.snapped(*at, SNAP_WIDE);
            let Some(i) = g.align_beat(t) else {
                return Err("No beat to set".into());
            };
            g.set_downbeat(i);
            od_i18n::trf("Beat 1 at {}", &[&format::cue_time(t)])
        }
        Op::Step(d) => {
            let bpm = ((pivot_bpm + d) * 100.0).round() / 100.0;
            g.set_bpm_about(cx.pivot, bpm.clamp(30.0, 300.0), end);
            od_i18n::trf("Tempo {} BPM", &[&bpm_text(bpm)])
        }
        Op::Scale(f) => {
            let bpm = (pivot_bpm * f).clamp(30.0, 300.0);
            g.set_bpm_about(cx.pivot, bpm, end);
            od_i18n::trf("Tempo {} BPM", &[&bpm_text(bpm)])
        }
        Op::Bpm(b) => {
            if !(30.0..=300.0).contains(b) {
                return Err("Type a tempo between 30 and 300 BPM".into());
            }
            g.set_bpm_about(cx.pivot, *b, end);
            od_i18n::trf("Beatgrid set to {} BPM", &[&bpm_text(*b)])
        }
        Op::Pin(at) => {
            let t = cx.snapped(*at, SNAP_WIDE);
            // A pin near an existing one replaces it.
            let close = beat_len(&g, t) * 0.5;
            pins.retain(|p| (p - t).abs() > close);
            pins.push(t);
            pins.sort_by(f64::total_cmp);
            if pins.len() == 1 {
                g.align_beat(t);
                od_i18n::trf("Pin at {}: the beat sits there", &[&format::cue_time(t)])
            } else {
                let before = bpm_at(grid, pins[0]).unwrap_or(0.0);
                g = solve_pins(&g, &pins, end).ok_or("These pins do not fit one grid")?;
                let after = bpm_at(&g, pins[0]).unwrap_or(0.0);
                let beats = g
                    .beat_position(pins[pins.len() - 1])
                    .zip(g.beat_position(pins[0]))
                    .map_or(0.0, |(b, a)| b - a);
                od_i18n::trf(
                    "Tempo {} → {} ({} pins, {} bars)",
                    &[&bpm_text(before), &bpm_text(after), &pins.len(), &((beats / 4.0).round() as i64)],
                )
            }
        }
        Op::ClearPins => {
            pins.clear();
            "Pins cleared".into()
        }
        Op::AutoFix => {
            let list = cx.transients.ok_or("The transients are still being computed — try again in a moment")?;
            let errors = tr::grid_error(&g, list);
            // Only the tempo segment at the pivot.
            let range = g.nearest_beat(cx.pivot).map(|i| segment(&g, i)).unwrap_or(0..0);
            let seg: Vec<BeatError> = errors.iter().filter(|e| range.contains(&e.beat)).copied().collect();
            let line = tr::fit_line(&seg).ok_or("Too few kicks to measure the grid")?;
            let period = 60.0 / g.beats[line.reference].bpm;
            let bpm = (line.bpm(period) * 1000.0).round() / 1000.0;
            g.shift(line.slip);
            let at = g.beats[line.reference].time;
            if (bpm - g.beats[line.reference].bpm).abs() > 0.0005 {
                g.set_bpm_about(at, bpm, end);
            }
            look_at = line.split.and_then(|b| grid.beats.get(b)).map(|b| b.time);
            match look_at {
                Some(t) => od_i18n::trf(
                    "Fixed: {} BPM, moved {} — the beat jumps at {}: TEMPO CHANGE HERE there",
                    &[&bpm_text(bpm), &ms_text(line.slip), &format::time(t)],
                ),
                None => od_i18n::trf("Fixed: {} BPM, moved {}", &[&bpm_text(bpm), &ms_text(line.slip)]),
            }
        }
        Op::TempoChange => {
            let list = cx.transients.ok_or("The transients are still being computed — try again in a moment")?;
            let k = g.nearest_beat(cx.pivot).ok_or("No beat to set")?;
            if k == 0 {
                return Err("A tempo change needs beats before it — move the cursor into the track".into());
            }
            let range = segment(&g, k);
            let period = 60.0 / g.beats[k].bpm;
            let next = (range.end < g.beats.len()).then(|| g.beats[range.end].time);
            let start = cx.snapped(g.beats[k].time, SNAP_WIDE);
            let (start, bpm) = measure_tempo(list, start, period, next.unwrap_or(end))
                .ok_or("Too few kicks after the cursor to measure a tempo")?;
            let bpm = (bpm * 1000.0).round() / 1000.0;
            if (bpm - g.beats[k].bpm).abs() < 0.005 && (start - g.beats[k].time).abs() < 0.002 {
                return Err("The beats after the cursor already fit — no tempo change here".into());
            }
            let stop = next.unwrap_or(end);
            let bar = g.beats[k].bar_position;
            let mut beats: Vec<od_core::Beat> = g.beats[..k].to_vec();
            let p = 60.0 / bpm;
            let mut j = 0;
            loop {
                let t = start + j as f64 * p;
                if t > stop - if next.is_some() { p * 0.5 } else { 0.0 } {
                    break;
                }
                beats.push(od_core::Beat { time: t, bpm, bar_position: ((bar as usize - 1 + j) % 4 + 1) as u8 });
                j += 1;
            }
            let last_bar = beats.last().map_or(1, |b| b.bar_position);
            for (n, b) in g.beats[range.end..].iter().enumerate() {
                beats.push(od_core::Beat { bar_position: ((last_bar as usize + n) % 4 + 1) as u8, ..*b });
            }
            g.beats = beats;
            od_i18n::trf("Tempo change at {}: {} BPM", &[&format::time(start), &bpm_text(bpm)])
        }
        Op::Phase(times) => {
            let mut offsets: Vec<f64> = times
                .iter()
                .filter_map(|&t| {
                    let s = cx.transients.and_then(|l| tr::snap(l, t, SNAP_WIDE))?;
                    g.nearest_beat(s).map(|i| s - g.beats[i].time)
                })
                .collect();
            if offsets.len() < 3 {
                return Err(String::new());
            }
            offsets.sort_by(f64::total_cmp);
            let m = offsets[offsets.len() / 2];
            g.shift(m);
            od_i18n::trf("Grid moved {}", &[&ms_text(m)])
        }
        Op::Lock => {
            g.locked = !g.locked;
            if g.locked { "Beatgrid locked".into() } else { "Beatgrid unlocked".into() }
        }
    };
    Ok(Outcome { grid: g, pins, status, look_at })
}

// ── The Prepare session ─────────────────────────────────────────────────

/// An EDIT GRID session.
pub(crate) struct Session {
    pub track: TrackId,
    /// Content hash of the track's audio (its transients).
    hash: od_core::ContentHash,
    pub duration: f64,
    /// The grid when the session started (CANCEL).
    before: Beatgrid,
    pub grid: Beatgrid,
    pub pins: Vec<f64>,
    undo: Vec<(Beatgrid, Vec<f64>)>,
    redo: Vec<(Beatgrid, Vec<f64>)>,
    /// Live preview of a drag (seconds the grid is shown moved by).
    pub drag: f64,
    /// Track positions of the taps (phase from TAP).
    taps: Vec<f64>,
    /// Errors of `grid` for the transients of version `errors_for`.
    errors: Vec<BeatError>,
    errors_for: Option<(u64, usize)>,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    /// HEAR: metronome and snapping (kept between sessions).
    static CLICK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static SNAP: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    /// Bumped on every change of what the enlarged waveform shows.
    static REV: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub(crate) fn active() -> bool {
    SESSION.with(|s| s.borrow().is_some())
}

/// The track an EDIT GRID session runs on (its decks follow the session's
/// grid, not the stored one, until DONE or CANCEL).
pub(crate) fn session_track() -> Option<TrackId> {
    SESSION.with(|s| s.borrow().as_ref().map(|s| s.track))
}

/// Changes whenever the session's picture changes (the waveform redraws).
pub(crate) fn revision() -> u64 {
    REV.with(|r| r.get()) ^ crate::transients::version().wrapping_mul(0x9E37_79B9)
}

fn bump() {
    REV.with(|r| r.set(r.get().wrapping_add(1)));
}

/// What the enlarged waveform shows of the session on `track`: the grid
/// (with a drag in progress), the transients, per-beat errors and pins.
pub(crate) struct View {
    pub grid: Beatgrid,
    pub transients: Option<Arc<Vec<Transient>>>,
    pub errors: Vec<BeatError>,
    pub pins: Vec<f64>,
}

pub(crate) fn view(track: TrackId) -> Option<View> {
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut().filter(|s| s.track == track)?;
        let transients = crate::transients::get(&s.hash);
        refresh_errors(s, transients.as_deref().map(|v| v.as_slice()));
        let mut grid = s.grid.clone();
        let mut errors = s.errors.clone();
        if s.drag != 0.0 {
            grid.shift(s.drag);
            if let Some(list) = transients.as_deref() {
                errors = tr::grid_error(&grid, list);
            }
        }
        Some(View { grid, transients, errors, pins: s.pins.clone() })
    })
}

fn refresh_errors(s: &mut Session, list: Option<&[Transient]>) {
    let key = (crate::transients::version(), s.undo.len() + s.redo.len() * 7919);
    if s.errors_for == Some(key) && s.errors.len() == s.grid.beats.len() {
        return;
    }
    s.errors = list.map(|l| tr::grid_error(&s.grid, l)).unwrap_or_default();
    s.errors_for = Some(key);
}

/// Starts the session on the player's track. False (with a status) when
/// there is nothing to edit.
pub(crate) fn start(app: &App) -> bool {
    if app.guard_read_only() || app.guard_locked(crate::perform_lock::Blocked::Grid) {
        return false;
    }
    // The grid is edited at the track's own tempo and key.
    player::reset_preview_shift(app);
    let Some(id) = app.state.borrow().player.current else {
        app.status("Select a track to edit its beatgrid");
        return false;
    };
    let (grid, duration, path, hash) = {
        let st = app.state.borrow();
        let grid = st.lib.beatgrid(id).ok().flatten();
        let t = st.lib.track(id).ok();
        (
            grid,
            t.as_ref().map_or(0.0, |t| t.duration),
            t.as_ref().map(|t| st.resolve(&t.path)),
            t.map(|t| t.content_hash).unwrap_or(od_core::ContentHash(String::new())),
        )
    };
    let Some(grid) = grid.filter(|g| !g.beats.is_empty()) else {
        app.status("This track has no beatgrid yet — analyse it first");
        return false;
    };
    if grid.locked {
        app.status("The beatgrid is locked — unlock it (LOCK GRID) to edit");
        return false;
    }
    if let Some(path) = path {
        crate::transients::ensure(hash.clone(), path);
    }
    SESSION.with(|s| {
        *s.borrow_mut() = Some(Session {
            track: id,
            hash,
            duration,
            before: grid.clone(),
            grid,
            pins: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            drag: 0.0,
            taps: Vec::new(),
            errors: Vec::new(),
            errors_for: None,
        })
    });
    app.ui.global::<crate::GridEdit>().set_active(true);
    set_click(app, CLICK.with(|c| c.get()));
    app.status("Edit grid: drag the grid onto the kicks, double-click the 1 · PIN two kicks far apart to solve the tempo");
    refresh(app);
    true
}

/// DONE: the session's grid goes to the library as one undo step.
pub(crate) fn done(app: &App) {
    let Some(s) = SESSION.with(|s| s.borrow_mut().take()) else {
        return;
    };
    finish(app);
    if s.grid == s.before {
        app.status("Beatgrid unchanged");
        refresh(app);
        return;
    }
    let r = app.state.borrow_mut().lib.set_beatgrid(s.track, s.grid.clone());
    if app.report(r).is_some() {
        decks::follow_grid(app, s.track, &s.grid);
        app.after_edit("Beatgrid saved");
    }
    refresh(app);
}

/// CANCEL: back to the grid of the session start (nothing was stored).
pub(crate) fn cancel(app: &App) {
    let Some(s) = SESSION.with(|s| s.borrow_mut().take()) else {
        return;
    };
    finish(app);
    decks::follow_grid(app, s.track, &s.before);
    app.status("Beatgrid edit cancelled");
    refresh(app);
}

fn finish(app: &App) {
    app.ui.global::<crate::GridEdit>().set_active(false);
    engine::deck_cmd(
        app,
        decks::PREVIEW_DECK,
        DeckCommand::SetMetronome { on: false, level: CLICK_LEVEL, cue: false },
    );
    bump();
}

/// Perform: deck `deck`'s track opens in the Prepare player in EDIT GRID.
pub(crate) fn open_deck(app: &App, deck: usize) {
    let track = app.decks.borrow().loaded_id(deck);
    let Some(track) = track else {
        app.status("Load a track on the deck first");
        return;
    };
    {
        let mut st = app.state.borrow_mut();
        st.player.current = Some(track);
    }
    app.ui.invoke_set_mode("prepare".into());
    player::refresh_detail(app);
    start(app);
}

/// Runs an operation in the session (at the cursor).
pub(crate) fn apply(app: &App, op: Op) {
    if app.guard_locked(crate::perform_lock::Blocked::Grid) {
        return;
    }
    let cursor = app.state.borrow().player.cursor;
    let Some((hash, grid, pins, duration)) =
        SESSION.with(|s| s.borrow().as_ref().map(|s| (s.hash.clone(), s.grid.clone(), s.pins.clone(), s.duration)))
    else {
        return;
    };
    let transients = crate::transients::get(&hash);
    let cx = Cx {
        pivot: cursor,
        duration,
        transients: transients.as_deref().map(|v| v.as_slice()),
        snap: SNAP.with(|s| s.get()),
    };
    match apply_op(&grid, &pins, &op, &cx) {
        Ok(out) => {
            commit(app, out.grid, out.pins);
            app.status(out.status);
            if let Some(t) = out.look_at {
                player::set_cursor(app, t);
            }
        }
        Err(why) if !why.is_empty() => app.status(why),
        Err(_) => {}
    }
}

/// A new session state (one session undo step); decks follow.
fn commit(app: &App, grid: Beatgrid, pins: Vec<f64>) {
    let track = SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut()?;
        if s.grid == grid && s.pins == pins {
            return None;
        }
        let prev = (std::mem::replace(&mut s.grid, grid.clone()), std::mem::replace(&mut s.pins, pins));
        s.undo.push(prev);
        s.redo.clear();
        s.drag = 0.0;
        s.errors_for = None;
        Some(s.track)
    });
    if let Some(track) = track {
        decks::follow_grid(app, track, &grid);
        bump();
        refresh(app);
    }
}

fn step_history(app: &App, back: bool) {
    let grid = SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut()?;
        let (from, to) = if back { (&mut s.undo, &mut s.redo) } else { (&mut s.redo, &mut s.undo) };
        let (g, p) = from.pop()?;
        to.push((std::mem::replace(&mut s.grid, g), std::mem::replace(&mut s.pins, p)));
        s.errors_for = None;
        Some((s.track, s.grid.clone()))
    });
    match grid {
        Some((track, g)) => {
            decks::follow_grid(app, track, &g);
            bump();
            refresh(app);
            app.status(if back { "Undone" } else { "Redone" });
        }
        None => app.status(if back { "Nothing to undo" } else { "Nothing to redo" }),
    }
}

fn set_click(app: &App, on: bool) {
    CLICK.with(|c| c.set(on));
    engine::deck_cmd(
        app,
        decks::PREVIEW_DECK,
        DeckCommand::SetMetronome { on: on && active(), level: CLICK_LEVEL, cue: false },
    );
    app.ui.global::<crate::GridEdit>().set_click(on);
}

/// TAP in the session: the tempo (from the fourth tap) about the cursor,
/// and while the preview plays, the phase: taps that land near a
/// transient move the grid by their median offset.
pub(crate) fn tap(app: &App, bpm: Option<f64>) {
    if app.guard_locked(crate::perform_lock::Blocked::Grid) {
        return;
    }
    let (previewing, cursor) = {
        let st = app.state.borrow();
        (st.player.previewing, st.player.cursor)
    };
    let taps = SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut()?;
        if previewing {
            if s.taps.last().is_some_and(|l| cursor - l > 2.0 || cursor <= *l) {
                s.taps.clear();
            }
            s.taps.push(cursor);
        }
        Some(s.taps.clone())
    });
    let Some(taps) = taps else { return };
    if let Some(bpm) = bpm {
        apply(app, Op::Bpm(bpm));
        if taps.len() >= 4 {
            apply(app, Op::Phase(taps));
        }
        app.status(od_i18n::trf("Tapped tempo: {} BPM", &[&bpm_text(bpm)]));
    }
}

/// The waveform and readouts of the session.
pub(crate) fn refresh(app: &App) {
    let g = app.ui.global::<crate::GridEdit>();
    let cursor = app.state.borrow().player.cursor;
    let info = SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut()?;
        let transients = crate::transients::get(&s.hash);
        refresh_errors(s, transients.as_deref().map(|v| v.as_slice()));
        Some((
            s.track,
            s.grid.clone(),
            s.pins.clone(),
            s.errors.clone(),
            transients.is_some(),
            !s.undo.is_empty(),
            !s.redo.is_empty(),
            s.duration,
        ))
    });
    player::invalidate_detail(app);
    let Some((track, grid, pins, errors, has_tr, can_undo, can_redo, duration)) = info else {
        return;
    };
    let (title, source) = {
        let st = app.state.borrow();
        let t = st.lib.track(track).ok();
        let title = t
            .map(|t| {
                if t.tags.artist.is_empty() {
                    t.tags.title.clone()
                } else {
                    format!("{} – {}", t.tags.artist, t.tags.title)
                }
            })
            .unwrap_or_default();
        let source = st
            .lib
            .beatgrid_with_source(track)
            .ok()
            .flatten()
            .map(|(_, s)| od_i18n::tr(&s.as_str().to_uppercase()).to_string())
            .unwrap_or_default();
        (title, source)
    };
    g.set_title(title.into());
    g.set_source(source.into());
    g.set_bpm(bpm_at(&grid, cursor).map(bpm_text).unwrap_or_default().into());
    g.set_locked(grid.locked);
    g.set_can_undo(can_undo);
    g.set_can_redo(can_redo);
    g.set_snap(SNAP.with(|s| s.get()));
    g.set_click(CLICK.with(|c| c.get()));
    g.set_tempos((player::tempo_changes(&grid) + 1) as i32);
    g.set_pins(crate::models::model(
        pins.iter().map(|p| SharedString::from(format::cue_time(*p))).collect(),
    ));
    if has_tr && !errors.is_empty() {
        let fit = tr::fit(&errors);
        g.set_fit(format!("{} %", fit.round() as i32).into());
        g.set_fit_tone(if fit >= 90.0 { 0 } else if fit >= 70.0 { 1 } else { 2 });
        let near = grid.nearest_beat(cursor).and_then(|i| errors.get(i)).filter(|e| e.matched);
        g.set_at_cursor(near.map(|e| ms_text(e.err)).unwrap_or_else(|| "—".into()).into());
        g.set_at_cursor_tone(near.map_or(-1, |e| tone_of(e.err)));
        let worst = errors
            .iter()
            .filter(|e| e.matched)
            .max_by(|a, b| a.err.abs().total_cmp(&b.err.abs()))
            .filter(|e| e.err.abs() > 0.005);
        g.set_worst(
            worst
                .map(|e| format!("{} @ {}", ms_text(e.err), format::time(grid.beats[e.beat].time)))
                .unwrap_or_default()
                .into(),
        );
        let scale = app.ui.window().scale_factor();
        let w = ((app.ui.window().size().width as f32 / scale - 760.0).max(200.0) * scale) as u32;
        g.set_drift(render::drift_strip(&errors, &grid, duration, w, (14.0 * scale) as u32));
    } else {
        g.set_fit(if has_tr { "—" } else { "…" }.into());
        g.set_fit_tone(-1);
        g.set_at_cursor("—".into());
        g.set_at_cursor_tone(-1);
        g.set_worst(SharedString::new());
        g.set_drift(slint::Image::default());
    }
}

/// The cursor moved: readouts at the cursor follow.
pub(crate) fn cursor_moved(app: &App) {
    if active() {
        refresh(app);
    }
}

/// [ / ]: the previous / next spot where the grid is more than 15 ms off.
fn jump_problem(app: &App, forward: bool) {
    let cursor = app.state.borrow().player.cursor;
    let spots: Vec<f64> = SESSION.with(|s| {
        let s = s.borrow();
        s.as_ref()
            .map(|s| {
                tr::problems(&s.errors)
                    .into_iter()
                    .filter_map(|(b, _)| s.grid.beats.get(b).map(|x| x.time))
                    .collect()
            })
            .unwrap_or_default()
    });
    let next = if forward {
        spots.into_iter().find(|t| *t > cursor + 0.05)
    } else {
        spots.into_iter().rev().find(|t| *t < cursor - 0.05)
    };
    match next {
        Some(t) => player::set_cursor(app, t),
        None => app.status("No more spots where the grid is off"),
    }
}

/// The worst spot of the drift strip.
fn jump_worst(app: &App) {
    let t = SESSION.with(|s| {
        let s = s.borrow();
        let s = s.as_ref()?;
        let e = s.errors.iter().filter(|e| e.matched).max_by(|a, b| a.err.abs().total_cmp(&b.err.abs()))?;
        s.grid.beats.get(e.beat).map(|b| b.time)
    });
    if let Some(t) = t {
        player::set_cursor(app, t);
    }
}

/// Outside a session: an operation on the stored grid of the player's
/// track at the cursor, one undo step; decks follow.
pub(crate) fn apply_stored(app: &App, op: Op) {
    if app.guard_read_only() || app.guard_locked(crate::perform_lock::Blocked::Grid) {
        return;
    }
    let (id, cursor) = {
        let st = app.state.borrow();
        match st.player.current {
            Some(id) => (id, st.player.cursor),
            None => return,
        }
    };
    let (grid, duration) = {
        let st = app.state.borrow();
        (
            st.lib.beatgrid(id).ok().flatten().unwrap_or_default(),
            st.lib.track(id).map_or(0.0, |t| t.duration),
        )
    };
    let transients = crate::transients::of_track(app, id);
    let cx = Cx {
        pivot: cursor,
        duration,
        transients: transients.as_deref().map(|v| v.as_slice()),
        snap: SNAP.with(|s| s.get()),
    };
    match apply_op(&grid, &[], &op, &cx) {
        Ok(out) => {
            let r = app.state.borrow_mut().lib.set_beatgrid(id, out.grid.clone());
            if app.report(r).is_some() {
                decks::follow_grid(app, id, &out.grid);
                app.after_edit(&out.status);
            }
        }
        Err(why) if !why.is_empty() => app.status(why),
        Err(_) => {}
    }
}

/// Runs a grid action by name (keys, buttons): see the `match`. Outside a
/// session the moves, tempo keys, SET 1 and LOCK edit the stored grid.
pub(crate) fn act(app: &App, action: &str, arg: &str) {
    let num = arg.trim().replace(',', ".").parse::<f64>().unwrap_or(0.0);
    if !active() {
        let cursor = app.state.borrow().player.cursor;
        match action {
            "edit" => {
                start(app);
            }
            "shift" => apply_stored(app, Op::Shift(num / 1000.0)),
            "beat" => apply_stored(app, Op::Beats(num)),
            "set-one" => apply_stored(app, Op::SetOne(cursor)),
            "step" => apply_stored(app, Op::Step(num)),
            "scale" if num > 0.0 => apply_stored(app, Op::Scale(num)),
            "lock" => apply_stored(app, Op::Lock),
            _ => {}
        }
        return;
    }
    match action {
        "shift" => apply(app, Op::Shift(num / 1000.0)),
        "beat" => apply(app, Op::Beats(num)),
        "set-one" => {
            let c = app.state.borrow().player.cursor;
            apply(app, Op::SetOne(c));
        }
        "step" => apply(app, Op::Step(num)),
        "scale" if num > 0.0 => apply(app, Op::Scale(num)),
        "bpm" => match player::parse_bpm(arg) {
            Some(b) => apply(app, Op::Bpm(b)),
            None => app.status("Type a tempo between 30 and 300 BPM"),
        },
        "tap" => player::tap(app),
        "pin" => {
            let c = app.state.borrow().player.cursor;
            apply(app, Op::Pin(c));
        }
        "clear-pins" => apply(app, Op::ClearPins),
        "auto-fix" => apply(app, Op::AutoFix),
        "tempo-change" => apply(app, Op::TempoChange),
        "lock" => apply(app, Op::Lock),
        "click" => set_click(app, !CLICK.with(|c| c.get())),
        "snap" => {
            let on = !SNAP.with(|s| s.get());
            SNAP.with(|s| s.set(on));
            app.ui.global::<crate::GridEdit>().set_snap(on);
            app.status(if on { "Snap to transients on" } else { "Snap to transients off" });
        }
        "undo" => step_history(app, true),
        "redo" => step_history(app, false),
        "prev-problem" => jump_problem(app, false),
        "next-problem" => jump_problem(app, true),
        "worst" => jump_worst(app),
        // A click on the waveform: the cursor goes exactly there.
        "seek" => {
            let (c, span) = (app.state.borrow().player.cursor, player::detail_span(app));
            player::set_cursor(app, c + num * span);
        }
        "done" | "edit" => done(app),
        "cancel" => cancel(app),
        _ => {}
    }
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::GridEdit>();
    g.set_snap(SNAP.with(|s| s.get()));
    g.on_start(handler!(app, |a| {
        start(&a);
    }));
    g.on_done(handler!(app, |a| done(&a)));
    g.on_cancel(handler!(app, |a| cancel(&a)));
    g.on_open_deck(handler!(app, |a, d: i32| open_deck(&a, (d.max(1) - 1) as usize)));
    g.on_act(handler!(app, |a, action: SharedString, arg: SharedString| act(
        &a,
        action.as_str(),
        arg.as_str()
    )));
    // Drag of the grid: `frac` of the enlarged waveform's width (its span).
    g.on_preview(handler!(app, |a, frac: f32| {
        let secs = frac as f64 * player::detail_span(&a);
        SESSION.with(|s| {
            if let Some(s) = s.borrow_mut().as_mut() {
                s.drag = secs;
            }
        });
        bump();
        player::invalidate_detail(&a);
    }));
    // Release: the beat nearest where the drag started lands on the
    // transient within ±15 ms of where it was dropped (Alt: exactly there).
    g.on_commit(handler!(app, |a, frac: f32, at: f32, alt: bool| {
        let span = player::detail_span(&a);
        let secs = frac as f64 * span;
        let Some((hash, grid)) = SESSION.with(|s| {
            let mut s = s.borrow_mut();
            let s = s.as_mut()?;
            s.drag = 0.0;
            Some((s.hash.clone(), s.grid.clone()))
        }) else {
            return;
        };
        bump();
        if secs.abs() < 0.0005 {
            player::invalidate_detail(&a);
            return;
        }
        let cursor = a.state.borrow().player.cursor;
        let from = cursor + at as f64 * span - secs;
        let mut shift = secs;
        if !alt && SNAP.with(|s| s.get()) {
            if let (Some(i), Some(list)) = (grid.nearest_beat(from), crate::transients::get(&hash)) {
                let dropped = grid.beats[i].time + secs;
                if let Some(t) = tr::snap(&list, dropped, SNAP_DRAG) {
                    shift = t - grid.beats[i].time;
                }
            }
        }
        apply(&a, Op::Shift(shift));
    }));
    // A double-click on a beat: it becomes the 1 (aligned to its transient).
    g.on_set_one_at(handler!(app, |a, frac: f32| {
        let span = player::detail_span(&a);
        let cursor = a.state.borrow().player.cursor;
        apply(&a, Op::SetOne(cursor + frac as f64 * span));
    }));
    // Drift strip: a click jumps there.
    g.on_drift_seek(handler!(app, |a, frac: f32| {
        let d = SESSION.with(|s| s.borrow().as_ref().map(|s| s.duration)).unwrap_or(0.0);
        player::set_cursor(&a, frac.clamp(0.0, 1.0) as f64 * d);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx<'a>(pivot: f64, tr: Option<&'a [Transient]>) -> Cx<'a> {
        Cx { pivot, duration: 120.0, transients: tr, snap: true }
    }

    fn kicks(bpm: f64, first: f64, secs: f64) -> Vec<Transient> {
        let p = 60.0 / bpm;
        (0..)
            .map(|i| first + i as f64 * p)
            .take_while(|t| *t < secs)
            .map(|time| Transient { time, strength: 1.0, low: true })
            .collect()
    }

    #[test]
    fn the_nearest_beat_takes_the_click() {
        let g = Beatgrid::constant(120.0, 0.0, 1, 20.0);
        assert_eq!(g.nearest_beat(2.1), Some(4));
        assert_eq!(g.nearest_beat(2.3), Some(5));
        assert_eq!(g.nearest_beat(-1.0), Some(0));
    }

    #[test]
    fn tempo_steps_pivot_on_the_cursor() {
        let g = Beatgrid::constant(128.0, 0.1, 1, 120.0);
        let at = g.beats[200].time;
        let out = apply_op(&g, &[], &Op::Step(0.01), &cx(at + 0.01, None)).unwrap();
        let i = out.grid.nearest_beat(at).unwrap();
        assert!((out.grid.beats[i].time - at).abs() < 1e-12, "the beat at the cursor stays");
        assert!((out.grid.beats[i].bpm - 128.01).abs() < 1e-9);
        assert_eq!(out.grid.beats[i].bar_position, g.beats[200].bar_position);
        // Halving keeps the pivot too.
        let half = apply_op(&g, &[], &Op::Scale(0.5), &cx(at, None)).unwrap();
        assert!(half.grid.beats.iter().any(|b| (b.time - at).abs() < 1e-12));
    }

    #[test]
    fn set_one_snaps_to_the_kick() {
        let tr = kicks(120.0, 0.25, 60.0);
        let g = Beatgrid::constant(120.0, 0.28, 3, 60.0);
        // The cursor 40 ms after the kick at 10.25 s.
        let out = apply_op(&g, &[], &Op::SetOne(10.29), &cx(10.29, Some(&tr))).unwrap();
        let i = out.grid.nearest_beat(10.25).unwrap();
        assert!((out.grid.beats[i].time - 10.25).abs() < 1e-9);
        assert_eq!(out.grid.beats[i].bar_position, 1);
        // Snap off: exactly at the cursor.
        let mut c = cx(10.29, Some(&tr));
        c.snap = false;
        let out = apply_op(&g, &[], &Op::SetOne(10.29), &c).unwrap();
        assert!(out.grid.beats.iter().any(|b| (b.time - 10.29).abs() < 1e-9));
    }

    #[test]
    fn two_pins_solve_the_tempo() {
        // The music runs at 128.02, the grid at 127.98 from the first kick.
        let tr = kicks(128.02, 0.5, 240.0);
        let g = Beatgrid::constant(127.98, 0.5, 1, 240.0);
        let long = |pivot| Cx { pivot, duration: 240.0, transients: Some(&tr[..]), snap: true };
        let one = apply_op(&g, &[], &Op::Pin(0.51), &long(0.51)).unwrap();
        assert_eq!(one.pins.len(), 1);
        let late = tr[448].time;
        let two = apply_op(&one.grid, &one.pins, &Op::Pin(late + 0.01), &long(late)).unwrap();
        assert_eq!(two.pins.len(), 2);
        assert!((bpm_at(&two.grid, 10.0).unwrap() - 128.02).abs() < 1e-6, "{}", bpm_at(&two.grid, 10.0).unwrap());
        assert!(two.status.contains("127.98 → 128.02"), "{}", two.status);
        assert!(two.status.contains("112 bars"), "{}", two.status);
        for t in &two.pins {
            assert!(two.grid.beats.iter().any(|b| (b.time - t).abs() < 1e-9));
        }
    }

    #[test]
    fn auto_fix_repairs_slip_and_tempo() {
        let tr = kicks(126.0, 0.3, 120.0);
        let g = Beatgrid::constant(126.04, 0.318, 1, 120.0);
        let out = apply_op(&g, &[], &Op::AutoFix, &cx(60.0, Some(&tr))).unwrap();
        let errors = tr::grid_error(&out.grid, &tr);
        assert!(tr::fit(&errors) > 99.0, "{}", tr::fit(&errors));
        assert!(apply_op(&g, &[], &Op::AutoFix, &cx(60.0, None)).is_err(), "needs transients");
    }

    #[test]
    fn tempo_change_starts_a_new_segment() {
        // 124 BPM up to 30 s, then 128 BPM.
        let mut tr = kicks(124.0, 0.2, 30.0);
        let last = tr.last().unwrap().time + 60.0 / 124.0;
        tr.extend(kicks(128.0, last, 90.0));
        let g = Beatgrid::constant(124.0, 0.2, 1, 90.0);
        let out = apply_op(&g, &[], &Op::TempoChange, &cx(last + 0.01, Some(&tr))).unwrap();
        assert_eq!(player::tempo_changes(&out.grid), 1);
        assert!((bpm_at(&out.grid, 60.0).unwrap() - 128.0).abs() < 0.01);
        assert!((bpm_at(&out.grid, 10.0).unwrap() - 124.0).abs() < 1e-9, "before stays");
        assert!(tr::fit(&tr::grid_error(&out.grid, &tr)) > 97.0);
        // A step about the later cursor changes only the later segment.
        let step = apply_op(&out.grid, &[], &Op::Step(0.01), &cx(60.0, Some(&tr))).unwrap();
        assert!((bpm_at(&step.grid, 10.0).unwrap() - 124.0).abs() < 1e-9);
    }

    #[test]
    fn a_locked_grid_refuses_edits() {
        let mut g = Beatgrid::constant(120.0, 0.0, 1, 10.0);
        g.locked = true;
        assert!(apply_op(&g, &[], &Op::Shift(0.01), &cx(1.0, None)).is_err());
        assert!(!apply_op(&g, &[], &Op::Lock, &cx(1.0, None)).unwrap().grid.locked);
    }

    #[test]
    fn readouts() {
        assert_eq!(ms_text(0.0042), "+4 ms");
        assert_eq!(ms_text(-0.012), "−12 ms");
        assert_eq!(tone_of(0.004), 0);
        assert_eq!(tone_of(-0.012), 1);
        assert_eq!(tone_of(0.03), 2);
    }
}
