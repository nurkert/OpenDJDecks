//! DMX lighting for OpenDJDecks (SPEC §10.5).
//!
//! * [`output`]: Art-Net, sACN and Enttec DMX USB Pro outputs.
//! * [`fixture`]: fixture types (built-in generics and Open Fixture Library
//!   files) and the patch.
//! * [`render`]: the attribute model ([`Look`]) rendered to DMX values.
//! * [`show`]: automatic, easy and pro modes driven by the music.
//! * [`looks`]: own looks and chases on pads, group masters, and how AUTO
//!   is steered (palettes, the phrase-to-look table, hold, release).
//! * [`discovery`]: finding Art-Net nodes on the local network.
//! * [`rig`]: editing a rig from outside the app and its live inbox.
//! * [`safety`]: strobe and flash cap, level caps, fog and laser lockout,
//!   applied to every frame before it is sent.
//! * [`Lighting`]: patch + show + outputs, ticked by the app; output links
//!   are retried when lost, with a chosen [`Failsafe`] meanwhile.

pub mod discovery;
pub mod fixture;
pub mod looks;
pub mod output;
pub mod render;
pub mod rig;
pub mod safety;
pub mod show;

pub use fixture::{
    ChannelKind, Collision, FixtureDef, FixtureMode, FixtureRole, MAX_UNIVERSES, Patch, Patched,
};
pub use discovery::{ArtNode, FakeNode};
pub use looks::{
    AutoPick, AutoSettings, AutoTable, Layer, PAGE_PADS, PAGES, PADS, PadAction, PadLook,
    PadState, Palette, PhraseSlot,
};
pub use output::{Link, Opener, Output, OutputKind, TestLink, Universe, serial_ports};
pub use render::{Look, render, render_fixture};
pub use rig::{LiveCommand, add_fixture, add_fixtures, parse_output};
pub use safety::{Arming, Limiter, SafetyLimits};
pub use show::{
    Attr, DeckMusic, Effect, GroupSettings, LightCue, Mode, Momentary, Program, Show, ShowSettings,
    TrackCue, Wave, hsv, hue_for_key,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("fixture: {0}")]
    Fixture(String),
    #[error("patch: {0}")]
    Patch(String),
    #[error("output: {0}")]
    Output(#[from] std::io::Error),
}

/// Everything about a rig the app keeps between sessions.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RigConfig {
    pub patch: Patch,
    pub settings: ShowSettings,
    /// Universe → output.
    pub outputs: Vec<(u16, OutputKind)>,
    /// Lighting on at all (outputs only send while enabled).
    #[serde(default)]
    pub enabled: bool,
    /// Strobe cap, group level caps and the setup PIN.
    #[serde(default)]
    pub safety: SafetyLimits,
    /// What the rig does while an output link is lost, after a show fault
    /// and when the app quits.
    #[serde(default)]
    pub failsafe: Failsafe,
}

impl RigConfig {
    /// Reads a rig; a missing file is the empty default.
    pub fn load(path: &std::path::Path) -> Result<Self, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| Error::Patch(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), Error> {
        let text = serde_json::to_string_pretty(self).map_err(|e| Error::Patch(e.to_string()))?;
        let tmp = path.with_extension("json.part");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}

/// What the rig shows while an output link is lost (all universes follow,
/// so the rig stays in one known state), after a fault in the show and
/// when the app quits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Failsafe {
    /// Freeze the last look.
    #[default]
    Hold,
    /// Fade to a calm warm wash: no strobe, no fog, no lasers.
    Safe,
    /// All channels to zero.
    Blackout,
}

/// Level of the safe scene.
pub const SAFE_LEVEL: f32 = 0.3;
/// Fade into the safe scene, seconds.
pub const SAFE_FADE: f32 = 2.0;
/// Fade from the failsafe back to the show after the link returns, seconds.
pub const RECOVER_FADE: f32 = 1.0;
/// Time between reconnect attempts, seconds.
pub const RETRY: f64 = 1.0;
/// Identify blinks at this rate (below the default strobe cap).
pub const IDENTIFY_HZ: f32 = 1.5;

