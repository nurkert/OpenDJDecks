//! Own looks and chases on pads, group masters, and the steering of AUTO.
//!
//! * A [`PadLook`] sits on one of [`PADS`] pads, [`PAGE_PADS`] to a page.
//!   A plain look sets some groups (program, colour, level, effect,
//!   position); the groups it leaves out keep what runs underneath. A chase
//!   steps through other pads' looks on the beat.
//! * Each pad either **latches** (on until pressed again or released) or
//!   **flashes** (on while held).
//! * AUTO picks a look for each kind of phrase from an editable table
//!   ([`AutoTable`]), takes its colours from a [`Palette`], and can be held
//!   on its current look ([`PadState::hold`]).
//!
//! What wins, from the top ([`Layer`]): blackout, the FLASH and STROBE
//! keys, flash pads while held (the last one pressed), the latched pad,
//! HOLD, then AUTO (or EASY / PRO). Group masters and the master dimmer
//! scale the result, and the safety limits apply to every frame last.

use std::collections::HashMap;

use od_core::{PhraseKind, PhraseMood};
use serde::{Deserialize, Serialize};

use crate::show::{GroupSettings, Program};

/// Pads on one page (the eight pads of a deck or a controller row).
pub const PAGE_PADS: usize = 8;
/// Pages of pads.
pub const PAGES: usize = 4;
/// All pads.
pub const PADS: usize = PAGE_PADS * PAGES;

/// Beats per chase step offered in the editor.
pub const STEP_BEATS: [f32; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

/// How a pad behaves when pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadAction {
    /// On until pressed again (or RELEASE).
    #[default]
    Latch,
    /// On while held.
    Flash,
}

fn default_step_beats() -> f32 {
    1.0
}

/// A look or chase stored on a pad.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PadLook {
    /// Pad number, 0..[`PADS`] (page = pad / [`PAGE_PADS`]).
    pub pad: u16,
    pub name: String,
    #[serde(default)]
    pub action: PadAction,
    /// What the look sets per group; groups not listed keep what runs
    /// underneath.
    #[serde(default)]
    pub groups: HashMap<String, GroupSettings>,
    /// A chase: the pads whose looks it steps through.
    #[serde(default)]
    pub steps: Vec<u16>,
    /// Made as a chase (it may have no steps yet).
    #[serde(default)]
    pub chase: bool,
    /// Beats per chase step.
    #[serde(default = "default_step_beats")]
    pub step_beats: f32,
}

impl PadLook {
    pub fn new(pad: u16, name: &str) -> Self {
        Self {
            pad,
            name: name.to_string(),
            action: PadAction::Latch,
            groups: HashMap::new(),
            steps: Vec::new(),
            chase: false,
            step_beats: 1.0,
        }
    }

    pub fn is_chase(&self) -> bool {
        self.chase || !self.steps.is_empty()
    }
}

/// The look on `pad`, if any.
pub fn find(pads: &[PadLook], pad: u16) -> Option<&PadLook> {
    pads.iter().find(|p| p.pad == pad)
}

/// What the look on `pad` sets per group at `beat`: the look itself, or the
/// current step of a chase (a step that is itself a chase, or empty, is
/// skipped over).
pub fn groups_at(pads: &[PadLook], pad: u16, beat: f64) -> Option<&HashMap<String, GroupSettings>> {
    let look = find(pads, pad)?;
    if !look.is_chase() {
        return Some(&look.groups);
    }
    let steps: Vec<&PadLook> = look
        .steps
        .iter()
        .filter_map(|s| find(pads, *s))
        .filter(|l| !l.is_chase())
        .collect();
    if steps.is_empty() {
        return None;
    }
    let len = look.step_beats.max(0.0625) as f64;
    let i = (beat.max(0.0) / len).floor() as usize % steps.len();
    Some(&steps[i].groups)
}

/// Which chase step is current (for the editor), if `pad` is a chase.
pub fn chase_step(pads: &[PadLook], pad: u16, beat: f64) -> Option<usize> {
    let look = find(pads, pad).filter(|l| !l.steps.is_empty())?;
    let len = look.step_beats.max(0.0625) as f64;
    Some((beat.max(0.0) / len).floor() as usize % look.steps.len())
}

