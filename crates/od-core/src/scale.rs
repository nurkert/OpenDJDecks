//! A key as CREATE plays in it: a root note and a scale (roadmap [73]).
//!
//! A track's key ([`MusicalKey`]) is major or minor; CREATE also offers a
//! few modes. Minor is the natural minor (the notes of the relative major
//! from its sixth), as a track's minor key is read. Roots are pitch
//! classes (0 = C … 11 = B).
//!
//! Notes in a key are spelled as its key signature writes them: each of the
//! seven letters once, sharps in a sharp key and flats in a flat one (E
//! major: E F# G# A B C# D#; D minor: D E F G A Bb C). The root's spelling
//! is the one with the fewest accidentals and none doubled; the keys with
//! six either way (F# / Gb major, D# / Eb minor) are spelled with flats. A
//! mode is spelled from its own notes; harmonic minor from the natural minor
//! with the seventh raised (A harmonic minor: G#; G# harmonic minor: F##).
//! Notes out of the scale follow the key's side (sharps or flats); C major
//! and A minor and everything outside a key keep the app's spelling
//! ([`crate::key::NAMES`]).

use crate::key::{Mode, MusicalKey, NAMES};
use serde::{Deserialize, Serialize};

/// The scales CREATE offers: major and minor first, the modes behind MORE.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    HarmonicMinor,
}

impl Scale {
    /// Every scale, in the order offered.
    pub const ALL: [Scale; 7] = [
        Scale::Major,
        Scale::Minor,
        Scale::Dorian,
        Scale::Phrygian,
        Scale::Lydian,
        Scale::Mixolydian,
        Scale::HarmonicMinor,
    ];

    /// The name it is stored and passed to the interface by.
    pub fn id(self) -> &'static str {
        match self {
            Scale::Major => "major",
            Scale::Minor => "minor",
            Scale::Dorian => "dorian",
            Scale::Phrygian => "phrygian",
            Scale::Lydian => "lydian",
            Scale::Mixolydian => "mixolydian",
            Scale::HarmonicMinor => "harmonic_minor",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }

    /// The seven notes as semitones above the root.
    pub fn steps(self) -> [u8; 7] {
        match self {
            Scale::Major => [0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => [0, 2, 3, 5, 7, 8, 10],
            Scale::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Scale::Lydian => [0, 2, 4, 6, 7, 9, 11],
            Scale::Mixolydian => [0, 2, 4, 5, 7, 9, 10],
            Scale::HarmonicMinor => [0, 2, 3, 5, 7, 8, 11],
        }
    }

    /// Behind MORE: not one of the two a track's key is read as.
    pub fn is_mode(self) -> bool {
        !matches!(self, Scale::Major | Scale::Minor)
    }
}

/// The quality of a chord of three notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Triad {
    Major,
    Minor,
    Diminished,
    Augmented,
}

impl Triad {
    /// What follows the root in a chord's name: "", "m", "dim", "aug".
    pub fn suffix(self) -> &'static str {
        match self {
            Triad::Major => "",
            Triad::Minor => "m",
            Triad::Diminished => "dim",
            Triad::Augmented => "aug",
        }
    }
}

/// A root note and a scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScaleKey {
    /// Pitch class, 0 = C … 11 = B.
    pub root: u8,
    pub scale: Scale,
}

impl ScaleKey {
    pub fn new(root: i32, scale: Scale) -> Self {
        Self { root: root.rem_euclid(12) as u8, scale }
    }

    /// A track's key: its tonic, major or (natural) minor.
    pub fn from_key(k: MusicalKey) -> Self {
        let scale = match k.mode {
            Mode::Major => Scale::Major,
            Mode::Minor => Scale::Minor,
        };
        Self::new(i32::from(k.tonic), scale)
    }

    /// The track key it is, for major and minor (the modes have none).
    pub fn musical_key(self) -> Option<MusicalKey> {
        match self.scale {
            Scale::Major => Some(MusicalKey::new(self.root, Mode::Major)),
            Scale::Minor => Some(MusicalKey::new(self.root, Mode::Minor)),
            _ => None,
        }
    }

    /// The root's name as the key spells it: "A", "F#", "Bb".
    pub fn root_name(self) -> String {
        self.spelled()[0].to_string()
    }

