//! CREATE (roadmap [70]): the fourth work mode, for what DJs make before a
//! set: a melody from one note (MELODY), a small drum machine (DRUMS) and
//! which notes fit (THEORY). The concept is in roadmap 0.10.0.
//!
//! [70] is the shell: the three views with what comes where, the RESULT
//! bar, and a slim deck strip so the decks stay in view. The decks keep
//! playing: a mode switch only changes what is shown, and this module only
//! reads the engine's deck state for the strip.
//!
//! [71] MELODY's SOURCE picks one note: the track on deck 1 or 2 or the one
//! in the player, a zoomed waveform with IN and OUT around the note, its
//! pitch from [`od_analysis::pitch`] (DETECTED or UNSURE) and a correction
//! by steps or a typed name (SET BY YOU). HEAR plays IN to OUT with a short
//! fade at OUT on the sampler's audition voice, to the headphones while a
//! deck plays, so the decks are never touched. The picked note is kept in
//! the library meta (`create.note`).
//!
//! [72] NOTES plays the note on 8 pads at other pitches: the chromatic notes
//! from the source note up (source, +1 … +7 semitones, until [73] maps them
//! to a scale), an octave switch, and a FAR warning on shifts over 7
//! semitones. [`od_engine::noteshift`] renders each target (resampled or
//! stretched, with a clean start and tail) on a worker thread into a cache
//! per target, so a pad plays at once on the audition voice.
//!
//! [73] NOTES plays in a key: a root and a scale ([`od_core::ScaleKey`]).
//! It follows the other deck's key until NEXT TRACK takes the next track's
//! (from the queue, else the set plan) or the root or scale is set by hand.
//! With SNAP TO SCALE on (the default) the pads hold the scale's notes from
//! the root at or below the source note (see [`pad_targets`]); off brings
//! back the chromatic pads of [72]. The keyboard strip shows the root, the
//! scale and the notes out of it over the pads' two octaves, and plays a
//! key on the audition voice. The choice is kept in `create.key`.
//!
//! [74] ARRANGE puts the notes in order on a strip of 32 steps on the
//! master grid ([`crate::arrange`]). REC records the pads and the keys:
//! on the nearest step of the grid while PLAY or the master deck plays,
//! else step by step from a cursor. Typed note names replace the strip from
//! step 1. A step is selected with a tap and moved, transposed or deleted;
//! CLEAR empties the strip and UNDO brings back what the last CLEAR or
//! typing replaced. PLAY sends the notes from the render cache to the
//! audition voice, each on its beat of the master clock (from the next bar
//! while a deck plays), so the decks are never touched. The strip is kept
//! in `create.melody`.

use crate::app::App;
use crate::arrange::{self, Grid, Melody, STEPS};
use crate::engine;
use crossbeam_channel::{Receiver, Sender, unbounded};
use od_analysis::pitch::{self, PitchOutcome};
use od_core::{KeyNotation, MusicalKey, NotePitch, Scale, ScaleKey, TrackId};
use od_engine::noteshift::{self, ShiftMethod};
use od_engine::{Command, DeckState, EngineState, SampleData, SamplerCommand, TrackData};
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The views of CREATE, in the order of their tabs.
pub(crate) const VIEWS: [&str; 3] = ["melody", "drums", "theory"];

/// One deck of the strip from its engine state, channel level and key.
pub(crate) fn strip_row(i: usize, s: &DeckState, level: f32, key: String) -> crate::CreateDeck {
    let loaded = s.track.is_some();
    // Where in the bar: the beat of the bar (1..=4) and how far into it.
    let bar = if loaded && s.beat_in_bar > 0 {
        ((f64::from(s.beat_in_bar) - 1.0 + s.beat.rem_euclid(1.0)) / 4.0) as f32
    } else {
        0.0
    };
    crate::CreateDeck {
        deck: i as i32 + 1,
        loaded,
        playing: loaded && s.playing,
        bpm: if loaded { crate::format::bpm(Some(s.bpm), 1) } else { String::new() }.into(),
        key: key.into(),
        progress: if loaded && s.duration > 0.0 { (s.position / s.duration).clamp(0.0, 1.0) as f32 } else { 0.0 },
        bar,
        level: if loaded { level.clamp(0.0, 1.0) } else { 0.0 },
    }
}

/// Frame tick while CREATE is shown: the deck strip (and the decks' keys
/// for THEORY). Before audio runs the decks show empty.
pub(crate) fn frame(app: &App, state: Option<&EngineState>) {
    let count = app.decks.borrow().deck_count;
    let rows: Vec<crate::CreateDeck> = (0..count)
        .map(|i| match state.filter(|s| i < s.decks.len()) {
            Some(s) => {
                let d = &s.decks[i];
                strip_row(i, d, s.channels[i].meter.peak, crate::decks::key_text(app, i, d.track))
            }
            None => strip_row(i, &DeckState::default(), 0.0, String::new()),
        })
        .collect();
    let g = app.ui.global::<crate::Create>();
    // [71] PLAYER picks from the track in Prepare's player.
    let ready = app.state.borrow().player.current.is_some();
    if g.get_player_ready() != ready {
        g.set_player_ready(ready);
    }
    // Same row count: update in place (no relayout every frame).
    let model = g.get_decks();
    if model.row_count() == rows.len() {
        for (i, row) in rows.into_iter().enumerate() {
            if model.row_data(i).as_ref() != Some(&row) {
                model.set_row_data(i, row);
            }
        }
    } else {
        g.set_decks(ModelRc::new(VecModel::from(rows)));
    }
    // [73] The key follows the other deck; NEXT TRACK says what it would take.
    follow_deck(app, state);
    // [74] The tempo for RESULT, and where REC puts a note now.
    let tempo = crate::format::bpm(Some(state.map_or(120.0, |s| s.master_bpm)), 1);
    if g.get_tempo().as_str() != tempo {
        g.set_tempo(tempo.into());
    }
    let grid = state.is_some_and(|s| master_beat(s).is_some()) || app.create.borrow().play.is_some();
    if app.create.borrow().rec_grid != grid {
        app.create.borrow_mut().rec_grid = grid;
        show_arrange(app);
    }
    let due = app.create.borrow().next_at.is_none_or(|t| t.elapsed() > NEXT_EVERY);
    if due {
        refresh_next(app);
    }
}

/// Sets the view (MELODY, DRUMS or THEORY); unknown names are ignored.
pub(crate) fn set_view(app: &App, view: &str) {
    if VIEWS.contains(&view) {
        app.ui.global::<crate::Create>().set_view(view.into());
    }
}

// ── [71] The picked note ────────────────────────────────────────────────

/// Library meta key of the picked note (JSON).
pub(crate) const META_NOTE: &str = "create.note";
/// Seconds across the zoomed waveform.
pub(crate) const SPAN: f64 = 4.0;
/// A new pick: IN at the playhead, OUT this much later.
const PICK_LEN: f64 = 0.5;
/// The shortest note IN and OUT may hold.
const MIN_LEN: f64 = 0.03;
/// A handle this close to the waveform's edge (share of [`SPAN`]) moves the
/// view to the note when it is let go.
const EDGE: f64 = 0.2;
/// One arrow key on a handle, seconds.
const NUDGE: f64 = 0.01;
/// The fade-in at IN against a click, seconds.
const FADE_IN: f64 = 0.003;
/// The pitch a correction starts from when nothing was detected (A3).
const NO_PITCH_START: i32 = 57;

/// The picked note as it is kept: its source, IN and OUT, the view, the
/// detected pitch and the correction.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PickedNote {
    /// "deck1", "deck2" or "player" (the track in Prepare's player).
    pub source: String,
    pub track: Option<TrackId>,
    /// "Title · Artist" at the time of the pick.
    pub title: String,
    pub path: PathBuf,
    pub duration: f64,
    /// IN and OUT, seconds.
    pub start: f64,
    pub end: f64,
    /// The time at the zoomed waveform's left edge.
    pub view: f64,
    /// What the detector found, and how sure it was.
    pub detected: Option<NotePitch>,
    pub confidence: f32,
    /// Why there is no pitch: "short", "silent", "none" ("" = there is one).
    pub why: String,
    /// The pitch set by the user (wins over the detected one).
    pub set: Option<NotePitch>,
}

impl PickedNote {
    /// The fade at OUT: a quarter of the note, 5 to 60 ms.
    pub fn fade(&self) -> f64 {
        ((self.end - self.start) * 0.25).clamp(0.005, 0.06)
    }

    /// The pitch in use: the user's, else the detected one.
    pub fn pitch(&self) -> Option<NotePitch> {
        self.set.or(self.detected)
    }

    /// "set", "detected" or "unsure".
    pub fn badge(&self) -> &'static str {
        if self.set.is_some() {
            "set"
        } else if self.detected.is_some() && self.confidence >= pitch::SURE {
            "detected"
        } else {
            "unsure"
        }
    }

    /// Moves IN (`in_handle`) or OUT to `t` seconds, inside the view and
    /// the track, at least [`MIN_LEN`] from the other.
    pub fn move_handle(&mut self, in_handle: bool, t: f64) {
        let lo = self.view.max(0.0);
        let hi = (self.view + SPAN).min(self.duration.max(MIN_LEN));
        if in_handle {
            self.start = t.clamp(lo, (self.end - MIN_LEN).max(lo));
        } else {
            self.end = t.clamp((self.start + MIN_LEN).min(hi), hi);
        }
    }

    /// The view centred on the note when a handle came near its edge.
    pub fn recentre(&mut self) {
        let near = |t: f64| {
            let f = (t - self.view) / SPAN;
            !(EDGE..=1.0 - EDGE).contains(&f)
        };
        if near(self.start) || near(self.end) {
            self.view = (self.start + self.end) / 2.0 - SPAN / 2.0;
        }
    }

    /// Takes a measurement of IN to OUT.
    pub fn measured(&mut self, e: &pitch::PitchEstimate) {
        self.detected = e.hz.and_then(NotePitch::from_hz);
        self.confidence = e.confidence;
        self.why = match (e.outcome, self.detected) {
            (PitchOutcome::TooShort, _) => "short",
            (PitchOutcome::Silent, _) => "silent",
            (_, None) => "none",
            _ => "",
        }
        .into();
    }
}

/// The audio a note is measured and played from.
#[derive(Clone)]
enum Audio {
    /// What a deck plays (interleaved stereo at the engine rate).
    Deck(Arc<TrackData>),
    /// The file decoded to mono at its own rate.
    File { samples: Arc<[f32]>, rate: u32 },
}

impl Audio {
    fn rate(&self) -> u32 {
        match self {
            Audio::Deck(t) => t.sample_rate,
            Audio::File { rate, .. } => *rate,
        }
    }

    fn frames(&self, start: f64, end: f64) -> std::ops::Range<usize> {
        let (n, rate) = match self {
            Audio::Deck(t) => (t.samples.len() / 2, t.sample_rate),
            Audio::File { samples, rate } => (samples.len(), *rate),
        };
        let at = |t: f64| ((t.max(0.0) * f64::from(rate)) as usize).min(n);
        at(start)..at(end)
    }

    /// IN to OUT in mono.
    fn mono(&self, start: f64, end: f64) -> Vec<f32> {
        let r = self.frames(start, end);
        match self {
            Audio::Deck(t) => t.samples[r.start * 2..r.end * 2].chunks_exact(2).map(|f| (f[0] + f[1]) * 0.5).collect(),
            Audio::File { samples, .. } => samples[r].to_vec(),
        }
    }

