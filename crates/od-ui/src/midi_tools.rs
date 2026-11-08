//! Preferences ▸ Controller tools: the MIDI monitor (raw messages with what
//! the mapping made of them) and MIDI learn (move a control, get a mapping
//! entry, save it to your own mapping file).

use crossbeam_channel::Receiver;
use od_controller::learn::{LearnSession, LearnedInput};
use od_controller::monitor::{Direction, MidiMonitor, MonitorEvent};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Lines the monitor keeps on screen.
pub(crate) const MONITOR_LINES: usize = 200;

/// Runtime state of the tools (part of the Preferences state).
#[derive(Default)]
pub(crate) struct MidiTools {
    /// The controller layer's monitor (tests put their own here).
    pub monitor: Option<MidiMonitor>,
    rx: Option<Receiver<MonitorEvent>>,
    pub monitoring: bool,
    pub lines: VecDeque<String>,
    pub learning: bool,
    learn: LearnSession,
    /// Port the learned control came from.
    pub learn_port: String,
    /// Index into `od_controller::templates::ACTION_NAMES`.
    pub action: usize,
    /// Deck / channel of the learned entry (1-based; 0 = none).
    pub deck: u8,
    pub suggestion: Option<LearnedInput>,
}

impl MidiTools {
    /// Starts or stops the monitor / learn stream.
    fn ensure_stream(&mut self) {
        let want = self.monitoring || self.learning;
        match (want, &self.rx, &self.monitor) {
            (true, None, Some(m)) => {
                m.set_enabled(true);
                self.rx = Some(m.subscribe());
            }
            (false, Some(_), _) => self.rx = None,
            _ => {}
        }
    }

    pub fn set_monitoring(&mut self, on: bool) {
        self.monitoring = on;
        self.ensure_stream();
    }

    pub fn start_learn(&mut self) {
        self.learn.reset();
        self.suggestion = None;
        self.learn_port.clear();
        self.learning = true;
        self.ensure_stream();
    }

    pub fn stop_learn(&mut self) {
        self.learning = false;
        self.ensure_stream();
    }

    /// Takes the new messages: monitor lines and learn input. Returns true
    /// when something changed.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else { return false };
        let events: Vec<MonitorEvent> = rx.try_iter().collect();
        if events.is_empty() {
            return false;
        }
        for e in events {
            if self.monitoring {
                for l in e.to_string().lines() {
                    if self.lines.len() == MONITOR_LINES {
                        self.lines.pop_front();
                    }
                    self.lines.push_back(l.to_string());
                }
            }
            if self.learning && e.direction == Direction::In {
                self.learn.feed(&e.bytes);
                if self.learn_port.is_empty() {
                    self.learn_port = e.port.clone();
                }
                self.suggestion = self.learn.suggest();
            }
        }
        true
    }

    pub fn action_name(&self) -> &'static str {
        od_controller::templates::ACTION_NAMES
            .get(self.action)
            .copied()
            .unwrap_or("ignore")
    }

    /// The `[[input]]` entry for the learned control, if any.
    pub fn entry(&self) -> Option<String> {
        self.suggestion
            .as_ref()
            .map(|l| input_block(l, self.action_name(), (self.deck > 0).then_some(self.deck)))
    }
}

/// A short description of a learned control: "CC 0x13 / 0x33 · channel 1 ·
/// 14-bit fader".
pub(crate) fn describe(l: &LearnedInput) -> String {
    use od_controller::mapping::InputMode;
    use od_controller::midi::MsgKind;
    let kind = match l.addr.kind {
        MsgKind::Note => format!("NOTE 0x{:02X}", l.addr.number),
        MsgKind::Cc => match l.lsb {
            Some(lsb) => format!("CC 0x{:02X} / 0x{lsb:02X}", l.addr.number),
            None => format!("CC 0x{:02X}", l.addr.number),
        },
    };
    let mode = match l.mode {
        InputMode::Button => od_i18n::tr("button"),
        InputMode::Absolute => od_i18n::tr("knob / fader"),
        InputMode::Absolute14 => od_i18n::tr("14-bit fader"),
        InputMode::Relative => od_i18n::tr("encoder / jog"),
    };
    od_i18n::trf(
        "{} · channel {} · {}",
        &[&kind, &(l.addr.channel + 1), &mode],
    )
}

/// The mapping entry of a learned control, with its deck when given.
pub(crate) fn input_block(l: &LearnedInput, action: &str, deck: Option<u8>) -> String {
    let name = action.replace(['.', '_'], " ");
    let mut s = l.to_toml(action, &name);
    if let Some(d) = deck {
        s = s.replace(
            &format!("action = {action:?}"),
            &format!("deck = {d}\naction = {action:?}"),
        );
    }
    s
}