/// A test pattern for checking a new connection; it replaces the show
/// while it runs and passes through the safety limits like everything else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestPattern {
    /// One fixture after another, white, one step each.
    Each,
    /// The whole rig red, green, blue, white, one step each.
    Colours,
    /// The whole rig white.
    Full,
}

/// Seconds per test pattern step.
pub const TEST_STEP: f32 = 1.0;
/// Level of the test pattern (bright enough to see, kind to eyes up close).
pub const TEST_LEVEL: f32 = 0.8;

/// Looks of a test pattern at `t` seconds with steps of `step` seconds.
pub fn test_looks(patch: &Patch, pattern: TestPattern, t: f64, step: f32) -> Vec<Look> {
    let step = step.max(0.001) as f64;
    let n = (t / step).floor() as usize;
    let within = (t / step).fract();
    let count = patch.fixtures.len().max(1);
    patch
        .fixtures
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let (on, color) = match pattern {
                // Lit for the first three quarters of its step, so a rig of
                // one fixture still blinks.
                TestPattern::Each => (n % count == i && within < 0.75, [1.0; 3]),
                TestPattern::Colours => (
                    true,
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0; 3]][n % 4],
                ),
                TestPattern::Full => (true, [1.0; 3]),
            };
            Look {
                intensity: if on { TEST_LEVEL } else { 0.0 },
                color,
                ..Look::default()
            }
        })
        .collect()
}

/// The safe scene for a patch.
pub fn safe_looks(patch: &Patch) -> Vec<Look> {
    patch
        .fixtures
        .iter()
        .map(|f| {
            let off = f.def.is_laser()
                || (f.def.role() == FixtureRole::Effect && f.def.has(ChannelKind::Strobe));
            Look {
                intensity: if off { 0.0 } else { SAFE_LEVEL },
                color: [1.0, 0.85, 0.7],
                ..Look::default()
            }
        })
        .collect()
}

/// State of one output link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkState {
    Sending,
    /// Not reachable; the rig retries every [`RETRY`] seconds.
    Lost,
}

/// Status of one universe's output, for the UI.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputStatus {
    pub universe: u16,
    pub kind: OutputKind,
    pub state: LinkState,
    /// Frames sent since the output was set.
    pub frames: u64,
    /// Last error ("" while sending).
    pub error: String,
    /// Seconds since the link was lost, or since it came back.
    pub since: f32,
    /// Times the link came back by itself.
    pub reconnects: u32,
}

struct Slot {
    universe: u16,
    kind: OutputKind,
    link: Option<Box<dyn output::Link>>,
    error: String,
    changed_at: f64,
    retry_at: f64,
    frames: u64,
    reconnects: u32,
}

/// A complete lighting rig: patch, show and the outputs per universe.
pub struct Lighting {
    pub patch: Patch,
    pub show: Show,
    slots: Vec<Slot>,
    opener: output::Opener,
    /// Last universes sent, after the safety limits (for the channel view).
    pub frame: Vec<Universe>,
    /// Last looks (for the UI's rig preview), before the limits.
    pub looks: Vec<Look>,
    pub limits: SafetyLimits,
    pub arming: Arming,
    limiter: Limiter,
    pub failsafe: Failsafe,
    /// 0 = the show, 1 = the failsafe scene.
    fail_mix: f32,
    held: Vec<Look>,
    /// Looks of the last frame every link took.
    delivered: Vec<Look>,
    /// Fixture blinking for identification.
    pub identify: Option<usize>,
    /// Test pattern running instead of the show, and when it started.
    pub test: Option<TestPattern>,
    test_started: f64,
    /// Seconds per test step ([`TEST_STEP`]).
    pub test_step: f32,
    clock: f64,
    /// The show panicked; only the failsafe scene is sent from then on.
    pub faulted: bool,
    /// Test hook: the next tick panics inside the show.
    #[doc(hidden)]
    pub inject_panic: bool,
}