    /// IN to OUT in interleaved stereo.
    fn stereo(&self, start: f64, end: f64) -> Vec<f32> {
        let r = self.frames(start, end);
        match self {
            Audio::Deck(t) => t.samples[r.start * 2..r.end * 2].to_vec(),
            Audio::File { samples, .. } => samples[r].iter().flat_map(|&v| [v, v]).collect(),
        }
    }
}

/// Applies the fades of a note to interleaved stereo at `rate`: a click
/// guard at IN and `fade` seconds down to silence at OUT (raised cosine).
pub(crate) fn apply_fades(samples: &mut [f32], rate: u32, fade: f64) {
    let frames = samples.len() / 2;
    let fin = ((FADE_IN * f64::from(rate)) as usize).min(frames / 2).max(1);
    let fout = ((fade * f64::from(rate)) as usize).min(frames).max(1);
    for (i, f) in samples.chunks_exact_mut(2).enumerate() {
        let mut g = 1.0;
        if i < fin {
            g *= i as f32 / fin as f32;
        }
        let left = frames - i;
        if left <= fout {
            let x = (left - 1) as f32 / fout as f32;
            g *= 0.5 - 0.5 * (std::f32::consts::PI * x).cos();
        }
        f[0] *= g;
        f[1] *= g;
    }
}

/// CREATE's running state beside what is stored.
pub(crate) struct Runtime {
    note: Option<PickedNote>,
    /// The audio of the note's track, once at hand.
    audio: Option<(TrackId, Audio)>,
    /// A track being decoded, and its result.
    loading: Option<TrackId>,
    tx: Sender<(TrackId, Option<Audio>)>,
    rx: Receiver<(TrackId, Option<Audio>)>,
    /// Measure (and play) once the audio is in.
    measure_due: bool,
    hear_due: bool,
    /// HEAR (or a pad) plays, since when.
    hearing: Option<Instant>,
    /// The zoomed waveform as drawn, for the view it shows.
    wave: slint::Image,
    /// [72] The pads: their octave, the rendered notes, the worker.
    octave: i32,
    cache: NoteCache,
    rendered_tx: Sender<Rendered>,
    rendered_rx: Receiver<Rendered>,
    /// The current render's number: a worker of an older one stops.
    generation: Arc<AtomicU64>,
    /// Render the pads once the audio is in.
    pads_due: bool,
    /// A pad or key pressed before its note was rendered: play it when it
    /// is in.
    due: Option<Due>,
    /// The pad that plays (on the audition voice).
    pad_playing: Option<usize>,
    /// [73] The key of the strip that plays (MIDI).
    key_playing: Option<i32>,
    /// The key and scale, where they come from, SNAP TO SCALE.
    choice: KeyChoice,
    /// What NEXT TRACK would take, looked up now and then.
    next: NextTrack,
    next_at: Option<Instant>,
    /// [74] The arrange strip and its selected step.
    melody: Melody,
    selected: Option<usize>,
    /// REC armed.
    rec: Option<Rec>,
    /// Where REC puts a note now: on the grid (PLAY or the master deck
    /// plays), else at the cursor.
    rec_grid: bool,
    /// PLAY runs.
    play: Option<Play>,
    /// The strip before the last CLEAR or typed notes (UNDO).
    undo: Option<Melody>,
    /// What the last typing could not read, and whether it was too long.
    typed_bad: Vec<String>,
    typed_long: bool,
    /// The notes PLAY sent, as (step, MIDI) (tests).
    #[cfg(test)]
    sent: Vec<(usize, i32)>,
}

/// REC armed: the beat the grid counts from (once a note came in on it),
/// and the step the next note fills when nothing plays.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rec {
    origin: Option<f64>,
    cursor: usize,
}

/// PLAY: the beat of the master clock step 1 starts on, the next step to
/// send (counted on from 0 across loops), the beat of the note sent and
/// not started yet, the clock last seen and the step that sounds.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Play {
    start: f64,
    next: i64,
    waiting: Option<f64>,
    last: f64,
    step: Option<usize>,
}

/// A note asked for before it was rendered: a pad, or a key of the strip.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Due {
    target: i32,
    pad: Option<usize>,
}

impl Default for Runtime {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        let (rendered_tx, rendered_rx) = unbounded();
        Self {
            note: None,
            audio: None,
            loading: None,
            tx,
            rx,
            measure_due: false,
            hear_due: false,
            hearing: None,
            wave: slint::Image::default(),
            octave: 0,
            cache: NoteCache::default(),
            rendered_tx,
            rendered_rx,
            generation: Arc::new(AtomicU64::new(0)),
            pads_due: false,
            due: None,
            pad_playing: None,
            key_playing: None,
            choice: KeyChoice::default(),
            next: NextTrack::default(),
            next_at: None,
            melody: Melody::default(),
            selected: None,
            rec: None,
            rec_grid: false,
            play: None,
            undo: None,
            typed_bad: Vec::new(),
            typed_long: false,
            #[cfg(test)]
            sent: Vec::new(),
        }
    }
}

/// The picked note (tests and the demo).
pub(crate) fn note(app: &App) -> Option<PickedNote> {
    app.create.borrow().note.clone()
}

/// Wires SOURCE and brings back the stored note.
pub(crate) fn wire(app: &std::rc::Rc<App>) {
    let g = app.ui.global::<crate::Create>();
    let weak = std::rc::Rc::downgrade(app);
    let with = move |f: &dyn Fn(&App)| {
        if let Some(a) = weak.upgrade() {
            f(&a);
        }
    };
    let w = with.clone();
    g.on_pick(move |s| w(&|a| pick(a, &s)));
    let w = with.clone();
    g.on_drag_handle(move |h, at| w(&|a| drag_handle(a, h == "in", at)));
    let w = with.clone();
    g.on_drag_done(move || w(&drag_done));
    let w = with.clone();
    g.on_nudge(move |h, d| w(&|a| nudge(a, h == "in", d)));
    let w = with.clone();
    g.on_pan(move |by| w(&|a| pan(a, by)));
    let w = with.clone();
    g.on_hear(move || w(&hear));
    let w = with.clone();
    g.on_step_note(move |d| w(&|a| correct(a, |p| p.transpose(d))));
    let w = with.clone();
    g.on_step_cents(move |d| w(&|a| correct(a, |p| p.detune(d))));
    let w = with.clone();
    g.on_play_pad(move |i| w(&|a| play_pad(a, i.max(0) as usize)));
    let w = with.clone();
    g.on_step_octave(move |d| w(&|a| step_octave(a, d)));
    let w = with.clone();
    g.on_step_root(move |d| w(&|a| step_root(a, d)));
    let w = with.clone();
    g.on_choose_scale(move |id| w(&|a| choose_scale(a, &id)));
    let w = with.clone();
    g.on_key_from_deck(move || w(&key_from_deck));
    let w = with.clone();
    g.on_key_from_next(move || w(&key_from_next));
    let w = with.clone();
    g.on_set_snap(move |on| w(&|a| set_snap(a, on)));
    let w = with.clone();
    g.on_set_hints(move |on| w(&|a| set_hints(a, on)));
    let w = with.clone();
    g.on_play_key(move |i| w(&|a| play_key(a, i)));
    let w = with.clone();
    g.on_set_grid(move |id| w(&|a| set_grid(a, &id)));
    let w = with.clone();
    g.on_toggle_rec(move || w(&toggle_rec));
    let w = with.clone();
    g.on_toggle_play(move || w(&toggle_play));
    let w = with.clone();
    g.on_set_loop(move |on| w(&|a| set_loop(a, on)));
    let w = with.clone();
    g.on_clear_steps(move || w(&clear_steps));
    let w = with.clone();
    g.on_undo_steps(move || w(&undo_steps));
    let w = with.clone();
    g.on_select_step(move |i| w(&|a| select_step(a, i)));
    let w = with.clone();
    g.on_move_step(move |d| w(&|a| move_step(a, d)));
    let w = with.clone();
    g.on_delete_step(move || w(&delete_step));
    let w = with.clone();
    g.on_shift_step(move |d, semitone| w(&|a| shift_step(a, d, semitone)));
    let w = with.clone();
    g.on_type_steps(move |t| w(&|a| type_steps(a, &t)));
    let w = with;
    g.on_type_note(move |t| {
        let ok = std::cell::Cell::new(false);
        w(&|a| ok.set(type_note(a, &t)));
        ok.get()
    });
    restore(app);
}

/// Reads the stored note back.
fn restore(app: &App) {
    let stored = app.state.borrow().lib.meta(META_NOTE).ok().flatten();
    let note = stored.and_then(|j| serde_json::from_str::<PickedNote>(&j).ok()).filter(|n| n.track.is_some());
    let choice = app.state.borrow().lib.meta(META_KEY).ok().flatten().and_then(|j| serde_json::from_str::<KeyChoice>(&j).ok());
    let melody = app.state.borrow().lib.meta(META_MELODY).ok().flatten().and_then(|j| serde_json::from_str::<Melody>(&j).ok());
    {
        let mut rt = app.create.borrow_mut();
        rt.note = note;
        rt.choice = choice.unwrap_or_default();
        rt.melody = melody.unwrap_or_default();
        rt.melody.normalize();
    }
    render_wave(app);
    show(app);
    show_arrange(app);
}

fn save(app: &App) {
    if app.state.borrow().read_only() {
        return;
    }
    let json = app.create.borrow().note.as_ref().map(|n| serde_json::to_string(n).unwrap_or_default()).unwrap_or_default();
    if let Err(e) = app.state.borrow_mut().lib.set_meta(META_NOTE, &json) {
        tracing::warn!(error = %e, "picked note not saved");
    }
}

/// Picks a new note from `source` at where it stands: the deck's playhead,
/// or the player's cursor in its track. A new pick drops a correction.
pub(crate) fn pick(app: &App, source: &str) {
    let found = match source {
        "deck1" | "deck2" => {
            let deck = usize::from(source == "deck2");
            let id = app.decks.borrow().loaded_id(deck);
            let at = engine::snapshot(app).map_or(0.0, |s| s.decks[deck].position);
            id.map(|id| (id, at, app.decks.borrow().engine_track(deck).filter(|t| t.id == id)))
        }
        "player" => {
            let st = app.state.borrow();
            st.player.current.map(|id| (id, st.player.cursor, None))
        }
        _ => return,
    };
    let Some((id, at, deck_audio)) = found else {
        app.status(if source == "player" {
            od_i18n::tr("No track in the player: select one in the library").to_string()
        } else {
            od_i18n::trf("No track on deck {}", &[&source.trim_start_matches("deck")])
        });
        return;
    };
    let (title, path, duration) = {
        let st = app.state.borrow();
        let Ok(t) = st.src().track(id) else { return };
        let title = if t.tags.artist.is_empty() {
            t.tags.title.clone()
        } else {
            format!("{} · {}", t.tags.title, t.tags.artist)
        };
        (title, st.resolve(&t.path), t.duration)
    };
    let start = at.clamp(0.0, (duration - MIN_LEN).max(0.0));
    let end = (start + PICK_LEN).min(duration.max(start + MIN_LEN));
    let note = PickedNote {
        source: source.into(),
        track: Some(id),
        title,
        path,
        duration,
        start,
        end,
        view: (start + end) / 2.0 - SPAN / 2.0,
        ..Default::default()
    };
    stop_hearing(app);
    {
        let mut rt = app.create.borrow_mut();
        rt.note = Some(note);
        if let Some(a) = deck_audio {
            rt.audio = Some((id, Audio::Deck(a)));
        }
    }
    render_wave(app);
    measure(app);
    // [73] The other deck is another one now.
    follow_deck(app, engine::snapshot(app).as_ref());
}

