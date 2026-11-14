//! FX: the Tempo FX panel (two units), Exit FX, the Channel FX selection of
//! the mixer's CH FX knobs and Hold FX presets (SPEC §10.2).
//!
//! The panel is a projection of `EngineState::tempo_fx`; every control sends
//! a `Command::TempoFx`. The Channel FX selection is one global choice (as on
//! the mixer hardware), kept in the settings and applied to every channel.

use crate::app::{App, handler};
use crate::models::Rows;
use crate::settings::CHANNEL_FX;
use crate::{FxSlotView, FxUnitView, decks, engine};
use od_engine::{
    TEMPO_FX_SLOTS, TEMPO_FX_UNITS, TempoFxCommand, TempoFxKind, TempoFxState, ChannelCommand,
    ChannelFxKind, Command, EngineState, FxTail, FxTarget, HoldFxEffect, HoldFxPreset, ExitFxKind, Stem,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

/// Beat lengths of the beat ‹ › buttons, as on the hardware.
pub(crate) const FX_BEATS: [f32; 9] = [0.125, 0.25, 0.5, 0.75, 1.0, 2.0, 4.0, 8.0, 16.0];

/// Exit FX in panel order (BRAKE, BACKSPIN, ECHO OUT).
pub(crate) const EXIT_FX: [ExitFxKind; 3] = [
    ExitFxKind::VinylBrake,
    ExitFxKind::Backspin,
    ExitFxKind::EchoOut,
];

/// Runtime part of the FX panel: the bound unit rows and tap tempo.
#[derive(Default)]
pub(crate) struct FxRuntime {
    pub units: Rows<FxUnitView>,
    /// Slots 2 and 3 per unit; bound once so rows keep their identity.
    slots: [Rows<FxSlotView>; TEMPO_FX_UNITS],
    taps: RefCell<[TapTempo; TEMPO_FX_UNITS]>,
    /// The XY pad switched the unit on: its release switches it off again.
    xy_owned: std::cell::Cell<[bool; TEMPO_FX_UNITS]>,
    /// The unit shows its XY surface.
    xy_mode: std::cell::Cell<[bool; TEMPO_FX_UNITS]>,
}

/// Tap tempo: the average of the last taps; a pause of two seconds starts
/// over.
#[derive(Clone, Debug, Default)]
pub(crate) struct TapTempo {
    taps: Vec<f64>,
}

impl TapTempo {
    /// Registers a tap at `t` seconds; returns the tapped BPM once there are
    /// three taps (40–300 BPM).
    pub fn tap(&mut self, t: f64) -> Option<f32> {
        if self
            .taps
            .last()
            .is_some_and(|&last| t - last > 2.0 || t <= last)
        {
            self.taps.clear();
        }
        self.taps.push(t);
        if self.taps.len() > 8 {
            self.taps.remove(0);
        }
        if self.taps.len() < 3 {
            return None;
        }
        let span = self.taps.last()? - self.taps.first()?;
        let bpm = 60.0 * (self.taps.len() - 1) as f64 / span;
        (40.0..=300.0).contains(&bpm).then_some(bpm as f32)
    }
}

/// "1/8", "3/4", "16" for a beat length.
pub(crate) fn beats_label(b: f32) -> String {
    if (b - 0.75).abs() < 1e-3 {
        "3/4".into()
    } else if b < 1.0 {
        format!("1/{}", (1.0 / b).round())
    } else {
        format!("{}", b.round())
    }
}

/// The next beat length from `current` (`delta` steps through [`FX_BEATS`]).
pub(crate) fn step_beats(current: f32, delta: i32) -> f32 {
    let i = FX_BEATS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - current).abs().total_cmp(&(b.1 - current).abs()))
        .map(|(i, _)| i)
        .unwrap_or(3);
    FX_BEATS[(i as i32 + delta).clamp(0, FX_BEATS.len() as i32 - 1) as usize]
}

/// The effect `delta` places from `kind`, wrapping around the list.
pub(crate) fn step_kind(kind: TempoFxKind, delta: i32) -> TempoFxKind {
    let n = TempoFxKind::ALL.len() as i32;
    TempoFxKind::ALL[(kind.index() as i32 + delta).rem_euclid(n) as usize]
}

