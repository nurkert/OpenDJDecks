//! Loudness of a recording ([27]): integrated, short-term and momentary
//! loudness after ITU-R BS.1770-4 / EBU R 128 (K-weighting, 400 ms blocks
//! with 75 % overlap, the absolute gate at −70 LUFS and the relative gate
//! 10 LU below), the true peak by 4× oversampling, and the clips (runs of
//! samples at or past digital full scale).
//!
//! The meter runs on the recorder's writer thread, block by block, so the
//! reading is there while recording and the same numbers describe the
//! finished file.

use serde::{Deserialize, Serialize};

/// What the meter says about a recording (dB values; `None` until there is
/// enough sound to say it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Loudness {
    /// Integrated loudness (LUFS) over everything so far, gated.
    pub integrated: Option<f64>,
    /// Short-term loudness (LUFS) of the last 3 s.
    pub short_term: Option<f64>,
    /// Momentary loudness (LUFS) of the last 400 ms.
    pub momentary: Option<f64>,
    /// Highest short-term loudness so far (LUFS).
    pub max_short_term: Option<f64>,
    /// True peak (dBTP) so far.
    pub true_peak: Option<f64>,
    /// Runs of samples at or past full scale (before the file clamps them).
    pub clips: u64,
}

/// The loudness of silence and anything under the absolute gate.
pub const ABSOLUTE_GATE: f64 = -70.0;
/// A true peak above this is called out (dBTP): streaming services and
/// encoders want headroom under 0 dBTP.
pub const TRUE_PEAK_LIMIT: f64 = -1.0;

#[derive(Clone, Copy, Debug, Default)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 3],
    z1: f64,
    z2: f64,
}

impl Biquad {
    #[inline]
    fn run(&mut self, x: f64) -> f64 {
        // Transposed direct form II.
        let y = self.b[0] * x + self.z1;
        self.z1 = self.b[1] * x - self.a[1] * y + self.z2;
        self.z2 = self.b[2] * x - self.a[2] * y;
        y
    }
}

