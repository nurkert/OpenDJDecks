//! Decks: loading tracks (decode and stems off the UI thread), stem
//! separation, the deck view models and the deck controls.
//!
//! The deck views are persistent models: every frame the engine state is
//! projected into plain [`DeckFields`]; a row is written only when its
//! fields or its per-track models changed. The zoomed waveforms scroll
//! smoothly: tiles one lane wide are rendered once and the UI translates
//! them, extrapolating the playhead at the display rate from the position
//! and speed published here (see [`LaneScroll`]).

use crate::app::{App, handler};
use crate::models::{Rows, model};
use crate::player::{cue_marks, phrase_marks};
use crate::{
    CueMark, DeckView, LaneMotion, PadView, PhraseMark, engine, format, pads, player, render,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use od_controller::PadMode;
use od_core::{Beatgrid, CueKind, TrackId, Waveform};
use od_engine::{Command, DeckCommand, DeckState, EngineState, TempoRange};
use slint::{ComponentHandle, Image, Model, ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub(crate) const DECKS: usize = 4;
/// The hidden deck the Prepare preview plays on.
pub(crate) const PREVIEW_DECK: usize = 3;
/// Seconds of audio shown across the scrolling waveform.
const DETAIL_SPAN: f64 = 8.0;
/// Loudness target for auto-gain.
const TARGET_LUFS: f32 = -10.0;

/// View data of a loaded track.
#[derive(Clone)]
pub(crate) struct Loaded {
    pub id: TrackId,
    /// Cover art thumbnail.
    artwork: Option<Image>,
    pub title: String,
    pub artist: String,
    key: String,
    /// The track's key for key sync.
    musical_key: Option<od_core::MusicalKey>,
    waveform: Waveform,
    pub grid: Beatgrid,
    overview: Image,
    cues: Vec<CueMark>,
    /// Hot cue labels by library index (bank * 8 + slot).
    labels: Vec<String>,
    phrases: Vec<od_core::Phrase>,
    phrase_marks: Vec<PhraseMark>,
    vocals: Vec<crate::VocalMark>,
    /// What a reload with stems needs.
    source: TrackSource,
    /// Your correction of the auto gain (dB, stored per track).
    pub gain_adjust: f32,
    /// How sure key and grid analysis were ([`crate::key_mix`]).
    confidence: Option<od_core::AnalysisConfidence>,
}

/// Perform layouts in the order of the top bar's layout button.
pub(crate) const PERFORM_LAYOUTS: [&str; 3] = ["standard", "vertical", "browse"];

const META_PERFORM_LAYOUT: &str = "ui.perform-layout";

/// Sets and remembers the Perform layout.
pub(crate) fn set_perform_layout(app: &App, layout: &str) {
    let layout = if PERFORM_LAYOUTS.contains(&layout) {
        layout
    } else {
        "standard"
    };
    app.ui
        .global::<crate::PerformLayout>()
        .set_layout(layout.into());
    app.ui.set_upper_height(0.0);
    let _ = app
        .state
        .borrow_mut()
        .lib
        .set_meta(META_PERFORM_LAYOUT, layout);
}

const META_DECK_COUNT: &str = "ui.deck-count";

/// A Perform view from the top-bar menu: "2h", "2v", "4h", "4v" (decks
/// shown, horizontal or vertical lanes) or "browse" (decks as they are).
pub(crate) fn set_perform_view(app: &App, view: &str) {
    if view == "browse" {
        set_perform_layout(app, "browse");
        return;
    }
    let count = if view.starts_with('4') { 4 } else { 2 };
    set_perform_layout(app, if view.ends_with('v') { "vertical" } else { "standard" });
    set_deck_count(app, count);
}

/// Shows 2 or 4 decks and remembers it.
pub(crate) fn set_deck_count(app: &App, count: usize) {
    let count = count.clamp(2, DECKS);
    app.decks.borrow_mut().deck_count = count;
    show_empty(app);
    let _ = app
        .state
        .borrow_mut()
        .lib
        .set_meta(META_DECK_COUNT, &count.to_string());
}

/// Next Perform layout (standard → vertical → browse → standard).
pub(crate) fn cycle_perform_layout(app: &App) {
    let cur = app.ui.global::<crate::PerformLayout>().get_layout();
    let i = PERFORM_LAYOUTS
        .iter()
        .position(|l| *l == cur.as_str())
        .unwrap_or(0);
    set_perform_layout(app, PERFORM_LAYOUTS[(i + 1) % PERFORM_LAYOUTS.len()]);
}

/// Perform layout: vertical waveforms on / off.
pub(crate) fn toggle_vertical_layout(app: &App) {
    let cur = app.ui.global::<crate::PerformLayout>().get_layout();
    set_perform_layout(
        app,
        if cur == "vertical" {
            "standard"
        } else {
            "vertical"
        },
    );
}

/// Perform layout: standard ↔ Browse (decks reduced, the browser gets the room).
pub(crate) fn toggle_browse_layout(app: &App) {
    let cur = app.ui.global::<crate::PerformLayout>().get_layout();
    set_perform_layout(
        app,
        if cur == "browse" {
            "standard"
        } else {
            "browse"
        },
    );
}

/// Vocal spans as fractions of the track.
pub(crate) fn vocal_marks(spans: &[(f64, f64)], duration: f64) -> Vec<crate::VocalMark> {
    let d = duration.max(1e-6);
    spans
        .iter()
        .map(|(a, b)| crate::VocalMark {
            start: (a / d) as f32,
            end: (b / d).min(1.0) as f32,
        })
        .collect()
}

/// The track's active loop (a saved loop marked active), if any.
pub(crate) fn active_loop(cues: &[od_core::Cue]) -> Option<(f64, f64)> {
    cues.iter()
        .filter(|c| c.active)
        .find_map(|c| c.loop_end.map(|e| (c.start, e)))
}

/// "SPLIT 42%" while parts are separated (fraction 0..1), "SPLIT…" at the start.
pub(crate) fn split_text(progress: Option<f32>) -> String {
    match progress {
        Some(p) if p > 0.005 => format!("SPLIT {:.0}%", (p * 100.0).clamp(0.0, 99.0)),
        Some(_) => "SPLIT…".into(),
        None => String::new(),
    }
}

/// The stem cache a stick carries (`/OPENDJDECKS/stems`).
pub(crate) fn stick_stem_cache(root: &std::path::Path) -> od_stems::StemCache {
    od_stems::StemCache::new(root.join("OPENDJDECKS").join("stems"))
}

impl Loaded {
    /// Memory cue starts in seconds, in track order.
    pub(crate) fn memory_starts(&self, duration: f64) -> Vec<f64> {
        let mut v: Vec<f64> = self
            .cues
            .iter()
            .filter(|c| c.slot < 0)
            .map(|c| c.pos as f64 * duration)
            .collect();
        v.sort_by(f64::total_cmp);
        v
    }
}

/// File and analysis of a loaded track, kept to reload it with stems.
#[derive(Clone)]
struct TrackSource {
    path: PathBuf,
    hash: od_core::ContentHash,
    cues: Vec<od_core::Cue>,
    gain: f32,
}

struct Prepared {
    deck: usize,
    data: Arc<od_engine::TrackData>,
    /// Vocal spans (seconds) when the data carries parts.
    vocals: Vec<(f64, f64)>,
    play_from: Option<f64>,
    /// Replaces the data of the track already on `deck` without a gap.
    swap: Option<Swap>,
    /// Cached parts follow in a second message (no separation needed).
    stems_follow: bool,
    /// The load of this track failed (the deck stops showing it loading).
    failed: Option<TrackId>,
}

impl Prepared {
    fn failed(deck: usize, track: TrackId) -> Self {
        Self {
            deck,
            data: od_engine::TrackData::new(
                track,
                od_decode::DecodedAudio::default(),
                48_000,
                Beatgrid::default(),
                &[],
                0.0,
            ),
            vocals: Vec::new(),
            play_from: None,
            swap: None,
            stems_follow: false,
            failed: Some(track),
        }
    }
}

/// Why a loaded track's data is swapped in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Swap {
    /// Stems were separated.
    Stems,
    /// The cached parts arrived right after a load.
    CachedStems,
    /// The cached parts did not fit the audio: separate them again.
    NoStems,
    /// Analysis (beatgrid, loudness) arrived after loading.
    Analysis,
}

pub(crate) struct Decks {
    /// Decks shown in Perform (2 or 4).
    pub deck_count: usize,
    loaded: [Option<Loaded>; DECKS],
    /// View data of tracks being prepared, moved to `loaded` on arrival.
    pending: [Option<Loaded>; DECKS],
    prepared_tx: Sender<Prepared>,
    prepared_rx: Receiver<Prepared>,
    loaded_from_stick: bool,
    /// Auto-loop size per deck, in beats.
    pub loop_size: [f64; DECKS],
    pub shift: [bool; DECKS],
    pub pad_mode: [PadMode; DECKS],
    /// Hot cue the Keyboard pads play, per deck (0..8).
    pub keyboard_cue: [usize; DECKS],
    /// Slicer domain per deck, in beats.
    pub slicer_beats: [f64; DECKS],
    /// TAP BPM per deck (controller).
    taps: [crate::fx::TapTempo; DECKS],
    stem_cache: od_stems::StemCache,
    #[cfg(feature = "separation")]
    stem_worker: Option<od_stems::StemWorker>,
    /// Separation progress (0..1) of queued or running tracks.
    stem_progress: HashMap<TrackId, f32>,
    pub views: DeckViews,
    /// Waveform zoom per deck (1 = 8 s across, smaller = closer).
    pub zoom: [f64; DECKS],
    scroll: [LaneScroll; DECKS],
    motion: Rc<VecModel<LaneMotion>>,
    /// The incoming track drawn over each lane ([`phrase_mix`]).
    overlay_scroll: [LaneScroll; DECKS],
    overlay: Rc<VecModel<LaneMotion>>,
    /// The loop each deck played last (its track, seconds), for CAPTURE.
    pub last_loop: [Option<(TrackId, f64, f64)>; DECKS],
    /// The track data each deck plays (a grid edit swaps in a copy with
    /// the new grid, sharing the audio).
    engine_data: [Option<Arc<od_engine::TrackData>>; DECKS],
    /// A deck being restored after a crash: applied when its track is in.
    pub restore: [Option<crate::restore::DeckSnap>; DECKS],
    /// A running stem solo per deck ([`crate::stem_mix`]).
    pub stem_solo: [Option<crate::stem_mix::Solo>; DECKS],
    /// Stems were in use where this deck's track came in, and it has none
    /// yet: the stem keys warn until they arrive.
    pub stems_expected: [bool; DECKS],
}

impl Decks {
    /// The waveform lanes' motion rows (shared with the booth window).
    pub(crate) fn lane_motion(&self) -> slint::ModelRc<crate::LaneMotion> {
        slint::ModelRc::from(self.motion.clone())
    }

    /// Per lane, the incoming track drawn over it (Perform).
    pub(crate) fn lane_overlay(&self) -> slint::ModelRc<crate::LaneMotion> {
        slint::ModelRc::from(self.overlay.clone())
    }

    /// The decks' own separation of `track`: `Some(progress)` while it runs
    /// (or has made progress), `None` otherwise; see [`Self::stem_waiting`].
    pub(crate) fn stem_running(&self, track: TrackId) -> Option<f32> {
        let p = *self.stem_progress.get(&track)?;
        (p > 0.0 || self.stem_current() == Some(track)).then_some(p)
    }

    /// What the STEMS column shows of the decks' own separations (track,
    /// progress in 5 % steps, 255 = waiting); changes when it should redraw.
    pub(crate) fn stem_signature(&self) -> Vec<(TrackId, u8)> {
        let mut v: Vec<(TrackId, u8)> = self
            .stem_progress
            .keys()
            .map(|&t| (t, self.stem_running(t).map_or(255, |p| (p * 20.0) as u8)))
            .collect();
        v.sort();
        v
    }

    /// `track` waits in the decks' separation queue behind another track.
    pub(crate) fn stem_waiting(&self, track: TrackId) -> bool {
        self.stem_progress.contains_key(&track) && self.stem_running(track).is_none()
    }

    #[cfg(feature = "separation")]
    fn stem_current(&self) -> Option<TrackId> {
        self.stem_worker.as_ref().and_then(|w| w.current())
    }

    #[cfg(not(feature = "separation"))]
    fn stem_current(&self) -> Option<TrackId> {
        None
    }

    /// Parts of the track on `deck` are being separated.
    pub(crate) fn stems_pending(&self, deck: usize) -> bool {
        self.loaded[deck]
            .as_ref()
            .is_some_and(|m| self.stem_progress.contains_key(&m.id))
    }
}

impl Default for Decks {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self {
            deck_count: 2,
            loaded: Default::default(),
            pending: Default::default(),
            prepared_tx: tx,
            prepared_rx: rx,
            loaded_from_stick: false,
            loop_size: [4.0; DECKS],
            shift: [false; DECKS],
            pad_mode: [PadMode::HotCue; DECKS],
            keyboard_cue: [0; DECKS],
            slicer_beats: [8.0; DECKS],
            taps: Default::default(),
            stem_cache: od_stems::StemCache::open_default(),
            #[cfg(feature = "separation")]
            stem_worker: None,
            stem_progress: HashMap::new(),
            engine_data: Default::default(),
            restore: Default::default(),
            views: DeckViews::default(),
            zoom: [1.0; DECKS],
            scroll: Default::default(),
            motion: Rc::new(VecModel::from(vec![LaneMotion::default(); DECKS])),
            overlay_scroll: Default::default(),
            overlay: Rc::new(VecModel::from(vec![LaneMotion::default(); DECKS])),
            last_loop: [None; DECKS],
            stem_solo: [None; DECKS],
            stems_expected: [false; DECKS],
        }
    }
}

/// Smooth scrolling of one zoomed waveform lane: the rendered tiles and the
/// motion last published to the UI.
#[derive(Default)]
struct LaneScroll {
    /// What the tiles were rendered for: track, span (ms), size, phrases,
    /// look, vertical, waveform generation, outline.
    key: Option<(TrackId, i64, u32, u32, bool, bool, bool, u32, bool)>,
    tiles: HashMap<i64, Image>,
    first: Option<i64>,
    sent_pos: f64,
    sent_rate: f64,
    sent_at: Option<Instant>,
    loaded: bool,
}

/// Inputs for one lane's scroll update.
struct ScrollIn<'a> {
    meta: Option<&'a Loaded>,
    position: f64,
    /// Seconds of track per real second (0 when paused or scratching).
    speed: f64,
    span: f64,
    width: u32,
    height: u32,
    phrases: bool,
    /// Tiles turned for the vertical lanes.
    vertical: bool,
    /// Outline tiles of an incoming track ([`render::outline_tile`]).
    outline: bool,
    now: Instant,
}