    /// The scale's seven notes from the root, one letter each.
    pub fn spelled(self) -> [Spelled; 7] {
        // The root's spelling: the letter at, below or above its pitch class
        // with the fewest accidentals in the scale (harmonic minor counts as
        // natural minor), none doubled; at a tie flats.
        let counted = if self.scale == Scale::HarmonicMinor { Scale::Minor } else { self.scale };
        let mut best: Option<(i32, Spelled)> = None;
        for letter in 0..7u8 {
            let acc = (i32::from(self.root) - i32::from(NATURAL[usize::from(letter)]) + 6).rem_euclid(12) - 6;
            if acc.abs() > 1 {
                continue;
            }
            let notes = spell_steps(Spelled { letter, acc: acc as i8 }, counted.steps());
            let doubled = notes.iter().any(|n| n.acc.abs() > 1);
            let count: i32 = notes.iter().map(|n| i32::from(n.acc.abs())).sum();
            let flats = notes.iter().any(|n| n.acc < 0);
            let score = count * 4 + if doubled { 100 } else { 0 } + if flats { 0 } else { 1 };
            if best.is_none_or(|(b, _)| score < b) {
                best = Some((score, Spelled { letter, acc: acc as i8 }));
            }
        }
        let root = best.map_or(Spelled { letter: 0, acc: 0 }, |(_, r)| r);
        spell_steps(root, self.scale.steps())
    }

    /// Which side the key writes its notes on: 1 sharps, -1 flats, 0 none
    /// (C major, A minor).
    pub fn side(self) -> i32 {
        self.spelled().iter().map(|n| i32::from(n.acc)).sum::<i32>().signum()
    }

    /// A pitch class (or MIDI note) named as the key spells it: a scale
    /// note by its letter, any other on the key's side.
    pub fn name_of(self, note: i32) -> String {
        self.spelling_of(note).to_string()
    }

    fn spelling_of(self, note: i32) -> Spelled {
        let pc = note.rem_euclid(12);
        if let Some(n) = self.spelled().into_iter().find(|n| n.pitch_class() == pc) {
            return n;
        }
        let names = match self.side() {
            1 => SHARPS,
            -1 => FLATS,
            _ => NAMES,
        };
        Spelled::parse(names[pc as usize])
    }

    /// A MIDI note with its octave as the key spells it: "C#4", "Bb3".
    /// The octave goes with the letter (B#3 sounds as C4, Cb4 as B3).
    pub fn note_name(self, midi: i32) -> String {
        let n = self.spelling_of(midi);
        format!("{n}{}", (midi - i32::from(n.acc)).div_euclid(12) - 1)
    }

    /// The scale's pitch classes from the root up.
    pub fn pitch_classes(self) -> [u8; 7] {
        self.scale.steps().map(|s| (self.root + s) % 12)
    }

    /// Whether a note (MIDI number, or any pitch class) is in the scale.
    pub fn contains(self, note: i32) -> bool {
        let pc = note.rem_euclid(12) as u8;
        self.pitch_classes().contains(&pc)
    }

    /// The scale's note names from the root: "A B C D E F G".
    pub fn note_names(self) -> [String; 7] {
        self.spelled().map(|n| n.to_string())
    }

    /// The root at or below `note` (MIDI): where the pads start.
    pub fn root_at_or_below(self, note: i32) -> i32 {
        note - (note - i32::from(self.root)).rem_euclid(12)
    }

    /// `count` notes of the scale from `from` (MIDI) up, `from` included
    /// when it is in the scale.
    pub fn notes_from(self, from: i32, count: usize) -> Vec<i32> {
        (from..).filter(|&n| self.contains(n)).take(count).collect()
    }

    /// The chords on the scale's seven notes (each note with the third and
    /// the fifth above it in the scale), as (root pitch class, quality).
    pub fn triads(self) -> [(u8, Triad); 7] {
        let steps = self.scale.steps();
        std::array::from_fn(|i| {
            let at = |k: usize| i32::from(steps[(i + k) % 7]) + if i + k >= 7 { 12 } else { 0 };
            let (third, fifth) = (at(2) - at(0), at(4) - at(0));
            let quality = match (third, fifth) {
                (4, 7) => Triad::Major,
                (3, 7) => Triad::Minor,
                (3, 6) => Triad::Diminished,
                _ => Triad::Augmented,
            };
            ((self.root + steps[i]) % 12, quality)
        })
    }

    /// The chords' names, spelled as the key: "Am Bdim C Dm Em F G".
    pub fn chord_names(self) -> [String; 7] {
        let notes = self.spelled();
        let triads = self.triads();
        std::array::from_fn(|i| format!("{}{}", notes[i], triads[i].1.suffix()))
    }
}

/// The natural notes' pitch classes, C D E F G A B.
const NATURAL: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
/// Notes out of a sharp or a flat key.
const SHARPS: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLATS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];

/// A note as written: a letter and its sharps (positive) or flats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spelled {
    /// 0 = C … 6 = B.
    pub letter: u8,
    pub acc: i8,
}

impl Spelled {
    pub fn pitch_class(self) -> i32 {
        (i32::from(NATURAL[usize::from(self.letter)]) + i32::from(self.acc)).rem_euclid(12)
    }