/// Pads pressed and held right now, and HOLD (never saved).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PadState {
    /// The latched pad.
    pub latched: Option<u16>,
    /// Flash pads held, in the order pressed (the last one wins).
    pub flashing: Vec<u16>,
    /// AUTO keeps its current look; phrase changes do not change it.
    pub hold: bool,
}

impl PadState {
    /// A pad pressed or released. Returns false when the pad is empty.
    pub fn press(&mut self, pads: &[PadLook], pad: u16, pressed: bool) -> bool {
        let Some(look) = find(pads, pad) else {
            return false;
        };
        match look.action {
            PadAction::Flash => {
                self.flashing.retain(|p| *p != pad);
                if pressed {
                    self.flashing.push(pad);
                }
            }
            PadAction::Latch => {
                if pressed {
                    self.latched = if self.latched == Some(pad) {
                        None
                    } else {
                        Some(pad)
                    };
                }
            }
        }
        true
    }

    /// Hands everything back to the music: the latched pad off, HOLD off.
    /// Flash pads end by themselves when let go.
    pub fn release(&mut self) {
        self.latched = None;
        self.hold = false;
    }

    /// Forgets pads that no longer hold a look.
    pub fn retain(&mut self, pads: &[PadLook]) {
        if self.latched.is_some_and(|p| find(pads, p).is_none()) {
            self.latched = None;
        }
        self.flashing.retain(|p| find(pads, *p).is_some());
    }

    /// The pad in control (a flash pad over the latched one).
    pub fn top(&self) -> Option<u16> {
        self.flashing.last().copied().or(self.latched)
    }
}

/// The kinds of phrase AUTO has a look for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhraseSlot {
    Intro,
    Verse,
    BuildUp,
    /// Chorus or drop.
    Drop,
    /// Breakdown or bridge.
    Breakdown,
    Outro,
}

impl PhraseSlot {
    pub const ALL: [PhraseSlot; 6] = [
        PhraseSlot::Intro,
        PhraseSlot::Verse,
        PhraseSlot::BuildUp,
        PhraseSlot::Drop,
        PhraseSlot::Breakdown,
        PhraseSlot::Outro,
    ];

    pub fn of(kind: Option<PhraseKind>) -> Option<PhraseSlot> {
        Some(match kind? {
            PhraseKind::Intro => PhraseSlot::Intro,
            PhraseKind::Verse => PhraseSlot::Verse,
            PhraseKind::Up => PhraseSlot::BuildUp,
            PhraseKind::Chorus => PhraseSlot::Drop,
            PhraseKind::Bridge | PhraseKind::Down => PhraseSlot::Breakdown,
            PhraseKind::Outro => PhraseSlot::Outro,
        })
    }

    /// The program AUTO has always used for this phrase.
    pub fn default_program(self) -> Program {
        match self {
            PhraseSlot::Intro => Program::Breathe,
            PhraseSlot::Verse => Program::Chase,
            PhraseSlot::BuildUp => Program::Build,
            PhraseSlot::Drop => Program::Sweep,
            PhraseSlot::Breakdown => Program::Breathe,
            PhraseSlot::Outro => Program::Static,
        }
    }
}

/// What AUTO plays for a phrase: a built-in program or a pad's look or
/// chase.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AutoPick {
    Program(Program),
    Pad(u16),
}

/// The editable phrase-to-look table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoTable {
    pub intro: AutoPick,
    pub verse: AutoPick,
    pub build_up: AutoPick,
    pub drop: AutoPick,
    pub breakdown: AutoPick,
    pub outro: AutoPick,
}

impl Default for AutoTable {
    fn default() -> Self {
        let p = |s: PhraseSlot| AutoPick::Program(s.default_program());
        Self {
            intro: p(PhraseSlot::Intro),
            verse: p(PhraseSlot::Verse),
            build_up: p(PhraseSlot::BuildUp),
            drop: p(PhraseSlot::Drop),
            breakdown: p(PhraseSlot::Breakdown),
            outro: p(PhraseSlot::Outro),
        }
    }
}