impl LaneScroll {
    /// Brings the tiles up to date and returns a new motion row when the UI
    /// needs one (new tiles, speed change, drift, or a periodic resync).
    fn update(&mut self, i: &ScrollIn) -> Option<LaneMotion> {
        let Some(m) = i.meta else {
            if !self.loaded {
                return None;
            }
            *self = Self::default();
            return Some(LaneMotion::default());
        };
        let key = (
            m.id,
            (i.span * 1000.0).round() as i64,
            i.width,
            i.height,
            i.phrases,
            crate::appearance::is_light(),
            i.vertical,
            render::waveform_generation(),
            i.outline,
        );
        let mut force = !self.loaded;
        if self.key != Some(key) {
            self.key = Some(key);
            self.tiles.clear();
            self.first = None;
            force = true;
        }
        let rate = i.speed / 1000.0;
        let elapsed_ms = self
            .sent_at
            .map(|t| i.now.duration_since(t).as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        let predicted = self.sent_pos + self.sent_rate * elapsed_ms;
        let drift = i.position - predicted;
        // While playing steadily, glide towards the engine position instead
        // of jumping to it: engine updates arrive in audio-block steps.
        let pos = if rate != 0.0 && self.sent_rate == rate && drift.abs() < 0.05 {
            predicted + drift * 0.15
        } else {
            i.position
        };
        let first = ((pos - i.span / 2.0) / i.span).floor() as i64;
        let tiles_moved = self.first != Some(first);
        if tiles_moved || force {
            self.tiles.retain(|k, _| (first..first + 3).contains(k));
            for k in first..first + 3 {
                self.tiles.entry(k).or_insert_with(|| {
                    if i.outline {
                        return render::outline_tile(&m.waveform, k as f64 * i.span, i.span, i.width, i.height);
                    }
                    let tile = render::detail_tile(
                        &m.waveform,
                        &m.grid,
                        if i.phrases { &m.phrases } else { &[] },
                        k as f64 * i.span,
                        i.span,
                        i.width,
                        i.height,
                    );
                    if i.vertical {
                        render::transpose(&tile)
                    } else {
                        tile
                    }
                });
            }
            self.first = Some(first);
        }
        let resync = elapsed_ms > 500.0;
        if !(force || tiles_moved || rate != self.sent_rate || drift.abs() > 0.004 || resync) {
            return None;
        }
        self.loaded = true;
        self.sent_pos = pos;
        self.sent_rate = rate;
        self.sent_at = Some(i.now);
        let tile = |k: i64| self.tiles.get(&k).cloned().unwrap_or_default();
        Some(LaneMotion {
            loaded: true,
            pos: pos as f32,
            rate: rate as f32,
            span: i.span as f32,
            tile0: tile(first),
            tile1: tile(first + 1),
            tile2: tile(first + 2),
            t0: (first as f64 * i.span) as f32,
            t1: ((first + 1) as f64 * i.span) as f32,
            t2: ((first + 2) as f64 * i.span) as f32,
            of_deck: 0,
            mark: -1.0,
        })
    }
}

impl Decks {
    /// Tracks currently loaded on decks (preview deck excluded), 0-based decks.
    pub fn loaded_decks(&self) -> Vec<(TrackId, usize)> {
        (0..self.deck_count)
            .filter_map(|d| self.loaded[d].as_ref().map(|l| (l.id, d)))
            .collect()
    }

    /// Whether loaded tracks came from a stick (ids then refer to it).
    pub fn source_is_stick(&self) -> bool {
        self.loaded_from_stick
    }

    /// Tracks on the decks.
    pub fn loaded_ids(&self) -> Vec<TrackId> {
        self.loaded.iter().flatten().map(|l| l.id).collect()
    }

    /// Where separated parts are kept.
    pub fn stem_cache(&self) -> od_stems::StemCache {
        self.stem_cache.clone()
    }

    pub fn loaded_id(&self, deck: usize) -> Option<TrackId> {
        self.loaded.get(deck).and_then(|l| l.as_ref()).map(|l| l.id)
    }

    pub fn loaded(&self, deck: usize) -> Option<&Loaded> {
        self.loaded.get(deck).and_then(|l| l.as_ref())
    }

    /// A track is on its way into `deck`.
    pub fn loading_on(&self, deck: usize) -> bool {
        self.pending.get(deck).is_some_and(|p| p.is_some())
    }

    pub fn loaded_mut(&mut self, deck: usize) -> Option<&mut Loaded> {
        self.loaded.get_mut(deck).and_then(|l| l.as_mut())
    }
}

// ── Loading ─────────────────────────────────────────────────────────────

/// Loads `track` (of the current source) into `deck` in the background.
pub(crate) fn load_track(app: &App, deck: usize, track: TrackId, play_from: Option<f64>) {
    if crate::perform_lock::blocks_load(app, deck) {
        app.guard_locked(crate::perform_lock::Blocked::Load(deck));
        return;
    }
    if deck == PREVIEW_DECK {
        crate::player::reset_shift_for(app, track);
    }
    if !engine::ensure_engine(app) {
        engine::show_message(app);
        return;
    }
    let rate = app.engine.borrow().sample_rate();
    let st = app.state.borrow();
    let Ok(t) = st.src().track(track) else { return };
    let analysis = st.src().analysis(track).unwrap_or_default();
    let cues = st.src().cues(track).unwrap_or_default();
    let scale = app.ui.window().scale_factor();
    let gain = analysis
        .loudness_lufs
        .map(|l| (TARGET_LUFS - l).clamp(-12.0, 12.0))
        .unwrap_or(0.0);
    let source = TrackSource {
        path: st.resolve(&t.path),
        hash: t.content_hash.clone(),
        cues: cues.clone(),
        gain,
    };
    let artwork = st
        .src()
        .artwork(track)
        .ok()
        .flatten()
        .and_then(|(data, _)| render::artwork_thumbnail(&data, (72.0 * scale) as u32));
    let meta = Loaded {
        id: track,
        artwork,
        title: t.tags.title.clone(),
        artist: t.tags.artist.clone(),
        key: st.key_text(&t),
        musical_key: t.key,
        overview: render::overview(
            &analysis.waveform,
            &analysis.beatgrid,
            t.duration,
            (900.0 * scale) as u32,
            (30.0 * scale) as u32,
        ),
        waveform: analysis.waveform.clone(),
        grid: analysis.beatgrid.clone(),
        cues: cue_marks(&cues, t.duration, false),
        labels: hot_labels(&cues),
        phrases: analysis.phrases.clone(),
        phrase_marks: phrase_marks(&analysis.phrases, &analysis.beatgrid, t.duration),
        vocals: Vec::new(),
        source: source.clone(),
        gain_adjust: crate::deck_grid::stored_gain(st.src(), track),
        confidence: analysis.confidence,
    };
    let read_only = st.read_only();
    drop(st);
    app.ui
        .set_engine_message(format!("Loading {} …", meta.title).into());
    // Tracks from a stick use the parts separated on the desktop and carried
    // onto the stick (SPEC §10.4), so a Raspberry Pi plays them too.
    let stick_stems = app.state.borrow().stick_root().map(stick_stem_cache);
    // The old track leaves at once and the new one shows as loading.
    engine::deck_cmd(app, deck, DeckCommand::Unload);
    let (cache, tx) = {
        let mut d = app.decks.borrow_mut();
        // Stems in use here or on a playing deck: this track should have them too.
        let replaced = d.engine_data[deck].as_ref().is_some_and(|t| t.has_stems());
        let others = (0..d.deck_count.min(DECKS))
            .filter(|&j| j != deck && j != PREVIEW_DECK)
            .any(|j| d.engine_data[j].as_ref().is_some_and(|t| t.has_stems()));
        d.stems_expected[deck] = deck != PREVIEW_DECK && crate::stem_mix::expected(replaced, others);
        d.stem_solo[deck] = None;
        d.loaded[deck] = None;
        d.engine_data[deck] = None;
        d.pending[deck] = Some(meta);
        d.loaded_from_stick = read_only;
        (
            stick_stems.unwrap_or_else(|| d.stem_cache.clone()),
            d.prepared_tx.clone(),
        )
    };
    prepare_in_background(
        cache,
        tx,
        deck,
        track,
        source,
        rate,
        analysis.beatgrid,
        play_from,
        None,
    );
}

/// Decodes a track (with its cached stems, if any) off the UI thread.
#[allow(clippy::too_many_arguments)]
fn prepare_in_background(
    cache: od_stems::StemCache,
    tx: Sender<Prepared>,
    deck: usize,
    track: TrackId,
    src: TrackSource,
    rate: u32,
    grid: Beatgrid,
    play_from: Option<f64>,
    swap: Option<Swap>,
) {
    std::thread::spawn(move || {
        if swap.is_none() {
            // A fresh load: the mix first (playable at once), the cached
            // parts follow and swap in without a gap.
            let (mix, shape) =
                match od_stems::load_mix(track, &src.path, rate, grid, &src.cues, src.gain) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(error = %e, "track load failed");
                        let _ = tx.send(Prepared::failed(deck, track));
                        return;
                    }
                };
            let stems_follow = cache.exists(&src.hash);
            let mix_for_transients = mix.clone();
            let _ = tx.send(Prepared {
                deck,
                data: mix.clone(),
                vocals: Vec::new(),
                play_from,
                swap: None,
                stems_follow,
                failed: None,
            });
            // Transients for the grid editor, from the samples just decoded.
            crate::transients::from_track_data(&src.hash, &mix_for_transients);
            drop(mix_for_transients);
            if let Some(full) = stems_follow
                .then(|| od_stems::attach_cached_stems(&cache, &mix, &src.hash, shape))
                .flatten()
            {
                let vocals = full
                    .stems
                    .as_ref()
                    .map(|s| s.vocal_regions(rate))
                    .unwrap_or_default();
                let _ = tx.send(Prepared {
                    deck,
                    data: full,
                    vocals,
                    play_from: None,
                    swap: Some(Swap::CachedStems),
                    stems_follow: false,
                    failed: None,
                });
            } else if stems_follow {
                // The cache did not fit after all: separate again.
                let _ = tx.send(Prepared {
                    deck,
                    data: mix,
                    vocals: Vec::new(),
                    play_from: None,
                    swap: Some(Swap::NoStems),
                    stems_follow: false,
                    failed: None,
                });
            }
            return;
        }
        match od_stems::load_track_data(
            &cache,
            track,
            &src.path,
            Some(&src.hash),
            rate,
            grid,
            &src.cues,
            src.gain,
        ) {
            Ok(data) => {
                let vocals = data
                    .stems
                    .as_ref()
                    .map(|s| s.vocal_regions(rate))
                    .unwrap_or_default();
                let _ = tx.send(Prepared {
                    deck,
                    data,
                    vocals,
                    play_from,
                    swap,
                    stems_follow: false,
                    failed: None,
                });
            }
            Err(e) => tracing::warn!(error = %e, "track load failed"),
        }
    });
}

/// Frame tick: prepared tracks → engine.
pub(crate) fn receive_prepared(app: &App) {
    let prepared: Vec<Prepared> = app.decks.borrow().prepared_rx.try_iter().collect();
    for pr in prepared {
        let (deck, play_from) = (pr.deck, pr.play_from);
        if let Some(track) = pr.failed {
            let title = {
                let mut d = app.decks.borrow_mut();
                let gone = d.pending[deck].take_if(|m| m.id == track);
                gone.map(|m| m.title).unwrap_or_default()
            };
            engine::show_message(app);
            app.status(format!("Could not load {title}"));
            continue;
        }
        if pr.swap == Some(Swap::NoStems) {
            if deck != PREVIEW_DECK && app.decks.borrow().loaded_id(deck) == Some(pr.data.id) {
                let source = app.decks.borrow().loaded[deck]
                    .as_ref()
                    .map(|m| (m.id, m.source.clone()));
                if let Some((id, source)) = source {
                    request_stems(app, id, &source);
                }
            }
            continue;
        }
        if let Some(why) = pr.swap {
            // Same track still on the deck: swap without a gap.
            if app.decks.borrow().loaded_id(deck) == Some(pr.data.id) {
                let duration = pr.data.duration();
                {
                    let mut d = app.decks.borrow_mut();
                    if let Some(m) = d.loaded[deck].as_mut() {
                        m.vocals = vocal_marks(&pr.vocals, duration);
                    }
                    // Rebuild the per-track view parts (vocal marks).
                    d.views.lanes[deck].track = None;
                }
                // The grid may have been edited while this build was made.
                let data = {
                    let d = app.decks.borrow();
                    match d.loaded(deck) {
                        Some(m) if m.grid != pr.data.beatgrid => pr.data.with_beatgrid(m.grid.clone()),
                        _ => pr.data,
                    }
                };
                app.decks.borrow_mut().engine_data[deck] = Some(data.clone());
                engine::deck_cmd(app, deck, DeckCommand::SwapTrackData(data));
                if why == Swap::Stems {
                    app.decks.borrow_mut().stems_expected[deck] = false;
                    app.status(format!("Stems ready on deck {}", deck + 1));
                }
            }
            continue;
        }
        let needs_stems = !pr.data.has_stems() && !pr.stems_follow;
        let meta = app.decks.borrow_mut().pending[deck]
            .take()
            .filter(|m| m.id == pr.data.id);
        let Some(mut meta) = meta else { continue };
        meta.vocals = vocal_marks(&pr.vocals, pr.data.duration());
        let title = meta.title.clone();
        let musical_key = meta.musical_key;
        let active_loop = active_loop(&meta.source.cues);
        if needs_stems && deck != PREVIEW_DECK {
            request_stems(app, meta.id, &meta.source);
        }
        let warn = needs_stems && app.decks.borrow().stems_expected[deck];
        if !needs_stems {
            app.decks.borrow_mut().stems_expected[deck] = false;
        }
        let data = if meta.grid != pr.data.beatgrid {
            pr.data.with_beatgrid(meta.grid.clone())
        } else {
            pr.data
        };
        {
            let mut d = app.decks.borrow_mut();
            d.loaded[deck] = Some(meta);
            d.engine_data[deck] = Some(data.clone());
        }
        crate::browser::refresh_tracks(app);
        engine::deck_cmd(app, deck, DeckCommand::Load(data));
        engine::deck_cmd(app, deck, DeckCommand::SetTrackKey(musical_key));
        let gain = app.decks.borrow().loaded(deck).map_or(0.0, |m| m.gain_adjust);
        engine::deck_cmd(app, deck, DeckCommand::SetGainAdjust(gain));
        // An active loop engages when the playhead reaches it.
        engine::deck_cmd(app, deck, DeckCommand::SetActiveLoop(active_loop));
        if let Some(t) = play_from {
            engine::deck_cmd(app, deck, DeckCommand::Seek { seconds: t });
            engine::deck_cmd(app, deck, DeckCommand::Play);
        }
        let on_deck = app.decks.borrow().loaded_id(deck).map(|t| t.0);
        let restore = app.decks.borrow_mut().restore[deck].take_if(|r| Some(r.track) == on_deck);
        if let Some(r) = restore {
            crate::restore::apply_deck(app, deck, &r);
        }
        engine::show_message(app);
        if warn {
            app.status(stems_warning(&title, deck, app.decks.borrow().stems_pending(deck)));
        } else {
            app.status(format!("Loaded {title} on deck {}", deck + 1));
        }
    }
}

