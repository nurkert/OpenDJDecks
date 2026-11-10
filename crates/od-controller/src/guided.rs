//! Guided setup for controllers without a mapping (roadmap [13]).
//!
//! A [`Template`] for a controller class (two decks entry level, two decks
//! battle style, four decks) lists the core controls in the order a person
//! finds them: play, cue, sync, jog, pitch, load per deck, then the mixer
//! (channel faders, EQ) and the crossfader. [`GuidedSetup`] walks through
//! the list, learns each control from the messages it sends (the same
//! [`LearnSession`] as MIDI learn) and writes a mapping file at the end that
//! can be shared like any other.
//!
//! The templates name no model and hold no MIDI numbers: every number in the
//! file comes from the owner's own unit. Captured entries are marked
//! `verify = true`, because nobody has confirmed them for anyone else yet.

use crate::learn::{LearnSession, LearnedInput};
use crate::mapping::{InputMode, Mapping, MappingError};
use crate::midi::{MidiAddr, MidiMessage, MsgKind};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write;

/// The templates shipped with the app (`mappings/templates/`).
pub const TEMPLATE_FILES: [(&str, &str); 3] = [
    (
        "two-deck-entry",
        include_str!("../../../mappings/templates/two-deck-entry.toml"),
    ),
    (
        "two-deck-battle",
        include_str!("../../../mappings/templates/two-deck-battle.toml"),
    ),
    (
        "four-deck",
        include_str!("../../../mappings/templates/four-deck.toml"),
    ),
];

/// What a step expects the person to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    /// A key or pad (note, or a CC that only sends 0 / 127).
    Button,
    /// A fader or knob (absolute CC, 7 or 14 bit).
    Fader,
    /// A jog wheel or endless encoder (relative CC).
    Wheel,
}

/// One control the setup asks for.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// Mapping action (`deck.play_pause`).
    pub action: String,
    /// Deck or mixer channel (1-based), when the action has one.
    #[serde(default)]
    pub deck: Option<u8>,
    /// Action arguments, written into the mapping as they are.
    #[serde(default)]
    pub args: BTreeMap<String, toml::Value>,
    pub expect: Expect,
    /// Name in the mapping file and the MIDI monitor.
    pub name: String,
    /// Accessible label of the matching on-screen control (shown with the
    /// guide outline while the step is current), if there is one.
    #[serde(default)]
    pub screen: Option<String>,
}

/// A controller class: the steps and the settings a mapping of this class
/// starts from.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub template: u32,
    pub id: String,
    pub name: String,
    pub decks: u8,
    /// Device module for the jog wheels (`jog-controller`), if any.
    #[serde(default)]
    pub module: Option<String>,
    /// Crossfader curve the class is played with ("cut" for battle style);
    /// the app offers it when the mapping is saved. Empty: leave as it is.
    #[serde(default)]
    pub crossfader_curve: String,
    #[serde(rename = "step")]
    pub steps: Vec<Step>,
}

impl Template {
    pub fn parse(s: &str) -> Result<Self, String> {
        let t: Template = toml::from_str(s).map_err(|e| e.to_string())?;
        if t.template != 1 {
            return Err(format!("unknown template version {}", t.template));
        }
        Ok(t)
    }

    /// A shipped template by id.
    pub fn bundled(id: &str) -> Option<Self> {
        TEMPLATE_FILES
            .iter()
            .find(|(i, _)| *i == id)
            .and_then(|(_, s)| Self::parse(s).ok())
    }

    /// Every shipped template, in the order the setup offers them.
    pub fn all() -> Vec<Self> {
        TEMPLATE_FILES
            .iter()
            .filter_map(|(_, s)| Self::parse(s).ok())
            .collect()
    }
}

/// Why a learned control was not taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Nothing usable arrived yet.
    Nothing,
    /// The message is already used by an earlier step (its index).
    Taken(usize),
}

/// The guided setup in progress.
#[derive(Debug)]
pub struct GuidedSetup {
    pub template: Template,
    /// Current step (== steps.len() when all are done).
    pub index: usize,
    /// What each step learned (`None`: skipped or not reached).
    pub learned: Vec<Option<LearnedInput>>,
    learn: LearnSession,
    /// MIDI port the controls came from.
    pub port: String,
}

