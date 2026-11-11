//! Evaluation of the phrase detector on generated tracks with a known
//! structure (test builds only).
//!
//! 31 arrangements across house (122–126 BPM), techno (128–135), drum &
//! bass (170–176), breakbeat, half-time, tracks without a drop, long intros
//! and tracks with 4-bar and 12-bar sections. Every track is rendered from
//! layers (kick, bass, hats, snare, snare roll, pad, lead, riser) with fades
//! and builds, see [`crate::testsig::render_arrangement`]. The detector gets
//! the true beatgrid, so the numbers measure phrase detection alone.
//!
//! Measured per track and summed over the set:
//!
//! - **Boundary hit rate:** the share of true section boundaries with a
//!   detected boundary within ±1 bar, and exactly on it; and the share of
//!   detected boundaries within ±1 bar of a true one (precision).
//! - **Label accuracy:** the share of bars whose detected kind is the true
//!   kind.
//! - **Fake drops:** detected Chorus sections of which less than half lies
//!   in a true drop.
//!
//! Truth and detection both pass through [`od_core::normalize_phrases`]
//! (neighbours of one kind are one section), as the app shows them.
//!
//! A **held-out** run renders the same arrangements with other seeds (key
//! and layer gains), up to ±1 BPM off the nominal tempo, a lead-in of up to
//! 0.4 s, mastering-style saturation and a noise floor, and gives the
//! detector the grid found by [`crate::tempo`] instead of the true one.
//! The detector was tuned on the clean run only; the held-out run checks
//! that it did not learn the clean renders by heart.
//!
//! These are generated tracks, not records: the numbers say the detector
//! does what it is built to do on clean, known structures, not how it
//! fares on any given real track.

use crate::testsig::{Groove, Mix, Section, render_arrangement};
use od_core::{Beatgrid, Phrase, PhraseKind, PhraseMood, normalize_phrases};
use rayon::prelude::*;
use std::time::Instant;

pub(crate) const FS: u32 = 11_025;

/// One generated track: name, tempo, drum pattern and its sections with
/// their true kind.
pub(crate) struct Case {
    pub name: &'static str,
    pub bpm: f64,
    pub groove: Groove,
    pub sections: Vec<(usize, PhraseKind, Mix, Mix, usize)>,
}

const fn mix(
    kick: f32,
    bass: f32,
    hats: f32,
    snare: f32,
    roll: f32,
    pad: f32,
    lead: f32,
    riser: f32,
) -> Mix {
    Mix {
        kick,
        bass,
        hats,
        snare,
        roll,
        pad,
        lead,
        riser,
    }
}

