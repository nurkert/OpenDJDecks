//! Controllers (MIDI): mapped actions → engine commands, and LED feedback.

use crate::app::App;
use crate::{browser, decks, engine, fx, recorder, sampler};
use od_controller::io::{ControllerEvent, ControllerManager};
use od_controller::{
    TempoFxAction, ChannelFxAction, ControlAction, ControlFeedback, CrossfaderSide, DeckAction,
    DeckElement, FeedbackTarget, LedState, MixerAction, MixerElement, PadMode, SamplerAction,
    StemPart,
};
use od_engine::{
    TEMPO_FX_UNITS, TempoFxCommand, ChannelCommand, Command, DeckCommand, JogMode, MemoryCueJump,
    MixerCommand, ExitFxKind, SamplerCommand, Stem, TempoRange, XfAssign,
};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct Controllers {
    manager: Option<ControllerManager>,
    /// Name of the connected controller.
    pub name: Option<String>,
    /// [34] The MIDI input given to the lights, and its port while it is
    /// connected.
    lighting_port: String,
    lighting_connected: Option<String>,
}

/// Mapping id of the default layout for a lighting controller
/// (`mappings/lighting/lighting-operator.toml`).
pub(crate) const LIGHTING_LAYOUT: &str = "lighting-operator";

/// [34] Gives the MIDI input `port` to the lights (the lighting layout),
/// and the one given before back to its own mapping. "" gives it back.
pub(crate) fn assign_lighting(app: &App, port: &str) {
    let mut c = app.controller.borrow_mut();
    if c.lighting_port == port {
        return;
    }
    let before = std::mem::replace(&mut c.lighting_port, port.to_string());
    if let Some(m) = c.manager.as_ref() {
        if !before.is_empty() {
            m.assign(&before, None);
        }
        if !port.is_empty() {
            m.assign(port, Some(LIGHTING_LAYOUT));
        }
    }
    if c.lighting_connected.as_deref() != Some(port) {
        c.lighting_connected = None;
    }
}

/// [34] The port of the lighting controller while it is connected.
pub(crate) fn lighting_connected(app: &App) -> Option<String> {
    app.controller.borrow().lighting_connected.clone()
}

/// MIDI inputs present now (for choosing the lighting controller).
pub(crate) fn midi_inputs(app: &App) -> Vec<String> {
    if app.state.borrow().config.headless {
        return Vec::new();
    }
    let mut v = od_controller::io::list_ports()
        .map(|p| p.inputs)
        .unwrap_or_default();
    v.retain(|p| !p.starts_with("OpenDJDecks"));
    v.sort();
    v.dedup();
    v
}


/// Directory with controller mapping files.
fn mappings_dir() -> PathBuf {
    if let Some(d) = od_core::env::var_os("MAPPINGS") {
        return PathBuf::from(d);
    }
    if let Ok(exe) = std::env::current_exe() {
        for anc in exe.ancestors().skip(1).take(4) {
            let p = anc.join("mappings");
            if p.is_dir() {
                return p;
            }
            for sub in [
                "Resources/mappings",
                "share/opendjdecks/mappings",
            ] {
                let p = anc.join(sub);
                if p.is_dir() {
                    return p;
                }
            }
        }
    }
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mappings"))
}

/// Names of the built-in mapping files (Preferences > Controller).
pub(crate) fn mapping_names() -> Vec<String> {
    let mut v: Vec<String> = od_controller::Mapping::load_dir(&mappings_dir())
        .into_iter()
        .map(|(path, m)| {
            let file = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            match m {
                Ok(m) => format!("{} — {file}", m.file.device.name),
                Err(e) => format!("{file} — {e}"),
            }
        })
        .collect();
    v.sort();
    v
}

/// The mappings the self-test can check: the own mapping file first, then
/// the built-in ones (those that load).
pub(crate) fn test_mappings(app: &App) -> Vec<Arc<od_controller::Mapping>> {
    let own = app.state.borrow().settings.controller.mapping_file.clone();
    let mut files = od_controller::Mapping::load_dir(&mappings_dir());
    files.sort_by(|a, b| a.0.cmp(&b.0));
    if !own.is_empty() {
        let p = PathBuf::from(&own);
        files.insert(0, (p.clone(), od_controller::Mapping::load(&p)));
    }
    files.into_iter().filter_map(|(_, m)| m.ok().map(Arc::new)).collect()
}

