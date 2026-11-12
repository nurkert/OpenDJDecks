//! CREATE's arrange strip (roadmap [74]): the melody MELODY makes, as up to
//! [`STEPS`] steps on the master grid. Each step is empty or holds one note
//! (MIDI) that lasts one or more steps. The grid (1/4, 1/8 or 1/16) says how
//! long a step is, so 32 steps are 8, 4 or 2 bars.
//!
//! Notes come in two ways: played in while REC is armed (on the nearest
//! step of the master grid while the master plays, else step by step at a
//! cursor), or typed as note names ("A3 C4 - E4", see [`parse`]), which
//! replaces the strip from step 1. The model here is pure; create.rs wires
//! it to the pads, the audition voice and the store.

use od_core::{NotePitch, ScaleKey};
use serde::{Deserialize, Serialize};

/// Steps of the strip.
pub(crate) const STEPS: usize = 32;
/// Beats per bar (the app's grids are in 4/4).
pub(crate) const BEATS_PER_BAR: u32 = 4;

/// How long a step is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Grid {
    #[serde(rename = "1/4")]
    Quarter,
    #[default]
    #[serde(rename = "1/8")]
    Eighth,
    #[serde(rename = "1/16")]
    Sixteenth,
}

impl Grid {
    pub const ALL: [Grid; 3] = [Grid::Quarter, Grid::Eighth, Grid::Sixteenth];

    /// "1/4", "1/8", "1/16".
    pub fn id(self) -> &'static str {
        match self {
            Grid::Quarter => "1/4",
            Grid::Eighth => "1/8",
            Grid::Sixteenth => "1/16",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|g| g.id() == id)
    }

    /// Steps in a beat: 1, 2 or 4.
    pub fn per_beat(self) -> u32 {
        match self {
            Grid::Quarter => 1,
            Grid::Eighth => 2,
            Grid::Sixteenth => 4,
        }
    }

    /// Steps in a bar: 4, 8 or 16.
    pub fn per_bar(self) -> u32 {
        self.per_beat() * BEATS_PER_BAR
    }
}

/// One note of the strip: its pitch (MIDI) and how many steps it lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Step {
    pub note: i32,
    pub len: u8,
}

impl Step {
    pub fn new(note: i32) -> Self {
        Self { note: note.clamp(0, 127), len: 1 }
    }
}

/// The strip: [`STEPS`] steps, the grid, and whether PLAY loops it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Melody {
    pub steps: Vec<Option<Step>>,
    pub grid: Grid,
    #[serde(rename = "loop")]
    pub looped: bool,
}

impl Default for Melody {
    fn default() -> Self {
        Self { steps: vec![None; STEPS], grid: Grid::default(), looped: false }
    }
}

impl Melody {
    /// Exactly [`STEPS`] steps, every note at least one step long and none
    /// running into the next note or past the end (after an edit, or as
    /// read back).
    pub fn normalize(&mut self) {
        self.steps.resize(STEPS, None);
        let starts: Vec<usize> = (0..STEPS).filter(|&i| self.steps[i].is_some()).collect();
        for (k, &i) in starts.iter().enumerate() {
            let next = starts.get(k + 1).copied().unwrap_or(STEPS);
            if let Some(s) = self.steps[i].as_mut() {
                s.note = s.note.clamp(0, 127);
                s.len = s.len.clamp(1, (next - i) as u8);
            }
        }
    }

    /// How many notes it holds.
    pub fn notes(&self) -> usize {
        self.steps.iter().flatten().count()
    }

    /// Steps up to the end of the last note (0 when empty).
    pub fn used(&self) -> usize {
        self.steps.iter().enumerate().filter_map(|(i, s)| s.map(|s| i + usize::from(s.len))).max().unwrap_or(0)
    }

    /// Whole bars the notes take on the grid (0 when empty).
    pub fn bars(&self) -> usize {
        self.used().div_ceil(self.grid.per_bar() as usize)
    }