/// The status line when a track without stems comes in where stems were in
/// use ([`crate::stem_mix::expected`]).
pub(crate) fn stems_warning(title: &str, deck: usize, separating: bool) -> String {
    if separating {
        od_i18n::trf(
            "No stems yet for {} on deck {}: separating now, the stem keys work when it is done",
            &[&title, &(deck + 1)],
        )
    } else {
        od_i18n::trf(
            "No stems for {} on deck {}: press a stem key to separate it, or prepare the set ahead",
            &[&title, &(deck + 1)],
        )
    }
}

/// Double-click in the library: load into the first deck that is not
/// playing (Perform) or start the preview (Prepare).
pub(crate) fn load_selected_into_free_deck(app: &App) {
    let Some(track) = app.state.borrow().player.current else {
        return;
    };
    if app.state.borrow().config.kiosk {
        app.ui.set_browser_open(false);
    }
    if app.ui.get_mode() == "prepare" {
        player::play_in_player(app);
        return;
    }
    let playing: Vec<bool> = match engine::snapshot(app) {
        Some(s) => s.decks.iter().map(|d| d.playing).collect(),
        None => vec![false; DECKS],
    };
    let deck = {
        let d = app.decks.borrow();
        free_deck(d.deck_count, &playing, |i| d.loaded[i].is_some())
    };
    load_track(app, deck, track, None);
}

/// First deck that is neither playing nor loaded, else the first that is
/// not playing, else deck 1.
pub(crate) fn free_deck(count: usize, playing: &[bool], loaded: impl Fn(usize) -> bool) -> usize {
    let idle = |d: usize| !playing.get(d).copied().unwrap_or(false);
    (0..count)
        .find(|&d| idle(d) && !loaded(d))
        .or_else(|| (0..count).find(|&d| idle(d)))
        .unwrap_or(0)
}

/// LOAD on a deck: the selected track (an Explorer file is added to the
/// collection first).
pub(crate) fn load_selected(app: &App, deck: usize) {
    if crate::explorer::load_selection(app, Some(deck)) {
        return;
    }
    if let Some(t) = app.state.borrow().player.current {
        load_track(app, deck, t, None);
    }
}

/// OPENDJDECKS_AUTOLOAD="1:12,2:13" loads tracks on start (standalone
/// setups and testing); "1:12@64" also plays from second 64.
pub(crate) fn autoload(app: &App) {
    let Some(spec) = od_core::env::var("AUTOLOAD") else {
        return;
    };
    for (d, t, from) in parse_autoload(&spec) {
        load_track(app, d - 1, t, from);
        if app.state.borrow().config.kiosk {
            app.ui.set_browser_open(false);
        }
    }
}

/// "1:12, 2:13@64" → [(1, track 12, None), (2, track 13, Some(64.0))];
/// invalid parts are skipped.
pub(crate) fn parse_autoload(spec: &str) -> Vec<(usize, TrackId, Option<f64>)> {
    spec.split(',')
        .filter_map(|part| part.split_once(':'))
        .filter_map(|(d, t)| {
            let (t, from) = match t.split_once('@') {
                Some((t, f)) => (t, Some(f.trim().parse::<f64>().ok()?)),
                None => (t, None),
            };
            Some((
                d.trim().parse::<usize>().ok()?,
                TrackId(t.trim().parse::<i64>().ok()?),
                from,
            ))
        })
        .filter(|(d, _, _)| (1..=DECKS).contains(d))
        .collect()
}

/// Redraws the overviews of the loaded tracks after a change of waveform colours.
pub(crate) fn redraw_overviews(app: &App) {
    let scale = app.ui.window().scale_factor();
    let st = app.state.borrow();
    let mut d = app.decks.borrow_mut();
    for i in 0..DECKS {
        let Some(m) = d.loaded[i].as_mut() else {
            continue;
        };
        let Ok(track) = st.src().track(m.id) else {
            continue;
        };
        m.overview = render::overview(
            &m.waveform,
            &m.grid,
            track.duration,
            (900.0 * scale) as u32,
            (30.0 * scale) as u32,
        );
        d.views.lanes[i].track = None;
    }
}

/// After analysis: tracks that were loaded before their analysis existed
/// (e.g. just added from the Explorer) get their waveform, grid and phrases,
/// and the engine gets the analysed data (beatgrid for BPM / sync, gain)
/// swapped in without a gap.
pub(crate) fn refresh_analysis(app: &App) {
    let rate = app.engine.borrow().sample_rate();
    let st = app.state.borrow();
    let mut d = app.decks.borrow_mut();
    if d.loaded_from_stick {
        return;
    }
    let scale = app.ui.window().scale_factor();
    let mut swaps = Vec::new();
    for i in 0..DECKS {
        let Some(m) = d.loaded[i]
            .as_mut()
            .filter(|m| m.waveform.columns.is_empty())
        else {
            continue;
        };
        let a = st.lib.analysis(m.id).unwrap_or_default();
        if a.waveform.columns.is_empty() {
            continue;
        }
        let duration = st.lib.track(m.id).map(|t| t.duration).unwrap_or(0.0);
        m.overview = render::overview(
            &a.waveform,
            &a.beatgrid,
            duration,
            (900.0 * scale) as u32,
            (30.0 * scale) as u32,
        );
        m.phrase_marks = phrase_marks(&a.phrases, &a.beatgrid, duration);
        m.waveform = a.waveform;
        m.grid = a.beatgrid;
        m.phrases = a.phrases;
        if let Some(k) = st.lib.track(m.id).ok().and_then(|t| t.key) {
            m.key = k.format(st.settings.key_notation());
            m.musical_key = Some(k);
        }
        m.source.gain = a
            .loudness_lufs
            .map(|l| (TARGET_LUFS - l).clamp(-12.0, 12.0))
            .unwrap_or(0.0);
        swaps.push((i, m.id, m.source.clone(), m.grid.clone(), m.musical_key));
        // Rebuild the per-track view parts on the next frame.
        d.views.lanes[i].track = None;
    }
    let (cache, tx) = (d.stem_cache.clone(), d.prepared_tx.clone());
    drop(d);
    drop(st);
    for (deck, id, src, grid, key) in swaps {
        engine::deck_cmd(app, deck, DeckCommand::SetTrackKey(key));
        prepare_in_background(
            cache.clone(),
            tx.clone(),
            deck,
            id,
            src,
            rate,
            grid,
            None,
            Some(Swap::Analysis),
        );
    }
}

// ── Stems ───────────────────────────────────────────────────────────────

#[cfg(test)]
impl Decks {
    /// A separation job was queued (the worker exists).
    pub(crate) fn stems_requested(&self) -> bool {
        self.stem_worker.is_some()
    }
}

/// Queues stem separation of a loaded track (desktop profile only). Without
/// the model nothing happens: loading a track never asks for a download;
/// a part key does ([`crate::stems_model::request`]).
#[cfg(feature = "separation")]
fn request_stems(app: &App, track: TrackId, src: &TrackSource) {
    if app.state.borrow().config.lite || !crate::stems_model::ready(app) {
        return;
    }
    let mut d = app.decks.borrow_mut();
    if d.stem_progress.contains_key(&track) {
        return;
    }
    let cache = d.stem_cache.clone();
    let worker = d.stem_worker.get_or_insert_with(|| {
        od_stems::StemWorker::spawn(cache, od_stems::StemWorkerOptions::default())
    });
    worker.enqueue(od_stems::StemJob {
        track,
        path: src.path.clone(),
        hash: src.hash.clone(),
    });
    worker.prioritize(track);
    d.stem_progress.insert(track, 0.0);
}

#[cfg(not(feature = "separation"))]
fn request_stems(_app: &App, _track: TrackId, _src: &TrackSource) {}

/// Queues the tracks on `deck` (all decks: `None`) that have no parts yet.
pub(crate) fn request_loaded_stems(app: &App, deck: Option<usize>) {
    let wanted: Vec<(TrackId, TrackSource)> = {
        let d = app.decks.borrow();
        (0..DECKS)
            .filter(|i| deck.is_none_or(|x| x == *i) && *i != PREVIEW_DECK)
            .filter(|i| d.engine_data[*i].as_ref().is_some_and(|t| !t.has_stems()))
            .filter_map(|i| d.loaded[i].as_ref().map(|m| (m.id, m.source.clone())))
            .collect()
    };
    for (id, source) in wanted {
        request_stems(app, id, &source);
    }
}

/// Stops the running and queued separations (the model is about to go).
pub(crate) fn cancel_stem_jobs(app: &App) {
    #[cfg(feature = "separation")]
    if let Some(w) = &app.decks.borrow().stem_worker {
        w.cancel_all();
    }
    #[cfg(not(feature = "separation"))]
    let _ = app;
}

/// Separation of `track` is `fraction` (0..1) done: the decks holding it
/// fill their part keys that far.
pub(crate) fn set_stem_progress(app: &App, track: TrackId, fraction: f32) {
    app.decks.borrow_mut().stem_progress.insert(track, fraction);
}

/// Separation events → progress, status and in-place stem swaps.
#[cfg(feature = "separation")]
pub(crate) fn poll_stems(app: &App) {
    use od_stems::StemEvent;
    let events: Vec<StemEvent> = app
        .decks
        .borrow()
        .stem_worker
        .as_ref()
        .map(|w| w.events().try_iter().collect())
        .unwrap_or_default();
    for ev in events {
        match ev {
            // The UI worker never downloads; stems_model.rs does, by request.
            StemEvent::ModelDownload { .. } => {}
            StemEvent::Started { track } => {
                app.decks.borrow_mut().stem_progress.insert(track, 0.0);
            }
            StemEvent::Progress { track, fraction } => set_stem_progress(app, track, fraction),
            StemEvent::Done { track, hash, .. } | StemEvent::AlreadyCached { track, hash } => {
                crate::stems_batch::mark_cached(&hash);
                stems_ready(app, track)
            }
            StemEvent::Failed { track, error } => {
                app.decks.borrow_mut().stem_progress.remove(&track);
                app.status(format!("Stem separation failed: {error}"));
            }
            StemEvent::ModelDamaged { track } => {
                app.decks.borrow_mut().stem_progress.remove(&track);
                crate::stems_model::damaged(app);
            }
            StemEvent::Cancelled { track } => {
                app.decks.borrow_mut().stem_progress.remove(&track);
            }
        }
    }
}

/// The parts of `track` are in the cache: decks holding it swap them in.
#[cfg(feature = "separation")]
pub(crate) fn stems_ready(app: &App, track: TrackId) {
    let rate = app.engine.borrow().sample_rate();
    let (tx, cache, decks) = {
        let mut d = app.decks.borrow_mut();
        d.stem_progress.remove(&track);
        let decks: Vec<(usize, TrackSource, Beatgrid)> = (0..DECKS)
            .filter_map(|i| {
                d.loaded[i]
                    .as_ref()
                    .filter(|l| l.id == track)
                    .map(|l| (i, l.source.clone(), l.grid.clone()))
            })
            .collect();
        (d.prepared_tx.clone(), d.stem_cache.clone(), decks)
    };
    for (deck, src, grid) in decks {
        prepare_in_background(
            cache.clone(),
            tx.clone(),
            deck,
            track,
            src,
            rate,
            grid,
            None,
            Some(Swap::Stems),
        );
    }
}

#[cfg(not(feature = "separation"))]
pub(crate) fn poll_stems(_app: &App) {}

// ── View model ──────────────────────────────────────────────────────────

/// The scalar part of a deck view; compared frame to frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DeckFields {
    pub deck: i32,
    pub loaded: bool,
    pub loading: bool,
    pub title: SharedString,
    pub artist: SharedString,
    pub bpm: SharedString,
    pub key: SharedString,
    pub tempo: SharedString,
    pub elapsed: SharedString,
    pub remaining: SharedString,
    /// Remaining time at the current tempo (Preferences › View › deck time).
    pub remaining_real: SharedString,
    pub progress: f32,
    /// Track length in seconds (cue markers on the lanes).
    pub length: f32,
    pub playing: bool,
    pub master: bool,
    pub sync: bool,
    pub keylock: bool,
    pub looping: bool,
    pub loop_in: f32,
    pub loop_out: f32,
    pub counter_mode: SharedString,
    pub loop_beats: SharedString,
    pub slip: bool,
    pub level: f32,
    pub fader: f32,
    pub tempo_value: f32,
    pub tempo_range: SharedString,
    pub quantize: bool,
    pub stems_available: bool,
    pub stems_busy: bool,
    pub split_text: SharedString,
    /// Separation progress 0..1 while `stems_busy`.
    pub split_progress: f32,
    pub mute_vocal: bool,
    pub mute_inst: bool,
    pub mute_drums: bool,
    pub mute_bass: bool,
    pub level_vocal: f32,
    pub level_inst: f32,
    pub level_drums: f32,
    pub level_bass: f32,
    pub loop_size: SharedString,
    pub phrase: SharedString,
    pub phrase_kind: SharedString,
    pub grid_locked: bool,
    pub gain_adjust: SharedString,
    pub jog_angle: f32,
    pub beat_pos: SharedString,
    pub pad_mode: SharedString,
    pub pad_param: SharedString,
    pub key_shift: SharedString,
    pub key_sync: bool,
    pub jump_size: SharedString,
    pub loop_in_set: bool,
    /// Bars to the next drop and breakdown ([`phrase_mix::ahead_text`]).
    pub phrase_ahead: SharedString,
    /// Waits to start on the master's next phrase, and how long ("-3.2").
    pub start_armed: bool,
    pub start_wait: SharedString,
    /// A loop roll is held, and its size ("1/8").
    pub rolling: bool,
    pub roll_size: SharedString,
    /// Where playback resumes after a roll / slip (seconds, −1 = none).
    pub slip_pos: f32,
    /// ÷2 / ×2 can change the track's grid.
    pub bpm_scalable: bool,
    /// [62] The key against the other playing decks ("fits", "energy",
    /// "clash", "" = none) and in words; "41 %" when the key is unsure.
    pub key_relation: SharedString,
    pub key_relation_text: SharedString,
    pub key_unsure: SharedString,
    /// [60] What the stem keys show ([`crate::stem_mix::KeyState`]), the
    /// warning that stems were expected, and the soloed part ("" = none).
    pub stems_state: SharedString,
    pub stems_expected: bool,
    pub stem_solo: SharedString,
}

