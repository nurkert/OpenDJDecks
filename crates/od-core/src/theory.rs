//! Which notes fit (roadmap [78]): what a key shares with another, the
//! keys next to it that move smoothly, a check of a melody against it, and
//! the plain-word reasons THEORY gives. Everything is worked out from the
//! notes of the scales ([`ScaleKey`]); no names are looked up.
//!
//! The wheel is the circle of fifths: one step round it moves the root a
//! fifth up (clockwise) or down and changes one note of seven. A minor key
//! sits with its relative major, the major scale with the same notes.

use crate::scale::{Scale, ScaleKey, Triad};

/// How a neighbour moves from a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Neighbour {
    /// The other mode with the same notes (relative major or minor).
    Relative,
    /// A fifth up: one step clockwise round the wheel.
    FifthUp,
    /// A fifth down: one step anticlockwise.
    FifthDown,
}

/// The pitch classes of a chord on `root`.
pub fn chord_notes(root: u8, quality: Triad) -> [u8; 3] {
    let (third, fifth) = match quality {
        Triad::Major => (4, 7),
        Triad::Minor => (3, 7),
        Triad::Diminished => (3, 6),
        Triad::Augmented => (4, 8),
    };
    [root % 12, (root + third) % 12, (root + fifth) % 12]
}

impl ScaleKey {
    /// The major scale with the same notes (none for a scale that is not
    /// one of the major scale's modes, such as harmonic minor).
    pub fn parent_major(self) -> Option<ScaleKey> {
        let mut mine = self.pitch_classes();
        mine.sort_unstable();
        (0..12).map(|r| ScaleKey::new(r, Scale::Major)).find(|m| {
            let mut theirs = m.pitch_classes();
            theirs.sort_unstable();
            theirs == mine
        })
    }

    /// How many of the seven notes the two keys share.
    pub fn shared_notes(self, other: ScaleKey) -> usize {
        let theirs = other.pitch_classes();
        self.pitch_classes().iter().filter(|n| theirs.contains(n)).count()
    }

    /// Steps round the wheel from this key to `other`, -5..=6 (positive:
    /// clockwise, fifths up), measured between their major scales; `None`
    /// when either is not on the wheel.
    pub fn wheel_steps(self, other: ScaleKey) -> Option<i32> {
        let (a, b) = (self.parent_major()?, other.parent_major()?);
        // A fifth is 7 semitones and 7 × 7 = 49 ≡ 1 (mod 12): a root times
        // 7 is its place on the wheel.
        let d = ((i32::from(b.root) - i32::from(a.root)) * 7).rem_euclid(12);
        Some(if d > 6 { d - 12 } else { d })
    }

    /// The keys that move smoothly from a major or minor key: the relative
    /// key (the same notes), a fifth up and a fifth down (one note changes).
    /// None for the modes.
    pub fn neighbours(self) -> Vec<(Neighbour, ScaleKey)> {
        let other = match self.scale {
            Scale::Major => (i32::from(self.root) + 9, Scale::Minor),
            Scale::Minor => (i32::from(self.root) + 3, Scale::Major),
            _ => return Vec::new(),
        };
        vec![
            (Neighbour::Relative, ScaleKey::new(other.0, other.1)),
            (Neighbour::FifthUp, ScaleKey::new(i32::from(self.root) + 7, self.scale)),
            (Neighbour::FifthDown, ScaleKey::new(i32::from(self.root) + 5, self.scale)),
        ]
    }

    /// Which of the key's seven notes are in `other` too.
    pub fn notes_in(self, other: ScaleKey) -> [bool; 7] {
        self.pitch_classes().map(|n| other.contains(i32::from(n)))
    }

    /// Which of the key's seven chords have all three notes in `other`.
    pub fn chords_in(self, other: ScaleKey) -> [bool; 7] {
        self.triads().map(|(root, q)| chord_notes(root, q).iter().all(|&n| other.contains(i32::from(n))))
    }

    /// A melody (MIDI notes, in order) against the key.
    pub fn check(self, notes: &[i32]) -> Check {
        let mut distinct = Vec::new();
        let mut outside = Vec::new();
        for &n in notes {
            let pc = n.rem_euclid(12) as u8;
            if distinct.contains(&pc) {
                continue;
            }
            distinct.push(pc);
            if !self.contains(n) {
                outside.push(pc);
            }
        }
        Check { distinct: distinct.len(), outside }
    }
}

/// What a check of a melody found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    /// Different notes (pitch classes) in the melody.
    pub distinct: usize,
    /// The ones out of the key, as pitch classes in the order they first
    /// play.
    pub outside: Vec<u8>,
}

impl Check {
    pub fn fits(&self) -> bool {
        self.outside.is_empty()
    }
}