/// Tempo FX by a controller mapping name (`echo`, `low_cut_echo`, …). Names
/// of effects OpenDJDecks does not have map to the closest one; a model's
/// own name for one of ours maps to it.
pub(crate) fn kind_from_name(name: &str) -> Option<TempoFxKind> {
    Some(match name {
        "echo" => TempoFxKind::Echo,
        "low_cut_echo" => TempoFxKind::LowCutEcho,
        "delay" => TempoFxKind::Delay,
        "multi_tap" | "multi_tap_delay" => TempoFxKind::MultiTap,
        "ping_pong" => TempoFxKind::PingPong,
        "reverb" | "shimmer" => TempoFxKind::Reverb,
        "flanger" => TempoFxKind::Flanger,
        "jet" | "enigma_jet" => TempoFxKind::Jet,
        "phaser" => TempoFxKind::Phaser,
        "roll" => TempoFxKind::Roll,
        "stretch" => TempoFxKind::Stretch,
        "slip_roll" => TempoFxKind::SlipRoll,
        "filter" => TempoFxKind::Filter,
        "shepard_saw" | "mobius_saw" => TempoFxKind::ShepardSaw,
        "shepard_tri" | "mobius_triangle" => TempoFxKind::ShepardTri,
        "trans" => TempoFxKind::Trans,
        "pitch" => TempoFxKind::Pitch,
        "vinyl_brake" => TempoFxKind::VinylBrake,
        "helix" => TempoFxKind::Helix,
        "spiral" => TempoFxKind::Spiral,
        "build_up" | "riser" | "noise_riser" => TempoFxKind::BuildUp,
        "breakdown" => TempoFxKind::Breakdown,
        "reverse_roll" | "rev_roll" => TempoFxKind::ReverseRoll,
        _ => return None,
    })
}

/// Settings name → engine Channel FX (unknown names: filter).
pub(crate) fn channel_fx_kind(name: &str) -> ChannelFxKind {
    match name {
        "echo" => ChannelFxKind::Echo,
        "noise" => ChannelFxKind::Noise,
        "space" => ChannelFxKind::Space,
        "crush" => ChannelFxKind::Crush,
        "pitch" => ChannelFxKind::Pitch,
        "dub_echo" => ChannelFxKind::DubEcho,
        "sweep" => ChannelFxKind::Sweep,
        "auto" => ChannelFxKind::Auto,
        _ => ChannelFxKind::Filter,
    }
}

/// Index of a Channel FX name in [`CHANNEL_FX`] (the panel order).
pub(crate) fn channel_fx_index(name: &str) -> usize {
    CHANNEL_FX.iter().position(|n| *n == name).unwrap_or(0)
}

/// Target → panel channel number (0 = master).
pub(crate) fn target_number(t: FxTarget) -> i32 {
    match t {
        FxTarget::Master => 0,
        FxTarget::Channel(c) => c as i32 + 1,
        FxTarget::Sampler => -1,
        FxTarget::Part(c, _) => c as i32 + 1,
    }
}

/// Panel channel number → target (−1 = sampler).
pub(crate) fn target_from_number(n: i32) -> FxTarget {
    if n < 0 {
        FxTarget::Sampler
    } else if n == 0 {
        FxTarget::Master
    } else {
        FxTarget::Channel((n as usize - 1).min(decks::DECKS - 1))
    }
}

/// What a unit shows before audio runs (the engine's start values).
pub(crate) fn idle_unit() -> TempoFxState {
    TempoFxState {
        level: 0.5,
        beats: 0.75,
        ..Default::default()
    }
}

/// Parts of the Stem FX keys, in their order.
pub(crate) const STEM_PARTS: [od_engine::Stem; 4] = [
    od_engine::Stem::Vocals,
    od_engine::Stem::Drums,
    od_engine::Stem::Bass,
    od_engine::Stem::Inst,
];

/// The target for Stem FX key `part` (-1 = whole channel) on the unit's channel.
pub(crate) fn part_target(current: FxTarget, part: i32) -> FxTarget {
    let channel = match current {
        FxTarget::Channel(c) | FxTarget::Part(c, _) => c,
        other => return other,
    };
    match STEM_PARTS.get(part.max(-1) as usize).filter(|_| part >= 0) {
        Some(p) => FxTarget::Part(channel, *p),
        None => FxTarget::Channel(channel),
    }
}

/// Projects slots 2 and 3 of a unit (pure).
pub(crate) fn slot_views(slots: &[TempoFxState]) -> Vec<FxSlotView> {
    slots
        .iter()
        .enumerate()
        .map(|(k, s)| FxSlotView {
            slot: k as i32 + 2,
            kind: s.kind.index() as i32,
            beats: beats_label(s.beats).into(),
            level: s.level,
            on: s.on,
        })
        .collect()
}

