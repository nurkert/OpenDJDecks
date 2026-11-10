//! LIGHTING (SPEC §10.5): feeds the DMX show from what the decks play and
//! drives the LIGHTING view.
//!
//! The show runs on its own thread at 40 frames per second, so DMX timing
//! does not depend on UI repaints. The UI thread only publishes what it
//! knows about the loaded tracks ([`TrackLight`]) and the latest engine
//! state; [`deck_music`] turns both into the show's input. The rig (patch,
//! show settings, outputs) is kept in `lighting.json` next to the library.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use od_core::{MusicalKey, Phrase, PhraseMood};
use od_engine::{DeckState, EngineState, MAX_DECKS};
#[cfg(test)]
pub use od_lighting::add_fixture;
pub use od_lighting::parse_output;
use od_lighting::{
    Arming, Attr, DeckMusic, Effect, Failsafe, FixtureDef, FixtureRole, LightCue, Lighting, Look,
    Mode, Momentary, OutputKind, Patch, Patched, Program, RigConfig, SafetyLimits, Show,
    ShowSettings, Universe, Wave, hue_for_key,
};
use slint::{Color, ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::path::PathBuf;
use std::rc::Rc;

use crate::app::{App, handler};
use crate::models::Rows;
use crate::{
    LightAutoRow, LightCapRow, LightFixtureView, LightGroupView, LightLookGroupRow,
    LightMasterRow, LightOutputRow, LightPadView, LightPatchRow,
};

/// Frames per second sent to the outputs (DMX refreshes at most ~44 Hz).
pub const FPS: u32 = 40;

/// What the show needs to know about a loaded track.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackLight {
    pub phrases: Vec<Phrase>,
    pub mood: Option<PhraseMood>,
    pub key: Option<MusicalKey>,
    /// Energy in 1-second steps, 0..1.
    pub energy: Vec<f32>,
}

/// Show input for one deck (pure).
pub fn deck_music(s: &DeckState, gain_to_master: f32, track: Option<&TrackLight>) -> DeckMusic {
    let beat = s.beat.max(0.0);
    let b = beat as u32;
    let phrase = track.and_then(|t| {
        t.phrases
            .iter()
            .find(|p| p.start_beat <= b && b < p.end_beat)
    });
    let progress = phrase
        .map(|p| ((beat - p.start_beat as f64) / (p.end_beat - p.start_beat).max(1) as f64) as f32)
        .unwrap_or(0.0);
    let energy = track
        .and_then(|t| t.energy.get(s.position.max(0.0) as usize).copied())
        .unwrap_or(0.5);
    DeckMusic {
        track: s.track.map(|t| t.0),
        playing: s.playing && s.track.is_some(),
        bpm: if s.bpm > 0.0 { s.bpm } else { 120.0 },
        beat,
        phrase: phrase.map(|p| p.kind),
        mood: track.and_then(|t| t.mood),
        phrase_progress: progress.clamp(0.0, 1.0),
        energy,
        weight: gain_to_master,
        hue: track
            .and_then(|t| t.key)
            .map(|k| hue_for_key(k.camelot_number()))
            .unwrap_or(0.6),
    }
}

/// Show input for all decks.
pub fn music(state: &EngineState, tracks: &[Option<TrackLight>]) -> Vec<DeckMusic> {
    (0..state.decks_count.min(MAX_DECKS))
        .map(|d| {
            deck_music(
                &state.decks[d],
                state.channels[d].gain_to_master,
                tracks.get(d).and_then(Option::as_ref),
            )
        })
        .collect()
}

/// What the UI reads back for the rig preview and channel monitor.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub looks: Vec<Look>,
    pub universes: Vec<Universe>,
    /// Pro cue in effect.
    pub cue: Option<usize>,
    /// Status per output link.
    pub outputs: Vec<od_lighting::OutputStatus>,
    /// The show failed and only the failsafe scene is sent.
    pub faulted: bool,
    /// AUTO's pick for the deck you hear most, and the one HOLD keeps.
    pub pick: Option<od_lighting::AutoPick>,
    pub held: Option<od_lighting::AutoPick>,
}

enum Msg {
    Engine(Box<EngineState>),
    Track(usize, Option<TrackLight>),
    Patch(Patch),
    Settings(ShowSettings),
    Momentary(Momentary),
    Output(u16, od_lighting::OutputKind),
    TapBpm(Option<f64>),
    ClearOutputs,
    Cue(Option<usize>),
    Limits(SafetyLimits),
    Arming(Arming),
    Failsafe(Failsafe),
    Identify(Option<usize>),
    Pads(od_lighting::PadState),
    Test(Option<od_lighting::TestPattern>),
    Opener(od_lighting::Opener),
    Stop,
}

/// Handle to the lighting thread.
pub struct Runner {
    tx: Sender<Msg>,
    pub snapshot: Arc<Mutex<Snapshot>>,
}

impl Runner {
    pub fn start(patch: Patch, settings: ShowSettings) -> Self {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let shared = snapshot.clone();
        std::thread::Builder::new()
            .name("od-lighting".into())
            .spawn(move || run(rx, Lighting::new(patch, Show::new(settings)), shared))
            .expect("spawn lighting thread");
        Self { tx, snapshot }
    }

    pub fn engine(&self, state: &EngineState) {
        let _ = self.tx.send(Msg::Engine(Box::new(*state)));
    }

    pub fn track(&self, deck: usize, track: Option<TrackLight>) {
        let _ = self.tx.send(Msg::Track(deck, track));
    }

    pub fn patch(&self, patch: Patch) {
        let _ = self.tx.send(Msg::Patch(patch));
    }

    pub fn settings(&self, settings: ShowSettings) {
        let _ = self.tx.send(Msg::Settings(settings));
    }

    pub fn momentary(&self, m: Momentary) {
        let _ = self.tx.send(Msg::Momentary(m));
    }

    pub fn output(&self, universe: u16, kind: od_lighting::OutputKind) {
        let _ = self.tx.send(Msg::Output(universe, kind));
    }

    pub fn clear_outputs(&self) {
        let _ = self.tx.send(Msg::ClearOutputs);
    }

    pub fn tap_bpm(&self, bpm: Option<f64>) {
        let _ = self.tx.send(Msg::TapBpm(bpm));
    }

    /// Selects a Pro cue (Go / Back).
    pub fn cue(&self, cue: Option<usize>) {
        let _ = self.tx.send(Msg::Cue(cue));
    }

    pub fn limits(&self, limits: SafetyLimits) {
        let _ = self.tx.send(Msg::Limits(limits));
    }

    pub fn arming(&self, arming: Arming) {
        let _ = self.tx.send(Msg::Arming(arming));
    }

    pub fn failsafe(&self, f: Failsafe) {
        let _ = self.tx.send(Msg::Failsafe(f));
    }

    pub fn identify(&self, fixture: Option<usize>) {
        let _ = self.tx.send(Msg::Identify(fixture));
    }

    /// Pads latched and held, and HOLD.
    pub fn pads(&self, pads: od_lighting::PadState) {
        let _ = self.tx.send(Msg::Pads(pads));
    }

    /// Starts or stops a test pattern.
    pub fn test(&self, pattern: Option<od_lighting::TestPattern>) {
        let _ = self.tx.send(Msg::Test(pattern));
    }

    /// Test outputs: links are opened by `opener` from now on.
    pub fn set_opener(&self, opener: od_lighting::Opener) {
        let _ = self.tx.send(Msg::Opener(opener));
    }

    pub fn active_cue(&self) -> Option<usize> {
        self.snapshot.lock().ok().and_then(|s| s.cue)
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
    }
}

fn run(rx: Receiver<Msg>, mut rig: Lighting, snapshot: Arc<Mutex<Snapshot>>) {
    let frame = Duration::from_secs(1) / FPS;
    let mut state = EngineState::default();
    let mut tracks: Vec<Option<TrackLight>> = vec![None; MAX_DECKS];
    let mut last = Instant::now();
    let mut next = Instant::now();
    loop {
        let wait = next.saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(Msg::Engine(s)) => state = *s,
            Ok(Msg::Track(d, t)) => {
                if d < MAX_DECKS {
                    tracks[d] = t;
                }
            }
            Ok(Msg::Patch(p)) => rig.patch = p,
            Ok(Msg::Settings(s)) => rig.show.settings = s,
            Ok(Msg::Momentary(m)) => rig.show.momentary = m,
            Ok(Msg::Output(u, kind)) => rig.add_output(u, kind),
            Ok(Msg::ClearOutputs) => rig.clear_outputs(),
            Ok(Msg::TapBpm(b)) => rig.show.tap_bpm = b,
            Ok(Msg::Cue(c)) => rig.show.manual_cue = c,
            Ok(Msg::Limits(l)) => rig.limits = l,
            Ok(Msg::Arming(a)) => rig.arming = a,
            Ok(Msg::Failsafe(f)) => rig.failsafe = f,
            Ok(Msg::Identify(i)) => rig.identify = i,
            Ok(Msg::Pads(p)) => rig.show.pads = p,
            Ok(Msg::Test(t)) => rig.set_test(t),
            Ok(Msg::Opener(o)) => rig.set_opener(o),
            // Quitting: the failsafe choice decides the last frame.
            Ok(Msg::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                rig.shutdown();
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let now = Instant::now();
        if now < next {
            continue;
        }
        let dt = now.duration_since(last).as_secs_f32().min(0.25);
        last = now;
        next += frame;
        if next < now {
            next = now + frame;
        }
        rig.tick_guarded(&music(&state, &tracks), dt);
        if let Ok(mut s) = snapshot.lock() {
            s.looks.clone_from(&rig.looks);
            s.universes.clone_from(&rig.frame);
            s.cue = rig.show.active_cue();
            s.outputs = rig.status();
            s.faulted = rig.faulted;
            s.pick = rig.show.current_pick();
            s.held = rig.show.held_pick();
        }
    }
}

/// Colours offered in Easy and Pro mode.
pub const SWATCHES: [[f32; 3]; 8] = [
    [1.0, 0.1, 0.1],
    [1.0, 0.45, 0.0],
    [1.0, 0.8, 0.1],
    [0.1, 1.0, 0.2],
    [0.0, 0.85, 1.0],
    [0.1, 0.25, 1.0],
    [0.65, 0.1, 1.0],
    [1.0, 1.0, 1.0],
];

pub(crate) fn program_label(p: Program) -> &'static str {
    match p {
        Program::Static => "STATIC",
        Program::Pulse => "PULSE",
        Program::Chase => "CHASE",
        Program::Sweep => "SWEEP",
        Program::Breathe => "BREATHE",
        Program::Build => "BUILD",
        Program::Strobe => "STROBE",
        Program::Blackout => "DARK",
    }
}

