//! Hardware status of mapped controls (roadmap [52]) and the self-test
//! before a gig.
//!
//! Every mapping entry has one of three states:
//!
//! * **verified**: someone confirmed it on a real unit (`confirmed = true`);
//! * **partial**: the message comes from a public source (another project's
//!   working mapping), but nobody has tried it with OpenDJDecks on a unit;
//! * **untested**: neither (`verify = true`).
//!
//! The self-test lists every control of a mapping, ticks a control when its
//! MIDI message arrives, and builds the messages that light every mapped LED.

use crate::mapping::{Mapping, OutputMode};
use crate::midi::{MidiAddr, MidiMessage};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HwStatus {
    Verified,
    Partial,
    Untested,
}

impl HwStatus {
    pub fn of(verify: bool, confirmed: bool) -> Self {
        if confirmed {
            HwStatus::Verified
        } else if verify {
            HwStatus::Untested
        } else {
            HwStatus::Partial
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            HwStatus::Verified => "verified",
            HwStatus::Partial => "partial",
            HwStatus::Untested => "untested",
        }
    }

    /// Several entries of one control: verified only if all are, untested
    /// only if all are.
    fn combine(self, other: HwStatus) -> HwStatus {
        if self == other { self } else { HwStatus::Partial }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    /// Something you press, turn or move.
    Input,
    /// Something the controller shows (LED, meter, colour).
    Light,
}

/// One control of a mapping with its hardware status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlStatus {
    pub kind: ControlKind,
    pub name: String,
    pub status: HwStatus,
}

/// "Play (deck 2)" → "Play": the public list names a control once for all
/// decks.
fn without_deck(name: &str) -> &str {
    match name.rfind(" (deck ") {
        Some(i) if name.ends_with(')') => &name[..i],
        _ => name,
    }
}

/// Every mapped control of `m`, inputs first, in file order. Entries with
/// the same name (one per deck, for example) are one control.
pub fn controls(m: &Mapping) -> Vec<ControlStatus> {
    let mut out: Vec<ControlStatus> = Vec::new();
    let mut at: HashMap<(bool, String), usize> = HashMap::new();
    let entries = m
        .inputs
        .iter()
        .map(|b| {
            let name = without_deck(&b.name).to_string();
            (ControlKind::Input, name, HwStatus::of(b.verify, b.confirmed))
        })
        .chain(m.outputs.iter().map(|b| {
            (ControlKind::Light, light_name(&b.name), HwStatus::of(b.verify, b.confirmed))
        }));
    for (kind, name, status) in entries {
        let key = (kind == ControlKind::Input, name.clone());
        match at.get(&key) {
            Some(&i) => out[i].status = out[i].status.combine(status),
            None => {
                at.insert(key, out.len());
                out.push(ControlStatus { kind, name, status });
            }
        }
    }
    out
}

/// "deck.play" → "deck play".
fn light_name(target: &str) -> String {
    target.replace(['.', '_'], " ")
}

/// Counts per status: (verified, partial, untested).
pub fn counts(controls: &[ControlStatus]) -> (usize, usize, usize) {
    controls.iter().fold((0, 0, 0), |(v, p, u), c| match c.status {
        HwStatus::Verified => (v + 1, p, u),
        HwStatus::Partial => (v, p + 1, u),
        HwStatus::Untested => (v, p, u + 1),
    })
}

/// One line of the self-test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestControl {
    pub name: String,
    pub status: HwStatus,
    pub seen: bool,
}

/// The self-test of one mapping: which inputs have answered so far.
#[derive(Clone, Debug)]
pub struct SelfTest {
    mapping: Arc<Mapping>,
    pub controls: Vec<TestControl>,
    /// MIDI address → controls it belongs to.
    index: HashMap<MidiAddr, Vec<usize>>,
}

impl SelfTest {
    pub fn new(mapping: Arc<Mapping>) -> Self {
        let mut controls: Vec<TestControl> = Vec::new();
        let mut by_name: HashMap<String, usize> = HashMap::new();
        let mut index: HashMap<MidiAddr, Vec<usize>> = HashMap::new();
        for b in &mapping.inputs {
            let status = HwStatus::of(b.verify, b.confirmed);
            let i = *by_name.entry(b.name.clone()).or_insert_with(|| {
                controls.push(TestControl {
                    name: b.name.clone(),
                    status,
                    seen: false,
                });
                controls.len() - 1
            });
            controls[i].status = controls[i].status.combine(status);
            for addr in std::iter::once(b.addr).chain(b.lsb) {
                let v = index.entry(addr).or_default();
                if !v.contains(&i) {
                    v.push(i);
                }
            }
        }
        Self {
            mapping,
            controls,
            index,
        }
    }