impl Lighting {
    pub fn new(patch: Patch, show: Show) -> Self {
        Self::with_opener(patch, show, output::default_opener())
    }

    /// A rig whose outputs are opened by `opener` (tests, fault hook).
    pub fn with_opener(patch: Patch, show: Show, opener: output::Opener) -> Self {
        Self {
            patch,
            show,
            slots: Vec::new(),
            opener,
            frame: Vec::new(),
            looks: Vec::new(),
            limits: SafetyLimits::default(),
            arming: Arming::default(),
            limiter: Limiter::default(),
            failsafe: Failsafe::default(),
            fail_mix: 0.0,
            held: Vec::new(),
            delivered: Vec::new(),
            identify: None,
            test: None,
            test_started: 0.0,
            test_step: TEST_STEP,
            clock: 0.0,
            faulted: false,
            inject_panic: false,
        }
    }

    /// Sends `universe` through `kind`. A link that cannot be opened now is
    /// kept and retried, so an interface plugged in later is picked up.
    pub fn add_output(&mut self, universe: u16, kind: OutputKind) {
        let (link, error) = match (self.opener)(&kind) {
            Ok(l) => (Some(l), String::new()),
            Err(e) => {
                tracing::warn!(universe, "dmx output: {e}");
                (None, e.to_string())
            }
        };
        self.slots.push(Slot {
            universe,
            kind,
            link,
            error,
            changed_at: self.clock,
            retry_at: self.clock + RETRY,
            frames: 0,
            reconnects: 0,
        });
    }

    /// Starts (or stops, with `None`) a test pattern.
    pub fn set_test(&mut self, pattern: Option<TestPattern>) {
        if pattern != self.test {
            self.test = pattern;
            self.test_started = self.clock;
        }
    }

    pub fn clear_outputs(&mut self) {
        self.slots.clear();
    }

    /// Replaces how links are opened (test outputs); applies to outputs
    /// added from now on.
    pub fn set_opener(&mut self, opener: output::Opener) {
        self.opener = opener;
    }

    pub fn status(&self) -> Vec<OutputStatus> {
        self.slots
            .iter()
            .map(|s| OutputStatus {
                universe: s.universe,
                kind: s.kind.clone(),
                state: if s.link.is_some() {
                    LinkState::Sending
                } else {
                    LinkState::Lost
                },
                frames: s.frames,
                error: s.error.clone(),
                since: (self.clock - s.changed_at) as f32,
                reconnects: s.reconnects,
            })
            .collect()
    }

    /// How far the output has gone over to the failsafe scene (0..1).
    pub fn failsafe_mix(&self) -> f32 {
        self.fail_mix
    }

