//! Transients and grid evidence for beatgrid editing.
//!
//! [`transients`] finds the attacks of a track (kicks marked `low`), so the
//! grid editor can show where the beats really are, snap edits to them and
//! measure how well a grid fits: [`grid_error`] pairs every beat with its
//! transient, [`fit`] turns that into a percentage and [`fit_line`] finds
//! the slip and tempo error a grid has (and where a tempo jumps).
//!
//! Nothing here is stored: the editor computes it from the samples a deck
//! or the preview already decoded.

use crate::dsp::{Biquad, decimate, decimation_factor, energy_prefix, mean_square};
use od_core::Beatgrid;

/// One attack in the audio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transient {
    /// Onset in seconds.
    pub time: f64,
    /// Detection strength (relative; larger is a harder attack).
    pub strength: f32,
    /// Mostly low-frequency energy (below ≈ 150 Hz): a kick.
    pub low: bool,
}

/// Analysis rate of the detector (Hz, approximately).
const RATE: u32 = 11_025;
/// Energy windows after and before a candidate onset (seconds).
const FWD: f64 = 0.003;
const BWD: f64 = 0.030;
/// Peaks must be the largest within this distance (seconds).
const PEAK_HALF: f64 = 0.025;
/// Adaptive threshold: the mean detection within ±1 s, times this.
const THRESHOLD_FACTOR: f64 = 1.5;
/// Window that decides whether an attack is low (seconds).
const LOW_WINDOW: f64 = 0.025;

/// Attacks of a mono signal, in time order.
///
/// The signal is decimated to ≈ 11 kHz; the detection function is the
/// ratio of the energy in the next 3 ms to the energy of the 30 ms before
/// (weighted by the attack's own level so silence does not count), and
/// peaks are local maxima within ±25 ms above an adaptive threshold over
/// 2 s. `low` compares the 150 Hz low-passed energy with the full energy
/// right after the onset.
pub fn transients(mono: &[f32], sample_rate: u32) -> Vec<Transient> {
    if mono.is_empty() || sample_rate == 0 {
        return Vec::new();
    }
    let factor = decimation_factor(sample_rate, RATE);
    let x = decimate(mono, factor);
    let fs = sample_rate as f64 / factor as f64;
    let fwd = ((FWD * fs).round() as usize).max(1);
    let bwd = ((BWD * fs).round() as usize).max(1);
    let prefix = energy_prefix(&x);
    let mut lowx = x.clone();
    Biquad::lowpass(fs, 150.0, std::f64::consts::FRAC_1_SQRT_2).process(&mut lowx);
    let low_prefix = energy_prefix(&lowx);
    let n = x.len();
    // Silence floor: −70 dBFS mean square.
    let floor = 1e-7;
    let d: Vec<f64> = (0..n)
        .map(|i| {
            if i < 1 {
                return 0.0;
            }
            let f = mean_square(&prefix, i, i + fwd);
            let b = mean_square(&prefix, i.saturating_sub(bwd), i);
            let ratio = (f + floor) / (b + floor);
            if ratio <= 1.0 { 0.0 } else { ratio.ln() * f.sqrt() }
        })
        .collect();
    // Adaptive threshold: mean of the detection within ±1 s.
    let mut dp = Vec::with_capacity(n + 1);
    let mut acc = 0.0;
    dp.push(0.0);
    for v in &d {
        acc += v;
        dp.push(acc);
    }
    let win = fs as usize;
    let max_d = d.iter().copied().fold(0.0, f64::max);
    let abs_floor = max_d * 0.02;
    let half = ((PEAK_HALF * fs).round() as usize).max(1);
    let low_n = ((LOW_WINDOW * fs).round() as usize).max(1);
    let mut out = Vec::new();
    let mut i = 1;
    while i < n {
        let v = d[i];
        if v <= abs_floor {
            i += 1;
            continue;
        }
        let (a, b) = (i.saturating_sub(win), (i + win).min(n));
        let mean = (dp[b] - dp[a]) / (b - a) as f64;
        if v < mean * THRESHOLD_FACTOR {
            i += 1;
            continue;
        }
        let (lo, hi) = (i.saturating_sub(half), (i + half + 1).min(n));
        // Strict maximum to the left, maximum to the right (plateaus once).
        let is_peak = d[lo..i].iter().all(|&u| u < v) && d[i + 1..hi].iter().all(|&u| u <= v);
        if is_peak {
            let full = mean_square(&prefix, i, i + low_n);
            let low = mean_square(&low_prefix, i, i + low_n);
            out.push(Transient {
                time: i as f64 / fs,
                strength: v as f32,
                low: full > 0.0 && low / full > 0.5,
            });
            i += half;
        } else {
            i += 1;
        }
    }
    out
}