/// Self-test: every LED of `device`'s mapping on, or back to normal.
pub(crate) fn light_all(app: &App, device: &str, on: bool) {
    if let Some(m) = app.controller.borrow().manager.as_ref() {
        m.light_all(Some(device), on);
    }
}

/// MIDI learn, the monitor and the guided setup need the controllers no
/// mapping knows: their ports are open while one of them runs. [82] So
/// does CREATE, for a MIDI keyboard: while it is shown.
pub(crate) fn refresh_unmapped(app: &App) {
    let want = wants_unmapped(
        {
            let p = app.prefs.borrow();
            p.midi.learning || p.midi.monitoring
        },
        app.controller_setup.borrow().active(),
        app.ui.get_mode().as_str(),
    );
    if let Some(m) = app.controller.borrow().manager.as_ref() {
        m.set_open_unmapped(want);
    }
}

/// Whether the MIDI inputs no mapping knows are open: while MIDI learn or
/// the monitor (`learning`) or the guided setup runs, or CREATE is the mode.
pub(crate) fn wants_unmapped(learning: bool, setup: bool, mode: &str) -> bool {
    learning || setup || mode == "create"
}

/// Whether the mapping of `device` (by id) has physical channel faders.
fn has_channel_faders(app: &App, device: &str) -> bool {
    test_mappings(app).iter().any(|m| {
        m.device().id == device && m.inputs.iter().any(|b| b.template.name == "mixer.fader")
    })
}

/// Mapping id of the connected controller.
pub(crate) fn connected_id(app: &App) -> Option<String> {
    app.controller.borrow().name.clone()
}

/// Starts the controller layer (MIDI) once.
pub(crate) fn start(app: &Rc<App>) {
    let mut c = app.controller.borrow_mut();
    if c.manager.is_some() {
        return;
    }
    let own = app.state.borrow().settings.controller.mapping_file.clone();
    let mut files = od_controller::Mapping::load_dir(&mappings_dir());
    if !own.is_empty() {
        // The user's file first, so it wins for its device.
        let p = PathBuf::from(&own);
        files.insert(0, (p.clone(), od_controller::Mapping::load(&p)));
    }
    let mappings: Vec<Arc<od_controller::Mapping>> = files
        .into_iter()
        .filter_map(|(path, m)| match m {
            Ok(m) => Some(Arc::new(m)),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "mapping failed to load");
                None
            }
        })
        .collect();
    let manager = ControllerManager::start(mappings, Default::default());
    if !c.lighting_port.is_empty() {
        manager.assign(&c.lighting_port, Some(LIGHTING_LAYOUT));
    }
    c.manager = Some(manager);
}

/// The controller layer's MIDI monitor (Preferences ▸ Controller: monitor
/// and learn), once the layer runs.
pub(crate) fn monitor(app: &App) -> Option<od_controller::monitor::MidiMonitor> {
    app.controller
        .borrow()
        .manager
        .as_ref()
        .map(|m| m.monitor().clone())
}

/// Frame tick: controller events. [82] The inputs no mapping knows follow
/// the mode (open while CREATE is shown).
pub(crate) fn poll(app: &App) {
    refresh_unmapped(app);
    let events: Vec<ControllerEvent> = app
        .controller
        .borrow()
        .manager
        .as_ref()
        .map(|c| c.events().try_iter().collect())
        .unwrap_or_default();
    for e in events {
        handle_event(app, e);
    }
}