/// What the projection needs besides the engine's deck state.
pub(crate) struct DeckInputs<'a> {
    pub deck: usize,
    pub meta: Option<&'a Loaded>,
    /// `meta` is still being decoded.
    pub loading: bool,
    pub level: f32,
    pub fader: f32,
    pub quantize: bool,
    /// Separation progress (0..1) while parts are being separated.
    pub stems_busy: Option<f32>,
    /// Auto-loop size when no loop runs, in beats.
    pub loop_size: f64,
    /// Bar counter mode (Preferences › View).
    pub bar_counter: &'a str,
    pub pad_mode: PadMode,
    /// Value of the pad mode's ‹ › (see [`pads::param_label`]).
    pub pad_param: String,
    /// A waiting start: bars.beats left on the master clock ("-3.2").
    pub start_wait: String,
    /// The track waits in the stems-ahead queue.
    pub stems_queued: bool,
    pub stems_expected: bool,
    pub stem_solo: Option<od_engine::Stem>,
}

/// The deck's bar counter in the chosen mode (falls back to counting up).
fn bar_counter(s: &DeckState, meta: Option<&Loaded>, mode: &str) -> String {
    let up = || format::bar_beat(s.beat, s.beat_in_bar);
    let Some(m) = meta.filter(|_| s.beat_in_bar > 0) else {
        return up();
    };
    let beat = s.beat.max(0.0);
    let target = match mode {
        "to-phrase" => m
            .phrases
            .iter()
            .find(|p| (p.start_beat as f64) <= beat && beat < p.end_beat as f64)
            .map(|p| p.end_beat as f64),
        "to-memory-cue" => {
            let dur = s.duration.max(1e-6);
            m.cues
                .iter()
                .filter(|c| c.slot < 0)
                .map(|c| c.pos as f64 * dur)
                .filter(|&t| t > s.position + 0.01)
                .fold(None, |a: Option<f64>, t| Some(a.map_or(t, |a| a.min(t))))
                .and_then(|t| m.grid.beat_index_at(t).map(|i| i as f64))
        }
        _ => None,
    };
    match target.map(|t| format::bar_countdown(beat, t)) {
        Some(c) if !c.is_empty() => c,
        _ => up(),
    }
}

pub(crate) fn tempo_range_label(r: TempoRange) -> &'static str {
    match r {
        TempoRange::Six => "±6%",
        TempoRange::Ten => "±10%",
        TempoRange::Sixteen => "±16%",
        TempoRange::Wide => "WIDE",
    }
}

/// "CHORUS -7.4" — the phrase at `beat` and the bars.beats left in it.
pub(crate) fn phrase_now(phrases: &[od_core::Phrase], beat: f64) -> String {
    let b = beat.max(0.0) as u32;
    phrases
        .iter()
        .find(|p| p.start_beat <= b && b < p.end_beat)
        .map(|p| {
            format!(
                "{} {}",
                render::phrase_label(p.kind),
                format::bar_countdown(beat, p.end_beat as f64)
            )
        })
        .unwrap_or_default()
}

/// Projects one deck's engine state (pure).
pub(crate) fn deck_fields(s: &DeckState, i: &DeckInputs) -> DeckFields {
    let meta = i.meta;
    let muted = |stem: od_engine::Stem| s.stem_levels[stem.index()] < 0.5;
    DeckFields {
        deck: i.deck as i32 + 1,
        // Playable: the engine has the track (a loading one only shows).
        loaded: meta.is_some() && !i.loading,
        loading: i.loading,
        title: meta.map(|m| m.title.as_str()).unwrap_or_default().into(),
        artist: meta.map(|m| m.artist.as_str()).unwrap_or_default().into(),
        bpm: format::bpm(Some(s.bpm), 2).into(),
        key: meta.map(|m| m.key.as_str()).unwrap_or_default().into(),
        tempo: format::tempo(s.tempo).into(),
        elapsed: format::deck_time(s.position).into(),
        remaining: if s.duration > 0.0 {
            format!("-{}", format::deck_time(s.duration - s.position))
        } else {
            String::new()
        }
        .into(),
        remaining_real: format::remaining_at_tempo(s.duration, s.position, s.tempo).into(),
        length: s.duration.max(0.0) as f32,
        progress: if s.duration > 0.0 {
            (s.position / s.duration) as f32
        } else {
            0.0
        },
        playing: s.playing,
        master: s.is_master,
        sync: s.sync,
        keylock: s.keylock,
        looping: s.loop_state.active,
        // A loop that is set (or an armed active loop) shows on the waveform.
        loop_in: s.loop_state.start.zip(s.loop_state.end).or(s.active_loop).map(|(a, _)| a as f32).unwrap_or(0.0),
        loop_out: s.loop_state.start.zip(s.loop_state.end).or(s.active_loop).map(|(_, b)| b as f32).unwrap_or(0.0),
        counter_mode: i.bar_counter.into(),
        loop_beats: s
            .loop_state
            .beats
            .map(format::beats)
            .unwrap_or_default()
            .into(),
        slip: s.slip,
        level: i.level,
        fader: i.fader,
        tempo_value: s.tempo_fader,
        tempo_range: tempo_range_label(s.tempo_range).into(),
        quantize: i.quantize,
        stems_available: s.stems_available,
        stems_busy: i.stems_busy.is_some(),
        split_text: split_text(i.stems_busy).into(),
        split_progress: i.stems_busy.unwrap_or(0.0).clamp(0.0, 1.0),
        mute_vocal: muted(od_engine::Stem::Vocals),
        mute_inst: muted(od_engine::Stem::Inst),
        mute_drums: muted(od_engine::Stem::Drums),
        mute_bass: muted(od_engine::Stem::Bass),
        level_vocal: s.stem_levels[od_engine::Stem::Vocals.index()],
        level_inst: s.stem_levels[od_engine::Stem::Inst.index()],
        level_drums: s.stem_levels[od_engine::Stem::Drums.index()],
        level_bass: s.stem_levels[od_engine::Stem::Bass.index()],
        loop_size: format::beats(
            s.loop_state
                .beats
                .filter(|_| s.loop_state.active)
                .unwrap_or(i.loop_size),
        )
        .into(),
        phrase: meta
            .map(|m| phrase_now(&m.phrases, s.beat))
            .unwrap_or_default()
            .into(),
        phrase_kind: meta
            .and_then(|m| {
                let b = s.beat.max(0.0) as u32;
                m.phrases.iter().find(|p| p.start_beat <= b && b < p.end_beat)
            })
            .map(|p| render::phrase_label(p.kind))
            .unwrap_or_default()
            .into(),
        grid_locked: meta.is_some_and(|m| m.grid.locked),
        gain_adjust: crate::deck_grid::gain_label(meta.map_or(0.0, |m| m.gain_adjust)).into(),
        // 33⅓ rpm: 200° per second of audio.
        jog_angle: (s.position * 200.0).rem_euclid(360.0) as f32,
        beat_pos: bar_counter(s, i.meta, i.bar_counter).into(),
        pad_mode: i.pad_mode.name().into(),
        pad_param: i.pad_param.as_str().into(),
        key_shift: pads::key_shift_label(s.key_shift).into(),
        key_sync: s.key_sync,
        jump_size: pads::beats_label(if s.beat_jump_size > 0.0 {
            s.beat_jump_size
        } else {
            4.0
        })
        .into(),
        loop_in_set: !s.loop_state.active
            && s.loop_state.start.is_some()
            && s.loop_state.end.is_none(),
        phrase_ahead: meta
            .filter(|_| s.beat_in_bar > 0)
            .map(|m| crate::phrase_mix::ahead_text(&m.phrases, s.beat))
            .unwrap_or_default()
            .into(),
        start_armed: s.start_on_beat.is_some(),
        start_wait: i.start_wait.as_str().into(),
        rolling: s.loop_state.roll,
        roll_size: if s.loop_state.roll {
            s.loop_state.beats.map(pads::beats_label).unwrap_or_default()
        } else {
            String::new()
        }
        .into(),
        slip_pos: s.slip_position.map_or(-1.0, |p| p as f32),
        bpm_scalable: meta.is_some_and(|m| !m.grid.locked && !m.grid.beats.is_empty()) && !i.loading,
        key_relation: SharedString::default(),
        key_relation_text: SharedString::default(),
        key_unsure: crate::key_mix::unsure_text(meta.and_then(|m| m.confidence).map(|c| c.key)).into(),
        stems_state: crate::stem_mix::key_state(
            meta.is_some(),
            s.stems_available,
            i.stems_busy.is_some(),
            i.stems_queued,
        )
        .id()
        .into(),
        stems_expected: i.stems_expected && meta.is_some() && !s.stems_available,
        stem_solo: i
            .stem_solo
            .filter(|_| s.stems_available)
            .map(crate::stem_mix::part_id)
            .unwrap_or_default()
            .into(),
    }
}

/// Identity of a rendered scrolling waveform: same key, same pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaveKey {
    track: TrackId,
    /// Playhead in whole pixels.
    px: i64,
    /// Visible span in 0.1 ms.
    span: i64,
    width: u32,
    height: u32,
}

impl WaveKey {
    #[cfg(test)]
    pub fn new(track: TrackId, position: f64, span: f64, width: u32, height: u32) -> Self {
        let px_per_sec = width as f64 / span.max(1e-6);
        Self {
            track,
            px: (position * px_per_sec).round() as i64,
            span: (span * 1e4).round() as i64,
            width,
            height,
        }
    }
}

/// Cached view parts of one deck.
struct Lane {
    fields: Option<DeckFields>,
    /// Track the per-track parts below belong to.
    track: Option<TrackId>,
    cues: ModelRc<CueMark>,
    phrases: ModelRc<PhraseMark>,
    vocals: ModelRc<crate::VocalMark>,
    /// Phrases were shown when `phrases` was built (Preferences > View).
    phrases_on: bool,
    overview: Image,
    artwork: Option<Image>,
    wave_key: Option<WaveKey>,
    waveform: Image,
    pads: Rows<PadView>,
    pads_left: Rows<PadView>,
    pads_right: Rows<PadView>,
}

impl Default for Lane {
    fn default() -> Self {
        Self {
            fields: None,
            track: None,
            cues: model(Vec::new()),
            phrases: model(Vec::new()),
            vocals: model(Vec::new()),
            phrases_on: true,
            overview: Image::default(),
            artwork: None,
            wave_key: None,
            waveform: Image::default(),
            pads: Rows::default(),
            pads_left: Rows::default(),
            pads_right: Rows::default(),
        }
    }
}

impl Lane {
    fn view(&self) -> DeckView {
        let f = self.fields.clone().unwrap_or_default();
        DeckView {
            deck: f.deck,
            loaded: f.loaded,
            loading: f.loading,
            title: f.title,
            artist: f.artist,
            bpm: f.bpm,
            key: f.key,
            tempo: f.tempo,
            elapsed: f.elapsed,
            remaining: f.remaining,
            remaining_real: f.remaining_real,
            progress: f.progress,
            length: f.length,
            playing: f.playing,
            master: f.master,
            sync: f.sync,
            keylock: f.keylock,
            looping: f.looping,
            loop_in: f.loop_in,
            loop_out: f.loop_out,
            counter_mode: f.counter_mode,
            loop_beats: f.loop_beats,
            slip: f.slip,
            level: f.level,
            fader: f.fader,
            waveform: self.waveform.clone(),
            overview: self.overview.clone(),
            artwork: self.artwork.clone().unwrap_or_default(),
            has_artwork: self.artwork.is_some(),
            cues: self.cues.clone(),
            pads: self.pads.handle(),
            pads_left: self.pads_left.handle(),
            pads_right: self.pads_right.handle(),
            tempo_value: f.tempo_value,
            tempo_range: f.tempo_range,
            quantize: f.quantize,
            stems_available: f.stems_available,
            stems_busy: f.stems_busy,
            split_text: f.split_text,
            split_progress: f.split_progress,
            mute_vocal: f.mute_vocal,
            mute_inst: f.mute_inst,
            mute_drums: f.mute_drums,
            mute_bass: f.mute_bass,
            level_vocal: f.level_vocal,
            level_inst: f.level_inst,
            level_drums: f.level_drums,
            level_bass: f.level_bass,
            loop_size: f.loop_size,
            phrases: self.phrases.clone(),
            vocals: self.vocals.clone(),
            phrase: f.phrase,
            phrase_kind: f.phrase_kind,
            grid_locked: f.grid_locked,
            gain_adjust: f.gain_adjust,
            jog_angle: f.jog_angle,
            beat_pos: f.beat_pos,
            pad_mode: f.pad_mode,
            pad_param: f.pad_param,
            key_shift: f.key_shift,
            key_sync: f.key_sync,
            jump_size: f.jump_size,
            loop_in_set: f.loop_in_set,
            phrase_ahead: f.phrase_ahead,
            start_armed: f.start_armed,
            start_wait: f.start_wait,
            rolling: f.rolling,
            roll_size: f.roll_size,
            slip_pos: f.slip_pos,
            bpm_scalable: f.bpm_scalable,
            key_relation: f.key_relation,
            key_relation_text: f.key_relation_text,
            key_unsure: f.key_unsure,
            stems_state: f.stems_state,
            stems_expected: f.stems_expected,
            stem_solo: f.stem_solo,
        }
    }
}

/// The deck models bound to `decks`, `decks-left` (odd decks) and
/// `decks-right` (even decks).
#[derive(Default)]
pub(crate) struct DeckViews {
    pub all: Rows<DeckView>,
    pub left: Rows<DeckView>,
    pub right: Rows<DeckView>,
    lanes: [Lane; DECKS],
}

impl DeckViews {
    /// Re-creates the rows for `count` decks (empty plates).
    fn resize(&mut self, count: usize) {
        for (d, lane) in self.lanes.iter_mut().enumerate() {
            *lane = Lane::default();
            // Empty decks show their hot cue pads and mode too.
            let s = DeckState::default();
            let ctx = pads::PadCtx {
                deck: d,
                mode: PadMode::HotCue,
                sampler_bank: 0,
                keyboard_cue: 0,
                slicer_beats: 8.0,
                labels: &[],
                sampler_slots: &[],
                memory: &[],
            };
            let rows = pads::pad_views(&s, &Default::default(), &[], &ctx);
            lane.pads.sync_eq(rows.clone());
            lane.pads_left.sync_eq(rows[..4].to_vec());
            lane.pads_right.sync_eq(rows[4..].to_vec());
            lane.fields = Some(DeckFields {
                deck: d as i32 + 1,
                pad_mode: PadMode::HotCue.name().into(),
                pad_param: pads::param_label(&s, &ctx).into(),
                key_shift: pads::key_shift_label(0.0).into(),
                jump_size: "4".into(),
                ..Default::default()
            });
        }
        let views: Vec<DeckView> = (0..count).map(|d| self.lanes[d].view()).collect();
        let side = |odd: bool| {
            views
                .iter()
                .filter(|v| (v.deck % 2 == 1) == odd)
                .cloned()
                .collect::<Vec<_>>()
        };
        self.left.sync(side(true), |_, _| false);
        self.right.sync(side(false), |_, _| false);
        self.all.sync(views, |_, _| false);
    }