impl AutoTable {
    pub fn get(&self, slot: PhraseSlot) -> AutoPick {
        match slot {
            PhraseSlot::Intro => self.intro,
            PhraseSlot::Verse => self.verse,
            PhraseSlot::BuildUp => self.build_up,
            PhraseSlot::Drop => self.drop,
            PhraseSlot::Breakdown => self.breakdown,
            PhraseSlot::Outro => self.outro,
        }
    }

    pub fn set(&mut self, slot: PhraseSlot, pick: AutoPick) {
        *match slot {
            PhraseSlot::Intro => &mut self.intro,
            PhraseSlot::Verse => &mut self.verse,
            PhraseSlot::BuildUp => &mut self.build_up,
            PhraseSlot::Drop => &mut self.drop,
            PhraseSlot::Breakdown => &mut self.breakdown,
            PhraseSlot::Outro => &mut self.outro,
        } = pick;
    }

    /// The pick for a phrase (no phrase: a steady wash).
    pub fn pick(&self, kind: Option<PhraseKind>) -> AutoPick {
        PhraseSlot::of(kind)
            .map(|s| self.get(s))
            .unwrap_or(AutoPick::Program(Program::Static))
    }

    /// Picks that point at `pad` go back to the phrase's own program.
    pub fn forget_pad(&mut self, pad: u16) {
        for s in PhraseSlot::ALL {
            if self.get(s) == AutoPick::Pad(pad) {
                self.set(s, AutoPick::Program(s.default_program()));
            }
        }
    }
}

/// Where AUTO takes its colours from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Palette {
    /// From the key of the track (harmonic neighbours look alike).
    #[default]
    Track,
    Warm,
    Cool,
    Sunset,
    Ocean,
    White,
    /// The colours chosen in the view.
    Custom,
}

impl Palette {
    pub const ALL: [Palette; 7] = [
        Palette::Track,
        Palette::Warm,
        Palette::Cool,
        Palette::Sunset,
        Palette::Ocean,
        Palette::White,
        Palette::Custom,
    ];

    /// The fixed colours of a built-in palette (empty for Track and Custom).
    pub fn colors(self) -> &'static [[f32; 3]] {
        match self {
            Palette::Track | Palette::Custom => &[],
            Palette::Warm => &[[1.0, 0.55, 0.1], [1.0, 0.25, 0.05], [1.0, 0.8, 0.4]],
            Palette::Cool => &[[0.0, 0.8, 1.0], [0.1, 0.3, 1.0], [0.5, 0.2, 1.0]],
            Palette::Sunset => &[[1.0, 0.1, 0.3], [1.0, 0.45, 0.0], [0.7, 0.1, 1.0]],
            Palette::Ocean => &[[0.0, 0.9, 0.8], [0.0, 0.5, 1.0], [0.2, 1.0, 0.6]],
            Palette::White => &[[1.0, 1.0, 1.0], [1.0, 0.85, 0.7]],
        }
    }
}

/// How AUTO is steered; saved with the rig.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoSettings {
    pub palette: Palette,
    /// Colours of [`Palette::Custom`].
    pub custom: Vec<[f32; 3]>,
    pub table: AutoTable,
}

/// Beats per palette colour: the colour moves on every eight bars.
pub const PALETTE_BEATS: f64 = 32.0;

impl AutoSettings {
    /// The colour of the moment, or `None` to take it from the track.
    pub fn color(&self, mood: Option<PhraseMood>, beat: f64) -> Option<[f32; 3]> {
        let colors: &[[f32; 3]] = match self.palette {
            Palette::Track => return None,
            Palette::Custom => &self.custom,
            p => p.colors(),
        };
        if colors.is_empty() {
            return None;
        }
        let mut i = (beat.max(0.0) / PALETTE_BEATS).floor() as usize;
        // High-energy phrases take the next colour every other bar pair.
        if matches!(mood, Some(PhraseMood::High)) && (beat / 4.0).floor() as i64 % 2 == 1 {
            i += 1;
        }
        Some(colors[i % colors.len()])
    }
}

/// Who is in control of the rig right now, from the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Layer {
    Blackout,
    /// The FLASH or STROBE key is held.
    Momentary,
    /// A flash pad is held.
    FlashPad(u16),
    Latched(u16),
    /// AUTO holds its look.
    Hold,
    /// AUTO, EASY or PRO underneath.
    Base,
}

