//! Sampler: 16 slots per bank, one-shot / loop / gate, gain, quantised
//! triggering, sync to the master tempo and a 16-step sequencer (SPEC
//! §10.3a).
//!
//! [77] The fifth bank is MELODY: CREATE's SEND TO PERFORM fills it with
//! notes ([`fill_bank`]), the MELODY pad mode plays it, and otherwise it is
//! a bank like the others (the same slots, store and controls).
//!
//! [80] The sixth bank is DRUMS: CREATE's drum machine puts its kit on pads
//! 1–8 ([`put_pads`], keeping each slot's steps) and samples from the
//! library on 9–16. Its step row is this sequencer's pattern of those
//! slots; SWING and HALF TIME are the sequencer's own and kept here. The
//! sequences of the PATTERN pads ([`Sequence`]) are kept too, with their
//! swing and half time, so a pattern saved in CREATE plays from Perform
//! after a restart.
//!
//! The engine holds the audio and the playing state; this module loads
//! samples (decoded off the UI thread), remembers what is in which slot
//! (library meta `sampler`, restored when audio starts) and projects the
//! engine state into the panel.

use crate::app::{App, handler};
use crate::models::Rows;
use crate::{SamplerPadView, SamplerView, engine};
use crossbeam_channel::{Receiver, Sender, unbounded};
use od_engine::{
    Command, EngineState, MixerCommand, SAMPLER_BANKS, SAMPLER_SLOTS, SAMPLER_TOTAL, SEQ_STEPS,
    SampleData, SampleMode, SamplerCommand,
};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Library meta key of the slot assignments.
pub(crate) const META_SAMPLER: &str = "sampler";

/// What is loaded in a slot (the engine holds the audio).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SlotInfo {
    pub path: PathBuf,
    pub name: String,
    pub bpm: Option<f64>,
    /// "oneshot", "loop" or "gate".
    pub mode: String,
    pub gain: f32,
    pub sync: bool,
    /// Sequencer pattern (bit n = step n).
    pub pattern: u16,
}

/// [80] A sequence on a PATTERN pad: every slot's steps, and the swing and
/// half time it plays with.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Sequence {
    pub patterns: Vec<u16>,
    pub swing: f32,
    pub half_time: bool,
}

impl Sequence {
    /// The sequencer as it is now.
    pub fn of(state: &EngineState) -> Self {
        Self {
            patterns: state.sampler.iter().map(|s| s.pattern).collect(),
            swing: state.sequencer.swing,
            half_time: state.sequencer.half_time,
        }
    }

    /// Steps set across all slots.
    pub fn hits(&self) -> u32 {
        self.patterns.iter().map(|m| m.count_ones()).sum()
    }
}

/// Pads of the PATTERN pad mode (one stored sequence each).
pub(crate) const SEQUENCE_PADS: usize = 8;

/// The stored sampler: slot assignments by slot number, and [80] the
/// sequencer's swing and half time and the PATTERN pads' sequences.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SamplerStore {
    pub slots: Vec<(usize, SlotInfo)>,
    pub quantize: bool,
    pub swing: f32,
    pub half_time: bool,
    pub sequences: Vec<Option<Sequence>>,
}

/// A decoded sample ready for the engine.
struct Decoded {
    slot: usize,
    /// Restored at start: no status message.
    quiet: bool,
    info: SlotInfo,
    data: Arc<SampleData>,
}

pub(crate) struct SamplerRuntime {
    /// Bank shown in the panel and played by the deck pads (0..5, the last
    /// one MELODY).
    pub bank: Cell<usize>,
    /// Slot edited in the panel (0..SAMPLER_TOTAL).
    pub selected: Cell<usize>,
    pub slots: RefCell<Vec<Option<SlotInfo>>>,
    pub pads: Rows<SamplerPadView>,
    pub steps: Rows<bool>,
    /// Sequences stored on the Pattern Recall pads (one pattern per slot).
    pub sequences: RefCell<[Option<Sequence>; SEQUENCE_PADS]>,
    quantize: Cell<bool>,
    /// [80] Samples being decoded.
    decoding: Cell<usize>,
    /// [80] The sequencer's swing (share of a step) and half time.
    swing: Cell<f32>,
    half_time: Cell<bool>,
    restored: Cell<bool>,
    tx: Sender<Result<Decoded, String>>,
    rx: Receiver<Result<Decoded, String>>,
}