    /// Writes deck `d` into the full list and its side column.
    fn publish(&self, d: usize) {
        let v = self.lanes[d].view();
        let side = if d % 2 == 0 { &self.left } else { &self.right };
        side.set(d / 2, v.clone());
        self.all.set(d, v);
    }
}

/// Before audio starts: empty plates so the Perform layout is complete.
pub(crate) fn show_empty(app: &App) {
    let mut d = app.decks.borrow_mut();
    let count = d.deck_count;
    if d.views.all.len() != count {
        d.views.resize(count);
    }
}

/// Frame tick: engine state → deck rows (only what changed).
pub(crate) fn project(app: &App, state: &EngineState) {
    let show_phrases = app.ui.get_show_phrases();
    // The phrase bar under the overview has its own switch (Preferences > View).
    let show_overview_phrases = app.ui.get_show_phrases_overview();
    let bar_mode = app.state.borrow().settings.view.bar_counter.clone();
    let show_overlay = app.state.borrow().settings.view.incoming_overlay;
    let notation = app.state.borrow().settings.key_notation();
    let scale = app.ui.window().scale_factor();
    let sampler_bank = app.sampler.bank.get();
    let sampler_slots = app.sampler.slots.borrow();
    let mut guard = app.decks.borrow_mut();
    let d = &mut *guard;
    let count = d.deck_count;
    let vertical = app.ui.global::<crate::PerformLayout>().get_layout() == "vertical";
    let (lane_width, height) = if vertical {
        // Vertical lanes: the length runs down the deck row, the thickness
        // is one column above the mixer.
        let len = ((app.ui.window().size().height as f32 / scale * 0.42).max(200.0) * scale) as u32;
        (len, ((if count > 2 { 56.0 } else { 84.0 }) * scale) as u32)
    } else {
        // Waveform area of a lane: the window minus the numeral gutter.
        (
            ((app.ui.window().size().width as f32 / scale - 44.0).max(200.0) * scale) as u32,
            ((if count > 2 { 64.0 } else { 110.0 }) * scale) as u32,
        )
    };
    let now = Instant::now();
    let master = state.master_deck.filter(|&m| m < count);
    let master_beat = master.map(|m| state.decks[m].beat);
    if d.views.all.len() != count {
        d.views.resize(count);
    }
    for i in 0..count {
        let s = &state.decks[i];
        // A track being loaded shows at once; a just-loaded one also while
        // the engine state has not caught up yet.
        let playable = d.loaded[i].as_ref().filter(|m| Some(m.id) == s.track);
        let meta = d.pending[i]
            .as_ref()
            .or(playable)
            .or_else(|| d.loaded[i].as_ref().filter(|_| s.track.is_none()));
        // Shown but not in the engine yet.
        let loading = meta.is_some() && (d.pending[i].is_some() || playable.is_none());
        // The decks' own separation of the track, or the stems-ahead batch's.
        let stem_busy = meta.and_then(|m| d.stem_running(m.id).or_else(|| crate::stems_batch::running(m.id)));
        let stem_queued = meta.is_some_and(|m| d.stem_waiting(m.id) || crate::stems_batch::is_queued(m.id));
        let lane = &mut d.views.lanes[i];
        let mut dirty = false;
        let tid = meta.map(|m| m.id);
        if lane.track != tid || lane.fields.is_none() || lane.phrases_on != show_overview_phrases {
            lane.track = tid;
            lane.phrases_on = show_overview_phrases;
            lane.cues = model(meta.map(|m| m.cues.clone()).unwrap_or_default());
            lane.phrases = model(
                meta.filter(|_| show_overview_phrases)
                    .map(|m| m.phrase_marks.clone())
                    .unwrap_or_default(),
            );
            lane.overview = meta.map(|m| m.overview.clone()).unwrap_or_default();
            lane.vocals = model(meta.map(|m| m.vocals.clone()).unwrap_or_default());
            lane.artwork = meta.and_then(|m| m.artwork.clone());
            lane.waveform = Image::default();
            lane.wave_key = None;
            dirty = true;
        }
        // Zoomed waveform: tiles + motion (span follows tempo, so beats keep
        // their spacing; quantised to 10 ms to avoid re-rendering on every
        // tempo-fader step).
        let span = ((DETAIL_SPAN * d.zoom[i] * s.tempo.max(0.25)) * 100.0).round() / 100.0;
        let speed = if s.playing && !s.scratching {
            s.speed
        } else {
            0.0
        };
        let input = ScrollIn {
            meta,
            position: s.position,
            speed,
            span,
            width: lane_width,
            height,
            phrases: show_phrases,
            vertical,
            outline: false,
            now,
        };
        if let Some(row) = d.scroll[i].update(&input) {
            d.motion.set_row_data(i, row);
        }
        let memory = meta
            .map(|m| m.memory_starts(s.duration))
            .unwrap_or_default();
        let ctx = pads::PadCtx {
            deck: i,
            mode: d.pad_mode[i],
            sampler_bank,
            keyboard_cue: d.keyboard_cue[i],
            slicer_beats: d.slicer_beats[i],
            labels: meta.map(|m| m.labels.as_slice()).unwrap_or(&[]),
            sampler_slots: &sampler_slots,
            memory: &memory,
        };
        let mut rows = pads::pad_views(s, &state.channels[i], &state.sampler, &ctx);
        if d.pad_mode[i] == PadMode::Lighting {
            crate::lighting::light_pads(app, &mut rows);
        }
        if d.pad_mode[i] == PadMode::PatternRecall {
            crate::sampler::seq_pads(app, state, &mut rows);
        }
        if d.pad_mode[i] == PadMode::HotCue
            && let Some(m) = meta
        {
            // Hot cue pads in their stored colours.
            let bank = s.hot_cue_bank * od_engine::HOT_CUES;
            for r in rows.iter_mut() {
                if let Some(c) = m.cues.iter().find(|c| c.slot == bank as i32 + r.index) {
                    r.tone = c.tone;
                }
            }
        }
        let pad_param = pads::param_label(s, &ctx);
        lane.pads.sync_eq(rows.clone());
        lane.pads_left.sync_eq(rows[..4].to_vec());
        lane.pads_right.sync_eq(rows[4..].to_vec());
        let fields = deck_fields(
            s,
            &DeckInputs {
                deck: i,
                meta,
                loading,
                level: state.channels[i].meter.peak,
                fader: state.channels[i].fader,
                quantize: state.quantize,
                stems_busy: stem_busy,
                stems_queued: stem_queued,
                stems_expected: d.stems_expected[i],
                stem_solo: d.stem_solo[i].map(|x| x.stem),
                loop_size: d.loop_size[i],
                bar_counter: &bar_mode,
                pad_mode: d.pad_mode[i],
                pad_param,
                start_wait: match (s.start_on_beat, master_beat) {
                    (Some(t), Some(b)) => crate::phrase_mix::wait_text(b, t),
                    _ => String::new(),
                },
            },
        );
        // [62] The key against the other playing decks.
        let mut fields = fields;
        let rel = if loading { None } else { crate::key_mix::relation(&state.decks, count, i) };
        fields.key_relation = crate::key_mix::relation_id(rel.map(|r| r.0)).into();
        fields.key_relation_text = crate::key_mix::relation_text(rel, notation).into();
        // The loop heard last (a roll too), for CAPTURE.
        if s.loop_state.active
            && let (Some(t), Some(a), Some(b)) = (s.track, s.loop_state.start, s.loop_state.end)
        {
            d.last_loop[i] = Some((t, a, b));
        }
        if lane.fields.as_ref() != Some(&fields) {
            lane.fields = Some(fields);
            dirty = true;
        }
        if dirty {
            d.views.publish(i);
        }
    }
    project_overlay(d, state, count, master, (lane_width, height), show_overlay && !vertical, now);
}

/// The deck whose track is drawn over the playing master `m`: one that
/// waits for the master's phrase first, then one that plays, then the
/// first other loaded deck.
fn incoming_deck(state: &EngineState, loaded: &[Option<Loaded>], count: usize, m: usize) -> Option<usize> {
    let ready = |j: usize| {
        j != m
            && loaded[j]
                .as_ref()
                .is_some_and(|l| Some(l.id) == state.decks[j].track)
    };
    let cands: Vec<usize> = (0..count).filter(|&j| ready(j)).collect();
    cands
        .iter()
        .copied()
        .find(|&j| state.decks[j].start_on_beat.is_some())
        .or_else(|| cands.iter().copied().find(|&j| state.decks[j].playing))
        .or_else(|| cands.first().copied())
}

/// The incoming track's outline over the playing master's lane: where it
/// plays, or (cued) where its cue meets the master's next phrase.
fn project_overlay(
    d: &mut Decks,
    state: &EngineState,
    count: usize,
    master: Option<usize>,
    (width, height): (u32, u32),
    show: bool,
    now: Instant,
) {
    let pair = master
        .filter(|&m| show && state.decks[m].playing && d.loaded[m].is_some())
        .and_then(|m| incoming_deck(state, &d.loaded, count, m).map(|j| (m, j)));
    for i in 0..DECKS {
        let mut extra = (0, -1.0f32);
        let row = match pair {
            Some((m, j)) if m == i && i < count => {
                let (ms, sj) = (&state.decks[m], &state.decks[j]);
                let rate = if sj.sync && sj.track_bpm > 0.0 {
                    ms.bpm / sj.track_bpm
                } else {
                    sj.tempo
                }
                .max(0.25);
                let phrases = d.loaded[m].as_ref().map(|l| l.phrases.as_slice()).unwrap_or(&[]);
                let start = sj
                    .start_on_beat
                    .unwrap_or_else(|| crate::phrase_mix::next_boundary(phrases, ms.beat, 0.25));
                let pos = crate::phrase_mix::incoming_position(
                    sj.position,
                    sj.playing,
                    rate,
                    ms.beat,
                    ms.bpm,
                    start,
                );
                extra = (j as i32 + 1, if sj.playing { -1.0 } else { sj.position as f32 });
                let span = ((DETAIL_SPAN * d.zoom[i] * rate) * 100.0).round() / 100.0;
                let input = ScrollIn {
                    meta: d.loaded[j].as_ref(),
                    position: pos,
                    speed: rate,
                    span,
                    width,
                    height,
                    phrases: false,
                    vertical: false,
                    outline: true,
                    now,
                };
                d.overlay_scroll[i].update(&input)
            }
            _ => {
                let input = ScrollIn {
                    meta: None,
                    position: 0.0,
                    speed: 0.0,
                    span: 1.0,
                    width,
                    height,
                    phrases: false,
                    vertical: false,
                    outline: true,
                    now,
                };
                d.overlay_scroll[i].update(&input)
            }
        };
        let current = d.overlay.row_data(i).unwrap_or_default();
        match row {
            Some(mut r) => {
                if r.loaded {
                    (r.of_deck, r.mark) = extra;
                }
                d.overlay.set_row_data(i, r);
            }
            None if current.loaded && (current.of_deck, current.mark) != extra => {
                d.overlay.set_row_data(
                    i,
                    LaneMotion {
                        of_deck: extra.0,
                        mark: extra.1,
                        ..current
                    },
                );
            }
            None => {}
        }
    }
}

// ── Controls ────────────────────────────────────────────────────────────

pub(crate) fn deck_state(app: &App, d: usize) -> Option<DeckState> {
    engine::snapshot(app).map(|s| s.decks[d])
}

/// Hot cues set or deleted while performing (pads, controller) become
/// library edits: one undo step each, synced to sticks like any edit. The
/// cue keeps its label and colour when only its position changes.
pub(crate) fn persist_cue_events(app: &App, events: &[od_engine::Event]) {
    use od_engine::Event;
    // Tracks played from a stick are read-only here.
    if events.is_empty() || app.state.borrow().read_only() || app.decks.borrow().loaded_from_stick {
        return;
    }
    for ev in events {
        let (track, index, cue) = match *ev {
            Event::HotCueSet {
                deck,
                track,
                bank,
                slot,
                cue,
            } if deck != PREVIEW_DECK => (track, bank * 8 + slot, Some(cue)),
            Event::HotCueDeleted {
                deck,
                track,
                bank,
                slot,
            } if deck != PREVIEW_DECK => (track, bank * 8 + slot, None),
            _ => continue,
        };
        let Ok(index) = u8::try_from(index) else {
            continue;
        };
        let current = app.state.borrow().lib.cues(track).unwrap_or_default();
        let next = apply_hot_cue(current.clone(), index, cue);
        if next == current {
            continue;
        }
        let result = app.state.borrow_mut().lib.set_cues(track, next);
        match result {
            Ok(()) => {
                refresh_deck_cues(app, track);
                app.after_edit("");
            }
            Err(e) => tracing::warn!(error = %e, "could not save a hot cue set while performing"),
        }
    }
}

/// Rebuilds the cue marks and pad labels of every deck showing `track`.
fn refresh_deck_cues(app: &App, track: TrackId) {
    let cues = app.state.borrow().lib.cues(track).unwrap_or_default();
    let duration = app
        .state
        .borrow()
        .lib
        .track(track)
        .map(|t| t.duration)
        .unwrap_or(0.0);
    let mut d = app.decks.borrow_mut();
    for i in 0..DECKS {
        if let Some(m) = d.loaded[i].as_mut().filter(|m| m.id == track) {
            m.cues = cue_marks(&cues, duration, false);
            m.labels = hot_labels(&cues);
            // Force the lane to rebuild its per-track models.
            d.views.lanes[i].track = None;
        }
    }
}

/// Hot cue labels by library index, for every bank.
fn hot_labels(cues: &[od_core::Cue]) -> Vec<String> {
    (0..od_engine::HOT_CUE_BANKS * od_engine::HOT_CUES)
        .map(|s| {
            cues.iter()
                .find(|c| c.kind == CueKind::Hot(s as u8))
                .map(|c| c.label.clone())
                .unwrap_or_default()
        })
        .collect()
}

/// The cue list with hot cue `index` set to `cue` (or removed when `None`).
pub(crate) fn apply_hot_cue(
    mut cues: Vec<od_core::Cue>,
    index: u8,
    cue: Option<od_engine::HotCue>,
) -> Vec<od_core::Cue> {
    let pos = cues.iter().position(|c| c.kind == CueKind::Hot(index));
    match (cue, pos) {
        (Some(h), Some(i)) => {
            cues[i].start = h.start;
            cues[i].loop_end = h.loop_end;
        }
        (Some(h), None) => {
            let mut c = od_core::Cue::hot(index, h.start);
            c.loop_end = h.loop_end;
            cues.push(c);
        }
        (None, Some(i)) => {
            cues.remove(i);
        }
        (None, None) => {}
    }
    cues
}