fn swatch_index(c: Option<[f32; 3]>) -> i32 {
    c.and_then(|c| SWATCHES.iter().position(|s| *s == c))
        .map_or(-1, |i| i as i32)
}

fn program_index(p: Option<Program>) -> i32 {
    p.and_then(|p| Program::ALL.iter().position(|x| *x == p))
        .map_or(-1, |i| i as i32)
}

/// Effect presets of the Pro group FX key (index 0 = none).
pub const EFFECTS: [(&str, Option<(Attr, Wave, f32)>); 8] = [
    ("NONE", None),
    ("DIM SINE", Some((Attr::Dimmer, Wave::Sine, 2.0))),
    ("DIM SQUARE", Some((Attr::Dimmer, Wave::Square, 1.0))),
    ("DIM RANDOM", Some((Attr::Dimmer, Wave::Random, 0.5))),
    ("PAN SINE", Some((Attr::Pan, Wave::Sine, 4.0))),
    ("PAN SAW", Some((Attr::Pan, Wave::Saw, 2.0))),
    ("TILT SINE", Some((Attr::Tilt, Wave::Sine, 4.0))),
    ("TILT SQUARE", Some((Attr::Tilt, Wave::Square, 2.0))),
];

/// The effect of preset `i`: full depth, phase spread over the group.
pub fn effect_preset(i: usize) -> Option<Effect> {
    EFFECTS
        .get(i)
        .and_then(|(_, e)| *e)
        .map(|(attr, wave, beats)| Effect {
            attr,
            wave,
            beats,
            depth: if attr == Attr::Dimmer { 1.0 } else { 0.6 },
            spread: 0.25,
        })
}

/// Preset index of a stored effect (0 when none or custom).
pub fn effect_index(e: Option<Effect>) -> i32 {
    (0..EFFECTS.len())
        .find(|&i| effect_preset(i) == e)
        .unwrap_or(0) as i32
}

/// The rig file of a library.
pub fn rig_path(library: &std::path::Path) -> PathBuf {
    od_lighting::rig::rig_path(library)
}

/// Stage position of each fixture: movers on the top truss, bars and
/// effects in the middle, washes at the front.
pub fn stage_positions(patch: &Patch) -> Vec<(f32, f32)> {
    let row = |r: FixtureRole| match r {
        FixtureRole::Mover => 0,
        FixtureRole::Bar | FixtureRole::Effect => 1,
        FixtureRole::Wash => 2,
    };
    let mut counts = [0usize; 3];
    for f in &patch.fixtures {
        counts[row(f.def.role())] += 1;
    }
    let mut seen = [0usize; 3];
    patch
        .fixtures
        .iter()
        .map(|f| {
            let r = row(f.def.role());
            let i = seen[r];
            seen[r] += 1;
            ((i as f32 + 0.5) / counts[r] as f32, [0.18, 0.5, 0.82][r])
        })
        .collect()
}

pub(crate) fn rgb(c: [f32; 3]) -> Color {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_rgb_u8(b(c[0]), b(c[1]), b(c[2]))
}