/// The transient an edit snaps to near `t`: within `window` seconds, the
/// strongest one, kicks counting double and nearer ones a little more.
pub fn snap(transients: &[Transient], t: f64, window: f64) -> Option<f64> {
    let a = transients.partition_point(|x| x.time < t - window);
    transients[a..]
        .iter()
        .take_while(|x| x.time <= t + window)
        .map(|x| {
            let near = 1.0 - 0.5 * ((x.time - t).abs() / window.max(1e-9));
            let w = x.strength as f64 * if x.low { 2.0 } else { 1.0 } * near;
            (x.time, w)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(time, _)| time)
}

/// How far a beat may be from its transient to count as matched (s).
pub const MATCH_WINDOW: f64 = 0.060;

/// Error of one beat of a grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeatError {
    /// Index into the grid's beats.
    pub beat: usize,
    /// Transient minus beat in seconds (positive: the beat is early).
    pub err: f64,
    /// A transient lies within [`MATCH_WINDOW`]; `err` is 0 otherwise.
    pub matched: bool,
}

/// Every beat of `grid` with the error to its transient (the strongest
/// within ±60 ms, kicks preferred).
pub fn grid_error(grid: &Beatgrid, transients: &[Transient]) -> Vec<BeatError> {
    grid.beats
        .iter()
        .enumerate()
        .map(|(i, b)| match snap(transients, b.time, MATCH_WINDOW) {
            Some(t) => BeatError { beat: i, err: t - b.time, matched: true },
            None => BeatError { beat: i, err: 0.0, matched: false },
        })
        .collect()
}

/// How well a grid sits on its transients, 0–100 %: every matched beat
/// scores 1 up to 5 ms off, falling to 0 at 30 ms; beats without a
/// transient do not count unless almost none match (then the fit drops).
pub fn fit(errors: &[BeatError]) -> f32 {
    let matched: Vec<f64> = errors.iter().filter(|e| e.matched).map(|e| e.err.abs()).collect();
    if matched.is_empty() {
        return 0.0;
    }
    let score: f64 = matched
        .iter()
        .map(|e| if *e <= 0.005 { 1.0 } else { (1.0 - (e - 0.005) / 0.025).max(0.0) })
        .sum::<f64>()
        / matched.len() as f64;
    let coverage = (2.0 * matched.len() as f64 / errors.len().max(1) as f64).min(1.0);
    (100.0 * score * coverage) as f32
}

/// Median error of the matched beats among `errors` (seconds).
pub fn median_error(errors: &[BeatError]) -> Option<f64> {
    let mut v: Vec<f64> = errors.iter().filter(|e| e.matched).map(|e| e.err).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let m = v.len() / 2;
    Some(if v.len() % 2 == 0 { (v[m - 1] + v[m]) / 2.0 } else { v[m] })
}

/// The systematic error of a grid: `err ≈ slip + drift · (beat − ref)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitLine {
    /// Error at beat `reference` (seconds; move the grid by this).
    pub slip: f64,
    /// Error gained per beat (seconds; the beat period is off by this).
    pub drift: f64,
    /// Beat index the slip refers to (the middle of the matched beats).
    pub reference: usize,
    /// A beat where the error jumps by more than a few ms and stays: the
    /// track changes tempo (or was edited) there.
    pub split: Option<usize>,
    /// Matched beats the line rests on.
    pub used: usize,
}