/// Tempo range and fader position (−1..1) that play a track of `track_bpm`
/// at `target`: the current range if it reaches, else the smallest that
/// does (WIDE goes from 0 to double speed). `None` when out of reach.
pub(crate) fn tempo_for_bpm(track_bpm: f64, target: f64, current: TempoRange) -> Option<(TempoRange, f32)> {
    if track_bpm <= 0.0 || !(1.0..=999.0).contains(&target) {
        return None;
    }
    let pct = target / track_bpm - 1.0;
    let fits = |r: TempoRange| pct.abs() <= r.fraction() + 1e-9;
    let range = if fits(current) {
        current
    } else {
        [TempoRange::Six, TempoRange::Ten, TempoRange::Sixteen, TempoRange::Wide].into_iter().find(|r| fits(*r))?
    };
    Some((range, (pct / range.fraction()) as f32))
}

/// New zoom after a scroll of `delta` logical pixels (up = closer): from
/// 1 s to 32 s across the lane.
pub(crate) fn zoom_by(zoom: f64, delta: f64) -> f64 {
    (zoom * (-delta * 0.004).exp()).clamp(0.125, 4.0)
}

// ── Pad modes, key shift ────────────────────────────────────────────────

/// Runs `f` with the pad context of deck `d` and its engine state (starts
/// audio first).
fn with_pads<R>(app: &App, d: usize, f: impl FnOnce(&DeckState, &pads::PadCtx) -> R) -> R {
    engine::ensure_engine(app);
    let s = deck_state(app, d).unwrap_or_default();
    let slots = app.sampler.slots.borrow();
    let dk = app.decks.borrow();
    let labels = dk.loaded(d).map(|m| m.labels.clone()).unwrap_or_default();
    let memory = dk
        .loaded(d)
        .map(|m| m.memory_starts(s.duration))
        .unwrap_or_default();
    let ctx = pads::PadCtx {
        deck: d,
        mode: dk.pad_mode[d],
        sampler_bank: app.sampler.bank.get(),
        keyboard_cue: dk.keyboard_cue[d],
        slicer_beats: dk.slicer_beats[d],
        labels: &labels,
        sampler_slots: &slots,
        memory: &memory,
    };
    f(&s, &ctx)
}

/// The pads of deck `d` in its mode (for the controller's LEDs).
pub(crate) fn pad_rows(app: &App, d: usize, state: &EngineState) -> Vec<PadView> {
    let slots = app.sampler.slots.borrow();
    let dk = app.decks.borrow();
    let labels = dk.loaded(d).map(|m| m.labels.as_slice()).unwrap_or(&[]);
    let memory = dk
        .loaded(d)
        .map(|m| m.memory_starts(state.decks[d].duration))
        .unwrap_or_default();
    let ctx = pads::PadCtx {
        deck: d,
        mode: dk.pad_mode[d],
        sampler_bank: app.sampler.bank.get(),
        keyboard_cue: dk.keyboard_cue[d],
        slicer_beats: dk.slicer_beats[d],
        labels,
        sampler_slots: &slots,
        memory: &memory,
    };
    pads::pad_views(&state.decks[d], &state.channels[d], &state.sampler, &ctx)
}

/// Pad `index` (0..8) of deck `d` pressed or released, in the deck's mode.
pub(crate) fn pad(app: &App, d: usize, index: usize, pressed: bool) {
    if pressed
        && app.decks.borrow().pad_mode[d] == PadMode::Stems
        && !deck_state(app, d).is_some_and(|st| st.stems_available)
    {
        crate::stems_model::part_pressed(app, d);
        return;
    }
    match app.decks.borrow().pad_mode[d] {
        PadMode::Lighting => {
            crate::lighting::pad(app, index, pressed);
            return;
        }
        PadMode::PatternRecall => {
            if pressed
                && engine::ensure_engine(app)
                && let Some(st) = engine::snapshot(app)
            {
                crate::sampler::pattern_recall(app, index, &st);
            }
            return;
        }
        _ => {}
    }
    let cmds = with_pads(app, d, |s, ctx| {
        pads::press_commands(s, ctx, index, pressed)
    });
    for c in cmds {
        engine::send(app, c);
    }
}

/// The shifted pad (right click): deletes a hot cue, stops a sample.
pub(crate) fn pad_alt(app: &App, d: usize, index: usize) {
    let mode = app.decks.borrow().pad_mode[d];
    engine::ensure_engine(app);
    match mode {
        PadMode::HotCue => {
            engine::deck_cmd(app, d, DeckCommand::DeleteHotCue { slot: index.min(7) })
        }
        PadMode::Sampler => {
            let slot = pads::sampler_slot(d, app.sampler.bank.get(), index);
            engine::send(
                app,
                Command::Sampler(od_engine::SamplerCommand::Stop { slot }),
            );
        }
        PadMode::PatternRecall => crate::sampler::seq_clear(app, index),
        _ => {}
    }
}

/// Switches deck `d` to pad mode `mode`.
pub(crate) fn set_pad_mode(app: &App, d: usize, mode: PadMode) {
    let (old, beats) = {
        let dk = app.decks.borrow();
        (dk.pad_mode[d], dk.slicer_beats[d])
    };
    if old != mode {
        engine::ensure_engine(app);
        for c in pads::mode_change_commands(d, old, mode, beats) {
            engine::send(app, c);
        }
    }
    app.decks.borrow_mut().pad_mode[d] = mode;
}

/// The ‹ › beside the pad mode: bank, page, jump size, key, slice length.
pub(crate) fn pad_param(app: &App, d: usize, delta: i32) {
    engine::ensure_engine(app);
    let s = deck_state(app, d).unwrap_or_default();
    let mode = app.decks.borrow().pad_mode[d];
    match mode {
        PadMode::HotCue => engine::deck_cmd(
            app,
            d,
            DeckCommand::SetHotCueBank(pads::step_hot_cue_bank(s.hot_cue_bank, delta)),
        ),
        PadMode::HoldFx1 => set_pad_mode(app, d, PadMode::HoldFx2),
        PadMode::HoldFx2 => set_pad_mode(app, d, PadMode::HoldFx1),
        PadMode::BeatJump => step_jump_size(app, d, delta),
        PadMode::Sampler => crate::sampler::step_bank_by(app, delta),
        PadMode::Keyboard => {
            let mut dk = app.decks.borrow_mut();
            dk.keyboard_cue[d] = (dk.keyboard_cue[d] as i32 + delta).rem_euclid(8) as usize;
        }
        PadMode::KeyShift => key_shift(app, d, delta),
        PadMode::Slicer => {
            let beats = pads::step_in(
                &pads::SLICER_BEATS,
                app.decks.borrow().slicer_beats[d],
                delta,
            );
            app.decks.borrow_mut().slicer_beats[d] = beats;
            if s.slicer.active {
                engine::deck_cmd(
                    app,
                    d,
                    DeckCommand::Slicer(od_engine::SlicerCommand::Enable {
                        beats,
                        loop_mode: s.slicer.loop_mode,
                    }),
                );
            }
        }
        PadMode::BeatLoop
        | PadMode::Stems
        | PadMode::MemoryCue
        | PadMode::Lighting
        | PadMode::TimedCensor
        | PadMode::PatternRecall
        | PadMode::LoopRoll => {}
    }
}

/// Beat jump size one step up or down.
pub(crate) fn step_jump_size(app: &App, d: usize, delta: i32) {
    let cur = deck_state(app, d)
        .map(|s| s.beat_jump_size)
        .filter(|b| *b > 0.0)
        .unwrap_or(4.0);
    engine::deck_cmd(
        app,
        d,
        DeckCommand::SetBeatJumpSize(pads::step_in(&pads::JUMP_SIZES, cur, delta)),
    );
}

/// Key shift by `delta` semitones (0 resets), −12 … +12.
pub(crate) fn key_shift(app: &App, d: usize, delta: i32) {
    engine::ensure_engine(app);
    let cur = deck_state(app, d).map(|s| s.key_shift).unwrap_or(0.0);
    let next = if delta == 0 {
        0.0
    } else {
        (cur.round() + delta as f32).clamp(-12.0, 12.0)
    };
    engine::deck_cmd(app, d, DeckCommand::SetKeyShift(next));
}

pub(crate) fn toggle_key_sync(app: &App, d: usize) {
    engine::ensure_engine(app);
    let on = deck_state(app, d).map(|s| s.key_sync).unwrap_or(false);
    engine::deck_cmd(app, d, DeckCommand::SetKeySync(!on));
}

// ── Cue and grid edits from the controller ──────────────────────────────

/// A constant grid at `bpm` that keeps the first downbeat of `grid` (pure).
pub(crate) fn tapped_grid(grid: &Beatgrid, bpm: f64, duration: f64) -> Beatgrid {
    let anchor = grid
        .beats
        .iter()
        .find(|b| b.bar_position == 1)
        .or(grid.beats.first())
        .map(|b| b.time)
        .unwrap_or(0.0);
    let mut g = Beatgrid::constant(bpm, anchor, 1, duration.max(anchor + 1.0));
    g.locked = grid.locked;
    g
}

/// The cue list with a memory cue added at `pos` (none within 50 ms), or
/// the memory cue nearest to `pos` (within 1 s) removed (pure).
pub(crate) fn edit_memory_cue(
    mut cues: Vec<od_core::Cue>,
    pos: f64,
    add: bool,
) -> Vec<od_core::Cue> {
    let memory = |c: &od_core::Cue| c.kind == CueKind::Memory;
    if add {
        if !cues
            .iter()
            .any(|c| memory(c) && (c.start - pos).abs() < 0.05)
        {
            cues.push(od_core::Cue::memory(pos));
        }
    } else if let Some(i) = cues
        .iter()
        .enumerate()
        .filter(|(_, c)| memory(c) && (c.start - pos).abs() <= 1.0)
        .min_by(|a, b| (a.1.start - pos).abs().total_cmp(&(b.1.start - pos).abs()))
        .map(|(i, _)| i)
    {
        cues.remove(i);
    }
    cues
}

// ── Phrase-aligned blends, loop capture, half / double ([56] [57] [59]) ─

/// SYNC (screen, keys, controller). In the PHRASE sync mode a playing deck
/// that syncs also jumps whole beats so it stands as far before one of its
/// phrase boundaries as the master before its next one.
pub(crate) fn toggle_sync(app: &App, d: usize) {
    engine::ensure_engine(app);
    let Some(state) = engine::snapshot(app) else { return };
    let s = state.decks[d];
    let on = !s.sync;
    engine::deck_cmd(app, d, DeckCommand::SetSync(on));
    let phrase = app.state.borrow().settings.deck.sync_mode == "phrase";
    if !(on && phrase && s.playing) {
        return;
    }
    let Some(m) = state.master_deck.filter(|&m| m != d && state.decks[m].playing) else {
        return;
    };
    let jump = {
        let dk = app.decks.borrow();
        let (Some(lm), Some(ld)) = (dk.loaded(m), dk.loaded(d)) else { return };
        crate::phrase_mix::sync_jump(&lm.phrases, state.decks[m].beat, &ld.phrases, s.beat)
    };
    if let Some(beats) = jump {
        engine::deck_cmd(app, d, DeckCommand::BeatJump { beats });
        app.status(od_i18n::trf("Synced to the phrase: {} beats", &[&format!("{beats:+}")]));
    }
}

/// Start on the master's next phrase: arms a cued deck to start when the
/// playing master reaches its next phrase boundary; a second press (or
/// PLAY / CUE) cancels.
pub(crate) fn phrase_start(app: &App, d: usize) {
    engine::ensure_engine(app);
    let Some(state) = engine::snapshot(app) else { return };
    let s = state.decks[d];
    if s.start_on_beat.is_some() {
        engine::send(app, Command::StartOnBeat { deck: d, beat: None });
        app.status("Phrase start cancelled");
        return;
    }
    if s.playing || s.track.is_none() {
        app.status("Phrase start: cue a track on a paused deck first");
        return;
    }
    let Some(m) = state.master_deck.filter(|&m| m != d && state.decks[m].playing) else {
        app.status("Phrase start: no other deck is playing to wait for");
        return;
    };
    let target = {
        let dk = app.decks.borrow();
        let phrases = dk.loaded(m).map(|l| l.phrases.as_slice()).unwrap_or(&[]);
        // Not a boundary that is reached before the start can be heard.
        crate::phrase_mix::next_boundary(phrases, state.decks[m].beat, 0.25)
    };
    engine::send(app, Command::StartOnBeat { deck: d, beat: Some(target) });
    app.status(od_i18n::trf(
        "Deck {} starts on deck {}'s next phrase ({} bars)",
        &[
            &(d + 1),
            &(m + 1),
            &crate::phrase_mix::wait_text(state.decks[m].beat, target),
        ],
    ));
}

/// CAPTURE: the loop heard last on deck `d` (a roll or a loop) becomes a
/// hot loop on the first free pad of the active bank, or a memory loop when
/// the bank is full. Saved like any cue edit (one undo step).
pub(crate) fn capture_loop(app: &App, d: usize) {
    engine::ensure_engine(app);
    let Some(s) = deck_state(app, d) else { return };
    let Some(id) = editable(app, d) else { return };
    let current = s
        .loop_state
        .start
        .zip(s.loop_state.end)
        .filter(|_| s.loop_state.active);
    let last = app.decks.borrow().last_loop[d]
        .filter(|(t, _, _)| *t == id)
        .map(|(_, a, b)| (a, b));
    let Some((start, end)) = current.or(last).filter(|(a, b)| b > a) else {
        app.status("Nothing to capture: roll or play a loop first");
        return;
    };
    match capture_target(&s) {
        Some(slot) => {
            engine::deck_cmd(
                app,
                d,
                DeckCommand::SetHotCueAt { slot, start, loop_end: Some(end) },
            );
            let letter = pads::hot_letter(s.hot_cue_bank * od_engine::HOT_CUES + slot);
            app.status(od_i18n::trf("Loop captured on hot cue {}", &[&letter]));
        }
        None => {
            let current = app.state.borrow().lib.cues(id).unwrap_or_default();
            let mut next = edit_memory_cue(current.clone(), start, true);
            set_memory_loop_end(&mut next, start, end);
            if next == current {
                app.status("This loop is already stored");
                return;
            }
            let r = app.state.borrow_mut().lib.set_cues(id, next.clone());
            if app.report(r).is_none() {
                return;
            }
            if let Some(m) = app.decks.borrow_mut().loaded[d].as_mut() {
                m.source.cues = next;
            }
            refresh_deck_cues(app, id);
            reload_engine_data(app, d);
            app.after_edit("Loop captured as a memory loop (the hot cues are full)");
        }
    }
}

/// The pad a captured loop goes to: the first free one of the active bank.
pub(crate) fn capture_target(s: &DeckState) -> Option<usize> {
    s.hot_cues.iter().position(|c| c.is_none())
}

/// ÷2 / ×2 on the deck: the track's grid at half or double the tempo (the
/// GRID panel's scale, one undo step).
pub(crate) fn bpm_scale(app: &App, d: usize, factor: f64) {
    crate::deck_grid::act(app, d, "scale", if factor < 1.0 { "0.5" } else { "2" });
}