/// One event of the controller layer.
pub(crate) fn handle_event(app: &App, e: ControllerEvent) {
    match e {
        ControllerEvent::Action { action, .. } => {
            engine::ensure_engine(app);
            handle_action(app, action);
        }
        // [82] A note from a MIDI input without a mapping: CREATE's.
        ControllerEvent::Note { port, note, .. } => crate::create::midi_note(app, &port, note),
        ControllerEvent::Connected { device, port, .. } if device == LIGHTING_LAYOUT => {
            app.status(od_i18n::trf("Lighting controller connected: {}", &[&port]));
            app.controller.borrow_mut().lighting_connected = Some(port);
            crate::lighting::refresh(app);
        }
        ControllerEvent::Disconnected { device, port } if device == LIGHTING_LAYOUT => {
            app.status(od_i18n::trf("Lighting controller disconnected: {}", &[&port]));
            app.controller.borrow_mut().lighting_connected = None;
            crate::lighting::refresh(app);
        }
        ControllerEvent::Connected { device, .. } => {
            if device.contains("FLX10") || device.contains("flx10") {
                app.decks.borrow_mut().deck_count = 4;
            }
            // [H1] The app cannot know where the physical channel faders are
            // until they move: its channels close instead of playing at the
            // level the software had, and each fader sets its channel on its
            // first move (soft takeover lets the first position through).
            if has_channel_faders(app, &device) && engine::ensure_engine(app) {
                for c in 0..decks::DECKS {
                    engine::send(app, Command::Channel(c, ChannelCommand::SetFader(0.0)));
                }
                app.status(od_i18n::trf(
                    "Controller connected: {}. Channels stay silent until you move their faders",
                    &[&device],
                ));
            } else {
                app.status(format!("Controller connected: {device}"));
            }
            app.controller.borrow_mut().name = Some(device);
        }
        ControllerEvent::Disconnected { device, .. } => {
            app.status(format!("Controller disconnected: {device}"));
            app.controller.borrow_mut().name = None;
        }
        // A MIDI device without a mapping: say once how to make it a DJ
        // controller. Not in CREATE, where keyboards are welcome as they are,
        // and not for the input given to the lights.
        ControllerEvent::Unmapped { port } => {
            let lights = app.controller.borrow().lighting_port == port;
            if !lights && app.ui.get_mode().as_str() != "create" {
                app.status(od_i18n::trf(
                    "{} has no controller mapping: set it up with View › Controller Setup",
                    &[&port],
                ));
            }
        }
        ControllerEvent::Error(e) => tracing::warn!(error = %e, "controller"),
    }
}

pub(crate) fn next_tempo_range(r: TempoRange) -> TempoRange {
    match r {
        TempoRange::Six => TempoRange::Ten,
        TempoRange::Ten => TempoRange::Sixteen,
        TempoRange::Sixteen => TempoRange::Wide,
        TempoRange::Wide => TempoRange::Six,
    }
}

/// Trim knob (0..1) → dB (−24..+12).
fn trim_db(v: f32) -> f32 {
    crate::mixer::knob_to_trim(v * 2.0 - 1.0)
}