impl GuidedSetup {
    pub fn new(template: Template) -> Self {
        let n = template.steps.len();
        Self {
            template,
            index: 0,
            learned: vec![None; n],
            learn: LearnSession::new(),
            port: String::new(),
        }
    }

    pub fn step(&self) -> Option<&Step> {
        self.template.steps.get(self.index)
    }

    pub fn done(&self) -> bool {
        self.index >= self.template.steps.len()
    }

    /// Steps with a learned control.
    pub fn learned_count(&self) -> usize {
        self.learned.iter().filter(|l| l.is_some()).count()
    }

    /// Feeds one incoming message (from `port`). Returns true when it
    /// counts for the current step.
    pub fn feed(&mut self, port: &str, bytes: &[u8]) -> bool {
        let Some(step) = self.step() else {
            return false;
        };
        let Some((addr, _)) = MidiMessage::parse(bytes).and_then(|m| m.addr_value()) else {
            return false;
        };
        // A fader or wheel step ignores notes (touch-sensitive jogs and
        // knobs send a touch note beside their CC).
        if step.expect != Expect::Button && addr.kind == MsgKind::Note {
            return false;
        }
        if self.port.is_empty() {
            self.port = port.to_string();
        }
        self.learn.feed(bytes);
        true
    }

    /// The best guess for the current step so far.
    pub fn pending(&self) -> Option<LearnedInput> {
        let step = self.step()?;
        let mut s = self.learn.suggest()?;
        match step.expect {
            Expect::Button => {
                if s.mode != InputMode::Button {
                    // A CC key that sends 0 / 127, read as a knob because it
                    // has only been pressed: it is a button here.
                    if s.addr.kind == MsgKind::Cc {
                        s.mode = InputMode::Button;
                        s.encoding = None;
                        s.lsb = None;
                    }
                }
            }
            Expect::Fader => {
                if s.mode == InputMode::Relative || s.mode == InputMode::Button {
                    s.mode = InputMode::Absolute;
                    s.encoding = None;
                }
            }
            Expect::Wheel => {
                if s.mode != InputMode::Relative {
                    s.mode = InputMode::Relative;
                    s.lsb = None;
                    s.encoding = Some(crate::mapping::RelEncoding::Offset64);
                }
            }
        }
        Some(s)
    }

    /// Which earlier step uses `addr` already.
    fn taken_by(&self, addr: MidiAddr, lsb: Option<u8>) -> Option<usize> {
        self.learned.iter().enumerate().find_map(|(i, l)| {
            let l = l.as_ref()?;
            let same = l.addr == addr
                || lsb.is_some_and(|b| l.addr == MidiAddr::cc(addr.channel, b))
                || l.lsb
                    .is_some_and(|b| addr == MidiAddr::cc(l.addr.channel, b));
            (same && i != self.index).then_some(i)
        })
    }

    /// Takes the learned control for the current step and moves on.
    pub fn accept(&mut self) -> Result<(), Refusal> {
        let l = self.pending().ok_or(Refusal::Nothing)?;
        if let Some(i) = self.taken_by(l.addr, l.lsb) {
            self.learn.reset();
            return Err(Refusal::Taken(i));
        }
        if let Some(slot) = self.learned.get_mut(self.index) {
            *slot = Some(l);
        }
        self.advance(1);
        Ok(())
    }

    /// Leaves the current step without a control.
    pub fn skip(&mut self) {
        if let Some(slot) = self.learned.get_mut(self.index) {
            *slot = None;
        }
        self.advance(1);
    }

    /// Back one step, to learn it again.
    pub fn back(&mut self) {
        self.advance(-1);
    }

    /// Forgets what arrived for the current step.
    pub fn retry(&mut self) {
        self.learn.reset();
    }

    fn advance(&mut self, by: isize) {
        let n = self.template.steps.len() as isize;
        self.index = (self.index as isize + by).clamp(0, n) as usize;
        self.learn.reset();
    }