/// A handle dragged to `at` (0..1 across the waveform).
fn drag_handle(app: &App, in_handle: bool, at: f32) {
    {
        let mut rt = app.create.borrow_mut();
        let Some(n) = rt.note.as_mut() else { return };
        let t = n.view + f64::from(at) * SPAN;
        n.move_handle(in_handle, t);
    }
    show(app);
}

/// A handle let go: the view follows the note, which is measured again.
fn drag_done(app: &App) {
    let moved = {
        let mut rt = app.create.borrow_mut();
        let Some(n) = rt.note.as_mut() else { return };
        let before = n.view;
        n.recentre();
        n.view != before
    };
    if moved {
        render_wave(app);
    }
    measure(app);
}

/// An arrow key on a handle: 10 ms earlier or later.
fn nudge(app: &App, in_handle: bool, dir: i32) {
    {
        let mut rt = app.create.borrow_mut();
        let Some(n) = rt.note.as_mut() else { return };
        let t = if in_handle { n.start } else { n.end } + f64::from(dir.signum()) * NUDGE;
        n.move_handle(in_handle, t);
    }
    drag_done(app);
}

/// The waveform dragged by `by` of its width: the view moves, the note stays.
fn pan(app: &App, by: f32) {
    {
        let mut rt = app.create.borrow_mut();
        let Some(n) = rt.note.as_mut() else { return };
        n.view = (n.view - f64::from(by) * SPAN).clamp(-SPAN / 2.0, (n.duration - SPAN / 2.0).max(0.0));
    }
    render_wave(app);
    save(app);
}

/// A correction of the pitch in use (from A3 when none was found).
fn correct(app: &App, f: impl Fn(NotePitch) -> NotePitch) {
    {
        let mut rt = app.create.borrow_mut();
        let Some(n) = rt.note.as_mut() else { return };
        let base = n.pitch().unwrap_or(NotePitch::new(NO_PITCH_START, 0));
        n.set = Some(f(base));
    }
    show(app);
    save(app);
    prepare_pads(app);
}

/// A typed note name ("A3", "F#2", "Bb4 +12"); false when it is none.
pub(crate) fn type_note(app: &App, text: &str) -> bool {
    if app.create.borrow().note.is_none() {
        return false;
    }
    match NotePitch::parse(text) {
        Some(p) => {
            correct(app, |_| p);
            true
        }
        None => {
            app.status(od_i18n::trf("Not a note: {}. Type one such as A3, F#2 or Bb4", &[&text.trim()]));
            false
        }
    }
}

/// The audio of the note's track: at hand, or decoded in the background
/// (`None` until it is in).
fn audio(app: &App) -> Option<Audio> {
    let (id, path) = {
        let rt = app.create.borrow();
        let n = rt.note.as_ref()?;
        (n.track?, n.path.clone())
    };
    let mut rt = app.create.borrow_mut();
    if let Some((t, a)) = &rt.audio
        && *t == id
    {
        return Some(a.clone());
    }
    if rt.loading != Some(id) {
        rt.loading = Some(id);
        let tx = rt.tx.clone();
        std::thread::spawn(move || {
            let a = od_decode::decode(&path, od_decode::Layout::Mono)
                .map_err(|e| tracing::warn!(error = %e, "note source not decoded"))
                .ok()
                .map(|d| Audio::File { samples: d.samples.into(), rate: d.sample_rate });
            let _ = tx.send((id, a));
        });
    }
    None
}

/// Measures the pitch from IN to OUT (once the audio is in).
fn measure(app: &App) {
    let Some(a) = audio(app) else {
        app.create.borrow_mut().measure_due = true;
        show(app);
        return;
    };
    {
        let mut rt = app.create.borrow_mut();
        rt.measure_due = false;
        let Some(n) = rt.note.as_mut() else { return };
        let x = a.mono(n.start, n.end);
        let e = pitch::detect_pitch(&x, a.rate());
        n.measured(&e);
    }
    show(app);
    save(app);
    prepare_pads(app);
}

/// HEAR: plays IN to OUT with its fades on the sampler's audition voice, to
/// the headphones while a deck plays, else to the master; again stops it.
pub(crate) fn hear(app: &App) {
    let hearing = {
        let rt = app.create.borrow();
        rt.hearing.is_some() && rt.pad_playing.is_none() && rt.key_playing.is_none()
    };
    if hearing {
        stop_hearing(app);
        return;
    }
    let Some(n) = note(app) else { return };
    if !engine::ensure_engine(app) {
        engine::show_message(app);
        return;
    }
    let Some(a) = audio(app) else {
        app.create.borrow_mut().hear_due = true;
        return;
    };
    let rate = a.rate();
    let mut samples = a.stereo(n.start, n.end);
    if samples.is_empty() {
        return;
    }
    apply_fades(&mut samples, rate, n.fade());
    let engine_rate = app.engine.borrow().sample_rate();
    let sample = SampleData::new(od_decode::DecodedAudio { sample_rate: rate, channels: 2, samples }, engine_rate);
    audition(app, sample, None, None);
}

/// Plays `sample` on the audition voice: to the headphones while a deck
/// plays on air, else to the master. `pad` says which pad it is, `key`
/// which key of the strip (HEAR: neither).
fn audition(app: &App, sample: Arc<SampleData>, pad: Option<usize>, key: Option<i32>) {
    let cue = to_cue(app);
    engine::send(app, Command::Sampler(SamplerCommand::Audition { sample, cue }));
    {
        let mut rt = app.create.borrow_mut();
        rt.hearing = Some(Instant::now());
        rt.pad_playing = pad;
        rt.key_playing = key;
    }
    show(app);
}

/// The audition voice plays to the headphones while a deck plays on air.
fn to_cue(app: &App) -> bool {
    engine::snapshot(app).is_some_and(|s| {
        let count = app.decks.borrow().deck_count.min(s.decks.len());
        s.decks[..count].iter().any(|d| d.playing)
    })
}

fn stop_hearing(app: &App) {
    let was = {
        let mut rt = app.create.borrow_mut();
        rt.pad_playing = None;
        rt.key_playing = None;
        rt.hearing.take().is_some()
    };
    if was {
        engine::send(app, Command::Sampler(SamplerCommand::StopAudition));
        show(app);
    }
}

/// Every frame: the decoded audio comes in, HEAR ends with its note.
pub(crate) fn poll(app: &App, state: Option<&EngineState>) {
    let got: Vec<(TrackId, Option<Audio>)> = app.create.borrow().rx.try_iter().collect();
    for (id, a) in got {
        let (measure_due, hear_due, pads_due) = {
            let mut rt = app.create.borrow_mut();
            if rt.loading == Some(id) {
                rt.loading = None;
            }
            let current = rt.note.as_ref().and_then(|n| n.track) == Some(id);
            match a {
                Some(a) if current => rt.audio = Some((id, a)),
                None if current => {
                    // Nothing to measure: say so instead of waiting.
                    if let Some(n) = rt.note.as_mut() {
                        n.why = "none".into();
                    }
                    rt.measure_due = false;
                    rt.hear_due = false;
                    rt.pads_due = false;
                    rt.due = None;
                }
                _ => {}
            }
            (rt.measure_due && current, std::mem::take(&mut rt.hear_due) && current, std::mem::take(&mut rt.pads_due) && current)
        };
        if measure_due {
            // Measuring renders the pads too.
            measure(app);
        } else {
            show(app);
            if pads_due {
                prepare_pads(app);
            }
        }
        if hear_due {
            hear(app);
        }
    }
    poll_rendered(app);
    tick_play(app, state);
    let ended = {
        let rt = app.create.borrow();
        rt.hearing.is_some_and(|t| t.elapsed() > Duration::from_millis(150) && !state.is_some_and(|s| s.auditioning))
    };
    if ended {
        let mut rt = app.create.borrow_mut();
        rt.hearing = None;
        rt.pad_playing = None;
        rt.key_playing = None;
        drop(rt);
        show(app);
    }
}

/// Draws the zoomed waveform of the note's view (from the track's stored
/// analysis, as the player draws it).
fn render_wave(app: &App) {
    let Some(n) = note(app) else {
        app.create.borrow_mut().wave = slint::Image::default();
        return;
    };
    let scale = app.ui.window().scale_factor();
    let image = {
        let st = app.state.borrow();
        let a = n.track.and_then(|id| st.src().analysis(id).ok().or_else(|| st.lib.analysis(id).ok())).unwrap_or_default();
        let w = louder(&a.waveform, n.view, SPAN);
        crate::render::detail_tile(&w, &a.beatgrid, &[], n.view, SPAN, (WAVE_W * scale) as u32, (WAVE_H * scale) as u32)
    };
    app.create.borrow_mut().wave = image;
    show(app);
}

/// The waveform raised so the loudest column of `span` seconds from
/// `start` fills the well (up to four times): a quiet passage still shows
/// the shape of its notes.
pub(crate) fn louder(w: &od_core::Waveform, start: f64, span: f64) -> od_core::Waveform {
    let rate = w.columns_per_second;
    let a = ((start.max(0.0) * rate) as usize).min(w.columns.len());
    let b = (((start + span).max(0.0) * rate) as usize).min(w.columns.len());
    let peak = w.columns[a..b].iter().map(|c| c.peak).max().unwrap_or(0).max(1);
    let gain = (255.0 / f32::from(peak)).clamp(1.0, 4.0);
    let up = |v: u8| (f32::from(v) * gain).min(255.0) as u8;
    od_core::Waveform {
        columns_per_second: rate,
        columns: w
            .columns
            .iter()
            .map(|c| od_core::WaveformColumn { low: up(c.low), mid: up(c.mid), high: up(c.high), peak: up(c.peak) })
            .collect(),
    }
}

/// Logical size the waveform is drawn at (the well stretches it).
const WAVE_W: f32 = 320.0;
const WAVE_H: f32 = 64.0;

/// What SOURCE shows of the note.
pub(crate) fn note_view(n: &PickedNote, measuring: bool) -> crate::CreateNote {
    let at = |t: f64| ((t - n.view) / SPAN) as f32;
    let (name, cents) = match n.pitch() {
        Some(p) => (p.name(), p.cents_text()),
        None => ("—".into(), String::new()),
    };
    crate::CreateNote {
        picked: true,
        source: n.source.clone().into(),
        title: n.title.clone().into(),
        in_at: at(n.start),
        out_at: at(n.end),
        fade_at: at(n.end - n.fade()),
        length: format!("{:.2} s", n.end - n.start).into(),
        start: crate::format::cue_time(n.start).into(),
        note: name.into(),
        cents: cents.into(),
        state: if measuring && n.set.is_none() { "measuring" } else { n.badge() }.into(),
        why: n.why.clone().into(),
        ..Default::default()
    }
}

fn show(app: &App) {
    let view = {
        let rt = app.create.borrow();
        rt.note.as_ref().map(|n| {
            let mut v = note_view(n, rt.measure_due);
            v.wave = rt.wave.clone();
            v.hearing = rt.hearing.is_some() && rt.pad_playing.is_none() && rt.key_playing.is_none();
            v
        })
    };
    let g = app.ui.global::<crate::Create>();
    g.set_note(view.unwrap_or_default());
    show_key(app);
    show_pads(app);
}

// ── [72] The note on pads ───────────────────────────────────────────────

/// Pads in NOTES.
pub(crate) const PADS: usize = 8;
/// The octave switch goes this many octaves down and up.
pub(crate) const OCTAVES: i32 = 2;

