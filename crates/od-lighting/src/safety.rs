//! Safety limits, enforced on the finished DMX frame.
//!
//! Every source of light (the automatic show, Easy and Pro mode, the
//! momentary buttons, cues fired by tracks, the remote inbox, identify and
//! the failsafe scenes) ends up as rendered universes. [`Limiter::apply`]
//! runs on those bytes right before they are sent, so no source can get
//! around it:
//!
//! * **Strobe rate cap.** Strobe channels are written in the generic layout
//!   of [`crate::render`]: 255 open, 64..=254 strobing from slow to fast. The
//!   cap assumes the fastest value means [`STROBE_CHANNEL_MAX_HZ`] flashes a
//!   second and lowers the value to the capped rate.
//! * **Flash rate cap.** Software flashing (a dimmer jumping from dark to
//!   bright) is held to the same rate per fixture: a fixture that went dark
//!   may not come back bright sooner than one cap period after its last
//!   flash. This covers fast chases and square-wave effects on fixtures
//!   without a strobe channel.
//! * **Level cap per group.** The intensity of every fixture of a group is
//!   scaled down to the group's cap.
//! * **Fog and laser lockout.** Fog channels and every channel of a laser
//!   stay at zero until fog or lasers are armed. Arming is not saved: each
//!   start of the app begins disarmed.

use std::collections::HashMap;

use crate::fixture::{ChannelKind, Patch, Patched};
use crate::output::Universe;

/// Strobe rate assumed for the fastest strobe value (flashes per second).
/// Real fixtures differ; the guide says so.
pub const STROBE_CHANNEL_MAX_HZ: f32 = 25.0;

/// Default strobe and flash cap. Flashing at 3 per second or slower keeps
/// below the range most likely to trigger photosensitive seizures (common
/// guidance puts the risk from about 3 flashes a second upwards).
pub const DEFAULT_STROBE_HZ: f32 = 3.0;

/// Highest cap the settings allow.
pub const MAX_STROBE_HZ: f32 = 10.0;

/// Dark and bright thresholds of the flash counter (0..255).
const DARK: u8 = 64;
const BRIGHT: u8 = 128;

/// The limits a rig keeps in `lighting.json`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SafetyLimits {
    /// Strobe and flash cap in flashes per second (0 = no strobe at all).
    pub strobe_hz: f32,
    /// Level cap per group, 0..1 (missing = no cap).
    pub group_caps: HashMap<String, f32>,
    /// PIN that keeps guests out of setup (empty = no PIN).
    pub pin: String,
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self {
            strobe_hz: DEFAULT_STROBE_HZ,
            group_caps: HashMap::new(),
            pin: String::new(),
        }
    }
}

impl SafetyLimits {
    /// Strobe cap clamped to the allowed range.
    pub fn strobe_cap(&self) -> f32 {
        if self.strobe_hz.is_finite() {
            self.strobe_hz.clamp(0.0, MAX_STROBE_HZ)
        } else {
            DEFAULT_STROBE_HZ
        }
    }

