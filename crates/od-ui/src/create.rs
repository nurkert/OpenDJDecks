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

use crate::app::App;
use crate::engine;
use crossbeam_channel::{Receiver, Sender, unbounded};
use od_analysis::pitch::{self, PitchOutcome};
use od_core::{NotePitch, TrackId};
use od_engine::{Command, DeckState, EngineState, SampleData, SamplerCommand, TrackData};
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::path::PathBuf;
use std::sync::Arc;
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
    /// HEAR plays, since when.
    hearing: Option<Instant>,
    /// The zoomed waveform as drawn, for the view it shows.
    wave: slint::Image,
}

impl Default for Runtime {
    fn default() -> Self {
        let (tx, rx) = unbounded();
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
    app.create.borrow_mut().note = note;
    render_wave(app);
    show(app);
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
}

/// HEAR: plays IN to OUT with its fades on the sampler's audition voice, to
/// the headphones while a deck plays, else to the master; again stops it.
pub(crate) fn hear(app: &App) {
    if app.create.borrow().hearing.is_some() {
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
    // A deck playing on air: the note goes to the headphones only.
    let cue = engine::snapshot(app).is_some_and(|s| {
        let count = app.decks.borrow().deck_count.min(s.decks.len());
        s.decks[..count].iter().any(|d| d.playing)
    });
    engine::send(app, Command::Sampler(SamplerCommand::Audition { sample, cue }));
    app.create.borrow_mut().hearing = Some(Instant::now());
    show(app);
}

fn stop_hearing(app: &App) {
    let was = app.create.borrow_mut().hearing.take().is_some();
    if was {
        engine::send(app, Command::Sampler(SamplerCommand::StopAudition));
        show(app);
    }
}

/// Every frame: the decoded audio comes in, HEAR ends with its note.
pub(crate) fn poll(app: &App, state: Option<&EngineState>) {
    let got: Vec<(TrackId, Option<Audio>)> = app.create.borrow().rx.try_iter().collect();
    for (id, a) in got {
        let (measure_due, hear_due) = {
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
                }
                _ => {}
            }
            (rt.measure_due && current, std::mem::take(&mut rt.hear_due) && current)
        };
        if measure_due {
            measure(app);
        } else {
            show(app);
        }
        if hear_due {
            hear(app);
        }
    }
    let ended = {
        let rt = app.create.borrow();
        rt.hearing.is_some_and(|t| t.elapsed() > Duration::from_millis(150) && !state.is_some_and(|s| s.auditioning))
    };
    if ended {
        app.create.borrow_mut().hearing = None;
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
            v.hearing = rt.hearing.is_some();
            v
        })
    };
    let g = app.ui.global::<crate::Create>();
    g.set_note(view.unwrap_or_default());
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

/// Documentation screenshots (`OPENDJDECKS_DEMO_PROOF`, comma separated),
/// on the demo library with the decks loaded by `OPENDJDECKS_AUTOLOAD`:
/// `create-note` opens MELODY and picks a note from deck 1;
/// `create-note-set` corrects it a semitone up as well (SET BY YOU).
pub(crate) fn demo(app: &std::rc::Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let parts: Vec<String> = v.split(',').map(str::to_string).collect();
    if !parts.iter().any(|p| p.starts_with("create-note")) {
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
    fn no_grid_no_bar_position() {
        let s = DeckState { track: Some(od_core::TrackId(1)), beat: 3.2, beat_in_bar: 0, ..Default::default() };
        assert_eq!(strip_row(0, &s, 0.0, String::new()).bar, 0.0);
    }
}