// Layer presets: kick, bass, hats, snare, roll, pad, lead, riser.
const SILENT: Mix = mix(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
const KICK: Mix = mix(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
const BEAT: Mix = mix(1.0, 0.0, 0.7, 0.3, 0.0, 0.0, 0.0, 0.0);
const GROOVE: Mix = mix(1.0, 0.8, 0.7, 0.6, 0.0, 0.2, 0.0, 0.0);
const FULL: Mix = mix(1.0, 1.0, 0.9, 0.8, 0.0, 0.5, 0.8, 0.0);
const BREAK: Mix = mix(0.0, 0.0, 0.0, 0.0, 0.0, 0.9, 0.3, 0.0);
const BREAK_BASS: Mix = mix(0.0, 0.4, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0);
const BUILD_FROM: Mix = mix(0.0, 0.0, 0.2, 0.0, 0.3, 0.6, 0.2, 0.1);
const BUILD_TO: Mix = mix(0.0, 0.0, 0.6, 0.0, 1.0, 0.8, 0.5, 1.0);
/// Drums without bass (drum & bass intros).
const DRUMS: Mix = mix(0.9, 0.0, 0.8, 0.7, 0.0, 0.3, 0.0, 0.0);

use PhraseKind::{Bridge, Chorus, Down, Intro, Outro, Up, Verse};

type S = (usize, PhraseKind, Mix, Mix, usize);

fn hold(bars: usize, kind: PhraseKind, m: Mix) -> S {
    (bars, kind, m, m, 0)
}
fn ramp(bars: usize, kind: PhraseKind, a: Mix, b: Mix) -> S {
    (bars, kind, a, b, 0)
}
fn alt(s: S) -> S {
    (s.0, s.1, s.2, s.3, 1)
}
fn build(bars: usize) -> S {
    (bars, Up, BUILD_FROM, BUILD_TO, 0)
}
fn with(m: Mix, f: impl Fn(&mut Mix)) -> Mix {
    let mut m = m;
    f(&mut m);
    m
}

/// The evaluation set.
#[rustfmt::skip]
pub(crate) fn cases() -> Vec<Case> {
    use Groove::*;
    let c = |name, bpm, groove, sections| Case {
        name,
        bpm,
        groove,
        sections,
    };
    let groove_pad = with(GROOVE, |m| m.pad = 0.4);
    let groove_lead = with(GROOVE, |m| m.lead = 0.4);
    let rumble = with(GROOVE, |m| {
        m.snare = 0.3;
        m.pad = 0.0
    });
    let techno_full = with(FULL, |m| {
        m.lead = 0.5;
        m.hats = 1.0;
        m.pad = 0.6
    });
    let hard_full = with(FULL, |m| {
        m.lead = 0.2;
        m.hats = 1.0;
        m.snare = 1.0
    });
    let liquid_break = with(BREAK, |m| {
        m.hats = 0.3;
        m.lead = 0.4
    });
    let sub = with(FULL, |m| {
        m.hats = 0.6;
        m.lead = 0.5
    });
    let deep = with(GROOVE, |m| {
        m.pad = 0.5;
        m.lead = 0.0
    });
    vec![
        // House.
        c("house-124-classic", 124.0, FourFloor, vec![
            hold(16, Intro, BEAT), hold(16, Verse, GROOVE), hold(16, Down, BREAK), build(8),
            hold(32, Chorus, FULL), hold(16, Bridge, groove_pad), alt(hold(8, Down, BREAK)), build(8),
            hold(32, Chorus, FULL), ramp(16, Outro, GROOVE, BEAT),
        ]),
        c("house-122-long-intro", 122.0, FourFloor, vec![
            ramp(32, Intro, KICK, BEAT), hold(32, Verse, GROOVE), hold(16, Down, BREAK_BASS),
            hold(32, Chorus, FULL), alt(hold(16, Down, BREAK)), hold(32, Chorus, FULL),
            ramp(32, Outro, BEAT, KICK),
        ]),
        c("house-126-vocal", 126.0, FourFloor, vec![
            hold(16, Intro, BEAT), hold(32, Verse, groove_lead), hold(8, Down, BREAK), build(8),
            hold(16, Chorus, FULL), hold(16, Bridge, groove_pad), build(8), hold(32, Chorus, FULL),
            hold(16, Outro, BEAT),
        ]),
        c("house-123-bridge", 123.0, FourFloor, vec![
            hold(16, Intro, BEAT), hold(16, Verse, GROOVE), build(8), hold(32, Chorus, FULL),
            hold(16, Bridge, rumble), build(8), hold(32, Chorus, FULL), hold(16, Outro, BEAT),
        ]),
        c("house-125-short", 125.0, FourFloor, vec![
            hold(16, Intro, BEAT), hold(16, Verse, GROOVE), build(8), hold(16, Chorus, FULL),
            hold(16, Down, BREAK), build(8), hold(16, Chorus, FULL), ramp(16, Outro, GROOVE, KICK),
        ]),
        c("house-124-fades", 124.0, FourFloor, vec![
            ramp(16, Intro, KICK, BEAT), ramp(16, Verse, GROOVE, groove_pad), ramp(16, Down, BREAK, BREAK_BASS),
            build(8), hold(32, Chorus, FULL), ramp(16, Down, BREAK, BREAK_BASS), build(8),
            hold(16, Chorus, FULL), ramp(32, Outro, BEAT, KICK),
        ]),
        // Techno.
        c("techno-130-classic", 130.0, FourFloor, vec![
            ramp(32, Intro, KICK, BEAT), hold(32, Verse, rumble), hold(16, Down, BREAK_BASS), build(16),
            hold(32, Chorus, techno_full), ramp(32, Outro, rumble, KICK),
        ]),
        c("techno-132-peak", 132.0, FourFloor, vec![
            hold(16, Intro, BEAT), hold(16, Verse, rumble), hold(8, Down, BREAK), build(8),
            hold(32, Chorus, techno_full), alt(hold(16, Down, BREAK)), build(8),
            hold(32, Chorus, techno_full), hold(16, Outro, BEAT),
        ]),
        c("techno-135-hard", 135.0, FourFloor, vec![
            hold(16, Intro, KICK), hold(32, Verse, rumble), hold(16, Down, BREAK_BASS), build(8),
            hold(32, Chorus, hard_full), hold(16, Bridge, rumble), build(8), hold(32, Chorus, hard_full),
            hold(16, Outro, KICK),
        ]),
        c("techno-128-melodic", 128.0, FourFloor, vec![
            hold(32, Intro, BEAT), hold(32, Verse, groove_pad), alt(hold(32, Down, BREAK)), build(8),
            hold(32, Chorus, techno_full), hold(16, Down, BREAK), build(8), hold(32, Chorus, techno_full),
            hold(32, Outro, BEAT),
        ]),
        c("techno-129-no-build", 129.0, FourFloor, vec![
            hold(16, Intro, BEAT), hold(32, Verse, rumble), hold(16, Down, BREAK), hold(32, Chorus, techno_full),
            hold(16, Down, BREAK_BASS), hold(32, Chorus, techno_full), hold(16, Outro, BEAT),
        ]),
        c("techno-134-rolling", 134.0, FourFloor, vec![
            ramp(16, Intro, KICK, BEAT), hold(32, Verse, rumble), build(16), hold(48, Chorus, techno_full),
            hold(16, Down, BREAK_BASS), build(8), hold(32, Chorus, techno_full), ramp(16, Outro, BEAT, KICK),
        ]),
        // Drum & bass.
        c("dnb-174-classic", 174.0, TwoStep, vec![
            hold(32, Intro, DRUMS), build(8), hold(32, Chorus, FULL), hold(16, Down, BREAK), build(8),
            hold(32, Chorus, FULL), hold(16, Outro, DRUMS),
        ]),
        c("dnb-172-liquid", 172.0, TwoStep, vec![
            // Drums without bass after the pads are still the mixable intro.
            hold(16, Intro, liquid_break), hold(16, Intro, DRUMS), build(8), hold(32, Chorus, FULL),
            hold(16, Down, liquid_break), build(8), hold(32, Chorus, FULL), hold(16, Outro, DRUMS),
        ]),
        c("dnb-176-neuro", 176.0, TwoStep, vec![
            hold(16, Intro, DRUMS), hold(16, Down, BREAK), build(8), hold(64, Chorus, sub),
            hold(16, Down, BREAK), build(8), hold(32, Chorus, sub), hold(32, Outro, DRUMS),
        ]),
        c("dnb-170-roller", 170.0, TwoStep, vec![
            ramp(32, Intro, with(DRUMS, |m| m.kick = 0.5), DRUMS), build(8), hold(32, Chorus, FULL),
            hold(32, Bridge, GROOVE), build(8), hold(32, Chorus, FULL), ramp(32, Outro, DRUMS, SILENT),
        ]),
        c("dnb-174-short-intro", 174.0, TwoStep, vec![
            hold(16, Intro, DRUMS), build(16), hold(32, Chorus, FULL), hold(32, Down, BREAK), build(8),
            hold(32, Chorus, FULL), hold(16, Outro, DRUMS),
        ]),
        // Breakbeat.
        c("breaks-130", 130.0, Breakbeat, vec![
            hold(16, Intro, BEAT), hold(16, Verse, GROOVE), hold(16, Down, BREAK), build(8),
            hold(32, Chorus, FULL), hold(16, Bridge, GROOVE), build(8), hold(32, Chorus, FULL),
            hold(16, Outro, BEAT),
        ]),
        c("breaks-134", 134.0, Breakbeat, vec![
            hold(32, Intro, BEAT), build(8), hold(32, Chorus, FULL), hold(16, Down, BREAK_BASS), build(8),
            hold(32, Chorus, FULL), hold(16, Outro, BEAT),
        ]),
        c("breaks-136-nu", 136.0, Breakbeat, vec![
            hold(16, Intro, BEAT), hold(32, Verse, groove_lead), hold(16, Down, BREAK), build(8),
            hold(32, Chorus, FULL), hold(16, Outro, GROOVE),
        ]),
        // Half-time.
        c("halftime-140-dubstep", 140.0, HalfTime, vec![
            hold(16, Intro, liquid_break), build(8), hold(16, Chorus, sub), hold(16, Down, BREAK), build(8),
            hold(16, Chorus, sub), hold(16, Outro, BEAT),
        ]),
        c("halftime-150-trap", 150.0, HalfTime, vec![
            hold(8, Intro, BREAK), build(8), hold(32, Chorus, FULL), hold(16, Bridge, groove_pad),
            build(8), hold(32, Chorus, FULL), hold(16, Outro, BREAK),
        ]),
        c("halftime-170-dnb", 170.0, HalfTime, vec![
            hold(32, Intro, DRUMS), build(8), hold(32, Chorus, sub), hold(16, Down, BREAK), build(8),
            hold(32, Chorus, sub), hold(32, Outro, DRUMS),
        ]),
        // No drop.
        c("nodrop-120-deep", 120.0, FourFloor, vec![
            hold(16, Intro, BEAT), ramp(32, Verse, GROOVE, deep), hold(32, Verse, deep),
            ramp(16, Outro, GROOVE, BEAT),
        ]),
        c("nodrop-127-minimal", 127.0, FourFloor, vec![
            ramp(32, Intro, KICK, BEAT), hold(32, Verse, rumble), ramp(32, Verse, rumble, GROOVE),
            hold(32, Verse, rumble), ramp(32, Outro, BEAT, KICK),
        ]),
        c("nodrop-100-downtempo", 100.0, Breakbeat, vec![
            hold(16, Intro, BREAK), hold(48, Verse, with(deep, |m| m.kick = 0.5)), hold(16, Outro, BREAK),
        ]),
        // Long intro.
        c("longintro-124-house", 124.0, FourFloor, vec![
            hold(32, Intro, KICK), hold(32, Intro, BEAT), hold(32, Verse, GROOVE), hold(16, Down, BREAK),
            build(8), hold(32, Chorus, FULL), hold(32, Outro, BEAT),
        ]),
        c("longintro-131-techno", 131.0, FourFloor, vec![
            ramp(48, Intro, KICK, BEAT), hold(16, Down, BREAK_BASS), build(8), hold(32, Chorus, techno_full),
            hold(16, Down, BREAK), hold(32, Chorus, techno_full), hold(32, Outro, BEAT),
        ]),
        // Oddities: 4-bar fills and 12-bar sections.
        c("odd-126-4bar-fill", 126.0, FourFloor, vec![
            hold(16, Intro, BEAT), hold(16, Verse, GROOVE), hold(12, Down, BREAK), build(4),
            hold(32, Chorus, FULL), hold(12, Down, BREAK), build(4), hold(32, Chorus, FULL),
            hold(16, Outro, BEAT),
        ]),
        c("odd-128-12bar", 128.0, FourFloor, vec![
            hold(12, Intro, BEAT), hold(24, Verse, GROOVE), hold(12, Down, BREAK), build(12),
            hold(24, Chorus, FULL), hold(12, Down, BREAK), hold(24, Chorus, FULL), hold(12, Outro, BEAT),
        ]),
        c("odd-124-shifted", 124.0, FourFloor, vec![
            hold(20, Intro, BEAT), hold(16, Verse, GROOVE), hold(16, Down, BREAK), build(8),
            hold(32, Chorus, FULL), hold(16, Down, BREAK), build(8), hold(32, Chorus, FULL),
            hold(16, Outro, BEAT),
        ]),
    ]
}

/// Renders `case`: the signal and its true beatgrid.
pub(crate) fn render(case: &Case, seed: u64) -> (Vec<f32>, Beatgrid) {
    let sections: Vec<Section> = case
        .sections
        .iter()
        .map(|&(bars, _, from, to, harmony)| Section {
            bars,
            from,
            to,
            harmony,
        })
        .collect();
    let x = render_arrangement(FS, case.bpm, case.groove, &sections, seed);
    let secs = x.len() as f64 / FS as f64;
    (x, Beatgrid::constant(case.bpm, 0.0, 1, secs))
}

/// The true phrases of `case`, normalized like detected ones.
pub(crate) fn truth(case: &Case) -> Vec<Phrase> {
    let mut bar = 0u32;
    let v: Vec<Phrase> = case
        .sections
        .iter()
        .map(|&(bars, kind, ..)| {
            let p = Phrase::new(bar * 4, (bar + bars as u32) * 4, kind);
            bar += bars as u32;
            p
        })
        .collect();
    normalize_phrases(&v)
}

/// Scores of one track.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Score {
    pub true_bounds: usize,
    pub hits_1: usize,
    pub hits_0: usize,
    pub found_bounds: usize,
    pub found_near: usize,
    pub bars: usize,
    pub bars_right: usize,
    pub fake_drops: usize,
    pub true_drops: usize,
    pub missed_drops: usize,
    /// Detected phrases (after normalization) flagged unsure, and how many
    /// of those are wrong (kind right on less than half their bars).
    pub unsure: usize,
    pub unsure_wrong: usize,
    pub sure: usize,
    pub sure_wrong: usize,
    /// Sum of the confidence of right and of wrong phrases.
    pub conf_right: f64,
    pub conf_wrong: f64,
}

impl std::ops::AddAssign for Score {
    fn add_assign(&mut self, o: Score) {
        self.true_bounds += o.true_bounds;
        self.hits_1 += o.hits_1;
        self.hits_0 += o.hits_0;
        self.found_bounds += o.found_bounds;
        self.found_near += o.found_near;
        self.bars += o.bars;
        self.bars_right += o.bars_right;
        self.fake_drops += o.fake_drops;
        self.true_drops += o.true_drops;
        self.missed_drops += o.missed_drops;
        self.unsure += o.unsure;
        self.unsure_wrong += o.unsure_wrong;
        self.sure += o.sure;
        self.sure_wrong += o.sure_wrong;
        self.conf_right += o.conf_right;
        self.conf_wrong += o.conf_wrong;
    }
}

impl Score {
    pub fn hit_rate_1(&self) -> f64 {
        self.hits_1 as f64 / self.true_bounds.max(1) as f64
    }
    pub fn hit_rate_0(&self) -> f64 {
        self.hits_0 as f64 / self.true_bounds.max(1) as f64
    }
    pub fn precision_1(&self) -> f64 {
        self.found_near as f64 / self.found_bounds.max(1) as f64
    }
    pub fn label_accuracy(&self) -> f64 {
        self.bars_right as f64 / self.bars.max(1) as f64
    }
}

/// Compares detected phrases with the truth (bars of four beats). A
/// boundary counts at its nearest bar and a bar takes the kind of the
/// phrase over its middle, so a downbeat a beat or two off (held-out run)
/// costs the exact boundary score, not the label score.
pub(crate) fn score(truth: &[Phrase], found: &[Phrase]) -> Score {
    let found = normalize_phrases(found);
    let starts = |v: &[Phrase]| -> Vec<i64> {
        v.iter()
            .skip(1)
            .map(|p| (p.start_beat as i64 + 2) / 4)
            .collect()
    };
    let tb = starts(truth);
    let fb = starts(&found);
    let near = |a: i64, set: &[i64], tol: i64| set.iter().any(|&b| (a - b).abs() <= tol);
    let total_bars = truth.last().map_or(0, |p| p.end_beat / 4);
    let kind_at = |v: &[Phrase], bar: u32| {
        let mid = bar * 4 + 2;
        v.iter()
            .find(|p| p.start_beat <= mid && mid < p.end_beat)
            .map(|p| p.kind)
    };
    let mut s = Score {
        true_bounds: tb.len(),
        hits_1: tb.iter().filter(|&&b| near(b, &fb, 1)).count(),
        hits_0: tb.iter().filter(|&&b| near(b, &fb, 0)).count(),
        found_bounds: fb.len(),
        found_near: fb.iter().filter(|&&b| near(b, &tb, 1)).count(),
        bars: total_bars as usize,
        bars_right: (0..total_bars)
            .filter(|&b| kind_at(truth, b).is_some() && kind_at(truth, b) == kind_at(&found, b))
            .count(),
        ..Default::default()
    };
    let overlap = |p: &Phrase, v: &[Phrase], kind: PhraseKind| {
        (p.start_beat / 4..p.end_beat / 4)
            .filter(|&b| kind_at(v, b) == Some(kind))
            .count() as f64
            / ((p.end_beat - p.start_beat) / 4).max(1) as f64
    };
    for p in found.iter().filter(|p| p.kind == PhraseKind::Chorus) {
        if overlap(p, truth, PhraseKind::Chorus) < 0.5 {
            s.fake_drops += 1;
        }
    }
    for p in truth.iter().filter(|p| p.kind == PhraseKind::Chorus) {
        s.true_drops += 1;
        if overlap(p, &found, PhraseKind::Chorus) < 0.5 {
            s.missed_drops += 1;
        }
    }
    for p in &found {
        let wrong = overlap(p, truth, p.kind) < 0.5;
        let c = p.confidence.unwrap_or(1.0) as f64;
        if wrong {
            s.conf_wrong += c;
        } else {
            s.conf_right += c;
        }
        if p.is_unsure() {
            s.unsure += 1;
            s.unsure_wrong += wrong as usize;
        } else {
            s.sure += 1;
            s.sure_wrong += wrong as usize;
        }
    }
    s
}

pub(crate) type Detector = fn(&[f32], u32, &Beatgrid) -> (Vec<Phrase>, PhraseMood);

/// The held-out render of case `i`: other seed, tempo off by up to ±1 BPM,
/// a lead-in, saturation and noise; with the grid the tempo detection
/// finds. Returns the signal, the detected grid, the lead-in (s) and the
/// true beat period (s).
pub(crate) fn render_held_out(case: &Case, i: usize) -> (Vec<f32>, Beatgrid, f64, f64) {
    let bpm = case.bpm + ((i % 5) as f64 - 2.0) * 0.5;
    let lead_in = 0.1 * (i % 5) as f64;
    let sections: Vec<Section> = case
        .sections
        .iter()
        .map(|&(bars, _, from, to, harmony)| Section {
            bars,
            from,
            to,
            harmony,
        })
        .collect();
    let body = render_arrangement(FS, bpm, case.groove, &sections, 5000 + i as u64);
    let mut noise = crate::testsig::Noise::new(77 + i as u64);
    let pad = (lead_in * FS as f64) as usize;
    let x: Vec<f32> = std::iter::repeat_n(0.0, pad)
        .chain(body)
        .map(|v| ((1.5 * v as f64).tanh() + 0.003 * noise.next()) as f32)
        .collect();
    let secs = x.len() as f64 / FS as f64;
    let grid =
        crate::tempo::detect_beatgrid_at(&x, FS as f64, secs, &crate::AnalysisOptions::default());
    (x, grid, lead_in, 60.0 / bpm)
}

/// Phrases found on a detected grid, moved onto the true grid (beat
/// indices counted from the true first downbeat).
fn onto_true_grid(found: &[Phrase], grid: &Beatgrid, lead_in: f64, period: f64) -> Vec<Phrase> {
    let at = |beat: u32| {
        let t = grid
            .beats
            .get(beat as usize)
            .map(|b| b.time)
            .or_else(|| grid.beats.last().map(|b| b.time + period))
            .unwrap_or(0.0);
        ((t - lead_in) / period).round().max(0.0) as u32
    };
    found
        .iter()
        .map(|p| Phrase {
            start_beat: at(p.start_beat),
            end_beat: at(p.end_beat),
            ..*p
        })
        .collect()
}

/// Runs `detect` over the whole set (clean, or the held-out variant).
/// Returns the summed score, the score per track and the slowest detection
/// time in seconds.
pub(crate) fn run(
    detect: Detector,
    held_out: bool,
) -> (Score, Vec<(&'static str, Score, Vec<Phrase>)>, f64) {
    let cases = cases();
    let per: Vec<_> = cases
        .par_iter()
        .enumerate()
        .map(|(i, c)| {
            let (x, grid, lead_in, period) = if held_out {
                render_held_out(c, i)
            } else {
                let (x, grid) = render(c, 1000 + i as u64);
                let period = 60.0 / c.bpm;
                (x, grid, 0.0, period)
            };
            let t = Instant::now();
            let (found, _) = detect(&x, FS, &grid);
            let el = t.elapsed().as_secs_f64();
            let found = onto_true_grid(&found, &grid, lead_in, period);
            (
                c.name,
                score(&truth(c), &found),
                normalize_phrases(&found),
                el,
            )
        })
        .collect();
    let mut total = Score::default();
    let mut slowest = 0f64;
    let mut out = Vec::new();
    for (name, s, found, el) in per {
        total += s;
        slowest = slowest.max(el);
        out.push((name, s, found));
    }
    (total, out, slowest)
}

fn row(name: &str, s: &Score) -> String {
    format!(
        "| {name} | {:.1} % | {:.1} % | {:.1} % | {:.1} % | {} | {}/{} |",
        100.0 * s.hit_rate_1(),
        100.0 * s.hit_rate_0(),
        100.0 * s.precision_1(),
        100.0 * s.label_accuracy(),
        s.fake_drops,
        s.missed_drops,
        s.true_drops
    )
}

fn compact(v: &[Phrase]) -> String {
    v.iter()
        .map(|p| {
            format!(
                "{:?}{}@{}{}",
                p.kind,
                (p.end_beat - p.start_beat) / 4,
                p.start_beat / 4,
                if p.is_unsure() { "?" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_is_broad_enough() {
        let cases = cases();
        assert!(cases.len() >= 30, "{}", cases.len());
        let in_range =
            |lo: f64, hi: f64| cases.iter().filter(|c| (lo..=hi).contains(&c.bpm)).count();
        assert!(in_range(122.0, 126.0) >= 5);
        assert!(in_range(128.0, 135.0) >= 5);
        assert!(in_range(170.0, 176.0) >= 5);
        assert!(
            cases
                .iter()
                .filter(|c| c.groove == Groove::Breakbeat)
                .count()
                >= 3
        );
        assert!(
            cases
                .iter()
                .filter(|c| c.groove == Groove::HalfTime)
                .count()
                >= 3
        );
        assert!(
            cases
                .iter()
                .filter(|c| !c.sections.iter().any(|s| s.1 == PhraseKind::Chorus))
                .count()
                >= 3
        );
        for c in &cases {
            let t = truth(c);
            assert_eq!(t.first().unwrap().start_beat, 0, "{}", c.name);
        }
    }

    #[test]
    fn scoring_counts_what_it_says() {
        let truth = normalize_phrases(&[
            Phrase::new(0, 64, Intro),
            Phrase::new(64, 128, Down),
            Phrase::new(128, 256, Chorus),
            Phrase::new(256, 320, Outro),
        ]);
        let perfect = score(&truth, &truth);
        assert_eq!((perfect.hits_0, perfect.true_bounds), (3, 3));
        assert_eq!(perfect.label_accuracy(), 1.0);
        assert_eq!(perfect.fake_drops, 0);
        // One bar late, a fake drop in the outro.
        let found = [
            Phrase::new(0, 68, Intro),
            Phrase::new(68, 128, Down),
            Phrase::new(128, 256, Chorus),
            Phrase::new(256, 320, Chorus),
        ];
        let s = score(&truth, &found);
        assert_eq!((s.hits_1, s.hits_0), (2, 1));
        // The outro became part of one long Chorus (normalized): no fake
        // drop on its own, but its 16 bars are wrong.
        assert_eq!(s.bars_right, 80 - 1 - 16);
        assert_eq!(s.fake_drops, 0);
        let found = [Phrase::new(0, 64, Chorus), Phrase::new(64, 320, Down)];
        assert_eq!(score(&truth, &found).fake_drops, 1);
    }

    /// The new detector against the old one on the whole set. Prints the
    /// table (run with `--nocapture`) and holds the new one to thresholds
    /// set below its measured results, with room for small changes.
    #[test]
    fn evaluation_set_old_vs_new() {
        let (old, old_per, old_slowest) = run(crate::phrase_v1::detect_phrases, false);
        let (new, new_per, new_slowest) = run(crate::phrase::detect_phrases, false);
        let (old_h, _, _) = run(crate::phrase_v1::detect_phrases, true);
        let (new_h, new_h_per, _) = run(crate::phrase::detect_phrases, true);
        println!(
            "| detector | boundary ±1 bar | exact | precision ±1 | label accuracy | fake drops | missed drops |"
        );
        println!("|---|---|---|---|---|---|---|");
        println!("{}", row("old", &old));
        println!("{}", row("new", &new));
        println!("{}", row("old, held-out", &old_h));
        println!("{}", row("new, held-out", &new_h));
        println!("slowest detection: old {old_slowest:.3} s, new {new_slowest:.3} s");
        for (name, s) in [("clean", &new), ("held-out", &new_h)] {
            let wrong = s.unsure_wrong + s.sure_wrong;
            let right = s.unsure + s.sure - wrong;
            println!(
                "confidence ({name}): unsure phrases wrong {}/{}, sure phrases wrong {}/{}; \
                 mean confidence right {:.2}, wrong {:.2}",
                s.unsure_wrong,
                s.unsure,
                s.sure_wrong,
                s.sure,
                s.conf_right / right.max(1) as f64,
                s.conf_wrong / wrong.max(1) as f64
            );
        }
        if std::env::var("OD_PHRASE_EVAL_DETAIL").is_ok() {
            let cases = cases();
            for ((c, (name, so, fo)), (_, sn, fnew)) in cases.iter().zip(&old_per).zip(&new_per) {
                println!("{}", row(&format!("{name} old"), so));
                println!("{}", row(&format!("{name} new"), sn));
                println!("    truth {}", compact(&truth(c)));
                println!("    old   {}", compact(fo));
                println!("    new   {}", compact(fnew));
            }
            for (c, (name, s, f)) in cases.iter().zip(&new_h_per) {
                println!("{}", row(&format!("{name} held-out"), s));
                println!("    truth {}", compact(&truth(c)));
                println!("    new   {}", compact(f));
            }
        }
        // Clearly better than the old detector on both runs.
        assert!(new.hit_rate_1() > old.hit_rate_1() + 0.1);
        assert!(new.label_accuracy() > old.label_accuracy() + 0.1);
        assert!(new.fake_drops < old.fake_drops);
        assert!(new_h.hit_rate_1() > old_h.hit_rate_1() + 0.1);
        assert!(new_h.label_accuracy() > old_h.label_accuracy() + 0.1);
        assert!(new_h.fake_drops < old_h.fake_drops);
        // Thresholds a few points under the measured results (clean 99 %,
        // held-out 96–100 %), so small changes pass and real regressions
        // do not.
        for (name, s) in [("clean", &new), ("held-out", &new_h)] {
            assert!(s.hit_rate_1() >= 0.93, "{name}: {}", row(name, s));
            assert!(s.precision_1() >= 0.9, "{name}: {}", row(name, s));
            assert!(s.label_accuracy() >= 0.93, "{name}: {}", row(name, s));
            assert!(s.fake_drops <= 1, "{name}: {}", row(name, s));
            assert!(s.missed_drops <= 2, "{name}: {}", row(name, s));
        }
        assert!(new.hit_rate_0() >= 0.9, "{}", row("new", &new));
    }
}

/// Detail of one track by name (`OD_PHRASE_CASE`), for tuning by hand.
#[test]
#[ignore = "manual; OD_PHRASE_CASE=<name> OD_PHRASE_DEBUG=1 -- --ignored --nocapture"]
fn one_case() {
    let name = std::env::var("OD_PHRASE_CASE").unwrap_or_default();
    for (i, c) in cases().iter().enumerate().filter(|(_, c)| c.name == name) {
        let (x, grid) = render(c, 1000 + i as u64);
        let (found, _) = crate::phrase::detect_phrases(&x, FS, &grid);
        println!("truth {}", compact(&truth(c)));
        println!("new   {}", compact(&normalize_phrases(&found)));
        if std::env::var("OD_PHRASE_HELD_OUT").is_ok() {
            let (x, grid, lead_in, period) = render_held_out(c, i);
            let (found, _) = crate::phrase::detect_phrases(&x, FS, &grid);
            let found = onto_true_grid(&found, &grid, lead_in, period);
            println!("held  {}", compact(&normalize_phrases(&found)));
        }
    }
}

/// Phrase detection time on a six-minute arrangement (192 bars at 128 BPM),
/// old and new detector, one after the other on a warm thread pool.
#[test]
#[ignore = "timing; -- --ignored --nocapture"]
fn timing_six_minutes() {
    let case = Case {
        name: "six-minutes",
        bpm: 128.0,
        groove: Groove::FourFloor,
        sections: vec![
            hold(32, Intro, BEAT),
            hold(32, Verse, GROOVE),
            hold(16, Down, BREAK),
            build(8),
            hold(32, Chorus, FULL),
            hold(16, Bridge, GROOVE),
            hold(8, Down, BREAK),
            build(8),
            hold(32, Chorus, FULL),
            hold(8, Outro, BEAT),
        ],
    };
    let (x, grid) = render(&case, 1);
    println!("track: {:.1} s", x.len() as f64 / FS as f64);
    for (name, detect) in [
        ("old", crate::phrase_v1::detect_phrases as Detector),
        ("new", crate::phrase::detect_phrases as Detector),
    ] {
        let _ = detect(&x, FS, &grid);
        let runs = 5;
        let t = Instant::now();
        for _ in 0..runs {
            let _ = detect(&x, FS, &grid);
        }
        println!(
            "{name}: {:.3} s per detection",
            t.elapsed().as_secs_f64() / runs as f64
        );
    }
    // The whole analysis (decode excluded) of the same track at 44.1 kHz.
    let sections: Vec<Section> = case
        .sections
        .iter()
        .map(|&(bars, _, from, to, harmony)| Section {
            bars,
            from,
            to,
            harmony,
        })
        .collect();
    let hi = render_arrangement(44_100, case.bpm, case.groove, &sections, 1);
    let audio = od_decode::DecodedAudio {
        sample_rate: 44_100,
        channels: 2,
        samples: hi.iter().flat_map(|&v| [v, v]).collect(),
    };
    let opts = crate::AnalysisOptions::default();
    let _ = crate::analyze(&audio, &opts);
    let t = Instant::now();
    let a = crate::analyze(&audio, &opts);
    println!(
        "full analysis: {:.3} s ({} phrases)",
        t.elapsed().as_secs_f64(),
        a.phrases.len()
    );
}