/// View rows for the stage and rig list (pure).
pub fn fixture_views(patch: &Patch, looks: &[Look]) -> Vec<LightFixtureView> {
    let pos = stage_positions(patch);
    patch
        .fixtures
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let look = looks.get(i).cloned().unwrap_or_default();
            let role = f.def.role();
            let (x, y) = pos[i];
            let pan = if f.invert_pan {
                1.0 - look.pan
            } else {
                look.pan
            };
            let cells = f.def.pixels() as usize;
            let cell_colors: Vec<Color> = if role == FixtureRole::Bar {
                (0..cells)
                    .map(|c| {
                        let px = if look.pixels.is_empty() {
                            look.color
                        } else {
                            look.pixels[(c * look.pixels.len()) / cells.max(1)]
                        };
                        rgb(px)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            LightFixtureView {
                name: f.name.clone().into(),
                group: f.group.clone().into(),
                role: match role {
                    FixtureRole::Wash => "wash",
                    FixtureRole::Mover => "mover",
                    FixtureRole::Bar => "bar",
                    FixtureRole::Effect => "effect",
                }
                .into(),
                address: format!("{}·{:03}", f.universe + 1, f.address).into(),
                x,
                y,
                color: rgb(look.color),
                intensity: look.intensity.clamp(0.0, 1.0),
                beam_x: (x + (pan - 0.5) * 0.7).clamp(0.0, 1.0),
                beam_y: (y + 0.25 + 0.5 * look.tilt).clamp(0.0, 1.0),
                strobe: look.strobe > 0.0,
                cells: ModelRc::new(VecModel::from(cell_colors)),
            }
        })
        .collect()
}

/// "DECK 1 · CHORUS → SWEEP": what the show follows now (pure); a pad in
/// AUTO's phrase table is named as "PAD 1.3 NAME".
pub fn status_text(
    mode: Mode,
    decks: &[DeckMusic],
    auto: &od_lighting::AutoSettings,
    pads: &[od_lighting::PadLook],
) -> String {
    let dominant = decks
        .iter()
        .enumerate()
        .filter(|(_, d)| d.playing && d.weight > 0.01)
        .max_by(|a, b| a.1.weight.total_cmp(&b.1.weight));
    match (mode, dominant) {
        (_, None) => "NO MUSIC · IDLE WASH".into(),
        (Mode::Auto, Some((i, d))) => {
            let phrase = d
                .phrase
                .map(|p| crate::render::phrase_label(p).to_string())
                .unwrap_or_else(|| "—".into());
            let mixing = decks
                .iter()
                .filter(|d| d.playing && d.weight > 0.01)
                .count()
                > 1;
            format!(
                "DECK {} · {phrase} → {}{}",
                i + 1,
                crate::lighting_pads::pick_label(auto.table.pick(d.phrase), pads),
                if mixing { " · MIX BLEND" } else { "" }
            )
        }
        (Mode::Easy, Some((i, _))) => format!("EASY · BEAT FROM DECK {}", i + 1),
        (Mode::Pro, Some((i, _))) => format!("PRO · BEAT FROM DECK {}", i + 1),
    }
}

/// UI side of the lighting: the rig file, the runner and the view rows.
pub struct LightingRuntime {
    pub config: RigConfig,
    taps: crate::fx::TapTempo,
    tap_bpm: Option<f64>,
    path: PathBuf,
    pub(crate) runner: Runner,
    pub(crate) momentary: Momentary,
    /// Track sent to the runner per deck.
    sent: [Option<od_core::TrackId>; MAX_DECKS],
    tracks: Vec<Option<TrackLight>>,
    fixtures: Rows<LightFixtureView>,
    /// The track you hear and its playhead beat (for track cues).
    focus: Option<(od_core::TrackId, f64, String)>,
    /// Modification time of `lighting.json` as last read or written here;
    /// another time means another program (the MCP server) changed it.
    seen: Option<std::time::SystemTime>,
    /// Last look at the rig file and the live inbox.
    polled: Option<Instant>,
    /// Fog and lasers armed (never saved: each start begins locked).
    pub arming: Arming,
    /// Fixture blinking for identification.
    pub identify: Option<usize>,
    /// Setup opened with the PIN (always open without a PIN).
    pub unlocked: bool,
    pin_error: String,
    /// Universe shown in the channel view.
    channel_universe: usize,
    channels: Rc<VecModel<i32>>,
    outputs: Rows<LightOutputRow>,
    patch_rows: Rows<LightPatchRow>,
    /// Last link summary sent to the UI.
    link: (String, String),
    /// Own pads: latched and held, HOLD (never saved).
    pub pads: od_lighting::PadState,
    /// Page of pads shown (0..4).
    pub page: usize,
    /// PADS: clicks open pads in the editor instead of playing them.
    pub editing: bool,
    pub edit_pad: Option<u16>,
    /// Clicks on pads add steps to the chase being edited.
    pub adding_steps: bool,
    pub(crate) pad_rows: Rows<LightPadView>,
    pub(crate) master_rows: Rows<LightMasterRow>,
    pub(crate) auto_rows: Rows<LightAutoRow>,
    pub(crate) edit_rows: Rows<LightLookGroupRow>,
    /// Who is in control (key, text) and the phrase heard now.
    pub(crate) layer: (String, String),
    pub(crate) now_slot: Option<od_lighting::PhraseSlot>,
    /// The pick HOLD keeps (from the show thread).
    pub(crate) held: Option<od_lighting::AutoPick>,
    /// Serial ports for USB-DMX; fixed by the documentation hook.
    pub ports: Vec<String>,
    pub(crate) ports_fixed: bool,
    /// Art-Net nodes found, the search running, and its result line.
    pub nodes: Vec<od_lighting::ArtNode>,
    pub(crate) finder: Option<Receiver<std::io::Result<Vec<od_lighting::ArtNode>>>>,
    pub(crate) find_text: String,
    /// Where ArtPoll goes (empty = the broadcast address; tests use a fake
    /// node on the loopback address).
    pub poll_targets: Vec<std::net::SocketAddr>,
    /// Test pattern running.
    pub test: Option<od_lighting::TestPattern>,
    pub guide: crate::lighting_pads::Guide,
}

impl LightingRuntime {
    pub fn load(path: PathBuf) -> Self {
        let config = RigConfig::load(&path).unwrap_or_else(|e| {
            tracing::warn!("lighting rig {}: {e}", path.display());
            RigConfig::default()
        });
        let runner = Runner::start(config.patch.clone(), config.settings.clone());
        let seen = modified(&path);
        let rt = Self {
            config,
            taps: Default::default(),
            tap_bpm: None,
            path,
            runner,
            momentary: Momentary::default(),
            sent: [None; MAX_DECKS],
            tracks: vec![None; MAX_DECKS],
            fixtures: Rows::default(),
            focus: None,
            seen,
            polled: None,
            arming: Arming::default(),
            identify: None,
            unlocked: false,
            pin_error: String::new(),
            channel_universe: 0,
            channels: Rc::new(VecModel::from(vec![0; 512])),
            outputs: Rows::default(),
            patch_rows: Rows::default(),
            link: Default::default(),
            pads: Default::default(),
            page: 0,
            editing: false,
            edit_pad: None,
            adding_steps: false,
            pad_rows: Rows::default(),
            master_rows: Rows::default(),
            auto_rows: Rows::default(),
            edit_rows: Rows::default(),
            layer: Default::default(),
            now_slot: None,
            held: None,
            ports: Vec::new(),
            ports_fixed: false,
            nodes: Vec::new(),
            finder: None,
            find_text: String::new(),
            poll_targets: Vec::new(),
            test: None,
            guide: Default::default(),
        };
        rt.push();
        rt.apply_outputs();
        rt
    }

    /// Setup is closed by the PIN.
    pub fn locked(&self) -> bool {
        !self.config.safety.pin.is_empty() && !self.unlocked
    }

    /// Sends everything the show thread keeps to it.
    fn push(&self) {
        self.runner.patch(self.config.patch.clone());
        self.runner.settings(self.config.settings.clone());
        self.runner.limits(self.config.safety.clone());
        self.runner.failsafe(self.config.failsafe);
        self.runner.arming(self.arming);
        self.runner.identify(self.identify);
    }

    /// Routes every output through `opener` (tests) and reopens them.
    pub fn use_opener(&self, opener: od_lighting::Opener) {
        self.runner.set_opener(opener);
        self.apply_outputs();
    }

    fn apply_outputs(&self) {
        self.runner.clear_outputs();
        if self.config.enabled {
            for (u, kind) in &self.config.outputs {
                self.runner.output(*u, kind.clone());
            }
        }
    }

    fn save(&mut self) {
        if let Err(e) = self.config.save(&self.path) {
            tracing::warn!("saving lighting rig: {e}");
        }
        self.seen = modified(&self.path);
    }

    /// Sends patch and settings to the show and stores the rig.
    fn commit(&mut self) {
        if self
            .identify
            .is_some_and(|i| i >= self.config.patch.fixtures.len())
        {
            self.identify = None;
        }
        self.push();
        self.save();
    }
}

/// Every Lighting property and model, in the main window and the detached
/// one.
pub(crate) fn refresh(app: &App) {
    refresh_main(app);
    crate::lighting_pads::sync(app);
}

fn refresh_main(app: &App) {
    let rt = app.lighting.borrow();
    let s = &rt.config.settings;
    let groups: Vec<LightGroupView> = rt
        .config
        .patch
        .groups()
        .into_iter()
        .map(|name| {
            let gs = s.groups.get(&name).cloned().unwrap_or_default();
            let has_movers = rt
                .config
                .patch
                .fixtures
                .iter()
                .any(|f| f.group == name && f.def.role() == FixtureRole::Mover);
            LightGroupView {
                name: name.into(),
                program: program_index(gs.program),
                color: swatch_index(gs.color),
                effect: effect_index(gs.effect),
                pan: gs.pan.unwrap_or(-1.0),
                tilt: gs.tilt.unwrap_or(-1.0),
                has_movers,
            }
        })
        .collect();
    let groups = ModelRc::new(VecModel::from(groups));
    let cues: Vec<SharedString> = s
        .cues
        .iter()
        .map(|c| SharedString::from(c.name.as_str()))
        .collect();
    let cues = ModelRc::new(VecModel::from(cues));
    let outputs: Vec<String> = rt
        .config
        .outputs
        .iter()
        .map(|(u, k)| {
            let to = match k {
                OutputKind::ArtNet { target, .. } => format!("Art-Net {target}"),
                OutputKind::Sacn { universe } => format!("sACN universe {universe}"),
                OutputKind::EnttecPro { device } => format!("USB-DMX {device}"),
            };
            format!("U{} → {to}", u + 1)
        })
        .collect();
    let (timeline_title, timeline) = timeline_rows(&rt);
    let patch = &rt.config.patch;
    let has_fog = patch.fixtures.iter().any(|f| f.def.is_fog());
    let has_laser = patch.fixtures.iter().any(|f| f.def.is_laser());
    let caps: Vec<LightCapRow> = patch
        .groups()
        .into_iter()
        .map(|name| LightCapRow {
            cap: rt.config.safety.group_cap(&name),
            name: name.into(),
        })
        .collect();
    let caps = ModelRc::new(VecModel::from(caps));
    let universes: Vec<SharedString> = (0..patch.universes())
        .map(|u| SharedString::from(format!("U{}", u + 1)))
        .collect();
    let universes = ModelRc::new(VecModel::from(universes));
    let timeline = ModelRc::new(VecModel::from(timeline));
    let output_text: SharedString = if outputs.is_empty() {
        "No output set".into()
    } else {
        outputs.join(" · ").into()
    };
    each(app, |g| {
        g.set_enabled(rt.config.enabled);
        g.set_mode(
            match s.mode {
                Mode::Auto => "auto",
                Mode::Easy => "easy",
                Mode::Pro => "pro",
            }
            .into(),
        );
        g.set_master(s.master);
        g.set_blackout(s.blackout);
        g.set_flash(rt.momentary.flash);
        g.set_strobe(rt.momentary.strobe);
        g.set_fog(rt.momentary.fog);
        g.set_tap_bpm(
            rt.tap_bpm
                .map(|b| format!("{b:.0}"))
                .unwrap_or_default()
                .into(),
        );
        g.set_easy_program(program_index(Some(s.easy_program)));
        g.set_easy_color(swatch_index(s.easy_color));
        g.set_groups(groups.clone());
        g.set_cues(cues.clone());
        g.set_output_text(output_text.clone());
        g.set_rig_path(rt.path.display().to_string().into());
        g.set_timeline_title(timeline_title.clone());
        g.set_timeline(timeline.clone());
        g.set_strobe_hz(rt.config.safety.strobe_cap());
        g.set_fog_armed(rt.arming.fog);
        g.set_laser_armed(rt.arming.laser);
        g.set_has_fog(has_fog);
        g.set_has_laser(has_laser);
        g.set_caps(caps.clone());
        g.set_pin_set(!rt.config.safety.pin.is_empty());
        g.set_locked(rt.locked());
        g.set_pin_error(rt.pin_error.clone().into());
        g.set_failsafe(
            match rt.config.failsafe {
                Failsafe::Hold => "hold",
                Failsafe::Safe => "safe",
                Failsafe::Blackout => "blackout",
            }
            .into(),
        );
        g.set_universes(universes.clone());
        g.set_channel_universe(rt.channel_universe as i32);
    });
    rt.patch_rows.sync_eq(patch_rows(&rt));
    // Rows also when no frame arrives (engine stopped): the rig list.
    let looks = rt
        .runner
        .snapshot
        .lock()
        .map(|s| s.looks.clone())
        .unwrap_or_default();
    rt.fixtures
        .sync(fixture_views(&rt.config.patch, &looks), |_, _| false);
}

/// Rows of the patch table.
fn patch_rows(rt: &LightingRuntime) -> Vec<LightPatchRow> {
    let patch = &rt.config.patch;
    let clashes = patch.collisions();
    patch
        .fixtures
        .iter()
        .enumerate()
        .map(|(i, f)| LightPatchRow {
            name: f.name.as_str().into(),
            group: f.group.as_str().into(),
            universe: (f.universe + 1).to_string().into(),
            address: f.address.to_string().into(),
            mode: f.def.mode.as_str().into(),
            channels: f.def.channels.len().to_string().into(),
            clash: match clashes[i] {
                Some(od_lighting::Collision::Overlaps(j)) => {
                    od_i18n::trf("OVERLAPS {}", &[&patch.fixtures[j].name.to_uppercase()]).into()
                }
                Some(od_lighting::Collision::OutOfRange) => od_i18n::tr("PAST CHANNEL 512").into(),
                None => SharedString::new(),
            },
            identify: rt.identify == Some(i),
            hazard: if f.def.is_laser() {
                "laser"
            } else if f.def.is_fog() {
                "fog"
            } else {
                ""
            }
            .into(),
        })
        .collect()
}

/// Output rows and the one-line summary for Perform, from the show
/// thread's link status.
pub fn link_rows(
    config: &RigConfig,
    status: &[od_lighting::OutputStatus],
    faulted: bool,
) -> (Vec<LightOutputRow>, String, String) {
    use od_lighting::LinkState;
    let target = |k: &OutputKind| match k {
        OutputKind::ArtNet { target, .. } => format!("Art-Net {target}"),
        OutputKind::Sacn { universe } => format!("sACN {universe}"),
        OutputKind::EnttecPro { device } => format!("USB-DMX {device}"),
    };
    let rows: Vec<LightOutputRow> = config
        .outputs
        .iter()
        .enumerate()
        .map(|(i, (u, k))| {
            let st = status.get(i).filter(|_| config.enabled);
            let (state, detail) = match st {
                None => ("off", od_i18n::tr("DMX OUT IS OFF").to_string()),
                Some(s) if s.state == LinkState::Lost => (
                    "lost",
                    od_i18n::trf(
                        "LOST {} S AGO · RETRYING · {}",
                        &[&(s.since as u32), &s.error],
                    ),
                ),
                Some(s) if s.reconnects > 0 && s.since < 20.0 => (
                    "back",
                    od_i18n::trf("BACK {} S AGO · SENDING", &[&(s.since as u32)]),
                ),
                Some(_) => ("ok", od_i18n::tr("SENDING").to_string()),
            };
            LightOutputRow {
                universe: format!("U{}", u + 1).into(),
                target: target(k).into(),
                state: state.into(),
                detail: detail.into(),
            }
        })
        .collect();
    let lost: Vec<String> = rows
        .iter()
        .filter(|r| r.state == "lost")
        .map(|r| r.universe.to_string())
        .collect();
    let (link, text) = if !config.enabled || rows.is_empty() {
        ("off", od_i18n::tr("OFF").to_string())
    } else if faulted {
        ("fault", od_i18n::tr("SHOW FAILED · FAILSAFE").to_string())
    } else if !lost.is_empty() {
        (
            "lost",
            od_i18n::trf("{} LOST · RETRYING", &[&lost.join(" ")]),
        )
    } else if rows.iter().any(|r| r.state == "back") {
        ("back", od_i18n::tr("BACK · SENDING").to_string())
    } else {
        ("ok", od_i18n::trf("{} SENDING", &[&rows.len()]))
    };
    (rows, link.to_string(), text)
}

/// Output status and the live channel view, from the show thread.
fn refresh_link(app: &App) {
    let mut rt = app.lighting.borrow_mut();
    let (status, faulted, universe) = {
        let snap = rt.runner.snapshot.lock();
        match snap {
            Ok(s) => (
                s.outputs.clone(),
                s.faulted,
                s.universes.get(rt.channel_universe).copied(),
            ),
            Err(_) => return,
        }
    };
    let (rows, link, text) = link_rows(&rt.config, &status, faulted);
    rt.outputs.sync_eq(rows);
    let channels_shown = app.ui.global::<crate::Lighting>().get_centre() == "channels"
        || app
            .lighting_window
            .borrow()
            .as_ref()
            .is_some_and(|w| w.global::<crate::Lighting>().get_centre() == "channels");
    if channels_shown {
        let data = universe.unwrap_or([0; 512]);
        for (i, v) in data.iter().enumerate() {
            if rt.channels.row_data(i) != Some(*v as i32) {
                rt.channels.set_row_data(i, *v as i32);
            }
        }
    }
    if rt.link != (link.clone(), text.clone()) {
        rt.link = (link.clone(), text.clone());
        drop(rt);
        each(app, |g| {
            g.set_link(link.as_str().into());
            g.set_link_text(text.as_str().into());
        });
    }
}

/// Title and rows ("BAR 17 → Drop") of the focused track's lighting cues.
fn timeline_rows(rt: &LightingRuntime) -> (SharedString, Vec<SharedString>) {
    let Some((id, _, title)) = &rt.focus else {
        return (SharedString::new(), Vec::new());
    };
    let s = &rt.config.settings;
    let mut cues = s.track_cues.get(&id.0).cloned().unwrap_or_default();
    cues.sort_by_key(|c| c.beat);
    let rows = cues
        .iter()
        .map(|c| {
            let name = s.cues.get(c.cue).map(|x| x.name.as_str()).unwrap_or("?");
            SharedString::from(format!("BAR {} → {name}", c.beat / 4 + 1))
        })
        .collect();
    (title.clone().into(), rows)
}

/// Runs `f` on the Lighting state of the main window and, when it is open,
/// the detached LIGHTING window.
pub(crate) fn each(app: &App, f: impl Fn(&crate::Lighting<'_>)) {
    f(&app.ui.global::<crate::Lighting>());
    if let Some(w) = app.lighting_window.borrow().as_ref() {
        f(&w.global::<crate::Lighting>());
    }
}

/// Opens the LIGHTING view in its own window (second screen).
pub(crate) fn detach(app: &Rc<App>) {
    if let Some(w) = app.lighting_window.borrow().as_ref() {
        let _ = w.show();
        return;
    }
    let Ok(w) = crate::LightingWindow::new() else {
        return;
    };
    {
        let st = app.state.borrow();
        crate::shell::apply_theme(&w.global::<crate::Theme>(), &st.settings, st.config.lite);
    }
    wire_global(app, &w.global::<crate::Lighting>());
    let _ = w.show();
    *app.lighting_window.borrow_mut() = Some(w);
    refresh(app);
}

pub(crate) fn edit(app: &App, f: impl FnOnce(&mut LightingRuntime)) {
    {
        let mut rt = app.lighting.borrow_mut();
        f(&mut rt);
        rt.commit();
    }
    refresh(app);
}

pub(crate) fn wire(app: &Rc<App>) {
    wire_global(app, &app.ui.global::<crate::Lighting>());
    refresh(app);
    demo(app);
}

/// Documentation screenshots, in memory only (nothing is saved):
/// `OPENDJDECKS_DEMO_PROOF` parts `light-patch` (patch table with a
/// collision), `light-lost` (a second universe on a USB-DMX link that is
/// missing), `light-safety`, `light-channels`, `light-strip` (the panel
/// in Perform), `light-pads` (own looks and a chase on page 1, one latched,
/// group masters), `light-edit` (PADS with a look open in the editor),
/// `light-auto` (a palette, a pad in the phrase table, HOLD), `light-connect`
/// (ports, a fake Art-Net node at 192.0.2.10 found on the loopback address,
/// the COLOURS test pattern) and `light-guide` (the guided setup at its second step).
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let parts: Vec<String> = v.split(',').map(str::to_string).collect();
    if !parts.iter().any(|p| p.starts_with("light-")) {
        return;
    }
    let has = |p: &str| parts.iter().any(|x| x == p);
    let g = app.ui.global::<crate::Lighting>();
    {
        let mut rt = app.lighting.borrow_mut();
        if has("light-patch") {
            let _ = od_lighting::add_fixtures(&mut rt.config.patch, 2, 4, Some(0), Some(60), "Back", "Wash");
            if let Some(i) = rt.config.patch.fixtures.iter().position(|f| f.def.role() == FixtureRole::Bar) {
                let u = rt.config.patch.fixtures[i].universe;
                rt.config.patch.set_place(i, u, 12);
            }
            rt.identify = Some(0);
            g.set_centre("patch".into());
        }
        if has("light-lost") {
            rt.config.enabled = true;
            rt.config.failsafe = Failsafe::Safe;
            rt.config.outputs.retain(|(u, _)| *u == 0);
            rt.config.outputs.push((
                1,
                OutputKind::EnttecPro {
                    device: "/dev/tty.usbserial-DMX".into(),
                },
            ));
            let link = od_lighting::TestLink::default();
            rt.runner.set_opener(Box::new(move |k: &OutputKind| {
                if matches!(k, OutputKind::EnttecPro { .. }) {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "no such device",
                    ))
                } else {
                    link.opener()(k)
                }
            }));
            g.set_setup_tab("output".into());
        }
        if has("light-safety") {
            rt.config.safety.pin = "2468".into();
            rt.unlocked = true;
            rt.config.safety.group_caps.insert("Bars".into(), 0.6);
            g.set_setup_tab("safety".into());
        }
        if has("light-channels") {
            g.set_centre("channels".into());
        }
        if has("light-strip") {
            g.set_panel(true);
        }
        if has("light-pads") || has("light-edit") || has("light-auto") {
            demo_looks(&mut rt);
        }
        if has("light-edit") {
            rt.editing = true;
            rt.edit_pad = Some(0);
            g.set_centre("pads".into());
        }
        if has("light-auto") {
            let a = &mut rt.config.settings.auto;
            a.palette = od_lighting::Palette::Cool;
            a.table.set(od_lighting::PhraseSlot::Drop, od_lighting::AutoPick::Pad(4));
            rt.pads.latched = None;
            rt.pads.hold = true;
            rt.config.settings.mode = Mode::Auto;
        }
        if has("light-connect") || has("light-guide") {
            rt.ports = vec!["/dev/cu.usbmodem2101".into(), "/dev/cu.usbserial-DMX1".into()];
            rt.ports_fixed = true;
            // A stand-in node on the loopback address, for the picture only.
            if let Ok(node) = od_lighting::FakeNode::start(
                std::net::Ipv4Addr::new(192, 0, 2, 10),
                "Stage left",
                "Two-port DMX node",
                &[0, 1],
            ) {
                rt.poll_targets = vec![node.addr];
                Box::leak(Box::new(node));
            }
            g.set_setup_tab("output".into());
        }
        if has("light-connect") {
            rt.test = Some(od_lighting::TestPattern::Colours);
            rt.runner.test(rt.test);
        }
        if has("light-guide") {
            rt.guide = crate::lighting_pads::Guide {
                step: 2,
                kind: "artnet".into(),
                ..Default::default()
            };
        }
        rt.runner.pads(rt.pads.clone());
        rt.push();
        rt.apply_outputs();
    }
    refresh(app);
    if has("light-connect") || has("light-guide") {
        crate::lighting_pads::find_nodes(app);
    }
}