/// The two K-weighting stages for a sample rate (the high shelf and the
/// RLB high-pass of BS.1770, derived for any rate).
fn k_weighting(rate: f64) -> [Biquad; 2] {
    use std::f64::consts::PI;
    let f0 = 1681.974_450_955_533;
    let g = 3.999_843_853_973_347;
    let q = 0.707_175_236_955_419_6;
    let k = (PI * f0 / rate).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.499_666_774_154_541_6);
    let a0 = 1.0 + k / q + k * k;
    let shelf = Biquad {
        b: [
            (vh + vb * k / q + k * k) / a0,
            2.0 * (k * k - vh) / a0,
            (vh - vb * k / q + k * k) / a0,
        ],
        a: [1.0, 2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
        ..Default::default()
    };
    let f0 = 38.135_470_876_024_44;
    let q = 0.500_327_037_323_877_3;
    let k = (PI * f0 / rate).tan();
    let a0 = 1.0 + k / q + k * k;
    let high_pass = Biquad {
        b: [1.0, -2.0, 1.0],
        a: [1.0, 2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
        ..Default::default()
    };
    [shelf, high_pass]
}

/// Taps per phase of the 4× true-peak interpolator.
const TAPS: usize = 12;
const PHASES: usize = 4;

/// A windowed-sinc low-pass cut at the original Nyquist frequency, split
/// into its four phases (each phase sums to about one).
fn interpolator() -> [[f64; TAPS]; PHASES] {
    let n = TAPS * PHASES;
    let mid = (n as f64 - 1.0) / 2.0;
    let mut h = vec![0.0; n];
    for (i, v) in h.iter_mut().enumerate() {
        let t = (i as f64 - mid) / PHASES as f64;
        let sinc = if t.abs() < 1e-12 {
            1.0
        } else {
            (std::f64::consts::PI * t).sin() / (std::f64::consts::PI * t)
        };
        // Blackman–Harris window.
        let w = {
            use std::f64::consts::TAU;
            let x = i as f64 / (n as f64 - 1.0);
            0.35875 - 0.48829 * (TAU * x).cos() + 0.14128 * (2.0 * TAU * x).cos()
                - 0.01168 * (3.0 * TAU * x).cos()
        };
        *v = sinc * w;
    }
    let mut out = [[0.0; TAPS]; PHASES];
    for (p, phase) in out.iter_mut().enumerate() {
        for (k, tap) in phase.iter_mut().enumerate() {
            *tap = h[p + k * PHASES];
        }
        let sum: f64 = phase.iter().sum();
        if sum.abs() > 1e-9 {
            phase.iter_mut().for_each(|t| *t /= sum);
        }
    }
    out
}

/// Streaming loudness meter for interleaved stereo.
pub struct LoudnessMeter {
    filters: [[Biquad; 2]; 2],
    /// Samples per 100 ms step.
    step: usize,
    /// Frames into the current step and the K-weighted energy summed there.
    in_step: usize,
    step_sum: f64,
    /// Mean square of each finished 100 ms step (both channels summed).
    steps: Vec<f64>,
    /// Mean square of every 400 ms block (one per step from the fourth).
    blocks: Vec<f64>,
    max_short_term: Option<f64>,
    phases: [[f64; TAPS]; PHASES],
    history: [[f64; TAPS]; 2],
    peak: f64,
    clipping: [bool; 2],
    clips: u64,
}

impl LoudnessMeter {
    pub fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(8_000) as f64;
        let k = k_weighting(rate);
        Self {
            filters: [k, k],
            step: (rate / 10.0).round() as usize,
            in_step: 0,
            step_sum: 0.0,
            steps: Vec::new(),
            blocks: Vec::new(),
            max_short_term: None,
            phases: interpolator(),
            history: [[0.0; TAPS]; 2],
            peak: 0.0,
            clipping: [false; 2],
            clips: 0,
        }
    }

    /// Feeds interleaved stereo samples.
    pub fn process(&mut self, samples: &[f32]) {
        for frame in samples.chunks_exact(2) {
            let mut energy = 0.0;
            for (c, &s) in frame.iter().enumerate() {
                let x = s as f64;
                // Clips: a run of samples at or past full scale counts once.
                let over = s.abs() >= 1.0;
                if over && !self.clipping[c] {
                    self.clips += 1;
                }
                self.clipping[c] = over;
                // True peak: the sample itself and three points between it
                // and the previous one.
                let h = &mut self.history[c];
                h.copy_within(0..TAPS - 1, 1);
                h[0] = x;
                self.peak = self.peak.max(x.abs());
                for phase in &self.phases[1..] {
                    let y: f64 = phase.iter().zip(h.iter()).map(|(t, v)| t * v).sum();
                    self.peak = self.peak.max(y.abs());
                }
                let [shelf, hp] = &mut self.filters[c];
                let y = hp.run(shelf.run(x));
                energy += y * y;
            }
            self.step_sum += energy;
            self.in_step += 1;
            if self.in_step == self.step {
                self.finish_step();
            }
        }
    }

    fn finish_step(&mut self) {
        self.steps.push(self.step_sum / self.step as f64);
        self.step_sum = 0.0;
        self.in_step = 0;
        if self.steps.len() >= 4 {
            let n = self.steps.len();
            let block = self.steps[n - 4..].iter().sum::<f64>() / 4.0;
            self.blocks.push(block);
        }
        if let Some(s) = self.window(30) {
            self.max_short_term = Some(self.max_short_term.map_or(s, |m: f64| m.max(s)));
        }
    }

    /// Loudness of the mean square of the last `steps` 100 ms steps.
    fn window(&self, steps: usize) -> Option<f64> {
        if self.steps.len() < steps {
            return None;
        }
        let n = self.steps.len();
        let ms = self.steps[n - steps..].iter().sum::<f64>() / steps as f64;
        Some(lufs(ms))
    }

    /// Gated integrated loudness.
    fn integrated(&self) -> Option<f64> {
        let above: Vec<f64> = self
            .blocks
            .iter()
            .copied()
            .filter(|b| lufs(*b) > ABSOLUTE_GATE)
            .collect();
        if above.is_empty() {
            return None;
        }
        let relative = lufs(above.iter().sum::<f64>() / above.len() as f64) - 10.0;
        let gated: Vec<f64> = above.into_iter().filter(|b| lufs(*b) > relative).collect();
        if gated.is_empty() {
            return None;
        }
        Some(lufs(gated.iter().sum::<f64>() / gated.len() as f64))
    }

    pub fn reading(&self) -> Loudness {
        Loudness {
            integrated: self.integrated(),
            short_term: self.window(30),
            momentary: self.window(4),
            max_short_term: self.max_short_term,
            true_peak: (self.peak > 0.0).then(|| 20.0 * self.peak.log10()),
            clips: self.clips,
        }
    }
}

fn lufs(mean_square: f64) -> f64 {
    if mean_square <= 0.0 {
        return -200.0;
    }
    -0.691 + 10.0 * mean_square.log10()
}