/// One plain-word reason on why notes fit (or do not), from the data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// Both decks play the same key: every note is shared.
    SameKey(ScaleKey),
    /// Two keys with the same notes in another mode.
    Relative(ScaleKey, ScaleKey),
    /// `to` lies `steps` round the wheel from `from` (sign: direction) and
    /// shares `shared` of seven notes.
    Steps { from: ScaleKey, to: ScaleKey, steps: i32, shared: usize },
    /// The melody (of `distinct` notes) uses only notes of the key.
    MelodyFits { key: ScaleKey, distinct: usize },
    /// The melody's notes out of the key (pitch classes).
    MelodyOutside { key: ScaleKey, outside: Vec<u8> },
    /// `fit` of the key's seven chords use only notes of `other`.
    Chords { key: ScaleKey, other: ScaleKey, fit: usize },
    /// The key's neighbours on the wheel: the relative key and the keys a
    /// fifth up and down.
    Neighbours { key: ScaleKey, relative: ScaleKey, up: ScaleKey, down: ScaleKey },
    /// The key's major chords (pitch-class roots), the brightest of its
    /// chords.
    MajorChords { key: ScaleKey, roots: Vec<u8> },
}

/// Up to three reasons, most telling first: how the two decks' keys
/// relate (else the key's neighbours), whether the melody fits the key in
/// use, and how many of its chords fit the key playing now; the key's
/// neighbours and its major chords fill up.
///
/// `key` is the key in use, `decks` the two decks' keys (if known), `now`
/// the key of the deck playing now, `melody` the check of the melody (none
/// without notes).
pub fn why(key: Option<ScaleKey>, decks: [Option<ScaleKey>; 2], now: Option<ScaleKey>, melody: Option<&Check>) -> Vec<Why> {
    let mut out = Vec::new();
    let mut push = |w: Option<Why>| {
        if let Some(w) = w {
            if out.len() < 3 && !out.contains(&w) {
                out.push(w);
            }
        }
    };
    if let [Some(a), Some(b)] = decks {
        push(Some(relation(a, b)));
    }
    if let (Some(k), Some(c)) = (key, melody) {
        push(Some(if c.fits() {
            Why::MelodyFits { key: k, distinct: c.distinct }
        } else {
            Why::MelodyOutside { key: k, outside: c.outside.clone() }
        }));
    }
    if let (Some(k), Some(n)) = (key, now) {
        if k.shared_notes(n) < 7 {
            push(Some(Why::Chords { key: k, other: n, fit: k.chords_in(n).iter().filter(|&&f| f).count() }));
        }
    }
    if let Some(k) = key {
        let ns = k.neighbours();
        if let [(_, relative), (_, up), (_, down)] = ns[..] {
            push(Some(Why::Neighbours { key: k, relative, up, down }));
        }
        let roots: Vec<u8> = k.triads().iter().filter(|(_, q)| *q == Triad::Major).map(|&(r, _)| r).collect();
        if !roots.is_empty() {
            push(Some(Why::MajorChords { key: k, roots }));
        }
    }
    out
}