/// Example looks for the documentation pictures: three looks, a flash hit
/// and a chase on page 1, the first latched, the movers at 70 %.
fn demo_looks(rt: &mut LightingRuntime) {
    use od_lighting::{GroupSettings as G, PadAction, PadLook};
    let set = |l: &mut PadLook, group: &str, program: Program, color: [f32; 3], level: Option<f32>| {
        l.groups.insert(
            group.into(),
            G {
                program: Some(program),
                color: Some(color),
                intensity: level,
                ..Default::default()
            },
        );
    };
    let mut red = PadLook::new(0, "Red wash");
    set(&mut red, "Wash", Program::Static, SWATCHES[0], Some(0.9));
    set(&mut red, "Bars", Program::Breathe, SWATCHES[1], None);
    let mut blue = PadLook::new(1, "Blue sweep");
    set(&mut blue, "Movers", Program::Sweep, SWATCHES[5], None);
    set(&mut blue, "Wash", Program::Static, SWATCHES[4], Some(0.6));
    let mut warm = PadLook::new(2, "Warm room");
    for g in ["Wash", "Bars"] {
        set(&mut warm, g, Program::Breathe, SWATCHES[2], Some(0.7));
    }
    set(&mut warm, "Movers", Program::Blackout, SWATCHES[2], Some(0.0));
    let mut hit = PadLook::new(3, "White hit");
    hit.action = PadAction::Flash;
    for g in ["Wash", "Bars", "Movers"] {
        set(&mut hit, g, Program::Static, SWATCHES[7], Some(1.0));
    }
    let mut chase = PadLook::new(4, "Colour run");
    chase.steps = vec![0, 1, 2];
    chase.step_beats = 2.0;
    rt.config.settings.pads = vec![red, blue, warm, hit, chase];
    rt.config.settings.group_levels.insert("Movers".into(), 0.7);
    rt.pads.latched = Some(0);
}