    /// One frame with a guard: a panic inside the show switches the rig to
    /// the failsafe scene instead of taking the lighting thread down.
    pub fn tick_guarded(&mut self, decks: &[DeckMusic], dt: f32) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.tick(decks, dt);
        }));
        if result.is_err() {
            tracing::error!("lighting show failed; sending the failsafe scene");
            self.faulted = true;
            self.tick(decks, dt);
        }
    }

    /// Renders one frame and sends it. Output errors are not fatal: a lost
    /// link is retried, and the rig shows the failsafe scene meanwhile.
    pub fn tick(&mut self, decks: &[DeckMusic], dt: f32) {
        self.clock += dt.max(0.0) as f64;
        let live = if self.faulted {
            self.looks.clone()
        } else {
            if std::mem::take(&mut self.inject_panic) {
                panic!("injected show fault");
            }
            let mut looks = self.show.frame(&self.patch, decks, dt);
            if let Some(pattern) = self.test {
                looks = test_looks(
                    &self.patch,
                    pattern,
                    self.clock - self.test_started,
                    self.test_step,
                );
            }
            if let Some(look) = self.identify.and_then(|i| looks.get_mut(i)) {
                let on = (self.clock * IDENTIFY_HZ as f64).fract() < 0.5;
                *look = Look {
                    intensity: if on { 1.0 } else { 0.0 },
                    color: [1.0; 3],
                    ..Look::default()
                };
            }
            looks
        };
        let lost = self.slots.iter().any(|s| s.link.is_none()) || self.faulted;
        if lost {
            if self.fail_mix == 0.0 {
                self.held = if self.delivered.len() == self.patch.fixtures.len() {
                    self.delivered.clone()
                } else {
                    live.clone()
                };
            }
            self.fail_mix = match self.failsafe {
                Failsafe::Safe => (self.fail_mix + dt / SAFE_FADE).min(1.0),
                Failsafe::Hold | Failsafe::Blackout => 1.0,
            };
        } else {
            self.fail_mix = (self.fail_mix - dt / RECOVER_FADE).max(0.0);
        }
        self.looks = if self.fail_mix > 0.0 {
            let target = self.failsafe_looks();
            live.iter()
                .zip(&target)
                .map(|(a, b)| a.blend(b, self.fail_mix))
                .collect()
        } else {
            live
        };
        let mut frame = render(&self.patch, &self.looks);
        self.limiter
            .apply(&self.patch, &self.limits, self.arming, &mut frame, dt);
        self.frame = frame;
        let clock = self.clock;
        for slot in &mut self.slots {
            if slot.link.is_none() && clock >= slot.retry_at {
                slot.retry_at = clock + RETRY;
                match (self.opener)(&slot.kind) {
                    Ok(l) => {
                        tracing::info!(universe = slot.universe, "dmx output back");
                        slot.link = Some(l);
                        slot.error.clear();
                        slot.changed_at = clock;
                        slot.reconnects += 1;
                    }
                    Err(e) => slot.error = e.to_string(),
                }
            }
            let data = self
                .frame
                .get(slot.universe as usize)
                .copied()
                .unwrap_or([0; 512]);
            if let Some(link) = slot.link.as_mut() {
                match link.send(&data) {
                    Ok(()) => slot.frames += 1,
                    Err(e) => {
                        tracing::warn!(universe = slot.universe, "dmx output lost: {e}");
                        slot.link = None;
                        slot.error = e.to_string();
                        slot.changed_at = clock;
                        slot.retry_at = clock + RETRY;
                    }
                }
            }
        }
        if self.slots.iter().all(|s| s.link.is_some()) {
            self.delivered.clone_from(&self.looks);
        }
    }

    fn failsafe_looks(&self) -> Vec<Look> {
        match self.failsafe {
            Failsafe::Hold => self.held.clone(),
            Failsafe::Safe => safe_looks(&self.patch),
            Failsafe::Blackout => self
                .patch
                .fixtures
                .iter()
                .map(|_| Look::default())
                .collect(),
        }
        .into_iter()
        .chain(std::iter::repeat(Look::default()))
        .take(self.patch.fixtures.len())
        .collect()
    }

    /// Last frame when the app quits: the failsafe choice (Hold sends
    /// nothing more, so the interfaces keep the last frame).
    pub fn shutdown(&mut self) {
        let looks = match self.failsafe {
            Failsafe::Hold => return,
            Failsafe::Safe => safe_looks(&self.patch),
            Failsafe::Blackout => vec![Look::default(); self.patch.fixtures.len()],
        };
        let mut frame = render(&self.patch, &looks);
        self.limiter
            .apply(&self.patch, &self.limits, self.arming, &mut frame, 0.0);
        for slot in &mut self.slots {
            let data = frame
                .get(slot.universe as usize)
                .copied()
                .unwrap_or([0; 512]);
            if let Some(link) = slot.link.as_mut() {
                let _ = link.send(&data);
            }
        }
        self.frame = frame;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rig_config_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lighting.json");
        assert_eq!(RigConfig::load(&path).unwrap(), RigConfig::default());
        let mut rig = RigConfig {
            enabled: true,
            ..Default::default()
        };
        rig.outputs.push((0, OutputKind::Sacn { universe: 1 }));
        rig.settings.mode = Mode::Pro;
        rig.settings.groups.insert(
            "Movers".into(),
            GroupSettings {
                program: Some(Program::Sweep),
                ..Default::default()
            },
        );
        rig.save(&path).unwrap();
        assert_eq!(RigConfig::load(&path).unwrap(), rig);
    }

    #[test]
    fn tick_renders_and_sends() {
        let rx = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        rx.set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut patch = Patch::default();
        patch
            .add(Patched {
                name: "Par".into(),
                def: FixtureDef::builtin("rgbw-par").unwrap(),
                universe: 0,
                address: 1,
                group: "Wash".into(),
                invert_pan: false,
            })
            .unwrap();
        let mut rig = Lighting::new(patch, Show::default());
        rig.add_output(
            0,
            OutputKind::ArtNet {
                target: rx.local_addr().unwrap(),
                universe: 0,
            },
        );
        rig.tick(&[], 0.025);
        let mut buf = [0u8; 600];
        let n = rx.recv(&mut buf).unwrap();
        assert_eq!(n, 530);
        assert_eq!(buf[18], rig.frame[0][0]);
        assert!(rig.frame[0][0] > 0, "idle wash is on");
    }

    fn par_rig(link: &TestLink, failsafe: Failsafe) -> Lighting {
        let mut patch = Patch::default();
        let def = FixtureDef::builtin("rgbw-par").unwrap();
        patch
            .add_many(&def, 1, Some(0), Some(1), "Par", "Wash")
            .unwrap();
        let mut show = Show::default();
        show.settings.mode = Mode::Easy;
        show.settings.easy_program = Program::Static;
        let mut rig = Lighting::with_opener(patch, show, link.opener());
        rig.failsafe = failsafe;
        rig.add_output(0, OutputKind::Sacn { universe: 1 });
        rig
    }

    fn run(rig: &mut Lighting, secs: f32) {
        for _ in 0..(secs * 40.0) as usize {
            rig.tick(&[], 0.025);
        }
    }

    #[test]
    fn lost_link_is_shown_and_recovers_by_itself() {
        let link = TestLink::default();
        let mut rig = par_rig(&link, Failsafe::Blackout);
        run(&mut rig, 0.5);
        let live = link.last().unwrap()[0];
        assert!(live > 0, "show on");
        assert_eq!(rig.status()[0].state, LinkState::Sending);
        // Pull the interface.
        link.set_unplugged(true);
        let sent = link.count();
        run(&mut rig, 2.5);
        let st = &rig.status()[0];
        assert_eq!(st.state, LinkState::Lost);
        assert!(st.error.contains("unplugged"), "{}", st.error);
        assert!(st.since > 2.0);
        assert_eq!(link.count(), sent, "nothing arrives while unplugged");
        assert_eq!(rig.frame[0][0], 0, "rig follows the failsafe (blackout)");
        // Plug it back: the rig reconnects within the retry time and fades
        // back to the show.
        link.set_unplugged(false);
        run(&mut rig, 1.1);
        let st = &rig.status()[0];
        assert_eq!((st.state, st.reconnects), (LinkState::Sending, 1));
        assert!(link.count() > sent, "frames arrive again");
        let first = link.frames.lock().unwrap()[sent][0];
        assert_eq!(first, 0, "starts from the failsafe");
        run(&mut rig, RECOVER_FADE + 0.1);
        assert_eq!(rig.failsafe_mix(), 0.0);
        assert_eq!(link.last().unwrap()[0], live, "back to the show");
    }

    #[test]
    fn failsafe_choices() {
        // Hold: the look freezes while lost.
        let link = TestLink::default();
        let mut rig = par_rig(&link, Failsafe::Hold);
        run(&mut rig, 0.2);
        let before = rig.frame[0][0];
        rig.show.settings.master = 0.2;
        link.set_unplugged(true);
        run(&mut rig, 0.5);
        assert_eq!(rig.frame[0][0], before, "hold keeps the last look");
        // Safe: fades over SAFE_FADE to the safe level.
        let link = TestLink::default();
        let mut rig = par_rig(&link, Failsafe::Safe);
        run(&mut rig, 0.2);
        link.set_unplugged(true);
        run(&mut rig, 1.0);
        let mid = rig.frame[0][0];
        run(&mut rig, SAFE_FADE);
        let safe = (SAFE_LEVEL * 255.0).round() as u8;
        assert!(mid > safe, "still fading ({mid})");
        assert_eq!(rig.frame[0][0], safe);
    }

    #[test]
    fn interface_missing_at_start_is_picked_up_later() {
        let link = TestLink::default();
        link.set_unplugged(true);
        let mut rig = par_rig(&link, Failsafe::Hold);
        assert_eq!(rig.status()[0].state, LinkState::Lost);
        run(&mut rig, 0.5);
        link.set_unplugged(false);
        run(&mut rig, 1.0);
        assert_eq!(rig.status()[0].state, LinkState::Sending);
        assert!(link.count() > 0);
    }

    #[test]
    fn show_fault_and_quit_send_the_failsafe() {
        let link = TestLink::default();
        let mut rig = par_rig(&link, Failsafe::Safe);
        run(&mut rig, 0.2);
        rig.inject_panic = true;
        rig.tick_guarded(&[], 0.025);
        assert!(rig.faulted, "the panic was caught");
        for _ in 0..100 {
            rig.tick_guarded(&[], 0.025);
        }
        assert_eq!(rig.frame[0][0], (SAFE_LEVEL * 255.0).round() as u8);
        // Quit with Blackout sends a dark frame; with Hold, nothing.
        rig.failsafe = Failsafe::Blackout;
        rig.shutdown();
        assert_eq!(link.last().unwrap()[0], 0);
        rig.failsafe = Failsafe::Hold;
        let n = link.count();
        rig.shutdown();
        assert_eq!(link.count(), n);
    }

    #[test]
    fn identify_blinks_one_fixture() {
        let link = TestLink::default();
        let mut rig = par_rig(&link, Failsafe::Hold);
        rig.show.settings.blackout = true;
        rig.identify = Some(0);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..80 {
            rig.tick(&[], 0.025);
            seen.insert(rig.frame[0][0]);
        }
        assert!(seen.contains(&0) && seen.contains(&255), "{seen:?}");
    }

    #[test]
    fn test_pattern_steps_through_the_fixtures_and_stops() {
        let link = TestLink::default();
        let mut rig = par_rig(&link, Failsafe::Hold);
        let def = FixtureDef::builtin("rgbw-par").unwrap();
        rig.patch
            .add_many(&def, 2, Some(0), Some(20), "Par", "Wash")
            .unwrap();
        rig.tick(&[], 0.025);
        rig.set_test(Some(TestPattern::Each));
        let mut lit_frames = 0;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..1200 {
            rig.tick(&[], 0.025);
            let lit: Vec<usize> = (0..3).filter(|i| rig.looks[*i].intensity > 0.5).collect();
            assert!(lit.len() <= 1, "{lit:?}");
            if let Some(i) = lit.first() {
                lit_frames += 1;
                seen.insert(*i);
            }
        }
        assert_eq!(seen.len(), 3, "every fixture in turn");
        assert!(lit_frames > 800, "lit three quarters of the time ({lit_frames})");
        rig.set_test(None);
        rig.tick(&[], 0.025);
        assert!(rig.looks[0].intensity < 0.5 || rig.show.settings.mode == Mode::Easy);
    }
}
