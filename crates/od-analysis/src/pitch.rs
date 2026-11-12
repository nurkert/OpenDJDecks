//! Pitch of one picked note (CREATE, roadmap [71]).
//!
//! [`detect_pitch`] estimates the fundamental of a short monophonic region
//! with YIN (de Cheveigné & Kawahara 2002): per frame, the cumulative mean
//! normalised difference function over the lags of 30 Hz to 2 kHz, the
//! first dip below [`YIN_THRESHOLD`] (else the deepest one), refined by
//! parabolic interpolation. The difference function is computed from an FFT
//! cross-correlation, so long lags at the full sample rate stay cheap.
//!
//! The region's frames are gated by loudness (a decaying tail does not
//! vote once it is quiet), and the note is the median of the periodic
//! frames in cents. The confidence (0..1) falls with aperiodicity (noise),
//! with frames that found no pitch and with frames that disagree (a slide,
//! two notes). A region that is too short or silent has no pitch.

use realfft::RealFftPlanner;
use realfft::num_complex::Complex;

/// Lowest and highest fundamental searched, Hz.
pub const MIN_HZ: f64 = 30.0;
pub const MAX_HZ: f64 = 2000.0;
/// Confidence from which a pitch counts as detected (below: unsure).
pub const SURE: f32 = 0.7;
/// Shortest region measured, seconds.
pub const MIN_SECONDS: f64 = 0.06;
/// Region RMS under which it counts as silent (−50 dBFS).
const SILENCE_RMS: f64 = 0.003_16;
/// YIN's absolute threshold on the normalised difference.
const YIN_THRESHOLD: f64 = 0.15;
/// A frame whose best dip is above this found no pitch.
const APERIODIC: f64 = 0.4;
/// Frames this close to the median (cents) agree on the note.
const AGREE_CENTS: f64 = 30.0;
/// Most frames measured in one region.
const MAX_FRAMES: usize = 48;

/// What came of a measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PitchOutcome {
    /// A pitch was found (sure or not: see [`PitchEstimate::confidence`]).
    Pitch,
    /// Periodic frames were too few: noise, or many notes at once.
    NoPitch,
    /// Shorter than [`MIN_SECONDS`].
    TooShort,
    /// Quieter than −50 dBFS.
    Silent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchEstimate {
    pub outcome: PitchOutcome,
    /// The fundamental, Hz (`None` unless [`PitchOutcome::Pitch`]).
    pub hz: Option<f64>,
    /// 0..1; [`SURE`] and above counts as detected.
    pub confidence: f32,
}

impl PitchEstimate {
    fn none(outcome: PitchOutcome) -> Self {
        Self { outcome, hz: None, confidence: 0.0 }
    }

    /// A pitch the detector stands by.
    pub fn sure(&self) -> bool {
        self.hz.is_some() && self.confidence >= SURE
    }
}

/// One frame's result: the period in samples and how far from periodic.
struct Frame {
    period: f64,
    aperiodicity: f64,
}

/// The pitch of a mono region at `sample_rate`.
pub fn detect_pitch(samples: &[f32], sample_rate: u32) -> PitchEstimate {
    let fs = f64::from(sample_rate);
    if sample_rate == 0 || (samples.len() as f64) < fs * MIN_SECONDS {
        return PitchEstimate::none(PitchOutcome::TooShort);
    }
    let mean = samples.iter().map(|&s| f64::from(s)).sum::<f64>() / samples.len() as f64;
    let x: Vec<f64> = samples.iter().map(|&s| f64::from(s) - mean).collect();
    let rms = (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
    if rms < SILENCE_RMS {
        return PitchEstimate::none(PitchOutcome::Silent);
    }
    // Lags: the longest period, but no more than half the region (a short
    // region cannot hold a low note twice).
    let tau_min = ((fs / MAX_HZ).floor() as usize).max(2);
    let tau_max = ((fs / MIN_HZ).ceil() as usize).min(x.len() / 2);
    if tau_max <= tau_min + 2 {
        return PitchEstimate::none(PitchOutcome::TooShort);
    }
    let w = tau_max;
    let len = w + tau_max;
    let hop = (w / 2).max((x.len() - len) / MAX_FRAMES).max(1);
    let starts: Vec<usize> = (0..=(x.len() - len) / hop).map(|k| k * hop).collect();
    // The loudness gate: a frame votes while it is within 20 dB of the
    // loudest one and above silence.
    let frame_rms: Vec<f64> = starts
        .iter()
        .map(|&s| (x[s..s + w].iter().map(|v| v * v).sum::<f64>() / w as f64).sqrt())
        .collect();
    let loudest = frame_rms.iter().copied().fold(0.0, f64::max);
    let gate = (loudest * 0.1).max(SILENCE_RMS);
    let n = len.next_power_of_two();
    let mut planner = RealFftPlanner::<f64>::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);
    let mut frames = Vec::new();
    let mut heard = 0usize;
    for (&s, &r) in starts.iter().zip(&frame_rms) {
        if r < gate {
            continue;
        }
        heard += 1;
        let seg = &x[s..s + len];
        frames.push(yin_frame(seg, w, tau_min, tau_max, &*fwd, &*inv));
    }
    let voiced: Vec<&Frame> = frames.iter().filter(|f| f.aperiodicity < APERIODIC).collect();
    if heard == 0 || voiced.is_empty() {
        return PitchEstimate::none(PitchOutcome::NoPitch);
    }
    // The note: the median in cents (robust against a stray octave).
    let mut cents: Vec<f64> = voiced.iter().map(|f| 1200.0 * (fs / f.period / 440.0).log2()).collect();
    cents.sort_by(f64::total_cmp);
    let median = median_sorted(&cents);
    // Frames within a third of a semitone of it agree; the others (a
    // slide, another note, an octave slip) count against the note.
    let agree = cents.iter().filter(|c| (*c - median).abs() <= AGREE_CENTS).count();
    let mut ap: Vec<f64> = voiced.iter().map(|f| f.aperiodicity).collect();
    ap.sort_by(f64::total_cmp);
    let periodic = (1.0 - median_sorted(&ap) / APERIODIC).clamp(0.0, 1.0);
    let share = agree as f64 / heard as f64;
    let confidence = (periodic.sqrt() * share) as f32;
    PitchEstimate {
        outcome: PitchOutcome::Pitch,
        hz: Some(440.0 * 2f64.powf(median / 1200.0)),
        confidence,
    }
}