/// Binds models and callbacks of one window's Lighting state.
fn wire_global(app: &Rc<App>, g: &crate::Lighting<'_>) {
    g.on_detach(handler!(app, |a| detach(&a)));
    g.set_fixtures(app.lighting.borrow().fixtures.handle());
    {
        let rt = app.lighting.borrow();
        g.set_patch_rows(rt.patch_rows.handle());
        g.set_outputs(rt.outputs.handle());
        g.set_channels(ModelRc::from(rt.channels.clone()));
        g.set_link(rt.link.0.as_str().into());
        g.set_link_text(rt.link.1.as_str().into());
    }
    wire_setup(app, g);
    crate::lighting_pads::wire(app, g);
    g.set_programs(ModelRc::new(VecModel::from(
        Program::ALL
            .iter()
            .map(|p| SharedString::from(program_label(*p)))
            .collect::<Vec<_>>(),
    )));
    g.set_swatches(ModelRc::new(VecModel::from(
        SWATCHES.iter().map(|c| rgb(*c)).collect::<Vec<_>>(),
    )));
    g.set_fixture_types(ModelRc::new(VecModel::from(
        FixtureDef::BUILTIN
            .iter()
            .map(|n| {
                SharedString::from(FixtureDef::builtin(n).map(|d| d.model).unwrap_or_default())
            })
            .collect::<Vec<_>>(),
    )));
    g.on_set_enabled(handler!(app, |a, on: bool| {
        {
            let mut rt = a.lighting.borrow_mut();
            rt.config.enabled = on;
            rt.apply_outputs();
            rt.save();
        }
        refresh(&a)
    }));
    g.on_set_mode(handler!(app, |a, m: SharedString| edit(&a, |rt| {
        rt.config.settings.mode = match m.as_str() {
            "easy" => Mode::Easy,
            "pro" => Mode::Pro,
            _ => Mode::Auto,
        }
    })));
    g.on_set_master(handler!(app, |a, v: f32| edit(&a, |rt| rt
        .config
        .settings
        .master =
        v.clamp(0.0, 1.0))));
    g.on_set_blackout(handler!(app, |a, on: bool| edit(&a, |rt| rt
        .config
        .settings
        .blackout =
        on)));
    g.on_flash_held(handler!(app, |a, on: bool| momentary(&a, |m| m.flash = on)));
    g.on_strobe_held(handler!(app, |a, on: bool| momentary(&a, |m| m.strobe = on)));
    g.on_choose_easy_program(handler!(app, |a, i: i32| edit(&a, |rt| {
        if let Some(p) = Program::ALL.get(i as usize) {
            rt.config.settings.easy_program = *p;
        }
    })));
    g.on_choose_easy_color(handler!(app, |a, i: i32| edit(&a, |rt| rt
        .config
        .settings
        .easy_color =
        SWATCHES.get(i as usize).copied())));
    g.on_group_program(handler!(app, |a, gi: i32, pi: i32| edit(&a, |rt| {
        if let Some(name) = rt.config.patch.groups().get(gi as usize) {
            rt.config
                .settings
                .groups
                .entry(name.clone())
                .or_default()
                .program = Program::ALL.get(pi as usize).copied();
        }
    })));
    g.on_group_color(handler!(app, |a, gi: i32, ci: i32| edit(&a, |rt| {
        if let Some(name) = rt.config.patch.groups().get(gi as usize) {
            rt.config
                .settings
                .groups
                .entry(name.clone())
                .or_default()
                .color = SWATCHES.get(ci as usize).copied();
        }
    })));
    g.on_remove_fixture(handler!(app, |a, i: i32| setup(&a, |rt| {
        if (i as usize) < rt.config.patch.fixtures.len() {
            rt.config.patch.fixtures.remove(i as usize);
        }
    })));
    g.on_set_output(handler!(
        app,
        |a, kind: SharedString, target: SharedString, universe: SharedString| {
            let u = match parse_universe(&universe) {
                Ok(u) => u.unwrap_or(0),
                Err(e) => return a.status(e),
            };
            match parse_output(kind.as_str(), target.as_str()) {
                // Several universes to one node or network: each universe
                // goes to its own Art-Net Port-Address or sACN universe
                // unless one is given.
                Ok(OutputKind::ArtNet { target: t, .. }) => {
                    set_output(&a, u, OutputKind::ArtNet { target: t, universe: u })
                }
                Ok(OutputKind::Sacn { .. }) if target.trim().is_empty() => {
                    set_output(&a, u, OutputKind::Sacn { universe: u + 1 })
                }
                Ok(k) => set_output(&a, u, k),
                Err(e) => a.status(format!("Lighting: {e}")),
            }
        }
    ));
    g.set_effects(ModelRc::new(VecModel::from(
        EFFECTS
            .iter()
            .map(|(n, _)| SharedString::from(*n))
            .collect::<Vec<_>>(),
    )));
    g.on_fog_held(handler!(app, |a, on: bool| momentary(&a, |m| m.fog = on)));
    g.on_tap(handler!(app, |a| tap(&a)));
    g.on_group_effect(handler!(app, |a, gi: i32, ei: i32| edit(&a, |rt| {
        if let Some(name) = rt.config.patch.groups().get(gi as usize) {
            rt.config
                .settings
                .groups
                .entry(name.clone())
                .or_default()
                .effect = effect_preset(ei.max(0) as usize);
        }
    })));
    g.on_group_position(handler!(app, |a, gi: i32, x: f32, y: f32| edit(&a, |rt| {
        if let Some(name) = rt.config.patch.groups().get(gi as usize) {
            let g = rt.config.settings.groups.entry(name.clone()).or_default();
            g.pan = Some(x.clamp(0.0, 1.0));
            g.tilt = Some(y.clamp(0.0, 1.0));
        }
    })));
    g.on_group_position_clear(handler!(app, |a, gi: i32| edit(&a, |rt| {
        if let Some(name) = rt.config.patch.groups().get(gi as usize) {
            let g = rt.config.settings.groups.entry(name.clone()).or_default();
            g.pan = None;
            g.tilt = None;
        }
    })));
    g.on_import_ofl(handler!(app, |a| {
        let Some(path) = rfd::FileDialog::new()
            .set_title(od_i18n::tr("Fixture file (Open Fixture Library)"))
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        if a.lighting.borrow().locked() {
            return;
        }
        match import_ofl(&mut a.lighting.borrow_mut().config.patch, &path) {
            Ok(_) => edit(&a, |_| {}),
            Err(e) => a.status(format!("Lighting: {e}")),
        }
    }));
    g.on_cue_go(handler!(app, |a| cue_step(&a, true)));
    g.on_cue_back(handler!(app, |a| cue_step(&a, false)));
    g.on_cue_store(handler!(app, |a, name: SharedString| edit(&a, |rt| {
        let s = &mut rt.config.settings;
        let name = if name.trim().is_empty() {
            format!("Cue {}", s.cues.len() + 1)
        } else {
            name.to_string()
        };
        s.cues.push(LightCue {
            name,
            groups: s.groups.clone(),
            fade_beats: 4.0,
        });
    })));
    g.on_timeline_add(handler!(app, |a| {
        let cue = app_active_cue(&a);
        edit(&a, |rt| {
            let Some((id, beat, _)) = rt.focus.clone() else {
                return;
            };
            if rt.config.settings.cues.is_empty() {
                return;
            }
            // On the bar: the cue starts where the bar the playhead is in starts.
            let bar = (beat.max(0.0) / 4.0).floor() as u32 * 4;
            let list = rt.config.settings.track_cues.entry(id.0).or_default();
            list.retain(|c| c.beat != bar);
            list.push(od_lighting::TrackCue {
                beat: bar,
                cue: cue.min(rt.config.settings.cues.len() - 1),
            });
            list.sort_by_key(|c| c.beat);
        })
    }));
    g.on_timeline_delete(handler!(app, |a, i: i32| edit(&a, |rt| {
        let Some((id, _, _)) = rt.focus.clone() else {
            return;
        };
        if let Some(list) = rt.config.settings.track_cues.get_mut(&id.0) {
            list.sort_by_key(|c| c.beat);
            if (i as usize) < list.len() {
                list.remove(i as usize);
            }
        }
    })));
    g.on_cue_delete(handler!(app, |a, i: i32| edit(&a, |rt| {
        if (i as usize) < rt.config.settings.cues.len() {
            rt.config.settings.cues.remove(i as usize);
        }
    })));
}