/// Projects one unit (pure).
pub(crate) fn unit_view(unit: usize, s: &TempoFxState) -> FxUnitView {
    FxUnitView {
        part: match s.target {
            FxTarget::Part(_, p) => STEM_PARTS
                .iter()
                .position(|x| *x == p)
                .map_or(-1, |i| i as i32),
            _ => -1,
        },
        multi: false,
        slots: Default::default(),
        unit: unit as i32 + 1,
        kind: s.kind.index() as i32,
        beats: beats_label(s.beats).into(),
        target: target_number(s.target),
        level: s.level,
        param: s.param,
        on: s.on,
        ringing: s.ringing,
        bpm: if s.bpm > 0.0 {
            format!("{:.1}", s.bpm)
        } else {
            String::new()
        }
        .into(),
        tapped: s.tap_bpm.is_some(),
        release: s
            .release
            .and_then(|r| EXIT_FX.iter().position(|k| *k == r))
            .map(|i| i as i32)
            .unwrap_or(-1),
        xy: false,
        level_text: crate::key_mix::percent(s.level).into(),
        tail: s.tail.index() as i32,
        build_bars: s.build_bars as i32,
        build_progress: s.build_progress,
        build_left: if s.build_bars > 0 && s.build_bars_left >= 0.0 {
            format!("-{}", s.build_bars_left.ceil() as i32)
        } else {
            String::new()
        }
        .into(),
    }
}

/// The tail after `t` (off → echo → reverb → off).
pub(crate) fn next_tail(t: FxTail) -> FxTail {
    FxTail::ALL[(t.index() + 1) % FxTail::ALL.len()]
}

/// XY pad position → (parameter −1..=1, level 0..=1).
pub(crate) fn xy_values(x: f32, y: f32) -> (f32, f32) {
    (x.clamp(0.0, 1.0) * 2.0 - 1.0, y.clamp(0.0, 1.0))
}

/// Hold FX presets of the Hold FX pad modes (page 1 and 2), pads A–H.
pub(crate) fn hold_fx_preset(page: usize, pad: usize) -> HoldFxPreset {
    use TempoFxKind as K;
    let beat = |kind: K, beats: f32, level: f32| HoldFxPreset {
        effect: HoldFxEffect::Beat(kind),
        beats,
        level,
        param: kind.default_param(),
    };
    let release = |kind: ExitFxKind, beats: f32| HoldFxPreset {
        effect: HoldFxEffect::Release(kind),
        beats,
        level: 1.0,
        param: 0.0,
    };
    let presets = [
        [
            beat(K::Echo, 0.5, 0.6),
            beat(K::Echo, 0.75, 0.6),
            beat(K::Reverb, 4.0, 0.7),
            beat(K::Trans, 0.25, 1.0),
            beat(K::Roll, 0.25, 1.0),
            beat(K::Roll, 0.5, 1.0),
            beat(K::Filter, 4.0, 1.0),
            release(ExitFxKind::EchoOut, 1.0),
        ],
        [
            beat(K::PingPong, 0.5, 0.6),
            beat(K::Helix, 1.0, 0.6),
            beat(K::Spiral, 0.5, 0.6),
            beat(K::Flanger, 8.0, 0.8),
            beat(K::SlipRoll, 0.125, 1.0),
            beat(K::Pitch, 1.0, 1.0),
            release(ExitFxKind::VinylBrake, 1.0),
            release(ExitFxKind::Backspin, 1.0),
        ],
    ];
    presets[page.min(1)][pad.min(7)]
}

/// Short pad caption of a Hold FX preset ("ECHO 1/2", "BRAKE").
pub(crate) fn hold_fx_caption(p: &HoldFxPreset) -> String {
    match p.effect {
        HoldFxEffect::Beat(k) => format!("{} {}", kind_short(k), beats_label(p.beats)),
        HoldFxEffect::Release(ExitFxKind::VinylBrake) => "BRAKE".into(),
        HoldFxEffect::Release(ExitFxKind::Backspin) => "BACKSPIN".into(),
        HoldFxEffect::Release(ExitFxKind::EchoOut) => "ECHO OUT".into(),
    }
}

fn kind_short(k: TempoFxKind) -> &'static str {
    use TempoFxKind as K;
    match k {
        K::Echo => "ECHO",
        K::Delay => "DELAY",
        K::PingPong => "P.PONG",
        K::Reverb => "REVERB",
        K::Flanger => "FLANGER",
        K::Phaser => "PHASER",
        K::Roll => "ROLL",
        K::SlipRoll => "S.ROLL",
        K::Filter => "FILTER",
        K::Trans => "TRANS",
        K::Pitch => "PITCH",
        K::VinylBrake => "BRAKE",
        K::Helix => "HELIX",
        K::Spiral => "SPIRAL",
        K::BuildUp => "BUILD",
        K::Breakdown => "BREAK",
        K::ReverseRoll => "REV.ROLL",
        K::LowCutEcho => "LC ECHO",
        K::MultiTap => "M.TAP",
        K::Jet => "JET",
        K::Stretch => "STRETCH",
        K::ShepardSaw => "SHEP.SAW",
        K::ShepardTri => "SHEP.TRI",
    }
}