/// "−14.2 LUFS" (one decimal, a real minus sign); "— LUFS" without a value.
pub fn lufs_text(v: Option<f64>) -> String {
    match v {
        Some(v) if v > ABSOLUTE_GATE => format!("{} LUFS", signed(v)),
        _ => "— LUFS".into(),
    }
}

/// "−0.8 dBTP"; "— dBTP" without a value.
pub fn true_peak_text(v: Option<f64>) -> String {
    match v {
        Some(v) if v > -120.0 => format!("{} dBTP", signed(v)),
        _ => "— dBTP".into(),
    }
}

fn signed(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r < 0.0 {
        format!("−{:.1}", -r)
    } else if r > 0.0 {
        format!("+{r:.1}")
    } else {
        "0.0".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f64, amp: f64, secs: f64, phase: f64) -> Vec<f32> {
        let n = (rate as f64 * secs) as usize;
        let mut v = Vec::with_capacity(n * 2);
        for i in 0..n {
            let s = (amp * (std::f64::consts::TAU * freq * i as f64 / rate as f64 + phase).sin()) as f32;
            v.push(s);
            v.push(s);
        }
        v
    }

    /// EBU Tech 3341, case 1: a stereo 1 kHz sine at −23 dBFS reads
    /// −23.0 LUFS (±0.1), at 44.1 and 48 kHz.
    #[test]
    fn a_1k_sine_at_minus_23_reads_minus_23_lufs() {
        for rate in [44_100, 48_000] {
            let mut m = LoudnessMeter::new(rate);
            m.process(&sine(rate, 1000.0, 10f64.powf(-23.0 / 20.0), 20.0, 0.0));
            let r = m.reading();
            let i = r.integrated.unwrap();
            assert!((i + 23.0).abs() < 0.1, "{rate}: {i}");
            assert!((r.short_term.unwrap() + 23.0).abs() < 0.1);
            assert!((r.momentary.unwrap() + 23.0).abs() < 0.1);
            assert_eq!(r.clips, 0);
        }
    }

    /// Case 2 shape: the relative gate drops the quiet part (−36 dBFS for
    /// 10 s, −23 dBFS for 60 s, −36 dBFS for 10 s reads −23 LUFS).
    #[test]
    fn the_relative_gate_leaves_out_quiet_passages() {
        let rate = 48_000;
        let mut m = LoudnessMeter::new(rate);
        for (db, secs) in [(-36.0, 10.0), (-23.0, 60.0), (-36.0, 10.0)] {
            m.process(&sine(rate, 1000.0, 10f64.powf(db / 20.0), secs, 0.0));
        }
        let i = m.reading().integrated.unwrap();
        assert!((i + 23.0).abs() < 0.1, "{i}");
        // Silence alone is under the absolute gate.
        let mut m = LoudnessMeter::new(rate);
        m.process(&vec![0.0; rate as usize * 4]);
        assert_eq!(m.reading().integrated, None);
        assert_eq!(lufs_text(None), "— LUFS");
    }

    /// The true peak finds what lies between the samples: a sine at a
    /// quarter of the rate, 45° off the samples, peaks at 0 dBTP while no
    /// sample is above −3 dBFS.
    #[test]
    fn the_true_peak_sees_between_samples() {
        let rate = 48_000;
        let mut m = LoudnessMeter::new(rate);
        let s = sine(rate, rate as f64 / 4.0, 1.0, 1.0, std::f64::consts::FRAC_PI_4);
        let sample_peak = s.iter().fold(0.0f32, |p, v| p.max(v.abs()));
        assert!(sample_peak < 0.71);
        m.process(&s);
        let tp = m.reading().true_peak.unwrap();
        assert!(tp > -0.6 && tp < 0.3, "{tp}");
        assert_eq!(m.reading().clips, 0, "no sample reaches full scale");
    }

    #[test]
    fn clips_count_runs_past_full_scale() {
        let mut m = LoudnessMeter::new(48_000);
        // Two runs on the left, one on the right.
        let mut v = vec![0.5f32; 40];
        v[2] = 1.2;
        v[4] = 1.0;
        v[6] = 0.3;
        v[10] = -1.5;
        v[11] = 1.1;
        v[13] = 1.1;
        m.process(&v);
        assert_eq!(m.reading().clips, 3);
        assert_eq!(true_peak_text(Some(-0.84)), "−0.8 dBTP");
        assert_eq!(lufs_text(Some(-14.25)), "−14.3 LUFS");
        assert_eq!(true_peak_text(Some(0.31)), "+0.3 dBTP");
    }
}