impl FitLine {
    /// The tempo the grid should have, given its current period (s).
    pub fn bpm(&self, period: f64) -> f64 {
        60.0 / (period + self.drift)
    }
}

/// Least-squares line through the per-beat errors (robust: beats far off
/// the first line are dropped and the line fitted again).
fn line(points: &[(f64, f64)]) -> Option<(f64, f64, f64)> {
    let n = points.len() as f64;
    if points.len() < 2 {
        return None;
    }
    let mx = points.iter().map(|p| p.0).sum::<f64>() / n;
    let my = points.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let b = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    Some((my - b * mx, b, mx))
}

/// Slip and tempo error of a grid from its per-beat errors (AUTO-FIX),
/// plus the beat where the error jumps, if it does.
pub fn fit_line(errors: &[BeatError]) -> Option<FitLine> {
    let mut pts: Vec<(f64, f64)> = errors
        .iter()
        .filter(|e| e.matched)
        .map(|e| (e.beat as f64, e.err))
        .collect();
    if pts.len() < 4 {
        return None;
    }
    let (mut a, mut b, _) = line(&pts)?;
    for _ in 0..3 {
        let mut res: Vec<f64> = pts.iter().map(|p| (p.1 - a - b * p.0).abs()).collect();
        res.sort_by(f64::total_cmp);
        let mad = res[res.len() / 2].max(0.001);
        let keep: Vec<(f64, f64)> = pts
            .iter()
            .copied()
            .filter(|p| (p.1 - a - b * p.0).abs() <= 3.0 * mad)
            .collect();
        if keep.len() < 4 || keep.len() == pts.len() {
            break;
        }
        pts = keep;
        (a, b, _) = line(&pts)?;
    }
    let mid = pts[pts.len() / 2].0;
    let split = find_split(errors);
    Some(FitLine {
        slip: a + b * mid,
        drift: b,
        reference: mid as usize,
        split,
        used: pts.len(),
    })
}