/// The commands that apply the Channel FX `name` to every channel whose
/// selection differs (empty when all match).
pub(crate) fn channel_fx_commands(state: &EngineState, name: &str) -> Vec<Command> {
    let kind = channel_fx_kind(name);
    (0..state.decks_count.min(decks::DECKS))
        .filter(|&c| state.channels[c].channel_fx != kind)
        .map(|c| Command::Channel(c, ChannelCommand::SetChannelFx(kind)))
        .collect()
}

/// Frame tick: engine state → unit rows; keeps the channels on the selected
/// Channel FX.
pub(crate) fn project(app: &App, state: &EngineState) {
    for u in 0..TEMPO_FX_UNITS {
        app.fx.slots[u].sync_eq(slot_views(&state.tempo_fx_slots[u]));
    }
    app.fx.units.sync_eq(
        (0..TEMPO_FX_UNITS)
            .map(|u| FxUnitView {
                multi: state.tempo_fx_multi[u],
                slots: app.fx.slots[u].handle(),
                xy: app.fx.xy_mode.get()[u],
                ..unit_view(u, &state.tempo_fx[u])
            })
            .collect(),
    );
    let name = app.state.borrow().settings.perform.channel_fx.clone();
    for cmd in channel_fx_commands(state, &name) {
        engine::send(app, cmd);
    }
}

/// Before audio starts: the units at their start values.
pub(crate) fn show_idle(app: &App) {
    if app.fx.units.len() == 0 {
        app.fx.units.sync_eq(
            (0..TEMPO_FX_UNITS)
                .map(|u| unit_view(u, &idle_unit()))
                .collect(),
        );
    }
}

/// Opens or closes the Tempo FX panel and remembers it.
pub(crate) fn toggle_panel(app: &App) {
    let on = !app.state.borrow().settings.perform.fx_panel;
    app.state
        .borrow_mut()
        .change_settings(|s| s.perform.fx_panel = on);
    app.ui.set_fx_panel(on);
}

/// Current state of unit `u` (0-based), if audio runs.
fn unit_state(app: &App, u: usize) -> Option<TempoFxState> {
    engine::snapshot(app).map(|s| s.tempo_fx[u.min(TEMPO_FX_UNITS - 1)])
}

/// Sends a command to unit `u` (0-based), starting audio first.
pub(crate) fn send(app: &App, u: usize, cmd: TempoFxCommand) {
    engine::ensure_engine(app);
    engine::send(app, Command::TempoFx(u.min(TEMPO_FX_UNITS - 1), cmd));
}

pub(crate) fn toggle_on(app: &App, u: usize) {
    let on = unit_state(app, u).map(|s| s.on).unwrap_or(false);
    send(app, u, TempoFxCommand::SetOn(!on));
}

/// Slot 2 or 3 of unit `u` (0-based).
fn slot_state(app: &App, u: usize, slot: usize) -> Option<TempoFxState> {
    engine::snapshot(app).and_then(|s| {
        s.tempo_fx_slots[u.min(TEMPO_FX_UNITS - 1)]
            .get(slot.wrapping_sub(2))
            .copied()
    })
}

fn send_slot(app: &App, u: usize, slot: usize, cmd: TempoFxCommand) {
    engine::ensure_engine(app);
    if (2..=TEMPO_FX_SLOTS).contains(&slot) {
        engine::send(
            app,
            Command::TempoFxSlot {
                unit: u,
                slot: slot - 1,
                cmd,
            },
        );
    }
}

/// Multi mode on / off for unit `u` (0-based).
pub(crate) fn toggle_multi(app: &App, u: usize) {
    engine::ensure_engine(app);
    let on = engine::snapshot(app).is_some_and(|s| s.tempo_fx_multi[u.min(TEMPO_FX_UNITS - 1)]);
    engine::send(app, Command::SetTempoFxMulti(u, !on));
}

pub(crate) fn step_unit_kind(app: &App, u: usize, delta: i32) {
    let kind = unit_state(app, u).map(|s| s.kind).unwrap_or_default();
    send(app, u, TempoFxCommand::SetKind(step_kind(kind, delta)));
}