/// What a rendered note depends on: the region, its pitch (and so its
/// fades) and the engine rate it is played at.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderKey {
    pub track: TrackId,
    pub start: f64,
    pub end: f64,
    pub pitch: NotePitch,
    pub rate: u32,
}

impl RenderKey {
    fn of(n: &PickedNote, rate: u32) -> Option<Self> {
        Some(Self { track: n.track?, start: n.start, end: n.end, pitch: n.pitch()?, rate })
    }
}

/// The notes rendered for one [`RenderKey`], by target (MIDI note): a pad
/// plays the same buffer every time, at once. A new key empties it.
#[derive(Default)]
pub(crate) struct NoteCache {
    key: Option<RenderKey>,
    notes: HashMap<i32, (Arc<SampleData>, ShiftMethod)>,
    /// Asked of the worker and not in yet.
    pending: HashSet<i32>,
}

impl NoteCache {
    /// Makes `key` the current one; true when that dropped what was there.
    pub fn select(&mut self, key: &RenderKey) -> bool {
        if self.key.as_ref() == Some(key) {
            return false;
        }
        self.key = Some(key.clone());
        self.notes.clear();
        self.pending.clear();
        true
    }

    pub fn get(&self, target: i32) -> Option<Arc<SampleData>> {
        self.notes.get(&target).map(|(s, _)| s.clone())
    }

    #[cfg(test)]
    pub fn method(&self, target: i32) -> Option<ShiftMethod> {
        self.notes.get(&target).map(|(_, m)| *m)
    }

    /// Keeps a rendered note if it is of the current key.
    pub fn put(&mut self, key: &RenderKey, target: i32, sample: Arc<SampleData>, method: ShiftMethod) -> bool {
        if self.key.as_ref() != Some(key) {
            return false;
        }
        self.pending.remove(&target);
        self.notes.insert(target, (sample, method));
        true
    }

    /// Of `targets`, the ones neither rendered nor asked for (now asked for).
    fn missing(&mut self, targets: &[i32]) -> Vec<i32> {
        let out: Vec<i32> = targets.iter().copied().filter(|t| !self.notes.contains_key(t) && !self.pending.contains(t)).collect();
        self.pending.extend(&out);
        out
    }
}

/// A note rendered by the worker.
struct Rendered {
    key: RenderKey,
    target: i32,
    sample: Arc<SampleData>,
    method: ShiftMethod,
}

/// The pads' notes (MIDI), moved by `octave` octaves.
///
/// Without a key to snap to (`key` is `None`: SNAP TO SCALE off, or no key
/// chosen) the source note and the 7 semitones above it. With one, the
/// scale's notes from the root at or below the source note: the source note
/// lies on the root or up to 11 semitones above it, and the 8 pads run from
/// that root up the scale to its octave (A minor from F#3: A2 B2 C3 D3 E3
/// F3 G3 A3).
pub(crate) fn pad_targets(source: NotePitch, octave: i32, key: Option<ScaleKey>) -> [i32; PADS] {
    match key {
        None => std::array::from_fn(|i| source.midi() + 12 * octave + i as i32),
        Some(k) => {
            let notes = k.notes_from(k.root_at_or_below(source.midi()) + 12 * octave, PADS);
            std::array::from_fn(|i| notes[i])
        }
    }
}

/// The semitones from the source pitch (with its cents) to `target`.
pub(crate) fn shift_to(source: NotePitch, target: i32) -> f64 {
    f64::from(target * 100 - source.total_cents()) / 100.0
}

/// A note's name with its octave: as `key` spells it, else as the app does.
pub(crate) fn note_name(key: Option<ScaleKey>, midi: i32) -> String {
    key.map_or_else(|| NotePitch::new(midi, 0).name(), |k| k.note_name(midi))
}

/// What the pads show: each target's name (spelled as the key in use,
/// `spell`), its distance from the source note in semitones and the FAR
/// warning beyond 7, whether it is rendered and whether it plays. Disabled
/// without a pitch.
pub(crate) fn pad_rows(
    source: Option<NotePitch>,
    octave: i32,
    key: Option<ScaleKey>,
    spell: Option<ScaleKey>,
    cache: &NoteCache,
    playing: Option<usize>,
) -> Vec<crate::CreatePad> {
    let Some(p) = source else {
        return vec![crate::CreatePad::default(); PADS];
    };
    pad_targets(p, octave, key)
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            let shift = t - p.midi();
            crate::CreatePad {
                enabled: true,
                note: note_name(spell, t).into(),
                shift,
                far: noteshift::far(shift),
                ready: cache.get(t).is_some(),
                playing: playing == Some(i),
            }
        })
        .collect()
}

/// The first key of the strip: the C at or below the lowest pad (C3
/// without a pitch). Its two octaves hold every pad.
pub(crate) fn strip_start(targets: Option<&[i32; PADS]>) -> i32 {
    targets.map_or(48, |t| t[0] - t[0].rem_euclid(12))
}

/// The keys of the strip from `start`: each note's name (spelled as the
/// key), whether it is the
/// root, in the scale or out of it ("" without a key), on a pad, and
/// whether it plays.
pub(crate) fn strip_rows(start: i32, key: Option<ScaleKey>, pads: &[i32], playing: &[i32]) -> Vec<crate::CreateKeyNote> {
    (start..start + STRIP_KEYS as i32)
        .map(|m| crate::CreateKeyNote {
            name: note_name(key, m).into(),
            kind: match key {
                None => "",
                Some(k) if m.rem_euclid(12) == i32::from(k.root) => "root",
                Some(k) if k.contains(m) => "scale",
                Some(_) => "out",
            }
            .into(),
            pad: pads.contains(&m),
            playing: playing.contains(&m),
        })
        .collect()
}

fn set_rows<T: Clone + PartialEq + 'static>(model: ModelRc<T>, rows: Vec<T>, set: impl FnOnce(ModelRc<T>)) {
    if model.row_count() == rows.len() {
        for (i, row) in rows.into_iter().enumerate() {
            if model.row_data(i).as_ref() != Some(&row) {
                model.set_row_data(i, row);
            }
        }
    } else {
        set(ModelRc::new(VecModel::from(rows)));
    }
}

fn show_pads(app: &App) {
    let (rows, keys, start, octave, why, nearer, on) = {
        let rt = app.create.borrow();
        let source = rt.note.as_ref().and_then(PickedNote::pitch);
        let key = rt.choice.pad_key();
        let rows = pad_rows(source, rt.octave, key, rt.choice.key, &rt.cache, rt.pad_playing);
        let targets = source.map(|p| pad_targets(p, rt.octave, key));
        let why = match &rt.note {
            None => "none",
            Some(_) if source.is_none() => "pitch",
            Some(_) => "",
        };
        // FAR pads: a source note in the middle of them is better.
        let nearer = match &targets {
            Some(t) if rows.iter().any(|r| r.far) => note_name(rt.choice.key, (t[0] + t[PADS - 1]) / 2),
            _ => String::new(),
        };
        let start = strip_start(targets.as_ref());
        let mut playing: Vec<i32> = rt.key_playing.into_iter().collect();
        if let (Some(t), Some(i)) = (&targets, rt.pad_playing) {
            playing.extend(t.get(i));
        }
        let keys = strip_rows(start, rt.choice.key, targets.as_ref().map_or(&[][..], |t| &t[..]), &playing);
        (rows, keys, start, rt.octave, why, nearer, source.is_some())
    };
    let g = app.ui.global::<crate::Create>();
    g.set_octave(octave);
    g.set_pads_why(why.into());
    g.set_pads_nearer(nearer.into());
    g.set_keyboard_on(on);
    g.set_keyboard_start(start);
    set_rows(g.get_pads(), rows, |m| g.set_pads(m));
    set_rows(g.get_keyboard(), keys, |m| g.set_keyboard(m));
}

/// Renders the pads' notes that are not in the cache yet, on a worker
/// thread (once the audio is in). A changed note or pitch starts afresh
/// and stops the worker of the old one.
fn prepare_pads(app: &App) {
    let rate = app.engine.borrow().sample_rate();
    let Some((key, octave, scale, n)) = ({
        let rt = app.create.borrow();
        rt.note.as_ref().and_then(|n| RenderKey::of(n, rate).map(|k| (k, rt.octave, rt.choice.pad_key(), n.clone())))
    }) else {
        return;
    };
    let Some(a) = audio(app) else {
        app.create.borrow_mut().pads_due = true;
        return;
    };
    let (targets, tx, generation, my) = {
        let mut rt = app.create.borrow_mut();
        if rt.cache.select(&key) {
            rt.generation.fetch_add(1, Ordering::SeqCst);
        }
        // The pad or key pressed first, then the rest.
        let mut want = pad_targets(key.pitch, octave, scale).to_vec();
        // [74] And the notes of the strip, for PLAY.
        want.extend(rt.melody.steps.iter().flatten().map(|s| s.note));
        if let Some(d) = rt.due {
            want.insert(0, d.target);
        }
        let targets = rt.cache.missing(&want);
        (targets, rt.rendered_tx.clone(), rt.generation.clone(), rt.generation.load(Ordering::SeqCst))
    };
    show_pads(app);
    if targets.is_empty() {
        return;
    }
    let src_rate = a.rate();
    let fade = n.fade();
    let (start, end) = (n.start, n.end);
    std::thread::spawn(move || {
        let mut note = a.stereo(start, end);
        if note.is_empty() {
            return;
        }
        apply_fades(&mut note, src_rate, fade);
        for target in targets {
            if generation.load(Ordering::SeqCst) != my {
                return;
            }
            let shifted = noteshift::shift_note(&note, src_rate, shift_to(key.pitch, target), fade);
            let audio = od_decode::DecodedAudio { sample_rate: src_rate, channels: 2, samples: shifted.samples };
            let sample = SampleData::new(audio, key.rate);
            let r = Rendered { key: key.clone(), target, sample, method: shifted.method };
            if tx.send(r).is_err() {
                return;
            }
        }
    });
}

/// Rendered notes come in: kept, and a pad or key waiting for one plays it.
fn poll_rendered(app: &App) {
    let got: Vec<Rendered> = app.create.borrow().rendered_rx.try_iter().collect();
    if got.is_empty() {
        return;
    }
    let mut play = None;
    {
        let mut rt = app.create.borrow_mut();
        for r in got {
            if rt.cache.put(&r.key, r.target, r.sample.clone(), r.method)
                && let Some(d) = rt.due.filter(|d| d.target == r.target)
            {
                rt.due = None;
                play = Some((r.sample, d));
            }
        }
    }
    match play {
        Some((sample, d)) => audition(app, sample, d.pad, d.pad.is_none().then_some(d.target)),
        None => show_pads(app),
    }
}

/// Plays `target` (MIDI) for pad `pad` or a key of the strip: at once when
/// rendered, else as soon as it is.
fn play_target(app: &App, target: i32, pad: Option<usize>) {
    if !engine::ensure_engine(app) {
        engine::show_message(app);
        return;
    }
    let rate = app.engine.borrow().sample_rate();
    let cached = {
        let rt = app.create.borrow();
        let current = rt.note.as_ref().and_then(|n| RenderKey::of(n, rate)) == rt.cache.key;
        if current { rt.cache.get(target) } else { None }
    };
    match cached {
        Some(sample) => audition(app, sample, pad, pad.is_none().then_some(target)),
        None => {
            app.create.borrow_mut().due = Some(Due { target, pad });
            prepare_pads(app);
        }
    }
}