/// The key of `track` changed (one-button fix): decks holding it show and
/// play with the new key.
pub(crate) fn refresh_key(app: &App, track: TrackId, key: od_core::MusicalKey) {
    let notation = app.state.borrow().settings.key_notation();
    let decks: Vec<usize> = {
        let mut d = app.decks.borrow_mut();
        (0..DECKS)
            .filter(|&i| {
                d.loaded[i].as_mut().is_some_and(|m| {
                    if m.id != track {
                        return false;
                    }
                    m.key = key.format(notation);
                    m.musical_key = Some(key);
                    if let Some(c) = m.confidence.as_mut() {
                        c.key = 1.0;
                        c.key_alt = None;
                    }
                    true
                })
            })
            .collect()
    };
    for i in decks {
        engine::deck_cmd(app, i, DeckCommand::SetTrackKey(Some(key)));
        app.decks.borrow_mut().views.lanes[i].fields = None;
    }
}

/// Makes the memory cue at `start` a saved loop ending at `end`.
pub(crate) fn set_memory_loop_end(cues: &mut [od_core::Cue], start: f64, end: f64) {
    if let Some(c) = cues
        .iter_mut()
        .find(|c| c.kind == CueKind::Memory && (c.start - start).abs() < 0.05)
    {
        c.loop_end = Some(end);
    }
}

/// Gives the engine a fresh build of deck `d`'s track (new cues or grid)
/// without a gap.
fn reload_engine_data(app: &App, d: usize) {
    let rate = app.engine.borrow().sample_rate();
    let dk = app.decks.borrow();
    let Some(m) = dk.loaded(d) else { return };
    let (cache, tx, id, src, grid) = (
        dk.stem_cache.clone(),
        dk.prepared_tx.clone(),
        m.id,
        m.source.clone(),
        m.grid.clone(),
    );
    drop(dk);
    prepare_in_background(
        cache,
        tx,
        d,
        id,
        src,
        rate,
        grid,
        None,
        Some(Swap::Analysis),
    );
}

/// Whether deck `d`'s track may be edited (not from a stick).
fn editable(app: &App, d: usize) -> Option<TrackId> {
    let dk = app.decks.borrow();
    if dk.loaded_from_stick {
        app.status("Tracks on a stick are read-only here — edit them in your library and the stick follows");
        return None;
    }
    dk.loaded_id(d)
}

/// MEMORY (controller): stores a memory cue at the playhead, or deletes the
/// one at the playhead; saved to the library like any cue edit.
pub(crate) fn memory_cue(app: &App, d: usize, add: bool) {
    if !add && app.guard_locked(crate::perform_lock::Blocked::Delete) {
        return;
    }
    let Some(id) = editable(app, d) else { return };
    let Some(st) = deck_state(app, d) else { return };
    // While a loop plays, MEMORY stores the loop (a saved memory loop).
    let looped = st
        .loop_state
        .start
        .zip(st.loop_state.end)
        .filter(|_| st.loop_state.active);
    let pos = looped.map_or(st.position, |(a, _)| a);
    let current = app.state.borrow().lib.cues(id).unwrap_or_default();
    let mut next = edit_memory_cue(current.clone(), pos, add);
    if add && let Some((_, end)) = looped {
        set_memory_loop_end(&mut next, pos, end);
    }
    if next == current {
        return;
    }
    let r = app.state.borrow_mut().lib.set_cues(id, next.clone());
    if app.report(r).is_none() {
        return;
    }
    if let Some(m) = app.decks.borrow_mut().loaded[d].as_mut() {
        m.source.cues = next;
    }
    refresh_deck_cues(app, id);
    reload_engine_data(app, d);
    app.after_edit(if add {
        "Memory cue added"
    } else {
        "Memory cue deleted"
    });
}

impl Loaded {
    /// Content hash of the loaded audio (the transients cache key).
    pub(crate) fn content_hash(&self) -> &od_core::ContentHash {
        &self.source.hash
    }
}

/// Decks playing `id` take `grid` at once (a grid edit, also while it is
/// only previewed): the view and the engine's copy of the track, which
/// keeps its audio, so playback goes on without a gap.
pub(crate) fn follow_grid(app: &App, id: TrackId, grid: &Beatgrid) {
    let mut swaps = Vec::new();
    {
        let mut dk = app.decks.borrow_mut();
        for d in 0..DECKS {
            let Some(m) = dk.loaded[d].as_mut().filter(|m| m.id == id) else {
                continue;
            };
            if m.grid == *grid {
                continue;
            }
            m.grid = grid.clone();
            dk.scroll[d] = LaneScroll::default();
            dk.views.lanes[d].track = None;
            if let Some(data) = dk.engine_data[d].as_ref().filter(|x| x.id == id) {
                let next = data.with_beatgrid(grid.clone());
                dk.engine_data[d] = Some(next.clone());
                swaps.push((d, next));
            }
        }
    }
    for (d, data) in swaps {
        engine::deck_cmd(app, d, DeckCommand::SwapTrackData(data));
    }
}

/// TAP (controller): after three taps the track gets a constant grid at the
/// tapped tempo, keeping its first downbeat.
pub(crate) fn tap_bpm(app: &App, d: usize) {
    if app.guard_locked(crate::perform_lock::Blocked::Grid) {
        return;
    }
    let t = TAP_EPOCH.with(|e| e.elapsed().as_secs_f64());
    let Some(bpm) = app.decks.borrow_mut().taps[d].tap(t) else {
        return;
    };
    let Some(id) = editable(app, d) else { return };
    let bpm = (bpm as f64 * 100.0).round() / 100.0;
    let grid = app
        .state
        .borrow()
        .lib
        .beatgrid(id)
        .ok()
        .flatten()
        .unwrap_or_default();
    if grid.locked {
        app.status("The beatgrid is locked — unlock it in Prepare to tap the tempo");
        return;
    }
    let duration = deck_state(app, d).map(|s| s.duration).unwrap_or(0.0);
    let next = tapped_grid(&grid, bpm, duration);
    let r = app.state.borrow_mut().lib.set_beatgrid(id, next.clone());
    if app.report(r).is_none() {
        return;
    }
    follow_grid(app, id, &next);
    app.after_edit(&format!("Tapped tempo: {bpm:.2} BPM"));
}

thread_local! {
    static TAP_EPOCH: Instant = Instant::now();
}

/// A part by its UI name ("vocal", "inst", "drums", "bass").
pub(crate) fn stem_of_ui(name: &str) -> Option<od_engine::Stem> {
    match name {
        "vocal" | "vocals" => Some(od_engine::Stem::Vocals),
        "drums" => Some(od_engine::Stem::Drums),
        "bass" => Some(od_engine::Stem::Bass),
        "inst" => Some(od_engine::Stem::Inst),
        _ => None,
    }
}

pub(crate) fn wire(app: &Rc<App>) {
    let zoom = crate::preferences::default_deck_zoom(&app.state.borrow().settings);
    app.decks.borrow_mut().zoom = [zoom; DECKS];
    let layout = app
        .state
        .borrow()
        .lib
        .meta(META_PERFORM_LAYOUT)
        .ok()
        .flatten();
    if let Some(l) = layout.filter(|l| PERFORM_LAYOUTS.contains(&l.as_str())) {
        app.ui.global::<crate::PerformLayout>().set_layout(l.into());
    }
    let count = app.state.borrow().lib.meta(META_DECK_COUNT).ok().flatten();
    if let Some(n) = count.and_then(|n| n.parse::<usize>().ok()) {
        app.decks.borrow_mut().deck_count = n.clamp(2, DECKS);
    }
    let ui = &app.ui;
    ui.on_set_perform_view(handler!(app, |a, v: SharedString| set_perform_view(&a, &v)));
    ui.set_lane_motion(ModelRc::from(app.decks.borrow().motion.clone()));
    // One zoom for every lane: beats keep the same spacing on all decks, so
    // lanes can be compared at a glance.
    ui.on_deck_zoom(handler!(app, |a, d: i32, delta: f32| {
        let i = (d as usize).saturating_sub(1).min(DECKS - 1);
        let mut decks = a.decks.borrow_mut();
        let zoom = zoom_by(decks.zoom[i], delta as f64);
        decks.zoom = [zoom; DECKS];
    }));
    // The lanes' bar counter: to the phrase end → to the next memory cue →
    // bars counted up (also in Preferences › View).
    ui.on_cycle_bar_counter(handler!(app, |a| {
        let modes = crate::settings::BAR_COUNTERS;
        let now = a.state.borrow().settings.view.bar_counter.clone();
        let next = modes[(modes.iter().position(|m| *m == now).unwrap_or(0) + 1) % modes.len()];
        a.state.borrow_mut().change_settings(|s| s.view.bar_counter = next.to_string());
        crate::preferences::refresh(&a);
    }));
    ui.on_deck_zoom_reset(handler!(app, |a, _d: i32| {
        let zoom = crate::preferences::default_deck_zoom(&a.state.borrow().settings);
        a.decks.borrow_mut().zoom = [zoom; DECKS];
    }));
    // Deck callbacks carry 1-based deck numbers.
    let idx = |d: i32| (d as usize).saturating_sub(1).min(DECKS - 1);
    ui.on_deck_toggle_play(handler!(app, |a, d: i32| engine::deck_cmd(
        &a,
        d as usize - 1,
        DeckCommand::TogglePlay
    )));
    ui.on_deck_cue(handler!(app, |a, d: i32| {
        engine::deck_cmd(&a, d as usize - 1, DeckCommand::Cue(true));
        engine::deck_cmd(&a, d as usize - 1, DeckCommand::Cue(false));
    }));
    ui.on_deck_sync(handler!(app, |a, d: i32| toggle_sync(&a, idx(d))));
    // Phrase start, loop capture and ÷2 / ×2 ([56] [57] [59]).
    let mix = ui.global::<crate::DeckMix>();
    mix.set_overlay(app.decks.borrow().lane_overlay());
    mix.on_phrase_start(handler!(app, |a, d: i32| phrase_start(&a, idx(d))));
    mix.on_capture(handler!(app, |a, d: i32| capture_loop(&a, idx(d))));
    mix.on_bpm_scale(handler!(app, |a, d: i32, f: f32| bpm_scale(&a, idx(d), f as f64)));
    mix.on_stem_solo(handler!(app, |a, d: i32, stem: SharedString| {
        if let Some(s) = stem_of_ui(stem.as_str()) {
            crate::stem_mix::toggle_solo(&a, idx(d), s);
        }
    }));
    mix.on_stem_keep(handler!(app, |a, d: i32, stem: SharedString| {
        if let Some(s) = stem_of_ui(stem.as_str()) {
            crate::stem_mix::keep_only(&a, idx(d), s);
        }
    }));
    ui.on_deck_seek(handler!(app, |a, d: i32, f: f32| engine::deck_cmd(
        &a,
        d as usize - 1,
        DeckCommand::NeedleSearch { fraction: f as f64 }
    )));
    ui.on_deck_hot_cue(handler!(app, |a, d: i32, s: i32| {
        engine::deck_cmd(
            &a,
            d as usize - 1,
            DeckCommand::HotCue {
                slot: s as usize,
                pressed: true,
            },
        );
        engine::deck_cmd(
            &a,
            d as usize - 1,
            DeckCommand::HotCue {
                slot: s as usize,
                pressed: false,
            },
        );
    }));
    ui.on_set_deck_count(handler!(app, |a, n: i32| a.decks.borrow_mut().deck_count =
        (n as usize).clamp(2, 4)));

    // These start audio on first use.
    ui.on_deck_master(handler!(app, |a, d: i32| {
        engine::ensure_engine(&a);
        engine::send(&a, Command::SetMasterDeck(Some(idx(d))));
    }));
    ui.on_deck_keylock(handler!(app, |a, d: i32| {
        engine::ensure_engine(&a);
        let d = idx(d);
        let on = deck_state(&a, d).map(|s| s.keylock).unwrap_or(false);
        engine::deck_cmd(&a, d, DeckCommand::SetKeylock(!on));
    }));
    ui.on_deck_quantize(handler!(app, |a, _d: i32| {
        engine::ensure_engine(&a);
        let q = engine::snapshot(&a).map(|s| s.quantize).unwrap_or(true);
        engine::send(&a, Command::SetQuantize(!q));
    }));
    ui.on_deck_slip(handler!(app, |a, d: i32| {
        engine::ensure_engine(&a);
        let d = idx(d);
        let on = deck_state(&a, d).map(|s| s.slip).unwrap_or(false);
        engine::deck_cmd(&a, d, DeckCommand::SetSlip(!on));
    }));
    ui.on_deck_tempo(handler!(app, |a, d: i32, v: f32| {
        engine::ensure_engine(&a);
        engine::deck_cmd(&a, idx(d), DeckCommand::SetTempoFader(v));
    }));
    ui.on_deck_stem(handler!(app, |a, d: i32, stem: SharedString| {
        engine::ensure_engine(&a);
        let d = idx(d);
        // Not separated yet: the press is a request for parts.
        if !deck_state(&a, d).is_some_and(|st| st.stems_available) {
            crate::stems_model::part_pressed(&a, d);
            return;
        }
        let s = match stem.as_str() {
            "vocal" => od_engine::Stem::Vocals,
            "drums" => od_engine::Stem::Drums,
            "bass" => od_engine::Stem::Bass,
            _ => od_engine::Stem::Inst,
        };
        // A click during a solo ends it first: the levels from before return.
        if a.decks.borrow().stem_solo[d].is_some() {
            crate::stem_mix::end_solo(&a, d);
            return;
        }
        let level = deck_state(&a, d)
            .map(|st| st.stem_levels[s.index()])
            .unwrap_or(1.0);
        engine::deck_cmd(
            &a,
            d,
            DeckCommand::SetStemLevel {
                stem: s,
                level: if level < 0.5 { 1.0 } else { 0.0 },
            },
        );
    }));
    ui.on_deck_stem_level(handler!(
        app,
        |a, d: i32, stem: SharedString, level: f32| {
            engine::ensure_engine(&a);
            if let Some(s) = stem_of_ui(stem.as_str()) {
                engine::deck_cmd(
                    &a,
                    idx(d),
                    DeckCommand::SetStemLevel {
                        stem: s,
                        level: level.clamp(0.0, 1.0),
                    },
                );
            }
        }
    ));
    ui.on_deck_loop(handler!(app, |a, d: i32, beats: f32| {
        engine::ensure_engine(&a);
        let d = idx(d);
        let b = if beats > 0.0 {
            beats as f64
        } else {
            a.decks.borrow().loop_size[d]
        };
        engine::deck_cmd(&a, d, DeckCommand::AutoLoop { beats: b });
    }));
    // Manual loop: in point, then out point (the loop starts); a running
    // loop is left.
    ui.on_deck_loop_manual(handler!(app, |a, d: i32| {
        engine::ensure_engine(&a);
        let d = idx(d);
        let l = deck_state(&a, d).map(|s| s.loop_state).unwrap_or_default();
        let cmd = if l.active {
            DeckCommand::LoopExit
        } else if l.start.is_some() && l.end.is_none() {
            DeckCommand::LoopOut
        } else {
            DeckCommand::LoopIn
        };
        engine::deck_cmd(&a, d, cmd);
    }));
    // A typed target BPM: the fader goes there (sync off), in the current
    // range if it reaches, else the smallest range that does.
    ui.on_deck_set_bpm(handler!(app, |a, d: i32, text: SharedString| {
        engine::ensure_engine(&a);
        let d = idx(d);
        let Some(s) = deck_state(&a, d) else { return };
        let target = text.trim().replace(',', ".").parse::<f64>().ok();
        let track_bpm = if s.tempo > 0.0 { s.bpm / s.tempo } else { 0.0 };
        match target.and_then(|t| tempo_for_bpm(track_bpm, t, s.tempo_range)) {
            Some((range, fader)) => {
                if s.sync {
                    engine::deck_cmd(&a, d, DeckCommand::SetSync(false));
                }
                if range != s.tempo_range {
                    engine::deck_cmd(&a, d, DeckCommand::SetTempoRange(range));
                }
                engine::deck_cmd(&a, d, DeckCommand::SetTempoFader(fader));
                a.status(format!("Deck {}: {} BPM", d + 1, format::bpm(target, 2)));
            }
            None => a.status("Type a tempo the track can reach (half to double its BPM)"),
        }
    }));
    // Tempo range of the fader: ±6 → ±10 → ±16 → WIDE → ±6.
    ui.on_deck_tempo_range(handler!(app, |a, d: i32| {
        engine::ensure_engine(&a);
        let d = idx(d);
        let r = deck_state(&a, d)
            .map(|s| s.tempo_range)
            .unwrap_or(TempoRange::Ten);
        engine::deck_cmd(
            &a,
            d,
            DeckCommand::SetTempoRange(crate::controller::next_tempo_range(r)),
        );
    }));
    ui.on_deck_loop_exit(handler!(app, |a, d: i32| {
        engine::ensure_engine(&a);
        engine::deck_cmd(&a, idx(d), DeckCommand::LoopExit);
    }));
    ui.on_deck_loop_scale(handler!(app, |a, d: i32, f: f32| {
        engine::ensure_engine(&a);
        let d = idx(d);
        let looping = deck_state(&a, d)
            .map(|s| s.loop_state.active)
            .unwrap_or(false);
        if looping {
            engine::deck_cmd(
                &a,
                d,
                if f < 1.0 {
                    DeckCommand::LoopHalve
                } else {
                    DeckCommand::LoopDouble
                },
            );
        }
        let mut decks = a.decks.borrow_mut();
        decks.loop_size[d] = (decks.loop_size[d] * f as f64).clamp(1.0 / 32.0, 32.0);
    }));
    ui.on_deck_beat_jump(handler!(app, |a, d: i32, beats: f32| {
        engine::ensure_engine(&a);
        engine::deck_cmd(
            &a,
            idx(d),
            DeckCommand::BeatJump {
                beats: beats as f64,
            },
        );
    }));
    ui.on_deck_pad(handler!(app, |a, d: i32, i: i32, pressed: bool| pad(
        &a,
        idx(d),
        i.max(0) as usize,
        pressed
    )));
    ui.on_deck_pad_alt(handler!(app, |a, d: i32, i: i32| pad_alt(
        &a,
        idx(d),
        i.max(0) as usize
    )));
    ui.on_deck_pad_mode_step(handler!(app, |a, d: i32, x: i32| {
        let mode = pads::step_mode(a.decks.borrow().pad_mode[idx(d)], x);
        set_pad_mode(&a, idx(d), mode);
    }));
    ui.on_deck_pad_mode_set(handler!(app, |a, d: i32, i: i32| set_pad_mode(
        &a,
        idx(d),
        pads::UI_MODES[(i.max(0) as usize).min(pads::UI_MODES.len() - 1)]
    )));
    ui.on_deck_pad_param(handler!(app, |a, d: i32, x: i32| pad_param(&a, idx(d), x)));
    ui.on_deck_key_sync(handler!(app, |a, d: i32| toggle_key_sync(&a, idx(d))));
    ui.on_deck_key_shift(handler!(app, |a, d: i32, x: i32| key_shift(&a, idx(d), x)));
    ui.on_deck_beat_jump_step(handler!(app, |a, d: i32, fwd: bool| {
        engine::ensure_engine(&a);
        engine::deck_cmd(&a, idx(d), DeckCommand::BeatJumpStep { forward: fwd });
    }));
    ui.on_deck_load(handler!(app, |a, d: i32| {
        engine::ensure_engine(&a);
        load_selected(&a, idx(d));
    }));
    demo(app);
}