pub(crate) fn step_unit_beats(app: &App, u: usize, delta: i32) {
    let beats = unit_state(app, u).map(|s| s.beats).unwrap_or(0.75);
    send(app, u, TempoFxCommand::SetBeats(step_beats(beats, delta)));
}

/// TAP: every tap refines the tapped tempo of unit `u`.
pub(crate) fn tap(app: &App, u: usize) {
    let t = TAP_EPOCH.with(|e| e.elapsed().as_secs_f64());
    let bpm = app.fx.taps.borrow_mut()[u.min(TEMPO_FX_UNITS - 1)].tap(t);
    if let Some(bpm) = bpm {
        send(app, u, TempoFxCommand::SetTapBpm(Some(bpm)));
    }
}

/// AUTO: back to the tempo of the decks.
pub(crate) fn auto_beat(app: &App, u: usize) {
    app.fx.taps.borrow_mut()[u.min(TEMPO_FX_UNITS - 1)] = TapTempo::default();
    send(app, u, TempoFxCommand::SetTapBpm(None));
}

/// Exit FX `kind` on unit `u` while `on`, lasting the unit's beat length.
pub(crate) fn release(app: &App, u: usize, kind: ExitFxKind, on: bool) {
    let beats = unit_state(app, u).map(|s| s.beats).unwrap_or(1.0).max(0.5);
    send(app, u, TempoFxCommand::Release { kind, beats, on });
}

/// Panic: every effect off at once (Tempo FX with slots and tails, exit
/// and Hold FX, the channel FX knobs).
pub(crate) fn panic(app: &App) {
    engine::ensure_engine(app);
    engine::send(app, Command::FxPanic);
    app.fx.xy_owned.set([false; TEMPO_FX_UNITS]);
    app.status(od_i18n::tr("All effects off"));
}

/// A build-up of `bars` (8, 16, 32) on unit `u`; the same length again
/// stops it.
pub(crate) fn build_up(app: &App, u: usize, bars: u8) {
    let running = unit_state(app, u).map_or(0, |s| s.build_bars);
    let bars = if running == bars { 0 } else { bars };
    send(app, u, TempoFxCommand::BuildUp { bars });
}

/// TAIL: the next of off, echo and reverb.
pub(crate) fn step_tail(app: &App, u: usize) {
    let t = unit_state(app, u).map(|s| s.tail).unwrap_or_default();
    send(app, u, TempoFxCommand::SetTail(next_tail(t)));
}

/// XY pad of unit `u`: across the parameter, up the level; `phase` 0 press
/// (switches an idle unit on), 1 move, 2 release (switches off what the
/// press switched on).
pub(crate) fn xy(app: &App, u: usize, x: f32, y: f32, phase: i32) {
    let u = u.min(TEMPO_FX_UNITS - 1);
    let (param, level) = xy_values(x, y);
    send(app, u, TempoFxCommand::SetParam(param));
    send(app, u, TempoFxCommand::SetLevel(level));
    let mut owned = app.fx.xy_owned.get();
    match phase {
        0 if !unit_state(app, u).is_some_and(|s| s.on) => {
            owned[u] = true;
            send(app, u, TempoFxCommand::SetOn(true));
        }
        2 if owned[u] => {
            owned[u] = false;
            send(app, u, TempoFxCommand::SetOn(false));
        }
        _ => {}
    }
    app.fx.xy_owned.set(owned);
}

/// Shows unit `u`'s XY surface instead of its controls (or back).
pub(crate) fn set_xy_mode(app: &App, u: usize, on: bool) {
    let mut m = app.fx.xy_mode.get();
    m[u.min(TEMPO_FX_UNITS - 1)] = on;
    app.fx.xy_mode.set(m);
    // Before audio runs nothing projects: show it now.
    if engine::snapshot(app).is_none() {
        app.fx.units.sync_eq(
            (0..TEMPO_FX_UNITS)
                .map(|k| FxUnitView { xy: m[k], ..unit_view(k, &idle_unit()) })
                .collect(),
        );
    }
}

/// Selects the Channel FX of the CH FX knobs (index into [`CHANNEL_FX`]).
pub(crate) fn select_channel_fx(app: &App, index: usize) {
    let name = CHANNEL_FX[index.min(CHANNEL_FX.len() - 1)];
    app.state
        .borrow_mut()
        .change_settings(|s| s.perform.channel_fx = name.to_string());
    app.ui.set_channel_fx(index as i32);
    engine::ensure_engine(app);
    for c in 0..decks::DECKS {
        engine::send(
            app,
            Command::Channel(c, ChannelCommand::SetChannelFx(channel_fx_kind(name))),
        );
    }
}