/// A pad pressed: its note plays at once when rendered, else as soon as it
/// is. Again restarts it.
pub(crate) fn play_pad(app: &App, i: usize) {
    let target = {
        let rt = app.create.borrow();
        let Some(p) = rt.note.as_ref().and_then(PickedNote::pitch) else { return };
        match pad_targets(p, rt.octave, rt.choice.pad_key()).get(i) {
            Some(&t) => t,
            None => return,
        }
    };
    play_target(app, target, Some(i));
    record(app, target);
}

/// [73] Key `i` of the strip pressed: its note plays as a pad's would.
pub(crate) fn play_key(app: &App, i: i32) {
    let target = {
        let rt = app.create.borrow();
        let Some(p) = rt.note.as_ref().and_then(PickedNote::pitch) else { return };
        if !(0..STRIP_KEYS as i32).contains(&i) {
            return;
        }
        strip_start(Some(&pad_targets(p, rt.octave, rt.choice.pad_key()))) + i
    };
    play_target(app, target, None);
    record(app, target);
}

/// The octave switch: the pads an octave down or up (within ±2).
pub(crate) fn step_octave(app: &App, d: i32) {
    {
        let mut rt = app.create.borrow_mut();
        rt.octave = (rt.octave + d.signum()).clamp(-OCTAVES, OCTAVES);
        rt.due = None;
    }
    show_pads(app);
    prepare_pads(app);
}

/// The rendered note of pad `i` and how it was made, if it is in (tests).
#[cfg(test)]
pub(crate) fn pad_note(app: &App, i: usize) -> Option<(Arc<SampleData>, ShiftMethod)> {
    let rt = app.create.borrow();
    let p = rt.note.as_ref()?.pitch()?;
    let t = *pad_targets(p, rt.octave, rt.choice.pad_key()).get(i)?;
    Some((rt.cache.get(t)?, rt.cache.method(t)?))
}

// ── [73] Key and scale ──────────────────────────────────────────────────

/// Library meta key of the key and scale (JSON).
pub(crate) const META_KEY: &str = "create.key";
/// Keys of the strip: two octaves from a C.
pub(crate) const STRIP_KEYS: usize = 24;
/// How often NEXT TRACK looks up the next track while CREATE is shown.
const NEXT_EVERY: Duration = Duration::from_millis(500);

/// The key NOTES plays in, where it comes from, and how the pads use it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct KeyChoice {
    /// The key in use (`None`: none found or chosen yet).
    pub key: Option<ScaleKey>,
    /// "deck" (follows the other deck), "next" (taken by NEXT TRACK) or
    /// "set" (by you).
    pub from: String,
    /// The deck the key follows, 1-based (0: none).
    pub deck: usize,
    /// SNAP TO SCALE: the pads hold the scale's notes.
    pub snap: bool,
    /// HAPPY and SAD beside MAJOR and MINOR.
    pub hints: bool,
}

impl Default for KeyChoice {
    fn default() -> Self {
        Self { key: None, from: "deck".into(), deck: 0, snap: true, hints: true }
    }
}

impl KeyChoice {
    /// The key the pads snap to (none with SNAP TO SCALE off).
    pub fn pad_key(&self) -> Option<ScaleKey> {
        self.key.filter(|_| self.snap)
    }
}

/// The deck whose key is the default, 0-based: for a note from deck 1 or 2
/// the other one; for a note from the player (or none) the deck playing
/// on air, else any deck playing, else the first with a key.
pub(crate) fn key_deck(source: &str, keys: &[Option<MusicalKey>], playing: &[bool], on_air: Option<usize>) -> Option<usize> {
    let has = |d: usize| keys.get(d).is_some_and(Option::is_some);
    let plays = |d: usize| playing.get(d) == Some(&true);
    match source {
        "deck1" => Some(1),
        "deck2" => Some(0),
        _ => on_air
            .filter(|&d| plays(d) && has(d))
            .or_else(|| (0..keys.len()).find(|&d| plays(d) && has(d)))
            .or_else(|| (0..keys.len()).find(|&d| has(d))),
    }
}

/// While the key follows a deck: the other deck's key, as it is now.
fn follow_deck(app: &App, state: Option<&EngineState>) {
    let source = {
        let rt = app.create.borrow();
        if rt.choice.from != "deck" {
            return;
        }
        rt.note.as_ref().map(|n| n.source.clone()).unwrap_or_default()
    };
    let count = app.decks.borrow().deck_count;
    let keys: Vec<Option<MusicalKey>> = (0..count).map(|i| crate::decks::musical_key(app, i)).collect();
    let playing: Vec<bool> = (0..count).map(|i| state.is_some_and(|s| s.decks.get(i).is_some_and(|d| d.playing))).collect();
    let deck = key_deck(&source, &keys, &playing, state.and_then(|s| s.audible.dominant));
    let key = deck.and_then(|d| keys.get(d).copied().flatten()).map(ScaleKey::from_key);
    let deck = deck.map_or(0, |d| d + 1);
    let changed = {
        let mut rt = app.create.borrow_mut();
        let c = &mut rt.choice;
        let changed = c.key != key || c.deck != deck;
        c.key = key;
        c.deck = deck;
        changed
    };
    if changed {
        key_changed(app);
    }
}

/// What NEXT TRACK would take: the next track in the queue, else in the
/// set plan, with its key.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NextTrack {
    pub title: String,
    pub key: Option<MusicalKey>,
    /// "" (it has a key), "none" (no next track) or "no-key".
    pub why: &'static str,
}

fn next_track(app: &App) -> NextTrack {
    let Some(id) = crate::queue::next_track(app).or_else(|| crate::set_plan::next_track(app)) else {
        return NextTrack { why: "none", ..Default::default() };
    };
    let st = app.state.borrow();
    let Ok(t) = st.lib.track(id) else {
        return NextTrack { why: "none", ..Default::default() };
    };
    NextTrack { title: t.tags.title.clone(), key: t.key, why: if t.key.is_some() { "" } else { "no-key" } }
}

fn refresh_next(app: &App) {
    let next = next_track(app);
    {
        let mut rt = app.create.borrow_mut();
        rt.next_at = Some(Instant::now());
        if rt.next == next {
            return;
        }
        rt.next = next.clone();
    }
    let g = app.ui.global::<crate::Create>();
    g.set_next_why(next.why.into());
    g.set_next_title(next.title.into());
}

/// NEXT TRACK: the next track's key (it stays when the queue moves on).
pub(crate) fn key_from_next(app: &App) {
    refresh_next(app);
    let next = app.create.borrow().next.clone();
    let Some(k) = next.key else {
        app.status(if next.why == "none" {
            od_i18n::tr("No next track: add one to the queue or open a set").to_string()
        } else {
            od_i18n::trf("{} has no key yet: analyse it first", &[&next.title])
        });
        return;
    };
    {
        let mut rt = app.create.borrow_mut();
        rt.choice.key = Some(ScaleKey::from_key(k));
        rt.choice.from = "next".into();
        rt.choice.deck = 0;
    }
    key_changed(app);
}

/// Back to the other deck's key.
pub(crate) fn key_from_deck(app: &App) {
    app.create.borrow_mut().choice.from = "deck".into();
    follow_deck(app, engine::snapshot(app).as_ref());
    key_changed(app);
}

/// A key chosen by hand starts from the source note (else A) in minor.
fn hand_key(rt: &Runtime) -> ScaleKey {
    rt.choice.key.unwrap_or_else(|| {
        let root = rt.note.as_ref().and_then(PickedNote::pitch).map_or(9, NotePitch::midi);
        ScaleKey::new(root, Scale::Minor)
    })
}

/// The root a semitone down or up (SET BY YOU).
pub(crate) fn step_root(app: &App, d: i32) {
    {
        let mut rt = app.create.borrow_mut();
        let k = hand_key(&rt);
        rt.choice.key = Some(ScaleKey::new(i32::from(k.root) + d.signum(), k.scale));
        rt.choice.from = "set".into();
    }
    key_changed(app);
}

/// A scale chosen (SET BY YOU); the one in use again changes nothing.
pub(crate) fn choose_scale(app: &App, id: &str) {
    let Some(scale) = Scale::from_id(id) else { return };
    {
        let mut rt = app.create.borrow_mut();
        let k = hand_key(&rt);
        if rt.choice.key.is_some_and(|k| k.scale == scale) {
            return;
        }
        rt.choice.key = Some(ScaleKey { scale, ..k });
        rt.choice.from = "set".into();
    }
    key_changed(app);
}

pub(crate) fn set_snap(app: &App, on: bool) {
    {
        let mut rt = app.create.borrow_mut();
        rt.choice.snap = on;
        rt.due = None;
    }
    key_changed(app);
}

pub(crate) fn set_hints(app: &App, on: bool) {
    app.create.borrow_mut().choice.hints = on;
    show_key(app);
    save_key(app);
}

/// The key or how the pads use it changed: shown, kept, the pads rendered.
fn key_changed(app: &App) {
    show_key(app);
    show_pads(app);
    show_arrange(app);
    save_key(app);
    prepare_pads(app);
}

fn save_key(app: &App) {
    if app.state.borrow().read_only() {
        return;
    }
    let json = serde_json::to_string(&app.create.borrow().choice).unwrap_or_default();
    if let Err(e) = app.state.borrow_mut().lib.set_meta(META_KEY, &json) {
        tracing::warn!(error = %e, "key and scale not saved");
    }
}

/// What NOTES and FITS show of the key: the root and scale, the wheel
/// notation as a small label beside (none for the modes, or when keys are
/// shown as note names anyway), where it comes from, its notes and chords.
pub(crate) fn key_view(c: &KeyChoice, notation: KeyNotation) -> crate::CreateKey {
    let Some(k) = c.key else {
        return crate::CreateKey { from: c.from.clone().into(), deck: c.deck as i32, ..Default::default() };
    };
    let wheel = k.musical_key().filter(|_| notation != KeyNotation::Musical).map(|m| m.format(notation)).unwrap_or_default();
    crate::CreateKey {
        chosen: true,
        root: k.root_name().into(),
        scale: k.scale.id().into(),
        wheel: wheel.into(),
        from: c.from.clone().into(),
        deck: c.deck as i32,
        notes: k.note_names().join(" ").into(),
        chords: k.chord_names().join(" ").into(),
    }
}

fn show_key(app: &App) {
    let notation = app.state.borrow().settings.key_notation();
    let (view, snap, hints) = {
        let rt = app.create.borrow();
        (key_view(&rt.choice, notation), rt.choice.snap, rt.choice.hints)
    };
    let g = app.ui.global::<crate::Create>();
    g.set_key(view);
    g.set_snap(snap);
    g.set_hints(hints);
}

/// The key and scale in use (tests).
#[cfg(test)]
pub(crate) fn choice(app: &App) -> KeyChoice {
    app.create.borrow().choice.clone()
}

// ── [74] Arrange ────────────────────────────────────────────────────────

/// Library meta key of the arrange strip (JSON).
pub(crate) const META_MELODY: &str = "create.melody";
/// PLAY sends a note at most this many beats before it starts.
const AHEAD: f64 = 1.0;

/// The master deck's beat and its place in the bar (1..=4, 0 without a
/// grid), while it plays.
fn master_beat(s: &EngineState) -> Option<(f64, u8)> {
    let m = s.master_deck?;
    let d = s.decks.get(m)?;
    (d.playing && d.track.is_some()).then_some((d.beat, d.beat_in_bar))
}