pub(crate) fn handle_action(app: &App, action: ControlAction) {
    match action {
        ControlAction::Deck { deck, action } => deck_action(
            app,
            (deck as usize).saturating_sub(1).min(decks::DECKS - 1),
            action,
        ),
        ControlAction::Mixer { channel, action } => {
            let c = (channel as usize).saturating_sub(1).min(decks::DECKS - 1);
            let cmd = match action {
                MixerAction::Trim(v) => ChannelCommand::SetTrim(trim_db(v)),
                MixerAction::EqHigh(v) => ChannelCommand::SetEq {
                    band: 2,
                    value: v * 2.0 - 1.0,
                },
                MixerAction::EqMid(v) => ChannelCommand::SetEq {
                    band: 1,
                    value: v * 2.0 - 1.0,
                },
                MixerAction::EqLow(v) => ChannelCommand::SetEq {
                    band: 0,
                    value: v * 2.0 - 1.0,
                },
                MixerAction::ChannelFx(v) => ChannelCommand::SetChannelFxAmount(v),
                MixerAction::EqKill(band) => {
                    crate::mixer::toggle_kill(app, c, band.index());
                    return;
                }
                MixerAction::Fader(v) => ChannelCommand::SetFader(v),
                MixerAction::CueToggle => {
                    let on = engine::snapshot(app)
                        .map(|s| s.channels[c].cue)
                        .unwrap_or(false);
                    ChannelCommand::SetCue(!on)
                }
                MixerAction::CrossfaderAssign(side) => {
                    ChannelCommand::SetCrossfaderAssign(match side {
                        CrossfaderSide::A => XfAssign::A,
                        CrossfaderSide::B => XfAssign::B,
                        CrossfaderSide::Thru => XfAssign::Thru,
                    })
                }
            };
            engine::send(app, Command::Channel(c, cmd));
        }
        ControlAction::Crossfader(v) => engine::send(
            app,
            Command::Mixer(MixerCommand::SetCrossfader(v * 2.0 - 1.0)),
        ),
        ControlAction::MasterLevel(v) => {
            engine::send(app, Command::Mixer(MixerCommand::SetMasterLevel(v)))
        }
        ControlAction::HeadphoneMix(v) => {
            engine::send(app, Command::Mixer(MixerCommand::SetHeadphoneMix(v)))
        }
        ControlAction::HeadphoneLevel(v) => {
            engine::send(app, Command::Mixer(MixerCommand::SetHeadphoneLevel(v)))
        }
        ControlAction::Browse(b) => browser::navigate::controller(app, &b),
        ControlAction::RecordingToggle => recorder::toggle_from_controller(app),
        ControlAction::Lighting(l) => crate::lighting::controller(app, l),
        ControlAction::BoothLevel(v) => {
            engine::send(app, Command::Mixer(MixerCommand::SetBoothLevel(v)))
        }
        ControlAction::EqSwap(band) => crate::mixer::swap(app, band.index()),
        ControlAction::FxPanic => fx::panic(app),
        ControlAction::SplitCue => crate::mixer::toggle_split_cue(app),
        ControlAction::CrossfaderToggle => crate::mixer::toggle_crossfader(app),
        ControlAction::CrossfaderCurve(c) => crate::mixer::set_crossfader_curve(app, c.as_deref()),
        ControlAction::MasterCue => {
            let on = engine::snapshot(app).map(|s| s.master_cue).unwrap_or(false);
            engine::send(app, Command::Mixer(MixerCommand::SetMasterCue(!on)));
        }
        ControlAction::TempoFx { unit, action } => tempo_fx_action(
            app,
            (unit as usize).saturating_sub(1).min(TEMPO_FX_UNITS - 1),
            action,
        ),
        ControlAction::ChannelFx(a) => match a {
            ChannelFxAction::Select(name) => match channel_fx_by_name(&name) {
                Some(i) => fx::select_channel_fx(app, i),
                None => tracing::warn!(name, "unknown channel FX in mapping"),
            },
            // One button cycles through the effects (two-deck controllers).
            ChannelFxAction::Next | ChannelFxAction::Toggle => fx::step_channel_fx(app, 1),
        },
        ControlAction::Sampler(a) => sampler_action(app, a),
        ControlAction::Ignored => {}
    }
}

/// Channel FX index (settings order) of a mapping name.
pub(crate) fn channel_fx_by_name(name: &str) -> Option<usize> {
    crate::settings::CHANNEL_FX.iter().position(|n| *n == name)
}

/// A Tempo FX unit's controls (FX section of the controller).
fn tempo_fx_action(app: &App, u: usize, action: TempoFxAction) {
    match action {
        TempoFxAction::Toggle => fx::toggle_on(app, u),
        // The momentary release button: echo out, as on the hardware.
        TempoFxAction::Release(on) => fx::release(app, u, ExitFxKind::EchoOut, on),
        TempoFxAction::Level(v) => fx::send(app, u, TempoFxCommand::SetLevel(v)),
        TempoFxAction::BeatDown => fx::step_unit_beats(app, u, -1),
        TempoFxAction::BeatUp => fx::step_unit_beats(app, u, 1),
        TempoFxAction::AutoBeat => fx::auto_beat(app, u),
        TempoFxAction::Tap => fx::tap(app, u),
        TempoFxAction::SelectNext => fx::step_unit_kind(app, u, 1),
        TempoFxAction::SelectPrev => fx::step_unit_kind(app, u, -1),
        TempoFxAction::Select(name) => match fx::kind_from_name(&name) {
            Some(k) => fx::send(app, u, TempoFxCommand::SetKind(k)),
            None => tracing::warn!(name, "unknown Tempo FX in mapping"),
        },
        TempoFxAction::Assign(ch) => fx::send(
            app,
            u,
            TempoFxCommand::SetTarget(fx::target_from_number(ch as i32)),
        ),
        TempoFxAction::BuildUp(bars) => fx::build_up(app, u, bars),
    }
}