fn median_sorted(v: &[f64]) -> f64 {
    let m = v.len() / 2;
    if v.len() % 2 == 1 { v[m] } else { (v[m - 1] + v[m]) / 2.0 }
}

/// YIN on one frame `seg` (integration window `w`, then `tau_max` more).
fn yin_frame(
    seg: &[f64],
    w: usize,
    tau_min: usize,
    tau_max: usize,
    fwd: &dyn realfft::RealToComplex<f64>,
    inv: &dyn realfft::ComplexToReal<f64>,
) -> Frame {
    let n = fwd.len();
    // r(τ) = Σ_{j<w} x_j · x_{j+τ}: the window against the whole frame.
    let mut a = vec![0.0; n];
    a[..w].copy_from_slice(&seg[..w]);
    let mut b = vec![0.0; n];
    b[..seg.len()].copy_from_slice(seg);
    let mut fa = fwd.make_output_vec();
    let mut fb = fwd.make_output_vec();
    fwd.process(&mut a, &mut fa).expect("fft size");
    fwd.process(&mut b, &mut fb).expect("fft size");
    let mut prod: Vec<Complex<f64>> = fa.iter().zip(&fb).map(|(p, q)| p.conj() * q).collect();
    // The inverse needs real DC and Nyquist bins.
    prod[0].im = 0.0;
    if let Some(last) = prod.last_mut() {
        last.im = 0.0;
    }
    let mut r = vec![0.0; n];
    inv.process(&mut prod, &mut r).expect("fft size");
    let scale = 1.0 / n as f64;
    // Energies of the window at each lag, from a prefix sum.
    let mut prefix = vec![0.0; seg.len() + 1];
    for (i, v) in seg.iter().enumerate() {
        prefix[i + 1] = prefix[i] + v * v;
    }
    let e0 = prefix[w];
    // The cumulative mean normalised difference d'(τ).
    let mut cmnd = vec![1.0; tau_max + 1];
    let mut sum = 0.0;
    for tau in 1..=tau_max {
        let et = prefix[tau + w] - prefix[tau];
        let d = (e0 + et - 2.0 * r[tau] * scale).max(0.0);
        sum += d;
        cmnd[tau] = if sum > 0.0 { d * tau as f64 / sum } else { 1.0 };
    }
    // The first dip under the threshold, down to its bottom; else the deepest.
    let mut tau = (tau_min..=tau_max).find(|&t| cmnd[t] < YIN_THRESHOLD);
    if let Some(t) = tau.as_mut() {
        while *t < tau_max && cmnd[*t + 1] < cmnd[*t] {
            *t += 1;
        }
    }
    let t = tau.unwrap_or_else(|| {
        (tau_min..=tau_max)
            .min_by(|&p, &q| cmnd[p].total_cmp(&cmnd[q]))
            .unwrap_or(tau_min)
    });
    // Parabolic interpolation between the neighbours.
    let period = if t > 1 && t < tau_max {
        let (y0, y1, y2) = (cmnd[t - 1], cmnd[t], cmnd[t + 1]);
        let den = y0 - 2.0 * y1 + y2;
        if den.abs() > 1e-12 { t as f64 + 0.5 * (y0 - y2) / den } else { t as f64 }
    } else {
        t as f64
    };
    Frame { period, aperiodicity: cmnd[t] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsig::Noise;
    use std::f64::consts::TAU;

    /// MIDI note `m` + `cents` in Hz.
    fn hz(m: f64, cents: f64) -> f64 {
        440.0 * 2f64.powf((m - 69.0 + cents / 100.0) / 12.0)
    }

    fn cents_off(est: f64, want: f64) -> f64 {
        1200.0 * (est / want).log2()
    }

    /// `harmonics` partials at amplitudes 1/k, under `env(t)`.
    fn tone(fs: u32, f: f64, secs: f64, harmonics: usize, env: impl Fn(f64) -> f64) -> Vec<f32> {
        let n = (f64::from(fs) * secs) as usize;
        (0..n)
            .map(|i| {
                let t = i as f64 / f64::from(fs);
                let s: f64 = (1..=harmonics)
                    .filter(|k| f * *k as f64 * 2.0 < f64::from(fs))
                    .map(|k| (TAU * f * k as f64 * t + 0.3 * k as f64).sin() / k as f64)
                    .sum();
                (0.4 * s * env(t)) as f32
            })
            .collect()
    }

    /// C2 to C6 in steps of a fifth and a bit, some off the note.
    fn notes() -> Vec<(f64, f64)> {
        let mut v = Vec::new();
        for (i, m) in (36..=84).step_by(5).chain([84]).enumerate() {
            v.push((m as f64, [0.0, 23.0, -37.0, 11.0][i % 4]));
        }
        v
    }

    fn check(name: &str, harmonics: usize, env: impl Fn(f64) -> f64 + Copy, tol: f64) -> f64 {
        let mut worst: f64 = 0.0;
        for fs in [44_100, 48_000] {
            for (m, c) in notes() {
                let want = hz(m, c);
                let x = tone(fs, want, 0.5, harmonics, env);
                let e = detect_pitch(&x, fs);
                let got = e.hz.unwrap_or_else(|| panic!("{name} {m}+{c} @ {fs}: {e:?}"));
                let off = cents_off(got, want);
                assert!(off.abs() <= tol, "{name} {m}+{c}ct @ {fs}: {off:.2} ct off");
                assert!(e.sure(), "{name} {m}+{c}ct @ {fs}: sure ({})", e.confidence);
                worst = worst.max(off.abs());
            }
        }
        worst
    }

    #[test]
    fn sines_from_c2_to_c6_within_two_cents() {
        let worst = check("sine", 1, |_| 1.0, 2.0);
        eprintln!("sine: worst {worst:.2} ct");
    }

    #[test]
    fn harmonic_rich_tones_keep_their_octave() {
        let worst = check("saw", 12, |_| 1.0, 3.0);
        eprintln!("harmonic: worst {worst:.2} ct");
    }

    #[test]
    fn a_decaying_tail_does_not_pull_the_note() {
        // A pluck: 5 ms attack, then down 60 dB over the half second.
        let pluck = |t: f64| (t / 0.005).min(1.0) * (-t * 13.8).exp();
        let worst = check("pluck", 8, pluck, 3.0);
        eprintln!("decaying: worst {worst:.2} ct");
    }

    #[test]
    fn a_detuned_note_reads_its_cents() {
        let x = tone(48_000, hz(57.0, 12.0), 0.4, 6, |_| 1.0);
        let p = od_core::NotePitch::from_hz(detect_pitch(&x, 48_000).hz.unwrap()).unwrap();
        assert_eq!(p.to_string(), "A3 +12 ct");
    }

    #[test]
    fn noise_is_unsure() {
        let mut rng = Noise::new(7);
        let x: Vec<f32> = (0..24_000).map(|_| (rng.next() * 0.3) as f32).collect();
        let e = detect_pitch(&x, 48_000);
        assert!(!e.sure(), "{e:?}");
        assert!(e.confidence < 0.3, "{e:?}");
    }

    #[test]
    fn a_tone_under_loud_noise_is_unsure() {
        let mut rng = Noise::new(3);
        let mut x = tone(48_000, 220.0, 0.5, 1, |_| 0.2);
        x.iter_mut().for_each(|v| *v += (rng.next() * 0.5) as f32);
        assert!(!detect_pitch(&x, 48_000).sure());
    }

    #[test]
    fn silence_and_short_regions_have_no_pitch() {
        let e = detect_pitch(&vec![0.0; 48_000], 48_000);
        assert_eq!((e.outcome, e.hz, e.sure()), (PitchOutcome::Silent, None, false));
        let quiet = tone(48_000, 220.0, 0.5, 1, |_| 0.002);
        assert_eq!(detect_pitch(&quiet, 48_000).outcome, PitchOutcome::Silent);
        let short = tone(48_000, 220.0, 0.04, 1, |_| 1.0);
        let e = detect_pitch(&short, 48_000);
        assert_eq!((e.outcome, e.sure()), (PitchOutcome::TooShort, false));
        assert_eq!(detect_pitch(&[], 48_000).outcome, PitchOutcome::TooShort);
        // Just long enough: a mid note still reads.
        let e = detect_pitch(&tone(48_000, 440.0, MIN_SECONDS + 0.005, 1, |_| 1.0), 48_000);
        assert!(e.sure() && cents_off(e.hz.unwrap(), 440.0).abs() < 3.0, "{e:?}");
    }

    #[test]
    fn two_notes_in_a_row_are_unsure() {
        // Half A3, half E4: the frames disagree.
        let mut x = tone(48_000, 220.0, 0.3, 4, |_| 1.0);
        x.extend(tone(48_000, 329.63, 0.3, 4, |_| 1.0));
        assert!(!detect_pitch(&x, 48_000).sure());
    }
}