/// The layer in control (see the module docs for the order).
pub fn layer(blackout: bool, momentary: bool, pads: &PadState, looks: &[PadLook]) -> Layer {
    if blackout {
        return Layer::Blackout;
    }
    if momentary {
        return Layer::Momentary;
    }
    if let Some(p) = pads
        .flashing
        .iter()
        .rev()
        .find(|p| find(looks, **p).is_some())
    {
        return Layer::FlashPad(*p);
    }
    if let Some(p) = pads.latched.filter(|p| find(looks, *p).is_some()) {
        return Layer::Latched(p);
    }
    if pads.hold {
        return Layer::Hold;
    }
    Layer::Base
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(pad: u16, color: [f32; 3]) -> PadLook {
        let mut l = PadLook::new(pad, &format!("L{pad}"));
        l.groups.insert(
            "Wash".into(),
            GroupSettings {
                color: Some(color),
                ..Default::default()
            },
        );
        l
    }

    #[test]
    fn chase_steps_on_the_beat_and_skips_missing_steps() {
        let mut chase = PadLook::new(10, "Chase");
        chase.steps = vec![0, 99, 1];
        chase.step_beats = 2.0;
        let pads = vec![look(0, [1.0, 0.0, 0.0]), look(1, [0.0, 0.0, 1.0]), chase];
        let color = |beat| groups_at(&pads, 10, beat).unwrap()["Wash"].color.unwrap();
        assert_eq!(color(0.5), [1.0, 0.0, 0.0]);
        assert_eq!(color(2.5), [0.0, 0.0, 1.0]);
        assert_eq!(color(4.5), [1.0, 0.0, 0.0], "wraps");
        assert_eq!(chase_step(&pads, 10, 2.5), Some(1));
        assert!(groups_at(&pads, 5, 0.0).is_none(), "empty pad");
    }

    #[test]
    fn latch_flash_release_and_the_layer_order() {
        let mut flash = look(1, [1.0; 3]);
        flash.action = PadAction::Flash;
        let pads = vec![look(0, [1.0, 0.0, 0.0]), flash];
        let mut s = PadState::default();
        assert!(!s.press(&pads, 7, true), "empty pad does nothing");
        assert!(s.press(&pads, 0, true));
        s.press(&pads, 0, false);
        assert_eq!(s.latched, Some(0), "latch stays after release of the key");
        assert_eq!(layer(false, false, &s, &pads), Layer::Latched(0));
        s.press(&pads, 1, true);
        assert_eq!(layer(false, false, &s, &pads), Layer::FlashPad(1));
        assert_eq!(layer(false, true, &s, &pads), Layer::Momentary);
        assert_eq!(layer(true, true, &s, &pads), Layer::Blackout);
        s.press(&pads, 1, false);
        assert_eq!(s.top(), Some(0));
        s.hold = true;
        s.press(&pads, 0, true);
        assert_eq!(s.latched, None, "a second press unlatches");
        assert_eq!(layer(false, false, &s, &pads), Layer::Hold);
        s.press(&pads, 0, true);
        s.release();
        assert_eq!(layer(false, false, &s, &pads), Layer::Base);
    }

    #[test]
    fn table_and_palette() {
        let mut t = AutoTable::default();
        assert_eq!(
            t.pick(Some(PhraseKind::Chorus)),
            AutoPick::Program(Program::Sweep)
        );
        t.set(PhraseSlot::Drop, AutoPick::Pad(3));
        assert_eq!(t.pick(Some(PhraseKind::Chorus)), AutoPick::Pad(3));
        t.forget_pad(3);
        assert_eq!(t.drop, AutoPick::Program(Program::Sweep));
        let a = AutoSettings {
            palette: Palette::Cool,
            ..Default::default()
        };
        assert_eq!(a.color(None, 0.0), Some(Palette::Cool.colors()[0]));
        assert_eq!(a.color(None, 33.0), Some(Palette::Cool.colors()[1]));
        assert_eq!(AutoSettings::default().color(None, 0.0), None);
        let empty = AutoSettings {
            palette: Palette::Custom,
            ..Default::default()
        };
        assert_eq!(
            empty.color(None, 0.0),
            None,
            "an empty custom palette follows the track"
        );
    }
}