    /// Steps PLAY goes through: the whole bars.
    pub fn play_steps(&self) -> usize {
        (self.bars() * self.grid.per_bar() as usize).min(STEPS)
    }

    /// The note sounding on step `i`: its own, or one held over it.
    pub fn held_by(&self, i: usize) -> Option<usize> {
        (0..=i.min(STEPS - 1)).rev().find(|&j| self.steps[j].is_some_and(|s| j + usize::from(s.len) > i))
    }

    /// Puts `note` on step `i` (one step long); a note held over it ends
    /// before it.
    pub fn set(&mut self, i: usize, note: i32) {
        if i < STEPS {
            if let Some(j) = self.held_by(i).filter(|&j| j < i)
                && let Some(s) = self.steps[j].as_mut()
            {
                s.len = (i - j) as u8;
            }
            self.steps[i] = Some(Step::new(note));
            self.normalize();
        }
    }

    /// Empties step `i`.
    pub fn delete(&mut self, i: usize) {
        if let Some(s) = self.steps.get_mut(i) {
            *s = None;
        }
    }

    /// Moves the note on step `i` one step left (`d` < 0) or right, swapping
    /// it with what is there; returns where it is now (`i` when it cannot
    /// move).
    pub fn move_step(&mut self, i: usize, d: i32) -> usize {
        let j = i as i64 + i64::from(d.signum());
        if i >= STEPS || self.steps[i].is_none() || !(0..STEPS as i64).contains(&j) {
            return i;
        }
        let j = j as usize;
        // A note moved onto the tail of a held note shortens it, as a new
        // one would; the moved note keeps its length where there is room.
        let mut moved = self.steps[i].take();
        let other = self.steps[j].take();
        if let Some(h) = self.held_by(j)
            && let Some(s) = self.steps[h].as_mut()
        {
            s.len = (j - h) as u8;
        }
        self.steps[i] = other;
        if let Some(m) = moved.as_mut() {
            m.len = m.len.max(1);
        }
        self.steps[j] = moved;
        self.normalize();
        j
    }

    /// The note on step `i` a step of `key`'s scale up or down (`d`); a
    /// semitone without a key or with `semitone`. False when there is none.
    pub fn transpose(&mut self, i: usize, d: i32, key: Option<ScaleKey>, semitone: bool) -> bool {
        let Some(s) = self.steps.get_mut(i).and_then(Option::as_mut) else { return false };
        s.note = step_note(s.note, d, key.filter(|_| !semitone));
        true
    }

    /// The strip replaced by typed steps, from step 1.
    pub fn replace(&mut self, steps: Vec<Option<Step>>) {
        self.steps = steps;
        self.normalize();
    }

    /// Every step empty (the grid and LOOP stay).
    pub fn clear(&mut self) {
        self.steps = vec![None; STEPS];
    }

    /// The notes as they can be typed again: "A3 C4 - E4 _".
    pub fn text(&self, key: Option<ScaleKey>) -> String {
        let mut out: Vec<String> = Vec::new();
        let mut i = 0;
        while i < self.used() {
            match self.steps[i] {
                Some(s) => {
                    out.push(name(key, s.note));
                    out.extend(std::iter::repeat_n("_".to_string(), usize::from(s.len) - 1));
                    i += usize::from(s.len);
                }
                None => {
                    out.push("-".into());
                    i += 1;
                }
            }
        }
        out.join(" ")
    }
}

/// A note's name as `key` spells it, else as the app does.
pub(crate) fn name(key: Option<ScaleKey>, midi: i32) -> String {
    key.map_or_else(|| NotePitch::new(midi, 0).name(), |k| k.note_name(midi))
}

/// `note` a step of `key`'s scale up or down (from a note out of the
/// scale: the nearest scale note that way), or a semitone without a key.
pub(crate) fn step_note(note: i32, d: i32, key: Option<ScaleKey>) -> i32 {
    let d = d.signum();
    if d == 0 {
        return note;
    }
    let mut n = note + d;
    if let Some(k) = key {
        while (0..=127).contains(&n) && !k.contains(n) {
            n += d;
        }
    }
    if (0..=127).contains(&n) { n } else { note }
}

