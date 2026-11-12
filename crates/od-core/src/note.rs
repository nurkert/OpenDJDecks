//! A single note's pitch: its name with the octave and an offset in cents
//! ("A3 +12 ct"), as CREATE shows and corrects a picked note (roadmap [71]).
//!
//! Notes count as MIDI numbers (60 = C4, 69 = A4 = 440 Hz). The offset is
//! kept in whole cents from −50 to +49: a pitch exactly half way between
//! two notes is written as the upper one at −50 ct.

use crate::key::NAMES;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The lowest and highest note (MIDI 0 = C-1, 127 = G9).
const LOWEST: i32 = 0;
const HIGHEST: i32 = 127;

/// A pitch in whole cents above C-1 (MIDI note 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotePitch(i32);

impl NotePitch {
    /// The pitch `cents` above C-1, kept within C-1 −50 ct … G9 +49 ct.
    pub fn from_cents(cents: i32) -> Self {
        Self(cents.clamp(LOWEST * 100 - 50, HIGHEST * 100 + 49))
    }

    /// Note `midi` at `cents` (any offset; it carries into the note).
    pub fn new(midi: i32, cents: i32) -> Self {
        Self::from_cents(midi * 100 + cents)
    }

    /// The pitch of a frequency, rounded to the cent (`None` outside the
    /// MIDI range or for no frequency).
    pub fn from_hz(hz: f64) -> Option<Self> {
        if !(hz.is_finite() && hz > 0.0) {
            return None;
        }
        let cents = (6900.0 + 1200.0 * (hz / 440.0).log2()).round();
        let range = f64::from(LOWEST * 100 - 50)..=f64::from(HIGHEST * 100 + 49);
        range.contains(&cents).then(|| Self(cents as i32))
    }

    /// Cents above C-1.
    pub fn total_cents(self) -> i32 {
        self.0
    }

    pub fn hz(self) -> f64 {
        440.0 * 2f64.powf((f64::from(self.0) - 6900.0) / 1200.0)
    }

    /// The nearest note as a MIDI number.
    pub fn midi(self) -> i32 {
        (self.0 + 50).div_euclid(100)
    }

    /// The offset from [`Self::midi`], −50..=49.
    pub fn cents(self) -> i32 {
        self.0 - self.midi() * 100
    }

    /// The note name with its octave: "A3", "F#2", "Bb4".
    pub fn name(self) -> String {
        let m = self.midi();
        format!("{}{}", NAMES[m.rem_euclid(12) as usize], m.div_euclid(12) - 1)
    }

    /// The offset as shown beside the name: "+12 ct", "-8 ct", "0 ct".
    pub fn cents_text(self) -> String {
        match self.cents() {
            0 => "0 ct".into(),
            c => format!("{c:+} ct"),
        }
    }

    /// `semitones` higher (lower when negative), the offset kept.
    pub fn transpose(self, semitones: i32) -> Self {
        Self::from_cents(self.0 + semitones * 100)
    }

    /// `cents` higher (lower when negative); past ±50 the note changes.
    pub fn detune(self, cents: i32) -> Self {
        Self::from_cents(self.0 + cents)
    }

    /// Reads a typed note: a letter, an optional sharp or flat (`#`, `b`,
    /// `♯`, `♭`), the octave, and optionally an offset in cents ("A3",
    /// "f#2", "Bb4", "C-1", "A3 +12", "A3 -8 ct"). B#3 is C4 and Cb4 is B3.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let mut chars = s.chars().peekable();
        let base: i32 = match chars.next()?.to_ascii_uppercase() {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return None,
        };
        let accidental = match chars.peek() {
            Some('#' | '♯') => 1,
            Some('b' | '♭') => -1,
            _ => 0,
        };
        if accidental != 0 {
            chars.next();
        }
        let rest: String = chars.collect();
        let rest = rest.trim_start();
        // The octave: an optional minus and one or two digits.
        let sign_len = usize::from(rest.starts_with('-'));
        let digits = rest[sign_len..].chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || digits > 2 {
            return None;
        }
        let octave: i32 = rest[..sign_len + digits].parse().ok()?;
        let tail = rest[sign_len + digits..].trim();
        let cents = if tail.is_empty() {
            0
        } else {
            let t = tail
                .strip_suffix("cents")
                .or_else(|| tail.strip_suffix("cent"))
                .or_else(|| tail.strip_suffix("ct"))
                .unwrap_or(tail)
                .trim()
                .replace('−', "-");
            if !(t.starts_with('+') || t.starts_with('-')) {
                return None;
            }
            let c: i32 = t.parse().ok()?;
            if !(-50..=50).contains(&c) {
                return None;
            }
            c
        };
        let midi = (octave + 1) * 12 + base + accidental;
        if !(LOWEST..=HIGHEST).contains(&midi) {
            return None;
        }
        Some(Self::new(midi, cents))
    }
}