impl Default for SamplerRuntime {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self {
            bank: Cell::new(0),
            selected: Cell::new(0),
            slots: RefCell::new(vec![None; SAMPLER_TOTAL]),
            pads: Rows::default(),
            steps: Rows::default(),
            sequences: RefCell::new(Default::default()),
            quantize: Cell::new(false),
            decoding: Cell::new(0),
            swing: Cell::new(0.0),
            half_time: Cell::new(false),
            restored: Cell::new(false),
            tx,
            rx,
        }
    }
}

pub(crate) fn mode_name(m: SampleMode) -> &'static str {
    match m {
        SampleMode::OneShot => "oneshot",
        SampleMode::Loop => "loop",
        SampleMode::Gate => "gate",
    }
}

pub(crate) fn mode_from_name(s: &str) -> SampleMode {
    match s {
        "loop" => SampleMode::Loop,
        "gate" => SampleMode::Gate,
        _ => SampleMode::OneShot,
    }
}

/// Panel index of a mode (ONE SHOT, LOOP, GATE).
pub(crate) fn mode_index(m: SampleMode) -> i32 {
    match m {
        SampleMode::OneShot => 0,
        SampleMode::Loop => 1,
        SampleMode::Gate => 2,
    }
}

pub(crate) fn mode_from_index(i: i32) -> SampleMode {
    match i {
        1 => SampleMode::Loop,
        2 => SampleMode::Gate,
        _ => SampleMode::OneShot,
    }
}

/// Slot number of pad `pad` (0..16) in `bank`.
pub(crate) fn slot_of(bank: usize, pad: usize) -> usize {
    (bank.min(SAMPLER_BANKS - 1)) * SAMPLER_SLOTS + pad.min(SAMPLER_SLOTS - 1)
}

/// The bank `delta` banks away, wrapping.
pub(crate) fn step_bank(bank: usize, delta: i32) -> usize {
    (bank as i32 + delta).rem_euclid(SAMPLER_BANKS as i32) as usize
}

/// Sampler volume knob (0..1) → engine level (square law, 1 = 0 dB).
pub(crate) fn volume_to_level(v: f32) -> f32 {
    v.clamp(0.0, 1.0).powi(2)
}

pub(crate) fn level_to_volume(l: f32) -> f32 {
    l.max(0.0).sqrt().min(1.0)
}

/// The step pattern as 16 flags.
pub(crate) fn pattern_steps(mask: u16) -> Vec<bool> {
    (0..SEQ_STEPS).map(|i| mask & (1 << i) != 0).collect()
}

/// Projects the 16 pads of `bank` (pure).
pub(crate) fn pad_views(
    state: &EngineState,
    slots: &[Option<SlotInfo>],
    bank: usize,
    selected: usize,
) -> Vec<SamplerPadView> {
    (0..SAMPLER_SLOTS)
        .map(|pad| {
            let slot = slot_of(bank, pad);
            let s = &state.sampler[slot];
            SamplerPadView {
                slot: slot as i32,
                pad: pad as i32 + 1,
                name: slots
                    .get(slot)
                    .and_then(|i| i.as_ref())
                    .map(|i| i.name.as_str())
                    .unwrap_or_default()
                    .into(),
                loaded: s.loaded,
                playing: s.playing,
                pending: s.pending,
                mode: mode_index(s.mode),
                gain: s.gain,
                progress: s.progress,
                selected: slot == selected,
                sync: s.sync,
            }
        })
        .collect()
}

/// Projects the panel's scalar part (pure).
pub(crate) fn sampler_view(
    state: &EngineState,
    slots: &[Option<SlotInfo>],
    bank: usize,
    selected: usize,
) -> SamplerView {
    let s = &state.sampler[selected.min(SAMPLER_TOTAL - 1)];
    let seq = &state.sequencer;
    SamplerView {
        bank: bank as i32,
        selected: if selected / SAMPLER_SLOTS == bank {
            (selected % SAMPLER_SLOTS) as i32
        } else {
            -1
        },
        selected_number: selected as i32 % SAMPLER_SLOTS as i32 + 1,
        selected_name: slots
            .get(selected)
            .and_then(|i| i.as_ref())
            .map(|i| i.name.as_str())
            .unwrap_or_default()
            .into(),
        selected_loaded: s.loaded,
        selected_mode: mode_index(s.mode),
        selected_gain: s.gain,
        selected_sync: s.sync,
        selected_bpm: s.bpm.map(|b| format!("{b:.1}")).unwrap_or_default().into(),
        quantize: seq.quantize.is_some(),
        volume: level_to_volume(state.sampler_level),
        cue: state.sampler_cue,
        seq_playing: seq.playing,
        seq_recording: seq.recording,
        step: if seq.playing { seq.step as i32 } else { -1 },
    }
}