/// A typed melody: the steps from step 1, and what could not be read.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Typed {
    pub steps: Vec<Option<Step>>,
    /// Tokens that are not a note, a rest or a hold, as typed.
    pub bad: Vec<String>,
    /// Steps beyond [`STEPS`] were left out.
    pub too_long: bool,
}

/// Reads typed notes separated by spaces (or commas): a note name with its
/// octave as [`NotePitch::parse`] reads it ("A3", "F#2", "Bb4"; cents are
/// ignored), "-" or "." for a rest, "_" to hold the note before one more
/// step. A token that is none of these is reported and takes its step as a
/// rest, so the notes after it stay where they were typed.
pub(crate) fn parse(text: &str) -> Typed {
    let mut t = Typed::default();
    let mut last: Option<usize> = None;
    for tok in text.split([' ', ',', '\t', '\n']).filter(|s| !s.is_empty()) {
        if t.steps.len() >= STEPS {
            t.too_long = true;
            break;
        }
        match tok {
            "-" | "." | "–" => {
                t.steps.push(None);
                last = None;
            }
            "_" => match last.and_then(|i| t.steps[i].as_mut()) {
                Some(s) => {
                    s.len += 1;
                    t.steps.push(None);
                }
                // Nothing to hold: a rest.
                None => t.steps.push(None),
            },
            _ => match NotePitch::parse(tok) {
                Some(p) => {
                    last = Some(t.steps.len());
                    t.steps.push(Some(Step::new(p.midi())));
                }
                None => {
                    t.bad.push(tok.to_string());
                    t.steps.push(None);
                    last = None;
                }
            },
        }
    }
    t.steps.resize(STEPS, None);
    t
}

/// The step a note played at master beat `beat` lands on: the nearest step
/// of the grid counted from `origin` (the bar REC started in, or where
/// PLAY started), around the strip. Rounded as the sampler's pattern
/// recording rounds its steps.
pub(crate) fn nearest_step(beat: f64, origin: f64, grid: Grid) -> usize {
    (((beat - origin) * f64::from(grid.per_beat())).round() as i64).rem_euclid(STEPS as i64) as usize
}

/// The first beat of the bar `beat` is in, from the master deck's place in
/// its bar (1..=4; 0 without a grid: bars from beat 0).
pub(crate) fn bar_start(beat: f64, beat_in_bar: u8) -> f64 {
    if beat_in_bar > 0 {
        beat.floor() - f64::from(beat_in_bar - 1)
    } else {
        (beat / f64::from(BEATS_PER_BAR)).floor() * f64::from(BEATS_PER_BAR)
    }
}

/// The step (counted from 0, without wrapping) that sounds at `beat` when
/// PLAY started on `start`; negative before it.
pub(crate) fn step_at(beat: f64, start: f64, grid: Grid) -> i64 {
    ((beat - start) * f64::from(grid.per_beat())).floor() as i64
}

