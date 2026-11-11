//! Beat and energy from an audio input (roadmap [36]): line-in or a
//! microphone, so AUTO can follow music that does not come from the decks
//! (a band, another DJ's mixer, a playlist on another computer).
//!
//! [`BeatFollower`] works on mono samples in 10 ms steps:
//!
//! * the level of each step (RMS) gives the energy, in decibels mapped to
//!   0..1, and silence (below [`SILENCE_DB`] for half a second);
//! * the rise in level from step to step is an onset curve; every half
//!   second its autocorrelation over the last six seconds picks a tempo
//!   between [`MIN_BPM`] and [`MAX_BPM`] (leaning towards 120 BPM, so a
//!   tempo and its double are told apart the same way each time);
//! * a comb over the onsets at that tempo finds where the beats fall, and a
//!   running beat counter is pulled gently towards them.
//!
//! It is meant for lights, not for beat matching: it settles in a few
//! seconds and holds its tempo through short breaks.

use crate::show::DeckMusic;

/// Steps per second.
const RATE: usize = 100;
/// Seconds of onsets the tempo is estimated from.
const WINDOW_SECS: usize = 6;
/// Re-estimate every this many steps.
const ESTIMATE_EVERY: usize = 50;
pub const MIN_BPM: f64 = 70.0;
pub const MAX_BPM: f64 = 180.0;
/// Below this level (dBFS) a step counts as silent.
pub const SILENCE_DB: f32 = -50.0;
/// Silent this long, the input counts as not playing.
pub const SILENCE_SECS: f32 = 0.5;
/// Level mapped to energy 0 and 1, dBFS.
const ENERGY_FLOOR_DB: f32 = -45.0;
const ENERGY_TOP_DB: f32 = -6.0;

#[derive(Clone, Debug)]
pub struct BeatFollower {
    rate: u32,
    hop: usize,
    acc: f64,
    acc_n: usize,
    last_db: f32,
    onsets: Vec<f32>,
    steps: usize,
    bpm: f64,
    /// Confidence of the tempo, 0..1.
    pub confidence: f32,
    beat: f64,
    /// Energy, slowly smoothed, 0..1, from the mean power.
    energy: f32,
    power: f32,
    silent: f32,
    /// Level of the last step, dBFS.
    pub level_db: f32,
}