/// A new mapping file for learned controls of the device on `port`.
pub(crate) fn mapping_header(port: &str) -> String {
    let port = if port.is_empty() { "MIDI" } else { port };
    format!(
        "# Controls learned in OpenDJDecks (Preferences > Controller > MIDI learn).\n\
         schema = 1\n\n\
         [device]\n\
         id = \"user-learned\"\n\
         name = \"My controller\"\n\
         vendor = \"\"\n\
         model = {port:?}\n\
         decks = 4\n\
         sources = [\"MIDI learn\"]\n\n\
         [device.match]\n\
         port_names = [{port:?}]\n\n"
    )
}

/// Adds `entry` to the mapping file at `path` (created with a header for
/// `port` when missing) and checks that the result still loads; a file
/// that would not load is left as it was.
pub(crate) fn save_entry(path: &Path, port: &str, entry: &str) -> Result<(), String> {
    let before = std::fs::read_to_string(path).ok();
    let mut text = before.clone().unwrap_or_else(|| mapping_header(port));
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
    text.push_str(entry);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, &text).map_err(|e| e.to_string())?;
    if let Err(e) = od_controller::Mapping::load(path) {
        match before {
            Some(b) => {
                let _ = std::fs::write(path, b);
            }
            None => {
                let _ = std::fs::remove_file(path);
            }
        }
        return Err(e.to_string());
    }
    Ok(())
}

/// Where learned controls go when no own mapping file is set.
pub(crate) fn default_learn_file(library: &Path) -> PathBuf {
    library
        .parent()
        .unwrap_or(Path::new("."))
        .join("mappings/learned.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_controller::monitor::Direction;

    #[test]
    fn monitor_lines_and_learning_from_the_stream() {
        let m = MidiMonitor::new(64);
        let mut t = MidiTools {
            monitor: Some(m.clone()),
            ..Default::default()
        };
        t.set_monitoring(true);
        m.record("CTRL-TEST", Direction::In, &[0x90, 0x0B, 0x7F], Vec::new());
        assert!(t.poll());
        assert_eq!(t.lines.len(), 1);
        assert!(
            t.lines[0].contains("90 0B 7F") || t.lines[0].to_uppercase().contains("90 0B 7F"),
            "{}",
            t.lines[0]
        );
        // Learn a 14-bit fader: CC 0x13 with its LSB 0x33.
        t.start_learn();
        for v in [10u8, 20, 30] {
            m.record("CTRL-TEST", Direction::In, &[0xB0, 0x13, v], Vec::new());
            m.record("CTRL-TEST", Direction::In, &[0xB0, 0x33, 0], Vec::new());
        }
        t.poll();
        let s = t.suggestion.clone().expect("a suggestion");
        assert_eq!((s.addr.number, s.lsb), (0x13, Some(0x33)));
        assert_eq!(t.learn_port, "CTRL-TEST");
        assert!(describe(&s).contains("CC 0x13 / 0x33"));
        t.action = od_controller::templates::ACTION_NAMES
            .iter()
            .position(|a| *a == "mixer.fader")
            .unwrap();
        t.deck = 2;
        let e = t.entry().unwrap();
        assert!(
            e.contains("cc = 0x13")
                && e.contains("lsb = 0x33")
                && e.contains("deck = 2")
                && e.contains("action = \"mixer.fader\""),
            "{e}"
        );
        t.stop_learn();
        t.set_monitoring(false);
        m.record("CTRL-TEST", Direction::In, &[0x90, 0x0C, 0x7F], Vec::new());
        assert!(!t.poll(), "nothing is read while both are off");
    }

    #[test]
    fn learned_entries_make_a_loadable_mapping_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mappings/learned.toml");
        let mut s = LearnSession::new();
        s.feed(&[0x91, 0x0B, 0x7F]);
        s.feed(&[0x91, 0x0B, 0x00]);
        let l = s.suggest().unwrap();
        save_entry(
            &path,
            "CTRL-TEST MIDI 1",
            &input_block(&l, "deck.play_pause", Some(2)),
        )
        .unwrap();
        let m = od_controller::Mapping::load(&path).expect("loads");
        assert!(m.matches_port("CTRL-TEST MIDI 1"));
        let mut s2 = LearnSession::new();
        s2.feed(&[0xB0, 0x40, 0x41]);
        s2.feed(&[0xB0, 0x40, 0x41]);
        s2.feed(&[0xB0, 0x40, 0x3F]);
        save_entry(
            &path,
            "",
            &input_block(&s2.suggest().unwrap(), "browse.rotate", None),
        )
        .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("[[input]]").count(), 2);
        assert!(od_controller::Mapping::load(&path).is_ok());
        // An entry the mapping cannot use is refused and the file kept.
        let bad = input_block(&l, "no.such_action", None);
        assert!(save_entry(&path, "", &bad).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
}