/// The beat step `k` (counted from 0) starts on.
pub(crate) fn beat_of(k: i64, start: f64, grid: Grid) -> f64 {
    start + k as f64 / f64::from(grid.per_beat())
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::Scale;

    fn melody(notes: &[(usize, i32)]) -> Melody {
        let mut m = Melody::default();
        for &(i, n) in notes {
            m.set(i, n);
        }
        m
    }

    #[test]
    fn the_grid_sets_the_bars() {
        let mut m = melody(&[(0, 57), (9, 60)]);
        assert_eq!((m.notes(), m.used()), (2, 10));
        assert_eq!((m.bars(), m.play_steps()), (2, 16), "1/8: 8 steps a bar");
        m.grid = Grid::Quarter;
        assert_eq!(m.bars(), 3, "1/4: 4 steps a bar");
        m.grid = Grid::Sixteenth;
        assert_eq!(m.bars(), 1);
        m.set(31, 64);
        assert_eq!((m.bars(), m.play_steps()), (2, 32), "32 steps of 1/16 are 2 bars");
        m.grid = Grid::Quarter;
        assert_eq!(m.bars(), 8, "and of 1/4 the most there is: 8 bars");
        assert_eq!(Melody::default().bars(), 0);
        assert_eq!(Grid::from_id("1/16"), Some(Grid::Sixteenth));
        assert_eq!(Grid::from_id("1/3"), None);
    }

    #[test]
    fn never_more_than_32_steps() {
        let mut m = Melody::default();
        m.set(32, 60);
        m.set(40, 60);
        assert_eq!(m.notes(), 0, "past the end: nothing");
        let long = vec!["A3"; 40].join(" ");
        let t = parse(&long);
        assert!(t.too_long);
        assert_eq!(t.steps.len(), STEPS);
        m.replace(t.steps);
        assert_eq!((m.notes(), m.steps.len()), (32, 32));
        // Read back with too many steps and a note running past the end.
        let mut odd = Melody { steps: vec![Some(Step { note: 60, len: 9 }); 40], ..Melody::default() };
        odd.normalize();
        assert_eq!(odd.steps.len(), 32);
        assert!(odd.steps.iter().flatten().all(|s| s.len == 1), "each ends at the next");
        let mut tail = Melody::default();
        tail.steps[30] = Some(Step { note: 60, len: 9 });
        tail.normalize();
        assert_eq!(tail.steps[30].unwrap().len, 2);
    }

    #[test]
    fn typing_reads_notes_rests_holds_and_reports_the_rest() {
        let t = parse("A3 C4 - E4 . D#4 Eb4 _ _ B3");
        assert!(t.bad.is_empty() && !t.too_long);
        let notes: Vec<Option<(i32, u8)>> = t.steps[..10].iter().map(|s| s.map(|s| (s.note, s.len))).collect();
        assert_eq!(
            notes,
            [Some((57, 1)), Some((60, 1)), None, Some((64, 1)), None, Some((63, 1)), Some((63, 3)), None, None, Some((59, 1))]
        );
        assert!(t.steps[10..].iter().all(Option::is_none));
        // Sharps and flats, lower case, commas; cents are dropped.
        let t = parse("f#2, Gb2,bb4 A3+12");
        assert_eq!(t.steps[..4].iter().map(|s| s.unwrap().note).collect::<Vec<_>>(), [42, 42, 70, 57]);
        // Not a note: reported, its step left empty, the rest kept in place.
        let t = parse("A3 H4 C4 X");
        assert_eq!(t.bad, ["H4", "X"]);
        assert_eq!((t.steps[0].map(|s| s.note), t.steps[1], t.steps[2].map(|s| s.note)), (Some(57), None, Some(60)));
        // A hold with nothing to hold is a rest.
        assert_eq!(parse("_ A3").steps[1].map(|s| s.note), Some(57));
        assert_eq!(parse("").steps, vec![None; STEPS]);
        // And back to text, as typed.
        let mut m = Melody::default();
        m.replace(parse("A3 - Eb4 _ C4").steps);
        assert_eq!(m.text(None), "A3 - Eb4 _ C4");
        assert_eq!(m.text(Some(ScaleKey::new(4, Scale::Major))), "A3 - D#4 _ C4", "spelled as the key");
    }

    #[test]
    fn move_delete_and_transpose_within_the_scale() {
        let mut m = melody(&[(0, 57), (1, 60), (3, 64)]);
        assert_eq!(m.move_step(0, 1), 1, "swapped with C4");
        assert_eq!((m.steps[0].map(|s| s.note), m.steps[1].map(|s| s.note)), (Some(60), Some(57)));
        assert_eq!(m.move_step(1, 1), 2, "onto an empty step");
        assert!(m.steps[1].is_none());
        assert_eq!(m.move_step(0, -1), 0, "not past the start");
        assert_eq!(m.move_step(5, 1), 5, "an empty step does not move");
        m.set(31, 50);
        assert_eq!(m.move_step(31, 1), 31, "not past the end");
        // A held note shortens where another moves onto it.
        let mut h = Melody::default();
        h.replace(parse("A3 _ _ - C4").steps);
        assert_eq!(h.move_step(4, -1), 3);
        assert_eq!(h.steps[0].unwrap().len, 3, "still clear of it");
        assert_eq!(h.move_step(3, -1), 2);
        assert_eq!((h.steps[0].unwrap().len, h.held_by(1), h.held_by(2)), (2, Some(0), Some(2)));
        // Delete.
        m.delete(2);
        assert!(m.steps[2].is_none());
        m.delete(99);
        // In A minor: a scale step from C4 is D4 and B3; a semitone C#4.
        let am = Some(ScaleKey::new(9, Scale::Minor));
        let mut t = melody(&[(0, 60)]);
        assert!(t.transpose(0, 1, am, false));
        assert_eq!(t.steps[0].unwrap().note, 62);
        t.transpose(0, -1, am, false);
        t.transpose(0, -1, am, false);
        assert_eq!(t.steps[0].unwrap().note, 59);
        t.transpose(0, 1, am, true);
        assert_eq!(t.steps[0].unwrap().note, 60, "a semitone with the modifier");
        t.transpose(0, 1, None, false);
        assert_eq!(t.steps[0].unwrap().note, 61, "without a key: a semitone");
        t.transpose(0, 1, am, false);
        assert_eq!(t.steps[0].unwrap().note, 62, "from C#4 out of the scale: D4");
        assert!(!t.transpose(5, 1, am, false), "an empty step");
        assert_eq!(step_note(127, 1, None), 127, "stays in range");
    }

    #[test]
    fn a_played_note_lands_on_the_nearest_step() {
        // From bar 8 (beat 32) on 1/8: two steps a beat.
        assert_eq!(nearest_step(32.0, 32.0, Grid::Eighth), 0);
        assert_eq!(nearest_step(32.24, 32.0, Grid::Eighth), 0, "a little late: the step before");
        assert_eq!(nearest_step(32.26, 32.0, Grid::Eighth), 1, "nearer the next");
        assert_eq!(nearest_step(33.9, 32.0, Grid::Eighth), 4);
        assert_eq!(nearest_step(31.9, 32.0, Grid::Eighth), 0, "just before the bar: its first step");
        assert_eq!(nearest_step(31.6, 32.0, Grid::Eighth), 31, "earlier: the end of the strip");
        assert_eq!(nearest_step(48.0, 32.0, Grid::Eighth), 0, "after 4 bars it comes round");
        assert_eq!(nearest_step(33.0, 32.0, Grid::Sixteenth), 4);
        assert_eq!(nearest_step(33.0, 32.0, Grid::Quarter), 1);
        // Where the bar starts, from the master's place in it.
        assert_eq!(bar_start(34.5, 3), 32.0);
        assert_eq!(bar_start(34.5, 0), 32.0, "no grid: bars from beat 0");
        assert_eq!(bar_start(32.0, 1), 32.0);
        // PLAY: the step sounding, and where a step starts.
        assert_eq!(step_at(35.9, 36.0, Grid::Eighth), -1);
        assert_eq!(step_at(37.6, 36.0, Grid::Eighth), 3);
        assert_eq!(beat_of(3, 36.0, Grid::Eighth), 37.5);
        assert_eq!(beat_of(3, 36.0, Grid::Sixteenth), 36.75);
    }

    #[test]
    fn stored_and_read_back() {
        let mut m = melody(&[(0, 57), (5, 62)]);
        m.grid = Grid::Sixteenth;
        m.looped = true;
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"1/16\"") && json.contains("\"loop\":true"), "{json}");
        let back: Melody = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        let old: Melody = serde_json::from_str("{}").unwrap();
        assert_eq!(old, Melody::default());
    }
}