    pub fn group_cap(&self, group: &str) -> f32 {
        self.group_caps
            .get(group)
            .copied()
            .filter(|c| c.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    }
}

/// What is armed right now (never saved).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Arming {
    pub fog: bool,
    pub laser: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Flash {
    /// Last seen bright (after a dark phase).
    bright: bool,
    /// Time of the last allowed flash, if any.
    last: Option<f64>,
}

/// Applies [`SafetyLimits`] to rendered universes; keeps the flash timing
/// per fixture between frames.
#[derive(Debug, Default)]
pub struct Limiter {
    clock: f64,
    flashes: Vec<Flash>,
}

fn is_level(kind: ChannelKind) -> bool {
    use ChannelKind::*;
    matches!(
        kind,
        Dimmer
            | Red
            | Green
            | Blue
            | White
            | Amber
            | Uv
            | PixelRed(_)
            | PixelGreen(_)
            | PixelBlue(_)
    )
}

/// Channels that carry a fixture's brightness: its dimmers, or its colour
/// channels when it has no dimmer.
fn level_slots(f: &Patched) -> Vec<usize> {
    let has_dimmer = f.def.has(ChannelKind::Dimmer);
    f.def
        .channels
        .iter()
        .enumerate()
        .filter(|(_, k)| {
            if has_dimmer {
                **k == ChannelKind::Dimmer
            } else {
                is_level(**k)
            }
        })
        .map(|(i, _)| f.address as usize - 1 + i)
        .filter(|s| *s < 512)
        .collect()
}

impl Limiter {
    /// Limits `universes` in place. `dt` is the time since the last frame.
    pub fn apply(
        &mut self,
        patch: &Patch,
        limits: &SafetyLimits,
        arming: Arming,
        universes: &mut [Universe],
        dt: f32,
    ) {
        self.clock += dt.max(0.0) as f64;
        self.flashes.resize(patch.fixtures.len(), Flash::default());
        let cap_hz = limits.strobe_cap();
        let strobe_max = if cap_hz <= 0.0 {
            None
        } else {
            Some(64 + ((cap_hz / STROBE_CHANNEL_MAX_HZ).min(1.0) * 190.0).floor() as u8)
        };
        // Flash period: 0 Hz still allows one flash a second (a look change).
        let period = 1.0 / cap_hz.max(1.0) as f64;
        for (i, f) in patch.fixtures.iter().enumerate() {
            let Some(u) = universes.get_mut(f.universe as usize) else {
                continue;
            };
            if f.address == 0 {
                continue;
            }
            let base = f.address as usize - 1;
            let slots = || {
                f.def
                    .channels
                    .iter()
                    .enumerate()
                    .map(move |(n, k)| (base + n, *k))
                    .filter(|(s, _)| *s < 512)
            };
            // Lockout.
            if f.def.is_laser() && !arming.laser {
                for (s, _) in slots() {
                    u[s] = 0;
                }
                continue;
            }
            if !arming.fog {
                for (s, k) in slots() {
                    if k == ChannelKind::Fog {
                        u[s] = 0;
                    }
                }
            }
            // Strobe rate.
            for (s, k) in slots() {
                if k == ChannelKind::Strobe && (64..=254).contains(&u[s]) {
                    u[s] = match strobe_max {
                        Some(max) => u[s].min(max),
                        None => 255,
                    };
                }
            }
            // Level cap.
            let levels = level_slots(f);
            let cap = (limits.group_cap(&f.group) * 255.0).round() as u8;
            let level = levels.iter().map(|s| u[*s]).max().unwrap_or(0);
            if level > cap {
                let k = cap as f32 / level as f32;
                for s in &levels {
                    u[*s] = (u[*s] as f32 * k).floor() as u8;
                }
            }
            // Flash rate.
            let level = levels.iter().map(|s| u[*s]).max().unwrap_or(0);
            let flash = &mut self.flashes[i];
            if flash.bright {
                if level < DARK {
                    flash.bright = false;
                }
            } else if level > BRIGHT {
                if flash.last.is_some_and(|t| self.clock - t < period - 1e-6) {
                    // Too soon: stay dark until the period is over.
                    let k = (DARK - 1) as f32 / level as f32;
                    for s in &levels {
                        u[*s] = (u[*s] as f32 * k).floor() as u8;
                    }
                } else {
                    flash.bright = true;
                    flash.last = Some(self.clock);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::FixtureDef;
    use crate::render::{Look, render};
    use crate::show::{Effect, GroupSettings, Mode, Momentary, Program, Show, ShowSettings, Wave};
    use crate::{Attr, DeckMusic};

    fn rig(kinds: &[(&str, &str)]) -> Patch {
        let mut patch = Patch::default();
        for (kind, group) in kinds {
            let def = FixtureDef::builtin(kind).unwrap();
            patch.add_many(&def, 1, Some(0), None, "", group).unwrap();
        }
        patch
    }

    /// Flashes (dark → bright) of fixture `index` over `secs` seconds of
    /// the show at 40 frames per second, after the limiter.
    fn count_flashes(
        patch: &Patch,
        show: &mut Show,
        limits: &SafetyLimits,
        decks: &[DeckMusic],
        secs: f32,
        index: usize,
    ) -> (usize, Vec<u8>) {
        let mut limiter = Limiter::default();
        let dt = 1.0 / 40.0;
        let f = &patch.fixtures[index];
        let slots = level_slots(f);
        let mut bright = false;
        let mut flashes = 0;
        let mut strobe_values = Vec::new();
        let mut decks = decks.to_vec();
        for _ in 0..(secs / dt) as usize {
            for d in &mut decks {
                d.beat += dt as f64 * d.bpm / 60.0;
            }
            let looks = show.frame(patch, &decks, dt);
            let mut u = render(patch, &looks);
            limiter.apply(patch, limits, Arming::default(), &mut u, dt);
            let level = slots.iter().map(|s| u[0][*s]).max().unwrap();
            if !bright && level > BRIGHT {
                flashes += 1;
                bright = true;
            } else if bright && level < DARK {
                bright = false;
            }
            for (n, k) in f.def.channels.iter().enumerate() {
                if *k == ChannelKind::Strobe {
                    strobe_values.push(u[0][f.address as usize - 1 + n]);
                }
            }
        }
        (flashes, strobe_values)
    }

    fn playing(bpm: f64) -> DeckMusic {
        DeckMusic {
            bpm,
            beat: 0.0,
            energy: 1.0,
            weight: 1.0,
            playing: true,
            ..Default::default()
        }
    }

    #[test]
    fn strobe_button_cannot_exceed_the_cap() {
        let patch = rig(&[("strobe", "Strobes"), ("rgbw-par", "Wash")]);
        let mut show = Show::default();
        show.momentary = Momentary {
            strobe: true,
            ..Default::default()
        };
        let limits = SafetyLimits::default();
        let (_, values) = count_flashes(&patch, &mut show, &limits, &[], 2.0, 0);
        let max = values.iter().copied().filter(|v| *v != 255).max().unwrap();
        let hz = (max - 64) as f32 / 190.0 * STROBE_CHANNEL_MAX_HZ;
        assert!(hz <= DEFAULT_STROBE_HZ + 0.01, "strobe at {hz} Hz");
        assert!(max > 64, "still strobes, just slower");
        // Cap 0: strobe channels stay open.
        let off = SafetyLimits {
            strobe_hz: 0.0,
            ..Default::default()
        };
        let (_, values) = count_flashes(&patch, &mut show, &off, &[], 1.0, 0);
        assert!(values.iter().all(|v| *v == 255));
    }

    #[test]
    fn fast_dimmer_effects_are_held_to_the_cap() {
        // A square wave on the dimmer every quarter beat at 140 BPM would
        // flash about 9 times a second.
        let patch = rig(&[("rgbw-par", "Wash")]);
        let mut settings = ShowSettings {
            mode: Mode::Pro,
            ..Default::default()
        };
        settings.groups.insert(
            "Wash".into(),
            GroupSettings {
                program: Some(Program::Pulse),
                effect: Some(Effect {
                    attr: Attr::Dimmer,
                    wave: Wave::Square,
                    beats: 0.25,
                    depth: 1.0,
                    spread: 0.0,
                }),
                ..Default::default()
            },
        );
        let deck = [playing(140.0)];
        let unlimited = SafetyLimits {
            strobe_hz: MAX_STROBE_HZ,
            ..Default::default()
        };
        let (fast, _) = count_flashes(
            &patch,
            &mut Show::new(settings.clone()),
            &unlimited,
            &deck,
            10.0,
            0,
        );
        assert!(fast > 40, "the effect itself flashes fast ({fast} in 10 s)");
        let (capped, _) = count_flashes(
            &patch,
            &mut Show::new(settings),
            &SafetyLimits::default(),
            &deck,
            10.0,
            0,
        );
        assert!(capped <= 30 && capped > 0, "{capped} flashes in 10 s");
    }

    #[test]
    fn every_automatic_program_stays_within_the_limits() {
        let patch = rig(&[
            ("rgbw-par", "Wash"),
            ("strobe", "Strobes"),
            ("light-bar-8", "Bars"),
            ("moving-head", "Movers"),
            ("blinder", "Blinders"),
            ("fog", "Fog"),
            ("laser", "Laser"),
        ]);
        let mut limits = SafetyLimits::default();
        limits.group_caps.insert("Blinders".into(), 0.5);
        for program in Program::ALL {
            let mut settings = ShowSettings {
                mode: Mode::Easy,
                easy_program: program,
                ..Default::default()
            };
            settings.master = 1.0;
            let mut show = Show::new(settings);
            show.momentary.fog = true;
            let mut limiter = Limiter::default();
            let dt = 1.0 / 40.0;
            let mut deck = playing(174.0);
            let mut last_flash = vec![None::<f32>; patch.fixtures.len()];
            let mut bright = vec![false; patch.fixtures.len()];
            for frame in 0..400 {
                let t = frame as f32 * dt;
                deck.beat += dt as f64 * deck.bpm / 60.0;
                let looks = show.frame(&patch, &[deck], dt);
                let mut u = render(&patch, &looks);
                limiter.apply(&patch, &limits, Arming::default(), &mut u, dt);
                for (i, f) in patch.fixtures.iter().enumerate() {
                    let base = f.address as usize - 1;
                    for (n, k) in f.def.channels.iter().enumerate() {
                        let v = u[0][base + n];
                        if *k == ChannelKind::Strobe && v != 255 && v >= 64 {
                            let hz = (v - 64) as f32 / 190.0 * STROBE_CHANNEL_MAX_HZ;
                            assert!(hz <= 3.01, "{program:?} strobes {} at {hz}", f.name);
                        }
                        if *k == ChannelKind::Fog || f.def.is_laser() {
                            assert_eq!(v, 0, "{program:?}: {} not locked out", f.name);
                        }
                    }
                    let level = level_slots(f).iter().map(|s| u[0][*s]).max().unwrap_or(0);
                    if f.group == "Blinders" {
                        assert!(level <= 128, "{program:?}: blinder at {level}");
                    }
                    if !bright[i] && level > BRIGHT {
                        if let Some(prev) = last_flash[i] {
                            assert!(
                                t - prev >= 1.0 / 3.0 - 0.03,
                                "{program:?}: {} flashed after {} s",
                                f.name,
                                t - prev
                            );
                        }
                        last_flash[i] = Some(t);
                        bright[i] = true;
                    } else if bright[i] && level < DARK {
                        bright[i] = false;
                    }
                }
            }
        }
    }

    #[test]
    fn fog_and_laser_need_arming() {
        let patch = rig(&[("fog", "Fog"), ("laser", "Laser")]);
        let looks = vec![
            Look {
                fog: true,
                ..Look::default()
            },
            Look {
                intensity: 1.0,
                ..Look::default()
            },
        ];
        let limits = SafetyLimits::default();
        let mut limiter = Limiter::default();
        let mut u = render(&patch, &looks);
        assert_eq!((u[0][0], u[0][1]), (255, 255), "rendered on");
        limiter.apply(&patch, &limits, Arming::default(), &mut u, 0.025);
        assert_eq!((u[0][0], u[0][1]), (0, 0), "locked out");
        let mut u = render(&patch, &looks);
        limiter.apply(
            &patch,
            &limits,
            Arming {
                fog: true,
                laser: false,
            },
            &mut u,
            0.025,
        );
        assert_eq!((u[0][0], u[0][1]), (255, 0));
        let mut u = render(&patch, &looks);
        limiter.apply(
            &patch,
            &limits,
            Arming {
                fog: true,
                laser: true,
            },
            &mut u,
            0.025,
        );
        assert_eq!((u[0][0], u[0][1]), (255, 255));
    }

    #[test]
    fn group_cap_scales_colour_without_a_dimmer() {
        let patch = rig(&[("rgb-par", "Front")]);
        let mut limits = SafetyLimits::default();
        limits.group_caps.insert("Front".into(), 0.4);
        let look = Look {
            intensity: 1.0,
            color: [1.0, 0.5, 0.0],
            ..Look::default()
        };
        let mut u = render(&patch, &[look]);
        Limiter::default().apply(&patch, &limits, Arming::default(), &mut u, 0.0);
        assert_eq!(&u[0][0..3], &[102, 51, 0], "hue kept, level capped");
        let old: SafetyLimits = serde_json::from_str("{}").unwrap();
        assert_eq!(old, SafetyLimits::default());
    }
}