/// Frame tick: decoded samples → engine, restore, engine state → panel.
pub(crate) fn poll(app: &App) {
    let done: Vec<Result<Decoded, String>> = app.sampler.rx.try_iter().collect();
    let left = app.sampler.decoding.get().saturating_sub(done.len());
    app.sampler.decoding.set(left);
    for d in done {
        match d {
            Ok(d) => install(app, d),
            Err(e) => app.status(format!("Sample could not be loaded: {e}")),
        }
    }
}

/// Puts a decoded sample into its slot.
fn install(app: &App, d: Decoded) {
    let slot = d.slot;
    engine::send(
        app,
        Command::Sampler(SamplerCommand::Load {
            slot,
            sample: d.data,
        }),
    );
    engine::send(
        app,
        Command::Sampler(SamplerCommand::SetBpm {
            slot,
            bpm: d.info.bpm,
        }),
    );
    engine::send(
        app,
        Command::Sampler(SamplerCommand::SetMode {
            slot,
            mode: mode_from_name(&d.info.mode),
        }),
    );
    engine::send(
        app,
        Command::Sampler(SamplerCommand::SetGain {
            slot,
            gain: d.info.gain,
        }),
    );
    engine::send(
        app,
        Command::Sampler(SamplerCommand::SetSync {
            slot,
            on: d.info.sync && d.info.bpm.is_some(),
        }),
    );
    if d.info.pattern != 0 {
        engine::send(
            app,
            Command::Sampler(SamplerCommand::SeqPattern {
                slot,
                mask: d.info.pattern,
            }),
        );
    }
    let name = d.info.name.clone();
    let d_quiet = d.quiet;
    app.sampler.slots.borrow_mut()[slot] = Some(d.info);
    if !d_quiet {
        save_soon(app);
        app.status(format!(
            "Loaded {name} into sampler slot {}",
            slot % SAMPLER_SLOTS + 1
        ));
    }
}

/// [77] Puts samples already in memory into `bank`, pad by pad (`None`
/// empties a pad), as one-shots at unity gain without tempo or pattern,
/// and keeps them like loaded files: `info.path` is where each one is
/// stored, so they come back after a restart.
pub(crate) fn fill_bank(app: &App, bank: usize, pads: Vec<Option<(SlotInfo, Arc<SampleData>)>>) {
    engine::ensure_engine(app);
    for (pad, entry) in pads.into_iter().enumerate().take(SAMPLER_SLOTS) {
        let slot = slot_of(bank, pad);
        let Some((info, data)) = entry else {
            engine::send(app, Command::Sampler(SamplerCommand::Unload { slot }));
            app.sampler.slots.borrow_mut()[slot] = None;
            continue;
        };
        engine::send(app, Command::Sampler(SamplerCommand::SeqPattern { slot, mask: 0 }));
        install(
            app,
            Decoded {
                slot,
                quiet: true,
                info,
                data,
            },
        );
    }
    save_soon(app);
    if engine::snapshot(app).is_none() {
        show_idle(app);
    }
}

/// [80] Puts samples already in memory on pads of `bank` (`(pad, sample)`),
/// as [`fill_bank`] does, but each slot keeps its steps: the kit's sounds
/// change under the pattern.
pub(crate) fn put_pads(app: &App, bank: usize, pads: Vec<(usize, SlotInfo, Arc<SampleData>)>) {
    engine::ensure_engine(app);
    let state = engine::snapshot(app);
    for (pad, mut info, data) in pads {
        let slot = slot_of(bank, pad);
        info.pattern = state
            .as_ref()
            .map(|s| s.sampler[slot].pattern)
            .or_else(|| app.sampler.slots.borrow()[slot].as_ref().map(|i| i.pattern))
            .unwrap_or(0);
        install(app, Decoded { slot, quiet: true, info, data });
    }
    save_soon(app);
}

/// [80] Loads the track or file selected (as LOAD SELECTED does) onto pad
/// `pad` of `bank`; its steps stay. Its name, or none when nothing is
/// selected.
pub(crate) fn load_onto(app: &App, bank: usize, pad: usize) -> Option<String> {
    let (path, name, bpm) = selected_source(app)?;
    load_file(app, bank, pad, path, name, bpm)
}