/// The part the Channel FX goes on after pressing `part`'s button: that
/// part, or the whole channel again when it is on that part already.
pub(crate) fn next_channel_fx_part(current: Option<Stem>, part: Stem) -> Option<Stem> {
    (current != Some(part)).then_some(part)
}

/// A controller's FX part button: the Channel FX of every channel on
/// `part` (needs stems), or back on the whole channels.
pub(crate) fn toggle_channel_fx_part(app: &App, part: Stem) {
    let current = engine::snapshot(app).and_then(|s| s.channels[0].channel_fx_part);
    let next = next_channel_fx_part(current, part);
    engine::ensure_engine(app);
    for c in 0..decks::DECKS {
        engine::send(app, Command::Channel(c, ChannelCommand::SetChannelFxPart(next)));
    }
}

pub(crate) fn step_channel_fx(app: &App, delta: i32) {
    let cur = channel_fx_index(&app.state.borrow().settings.perform.channel_fx) as i32;
    select_channel_fx(
        app,
        (cur + delta).rem_euclid(CHANNEL_FX.len() as i32) as usize,
    );
}

thread_local! {
    static TAP_EPOCH: Instant = Instant::now();
}

/// Documentation screenshots (`OPENDJDECKS_DEMO_PROOF=fx-drop`): the FX
/// panel open (not saved); FX1 on the master runs a 16-bar build-up with a
/// reverb tail (started again whenever it reaches its drop), FX2 on
/// channel 2 shows its XY surface with an echo on.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    if !v.split(',').any(|p| p == "fx-drop") {
        return;
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(5), move || {
        let Some(a) = weak.upgrade() else { return };
        a.ui.set_fx_panel(true);
        send(&a, 0, TempoFxCommand::SetTarget(FxTarget::Master));
        send(&a, 0, TempoFxCommand::SetLevel(0.8));
        send(&a, 0, TempoFxCommand::SetTail(FxTail::Reverb));
        send(&a, 1, TempoFxCommand::SetTarget(FxTarget::Channel(1)));
        send(&a, 1, TempoFxCommand::SetKind(TempoFxKind::Echo));
        send(&a, 1, TempoFxCommand::SetParam(0.35));
        send(&a, 1, TempoFxCommand::SetLevel(0.65));
        send(&a, 1, TempoFxCommand::SetOn(true));
        set_xy_mode(&a, 1, true);
        let w = Rc::downgrade(&a);
        let again = Rc::new(slint::Timer::default());
        let keep = again.clone();
        again.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(500), move || {
            let _ = &keep;
            let Some(a) = w.upgrade() else { return };
            if unit_state(&a, 0).is_some_and(|s| s.build_bars == 0) {
                send(&a, 0, TempoFxCommand::BuildUp { bars: 16 });
            }
        });
    });
}