    /// The mapping file: a header that says where it came from, the
    /// template's device settings and one `[[input]]` per learned step.
    pub fn to_toml(&self, name: &str) -> String {
        let port = if self.port.is_empty() {
            "MIDI"
        } else {
            self.port.as_str()
        };
        let name = if name.trim().is_empty() {
            port
        } else {
            name.trim()
        };
        let id = slug(name);
        let mut s = String::new();
        let _ = writeln!(s, "# OpenDJDecks controller mapping: {name}");
        let _ = writeln!(s, "#");
        let _ = writeln!(
            s,
            "# Made with the guided setup (Preferences > Controller) from the template"
        );
        let _ = writeln!(
            s,
            "# \"{}\". Every message below was captured on the owner's own unit;",
            self.template.name
        );
        let _ = writeln!(
            s,
            "# none of it is confirmed for other units yet, so every entry has"
        );
        let _ = writeln!(s, "# `verify = true`. Share it as it is.\n");
        let _ = writeln!(s, "schema = 1\n");
        let _ = writeln!(s, "[device]");
        let _ = writeln!(s, "id = {:?}", format!("user-{id}"));
        let _ = writeln!(s, "name = {name:?}");
        let _ = writeln!(s, "vendor = \"\"");
        let _ = writeln!(s, "model = {port:?}");
        let _ = writeln!(s, "decks = {}", self.template.decks);
        if let Some(m) = &self.template.module {
            let _ = writeln!(s, "module = {m:?}");
        }
        let _ = writeln!(
            s,
            "sources = [\"guided setup, template {}\"]\n",
            self.template.id
        );
        let _ = writeln!(s, "[device.match]");
        let _ = writeln!(s, "port_names = [{port:?}]\n");
        for (step, l) in self.template.steps.iter().zip(&self.learned) {
            let Some(l) = l else { continue };
            s.push_str(&input_entry(step, l));
            s.push('\n');
        }
        s
    }

    /// The mapping as it would load (checks the file before it is written).
    pub fn mapping(&self, name: &str) -> Result<Mapping, MappingError> {
        Mapping::from_toml_str(&self.to_toml(name))
    }
}

/// One `[[input]]` table for a learned step.
fn input_entry(step: &Step, l: &LearnedInput) -> String {
    let mut s = l.to_toml(&step.action, &step.name);
    let mut extra = String::new();
    if let Some(d) = step.deck {
        let _ = writeln!(extra, "deck = {d}");
    }
    if !step.args.is_empty() {
        let parts: Vec<String> = step
            .args
            .iter()
            .map(|(k, v)| format!("{k} = {v}"))
            .collect();
        let _ = writeln!(extra, "args = {{ {} }}", parts.join(", "));
    }
    extra.push_str("verify = true\n");
    let action_line = format!("action = {:?}\n", step.action);
    s = s.replace(&action_line, &format!("{extra}{action_line}"));
    s
}