/// A bar where the error steps by more than 8 ms between the eight beats
/// before and the eight after, both sides steady (spread under 4 ms).
fn find_split(errors: &[BeatError]) -> Option<usize> {
    let m: Vec<&BeatError> = errors.iter().filter(|e| e.matched).collect();
    const SIDE: usize = 8;
    if m.len() < 2 * SIDE {
        return None;
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let mut best: Option<(usize, f64)> = None;
    for k in SIDE..=m.len() - SIDE {
        let mut before: Vec<f64> = m[k - SIDE..k].iter().map(|e| e.err).collect();
        let mut after: Vec<f64> = m[k..k + SIDE].iter().map(|e| e.err).collect();
        let spread = |v: &[f64]| {
            let lo = v.iter().copied().fold(f64::MAX, f64::min);
            let hi = v.iter().copied().fold(f64::MIN, f64::max);
            hi - lo
        };
        if spread(&before) > 0.004 || spread(&after) > 0.004 {
            continue;
        }
        let step = (median(&mut after) - median(&mut before)).abs();
        if step > 0.008 && best.is_none_or(|(_, s)| step > s) {
            best = Some((m[k].beat, step));
        }
    }
    best.map(|(b, _)| b)
}

/// Problem spots for the drift strip and [ / ]: matched beats more than
/// 15 ms off, as `(beat, err)`, worst first within runs (one per bar).
pub fn problems(errors: &[BeatError]) -> Vec<(usize, f64)> {
    let mut out: Vec<(usize, f64)> = Vec::new();
    for e in errors.iter().filter(|e| e.matched && e.err.abs() > 0.015) {
        match out.last_mut() {
            Some(last) if e.beat < last.0 + 4 => {
                if e.err.abs() > last.1.abs() {
                    *last = (e.beat, e.err);
                }
            }
            _ => out.push((e.beat, e.err)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsig::{click_track, kick_track};

    #[test]
    fn finds_the_kicks_of_a_kick_track_within_2_ms() {
        let fs = 44_100;
        let (x, beats) = kick_track(fs, 128.0, 0.137, 30.0, true);
        let tr = transients(&x, fs);
        let kicks: Vec<&Transient> = tr.iter().filter(|t| t.low).collect();
        assert!(kicks.len() >= beats.len() - 1, "{} kicks for {} beats", kicks.len(), beats.len());
        for b in &beats {
            let near = kicks.iter().map(|k| k.time - b).min_by(|a, c| a.abs().total_cmp(&c.abs())).unwrap();
            assert!(near.abs() < 0.002, "kick at {b}: off by {:.2} ms", near * 1000.0);
        }
        // Drum & bass: kicks and snares alternate; snares are no kicks.
        let (x, beats) = kick_track(fs, 172.0, 0.05, 20.0, false);
        let tr = transients(&x, fs);
        for (n, b) in beats.iter().enumerate().take(40) {
            let t = tr.iter().find(|t| (t.time - b).abs() < 0.002).expect("every hit found");
            assert_eq!(t.low, n % 2 == 0, "hit {n} at {b}");
        }
    }

    #[test]
    fn snap_prefers_the_kick() {
        let tr = [
            Transient { time: 1.0, strength: 1.0, low: false },
            Transient { time: 1.03, strength: 0.8, low: true },
        ];
        assert_eq!(snap(&tr, 1.01, 0.06), Some(1.03));
        assert_eq!(snap(&tr, 1.2, 0.06), None);
    }

    #[test]
    fn align_beat_with_snap() {
        let fs = 44_100;
        let (x, beats) = click_track(fs, 120.0, 0.25, 20.0);
        let tr = transients(&x, fs);
        // A grid 20 ms late; the user sets beat 1 near the 5th click.
        let mut g = Beatgrid::constant(120.0, 0.27, 1, 20.0);
        let t = snap(&tr, beats[4] + 0.04, MATCH_WINDOW).unwrap();
        g.align_beat(t);
        assert!((g.beats[4].time - beats[4]).abs() < 0.002, "{}", g.beats[4].time);
        assert!(fit(&grid_error(&g, &tr)) > 95.0);
    }

    #[test]
    fn fit_line_recovers_slip_and_tempo_error() {
        let fs = 44_100;
        let (x, beats) = kick_track(fs, 128.0, 0.1, 60.0, true);
        let tr = transients(&x, fs);
        // The grid: 12 ms late at the start and 0.05 BPM too fast.
        let g = Beatgrid::constant(128.05, 0.112, 1, 60.0);
        let errors = grid_error(&g, &tr);
        let before = fit(&errors);
        let l = fit_line(&errors).unwrap();
        let period = 60.0 / 128.05;
        assert!((l.bpm(period) - 128.0).abs() < 0.01, "bpm {}", l.bpm(period));
        // Apply: shift by the slip at the reference beat, then the tempo about it.
        let mut fixed = g.clone();
        fixed.shift(l.slip);
        let pivot = fixed.beats[l.reference].time;
        fixed.set_bpm_about(pivot, l.bpm(period), 60.0);
        for (i, b) in beats.iter().enumerate().take(100) {
            assert!((fixed.beats[i].time - b).abs() < 0.003, "beat {i}: {} vs {b}", fixed.beats[i].time);
        }
        let after = fit(&grid_error(&fixed, &tr));
        assert!(after > 95.0 && after > before + 3.0, "fit {before} → {after}");
        assert_eq!(l.split, None);
    }

    #[test]
    fn a_jump_in_the_error_is_a_split() {
        let errors: Vec<BeatError> = (0..64)
            .map(|i| BeatError { beat: i, err: if i < 40 { 0.001 } else { 0.014 }, matched: true })
            .collect();
        assert_eq!(fit_line(&errors).unwrap().split, Some(40));
        assert_eq!(problems(&errors), vec![]);
        let mut bad = errors.clone();
        bad[50].err = 0.03;
        assert_eq!(problems(&bad), vec![(50, 0.03)]);
        assert_eq!(median_error(&errors[..5]), Some(0.001));
    }
}