/// Sampler section: pads of the shown bank, banks, volume, cue.
fn sampler_action(app: &App, a: SamplerAction) {
    let bank = app.sampler.bank.get();
    match a {
        SamplerAction::Pad { slot, pressed, .. } => {
            sampler::trigger(app, sampler::slot_of(bank, slot as usize), pressed)
        }
        SamplerAction::Stop { slot } => engine::send(
            app,
            Command::Sampler(SamplerCommand::Stop {
                slot: sampler::slot_of(bank, slot as usize),
            }),
        ),
        SamplerAction::BankNext => sampler::step_bank_by(app, 1),
        SamplerAction::BankPrev => sampler::step_bank_by(app, -1),
        SamplerAction::Volume(v) => sampler::set_volume(app, v),
        SamplerAction::CueToggle => sampler::toggle_cue(app),
    }
}

fn stem_of(p: StemPart) -> Stem {
    match p {
        StemPart::Vocals => Stem::Vocals,
        StemPart::Drums => Stem::Drums,
        StemPart::Bass => Stem::Bass,
        StemPart::Inst => Stem::Inst,
    }
}

fn deck_action(app: &App, d: usize, action: DeckAction) {
    let Some(s) = engine::snapshot(app).map(|st| st.decks[d]) else {
        if matches!(action, DeckAction::Load) {
            decks::load_selected(app, d);
        }
        return;
    };
    let cmd = match action {
        DeckAction::PlayPause => Some(DeckCommand::TogglePlay),
        DeckAction::CuePress => Some(DeckCommand::Cue(true)),
        DeckAction::CueRelease => Some(DeckCommand::Cue(false)),
        DeckAction::JumpToStart => Some(DeckCommand::Seek { seconds: 0.0 }),
        DeckAction::Sync => {
            decks::toggle_sync(app, d);
            None
        }
        DeckAction::PhraseStart => {
            decks::phrase_start(app, d);
            None
        }
        DeckAction::LoopCapture => {
            decks::capture_loop(app, d);
            None
        }
        DeckAction::BpmScale(f) => {
            decks::bpm_scale(app, d, f as f64);
            None
        }
        DeckAction::SyncOff => Some(DeckCommand::SetSync(false)),
        DeckAction::SetMaster => {
            engine::send(app, Command::SetMasterDeck(Some(d)));
            None
        }
        DeckAction::KeyLock => Some(DeckCommand::SetKeylock(!s.keylock)),
        DeckAction::KeyReset => Some(DeckCommand::SetKeyShift(0.0)),
        DeckAction::KeySync => Some(DeckCommand::SetKeySync(!s.key_sync)),
        DeckAction::TapBpm => {
            decks::tap_bpm(app, d);
            None
        }
        DeckAction::EditInPrepare => {
            crate::edit_in_prepare::toggle(app, d);
            None
        }
        DeckAction::MemoryCueSet => {
            decks::memory_cue(app, d, true);
            None
        }
        DeckAction::MemoryCueDelete => {
            decks::memory_cue(app, d, false);
            None
        }
        DeckAction::StemLevel { stem, level } => Some(DeckCommand::SetStemLevel {
            stem: stem_of(stem),
            level,
        }),
        DeckAction::StemMuteToggle { .. } if !s.stems_available => {
            crate::stems_model::part_pressed(app, d);
            None
        }
        DeckAction::StemSolo { stem, on } => {
            crate::stem_mix::set_solo(app, d, stem_of(stem), on);
            None
        }
        DeckAction::StemKeepOnly { stem } => {
            crate::stem_mix::keep_only(app, d, stem_of(stem));
            None
        }
        DeckAction::StemCue { stem } => {
            crate::stem_mix::toggle_audition(app, d, stem_of(stem));
            None
        }
        DeckAction::StemMuteToggle { stem } => {
            let st = stem_of(stem);
            Some(DeckCommand::SetStemLevel {
                stem: st,
                level: if s.stem_levels[st.index()] < 0.5 {
                    1.0
                } else {
                    0.0
                },
            })
        }
        DeckAction::Tempo(v) => Some(DeckCommand::SetTempoFader(v)),
        DeckAction::TempoRange => Some(DeckCommand::SetTempoRange(next_tempo_range(s.tempo_range))),
        DeckAction::TempoReset => Some(DeckCommand::SetTempoFader(0.0)),
        DeckAction::Quantize => {
            let q = engine::snapshot(app).map(|s| s.quantize).unwrap_or(true);
            engine::send(app, Command::SetQuantize(!q));
            None
        }
        DeckAction::JogTouch(t) => Some(DeckCommand::JogTouch(t)),
        // Which surface of the platter turned does not change the deck's jog
        // mode (VINYL / bend is the deck's setting): the engine scratches while
        // a vinyl-mode platter is touched and bends otherwise. Ticks come in
        // the common unit of the controller layer.
        DeckAction::JogTurn { ticks, mode } => {
            let per_rev = od_controller::device_module::JOG_TICKS_PER_REVOLUTION as f32;
            let rev = ticks as f32 / per_rev;
            if matches!(mode, od_controller::JogMode::Search) {
                // SHIFT + platter: eight times as far, playing or not.
                let by = rev as f64 * 8.0 * od_engine::SECONDS_PER_REVOLUTION;
                Some(DeckCommand::Seek {
                    seconds: (s.position + by).max(0.0),
                })
            } else {
                Some(DeckCommand::JogTurn(rev))
            }
        }
        DeckAction::HotCue { slot, pressed } => Some(DeckCommand::HotCue {
            slot: slot as usize,
            pressed,
        }),
        DeckAction::HotCueDelete { slot } => Some(DeckCommand::DeleteHotCue {
            slot: slot as usize,
        }),
        DeckAction::MemoryCuePrev => Some(DeckCommand::MemoryCue(MemoryCueJump::Previous)),
        DeckAction::MemoryCueNext => Some(DeckCommand::MemoryCue(MemoryCueJump::Next)),
        DeckAction::LoopIn => Some(DeckCommand::LoopIn),
        DeckAction::LoopOut => Some(DeckCommand::LoopOut),
        DeckAction::AutoLoop { beats } => Some(DeckCommand::AutoLoop {
            beats: beats as f64,
        }),
        DeckAction::LoopRoll { beats, pressed } => Some(DeckCommand::LoopRoll {
            beats: beats as f64,
            on: pressed,
        }),
        DeckAction::LoopHalve => Some(DeckCommand::LoopHalve),
        DeckAction::LoopDouble => Some(DeckCommand::LoopDouble),
        DeckAction::ReloopExit => Some(if s.loop_state.active {
            DeckCommand::LoopExit
        } else {
            DeckCommand::Reloop
        }),
        DeckAction::BeatJump { beats } => Some(DeckCommand::BeatJump {
            beats: beats as f64,
        }),
        DeckAction::Slip => Some(DeckCommand::SetSlip(!s.slip)),
        DeckAction::Reverse => Some(DeckCommand::SetReverse(!s.reverse)),
        DeckAction::CutIn(on) => {
            crate::mixer::cut_in(app, d, on);
            None
        }
        DeckAction::PhraseJump { forward } => {
            crate::deck_craft::phrase_jump(app, d, forward);
            None
        }
        DeckAction::DropMark => {
            crate::deck_craft::toggle_drop(app, d);
            None
        }
        DeckAction::Censor(on) => Some(DeckCommand::Censor(on)),
        DeckAction::VinylMode => Some(DeckCommand::SetJogMode(if s.jog_mode == JogMode::Vinyl {
            JogMode::Bend
        } else {
            JogMode::Vinyl
        })),
        DeckAction::NeedleSearch(f) => Some(DeckCommand::NeedleSearch { fraction: f as f64 }),
        DeckAction::Shift(on) => {
            app.decks.borrow_mut().shift[d] = on;
            None
        }
        DeckAction::PadMode(m) => {
            decks::set_pad_mode(app, d, m);
            None
        }
        // Every pad mode: hot cues, Hold FX, keyboard, key shift, slicer, …
        DeckAction::Pad { index, pressed, .. } => {
            decks::pad(app, d, index as usize, pressed);
            None
        }
        DeckAction::Load => {
            decks::load_selected(app, d);
            None
        }
        _ => None,
    };
    if let Some(c) = cmd {
        engine::deck_cmd(app, d, c);
    }
}