/// Where PLAY starts: on the next bar while the master deck plays, else at
/// once on the running clock.
fn play_start(s: &EngineState) -> f64 {
    match master_beat(s) {
        Some((beat, in_bar)) => arrange::bar_start(beat, in_bar) + f64::from(arrange::BEATS_PER_BAR),
        None => s.clock_beat,
    }
}

/// What the strip shows: each step's note (spelled as `key`), whether the
/// note before holds over it, the gap before it (a beat, a bar), the bar it
/// starts, and whether it is selected, sounds or is REC's next step.
pub(crate) fn step_rows(
    m: &Melody,
    key: Option<ScaleKey>,
    selected: Option<usize>,
    playing: Option<usize>,
    cursor: Option<usize>,
) -> Vec<crate::CreateStep> {
    let (per_beat, per_bar) = (m.grid.per_beat() as usize, m.grid.per_bar() as usize);
    (0..STEPS)
        .map(|i| crate::CreateStep {
            note: m.steps[i].map(|s| arrange::name(key, s.note)).unwrap_or_default().into(),
            held: m.steps[i].is_none() && m.held_by(i).is_some(),
            gap: if i == 0 {
                0
            } else if i % per_bar == 0 {
                2
            } else {
                i32::from(i % per_beat == 0)
            },
            bar: if i % per_bar == 0 { (i / per_bar + 1) as i32 } else { 0 },
            selected: selected == Some(i),
            playing: playing == Some(i),
            cursor: cursor == Some(i),
        })
        .collect()
}

fn show_arrange(app: &App) {
    let (rows, m, text, rt_sel, rec, rec_grid, playing, undo, bad, long) = {
        let rt = app.create.borrow();
        let cursor = rt.rec.filter(|_| !rt.rec_grid).map(|r| r.cursor).filter(|&c| c < STEPS);
        let playing = rt.play.and_then(|p| p.step);
        let rows = step_rows(&rt.melody, rt.choice.key, rt.selected, playing, cursor);
        (rows, rt.melody.clone(), rt.melody.text(rt.choice.key), rt.selected, rt.rec, rt.rec_grid, rt.play.is_some(), rt.undo.is_some(), rt.typed_bad.join(", "), rt.typed_long)
    };
    let g = app.ui.global::<crate::Create>();
    set_rows(g.get_steps(), rows, |r| g.set_steps(r));
    g.set_grid(m.grid.id().into());
    g.set_steps_text(text.into());
    g.set_notes(m.notes() as i32);
    g.set_bars(m.bars() as i32);
    g.set_looped(m.looped);
    g.set_selected(rt_sel.map_or(-1, |i| i as i32));
    g.set_rec(rec.is_some());
    g.set_rec_mode(if rec_grid { "grid" } else { "steps" }.into());
    g.set_rec_cursor(rec.map_or(0, |r| r.cursor as i32));
    g.set_playing(playing);
    g.set_can_undo(undo);
    g.set_typed_bad(bad.into());
    g.set_typed_long(long);
}

fn save_melody(app: &App) {
    if app.state.borrow().read_only() {
        return;
    }
    let json = serde_json::to_string(&app.create.borrow().melody).unwrap_or_default();
    if let Err(e) = app.state.borrow_mut().lib.set_meta(META_MELODY, &json) {
        tracing::warn!(error = %e, "melody not saved");
    }
}

/// The strip changed: shown, kept, its notes rendered. An edit after CLEAR
/// or typing leaves nothing for UNDO to bring back.
fn edited(app: &App, keep_undo: bool) {
    {
        let mut rt = app.create.borrow_mut();
        if !keep_undo {
            rt.undo = None;
        }
    }
    show_arrange(app);
    save_melody(app);
    prepare_pads(app);
}

/// The grid: 1/4, 1/8 or 1/16 a step (the notes keep their steps).
pub(crate) fn set_grid(app: &App, id: &str) {
    let Some(grid) = Grid::from_id(id) else { return };
    app.create.borrow_mut().melody.grid = grid;
    restart_play(app);
    show_arrange(app);
    save_melody(app);
}

pub(crate) fn set_loop(app: &App, on: bool) {
    app.create.borrow_mut().melody.looped = on;
    show_arrange(app);
    save_melody(app);
}

/// REC: armed, the next note goes on the selected step (else after the
/// last note); again disarms it.
pub(crate) fn toggle_rec(app: &App) {
    {
        let mut rt = app.create.borrow_mut();
        rt.rec = match rt.rec {
            Some(_) => None,
            None => Some(Rec { origin: None, cursor: rt.selected.unwrap_or(rt.melody.used() % STEPS) }),
        };
    }
    show_arrange(app);
}

/// A note played on a pad or a key while REC is armed: on the nearest step
/// of PLAY's grid, else of the master deck's (counted from the bar it was
/// in at the first note), else on the cursor's step, which moves on. The
/// decks are only read.
pub(crate) fn record(app: &App, target: i32) {
    if app.create.borrow().rec.is_none() {
        return;
    }
    let state = engine::snapshot(app);
    let step = {
        let mut rt = app.create.borrow_mut();
        let grid = rt.melody.grid;
        let play = rt.play;
        let Some(rec) = rt.rec.as_mut() else { return };
        let on_grid = match (play, state.as_ref().and_then(master_beat)) {
            (Some(p), _) => state.as_ref().map(|s| (s.clock_beat, p.start)),
            (None, Some((beat, in_bar))) => Some((beat, *rec.origin.get_or_insert(arrange::bar_start(beat, in_bar)))),
            (None, None) => None,
        };
        match on_grid {
            Some((beat, origin)) => Some(arrange::nearest_step(beat, origin, grid)),
            None if rec.cursor < STEPS => {
                rec.cursor += 1;
                Some(rec.cursor - 1)
            }
            None => None,
        }
    };
    let Some(i) = step else {
        app.status(od_i18n::tr("The strip is full: select a step to record from there, or CLEAR it").to_string());
        return;
    };
    {
        let mut rt = app.create.borrow_mut();
        rt.melody.set(i, target);
        rt.selected = Some(i);
    }
    edited(app, false);
}

/// A step tapped: selected, its note played (unless PLAY runs); REC's
/// cursor moves there.
pub(crate) fn select_step(app: &App, i: i32) {
    let Ok(i) = usize::try_from(i) else { return };
    if i >= STEPS {
        return;
    }
    let note = {
        let mut rt = app.create.borrow_mut();
        rt.selected = Some(i);
        if let Some(r) = rt.rec.as_mut() {
            r.cursor = i;
        }
        rt.melody.steps[i].map(|s| s.note).filter(|_| rt.play.is_none())
    };
    show_arrange(app);
    if let Some(n) = note {
        play_target(app, n, None);
    }
}

/// The selected note a step left or right (swapped with what is there);
/// on an empty step the selection moves instead.
pub(crate) fn move_step(app: &App, d: i32) {
    let moved = {
        let mut rt = app.create.borrow_mut();
        let Some(i) = rt.selected else { return };
        if rt.melody.steps[i].is_some() {
            let j = rt.melody.move_step(i, d);
            rt.selected = Some(j);
            j != i
        } else {
            rt.selected = Some((i as i32 + d.signum()).clamp(0, STEPS as i32 - 1) as usize);
            false
        }
    };
    if moved {
        edited(app, false);
    } else {
        show_arrange(app);
    }
}

/// The selected step emptied.
pub(crate) fn delete_step(app: &App) {
    {
        let mut rt = app.create.borrow_mut();
        let Some(i) = rt.selected else { return };
        if rt.melody.steps[i].is_none() {
            return;
        }
        rt.melody.delete(i);
    }
    edited(app, false);
}

/// The selected note a step of the scale up or down (a semitone with
/// `semitone`, or without a key); it plays at its new pitch.
pub(crate) fn shift_step(app: &App, d: i32, semitone: bool) {
    let note = {
        let mut rt = app.create.borrow_mut();
        let Some(i) = rt.selected else { return };
        let key = rt.choice.key;
        if !rt.melody.transpose(i, d, key, semitone) {
            return;
        }
        rt.melody.steps[i].map(|s| s.note).filter(|_| rt.play.is_none())
    };
    edited(app, false);
    if let Some(n) = note {
        play_target(app, n, None);
    }
}

/// Typed note names replace the strip from step 1 (UNDO brings the old one
/// back); what is not a note is reported and takes its step as a rest.
pub(crate) fn type_steps(app: &App, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let typed = arrange::parse(text);
    {
        let mut rt = app.create.borrow_mut();
        let old = rt.melody.clone();
        rt.melody.replace(typed.steps);
        if old.notes() > 0 && old != rt.melody {
            rt.undo = Some(old);
        }
        rt.typed_bad = typed.bad;
        rt.typed_long = typed.too_long;
        rt.selected = None;
    }
    restart_play(app);
    edited(app, true);
}

/// CLEAR: every step empty; UNDO brings them back.
pub(crate) fn clear_steps(app: &App) {
    {
        let mut rt = app.create.borrow_mut();
        if rt.melody.notes() == 0 {
            return;
        }
        let old = rt.melody.clone();
        rt.melody.clear();
        rt.undo = Some(old);
        rt.selected = None;
        rt.typed_bad.clear();
        rt.typed_long = false;
    }
    stop_play(app);
    edited(app, true);
}

/// UNDO: the strip as it was before the last CLEAR or typed notes.
pub(crate) fn undo_steps(app: &App) {
    {
        let mut rt = app.create.borrow_mut();
        let Some(old) = rt.undo.take() else { return };
        rt.melody = old;
        rt.selected = None;
        rt.typed_bad.clear();
        rt.typed_long = false;
    }
    edited(app, false);
}

/// PLAY: the strip through the audition voice, once or looped, in time
/// with the master (from the next bar while a deck plays); again stops it
/// at once.
pub(crate) fn toggle_play(app: &App) {
    if app.create.borrow().play.is_some() {
        stop_play(app);
        return;
    }
    if app.create.borrow().melody.notes() == 0 {
        return;
    }
    if !engine::ensure_engine(app) {
        engine::show_message(app);
        return;
    }
    let Some(s) = engine::snapshot(app) else { return };
    stop_hearing(app);
    app.create.borrow_mut().play = Some(Play { start: play_start(&s), next: 0, waiting: None, last: s.clock_beat, step: None });
    prepare_pads(app);
    show_arrange(app);
}

fn stop_play(app: &App) {
    if app.create.borrow_mut().play.take().is_some() {
        engine::send(app, Command::Sampler(SamplerCommand::StopAudition));
        show_arrange(app);
    }
}

/// While PLAY runs the strip changed under it: it starts again from step 1
/// as PLAY would.
fn restart_play(app: &App) {
    if app.create.borrow().play.is_some() {
        stop_play(app);
        toggle_play(app);
    }
}