    /// From "C", "F#", "Bb".
    fn parse(s: &str) -> Self {
        let letter = LETTERS.iter().position(|&c| s.starts_with(c)).unwrap_or(0) as u8;
        let acc = s.chars().skip(1).map(|c| if c == '#' { 1 } else { -1 }).sum();
        Self { letter, acc }
    }
}

impl std::fmt::Display for Spelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let acc = match self.acc {
            a if a > 0 => "#".repeat(a as usize),
            a => "b".repeat(a.unsigned_abs() as usize),
        };
        write!(f, "{}{acc}", LETTERS[usize::from(self.letter)])
    }
}

/// The seven notes `steps` above `root`, one letter each from the root's.
fn spell_steps(root: Spelled, steps: [u8; 7]) -> [Spelled; 7] {
    let base = root.pitch_class();
    std::array::from_fn(|i| {
        let letter = (root.letter + i as u8) % 7;
        let want = (base + i32::from(steps[i])).rem_euclid(12);
        let acc = (want - i32::from(NATURAL[usize::from(letter)]) + 6).rem_euclid(12) - 6;
        Spelled { letter, acc: acc as i8 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> MusicalKey {
        MusicalKey::parse(s).unwrap()
    }

    #[test]
    fn every_scale_has_seven_rising_notes_from_the_root() {
        for s in Scale::ALL {
            let steps = s.steps();
            assert_eq!(steps[0], 0, "{s:?} starts on the root");
            assert!(steps.windows(2).all(|w| w[0] < w[1] && w[1] - w[0] <= 3), "{s:?} rises by 1 to 3 semitones");
            assert_eq!(Scale::from_id(s.id()), Some(s));
        }
        assert_eq!(Scale::from_id("blues"), None);
        // The modes are the major scale from another degree.
        let major = Scale::Major.steps();
        let rotate = |from: usize| -> [u8; 7] { std::array::from_fn(|i| (major[(from + i) % 7] + 12 - major[from]) % 12) };
        assert_eq!(rotate(1), Scale::Dorian.steps());
        assert_eq!(rotate(2), Scale::Phrygian.steps());
        assert_eq!(rotate(3), Scale::Lydian.steps());
        assert_eq!(rotate(4), Scale::Mixolydian.steps());
        assert_eq!(rotate(5), Scale::Minor.steps(), "natural minor from the sixth");
        // Harmonic minor raises the seventh.
        let mut h = Scale::Minor.steps();
        h[6] += 1;
        assert_eq!(h, Scale::HarmonicMinor.steps());
        assert!(!Scale::Major.is_mode() && !Scale::Minor.is_mode() && Scale::Dorian.is_mode());
    }

    #[test]
    fn track_keys_become_root_and_scale() {
        let am = ScaleKey::from_key(key("8A"));
        assert_eq!((am.root, am.scale), (9, Scale::Minor));
        assert_eq!(am.note_names(), ["A", "B", "C", "D", "E", "F", "G"]);
        let c = ScaleKey::from_key(key("8B"));
        assert_eq!((c.root_name().as_str(), c.scale), ("C", Scale::Major));
        assert_eq!(c.pitch_classes(), [0, 2, 4, 5, 7, 9, 11]);
        assert_eq!(ScaleKey::from_key(key("F#m")).note_names(), ["F#", "G#", "A", "B", "C#", "D", "E"]);
        assert_eq!(ScaleKey::from_key(key("Bb")).note_names(), ["Bb", "C", "D", "Eb", "F", "G", "A"]);
        // And back, for major and minor only.
        for n in 1..=12u8 {
            for minor in [false, true] {
                let k = MusicalKey::from_camelot(n, minor).unwrap();
                assert_eq!(ScaleKey::from_key(k).musical_key(), Some(k));
            }
        }
        assert_eq!(ScaleKey::new(9, Scale::Dorian).musical_key(), None);
        // A relative major and minor share their notes.
        let mut a = ScaleKey::from_key(key("8A")).pitch_classes();
        let mut b = ScaleKey::from_key(key("8B")).pitch_classes();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        assert_eq!(ScaleKey::new(-3, Scale::Minor).root, 9, "roots wrap");
    }

    #[test]
    fn pads_start_on_the_root_at_or_below_the_note() {
        let am = ScaleKey::new(9, Scale::Minor);
        assert_eq!(am.root_at_or_below(57), 57, "A3 is the root");
        assert_eq!(am.root_at_or_below(60), 57, "C4: the A below");
        assert_eq!(am.root_at_or_below(56), 45, "Ab3: the A an octave lower");
        // An octave of the scale, root to root.
        assert_eq!(am.notes_from(57, 8), [57, 59, 60, 62, 64, 65, 67, 69]);
        assert_eq!(am.notes_from(58, 2), [59, 60], "from a note out of the scale");
        assert!(am.contains(60) && am.contains(-3) && !am.contains(61));
    }

    #[test]
    fn chords_of_major_minor_and_modes() {
        assert_eq!(ScaleKey::new(0, Scale::Major).chord_names(), ["C", "Dm", "Em", "F", "G", "Am", "Bdim"]);
        assert_eq!(ScaleKey::new(9, Scale::Minor).chord_names(), ["Am", "Bdim", "C", "Dm", "Em", "F", "G"]);
        assert_eq!(ScaleKey::new(2, Scale::Dorian).chord_names(), ["Dm", "Em", "F", "G", "Am", "Bdim", "C"]);
        assert_eq!(ScaleKey::new(9, Scale::HarmonicMinor).chord_names(), ["Am", "Bdim", "Caug", "Dm", "E", "F", "G#dim"]);
        assert_eq!(ScaleKey::new(4, Scale::Major).chord_names(), ["E", "F#m", "G#m", "A", "B", "C#m", "D#dim"]);
    }

    /// Every key in major and minor: each letter once, as its key signature
    /// writes it.
    #[test]
    fn notes_are_spelled_as_the_key_signature_writes_them() {
        let major = [
            "C D E F G A B", "Db Eb F Gb Ab Bb C", "D E F# G A B C#", "Eb F G Ab Bb C D",
            "E F# G# A B C# D#", "F G A Bb C D E", "Gb Ab Bb Cb Db Eb F", "G A B C D E F#",
            "Ab Bb C Db Eb F G", "A B C# D E F# G#", "Bb C D Eb F G A", "B C# D# E F# G# A#",
        ];
        let minor = [
            "C D Eb F G Ab Bb", "C# D# E F# G# A B", "D E F G A Bb C", "Eb F Gb Ab Bb Cb Db",
            "E F# G A B C D", "F G Ab Bb C Db Eb", "F# G# A B C# D E", "G A Bb C D Eb F",
            "G# A# B C# D# E F#", "A B C D E F G", "Bb C Db Eb F Gb Ab", "B C# D E F# G A",
        ];
        for root in 0..12 {
            for (scale, want) in [(Scale::Major, major[root]), (Scale::Minor, minor[root])] {
                let k = ScaleKey::new(root as i32, scale);
                assert_eq!(k.note_names().join(" "), want, "{root} {scale:?}");
                let mut letters: Vec<u8> = k.spelled().iter().map(|n| n.letter).collect();
                letters.sort_unstable();
                assert_eq!(letters, [0, 1, 2, 3, 4, 5, 6], "each letter once: {want}");
                // The spelling sounds as the scale.
                let pcs: Vec<i32> = k.spelled().iter().map(|n| n.pitch_class()).collect();
                assert_eq!(pcs, k.pitch_classes().map(i32::from).to_vec());
            }
        }
        // Modes and harmonic minor.
        assert_eq!(ScaleKey::new(2, Scale::Dorian).note_names().join(" "), "D E F G A B C");
        assert_eq!(ScaleKey::new(4, Scale::Phrygian).note_names().join(" "), "E F G A B C D");
        assert_eq!(ScaleKey::new(5, Scale::Lydian).note_names().join(" "), "F G A B C D E");
        assert_eq!(ScaleKey::new(7, Scale::Mixolydian).note_names().join(" "), "G A B C D E F");
        assert_eq!(ScaleKey::new(3, Scale::Dorian).note_names().join(" "), "Eb F Gb Ab Bb C Db");
        assert_eq!(ScaleKey::new(9, Scale::HarmonicMinor).note_names().join(" "), "A B C D E F G#");
        assert_eq!(ScaleKey::new(2, Scale::HarmonicMinor).note_names().join(" "), "D E F G A Bb C#");
        assert_eq!(ScaleKey::new(8, Scale::HarmonicMinor).note_names().join(" "), "G# A# B C# D# E F##");
        // Notes with octaves, and notes out of the scale on the key's side.
        let e = ScaleKey::new(4, Scale::Major);
        assert_eq!((e.note_name(61), e.note_name(63), e.note_name(70)), ("C#4".into(), "D#4".into(), "A#4".into()));
        let f = ScaleKey::new(5, Scale::Major);
        assert_eq!((f.note_name(70), f.note_name(61)), ("Bb4".into(), "Db4".into()));
        let gb = ScaleKey::new(6, Scale::Major);
        assert_eq!(gb.note_name(59), "Cb4", "Cb4 sounds as B3");
        assert_eq!(ScaleKey::new(0, Scale::Major).note_name(61), "Db4", "C major keeps the app's spelling");
        assert_eq!((e.side(), f.side(), ScaleKey::new(9, Scale::Minor).side()), (1, -1, 0));
        assert_eq!(ScaleKey::new(1, Scale::Minor).root_name(), "C#");
        assert_eq!(ScaleKey::new(1, Scale::Major).root_name(), "Db");
    }
}