/// [80] Loads `path` onto pad `pad` of `bank` (its steps stay); its name.
pub(crate) fn load_file(app: &App, bank: usize, pad: usize, path: PathBuf, name: String, bpm: Option<f64>) -> Option<String> {
    if !engine::ensure_engine(app) {
        return None;
    }
    let slot = slot_of(bank, pad);
    let pattern = engine::snapshot(app).map_or(0, |s| s.sampler[slot].pattern);
    let info = SlotInfo { path, name: name.clone(), bpm, mode: "oneshot".into(), gain: 1.0, sync: false, pattern };
    decode(app, slot, info, false);
    save_soon(app);
    Some(name)
}

/// Decodes `info.path` in the background for `slot`.
fn decode(app: &App, slot: usize, info: SlotInfo, quiet: bool) {
    let rate = app.engine.borrow().sample_rate();
    let tx = app.sampler.tx.clone();
    app.sampler.decoding.set(app.sampler.decoding.get() + 1);
    std::thread::spawn(move || {
        let r = SampleData::load(&info.path, rate)
            .map(|data| Decoded {
                slot,
                quiet,
                info,
                data,
            })
            .map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
}

/// [80] The stored slots are back (or none was stored) and no sample is
/// being decoded.
pub(crate) fn settled(app: &App) -> bool {
    app.sampler.restored.get() && app.sampler.decoding.get() == 0
}

/// After audio starts: reloads the stored slots once.
fn restore(app: &App) {
    if app.sampler.restored.replace(true) {
        return;
    }
    let store = app
        .state
        .borrow()
        .lib
        .meta(META_SAMPLER)
        .ok()
        .flatten()
        .and_then(|j| serde_json::from_str::<SamplerStore>(&j).ok())
        .unwrap_or_default();
    if store.quantize {
        set_quantize(app, true);
    }
    set_swing(app, store.swing);
    set_half_time(app, store.half_time);
    {
        let mut seqs = app.sampler.sequences.borrow_mut();
        for (i, s) in store.sequences.into_iter().take(SEQUENCE_PADS).enumerate() {
            seqs[i] = s.map(|mut s| {
                s.patterns.resize(SAMPLER_TOTAL, 0);
                s
            });
        }
    }
    for (slot, info) in store
        .slots
        .into_iter()
        .filter(|(s, i)| *s < SAMPLER_TOTAL && i.path.exists())
    {
        decode(app, slot, info, true);
    }
}

/// Writes the slot assignments (with the engine's mode, gain, sync and
/// pattern) to the library meta.
fn save(app: &App) {
    let state = engine::snapshot(app);
    let slots: Vec<(usize, SlotInfo)> = app
        .sampler
        .slots
        .borrow()
        .iter()
        .enumerate()
        .filter_map(|(slot, i)| {
            let mut info = i.clone()?;
            if let Some(s) = state.as_ref().map(|st| st.sampler[slot]) {
                info.mode = mode_name(s.mode).into();
                info.gain = s.gain;
                info.sync = s.sync;
                info.pattern = s.pattern;
            }
            Some((slot, info))
        })
        .collect();
    let store = SamplerStore {
        slots,
        quantize: app.sampler.quantize.get(),
        swing: app.sampler.swing.get(),
        half_time: app.sampler.half_time.get(),
        sequences: app.sampler.sequences.borrow().to_vec(),
    };
    let json = serde_json::to_string(&store).unwrap_or_default();
    if let Err(e) = app.state.borrow_mut().lib.set_meta(META_SAMPLER, &json) {
        tracing::warn!(error = %e, "sampler not saved");
    }
}

/// Saves once the engine has applied the commands just sent.
pub(crate) fn save_soon(_app: &App) {
    SAVE_DUE.with(|d| d.set(1));
}

thread_local! {
    static SAVE_DUE: Cell<u8> = const { Cell::new(0) };
}

/// [80] Frame tick with audio running, in every mode: the stored slots come
/// back once, and changes are kept (CREATE's drum machine edits the store
/// as Perform does).
pub(crate) fn tick(app: &App) {
    restore(app);
    // Commands are applied with the next audio block: save a few frames on.
    let due = SAVE_DUE.with(|d| {
        let v = d.get();
        if v > 0 {
            d.set(v + 1);
        }
        v >= 4
    });
    if due {
        SAVE_DUE.with(|d| d.set(0));
        save(app);
    }
}

/// Frame tick of the panel with audio running.
pub(crate) fn project(app: &App, state: &EngineState) {
    let rt = &app.sampler;
    let (bank, selected) = (rt.bank.get(), rt.selected.get());
    let slots = rt.slots.borrow();
    rt.pads.sync_eq(pad_views(state, &slots, bank, selected));
    rt.steps
        .sync_eq(pattern_steps(state.sampler[selected].pattern));
    let view = sampler_view(state, &slots, bank, selected);
    drop(slots);
    if app.ui.get_sampler() != view {
        app.ui.set_sampler(view);
    }
}

/// Before audio starts: empty pads, so the panel is complete.
pub(crate) fn show_idle(app: &App) {
    let st = EngineState::default();
    let rt = &app.sampler;
    let slots = rt.slots.borrow();
    rt.pads
        .sync_eq(pad_views(&st, &slots, rt.bank.get(), rt.selected.get()));
    rt.steps.sync_eq(pattern_steps(0));
    app.ui
        .set_sampler(sampler_view(&st, &slots, rt.bank.get(), rt.selected.get()));
}

fn cmd(app: &App, c: SamplerCommand) {
    engine::ensure_engine(app);
    engine::send(app, Command::Sampler(c));
}

/// Opens or closes the sampler panel and remembers it.
pub(crate) fn toggle_panel(app: &App) {
    let on = !app.state.borrow().settings.perform.sampler_panel;
    app.state
        .borrow_mut()
        .change_settings(|s| s.perform.sampler_panel = on);
    app.ui.set_sampler_panel(on);
}

/// Pad of the current bank pressed or released (0..16).
pub(crate) fn pad(app: &App, pad: usize, pressed: bool) {
    let slot = slot_of(app.sampler.bank.get(), pad);
    if pressed {
        app.sampler.selected.set(slot);
    }
    trigger(app, slot, pressed);
}

/// Triggers (press) or releases a slot.
pub(crate) fn trigger(app: &App, slot: usize, pressed: bool) {
    cmd(
        app,
        if pressed {
            SamplerCommand::Trigger { slot }
        } else {
            SamplerCommand::Release { slot }
        },
    );
}

pub(crate) fn set_bank(app: &App, bank: usize) {
    let b = bank.min(SAMPLER_BANKS - 1);
    app.sampler.bank.set(b);
    let sel = app.sampler.selected.get() % SAMPLER_SLOTS;
    app.sampler.selected.set(slot_of(b, sel));
    if engine::snapshot(app).is_none() {
        show_idle(app);
    }
}

pub(crate) fn step_bank_by(app: &App, delta: i32) {
    set_bank(app, step_bank(app.sampler.bank.get(), delta));
}

pub(crate) fn set_quantize(app: &App, on: bool) {
    app.sampler.quantize.set(on);
    cmd(app, SamplerCommand::SetQuantize(on.then_some(1.0)));
}

pub(crate) fn set_volume(app: &App, v: f32) {
    engine::ensure_engine(app);
    engine::send(
        app,
        Command::Mixer(MixerCommand::SetSamplerLevel(volume_to_level(v))),
    );
}

pub(crate) fn toggle_cue(app: &App) {
    let on = engine::snapshot(app)
        .map(|s| s.sampler_cue)
        .unwrap_or(false);
    engine::ensure_engine(app);
    engine::send(app, Command::Mixer(MixerCommand::SetSamplerCue(!on)));
}

/// [80] SWING, `0..=SEQ_SWING_MAX` of a step on the off-beat steps.
pub(crate) fn set_swing(app: &App, swing: f32) {
    let swing = if swing.is_finite() { swing.clamp(0.0, od_engine::SEQ_SWING_MAX) } else { 0.0 };
    app.sampler.swing.set(swing);
    if engine::snapshot(app).is_some() || swing > 0.0 {
        cmd(app, SamplerCommand::SeqSwing(swing));
    }
}

/// [80] HALF TIME: a step lasts an eighth note.
pub(crate) fn set_half_time(app: &App, on: bool) {
    app.sampler.half_time.set(on);
    if engine::snapshot(app).is_some() || on {
        cmd(app, SamplerCommand::SeqHalfTime(on));
    }
}

/// [80] The sequencer's swing and half time as set.
pub(crate) fn feel(app: &App) -> (f32, bool) {
    (app.sampler.swing.get(), app.sampler.half_time.get())
}

pub(crate) fn toggle_seq_play(app: &App) {
    let on = engine::snapshot(app)
        .map(|s| s.sequencer.playing)
        .unwrap_or(false);
    cmd(app, SamplerCommand::SeqPlay(!on));
}

/// What LOAD SELECTED loads: the selected file of the open folder, else the
/// selected track (path, name, BPM).
fn selected_source(app: &App) -> Option<(PathBuf, String, Option<f64>)> {
    let st = app.state.borrow();
    if st.explorer.open.is_some() {
        let i = st.browser.selection.iter().next_back().copied()?;
        let f = st.explorer.files.get(i)?;
        return Some((f.path.clone(), f.title.clone(), f.bpm));
    }
    let id = st.player.current?;
    let t = st.src().track(id).ok()?;
    let name = if t.tags.title.is_empty() {
        t.path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        t.tags.title.clone()
    };
    Some((st.resolve(&t.path), name, t.bpm))
}

/// Loads the selected track (or file) into `slot`.
pub(crate) fn load_selected(app: &App, slot: usize) {
    let Some((path, name, bpm)) = selected_source(app) else {
        app.status("Select a track to load into the sampler");
        return;
    };
    if !engine::ensure_engine(app) {
        engine::show_message(app);
        return;
    }
    app.sampler.selected.set(slot);
    app.status(format!("Loading {name} into the sampler …"));
    // A new sample starts as a one-shot at unity gain (loops of tracks are
    // usually wanted as loops; the mode is one click away).
    let info = SlotInfo {
        path,
        name,
        bpm,
        mode: "oneshot".into(),
        gain: 1.0,
        sync: false,
        pattern: 0,
    };
    decode(app, slot, info, false);
    save_soon(app);
}

/// Empties the selected slot.
pub(crate) fn unload_selected(app: &App) {
    let slot = app.sampler.selected.get();
    cmd(app, SamplerCommand::Unload { slot });
    app.sampler.slots.borrow_mut()[slot] = None;
    save_soon(app);
}

/// Toggles step `step` of the selected slot's pattern.
pub(crate) fn toggle_step(app: &App, step: usize) {
    let slot = app.sampler.selected.get();
    let on = engine::snapshot(app)
        .map(|s| s.sampler[slot].pattern & (1 << step) != 0)
        .unwrap_or(false);
    cmd(
        app,
        SamplerCommand::SeqStep {
            slot,
            step: step.min(SEQ_STEPS - 1),
            on: !on,
        },
    );
    save_soon(app);
}

pub(crate) fn wire(app: &Rc<App>) {
    let ui = &app.ui;
    ui.set_sampler_panel(app.state.borrow().settings.perform.sampler_panel);
    ui.set_sampler_pads(app.sampler.pads.handle());
    ui.set_sampler_steps(app.sampler.steps.handle());
    show_idle(app);
    ui.on_toggle_sampler_panel(handler!(app, |a| toggle_panel(&a)));
    let pad_i = |p: i32| (p.max(0) as usize).min(SAMPLER_SLOTS - 1);
    ui.on_sampler_pad(handler!(app, |a, p: i32, pressed: bool| pad(
        &a,
        pad_i(p),
        pressed
    )));
    ui.on_sampler_select(handler!(app, |a, p: i32| {
        a.sampler
            .selected
            .set(slot_of(a.sampler.bank.get(), pad_i(p)));
        if engine::snapshot(&a).is_none() {
            show_idle(&a);
        }
    }));
    ui.on_sampler_bank(handler!(app, |a, b: i32| set_bank(&a, b.max(0) as usize)));
    ui.on_sampler_load(handler!(app, |a, p: i32| {
        let slot = if p < 0 {
            a.sampler.selected.get()
        } else {
            slot_of(a.sampler.bank.get(), pad_i(p))
        };
        load_selected(&a, slot);
    }));
    ui.on_sampler_unload(handler!(app, |a| unload_selected(&a)));
    ui.on_sampler_stop_all(handler!(app, |a| cmd(&a, SamplerCommand::StopAll)));
    ui.on_sampler_mode(handler!(app, |a, m: i32| {
        cmd(
            &a,
            SamplerCommand::SetMode {
                slot: a.sampler.selected.get(),
                mode: mode_from_index(m),
            },
        );
        save_soon(&a);
    }));
    ui.on_sampler_gain(handler!(app, |a, g: f32| {
        cmd(
            &a,
            SamplerCommand::SetGain {
                slot: a.sampler.selected.get(),
                gain: g.clamp(0.0, 2.0),
            },
        );
        save_soon(&a);
    }));
    ui.on_sampler_sync(handler!(app, |a| {
        let slot = a.sampler.selected.get();
        let on = engine::snapshot(&a)
            .map(|s| s.sampler[slot].sync)
            .unwrap_or(false);
        cmd(&a, SamplerCommand::SetSync { slot, on: !on });
        save_soon(&a);
    }));
    ui.on_sampler_quantize(handler!(app, |a| {
        set_quantize(&a, !a.sampler.quantize.get());
        save_soon(&a);
    }));
    ui.on_sampler_volume(handler!(app, |a, v: f32| set_volume(&a, v)));
    ui.on_sampler_cue(handler!(app, |a| toggle_cue(&a)));
    ui.on_sampler_step(handler!(app, |a, s: i32| toggle_step(
        &a,
        s.max(0) as usize
    )));
    ui.on_sampler_seq_play(handler!(app, |a| toggle_seq_play(&a)));
    ui.on_sampler_seq_record(handler!(app, |a| {
        let on = engine::snapshot(&a)
            .map(|s| s.sequencer.recording)
            .unwrap_or(false);
        cmd(&a, SamplerCommand::SeqRecord(!on));
        save_soon(&a);
    }));
    // The selected slot's tempo halved / doubled (an analysis at the wrong
    // octave would sync it at half or double speed).
    ui.on_sampler_bpm_scale(handler!(app, |a, f: f32| {
        let slot = a.sampler.selected.get();
        let bpm = {
            let mut slots = a.sampler.slots.borrow_mut();
            let Some(Some(info)) = slots.get_mut(slot) else {
                return;
            };
            let Some(b) = info.bpm else { return };
            let b = (b * f as f64).clamp(20.0, 400.0);
            info.bpm = Some(b);
            b
        };
        cmd(
            &a,
            SamplerCommand::SetBpm {
                slot,
                bpm: Some(bpm),
            },
        );
        save_soon(&a);
        if engine::snapshot(&a).is_none() {
            show_idle(&a);
        }
    }));
    // SAVE: the patterns go onto the first free Pattern Recall pad.
    ui.on_sampler_seq_save(handler!(app, |a| {
        let Some(state) = engine::snapshot(&a) else {
            return;
        };
        let free = a
            .sampler
            .sequences
            .borrow()
            .iter()
            .position(|s| s.is_none());
        match free {
            Some(i) => {
                pattern_recall(&a, i, &state);
                save_soon(&a);
                a.status(format!("Sequence stored on Pattern Recall pad {}", i + 1));
            }
            None => a.status("Every Pattern Recall pad holds a sequence: clear one first"),
        }
    }));
    ui.on_sampler_seq_clear(handler!(app, |a| {
        let slot = a.sampler.selected.get();
        cmd(&a, SamplerCommand::SeqPattern { slot, mask: 0 });
        save_soon(&a);
    }));
}

/// Pattern Recall pad `index`: stores the current patterns on an empty pad,
/// recalls them on a stored one (and starts the sequencer).
pub(crate) fn pattern_recall(app: &App, index: usize, state: &od_engine::EngineState) {
    let mut seqs = app.sampler.sequences.borrow_mut();
    let Some(slot) = seqs.get_mut(index) else {
        return;
    };
    match slot.clone() {
        Some(seq) => {
            drop(seqs);
            for (s, mask) in seq.patterns.iter().enumerate().take(SAMPLER_TOTAL) {
                engine::send(
                    app,
                    Command::Sampler(SamplerCommand::SeqPattern {
                        slot: s,
                        mask: *mask,
                    }),
                );
            }
            set_swing(app, seq.swing);
            set_half_time(app, seq.half_time);
            engine::send(app, Command::Sampler(SamplerCommand::SeqPlay(true)));
            save_soon(app);
        }
        None => {
            *slot = Some(Sequence::of(state));
            drop(seqs);
            save_soon(app);
        }
    }
}

/// [80] Stores `seq` on PATTERN pad `index` (replacing what is there).
pub(crate) fn store_sequence(app: &App, index: usize, seq: Sequence) {
    if let Some(s) = app.sampler.sequences.borrow_mut().get_mut(index) {
        *s = Some(seq);
    }
    save_soon(app);
}

/// Clears Pattern Recall pad `index` (the shifted pad).
pub(crate) fn seq_clear(app: &App, index: usize) {
    if let Some(s) = app.sampler.sequences.borrow_mut().get_mut(index) {
        *s = None;
    }
    save_soon(app);
}

/// Stored / playing state of the Pattern Recall pads.
pub(crate) fn seq_pads(app: &App, state: &od_engine::EngineState, rows: &mut [crate::PadView]) {
    let seqs = app.sampler.sequences.borrow();
    let current = Sequence::of(state);
    for r in rows.iter_mut() {
        if let Some(Some(p)) = seqs.get(r.index as usize) {
            r.set = true;
            r.sub = format!("{} STEPS", p.hits()).into();
            r.lit = state.sequencer.playing && p.patterns == current.patterns;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_banks_and_modes() {
        assert_eq!(slot_of(0, 0), 0);
        assert_eq!(slot_of(2, 5), 37);
        assert_eq!(slot_of(9, 99), SAMPLER_TOTAL - 1);
        assert_eq!(step_bank(0, -1), od_engine::DRUMS_BANK, "the DRUMS bank is the last");
        assert_eq!(step_bank(od_engine::DRUMS_BANK, 1), 0);
        assert_eq!(step_bank(od_engine::MELODY_BANK, 1), od_engine::DRUMS_BANK);
        for m in [SampleMode::OneShot, SampleMode::Loop, SampleMode::Gate] {
            assert_eq!(mode_from_name(mode_name(m)), m);
            assert_eq!(mode_from_index(mode_index(m)), m);
        }
        assert_eq!(mode_from_name("?"), SampleMode::OneShot);
    }

    #[test]
    fn volume_is_square_law() {
        assert_eq!(volume_to_level(1.0), 1.0);
        assert_eq!(volume_to_level(0.5), 0.25);
        assert!((level_to_volume(volume_to_level(0.7)) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn pattern_flags() {
        let p = pattern_steps(0b1000_0000_0000_0101);
        assert_eq!(p.len(), 16);
        assert!(p[0] && !p[1] && p[2] && p[15]);
    }

    #[test]
    fn panel_projection() {
        let mut st = EngineState::default();
        st.sampler[17].loaded = true;
        st.sampler[17].playing = true;
        st.sampler[17].mode = SampleMode::Gate;
        st.sampler[17].pattern = 0b11;
        st.sequencer.playing = true;
        st.sequencer.step = 5;
        st.sampler_level = 0.25;
        let mut slots = vec![None; SAMPLER_TOTAL];
        slots[17] = Some(SlotInfo {
            name: "Horn".into(),
            ..Default::default()
        });
        let pads = pad_views(&st, &slots, 1, 17);
        assert_eq!(pads.len(), 16);
        let p = &pads[1];
        assert_eq!((p.slot, p.pad, p.name.as_str(), p.mode), (17, 2, "Horn", 2));
        assert!(p.loaded && p.playing && p.selected);
        assert!(!pads[0].selected && !pads[0].loaded);
        let v = sampler_view(&st, &slots, 1, 17);
        assert_eq!(
            (v.bank, v.selected, v.selected_number, v.step),
            (1, 1, 2, 5)
        );
        assert_eq!(v.selected_name.as_str(), "Horn");
        assert!((v.volume - 0.5).abs() < 1e-6);
        // The selected slot in another bank is not marked in this one.
        assert_eq!(sampler_view(&st, &slots, 0, 17).selected, -1);
        st.sequencer.playing = false;
        assert_eq!(sampler_view(&st, &slots, 1, 17).step, -1);
    }

    #[test]
    fn store_round_trips() {
        let s = SamplerStore {
            slots: vec![(
                3,
                SlotInfo {
                    path: "/s/horn.wav".into(),
                    name: "Horn".into(),
                    bpm: Some(120.0),
                    mode: "loop".into(),
                    gain: 0.8,
                    sync: true,
                    pattern: 5,
                },
            )],
            quantize: true,
            swing: 0.25,
            half_time: true,
            sequences: vec![None, Some(Sequence { patterns: vec![1, 0, 5], swing: 0.5, half_time: false })],
        };
        let back: SamplerStore = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        assert_eq!(
            serde_json::from_str::<SamplerStore>("{}").unwrap(),
            SamplerStore::default()
        );
    }
}