/// Sends universe `u` (0-based) through `kind` and turns DMX OUT on (not
/// while the PIN locks the setup).
pub(crate) fn set_output(app: &App, u: u16, kind: OutputKind) {
    if app.lighting.borrow().locked() {
        return;
    }
    {
        let mut rt = app.lighting.borrow_mut();
        rt.config.outputs.retain(|(x, _)| *x != u);
        rt.config.outputs.push((u, kind));
        rt.config.outputs.sort_by_key(|(x, _)| *x);
        rt.config.enabled = true;
        rt.apply_outputs();
        rt.save();
    }
    refresh(app);
}

/// An edit of the setup (patch, outputs, safety): ignored while the PIN
/// locks it.
fn setup(app: &App, f: impl FnOnce(&mut LightingRuntime)) {
    if app.lighting.borrow().locked() {
        return;
    }
    edit(app, f);
}

/// "3" → universe index 2; "" → None.
pub fn parse_universe(text: &str) -> Result<Option<u16>, String> {
    let t = text.trim().trim_start_matches(['U', 'u']).trim();
    if t.is_empty() {
        return Ok(None);
    }
    match t.parse::<u16>() {
        Ok(n) if (1..=od_lighting::MAX_UNIVERSES).contains(&n) => Ok(Some(n - 1)),
        _ => Err(od_i18n::trf(
            "Lighting: universes are 1 to {}",
            &[&od_lighting::MAX_UNIVERSES],
        )),
    }
}

/// "17" → address 17; "" → None.
pub fn parse_address(text: &str) -> Result<Option<u16>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    match t.parse::<u16>() {
        Ok(n) if (1..=512).contains(&n) => Ok(Some(n)),
        _ => Err(od_i18n::tr("Lighting: addresses are 1 to 512").to_string()),
    }
}

/// Callbacks of the patch table, outputs and safety limits.
fn wire_setup(app: &Rc<App>, g: &crate::Lighting<'_>) {
    g.on_add_fixtures(handler!(
        app,
        |a,
         ti: i32,
         name: SharedString,
         group: SharedString,
         count: SharedString,
         universe: SharedString,
         start: SharedString| {
            if a.lighting.borrow().locked() {
                return;
            }
            let count = match count.trim() {
                "" => 1,
                t => match t.parse::<u16>() {
                    Ok(n) if (1..=512).contains(&n) => n,
                    _ => return a.status(od_i18n::tr("Lighting: the count is 1 to 512")),
                },
            };
            let (universe, start) = match (parse_universe(&universe), parse_address(&start)) {
                (Ok(u), Ok(s)) => (u, s),
                (Err(e), _) | (_, Err(e)) => return a.status(e),
            };
            let universe = universe.or(start.map(|_| 0));
            let result = od_lighting::add_fixtures(
                &mut a.lighting.borrow_mut().config.patch,
                ti as usize,
                count,
                universe,
                start,
                name.as_str(),
                group.as_str(),
            );
            match result {
                Ok(r) => {
                    edit(&a, |_| {});
                    a.status(od_i18n::trf("Lighting: {} fixture(s) patched", &[&r.len()]));
                }
                Err(e) => a.status(format!("Lighting: {e}")),
            }
        }
    ));
    g.on_patch_universe(handler!(app, |a, i: i32, t: SharedString| {
        match parse_universe(&t) {
            Ok(Some(u)) => setup(&a, |rt| {
                let addr = rt.config.patch.fixtures.get(i as usize).map(|f| f.address);
                if let Some(addr) = addr {
                    rt.config.patch.set_place(i as usize, u, addr);
                }
            }),
            Ok(None) => refresh(&a),
            Err(e) => a.status(e),
        }
    }));
    g.on_patch_address(handler!(app, |a, i: i32, t: SharedString| {
        match parse_address(&t) {
            Ok(Some(addr)) => setup(&a, |rt| {
                let u = rt.config.patch.fixtures.get(i as usize).map(|f| f.universe);
                if let Some(u) = u {
                    rt.config.patch.set_place(i as usize, u, addr);
                }
            }),
            Ok(None) => refresh(&a),
            Err(e) => a.status(e),
        }
    }));
    g.on_patch_mode(handler!(app, |a, i: i32, step: i32| setup(&a, |rt| {
        if let Some(f) = rt.config.patch.fixtures.get_mut(i as usize) {
            let names = f.def.mode_names();
            let at = names.iter().position(|n| *n == f.def.mode).unwrap_or(0) as i32;
            let next = (at + step).rem_euclid(names.len() as i32) as usize;
            f.def.set_mode(&names[next]);
        }
    })));
    g.on_patch_channels(handler!(app, |a, i: i32, t: SharedString| {
        match t.trim().parse::<u16>() {
            Ok(n) if (1..=512).contains(&n) => setup(&a, |rt| {
                if let Some(f) = rt.config.patch.fixtures.get_mut(i as usize) {
                    f.def.set_channel_count(n);
                }
            }),
            _ => a.status(od_i18n::tr("Lighting: a fixture has 1 to 512 channels")),
        }
    }));
    g.on_identify(handler!(app, |a, i: i32| {
        {
            let mut rt = a.lighting.borrow_mut();
            let i = i as usize;
            rt.identify = if rt.identify == Some(i) {
                None
            } else {
                Some(i)
            };
            rt.runner.identify(rt.identify);
        }
        refresh(&a);
    }));
    g.on_remove_output(handler!(app, |a, i: i32| {
        if a.lighting.borrow().locked() {
            return;
        }
        {
            let mut rt = a.lighting.borrow_mut();
            if (i as usize) < rt.config.outputs.len() {
                rt.config.outputs.remove(i as usize);
            }
            rt.apply_outputs();
            rt.save();
        }
        refresh(&a);
    }));
    g.on_set_failsafe(handler!(app, |a, f: SharedString| setup(&a, |rt| {
        rt.config.failsafe = match f.as_str() {
            "safe" => Failsafe::Safe,
            "blackout" => Failsafe::Blackout,
            _ => Failsafe::Hold,
        }
    })));
    g.on_choose_universe(handler!(app, |a, u: i32| {
        a.lighting.borrow_mut().channel_universe = u.max(0) as usize;
        refresh(&a);
        refresh_link(&a);
    }));
    g.on_step_strobe(handler!(app, |a, d: i32| setup(&a, |rt| {
        let hz = rt.config.safety.strobe_cap() + d as f32;
        rt.config.safety.strobe_hz = hz.clamp(0.0, od_lighting::safety::MAX_STROBE_HZ).round();
    })));
    g.on_step_cap(handler!(app, |a, i: i32, d: i32| setup(&a, |rt| {
        if let Some(name) = rt.config.patch.groups().get(i as usize).cloned() {
            let cap = (rt.config.safety.group_cap(&name) + d as f32 * 0.1).clamp(0.0, 1.0);
            let cap = (cap * 10.0).round() / 10.0;
            if cap >= 1.0 {
                rt.config.safety.group_caps.remove(&name);
            } else {
                rt.config.safety.group_caps.insert(name, cap);
            }
        }
    })));
    g.on_arm_fog(handler!(app, |a, on: bool| arm(&a, |m| m.fog = on)));
    g.on_arm_laser(handler!(app, |a, on: bool| arm(&a, |m| m.laser = on)));
    g.on_unlock(handler!(app, |a, pin: SharedString| {
        {
            let mut rt = a.lighting.borrow_mut();
            if pin.trim() == rt.config.safety.pin {
                rt.unlocked = true;
                rt.pin_error.clear();
            } else {
                rt.pin_error = od_i18n::tr("Wrong PIN").to_string();
            }
        }
        refresh(&a);
    }));
    g.on_lock(handler!(app, |a| {
        a.lighting.borrow_mut().unlocked = false;
        refresh(&a);
    }));
    g.on_set_pin(handler!(app, |a, pin: SharedString| {
        if a.lighting.borrow().locked() {
            return;
        }
        let pin = pin.trim().to_string();
        let ok = pin.is_empty()
            || ((4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit()));
        if !ok {
            a.lighting.borrow_mut().pin_error = od_i18n::tr("The PIN is 4 to 8 digits").to_string();
            return refresh(&a);
        }
        let cleared = pin.is_empty();
        edit(&a, |rt| {
            rt.config.safety.pin = pin;
            rt.unlocked = true;
            rt.pin_error.clear();
        });
        a.status(if cleared {
            od_i18n::tr("Lighting: setup PIN removed")
        } else {
            od_i18n::tr("Lighting: setup PIN set. LOCK closes the setup.")
        });
    }));
}