/// Every frame while PLAY runs: the step that sounds, and the next note
/// sent to the audition voice with its beat once the one before has
/// started, from the render cache (a note not rendered yet is left out).
/// A jump of the clock (a deck started, stopped or cued) starts again on
/// the next bar; without LOOP PLAY ends after the last bar.
fn tick_play(app: &App, state: Option<&EngineState>) {
    let Some(s) = state else { return };
    if app.create.borrow().play.is_none() {
        return;
    }
    let rate = app.engine.borrow().sample_rate();
    let mut send = None;
    let (shown, ended) = {
        let mut guard = app.create.borrow_mut();
        let rt = &mut *guard;
        let Some(mut p) = rt.play else { return };
        let clock = s.clock_beat;
        let m = &rt.melody;
        let len = m.play_steps() as i64;
        let before = p.step;
        if clock < p.last - 0.25 || clock > p.last + 8.0 {
            p = Play { start: play_start(s), next: 0, waiting: None, last: clock, step: None };
            send = Some(None);
        }
        p.last = clock;
        let cur = arrange::step_at(clock, p.start, m.grid);
        let ended = len == 0 || (!m.looped && cur >= len);
        p.step = (cur >= 0 && !ended).then(|| cur.rem_euclid(len.max(1)) as usize);
        if !ended && p.waiting.is_none_or(|b| clock >= b) {
            let end = if m.looped { p.next + len } else { len };
            let found = (p.next..end).find(|&k| m.steps[k.rem_euclid(len) as usize].is_some());
            match found {
                Some(k) => {
                    let beat = arrange::beat_of(k, p.start, m.grid);
                    if beat - clock <= AHEAD {
                        let step = m.steps[k.rem_euclid(len) as usize].unwrap_or(arrange::Step::new(0));
                        let late = beat < clock - 1.0 / f64::from(m.grid.per_beat());
                        let current = rt.note.as_ref().and_then(|n| RenderKey::of(n, rate)) == rt.cache.key;
                        if let Some(sample) = rt.cache.get(step.note).filter(|_| current && !late) {
                            let until = arrange::beat_of(k + i64::from(step.len), p.start, m.grid);
                            send = Some(Some((sample, beat, until)));
                            p.waiting = Some(beat);
                            #[cfg(test)]
                            rt.sent.push((k.rem_euclid(len) as usize, step.note));
                        }
                        p.next = k + 1;
                    }
                }
                None if !m.looped => p.next = len,
                None => {}
            }
        }
        rt.play = (!ended).then_some(p);
        (before != p.step, ended)
    };
    match send {
        Some(Some((sample, beat, until))) => {
            let cue = to_cue(app);
            engine::send(app, Command::Sampler(SamplerCommand::AuditionAt { sample, cue, beat, until }));
        }
        Some(None) => engine::send(app, Command::Sampler(SamplerCommand::StopAudition)),
        None => {}
    }
    if shown || ended {
        show_arrange(app);
    }
}

/// The arrange strip (tests).
#[cfg(test)]
pub(crate) fn melody(app: &App) -> Melody {
    app.create.borrow().melody.clone()
}

/// Whether `target` (MIDI) is rendered for the current note (tests).
#[cfg(test)]
pub(crate) fn rendered(app: &App, target: i32) -> bool {
    app.create.borrow().cache.get(target).is_some()
}

/// The notes PLAY sent, as (step, MIDI), since the last call (tests).
#[cfg(test)]
pub(crate) fn take_sent(app: &App) -> Vec<(usize, i32)> {
    std::mem::take(&mut app.create.borrow_mut().sent)
}

/// A picked note for the audits, in its widest state: a correction
/// (SET BY YOU) over a region where nothing clear was found.
#[cfg(test)]
pub(crate) fn stage(app: &App, track: TrackId) {
    let (title, path, duration) = {
        let st = app.state.borrow();
        let t = st.lib.track(track).expect("a track of the fixture");
        (format!("{} · {}", t.tags.title, t.tags.artist), t.path.clone(), t.duration)
    };
    let note = PickedNote {
        source: "player".into(),
        track: Some(track),
        title,
        path,
        duration,
        start: 1.05,
        end: 1.45,
        view: 1.25 - SPAN / 2.0,
        detected: None,
        confidence: 0.1,
        why: "none".into(),
        set: Some(NotePitch::new(42, 12)),
    };
    app.create.borrow_mut().note = Some(note);
    render_wave(app);
}

/// [72] The staged note with the pads an octave up: every pad but none
/// within 7 semitones, so all of them show FAR.
#[cfg(test)]
pub(crate) fn stage_far(app: &App, track: TrackId) {
    stage(app, track);
    {
        let mut rt = app.create.borrow_mut();
        rt.octave = 1;
        rt.choice = KeyChoice::default();
    }
    show_key(app);
    show_pads(app);
}

/// [73] The staged note in a key set by hand, in its widest state: a mode
/// (the modes shown under MORE), with the hints, the pads an octave down.
#[cfg(test)]
pub(crate) fn stage_scale(app: &App, track: TrackId) {
    stage(app, track);
    {
        let mut rt = app.create.borrow_mut();
        rt.octave = -1;
        rt.choice = KeyChoice { key: Some(ScaleKey::new(6, Scale::HarmonicMinor)), from: "set".into(), ..KeyChoice::default() };
    }
    app.ui.global::<crate::Create>().set_more(true);
    show_key(app);
    show_pads(app);
}

/// [74] The staged note and key (MORE folded) with a full strip, in its
/// widest state:
/// 32 notes on 1/16 spelled with sharps, a step selected, REC armed step by
/// step (its cursor shown), UNDO ready and a typing error reported.
#[cfg(test)]
pub(crate) fn stage_melody(app: &App, track: TrackId) {
    stage_scale(app, track);
    {
        let mut rt = app.create.borrow_mut();
        let key = ScaleKey::new(6, Scale::HarmonicMinor);
        let notes = key.notes_from(66, 8);
        let mut m = Melody { grid: Grid::Sixteenth, looped: true, ..Melody::default() };
        for i in 0..STEPS {
            m.set(i, notes[(i * 3) % notes.len()]);
        }
        rt.undo = Some(Melody::default());
        rt.melody = m;
        rt.selected = Some(5);
        rt.rec = Some(Rec { origin: None, cursor: 6 });
        rt.typed_bad = vec!["H4".into(), "X#9".into()];
    }
    // The modes folded: the strip is what is shown here.
    app.ui.global::<crate::Create>().set_more(false);
    show_arrange(app);
}