pub(crate) fn wire(app: &Rc<App>) {
    demo(app);
    let ui = &app.ui;
    ui.set_fx_panel(app.state.borrow().settings.perform.fx_panel);
    ui.set_fx_units(app.fx.units.handle());
    ui.set_channel_fx(channel_fx_index(&app.state.borrow().settings.perform.channel_fx) as i32);
    show_idle(app);
    ui.on_toggle_fx_panel(handler!(app, |a| toggle_panel(&a)));
    // Units are 1-based in the UI.
    let u = |n: i32| (n as usize).saturating_sub(1);
    ui.on_fx_kind(handler!(app, |a, n: i32, k: i32| send(
        &a,
        u(n),
        TempoFxCommand::SetKind(TempoFxKind::ALL[(k.max(0) as usize).min(TempoFxKind::ALL.len() - 1)])
    )));
    let slots = slint::ComponentHandle::global::<crate::FxSlots>(&app.ui);
    slots.on_multi(handler!(app, |a, n: i32| toggle_multi(&a, u(n))));
    slots.on_auto(handler!(app, |a, n: i32| auto_beat(&a, u(n))));
    slots.on_build(handler!(app, |a, n: i32, bars: i32| build_up(&a, u(n), bars.clamp(0, 32) as u8)));
    slots.on_tail(handler!(app, |a, n: i32| step_tail(&a, u(n))));
    slots.on_xy(handler!(app, |a, n: i32, x: f32, y: f32, phase: i32| xy(&a, u(n), x, y, phase)));
    slots.on_panic(handler!(app, |a| panic(&a)));
    slots.on_xy_mode(handler!(app, |a, n: i32, on: bool| set_xy_mode(&a, u(n), on)));
    slots.on_part(handler!(app, |a, n: i32, p: i32| {
        let current = unit_state(&a, u(n)).map(|s| s.target).unwrap_or_default();
        send(&a, u(n), TempoFxCommand::SetTarget(part_target(current, p)));
    }));
    slots.on_kind_step(handler!(app, |a, n: i32, k: i32, d: i32| {
        let kind = slot_state(&a, u(n), k as usize)
            .map(|s| s.kind)
            .unwrap_or_default();
        send_slot(
            &a,
            u(n),
            k as usize,
            TempoFxCommand::SetKind(step_kind(kind, d)),
        );
    }));
    slots.on_beats_step(handler!(app, |a, n: i32, k: i32, d: i32| {
        let beats = slot_state(&a, u(n), k as usize)
            .map(|s| s.beats)
            .unwrap_or(0.75);
        send_slot(
            &a,
            u(n),
            k as usize,
            TempoFxCommand::SetBeats(step_beats(beats, d)),
        );
    }));
    slots.on_level(handler!(app, |a, n: i32, k: i32, v: f32| send_slot(
        &a,
        u(n),
        k as usize,
        TempoFxCommand::SetLevel(v)
    )));
    slots.on_on(handler!(app, |a, n: i32, k: i32| {
        let on = slot_state(&a, u(n), k as usize).is_some_and(|s| s.on);
        send_slot(&a, u(n), k as usize, TempoFxCommand::SetOn(!on));
    }));
    ui.on_fx_kind_step(handler!(app, |a, n: i32, d: i32| step_unit_kind(
        &a,
        u(n),
        d
    )));
    ui.on_fx_beats_step(handler!(app, |a, n: i32, d: i32| step_unit_beats(
        &a,
        u(n),
        d
    )));
    ui.on_fx_target(handler!(app, |a, n: i32, t: i32| send(
        &a,
        u(n),
        TempoFxCommand::SetTarget(target_from_number(t))
    )));
    ui.on_fx_level(handler!(app, |a, n: i32, v: f32| send(
        &a,
        u(n),
        TempoFxCommand::SetLevel(v)
    )));
    ui.on_fx_param(handler!(app, |a, n: i32, v: f32| send(
        &a,
        u(n),
        TempoFxCommand::SetParam(v)
    )));
    ui.on_fx_on(handler!(app, |a, n: i32| toggle_on(&a, u(n))));
    ui.on_fx_tap(handler!(app, |a, n: i32| tap(&a, u(n))));
    ui.on_fx_release(handler!(app, |a, n: i32, k: i32, on: bool| release(
        &a,
        u(n),
        EXIT_FX[(k.max(0) as usize).min(2)],
        on
    )));
    ui.on_channel_fx_select(handler!(app, |a, i: i32| select_channel_fx(
        &a,
        i.max(0) as usize
    )));
    ui.on_channel_fx_step(handler!(app, |a, d: i32| step_channel_fx(&a, d)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beat_lengths_step_and_print() {
        assert_eq!(step_beats(0.75, 1), 1.0);
        assert_eq!(step_beats(0.75, -1), 0.5);
        assert_eq!(step_beats(16.0, 1), 16.0);
        assert_eq!(step_beats(0.125, -1), 0.125);
        // A value between steps snaps to the nearest one first.
        assert_eq!(step_beats(3.0, 1), 4.0);
        assert_eq!(beats_label(0.125), "1/8");
        assert_eq!(beats_label(0.75), "3/4");
        assert_eq!(beats_label(4.0), "4");
    }

    #[test]
    fn kinds_wrap_and_map_controller_names() {
        assert_eq!(step_kind(TempoFxKind::Echo, -1), TempoFxKind::ShepardTri);
        assert_eq!(step_kind(TempoFxKind::ShepardTri, 1), TempoFxKind::Echo);
        assert_eq!(step_kind(TempoFxKind::Echo, 3), TempoFxKind::Reverb);
        // The four-deck mapping's selector: every position an effect of its own.
        assert_eq!(kind_from_name("low_cut_echo"), Some(TempoFxKind::LowCutEcho));
        assert_eq!(kind_from_name("multi_tap_delay"), Some(TempoFxKind::MultiTap));
        assert_eq!(kind_from_name("enigma_jet"), Some(TempoFxKind::Jet));
        assert_eq!(kind_from_name("stretch"), Some(TempoFxKind::Stretch));
        assert_eq!(kind_from_name("mobius_saw"), Some(TempoFxKind::ShepardSaw));
        assert_eq!(kind_from_name("mobius_triangle"), Some(TempoFxKind::ShepardTri));
        assert_eq!(kind_from_name("slip_roll"), Some(TempoFxKind::SlipRoll));
        assert_eq!(kind_from_name("vinyl_brake"), Some(TempoFxKind::VinylBrake));
        assert_eq!(kind_from_name("laser"), None);
    }

    /// A part button puts the Channel FX on its part; pressed again (or
    /// another part's button) it moves or goes back to the whole channel.
    #[test]
    fn channel_fx_part_buttons_toggle() {
        assert_eq!(next_channel_fx_part(None, Stem::Drums), Some(Stem::Drums));
        assert_eq!(next_channel_fx_part(Some(Stem::Drums), Stem::Drums), None);
        assert_eq!(next_channel_fx_part(Some(Stem::Drums), Stem::Vocals), Some(Stem::Vocals));
    }

    #[test]
    fn tap_tempo_averages_and_restarts() {
        let mut t = TapTempo::default();
        assert_eq!(t.tap(0.0), None);
        assert_eq!(t.tap(0.5), None);
        assert_eq!(t.tap(1.0), Some(120.0));
        let bpm = t.tap(1.48).unwrap();
        assert!((bpm - 121.6).abs() < 0.1, "{bpm}");
        // A pause starts over.
        assert_eq!(t.tap(5.0), None);
    }

    #[test]
    fn unit_projection() {
        let s = TempoFxState {
            kind: TempoFxKind::Reverb,
            target: FxTarget::Channel(1),
            on: true,
            level: 0.7,
            beats: 2.0,
            param: -0.5,
            tap_bpm: Some(128.0),
            bpm: 128.0,
            release: Some(ExitFxKind::Backspin),
            ringing: false,
            tail: FxTail::Reverb,
            build_bars: 16,
            build_progress: 0.5,
            build_bars_left: 5.2,
        };
        let v = unit_view(1, &s);
        assert_eq!((v.unit, v.kind, v.target, v.release), (2, 3, 2, 1));
        assert_eq!((v.level_text.as_str(), v.tail, v.build_bars, v.build_left.as_str()), ("70 %", 2, 16, "-6"));
        assert_eq!(next_tail(FxTail::Reverb), FxTail::Off);
        assert_eq!(next_tail(FxTail::Off), FxTail::Echo);
        assert_eq!(xy_values(0.5, 0.25), (0.0, 0.25));
        assert_eq!(xy_values(-1.0, 2.0), (-1.0, 1.0));
        assert_eq!((v.beats.as_str(), v.bpm.as_str()), ("2", "128.0"));
        assert!(v.on && v.tapped);
        let idle = unit_view(0, &idle_unit());
        assert_eq!(
            (
                idle.target,
                idle.release,
                idle.beats.as_str(),
                idle.bpm.as_str()
            ),
            (0, -1, "3/4", "")
        );
        assert_eq!(
            target_from_number(target_number(FxTarget::Channel(3))),
            FxTarget::Channel(3)
        );
        assert_eq!(target_from_number(0), FxTarget::Master);
    }

    #[test]
    fn channel_fx_follows_the_selection() {
        let mut st = EngineState {
            decks_count: 2,
            ..Default::default()
        };
        assert!(channel_fx_commands(&st, "filter").is_empty());
        let cmds = channel_fx_commands(&st, "dub_echo");
        assert_eq!(cmds.len(), 2);
        assert!(matches!(
            cmds[0],
            Command::Channel(0, ChannelCommand::SetChannelFx(ChannelFxKind::DubEcho))
        ));
        st.channels[0].channel_fx = ChannelFxKind::DubEcho;
        assert_eq!(channel_fx_commands(&st, "dub_echo").len(), 1);
        for (i, n) in CHANNEL_FX.iter().enumerate() {
            assert_eq!(channel_fx_index(n), i);
        }
        assert_eq!(channel_fx_kind("sweep"), ChannelFxKind::Sweep);
        assert_eq!(channel_fx_kind("nope"), ChannelFxKind::Filter);
    }

    #[test]
    fn hold_fx_presets_cover_two_pages() {
        let p = hold_fx_preset(0, 0);
        assert_eq!(p.effect, HoldFxEffect::Beat(TempoFxKind::Echo));
        assert_eq!(hold_fx_caption(&p), "ECHO 1/2");
        assert_eq!(hold_fx_caption(&hold_fx_preset(1, 7)), "BACKSPIN");
        // Out-of-range pads clamp.
        assert_eq!(hold_fx_preset(5, 12), hold_fx_preset(1, 7));
    }
}