    pub fn mapping(&self) -> &Arc<Mapping> {
        &self.mapping
    }

    /// Feeds one incoming message; true when a control was ticked for the
    /// first time. Releases (note off, value 0) count too: they come from
    /// the control.
    pub fn feed(&mut self, bytes: &[u8]) -> bool {
        let Some((addr, _)) = MidiMessage::parse(bytes).as_ref().and_then(MidiMessage::addr_value)
        else {
            return false;
        };
        let mut changed = false;
        if let Some(ids) = self.index.get(&addr) {
            for &i in ids {
                if !self.controls[i].seen {
                    self.controls[i].seen = true;
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn seen(&self) -> usize {
        self.controls.iter().filter(|c| c.seen).count()
    }

    pub fn reset(&mut self) {
        self.controls.iter_mut().for_each(|c| c.seen = false);
    }

    /// Number of distinct lights (LEDs, meters) the mapping drives.
    pub fn lights(&self) -> usize {
        led_messages(&self.mapping, true).len()
    }
}

/// Messages that switch every mapped light fully on (`on`) or off. Each
/// address once.
pub fn led_messages(m: &Mapping, on: bool) -> Vec<Vec<u8>> {
    let mut seen = std::collections::HashSet::new();
    m.outputs
        .iter()
        .filter(|b| seen.insert(b.addr))
        .map(|b| {
            let value = match (on, b.mode) {
                (false, _) => b.off,
                (true, OutputMode::Level) => b.max,
                (true, _) => b.on,
            };
            b.addr.message(value).to_vec()
        })
        .collect()
}

/// The generated hardware status section of the controllers guide, in
/// English (`de = false`) or German.
pub fn status_markdown(mappings: &[Arc<Mapping>], de: bool) -> String {
    let t = |en: &'static str, de_s: &'static str| if de { de_s } else { en };
    let word = |s: HwStatus| match s {
        HwStatus::Verified => t("verified", "bestätigt"),
        HwStatus::Partial => t("partial", "teilweise"),
        HwStatus::Untested => t("untested", "ungetestet"),
    };
    let mut s = String::new();
    s.push_str(t(
        "| Model | Controls | Verified | Partial | Untested |\n|---|---|---|---|---|\n",
        "| Modell | Bedienelemente | Bestätigt | Teilweise | Ungetestet |\n|---|---|---|---|---|\n",
    ));
    for m in mappings {
        let c = controls(m);
        let (v, p, u) = counts(&c);
        s.push_str(&format!(
            "| {} | {} | {v} | {p} | {u} |\n",
            m.file.device.name,
            c.len()
        ));
    }
    for m in mappings {
        s.push_str(&format!("\n### {}\n\n", m.file.device.name));
        s.push_str(t(
            "| Control | Kind | Status |\n|---|---|---|\n",
            "| Bedienelement | Art | Status |\n|---|---|---|\n",
        ));
        for c in controls(m) {
            let kind = match c.kind {
                ControlKind::Input => t("input", "Eingabe"),
                ControlKind::Light => t("light", "Anzeige"),
            };
            s.push_str(&format!(
                "| {} | {kind} | {} |\n",
                c.name.replace('|', "\\|"),
                word(c.status)
            ));
        }
    }
    s
}

/// The status of a whole model: verified only when every control of it is
/// confirmed on a unit, "partly confirmed" when some are, else unconfirmed.
pub fn model_status(controls: &[ControlStatus], de: bool) -> &'static str {
    let (v, _, _) = counts(controls);
    let t = |en: &'static str, de_s: &'static str| if de { de_s } else { en };
    if !controls.is_empty() && v == controls.len() {
        t("verified", "bestätigt")
    } else if v > 0 {
        t("partly confirmed", "teilweise bestätigt")
    } else {
        t("not confirmed", "nicht bestätigt")
    }
}

/// The published controller list (roadmap [13]): one row per bundled
/// mapping with how it is recognised and its status, then one row per
/// controller class the guided setup covers. Generated like the status
/// list, between the `controller-list` markers of the controllers guide.
pub fn controller_list_markdown(
    mappings: &[Arc<Mapping>],
    templates: &[crate::guided::Template],
    de: bool,
) -> String {
    let t = |en: &'static str, de_s: &'static str| if de { de_s } else { en };
    let mut s = String::new();
    s.push_str(t(
        "| Controller | Decks | Mapping | Recognised by | Status |\n|---|---|---|---|---|\n",
        "| Controller | Decks | Mapping | Erkannt an | Status |\n|---|---|---|---|---|\n",
    ));
    for m in mappings {
        let d = &m.file.device;
        let c = controls(m);
        let (v, p, u) = counts(&c);
        let names: Vec<String> = d
            .matching
            .port_names
            .iter()
            .map(|n| format!("`{n}`"))
            .collect();
        let status = if de {
            format!(
                "{}: {v} bestätigt, {p} teilweise, {u} ungetestet",
                model_status(&c, true)
            )
        } else {
            format!(
                "{}: {v} verified, {p} partial, {u} untested",
                model_status(&c, false)
            )
        };
        s.push_str(&format!(
            "| {} {} | {} | {} | {} {} | {status} |\n",
            t("compatible with", "kompatibel mit"),
            d.name,
            d.decks,
            t("built in", "eingebaut"),
            t("MIDI port name", "MIDI-Portname"),
            names.join(", "),
        ));
    }
    for tpl in templates {
        let class = match tpl.id.as_str() {
            "two-deck-entry" => t("Any two-deck entry-level controller", "Jeder Einsteiger-Controller mit zwei Decks"),
            "two-deck-battle" => t("Any two-deck battle-style controller", "Jeder Battle-Controller mit zwei Decks"),
            "four-deck" => t("Any four-deck controller", "Jeder Controller mit vier Decks"),
            _ => t("Other controller", "Anderer Controller"),
        };
        s.push_str(&format!(
            "| {class} | {} | {} `{}` ({} {}) | {} | {} |\n",
            tpl.decks,
            t("guided setup, template", "geführte Einrichtung, Vorlage"),
            tpl.id,
            tpl.steps.len(),
            t("controls", "Bedienelemente"),
            t("the port of the unit you set up", "der Port des eingerichteten Geräts"),
            t(
                "not confirmed: your own capture, nobody else's unit",
                "nicht bestätigt: deine eigene Aufnahme, kein anderes Gerät"
            ),
        ));
    }
    s
}

/// Markers of the generated controller list.
pub const LIST_START: &str = "<!-- controller-list:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->";
pub const LIST_END: &str = "<!-- controller-list:end -->";

/// `doc` with the generated controller list replaced, or `None` when its
/// markers are missing.
pub fn replace_list(doc: &str, section: &str) -> Option<String> {
    let a = doc.find(LIST_START)? + LIST_START.len();
    let b = doc[a..].find(LIST_END)? + a;
    Some(format!("{}\n\n{}\n{}", &doc[..a], section.trim_end(), &doc[b..]))
}

/// Start and end markers of the generated section.
pub const STATUS_START: &str = "<!-- hardware-status:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->";
pub const STATUS_END: &str = "<!-- hardware-status:end -->";

/// `doc` with the generated section replaced by `section`, or `None` when
/// the markers are missing.
pub fn replace_section(doc: &str, section: &str) -> Option<String> {
    let a = doc.find(STATUS_START)? + STATUS_START.len();
    let b = doc[a..].find(STATUS_END)? + a;
    Some(format!("{}\n\n{}\n{}", &doc[..a], section.trim_end(), &doc[b..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r#"
schema = 1
[device]
id = "test"
name = "Test 2"
vendor = "Test"
model = "Test 2"
decks = 2
[device.match]
port_names = ["TEST2"]
[channels]
deck = [0, 1]
[[input]]
name = "Play"
note = 0x0B
channel = "deck"
action = "deck.play_pause"
[[input]]
name = "Cue"
note = 0x0C
channel = "deck"
action = "deck.cue"
verify = true
[[input]]
name = "Crossfader"
cc = 0x1F
channel = 6
action = "crossfader"
confirmed = true
[[output]]
target = "deck.play"
note = 0x0B
channel = "deck"
[[output]]
target = "deck.cue"
note = 0x0C
channel = "deck"
verify = true
"#;

    fn mapping() -> Arc<Mapping> {
        Arc::new(Mapping::from_toml_str(MAP).unwrap())
    }

    #[test]
    fn the_controller_list_never_claims_more_than_the_mappings() {
        let m = mapping();
        let tpl = crate::guided::Template::all();
        let en = controller_list_markdown(&[m.clone()], &tpl, false);
        // One confirmed entry makes the model "partly confirmed", never
        // "verified"; the classes are never confirmed.
        assert!(en.contains("| compatible with Test 2 | 2 | built in | MIDI port name `TEST2` | partly confirmed: 1 verified"), "{en}");
        assert!(en.contains("Any two-deck battle-style controller"));
        assert_eq!(en.matches("not confirmed").count(), 3, "{en}");
        assert!(!en.contains("| verified"), "{en}");
        let de = controller_list_markdown(&[m], &tpl, true);
        assert!(de.contains("nicht bestätigt"));
        let doc = format!("a\n{LIST_START}\nold\n{LIST_END}\nb");
        let new = replace_list(&doc, "new").unwrap();
        assert!(new.contains("new") && !new.contains("old") && new.ends_with("b"));
    }

    #[test]
    fn status_follows_the_markers() {
        let c = controls(&mapping());
        let find = |n: &str| c.iter().find(|c| c.name == n).unwrap().status;
        assert_eq!(find("Play"), HwStatus::Partial);
        assert_eq!(find("Cue"), HwStatus::Untested);
        assert_eq!(find("Crossfader"), HwStatus::Verified);
        assert_eq!(find("deck cue"), HwStatus::Untested);
        assert_eq!(counts(&c), (1, 2, 2));
        assert_eq!(HwStatus::Partial.combine(HwStatus::Untested), HwStatus::Partial);
    }

    #[test]
    fn the_self_test_ticks_controls_as_their_messages_arrive() {
        let mut t = SelfTest::new(mapping());
        assert_eq!(t.controls.len(), 5);
        assert_eq!(t.seen(), 0);
        assert!(t.feed(&[0x90, 0x0B, 0x7F]), "play on deck 1");
        assert!(!t.feed(&[0x80, 0x0B, 0x00]), "the release of the same control");
        assert!(!t.feed(&[0x95, 0x40, 0x7F]), "an unmapped message");
        assert!(t.feed(&[0xB6, 0x1F, 0x40]));
        assert_eq!(t.seen(), 2);
        let seen: Vec<&str> = t
            .controls
            .iter()
            .filter(|c| c.seen)
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(seen, ["Play (deck 1)", "Crossfader"]);
        t.reset();
        assert_eq!(t.seen(), 0);
    }

    #[test]
    fn led_messages_light_every_mapped_led_once() {
        let m = mapping();
        let on = led_messages(&m, true);
        assert_eq!(on.len(), 4, "two LEDs on two decks");
        assert!(on.contains(&vec![0x90, 0x0B, 0x7F]));
        assert!(on.contains(&vec![0x91, 0x0C, 0x7F]));
        let off = led_messages(&m, false);
        assert!(off.iter().all(|msg| msg[2] == 0));
        assert_eq!(SelfTest::new(m).lights(), 4);
    }

    #[test]
    fn the_generated_section_replaces_only_between_the_markers() {
        let doc = format!("intro\n{STATUS_START}\nold\n{STATUS_END}\noutro\n");
        let md = status_markdown(&[mapping()], false);
        assert!(md.contains("| Test 2 | 5 | 1 | 2 | 2 |"), "{md}");
        assert!(md.contains("| Cue | input | untested |"), "{md}");
        let new = replace_section(&doc, &md).unwrap();
        assert!(new.starts_with("intro\n") && new.ends_with(&format!("{STATUS_END}\noutro\n")));
        assert!(!new.contains("old"));
        assert!(status_markdown(&[mapping()], true).contains("| Cue | Eingabe | ungetestet |"));
        assert!(replace_section("no markers", &md).is_none());
    }
}