/// Documentation screenshots (`OPENDJDECKS_DEMO_PROOF`, comma separated),
/// on the demo library with the decks loaded by `OPENDJDECKS_AUTOLOAD`:
/// `create-note` opens MELODY and picks a note from deck 1;
/// `create-note-set` corrects it a semitone up as well (SET BY YOU);
/// `create-note-far` puts the pads an octave up (FAR on every pad);
/// `create-scale-set` chooses a mode by hand (MORE open, SET BY YOU);
/// `create-scale-off` turns SNAP TO SCALE off;
/// `create-melody` types a melody into the strip and selects a step;
/// `create-melody-rec` arms REC as well; `create-melody-play` plays the
/// strip in a loop.
pub(crate) fn demo(app: &std::rc::Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let parts: Vec<String> = v.split(',').map(str::to_string).collect();
    if !parts.iter().any(|p| p.starts_with("create-")) {
        return;
    }
    let weak = std::rc::Rc::downgrade(app);
    slint::Timer::single_shot(Duration::from_secs(12), move || {
        let Some(a) = weak.upgrade() else { return };
        a.ui.invoke_set_mode("create".into());
        set_view(&a, "melody");
        pick(&a, "deck1");
        if parts.iter().any(|p| p == "create-note-set") {
            correct(&a, |p| p.transpose(1));
        }
        if parts.iter().any(|p| p == "create-note-far") {
            step_octave(&a, 1);
        }
        if parts.iter().any(|p| p == "create-scale-set") {
            a.ui.global::<crate::Create>().set_more(true);
            choose_scale(&a, "dorian");
        }
        if parts.iter().any(|p| p == "create-scale-off") {
            set_snap(&a, false);
        }
        if parts.iter().any(|p| p == "create-melody") {
            type_steps(&a, "A3 C4 E4 D4 - C4 _ B3 A3 C4 E4 G4 _ _ F4 E4");
            select_step(&a, 3);
        }
        if parts.iter().any(|p| p == "create-melody-rec") {
            toggle_rec(&a);
        }
        if parts.iter().any(|p| p == "create-melody-play") {
            set_loop(&a, true);
            toggle_play(&a);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_row_reads_play_state_tempo_bar_and_level() {
        let empty = strip_row(0, &DeckState::default(), 0.7, String::new());
        assert!(!empty.loaded && !empty.playing);
        assert_eq!(empty.bpm, "");
        assert_eq!((empty.progress, empty.bar, empty.level), (0.0, 0.0, 0.0));

        let s = DeckState {
            track: Some(od_core::TrackId(7)),
            duration: 200.0,
            position: 50.0,
            playing: true,
            bpm: 124.04,
            beat: 33.5,
            beat_in_bar: 3,
            ..Default::default()
        };
        let r = strip_row(1, &s, 1.4, "8A".into());
        assert_eq!(r.deck, 2);
        assert!(r.loaded && r.playing);
        assert_eq!(r.bpm, "124.0");
        assert_eq!(r.key, "8A");
        assert!((r.progress - 0.25).abs() < 1e-6);
        // Beat 3 of the bar, half way through it: 2.5 of 4 beats.
        assert!((r.bar - 0.625).abs() < 1e-6, "{}", r.bar);
        assert_eq!(r.level, 1.0, "clamped");
    }

    fn picked() -> PickedNote {
        PickedNote { track: Some(TrackId(1)), duration: 60.0, start: 10.0, end: 10.5, view: 8.25, ..Default::default() }
    }

    #[test]
    fn handles_stay_in_the_view_and_apart() {
        let mut n = picked();
        n.move_handle(false, 10.01);
        assert!((n.end - (10.0 + MIN_LEN)).abs() < 1e-9, "OUT keeps the shortest note");
        n.move_handle(true, 11.0);
        assert!((n.start - (n.end - MIN_LEN)).abs() < 1e-9, "IN stops before OUT");
        n.move_handle(true, 2.0);
        assert_eq!(n.start, 8.25, "IN stays in the view");
        n.move_handle(false, 20.0);
        assert_eq!(n.end, 8.25 + SPAN, "OUT too");
        // At the very start of a track the view starts before it; IN stops at 0.
        let mut n = PickedNote { start: 0.1, end: 0.6, view: -1.65, duration: 60.0, ..picked() };
        n.move_handle(true, -1.0);
        assert_eq!(n.start, 0.0);
        // Let go near the edge: the view centres on the note.
        let mut n = picked();
        n.move_handle(false, 8.25 + SPAN * 0.95);
        n.recentre();
        assert!(((n.start + n.end) / 2.0 - (n.view + SPAN / 2.0)).abs() < 1e-9);
        let mut n = picked();
        n.recentre();
        assert_eq!(n.view, 8.25, "a note in the middle keeps its view");
    }

    #[test]
    fn badge_follows_detection_and_correction() {
        let mut n = picked();
        assert_eq!(n.badge(), "unsure", "nothing measured");
        n.measured(&pitch::PitchEstimate { outcome: PitchOutcome::Pitch, hz: Some(220.0 * 2f64.powf(0.12 / 12.0)), confidence: 0.9 });
        assert_eq!((n.badge(), n.pitch().unwrap().to_string().as_str()), ("detected", "A3 +12 ct"));
        n.measured(&pitch::PitchEstimate { outcome: PitchOutcome::Pitch, hz: Some(220.0), confidence: 0.3 });
        assert_eq!(n.badge(), "unsure", "a pitch it does not stand by");
        n.measured(&pitch::PitchEstimate { outcome: PitchOutcome::TooShort, hz: None, confidence: 0.0 });
        assert_eq!((n.badge(), n.why.as_str(), n.pitch()), ("unsure", "short", None));
        n.set = Some(NotePitch::new(42, 0));
        assert_eq!((n.badge(), n.pitch().unwrap().name().as_str()), ("set", "F#2"));
        let v = note_view(&n, true);
        assert_eq!((v.note.as_str(), v.cents.as_str(), v.state.as_str()), ("F#2", "0 ct", "set"), "a correction is never 'measuring'");
        assert_eq!(v.length, "0.50 s");
        // Stored and read back as it was.
        let back: PickedNote = serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn a_quiet_view_is_raised_up_to_four_times() {
        let col = |p: u8| od_core::WaveformColumn { low: p, mid: p / 2, high: 0, peak: p };
        let w = od_core::Waveform { columns_per_second: 10.0, columns: [vec![col(200); 10], vec![col(40); 10], vec![col(10); 10]].concat() };
        assert_eq!(louder(&w, 1.0, 1.0).columns[10].peak, 160, "four times at most");
        assert_eq!(louder(&w, 0.0, 1.0).columns[0].peak, 255, "the loudest fills the well");
        assert_eq!(louder(&w, 0.0, 1.0).columns[10].peak, 51, "the rest by the same gain");
    }

    #[test]
    fn fades_guard_in_and_close_out() {
        let rate = 1000;
        let mut x = vec![1.0f32; 2 * 400];
        let n = PickedNote { start: 0.0, end: 0.4, ..Default::default() };
        assert!((n.fade() - 0.06).abs() < 1e-9, "a quarter of the note, at most 60 ms");
        apply_fades(&mut x, rate, n.fade());
        assert_eq!(x[0], 0.0, "starts from silence");
        assert_eq!(x[2 * 3 + 1], 1.0, "full after the click guard");
        assert_eq!(x[2 * 300], 1.0, "full until the fade");
        assert_eq!(x[2 * 399], 0.0, "ends in silence");
        assert!(x[2 * 370] > 0.0 && x[2 * 370] < 1.0, "on the way down");
        let mut steps = x.chunks_exact(2).skip(340).map(|f| f[0]);
        let mut prev = steps.next().unwrap();
        for v in steps {
            assert!(v <= prev + 1e-6, "falls steadily");
            prev = v;
        }
    }

    #[test]
    fn pads_run_up_from_the_source_and_warn_beyond_seven() {
        let src = NotePitch::new(57, 12);
        assert_eq!(pad_targets(src, 0, None), [57, 58, 59, 60, 61, 62, 63, 64]);
        assert_eq!(pad_targets(src, -1, None)[0], 45);
        assert!((shift_to(src, 57) + 0.12).abs() < 1e-9, "the source pad corrects the cents");
        assert!((shift_to(src, 69) - 11.88).abs() < 1e-9);
        let cache = NoteCache::default();
        let rows = pad_rows(Some(src), 0, None, None, &cache, Some(2));
        assert_eq!((rows[0].note.as_str(), rows[0].shift, rows[7].note.as_str(), rows[7].shift), ("A3", 0, "E4", 7));
        assert!(rows.iter().all(|r| r.enabled && !r.far && !r.ready), "within 7 semitones");
        assert!(rows[2].playing && !rows[1].playing);
        let up = pad_rows(Some(src), 1, None, None, &cache, None);
        assert!(up.iter().all(|r| r.far), "an octave up: +12 … +19");
        let down = pad_rows(Some(src), -1, None, None, &cache, None);
        let far: Vec<bool> = down.iter().map(|r| r.far).collect();
        assert_eq!(far, [true, true, true, true, true, false, false, false], "-12 … -8 warn, -7 … -5 do not");
        assert!(pad_rows(None, 0, None, None, &cache, None).iter().all(|r| !r.enabled), "no pitch: off");
    }

    #[test]
    fn snapped_pads_run_up_the_scale_from_the_root_at_or_below_the_note() {
        let am = ScaleKey::new(9, Scale::Minor);
        // From A3 itself: A3 to A4.
        assert_eq!(pad_targets(NotePitch::new(57, 0), 0, Some(am)), [57, 59, 60, 62, 64, 65, 67, 69]);
        // From F#3 (out of the scale): the A below, A2 to A3.
        let fs = NotePitch::new(54, 30);
        assert_eq!(pad_targets(fs, 0, Some(am)), [45, 47, 48, 50, 52, 53, 55, 57]);
        assert_eq!(pad_targets(fs, 1, Some(am))[0], 57, "the octave switch moves them");
        // Every pad in the scale, and FAR where the shift is over 7.
        let rows = pad_rows(Some(fs), 0, Some(am), Some(am), &NoteCache::default(), None);
        let names: Vec<&str> = rows.iter().map(|r| r.note.as_str()).collect();
        assert_eq!(names, ["A2", "B2", "C3", "D3", "E3", "F3", "G3", "A3"]);
        let far: Vec<bool> = rows.iter().map(|r| r.far).collect();
        assert_eq!(far, [true, false, false, false, false, false, false, false], "A2 -9 warns, B2 -7 does not");
        // A major scale from its root: C to C.
        let c = ScaleKey::new(0, Scale::Major);
        assert_eq!(pad_targets(NotePitch::new(60, 0), 0, Some(c)), [60, 62, 64, 65, 67, 69, 71, 72]);
        // SNAP TO SCALE off: the chromatic pads of [72].
        let off = KeyChoice { key: Some(am), snap: false, ..KeyChoice::default() };
        assert_eq!(pad_targets(fs, 0, off.pad_key()), [54, 55, 56, 57, 58, 59, 60, 61]);
    }

    #[test]
    fn the_strip_covers_the_pads_and_marks_root_scale_and_out() {
        let am = ScaleKey::new(9, Scale::Minor);
        let pads = pad_targets(NotePitch::new(54, 0), 0, Some(am));
        let start = strip_start(Some(&pads));
        assert_eq!(start, 36, "C2: the C at or below A2");
        assert!(pads.iter().all(|&p| (start..start + STRIP_KEYS as i32).contains(&p)), "every pad on the strip");
        assert_eq!(strip_start(None), 48, "C3 without a pitch");
        let rows = strip_rows(start, Some(am), &pads, &[57]);
        assert_eq!(rows.len(), STRIP_KEYS);
        assert_eq!((rows[9].name.as_str(), rows[9].kind.as_str(), rows[9].pad), ("A2", "root", true));
        assert_eq!((rows[11].kind.as_str(), rows[10].kind.as_str()), ("scale", "out"), "B in, Bb out");
        assert!(!rows[0].pad && rows[0].kind == "scale", "C2 is in A minor, not on a pad");
        assert!(rows[21].playing && !rows[9].playing, "A3 plays");
        let kinds = |k: &str| rows.iter().filter(|r| r.kind == k).count();
        assert_eq!((kinds("root"), kinds("scale"), kinds("out")), (2, 12, 10));
        assert!(strip_rows(start, None, &[], &[]).iter().all(|r| r.kind.is_empty()), "no key: quiet");
        // Spelled as the key: E major writes its black keys as sharps.
        let e = ScaleKey::new(4, Scale::Major);
        let rows = strip_rows(48, Some(e), &[], &[]);
        assert_eq!((rows[1].name.as_str(), rows[3].name.as_str(), rows[10].name.as_str()), ("C#3", "D#3", "A#3"));
        assert_eq!(strip_rows(48, None, &[], &[])[1].name, "Db3", "no key: the app's spelling");
        let pads = pad_rows(Some(NotePitch::new(64, 0)), 0, Some(e), Some(e), &NoteCache::default(), None);
        let names: Vec<&str> = pads.iter().map(|r| r.note.as_str()).collect();
        assert_eq!(names, ["E4", "F#4", "G#4", "A4", "B4", "C#5", "D#5", "E5"]);
    }

    #[test]
    fn the_default_key_is_the_other_decks() {
        let k = |s: &str| MusicalKey::parse(s);
        let keys = [k("8A"), k("5B"), None, k("1A")];
        let none = [false; 4];
        assert_eq!(key_deck("deck1", &keys, &none, None), Some(1));
        assert_eq!(key_deck("deck2", &keys, &none, None), Some(0));
        // From the player: the deck on air, else one playing, else the first with a key.
        let playing = [false, true, false, true];
        assert_eq!(key_deck("player", &keys, &playing, Some(3)), Some(3));
        assert_eq!(key_deck("player", &keys, &playing, None), Some(1));
        assert_eq!(key_deck("player", &keys, &[false, false, true, false], Some(2)), Some(0), "deck 3 has no key");
        assert_eq!(key_deck("", &[None, None], &[false, false], None), None);
        // A note from deck 1 follows deck 2 even while it has no key.
        assert_eq!(key_deck("deck1", &[k("8A"), None], &none, None), Some(1));
    }

    #[test]
    fn the_key_shows_as_note_names_with_the_wheel_beside() {
        let c = KeyChoice { key: Some(ScaleKey::from_key(MusicalKey::parse("8A").unwrap())), deck: 2, ..KeyChoice::default() };
        let v = key_view(&c, KeyNotation::Camelot);
        assert_eq!((v.root.as_str(), v.scale.as_str(), v.wheel.as_str()), ("A", "minor", "8A"));
        assert_eq!((v.from.as_str(), v.deck), ("deck", 2));
        assert_eq!(v.notes, "A B C D E F G");
        assert_eq!(v.chords, "Am Bdim C Dm Em F G");
        assert_eq!(key_view(&c, KeyNotation::OpenKey).wheel, "1m");
        assert_eq!(key_view(&c, KeyNotation::Musical).wheel, "", "note names already");
        let mode = KeyChoice { key: Some(ScaleKey::new(2, Scale::Dorian)), from: "set".into(), ..KeyChoice::default() };
        assert_eq!(key_view(&mode, KeyNotation::Camelot).wheel, "", "a mode has no place on the wheel");
        assert!(!key_view(&KeyChoice::default(), KeyNotation::Camelot).chosen);
        // Stored and read back; what is missing takes the defaults (snap on).
        let back: KeyChoice = serde_json::from_str(&serde_json::to_string(&mode).unwrap()).unwrap();
        assert_eq!(back, mode);
        let old: KeyChoice = serde_json::from_str("{}").unwrap();
        assert!(old.snap && old.hints && old.from == "deck" && old.key.is_none());
    }

    #[test]
    fn the_cache_keeps_one_buffer_per_target_until_the_note_changes() {
        let key = RenderKey { track: TrackId(1), start: 1.0, end: 1.5, pitch: NotePitch::new(57, 0), rate: 48_000 };
        let mut c = NoteCache::default();
        assert!(c.select(&key), "a first key");
        assert!(!c.select(&key), "the same key keeps what is there");
        assert_eq!(c.missing(&[57, 58]), [57, 58]);
        assert!(c.missing(&[57, 58, 59]) == [59], "asked once");
        let s = Arc::new(SampleData { samples: vec![0.0; 8].into(), sample_rate: 48_000 });
        assert!(c.put(&key, 58, s.clone(), ShiftMethod::Resample));
        let (a, b) = (c.get(58).unwrap(), c.get(58).unwrap());
        assert!(Arc::ptr_eq(&a, &b) && Arc::ptr_eq(&a, &s), "the same buffer every time");
        // A late note of another key is dropped; a new key empties it.
        let other = RenderKey { pitch: NotePitch::new(58, 0), ..key.clone() };
        assert!(!c.put(&other, 60, s.clone(), ShiftMethod::Stretch));
        assert!(c.select(&other));
        assert!(c.get(58).is_none() && c.missing(&[57]) == [57]);
    }

    #[test]
    fn no_grid_no_bar_position() {
        let s = DeckState { track: Some(od_core::TrackId(1)), beat: 3.2, beat_in_bar: 0, ..Default::default() };
        assert_eq!(strip_row(0, &s, 0.0, String::new()).bar, 0.0);
    }
}