/// LED feedback from the engine state (every 40 ms).
pub(crate) fn feedback(app: &App) {
    let Some(state) = engine::snapshot(app) else {
        return;
    };
    let recording = app.recorder.borrow().is_recording();
    if app.controller.borrow().manager.is_none() {
        return;
    }
    let channel_fx = app.state.borrow().settings.perform.channel_fx.clone();
    let modes = app.decks.borrow().pad_mode;
    let pad_rows: Vec<Vec<crate::PadView>> = (0..decks::DECKS)
        .map(|d| decks::pad_rows(app, d, &state))
        .collect();
    let c = app.controller.borrow();
    let Some(ctrl) = c.manager.as_ref() else {
        return;
    };
    for d in 0..decks::DECKS {
        let s = &state.decks[d];
        let deck = d as u8 + 1;
        let led = |el: DeckElement, on: bool| {
            ControlFeedback::led(FeedbackTarget::Deck { deck, element: el }, on)
        };
        let blink_cue = !s.playing && s.track.is_some();
        ctrl.send_feedback(None, led(DeckElement::Play, s.playing));
        ctrl.send_feedback(
            None,
            ControlFeedback::led_state(
                FeedbackTarget::Deck {
                    deck,
                    element: DeckElement::Cue,
                },
                if blink_cue {
                    od_controller::LedState::Blink
                } else {
                    od_controller::LedState::from_bool(s.playing)
                },
            ),
        );
        ctrl.send_feedback(None, led(DeckElement::Sync, s.sync));
        ctrl.send_feedback(None, led(DeckElement::Master, s.is_master));
        ctrl.send_feedback(None, led(DeckElement::KeyLock, s.keylock));
        ctrl.send_feedback(None, led(DeckElement::Slip, s.slip));
        ctrl.send_feedback(None, led(DeckElement::LoopActive, s.loop_state.active));
        ctrl.send_feedback(None, led(DeckElement::Loaded, s.track.is_some()));
        ctrl.send_feedback(None, led(DeckElement::KeySync, s.key_sync));
        ctrl.send_feedback(None, led(DeckElement::Quantize, state.quantize));
        ctrl.send_feedback(None, led(DeckElement::Censor, s.censor));
        ctrl.send_feedback(None, led(DeckElement::Reverse, s.reverse));
        ctrl.send_feedback(None, led(DeckElement::Vinyl, s.jog_mode == JogMode::Vinyl));
        for slot in 0..8u8 {
            ctrl.send_feedback(
                None,
                led(
                    DeckElement::Pad {
                        mode: PadMode::HotCue,
                        index: slot,
                    },
                    s.hot_cues[slot as usize].is_some(),
                ),
            );
        }
        // The pad mode buttons, and the pads of the current mode.
        let mode = modes[d];
        for m in PadMode::ALL {
            ctrl.send_feedback(None, led(DeckElement::PadMode(m), m == mode));
        }
        if mode != PadMode::HotCue {
            for p in &pad_rows[d] {
                let target = FeedbackTarget::Deck {
                    deck,
                    element: DeckElement::Pad {
                        mode,
                        index: p.index as u8,
                    },
                };
                ctrl.send_feedback(
                    None,
                    ControlFeedback::led_state(target, pad_led(p.set, p.lit)),
                );
            }
        }
        ctrl.send_feedback(
            None,
            ControlFeedback::level(
                FeedbackTarget::Mixer {
                    channel: deck,
                    element: MixerElement::Vu,
                },
                state.channels[d].meter.peak.min(1.0),
            ),
        );
        ctrl.send_feedback(
            None,
            ControlFeedback::led(
                FeedbackTarget::Mixer {
                    channel: deck,
                    element: MixerElement::Cue,
                },
                state.channels[d].cue,
            ),
        );
    }
    for u in 0..TEMPO_FX_UNITS {
        let unit = u as u8 + 1;
        ctrl.send_feedback(
            None,
            ControlFeedback::led(FeedbackTarget::TempoFxOn { unit }, state.tempo_fx[u].on),
        );
        ctrl.send_feedback(
            None,
            ControlFeedback::led(
                FeedbackTarget::TempoFxRelease { unit },
                state.tempo_fx[u].release.is_some(),
            ),
        );
    }
    for name in crate::settings::CHANNEL_FX {
        ctrl.send_feedback(
            None,
            ControlFeedback::led(FeedbackTarget::ChannelFx(name.to_string()), name == channel_fx),
        );
    }
    ctrl.send_feedback(
        None,
        ControlFeedback::led(FeedbackTarget::SamplerCue, state.sampler_cue),
    );
    ctrl.send_feedback(
        None,
        ControlFeedback::led(FeedbackTarget::MasterCue, state.master_cue),
    );
    ctrl.send_feedback(
        None,
        ControlFeedback::led(FeedbackTarget::Recording, recording),
    );
}