impl BeatFollower {
    pub fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(1000);
        Self {
            rate,
            hop: (rate as usize / RATE).max(1),
            acc: 0.0,
            acc_n: 0,
            last_db: -100.0,
            onsets: Vec::with_capacity(RATE * WINDOW_SECS),
            steps: 0,
            bpm: 120.0,
            confidence: 0.0,
            beat: 0.0,
            energy: 0.0,
            power: 0.0,
            silent: SILENCE_SECS,
            level_db: -100.0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Feeds interleaved samples of `channels` channels.
    pub fn feed_interleaved(&mut self, samples: &[f32], channels: usize) {
        let ch = channels.max(1);
        for frame in samples.chunks(ch) {
            let m = frame.iter().sum::<f32>() / frame.len() as f32;
            self.push(m);
        }
    }

    /// Feeds mono samples.
    pub fn feed(&mut self, samples: &[f32]) {
        for s in samples {
            self.push(*s);
        }
    }

    fn push(&mut self, s: f32) {
        let s = if s.is_finite() { s } else { 0.0 };
        self.acc += (s * s) as f64;
        self.acc_n += 1;
        if self.acc_n >= self.hop {
            let rms = (self.acc / self.acc_n as f64).sqrt() as f32;
            self.acc = 0.0;
            self.acc_n = 0;
            self.step(rms);
        }
    }

    fn step(&mut self, rms: f32) {
        let dt = 1.0 / RATE as f32;
        let db = 20.0 * rms.max(1e-6).log10();
        self.level_db = db;
        let onset = (db - self.last_db).max(0.0).min(30.0);
        self.last_db = db;
        if db < SILENCE_DB {
            self.silent += dt;
        } else {
            self.silent = 0.0;
        }
        // Loudness over about two seconds (mean power), mapped to 0..1.
        self.power += (rms * rms - self.power) * (dt / 2.0).min(1.0);
        let loud = 10.0 * self.power.max(1e-12).log10();
        self.energy = ((loud - ENERGY_FLOOR_DB) / (ENERGY_TOP_DB - ENERGY_FLOOR_DB)).clamp(0.0, 1.0);
        if self.onsets.len() == RATE * WINDOW_SECS {
            self.onsets.remove(0);
        }
        self.onsets.push(if db < SILENCE_DB { 0.0 } else { onset });
        self.steps += 1;
        self.beat += self.bpm / 60.0 / RATE as f64;
        if self.steps % ESTIMATE_EVERY == 0 && self.onsets.len() >= RATE * 3 {
            self.estimate();
        }
    }

    /// Tempo by autocorrelation, then the beat phase by a comb.
    fn estimate(&mut self) {
        let n = self.onsets.len();
        let mean = self.onsets.iter().sum::<f32>() / n as f32;
        let x: Vec<f32> = self.onsets.iter().map(|v| v - mean).collect();
        let energy: f32 = x.iter().map(|v| v * v).sum();
        if energy <= 1e-6 {
            return;
        }
        let lag_min = (60.0 * RATE as f64 / MAX_BPM).floor() as usize;
        let lag_max = (60.0 * RATE as f64 / MIN_BPM).ceil() as usize;
        let ac = |lag: usize| -> f32 {
            if lag >= n {
                return 0.0;
            }
            x[lag..].iter().zip(&x[..n - lag]).map(|(a, b)| a * b).sum::<f32>()
                / (n - lag) as f32
        };
        let mut best = (0usize, f32::MIN);
        let mut raw = vec![0.0f32; lag_max + 2];
        for (lag, r) in raw.iter_mut().enumerate().take(lag_max + 2).skip(lag_min.saturating_sub(1)) {
            *r = ac(lag);
        }
        for lag in lag_min..=lag_max {
            let bpm = 60.0 * RATE as f64 / lag as f64;
            // Lean towards 120 BPM (a log-normal weight, wide).
            let w = (-(bpm / 120.0).log2().powi(2) / (2.0 * 0.6f64.powi(2))).exp() as f32;
            let score = raw[lag] * (0.6 + 0.4 * w);
            if score > best.1 {
                best = (lag, score);
            }
        }
        let lag = best.0;
        if lag == 0 || raw[lag] <= 0.0 {
            self.confidence = 0.0;
            return;
        }
        // Parabolic refinement of the peak.
        let (a, b, c) = (raw[lag - 1], raw[lag], raw[lag + 1]);
        let denom = a - 2.0 * b + c;
        let shift = if denom.abs() > 1e-9 {
            (0.5 * (a - c) / denom).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let period = lag as f64 + shift as f64;
        self.confidence = (raw[lag] / (energy / n as f32)).clamp(0.0, 1.0);
        if self.confidence < 0.1 {
            return;
        }
        self.bpm = 60.0 * RATE as f64 / period;
        // Phase: the offset (steps back from now) at which onsets one period
        // apart add up most.
        let p = period.round().max(1.0) as usize;
        let mut best_off = (0usize, f32::MIN);
        for off in 0..p {
            let mut sum = 0.0;
            let mut k = 0;
            while off + k * p < n {
                let i = n - 1 - (off + (k as f64 * period).round() as usize).min(n - 1);
                // Spread a little: the onset step and the ones next to it.
                sum += x[i] + 0.5 * x.get(i + 1).copied().unwrap_or(0.0)
                    + 0.5 * if i > 0 { x[i - 1] } else { 0.0 };
                k += 1;
            }
            if sum > best_off.1 {
                best_off = (off, sum);
            }
        }
        // The last beat fell `off` steps ago: the beat counter's fraction
        // should be off / period now. Pull halfway towards it.
        let want = best_off.0 as f64 / period;
        let have = self.beat.fract();
        let mut err = want - have;
        if err > 0.5 {
            err -= 1.0;
        } else if err < -0.5 {
            err += 1.0;
        }
        self.beat += err * 0.5;
        if self.beat < 0.0 {
            self.beat += 1.0;
        }
    }

    /// Tempo found, BPM.
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Running beat (whole numbers fall on the beats).
    pub fn beat(&self) -> f64 {
        self.beat
    }

    /// Smoothed energy, 0..1.
    pub fn energy(&self) -> f32 {
        self.energy
    }

    /// Seconds of silence so far (0 while there is sound).
    pub fn silent_for(&self) -> f32 {
        self.silent
    }

    pub fn is_silent(&self) -> bool {
        self.silent >= SILENCE_SECS
    }

    /// What the show follows: playing while there is sound, at the tempo and
    /// beat found, with the input's energy. Phrases are not detected.
    pub fn music(&self) -> DeckMusic {
        DeckMusic {
            playing: !self.is_silent(),
            bpm: self.bpm,
            beat: self.beat,
            energy: self.energy,
            weight: if self.is_silent() { 0.0 } else { 1.0 },
            hue: 0.6,
            ..Default::default()
        }
    }
}

/// Generated test audio: a kick (a falling sine with a click) on every beat
/// over quiet noise, mono.
#[doc(hidden)]
pub fn click_track(bpm: f64, secs: f32, rate: u32, offset_secs: f32) -> Vec<f32> {
    let n = (secs * rate as f32) as usize;
    let period = 60.0 / bpm;
    let mut seed: u32 = 0x1234_5678;
    (0..n)
        .map(|i| {
            let t = i as f64 / rate as f64;
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = ((seed >> 9) as f32 / (1u32 << 23) as f32 - 0.5) * 0.02;
            let since = (t - offset_secs as f64).rem_euclid(period);
            let kick = if t >= offset_secs as f64 && since < 0.15 {
                let env = (-since * 30.0).exp();
                let f = 50.0 + 100.0 * (-since * 40.0).exp();
                (env * (2.0 * std::f64::consts::PI * f * since).sin()) as f32 * 0.8
            } else {
                0.0
            };
            kick + noise
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn follow(bpm: f64, secs: f32, offset: f32) -> BeatFollower {
        let rate = 44_100;
        let audio = click_track(bpm, secs, rate, offset);
        let mut f = BeatFollower::new(rate);
        for chunk in audio.chunks(512) {
            f.feed(chunk);
        }
        f
    }

    #[test]
    fn finds_the_tempo_of_generated_kicks() {
        for bpm in [96.0, 124.0, 128.0, 140.0] {
            let f = follow(bpm, 12.0, 0.0);
            assert!((f.bpm() - bpm).abs() < 2.0, "{bpm}: found {}", f.bpm());
            assert!(f.music().playing);
        }
    }

    #[test]
    fn the_beat_lands_on_the_kicks() {
        // Kicks start 0.23 s in; after 12 s the counter's whole beats sit
        // on them.
        let bpm = 126.0;
        let rate = 44_100;
        let offset = 0.23;
        let audio = click_track(bpm, 12.0, rate, offset);
        let mut f = BeatFollower::new(rate);
        f.feed(&audio);
        let t = 12.0 - offset as f64;
        let beats_since_kick = (t * bpm / 60.0).fract();
        let mut err = (f.beat().fract() - beats_since_kick).abs();
        err = err.min(1.0 - err);
        assert!(err < 0.15, "phase off by {err} of a beat");
    }

    #[test]
    fn silence_stops_and_level_sets_energy() {
        let rate = 44_100;
        let mut f = BeatFollower::new(rate);
        f.feed(&click_track(128.0, 6.0, rate, 0.0));
        assert!(f.music().playing);
        let loud = f.energy();
        assert!(loud > 0.3, "energy {loud}");
        f.feed(&vec![0.0; rate as usize]);
        assert!(!f.music().playing, "a second of silence");
        assert!(f.silent_for() >= 0.9);
        let quiet: Vec<f32> = click_track(128.0, 6.0, rate, 0.0).iter().map(|s| s * 0.05).collect();
        let mut g = BeatFollower::new(rate);
        g.feed(&quiet);
        assert!(g.energy() < loud, "{} vs {loud}", g.energy());
    }

    #[test]
    fn stereo_is_mixed_down() {
        let rate = 48_000;
        let mono = click_track(128.0, 10.0, rate, 0.0);
        let stereo: Vec<f32> = mono.iter().flat_map(|s| [*s, *s]).collect();
        let mut f = BeatFollower::new(rate);
        f.feed_interleaved(&stereo, 2);
        assert!((f.bpm() - 128.0).abs() < 2.0, "{}", f.bpm());
    }
}