impl fmt::Display for NotePitch {
    /// "A3 +12 ct".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name(), self.cents_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> NotePitch {
        NotePitch::parse(s).unwrap_or_else(|| panic!("{s} parses"))
    }

    #[test]
    fn names_and_octaves() {
        assert_eq!(NotePitch::new(69, 0).name(), "A4");
        assert_eq!(NotePitch::new(60, 0).name(), "C4");
        assert_eq!(NotePitch::new(59, 0).name(), "B3", "the octave changes at C");
        assert_eq!(NotePitch::new(57, 0).name(), "A3");
        assert_eq!(NotePitch::new(42, 0).name(), "F#2");
        assert_eq!(NotePitch::new(70, 0).name(), "Bb4");
        assert_eq!(NotePitch::new(0, 0).name(), "C-1");
        assert_eq!(NotePitch::new(127, 0).name(), "G9");
        assert_eq!(NotePitch::new(57, 12).to_string(), "A3 +12 ct");
        assert_eq!(NotePitch::new(57, -8).to_string(), "A3 -8 ct");
        assert_eq!(NotePitch::new(57, 0).to_string(), "A3 0 ct");
    }

    #[test]
    fn parses_sharps_flats_and_octave_boundaries() {
        assert_eq!(p("A3").midi(), 57);
        assert_eq!(p("a3").midi(), 57, "lower case");
        assert_eq!(p("F#2").midi(), 42);
        assert_eq!(p("Gb2").midi(), 42, "the same key spelled flat");
        assert_eq!(p("F♯2"), p("G♭2"));
        assert_eq!(p("Bb4").midi(), 70);
        assert_eq!(p("bb4").midi(), 70, "a lower-case b note with a flat");
        assert_eq!(p("B3").midi(), 59);
        assert_eq!(p("C4").midi(), 60);
        assert_eq!(p("B#3"), p("C4"), "B# crosses into the next octave");
        assert_eq!(p("Cb4"), p("B3"), "Cb falls into the octave below");
        assert_eq!(p("C-1").midi(), 0);
        assert_eq!(p("G9").midi(), 127);
        assert_eq!(p(" A3 "), p("A3"));
        assert_eq!(p("A3 +12").cents(), 12);
        assert_eq!(p("A3 -8 ct").cents(), -8);
        assert_eq!(p("A3+12ct").cents(), 12);
        for bad in ["", "H3", "A", "A#", "A3x", "A3 12", "A123", "G#9", "Cb-1", "A3 +60", "#3"] {
            assert_eq!(NotePitch::parse(bad), None, "{bad:?}");
        }
        // Name and parse agree on every note.
        for m in 0..=127 {
            assert_eq!(p(&NotePitch::new(m, 0).name()).midi(), m);
        }
    }

    #[test]
    fn cents_round_at_fifty() {
        // Half way up from A3 reads as Bb3 at −50, never A3 +50.
        let half = NotePitch::new(57, 50);
        assert_eq!((half.name().as_str(), half.cents()), ("Bb3", -50));
        let below = NotePitch::new(57, 49);
        assert_eq!((below.name().as_str(), below.cents()), ("A3", 49));
        let down = NotePitch::new(57, -50);
        assert_eq!((down.name().as_str(), down.cents()), ("A3", -50));
        let further = NotePitch::new(57, -51);
        assert_eq!((further.name().as_str(), further.cents()), ("Ab3", 49));
        assert_eq!(p("A3 +50"), p("Bb3 -50"));
        // From frequencies: 440 Hz is A4, a quarter tone up rounds to the cent.
        assert_eq!(NotePitch::from_hz(440.0), Some(NotePitch::new(69, 0)));
        let q = NotePitch::from_hz(440.0 * 2f64.powf(0.5 / 12.0)).unwrap();
        assert_eq!((q.name().as_str(), q.cents()), ("Bb4", -50));
        let up = NotePitch::from_hz(440.0 * 2f64.powf(0.494 / 12.0)).unwrap();
        assert_eq!((up.name().as_str(), up.cents()), ("A4", 49));
        assert!((NotePitch::new(57, 0).hz() - 220.0).abs() < 1e-9);
        assert_eq!(NotePitch::from_hz(0.0), None);
        assert_eq!(NotePitch::from_hz(f64::NAN), None);
        assert_eq!(NotePitch::from_hz(30_000.0), None);
        // Steps carry across the note and the octave.
        assert_eq!(NotePitch::new(59, 0).transpose(1).name(), "C4");
        assert_eq!(NotePitch::new(57, 49).detune(1).to_string(), "Bb3 -50 ct");
        assert_eq!(NotePitch::new(57, -50).detune(-1).to_string(), "Ab3 +49 ct");
    }
}