/// Fog and laser arming: only from an open setup; disarming always works.
fn arm(app: &App, f: impl FnOnce(&mut Arming)) {
    {
        let mut rt = app.lighting.borrow_mut();
        let mut next = rt.arming;
        f(&mut next);
        let arming_more = (next.fog && !rt.arming.fog) || (next.laser && !rt.arming.laser);
        if arming_more && rt.locked() {
            return;
        }
        rt.arming = next;
        rt.runner.arming(next);
    }
    refresh(app);
}

fn momentary(app: &App, f: impl FnOnce(&mut Momentary)) {
    {
        let mut rt = app.lighting.borrow_mut();
        f(&mut rt.momentary);
        rt.runner.momentary(rt.momentary);
    }
    refresh(app);
}

fn cue_step(app: &App, forward: bool) {
    let (n, active) = {
        let rt = app.lighting.borrow();
        (
            rt.config.settings.cues.len() as i32,
            app.ui.global::<crate::Lighting>().get_active_cue(),
        )
    };
    if n == 0 {
        return;
    }
    let next = if active < 0 {
        if forward { 0 } else { n - 1 }
    } else if forward {
        (active + 1) % n
    } else {
        (active + n - 1) % n
    };
    app.ui.global::<crate::Lighting>().set_active_cue(next);
    app.lighting.borrow().runner.cue(Some(next as usize));
}

thread_local! {
    static TAP_EPOCH: std::time::Instant = std::time::Instant::now();
}

/// A Lighting pad (controller or screen): A–F pick the easy look, G / H
/// strobe and flash while held.
pub(crate) fn pad(app: &App, index: usize, pressed: bool) {
    // Own looks of the current page come first.
    if crate::lighting_pads::deck_pad(app, index, pressed) {
        return;
    }
    match index {
        0..=5 if pressed => edit(app, |rt| {
            rt.config.settings.mode = Mode::Easy;
            rt.config.settings.easy_program = crate::pads::LIGHT_LOOKS[index];
        }),
        6 => momentary(app, |m| m.strobe = pressed),
        7 => momentary(app, |m| m.flash = pressed),
        _ => {}
    }
}

/// The cue the stack shows as current (0 when none).
fn app_active_cue(app: &App) -> usize {
    app.ui.global::<crate::Lighting>().get_active_cue().max(0) as usize
}

/// A mapped controller / MIDI input for the light show.
pub(crate) fn controller(app: &App, a: od_controller::action::LightingAction) {
    use od_controller::action::LightingAction as L;
    match a {
        L::Look(i) => pad(app, i as usize, true),
        L::Strobe(on) => momentary(app, |m| m.strobe = on),
        L::Flash(on) => momentary(app, |m| m.flash = on),
        L::Fog(on) => momentary(app, |m| m.fog = on),
        L::BlackoutToggle => edit(app, |rt| {
            rt.config.settings.blackout = !rt.config.settings.blackout
        }),
        L::Master(v) => edit(app, |rt| rt.config.settings.master = v.clamp(0.0, 1.0)),
        L::CueGo => cue_step(app, true),
        L::CueBack => cue_step(app, false),
        L::Mode(m) => edit(app, |rt| {
            rt.config.settings.mode = match m.as_str() {
                "easy" => Mode::Easy,
                "pro" => Mode::Pro,
                _ => Mode::Auto,
            }
        }),
        other => crate::lighting_pads::controller(app, &other),
    }
}

/// Lit state of the Lighting pads (own looks of the current page, else
/// the built-in looks, strobe and flash).
pub(crate) fn light_pads(app: &App, rows: &mut [crate::PadView]) {
    crate::lighting_pads::deck_pad_views(app, rows);
    let rt = app.lighting.borrow();
    let page = rt.page * od_lighting::PAGE_PADS;
    for r in rows.iter_mut() {
        let i = r.index as usize;
        if od_lighting::looks::find(&rt.config.settings.pads, (page + i) as u16).is_some() {
            continue;
        }
        r.lit = match i {
            0..=5 => {
                rt.config.settings.mode == Mode::Easy
                    && rt.config.settings.easy_program == crate::pads::LIGHT_LOOKS[i]
            }
            6 => rt.momentary.strobe,
            _ => rt.momentary.flash,
        };
    }
}

/// Patches a fixture from an Open Fixture Library file (first mode), named
/// after the model; the manufacturer is the file's folder (OFL layout).
pub fn import_ofl(patch: &mut Patch, path: &std::path::Path) -> Result<usize, od_lighting::Error> {
    let json =
        std::fs::read_to_string(path).map_err(|e| od_lighting::Error::Fixture(e.to_string()))?;
    let maker = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let def = FixtureDef::from_ofl(&json, &maker, None)?;
    let len = def.channels.len() as u16;
    let (universe, address) = (0..16u16)
        .find_map(|u| patch.next_free(u, len).map(|a| (u, a)))
        .ok_or_else(|| od_lighting::Error::Patch("no free DMX address".into()))?;
    let group = match def.role() {
        FixtureRole::Mover => "Movers",
        FixtureRole::Bar => "Bars",
        FixtureRole::Wash => "Wash",
        FixtureRole::Effect => "Effects",
    }
    .to_string();
    let n = patch
        .fixtures
        .iter()
        .filter(|f| f.def.model == def.model)
        .count()
        + 1;
    let name = format!("{} {n}", def.model);
    patch.add(Patched {
        name,
        def,
        universe,
        address,
        group,
        invert_pan: false,
    })
}

fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Frame tick, a few times a second: takes a rig edited by another program
/// (the MCP server) and runs the live commands waiting in the inbox.
pub(crate) fn poll_remote(app: &App) {
    let path = {
        let mut rt = app.lighting.borrow_mut();
        if rt
            .polled
            .is_some_and(|t| t.elapsed() < Duration::from_millis(100))
        {
            return;
        }
        rt.polled = Some(Instant::now());
        rt.path.clone()
    };
    let now = modified(&path);
    if now.is_some() && now != app.lighting.borrow().seen {
        match RigConfig::load(&path) {
            Ok(config) => {
                {
                    let mut rt = app.lighting.borrow_mut();
                    rt.seen = now;
                    if rt.config != config {
                        rt.config = config;
                        rt.push();
                        rt.apply_outputs();
                    }
                }
                refresh(app);
            }
            // Half-written by someone else: try again next time.
            Err(e) => tracing::debug!("lighting rig changed but unreadable: {e}"),
        }
    }
    for cmd in od_lighting::rig::drain(&path) {
        live(app, cmd);
    }
}

/// Runs a live command from outside the app.
pub(crate) fn live(app: &App, cmd: od_lighting::LiveCommand) {
    use od_controller::action::LightingAction as L;
    use od_lighting::LiveCommand as C;
    match cmd {
        C::CueGo => controller(app, L::CueGo),
        C::CueBack => controller(app, L::CueBack),
        C::Cue { index } => {
            let n = app.lighting.borrow().config.settings.cues.len();
            if index < n {
                each(app, |g| g.set_active_cue(index as i32));
                app.lighting.borrow().runner.cue(Some(index));
            }
        }
        C::Flash { on } => controller(app, L::Flash(on)),
        C::Strobe { on } => controller(app, L::Strobe(on)),
        C::Fog { on } => controller(app, L::Fog(on)),
        C::Blackout { on } => edit(app, |rt| rt.config.settings.blackout = on),
        C::Tap => tap(app),
    }
}

fn tap(app: &App) {
    let t = TAP_EPOCH.with(|e| e.elapsed().as_secs_f64());
    let bpm = app.lighting.borrow_mut().taps.tap(t);
    if let Some(b) = bpm {
        let mut rt = app.lighting.borrow_mut();
        rt.tap_bpm = Some(b as f64);
        rt.runner.tap_bpm(Some(b as f64));
    }
    refresh(app);
}