/// Pad LED of a non-hot-cue mode: on while engaged, dim when it has
/// something to play.
pub(crate) fn pad_led(set: bool, lit: bool) -> LedState {
    if lit {
        LedState::On
    } else if set {
        LedState::Dim
    } else {
        LedState::Off
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pads;

    /// [82] The inputs without a mapping are open for learn, the monitor and
    /// the guided setup, and while CREATE is shown; else they stay closed.
    #[test]
    fn unmapped_inputs_open_in_create_only() {
        assert!(wants_unmapped(false, false, "create"));
        assert!(wants_unmapped(true, false, "perform") && wants_unmapped(false, true, "prepare"));
        for mode in ["perform", "prepare", "lighting"] {
            assert!(!wants_unmapped(false, false, mode), "{mode}");
        }
    }

    #[test]
    fn pads() {
        assert_eq!(pads::pad_beats(0), 0.25);
        assert_eq!(pads::pad_beats(9), 32.0);
        assert_eq!(pads::pad_jump(0), -1.0);
        assert_eq!(pads::pad_jump(7), 8.0);
        assert_eq!(next_tempo_range(TempoRange::Wide), TempoRange::Six);
    }

    #[test]
    fn mapping_names_and_leds() {
        // The channel FX names a four-deck mapping sends.
        for n in ["space", "dub_echo", "sweep", "noise", "crush", "filter"] {
            assert!(channel_fx_by_name(n).is_some(), "{n}");
        }
        assert_eq!(channel_fx_by_name("laser"), None);
        // Every Tempo FX name of that mapping resolves to an effect.
        for n in [
            "low_cut_echo",
            "echo",
            "multi_tap_delay",
            "spiral",
            "reverb",
            "trans",
            "enigma_jet",
            "flanger",
            "phaser",
            "stretch",
            "slip_roll",
            "roll",
            "mobius_saw",
            "mobius_triangle",
        ] {
            assert!(fx::kind_from_name(n).is_some(), "{n}");
        }
        assert_eq!(pad_led(true, true), LedState::On);
        assert_eq!(pad_led(true, false), LedState::Dim);
        assert_eq!(pad_led(false, false), LedState::Off);
    }

    #[test]
    fn controller_trim_spans_the_knob() {
        assert_eq!(trim_db(0.0), -24.0);
        assert_eq!(trim_db(0.5), 0.0);
        assert_eq!(trim_db(1.0), 12.0);
    }
}