/// Documentation screenshots (`OPENDJDECKS_DEMO_PROOF`):
/// `phrase-start` pauses deck 2 and keeps it armed for deck 1's next
/// phrase (the incoming outline over lane 1); `loop-roll` puts deck 1's
/// pads in LOOP ROLL and holds a 1/8 roll.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let phrase = v.split(',').any(|p| p == "phrase-start");
    let roll = v.split(',').any(|p| p == "loop-roll");
    if !phrase && !roll {
        return;
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(5), move || {
        let Some(a) = weak.upgrade() else { return };
        engine::send(&a, Command::SetMasterDeck(Some(0)));
        if roll {
            set_pad_mode(&a, 0, PadMode::LoopRoll);
            engine::deck_cmd(&a, 0, DeckCommand::LoopRoll { beats: 0.125, on: true });
        }
        if !phrase {
            return;
        }
        let cue = deck_state(&a, 1).map(|s| s.position).unwrap_or(0.0);
        engine::deck_cmd(&a, 1, DeckCommand::Pause);
        let w = Rc::downgrade(&a);
        let rearm = Rc::new(slint::Timer::default());
        let keep = rearm.clone();
        rearm.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(250),
            move || {
                let _ = &keep;
                let Some(a) = w.upgrade() else { return };
                let Some(st) = engine::snapshot(&a) else { return };
                let s = st.decks[1];
                if let Some(t) = s.start_on_beat {
                    // Keep it waiting: move on to the phrase after.
                    if t - st.decks[0].beat < 1.0 {
                        let next = {
                            let dk = a.decks.borrow();
                            let ph = dk.loaded(0).map(|l| l.phrases.clone()).unwrap_or_default();
                            crate::phrase_mix::next_boundary(&ph, t, 0.25)
                        };
                        engine::send(&a, Command::StartOnBeat { deck: 1, beat: Some(next) });
                    }
                    return;
                }
                if s.playing {
                    engine::deck_cmd(&a, 1, DeckCommand::Pause);
                    engine::deck_cmd(&a, 1, DeckCommand::Seek { seconds: cue });
                } else {
                    phrase_start(&a, 1);
                }
            },
        );
    });
}

#[cfg(test)]
mod tests {

    #[test]
    fn typed_bpm_picks_range_and_fader() {
        use od_engine::TempoRange::*;
        let (r, f) = tempo_for_bpm(120.0, 126.0, Ten).unwrap();
        assert_eq!(r, Ten);
        assert!((f - 0.5).abs() < 1e-6);
        // +12 % does not fit ±10 %: the smallest range that does.
        assert_eq!(tempo_for_bpm(120.0, 134.4, Ten).unwrap().0, Sixteen);
        assert_eq!(tempo_for_bpm(120.0, 180.0, Six).unwrap().0, Wide);
        assert!(tempo_for_bpm(120.0, 300.0, Ten).is_none(), "beyond double speed");
        assert!(tempo_for_bpm(0.0, 120.0, Ten).is_none(), "no grid");
    }

    use super::*;

    #[test]
    fn hot_cue_events_edit_the_cue_list() {
        let mut named = od_core::Cue::hot(2, 10.0);
        named.label = "Drop".into();
        let cues = vec![named, od_core::Cue::hot(0, 1.0)];
        // Moving C keeps its label; setting B adds it; deleting A removes it.
        let moved = apply_hot_cue(
            cues,
            2,
            Some(od_engine::HotCue {
                start: 12.5,
                loop_end: None,
            }),
        );
        let c = moved.iter().find(|c| c.kind == CueKind::Hot(2)).unwrap();
        assert_eq!((c.start, c.label.as_str()), (12.5, "Drop"));
        let added = apply_hot_cue(
            moved,
            1,
            Some(od_engine::HotCue {
                start: 4.0,
                loop_end: Some(6.0),
            }),
        );
        let b = added.iter().find(|c| c.kind == CueKind::Hot(1)).unwrap();
        assert_eq!((b.start, b.loop_end), (4.0, Some(6.0)));
        let removed = apply_hot_cue(added, 0, None);
        assert!(!removed.iter().any(|c| c.kind == CueKind::Hot(0)));
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn memory_cue_edits() {
        let cues = vec![od_core::Cue::hot(0, 1.0), od_core::Cue::memory(10.0)];
        let added = edit_memory_cue(cues.clone(), 20.0, true);
        assert_eq!(added.len(), 3);
        // No duplicate at the same spot.
        assert_eq!(edit_memory_cue(added.clone(), 20.01, true).len(), 3);
        // Delete takes the nearest memory cue within a second, never a hot cue.
        let removed = edit_memory_cue(added, 10.4, false);
        assert!(
            !removed
                .iter()
                .any(|c| c.kind == CueKind::Memory && c.start == 10.0)
        );
        assert_eq!(edit_memory_cue(removed.clone(), 1.0, false), removed);
        // MEMORY while looping saves the loop.
        let mut with_loop = edit_memory_cue(Vec::new(), 32.0, true);
        set_memory_loop_end(&mut with_loop, 32.0, 36.0);
        assert_eq!(with_loop[0].loop_end, Some(36.0));
        assert!(with_loop[0].is_loop());
    }

    #[test]
    fn tapped_grid_keeps_the_downbeat() {
        let g = Beatgrid::constant(120.0, 0.25, 3, 60.0);
        let first_down = g.beats.iter().find(|b| b.bar_position == 1).unwrap().time;
        let t = tapped_grid(&g, 125.0, 60.0);
        assert!(
            t.beats
                .iter()
                .any(|b| b.bar_position == 1 && (b.time - first_down).abs() < 1e-9)
        );
        assert!((t.beats[1].time - t.beats[0].time - 60.0 / 125.0).abs() < 1e-9);
        assert!(
            !tapped_grid(&Beatgrid::default(), 128.0, 30.0)
                .beats
                .is_empty()
        );
    }

    #[test]
    fn autoload_spec() {
        assert_eq!(
            parse_autoload("1:12, 2:13@64.5"),
            vec![(1, TrackId(12), None), (2, TrackId(13), Some(64.5))]
        );
        assert_eq!(parse_autoload("5:1,x:2,3:y,4"), vec![]);
        assert_eq!(parse_autoload(""), vec![]);
    }

    #[test]
    fn free_deck_prefers_idle_empty() {
        assert_eq!(free_deck(2, &[false, false], |d| d == 0), 1);
        assert_eq!(free_deck(2, &[true, false], |_| true), 1);
        assert_eq!(free_deck(2, &[true, true], |_| true), 0);
        assert_eq!(free_deck(4, &[false; 4], |_| false), 0);
    }

    #[test]
    fn empty_deck_projection() {
        let s = DeckState::default();
        let f = deck_fields(
            &s,
            &DeckInputs {
                deck: 1,
                meta: None,
                loading: false,
                level: 0.0,
                fader: 1.0,
                quantize: true,
                stems_busy: None,
                loop_size: 0.5,
                bar_counter: "up",
                pad_mode: PadMode::HotCue,
                pad_param: String::new(),
                start_wait: String::new(),
                stems_queued: false,
                stems_expected: false,
                stem_solo: None,
            },
        );
        assert_eq!(f.deck, 2);
        assert!(!f.loaded);
        assert_eq!(f.bpm, "");
        assert_eq!(f.remaining, "");
        assert_eq!(f.loop_size, "1/2");
        assert_eq!(f.elapsed, "0:00.0");
        // Unchanged state projects equal, so no row is written.
        let g = deck_fields(
            &s,
            &DeckInputs {
                deck: 1,
                meta: None,
                loading: false,
                level: 0.0,
                fader: 1.0,
                quantize: true,
                stems_busy: None,
                loop_size: 0.5,
                bar_counter: "up",
                pad_mode: PadMode::HotCue,
                pad_param: String::new(),
                start_wait: String::new(),
                stems_queued: false,
                stems_expected: false,
                stem_solo: None,
            },
        );
        assert_eq!(f, g);
    }

    #[test]
    fn waveform_key_changes_per_pixel() {
        let t = TrackId(1);
        // 800 px over 8 s: 100 px per second.
        let a = WaveKey::new(t, 10.000, 8.0, 800, 100);
        assert_eq!(a, WaveKey::new(t, 10.004, 8.0, 800, 100));
        assert_ne!(a, WaveKey::new(t, 10.01, 8.0, 800, 100));
        assert_ne!(a, WaveKey::new(t, 10.0, 8.08, 800, 100));
        assert_ne!(a, WaveKey::new(t, 10.0, 8.0, 801, 100));
    }

    #[test]
    fn vocal_spans_become_overview_fractions() {
        let m = vocal_marks(&[(30.0, 60.0), (150.0, 240.0)], 200.0);
        assert_eq!((m[0].start, m[0].end), (0.15, 0.3));
        assert_eq!(m[1].end, 1.0, "clamped to the track");
    }

    #[test]
    fn active_loop_comes_from_the_marked_cue() {
        let mut l = od_core::Cue {
            loop_end: Some(36.0),
            ..od_core::Cue::memory(32.0)
        };
        assert_eq!(active_loop(std::slice::from_ref(&l)), None);
        l.active = true;
        assert_eq!(
            active_loop(&[od_core::Cue::memory(1.0), l]),
            Some((32.0, 36.0))
        );
    }

    #[test]
    fn split_progress_text() {
        assert_eq!(split_text(None), "");
        assert_eq!(split_text(Some(0.0)), "SPLIT…");
        assert_eq!(split_text(Some(0.423)), "SPLIT 42%");
        assert_eq!(split_text(Some(1.0)), "SPLIT 99%");
    }

    #[test]
    fn stick_tracks_use_the_stems_on_the_stick() {
        let root = std::path::Path::new("/Volumes/STICK");
        let cache = stick_stem_cache(root);
        assert_eq!(
            cache.dir(),
            std::path::Path::new("/Volumes/STICK/OPENDJDECKS/stems")
        );
        // Same file name as the sync writes (od-devices `stem_file`).
        let hash = od_core::ContentHash("ab12".into());
        assert_eq!(
            cache.path(&hash),
            std::path::Path::new("/Volumes/STICK/OPENDJDECKS/stems/ab12.odstem")
        );
    }

    #[test]
    fn tempo_ranges() {
        assert_eq!(tempo_range_label(TempoRange::Ten), "±10%");
        assert_eq!(tempo_range_label(TempoRange::Wide), "WIDE");
    }
}