/// "My Controller 2" → "my-controller-2".
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "controller".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ControlAction, DeckAction};

    fn press(g: &mut GuidedSetup, ch: u8, note: u8) {
        g.feed("TEST PORT", &[0x90 | ch, note, 0x7F]);
        g.feed("TEST PORT", &[0x90 | ch, note, 0x00]);
    }

    #[test]
    fn every_template_parses_and_covers_the_core_controls() {
        let all = Template::all();
        assert_eq!(all.len(), 3);
        for t in &all {
            let actions: Vec<&str> = t.steps.iter().map(|s| s.action.as_str()).collect();
            for core in [
                "deck.play_pause",
                "deck.cue",
                "deck.sync",
                "deck.jog_turn",
                "deck.tempo",
                "deck.load",
                "mixer.fader",
                "mixer.eq_low",
                "crossfader",
            ] {
                assert!(actions.contains(&core), "{} lacks {core}", t.id);
            }
            for d in 1..=t.decks {
                assert!(
                    t.steps
                        .iter()
                        .any(|s| s.action == "deck.play_pause" && s.deck == Some(d)),
                    "{}: deck {d}",
                    t.id
                );
            }
            // Every action exists for the mapping loader.
            for s in &t.steps {
                assert!(
                    crate::templates::ACTION_NAMES.contains(&s.action.as_str()),
                    "{}: {}",
                    t.id,
                    s.action
                );
            }
        }
        let battle = Template::bundled("two-deck-battle").unwrap();
        assert_eq!(battle.crossfader_curve, "cut");
        assert_eq!(Template::bundled("four-deck").unwrap().decks, 4);
    }

    #[test]
    fn learns_steps_in_order_and_writes_a_loadable_mapping() {
        let mut g = GuidedSetup::new(Template::bundled("two-deck-entry").unwrap());
        assert_eq!(g.step().unwrap().action, "deck.play_pause");
        assert_eq!(g.accept(), Err(Refusal::Nothing));
        press(&mut g, 0, 0x0B);
        g.accept().unwrap();
        assert_eq!(g.index, 1);
        // Cue: the same key again is refused, another one is taken.
        press(&mut g, 0, 0x0B);
        assert_eq!(g.accept(), Err(Refusal::Taken(0)));
        assert_eq!(g.index, 1);
        press(&mut g, 0, 0x0C);
        g.accept().unwrap();
        // Skip the rest up to the tempo fader, learn it as a 14-bit fader.
        while g.step().unwrap().action != "deck.tempo" {
            g.skip();
        }
        for v in [10u8, 20, 30] {
            g.feed("TEST PORT", &[0xB0, 0x00, v]);
            g.feed("TEST PORT", &[0xB0, 0x20, 0x10]);
        }
        let p = g.pending().unwrap();
        assert_eq!((p.mode, p.lsb), (InputMode::Absolute14, Some(0x20)));
        g.accept().unwrap();
        // Jog: a touch note is ignored, the turning CC is relative.
        while g.step().unwrap().action != "deck.jog_turn" {
            g.skip();
        }
        g.feed("TEST PORT", &[0x90, 0x36, 0x7F]);
        for v in [65u8, 66, 65, 63] {
            g.feed("TEST PORT", &[0xB0, 0x22, v]);
        }
        let p = g.pending().unwrap();
        assert_eq!(p.addr, MidiAddr::cc(0, 0x22));
        assert_eq!(p.mode, InputMode::Relative);
        g.accept().unwrap();
        while !g.done() {
            g.skip();
        }
        assert_eq!(g.learned_count(), 4);
        let text = g.to_toml("My Controller");
        assert!(text.contains("id = \"user-my-controller\""), "{text}");
        assert!(text.contains("port_names = [\"TEST PORT\"]"));
        assert_eq!(text.matches("\nverify = true").count(), 4, "{text}");
        assert!(text.contains("args = { mode = \"scratch\" }"), "{text}");
        let m = g.mapping("My Controller").expect("loads");
        assert!(m.matches_port("test port 1"));
        // The learned PLAY plays deck 1.
        let hits = m.inputs_at(MidiAddr::note(0, 0x0B));
        assert_eq!(hits.len(), 1);
        let b = &m.inputs[hits[0]];
        let a = b.template.produce(
            b.slot,
            crate::templates::InputValue::Button {
                pressed: true,
                velocity: 127,
            },
        );
        assert_eq!(
            a,
            Some(ControlAction::Deck {
                deck: 1,
                action: DeckAction::PlayPause
            })
        );
    }

    #[test]
    fn a_cc_key_counts_as_a_button_and_a_knob_as_a_fader() {
        let mut g = GuidedSetup::new(Template::bundled("two-deck-battle").unwrap());
        g.feed("P", &[0xB0, 0x40, 0x7F]);
        assert_eq!(g.pending().unwrap().mode, InputMode::Button);
        g.accept().unwrap();
        while g.step().unwrap().expect != Expect::Fader {
            g.skip();
        }
        g.feed("P", &[0x90, 0x10, 0x7F]);
        assert!(g.pending().is_none(), "notes do not count for a fader");
        for v in [0u8, 127, 0] {
            g.feed("P", &[0xB2, 0x07, v]);
        }
        assert_eq!(g.pending().unwrap().mode, InputMode::Absolute);
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("My Controller 2"), "my-controller-2");
        assert_eq!(slug("  ** "), "controller");
        assert_eq!(slug("ÄBC—x"), "bc-x");
    }
}