/// How key `b` relates to key `a`, in one reason.
pub fn relation(a: ScaleKey, b: ScaleKey) -> Why {
    if a == b {
        return Why::SameKey(a);
    }
    let shared = a.shared_notes(b);
    match a.wheel_steps(b) {
        Some(0) if shared == 7 => Why::Relative(a, b),
        Some(steps) => Why::Steps { from: a, to: b, steps, shared },
        None => Why::Steps { from: a, to: b, steps: 0, shared },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(root: i32, scale: Scale) -> ScaleKey {
        ScaleKey::new(root, scale)
    }

    #[test]
    fn neighbours_are_the_relative_key_and_a_fifth_either_way() {
        let e = k(4, Scale::Major);
        let n = e.neighbours();
        assert_eq!(n[0], (Neighbour::Relative, k(1, Scale::Minor)), "C# minor");
        assert_eq!(n[1], (Neighbour::FifthUp, k(11, Scale::Major)), "B major");
        assert_eq!(n[2], (Neighbour::FifthDown, k(9, Scale::Major)), "A major");
        let am = k(9, Scale::Minor);
        assert_eq!(am.neighbours().iter().map(|n| n.1).collect::<Vec<_>>(), [k(0, Scale::Major), k(4, Scale::Minor), k(2, Scale::Minor)]);
        // Every major and minor key: the relative shares all seven notes,
        // a fifth either way six, one step round the wheel.
        for root in 0..12 {
            for scale in [Scale::Major, Scale::Minor] {
                let key = k(root, scale);
                for (how, other) in key.neighbours() {
                    let (shared, steps) = (key.shared_notes(other), key.wheel_steps(other));
                    match how {
                        Neighbour::Relative => assert_eq!((shared, steps), (7, Some(0)), "{key:?}"),
                        Neighbour::FifthUp => assert_eq!((shared, steps), (6, Some(1)), "{key:?}"),
                        Neighbour::FifthDown => assert_eq!((shared, steps), (6, Some(-1)), "{key:?}"),
                    }
                }
            }
        }
        assert!(k(2, Scale::Dorian).neighbours().is_empty(), "the modes have none");
    }

    #[test]
    fn the_wheel_counts_fifths_between_major_scales() {
        let c = k(0, Scale::Major);
        assert_eq!(c.wheel_steps(k(7, Scale::Major)), Some(1), "G: one up");
        assert_eq!(c.wheel_steps(k(5, Scale::Major)), Some(-1), "F: one down");
        assert_eq!(c.wheel_steps(k(2, Scale::Major)), Some(2), "D: two up");
        assert_eq!(c.wheel_steps(k(6, Scale::Major)), Some(6), "F#: across");
        assert_eq!(c.wheel_steps(k(0, Scale::Minor)), Some(-3), "C minor is Eb major's notes");
        assert_eq!(c.wheel_steps(k(2, Scale::Dorian)), Some(0), "D dorian is C major's notes");
        assert_eq!(c.wheel_steps(k(9, Scale::HarmonicMinor)), None, "not on the wheel");
        assert_eq!(k(2, Scale::Dorian).parent_major(), Some(c));
        assert_eq!(k(9, Scale::HarmonicMinor).parent_major(), None);
        assert_eq!(c.shared_notes(k(6, Scale::Major)), 2, "across the wheel");
    }

    #[test]
    fn chords_fit_a_key_when_all_their_notes_are_in_it() {
        assert_eq!(chord_notes(4, Triad::Major), [4, 8, 11]);
        assert_eq!(chord_notes(11, Triad::Diminished), [11, 2, 5]);
        let e = k(4, Scale::Major);
        // E major against B major: the chords with an A (F#m, A, D#dim)
        // do not fit, B major has A# instead.
        assert_eq!(e.chords_in(k(11, Scale::Major)), [true, false, true, false, true, true, false]);
        assert_eq!(e.notes_in(k(11, Scale::Major)), [true, true, true, false, true, true, true]);
        assert!(e.chords_in(k(1, Scale::Minor)).iter().all(|&f| f), "C# minor has them all");
        assert!(e.chords_in(e).iter().all(|&f| f));
        // Against C major every one of them has a sharp note.
        assert_eq!(e.chords_in(k(0, Scale::Major)).iter().filter(|&&f| f).count(), 0);
    }

    #[test]
    fn a_melody_check_names_the_notes_out_of_the_key_once_in_order() {
        let e = k(4, Scale::Major);
        let fits = e.check(&[64, 68, 71, 76, 73]);
        assert!(fits.fits());
        assert_eq!(fits.distinct, 4, "E twice counts once");
        let wrong = e.check(&[64, 65, 72, 77, 71]);
        assert_eq!(wrong.outside, [5, 0], "F then C, F once");
        assert_eq!(wrong.distinct, 4);
        assert!(!wrong.fits());
        assert!(e.check(&[]).fits());
    }

    #[test]
    fn why_lines_come_from_the_data() {
        let (e, cs, b) = (k(4, Scale::Major), k(1, Scale::Minor), k(11, Scale::Major));
        assert_eq!(relation(e, cs), Why::Relative(e, cs));
        assert_eq!(relation(e, b), Why::Steps { from: e, to: b, steps: 1, shared: 6 });
        assert_eq!(relation(e, e), Why::SameKey(e));
        assert_eq!(relation(e, k(10, Scale::Major)), Why::Steps { from: e, to: k(10, Scale::Major), steps: 6, shared: 2 });
        // Decks in E major and C# minor, a melody in key, deck 2 playing.
        let ok = e.check(&[64, 68]);
        let w = why(Some(e), [Some(e), Some(cs)], Some(cs), Some(&ok));
        assert_eq!(w.len(), 3);
        assert_eq!(w[0], Why::Relative(e, cs));
        assert_eq!(w[1], Why::MelodyFits { key: e, distinct: 2 });
        // Same notes: no chord line; the neighbours fill up.
        assert_eq!(w[2], Why::Neighbours { key: e, relative: cs, up: b, down: k(9, Scale::Major) });
        // B major playing, a melody with F and C.
        let bad = e.check(&[65, 72]);
        let w = why(Some(e), [Some(b), None], Some(b), Some(&bad));
        assert_eq!(w[0], Why::MelodyOutside { key: e, outside: vec![5, 0] });
        assert_eq!(w[1], Why::Chords { key: e, other: b, fit: 4 });
        assert!(matches!(w[2], Why::Neighbours { .. }));
        // Nothing playing, no melody: the key's own reasons.
        let w = why(Some(e), [None, None], None, None);
        assert_eq!(w, [Why::Neighbours { key: e, relative: cs, up: b, down: k(9, Scale::Major) }, Why::MajorChords { key: e, roots: vec![4, 9, 11] }]);
        assert!(why(None, [None, None], None, None).is_empty());
    }
}