/// Per engine frame: feeds the show and, in LIGHTING mode, the preview.
pub(crate) fn frame(app: &App, state: Option<&EngineState>) {
    refresh_link(app);
    crate::lighting_pads::poll(app);
    let Some(state) = state else {
        if app.ui.get_mode() == "lighting" {
            refresh_stage(app, &[]);
        }
        return;
    };
    // Track metadata when a deck's track changes.
    let changed: Vec<(usize, Option<od_core::TrackId>)> = {
        let rt = app.lighting.borrow();
        (0..MAX_DECKS)
            .filter(|&d| rt.sent[d] != state.decks[d].track)
            .map(|d| (d, state.decks[d].track))
            .collect()
    };
    for (d, id) in changed {
        let light = id.and_then(|id| {
            let st = app.state.borrow();
            let analysis = st.src().analysis(id).ok()?;
            let key = st.src().track(id).ok().and_then(|t| t.key).or(analysis.key);
            Some(TrackLight {
                phrases: analysis.phrases,
                mood: Some(analysis.phrase_mood),
                key,
                energy: analysis.energy,
            })
        });
        let mut rt = app.lighting.borrow_mut();
        rt.sent[d] = id;
        rt.tracks[d].clone_from(&light);
        rt.runner.track(d, light);
    }
    app.lighting.borrow().runner.engine(state);
    // The track you hear, for the Pro track cues.
    let focus = (0..state.decks_count.min(MAX_DECKS))
        .filter(|&d| state.decks[d].playing && state.decks[d].track.is_some())
        .max_by(|&a, &b| {
            state.channels[a]
                .gain_to_master
                .total_cmp(&state.channels[b].gain_to_master)
        })
        .or_else(|| (0..state.decks_count.min(MAX_DECKS)).find(|&d| state.decks[d].track.is_some()))
        .and_then(|d| state.decks[d].track.map(|t| (d, t)));
    let changed = {
        let mut rt = app.lighting.borrow_mut();
        let new = focus.map(|(d, t)| {
            let title = app
                .decks
                .borrow()
                .loaded(d)
                .map(|m| m.title.clone())
                .unwrap_or_default();
            (t, state.decks[d].beat, title)
        });
        let changed = rt.focus.as_ref().map(|f| f.0) != new.as_ref().map(|f| f.0);
        rt.focus = new;
        changed
    };
    if changed {
        refresh(app);
    }
    let window = app
        .lighting_window
        .borrow()
        .as_ref()
        .is_some_and(|w| w.window().is_visible());
    let visible = window
        || app.ui.get_mode() == "lighting"
        || (app.ui.get_mode() == "perform" && app.ui.global::<crate::Lighting>().get_panel());
    if visible {
        let decks = music(state, &app.lighting.borrow().tracks);
        refresh_stage(app, &decks);
    }
}

fn refresh_stage(app: &App, decks: &[DeckMusic]) {
    let rt = app.lighting.borrow();
    let looks = rt
        .runner
        .snapshot
        .lock()
        .map(|s| s.looks.clone())
        .unwrap_or_default();
    rt.fixtures
        .sync(fixture_views(&rt.config.patch, &looks), |a, b| {
            a.color == b.color
                && a.intensity == b.intensity
                && a.beam_x == b.beam_x
                && a.beam_y == b.beam_y
                && a.strobe == b.strobe
                && a.name == b.name
                && a.role != "bar"
        });
    let s = &rt.config.settings;
    let status: SharedString = status_text(s.mode, decks, &s.auto, &s.pads).into();
    let cue = rt.runner.active_cue();
    let held = rt.runner.snapshot.lock().ok().and_then(|s| s.held);
    drop(rt);
    app.lighting.borrow_mut().held = held;
    each(app, |g| {
        g.set_status(status.clone());
        if let Some(c) = cue {
            g.set_active_cue(c as i32);
        }
    });
    crate::lighting_pads::sync_live(app, decks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::{PhraseKind, TrackId};
    use slint::Model;

    fn track() -> TrackLight {
        TrackLight {
            phrases: vec![
                Phrase {
                    start_beat: 0,
                    end_beat: 64,
                    kind: PhraseKind::Intro,
                },
                Phrase {
                    start_beat: 64,
                    end_beat: 128,
                    kind: PhraseKind::Chorus,
                },
            ],
            mood: Some(PhraseMood::High),
            key: None,
            energy: vec![0.2, 0.9],
        }
    }

    #[test]
    fn deck_music_reads_phrase_progress_and_energy() {
        let s = DeckState {
            track: Some(TrackId(1)),
            playing: true,
            beat: 80.0,
            bpm: 126.0,
            position: 1.5,
            ..Default::default()
        };
        let m = deck_music(&s, 0.7, Some(&track()));
        assert_eq!(m.phrase, Some(PhraseKind::Chorus));
        assert!((m.phrase_progress - 0.25).abs() < 1e-6);
        assert_eq!((m.energy, m.weight, m.bpm), (0.9, 0.7, 126.0));
        assert!(m.playing);
        let empty = deck_music(&DeckState::default(), 0.0, None);
        assert!(!empty.playing);
        assert_eq!((empty.phrase, empty.bpm), (None, 120.0));
    }

    #[test]
    fn add_fixture_picks_address_group_and_name() {
        let mut patch = Patch::default();
        let mover = FixtureDef::BUILTIN
            .iter()
            .position(|n| *n == "moving-head")
            .unwrap();
        add_fixture(&mut patch, mover, "", "").unwrap();
        add_fixture(&mut patch, mover, "Spot R", "Truss").unwrap();
        assert_eq!(patch.fixtures[0].name, "Moving head 1");
        assert_eq!(patch.fixtures[0].group, "Movers");
        assert_eq!(
            (patch.fixtures[1].address, patch.fixtures[1].group.as_str()),
            (15, "Truss")
        );
        assert!(add_fixture(&mut patch, 99, "", "").is_err());
    }

    #[test]
    fn output_text_is_parsed() {
        assert_eq!(
            parse_output("artnet", "10.0.0.5").unwrap(),
            OutputKind::ArtNet {
                target: "10.0.0.5:6454".parse().unwrap(),
                universe: 0
            }
        );
        assert_eq!(
            parse_output("sacn", "").unwrap(),
            OutputKind::Sacn { universe: 1 }
        );
        assert!(parse_output("sacn", "70000").is_err());
        assert!(parse_output("enttec", "").is_err());
        assert!(parse_output("artnet", "nonsense").is_err());
    }

    #[test]
    fn stage_and_status() {
        let mut patch = Patch::default();
        for t in ["moving-head", "rgbw-par", "rgbw-par", "light-bar-8"] {
            let i = FixtureDef::BUILTIN.iter().position(|n| *n == t).unwrap();
            add_fixture(&mut patch, i, "", "").unwrap();
        }
        let pos = stage_positions(&patch);
        assert_eq!(pos[0], (0.5, 0.18));
        assert_eq!((pos[1], pos[2]), ((0.25, 0.82), (0.75, 0.82)));
        let looks = vec![
            Look {
                intensity: 1.0,
                color: [1.0, 0.0, 0.0],
                ..Look::default()
            };
            4
        ];
        let views = fixture_views(&patch, &looks);
        assert_eq!(views[0].role, "mover");
        assert_eq!(views[3].cells.row_count(), 8);
        assert_eq!(views[1].color, Color::from_rgb_u8(255, 0, 0));
        let deck = DeckMusic {
            playing: true,
            weight: 1.0,
            phrase: Some(od_core::PhraseKind::Chorus),
            ..Default::default()
        };
        let (auto, pads) = (od_lighting::AutoSettings::default(), Vec::new());
        assert!(status_text(Mode::Auto, &[deck], &auto, &pads).ends_with("→ SWEEP"));
        assert!(status_text(Mode::Auto, &[deck, deck], &auto, &pads).contains("MIX BLEND"));
        assert_eq!(status_text(Mode::Auto, &[], &auto, &pads), "NO MUSIC · IDLE WASH");
    }

    #[test]
    fn runner_renders_frames_on_its_own_thread() {
        let mut patch = Patch::default();
        patch
            .add(od_lighting::Patched {
                name: "Par".into(),
                def: od_lighting::FixtureDef::builtin("rgb-par").unwrap(),
                universe: 0,
                address: 1,
                group: "Wash".into(),
                invert_pan: false,
            })
            .unwrap();
        let runner = Runner::start(patch, ShowSettings::default());
        let mut state = EngineState::default();
        state.decks[0] = DeckState {
            track: Some(TrackId(1)),
            playing: true,
            beat: 70.0,
            bpm: 128.0,
            ..Default::default()
        };
        state.channels[0].gain_to_master = 1.0;
        runner.track(0, Some(track()));
        runner.engine(&state);
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let lit = runner
                .snapshot
                .lock()
                .unwrap()
                .universes
                .first()
                .is_some_and(|u| u[..3].iter().any(|&v| v > 100));
            if lit {
                break;
            }
            assert!(Instant::now() < deadline, "no chorus frame rendered");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn effect_presets_round_trip_and_ofl_import() {
        for i in 0..EFFECTS.len() {
            assert_eq!(effect_index(effect_preset(i)), i as i32);
        }
        assert_eq!(effect_preset(0), None);
        let dir = tempfile::tempdir().unwrap();
        let maker = dir.path().join("acme");
        std::fs::create_dir_all(&maker).unwrap();
        let file = maker.join("spot.json");
        std::fs::write(&file, r#"{"name":"Spot 60","availableChannels":{"Pan":{"capability":{"type":"Pan"}},"Tilt":{"capability":{"type":"Tilt"}},"Dimmer":{"capability":{"type":"Intensity"}}},"modes":[{"name":"3ch","channels":["Pan","Tilt","Dimmer"]}]}"#).unwrap();
        let mut patch = Patch::default();
        import_ofl(&mut patch, &file).unwrap();
        import_ofl(&mut patch, &file).unwrap();
        assert_eq!(patch.fixtures[1].name, "Spot 60 2");
        assert_eq!(
            (
                patch.fixtures[1].address,
                patch.fixtures[0].group.as_str(),
                patch.fixtures[0].def.manufacturer.as_str()
            ),
            (4, "Movers", "acme")
        );
    }
}
