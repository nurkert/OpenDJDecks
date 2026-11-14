//! Channel FX: one bipolar knob per channel (−1..=1, 0 = bypass).
//!
//! Echo, dub echo and space feed their wet path with the knob amount, so
//! their tails ring out after the knob returns to the centre; the channel
//! is bit-exact dry again once the tail is silent.
//!
//! The filter (also the dry path of Auto) is a mixer-style colour filter:
//! a centre dead zone that stays bit-exact dry, a log-frequency sweep with
//! more travel towards the extremes, and a 4-pole (24 dB/oct) low-pass or
//! high-pass whose resonance rises gently towards the ends.

use crate::tempofx::BeatClock;
use crate::command::ChannelFxKind;
use crate::dsp::Noise;
use crate::dsp::delay::DelayLine;
use crate::dsp::pitchshift::PitchShifter;
use crate::dsp::reverb::Reverb;
use crate::dsp::smooth::Smoothed;
use crate::dsp::svf::Svf;
use std::f32::consts::PI;

const UPDATE_EVERY: usize = 16;
const DEAD_ZONE: f32 = 0.01;
/// Wet energy per frame below which a tail counts as silent.
const QUIET: f32 = 1e-10;
/// Colour filter: knob travel around the centre that stays dry.
const FILTER_DEAD: f32 = 0.04;
/// Knob travel past the dead zone over which the filter fades in.
const FILTER_FADE: f32 = 0.06;
/// Sweep shape: >1 leaves more knob travel for the musical low end.
const FILTER_CURVE: f32 = 1.6;
const FILTER_LP_END_HZ: f32 = 70.0;
const FILTER_HP_END_HZ: f32 = 6_000.0;

pub struct ChannelFx {
    sr: f32,
    kind: ChannelFxKind,
    amount: Smoothed,
    svf: Svf,
    /// Two cascaded 2-pole stages of the colour filter.
    colour: [Svf; 2],
    noise_svf: Svf,
    noise: Noise,
    reverb: Reverb,
    delay: DelayLine,
    shifter: PitchShifter,
    crush_hold: [f32; 2],
    crush_phase: f32,
    gate: f32,
    /// Feedback amount held after the knob returns (dub echo).
    fb_hold: f32,
    /// Tail still ringing.
    ringing: bool,
    quiet_frames: f32,
    counter: usize,
    /// Panic: fade to dry over 5 ms (gain 1 → 0), then clear the tails.
    kill: bool,
    kill_gain: f32,
}

impl ChannelFx {
    /// Panic: the knob to the centre and its tail silenced (5 ms fade).
    pub fn panic(&mut self) {
        self.amount.set(0.0);
        if !self.bypassed() {
            self.kill = true;
        }
    }

    pub fn new(sample_rate: u32, seed: u32) -> Self {
        let sr = sample_rate as f32;
        Self {
            sr,
            kind: ChannelFxKind::Filter,
            amount: Smoothed::new(0.0, (sr * 0.02) as u32),
            svf: Svf::new(1000.0, 0.7, sr),
            colour: [Svf::new(1000.0, 0.54, sr); 2],
            noise_svf: Svf::new(1000.0, 0.7, sr),
            noise: Noise::new(seed),
            reverb: Reverb::new(sample_rate),
            delay: DelayLine::new((sr * 2.5) as usize),
            shifter: PitchShifter::new((sr * 0.04) as usize, sr),
            crush_hold: [0.0; 2],
            crush_phase: 0.0,
            gate: 1.0,
            fb_hold: 0.0,
            ringing: false,
            quiet_frames: 0.0,
            counter: 0,
            kill: false,
            kill_gain: 1.0,
        }
    }

    pub fn set_kind(&mut self, kind: ChannelFxKind) {
        if kind != self.kind {
            self.kind = kind;
            self.svf.reset();
            self.colour[0].reset();
            self.colour[1].reset();
            self.noise_svf.reset();
            if self.ringing {
                self.reverb.clear();
                self.delay.clear();
                self.ringing = false;
            }
            self.gate = 1.0;
            self.fb_hold = 0.0;
        }
    }

    pub fn kind(&self) -> ChannelFxKind {
        self.kind
    }

    pub fn set_amount(&mut self, a: f32) {
        self.amount.set(a.clamp(-1.0, 1.0));
    }

    pub fn amount(&self) -> f32 {
        self.amount.target()
    }

    fn has_tail(kind: ChannelFxKind) -> bool {
        matches!(
            kind,
            ChannelFxKind::Echo | ChannelFxKind::DubEcho | ChannelFxKind::Space | ChannelFxKind::Auto
        )
    }

    fn bypassed(&self) -> bool {
        self.amount.is_settled() && self.amount.current().abs() < DEAD_ZONE && !self.ringing
    }

    /// Filter cutoff for a knob value: negative sweeps a low-pass from 20 kHz
    /// down to 60 Hz, positive a high-pass from 20 Hz up to 10 kHz.
    fn cutoff(a: f32) -> f32 {
        if a < 0.0 {
            20_000.0 * (60.0f32 / 20_000.0).powf(-a)
        } else {
            20.0 * (10_000.0f32 / 20.0).powf(a)
        }
    }

    /// Colour filter cutoff and sweep position (0..=1) for a knob value. The
    /// sweep starts at the edge of the audible band (20 kHz low-pass, 20 Hz
    /// high-pass), so fading in past the dead zone causes no level jump.
    fn colour_cutoff(a: f32) -> (f32, f32) {
        let s = ((a.abs() - FILTER_DEAD) / (1.0 - FILTER_DEAD))
            .clamp(0.0, 1.0)
            .powf(FILTER_CURVE);
        let fc = if a < 0.0 {
            20_000.0 * (FILTER_LP_END_HZ / 20_000.0).powf(s)
        } else {
            20.0 * (FILTER_HP_END_HZ / 20.0).powf(s)
        };
        (fc, s)
    }

    /// One frame through the colour filter. Coefficients follow the smoothed
    /// knob every frame, so fast turns do not zipper; both channels share them.
    fn colour_filter(&mut self, x: [f32; 2], a: f32) -> [f32; 2] {
        let (fc, s) = Self::colour_cutoff(a);
        let g = (PI * fc.min(self.sr * 0.49) / self.sr).tan();
        // Butterworth-like stage pair (Q 0.54 and ~1.3), the second stage
        // rising from 0.9 to 1.4 towards the extremes: a gentle edge.
        self.colour[0].set_g(g, 0.54);
        self.colour[1].set_g(g, 0.9 + 0.5 * s);
        let w = ((a.abs() - FILTER_DEAD) / FILTER_FADE).clamp(0.0, 1.0);
        let lp = a < 0.0;
        let mut y = x;
        for ch in 0..2 {
            // Run in the dead zone too, so the state is warm on the way out.
            let o = self.colour[0].tick(ch, x[ch]);
            let o = self.colour[1].tick(ch, if lp { o.low } else { o.high });
            let filtered = if lp { o.low } else { o.high };
            if w > 0.0 {
                y[ch] = x[ch] + (filtered - x[ch]) * w;
            }
        }
        y
    }

    fn update_filters(&mut self, a: f32) {
        let mag = a.abs();
        match self.kind {
            ChannelFxKind::Sweep if a > 0.0 => {
                // Band-pass rising 250 Hz → 8 kHz.
                let fc = 250.0 * 32f32.powf(mag);
                self.svf.set(fc, 1.2 + 3.0 * mag, self.sr);
            }
            ChannelFxKind::Sweep => self.svf.set(120.0 + 400.0 * mag, 0.7, self.sr),
            ChannelFxKind::DubEcho => {
                // Feedback filter: dark on the left, thin on the right.
                let fc = if a < 0.0 {
                    3000.0 * (0.15f32).powf(mag)
                } else {
                    150.0 * 8f32.powf(mag)
                };
                self.svf.set(fc, 0.9, self.sr);
            }
            _ => {
                let q = 0.707 + 0.9 * mag;
                self.svf.set(Self::cutoff(a), q, self.sr);
                self.noise_svf.set(Self::cutoff(a), q, self.sr);
            }
        }
    }

    /// Processes interleaved stereo in place, synced to `clock` (the
    /// channel's deck or the master clock).
    pub fn process(&mut self, buf: &mut [f32], clock: &BeatClock) {
        if self.bypassed() {
            // Cold start for the colour filter: no stale state on re-entry.
            self.colour[0].reset();
            self.colour[1].reset();
            if self.kind == ChannelFxKind::Pitch {
                // Keep the grain history continuous for a click-free start.
                for f in buf.as_chunks::<2>().0 {
                    self.shifter.push(*f);
                }
            }
            return;
        }
        let beat_frames = clock.beat_frames();
        let frames = buf.len() / 2;
        let release = (-1.0 / (self.sr * 1.5)).exp();
        let mut wet_energy = 0.0f32;
        for (i, f) in buf.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let a = self.amount.next();
            if self.counter == 0 {
                self.update_filters(a);
            }
            self.counter = (self.counter + 1) % UPDATE_EVERY;
            let mag = a.abs();
            // Blend in over the first 5 % so the centre is seamless.
            let w = (mag / 0.05).min(1.0);
            let lp = a < 0.0;
            let x = [f[0], f[1]];
            let y = match self.kind {
                ChannelFxKind::Filter => self.colour_filter(x, a),
                ChannelFxKind::Noise => {
                    let n = self.noise.next() * mag * 0.25;
                    let mut y = [0.0; 2];
                    for ch in 0..2 {
                        let o = self.noise_svf.tick(ch, n);
                        let nf = if lp { o.low } else { o.high };
                        y[ch] = x[ch] + nf;
                    }
                    y
                }
                ChannelFxKind::Space => {
                    self.reverb.room = 0.8 + 0.17 * mag;
                    let wet = self.reverb.process([x[0] * mag * 2.0, x[1] * mag * 2.0]);
                    let mut y = [0.0; 2];
                    for ch in 0..2 {
                        let o = self.svf.tick(ch, wet[ch]);
                        let wf = if a < 0.0 { o.low } else { o.high };
                        y[ch] = x[ch] + wf;
                        wet_energy += wf * wf;
                    }
                    y
                }
                ChannelFxKind::Crush => {
                    let factor = 1.0 + 24.0 * mag;
                    self.crush_phase += 1.0;
                    if self.crush_phase >= factor {
                        self.crush_phase -= factor;
                        self.crush_hold = x;
                    }
                    let levels = 2f32.powf(16.0 - 13.0 * mag);
                    let mut y = [0.0; 2];
                    for ch in 0..2 {
                        let q = (self.crush_hold[ch] * levels).round() / levels;
                        let o = self.svf.tick(ch, q);
                        // Mild tone: low-pass side darkens, high-pass side thins.
                        let t = if lp { o.low } else { q };
                        y[ch] = x[ch] + (t - x[ch]) * (mag / 0.1).min(1.0);
                    }
                    y
                }
                ChannelFxKind::Echo => {
                    let d = (beat_frames * 0.75).clamp(1.0, self.delay.capacity() as f32);
                    let wet = self.delay.read(d);
                    self.fb_hold = (self.fb_hold * release).max(mag);
                    let fb = 0.3 + 0.5 * self.fb_hold;
                    let mut fw = [0.0; 2];
                    for ch in 0..2 {
                        let o = self.svf.tick(ch, wet[ch]);
                        fw[ch] = if a < 0.0 { o.low } else { o.high };
                        wet_energy += fw[ch] * fw[ch];
                    }
                    self.delay
                        .push([x[0] * mag + fw[0] * fb, x[1] * mag + fw[1] * fb]);
                    [x[0] + fw[0], x[1] + fw[1]]
                }
                ChannelFxKind::DubEcho => {
                    let d = (beat_frames * 0.5).clamp(1.0, self.delay.capacity() as f32);
                    let wet = self.delay.read(d);
                    self.fb_hold = (self.fb_hold * release).max(mag);
                    let fb = 0.6 + 0.32 * self.fb_hold;
                    let mut fw = [0.0; 2];
                    for ch in 0..2 {
                        let o = self.svf.tick(ch, wet[ch]);
                        let filtered = if a < 0.0 { o.low } else { o.high };
                        // Tape-like saturation keeps the feedback bounded.
                        fw[ch] = filtered.tanh();
                        wet_energy += fw[ch] * fw[ch];
                    }
                    self.delay
                        .push([x[0] * mag + fw[0] * fb, x[1] * mag + fw[1] * fb]);
                    [x[0] + fw[0], x[1] + fw[1]]
                }
                ChannelFxKind::Auto => {
                    // Filter the dry signal as Filter does …
                    let dry = self.colour_filter(x, a);
                    // … and add an echo (left) or a reverb (right).
                    let send = mag * mag;
                    let wet = if lp {
                        let d = (beat_frames * 0.75).clamp(1.0, self.delay.capacity() as f32);
                        let wet = self.delay.read(d);
                        self.fb_hold = (self.fb_hold * release).max(mag);
                        let fb = 0.25 + 0.45 * self.fb_hold;
                        self.delay
                            .push([dry[0] * send + wet[0] * fb, dry[1] * send + wet[1] * fb]);
                        wet
                    } else {
                        self.reverb.room = 0.75 + 0.2 * mag;
                        self.reverb
                            .process([dry[0] * send * 1.5, dry[1] * send * 1.5])
                    };
                    wet_energy += wet[0] * wet[0] + wet[1] * wet[1];
                    [dry[0] + wet[0], dry[1] + wet[1]]
                }
                ChannelFxKind::Pitch => {
                    let y = self.shifter.process(x, 2f32.powf(a));
                    [x[0] + (y[0] - x[0]) * w, x[1] + (y[1] - x[1]) * w]
                }
                ChannelFxKind::Sweep => {
                    if a < 0.0 {
                        // 1/16 gate locked to the beat, depth = knob.
                        let beat = clock.beat + i as f64 * clock.beats_per_frame;
                        let ph = (beat * 4.0).rem_euclid(1.0);
                        let target = if ph < 0.5 { 1.0 } else { 1.0 - mag };
                        self.gate += (target - self.gate) * (1.0 / (self.sr * 0.001)).min(1.0);
                        let mut y = [0.0; 2];
                        for ch in 0..2 {
                            // Slight high-pass tightens the gated sound.
                            let o = self.svf.tick(ch, x[ch]);
                            let thin = x[ch] + (o.high - x[ch]) * w;
                            y[ch] = thin * self.gate;
                        }
                        y
                    } else {
                        self.gate = 1.0;
                        let mut y = [0.0; 2];
                        for ch in 0..2 {
                            let o = self.svf.tick(ch, x[ch]);
                            y[ch] = x[ch] + (o.band * 1.5 - x[ch]) * w;
                        }
                        y
                    }
                }
            };
            let y = if self.kill {
                let k = self.kill_gain;
                self.kill_gain = (k - 1.0 / (self.sr * 0.005)).max(0.0);
                [x[0] + (y[0] - x[0]) * k, x[1] + (y[1] - x[1]) * k]
            } else {
                y
            };
            f[0] = y[0];
            f[1] = y[1];
        }
        if self.kill && self.kill_gain <= 0.0 {
            self.kill = false;
            self.kill_gain = 1.0;
            self.amount.reset(0.0);
            self.reverb.clear();
            self.delay.clear();
            self.fb_hold = 0.0;
            self.ringing = false;
            self.quiet_frames = 0.0;
            return;
        }
        // Tails: keep processing until the wet path is silent.
        if Self::has_tail(self.kind) {
            let centred = self.amount.is_settled() && self.amount.current().abs() < DEAD_ZONE;
            if !centred {
                self.ringing = true;
                self.quiet_frames = 0.0;
            } else if wet_energy / frames.max(1) as f32 <= QUIET {
                self.quiet_frames += frames as f32;
                if self.quiet_frames > beat_frames + 4096.0 {
                    self.ringing = false;
                    self.quiet_frames = 0.0;
                }
            } else {
                self.quiet_frames = 0.0;
            }
        } else {
            self.ringing = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;
    const BLOCK: usize = 256;

    fn clock() -> BeatClock {
        BeatClock { bpm: 120.0, beat: 0.0, beats_per_frame: 2.0 / SR as f64 }
    }

    fn sine(freq: f32, frames: usize, phase0: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let t = (i + phase0) as f32 / SR as f32;
                let v = 0.5 * (std::f32::consts::TAU * freq * t).sin();
                [v, v]
            })
            .collect()
    }

    fn filter_at(amount: f32) -> ChannelFx {
        let mut fx = ChannelFx::new(SR, 1);
        fx.set_kind(ChannelFxKind::Filter);
        fx.set_amount(amount);
        fx
    }

    /// Level change in dB of a settled tone through the filter at `amount`.
    fn gain_db(amount: f32, freq: f32) -> f32 {
        let mut fx = filter_at(amount);
        let frames = SR as usize;
        let mut buf = sine(freq, frames, 0);
        for b in buf.chunks_mut(BLOCK * 2) {
            fx.process(b, &clock());
        }
        let tail = &buf[frames..];
        let rms = (tail.iter().map(|v| v * v).sum::<f32>() / tail.len() as f32).sqrt();
        20.0 * (rms / (0.5 / 2f32.sqrt())).log10()
    }

    #[test]
    fn filter_dead_zone_is_bit_exact_dry() {
        for amount in [-0.03f32, 0.03, 0.0] {
            let mut fx = filter_at(amount);
            let dry = sine(440.0, SR as usize / 2, 0);
            let mut buf = dry.clone();
            for b in buf.chunks_mut(BLOCK * 2) {
                fx.process(b, &clock());
            }
            assert!(buf == dry, "{amount} dry");
        }
    }

    #[test]
    fn filter_low_pass_and_high_pass_sides() {
        let lp_hi = gain_db(-0.9, 3000.0);
        let lp_lo = gain_db(-0.9, 80.0);
        assert!(lp_hi < -30.0, "low-pass takes out 3 kHz: {lp_hi} dB");
        assert!(lp_lo.abs() < 3.0, "low-pass keeps 80 Hz: {lp_lo} dB");
        let hp_lo = gain_db(0.9, 80.0);
        let hp_hi = gain_db(0.9, 3000.0);
        assert!(hp_lo < -30.0, "high-pass takes out 80 Hz: {hp_lo} dB");
        assert!(hp_hi.abs() < 3.0, "high-pass keeps 3 kHz: {hp_hi} dB");
    }

    #[test]
    fn filter_fast_sweeps_are_smooth_and_stereo_identical() {
        let mut fx = filter_at(0.0);
        // 150 Hz at 0.5: the dry signal moves at most ~0.01 per sample.
        let frames = SR as usize;
        let mut buf = sine(150.0, frames, 0);
        let dry_step = buf
            .chunks(2)
            .zip(buf.chunks(2).skip(1))
            .map(|(a, b)| (b[0] - a[0]).abs())
            .fold(0.0f32, f32::max);
        // Knob slammed back and forth in MIDI-sized steps, 32 frames apart.
        for (i, b) in buf.chunks_mut(64).enumerate() {
            let p = (i % 64) as f32 / 32.0;
            let tri = if p < 1.0 { p } else { 2.0 - p };
            fx.set_amount(tri * 2.0 - 1.0);
            fx.process(b, &clock());
        }
        let mut max_step = 0.0f32;
        for (a, b) in buf.chunks(2).zip(buf.chunks(2).skip(1)) {
            assert_eq!(a[0], a[1], "stereo identical");
            max_step = max_step.max((b[0] - a[0]).abs());
        }
        assert!(
            max_step < dry_step * 2.5,
            "no zipper or jump: {max_step} vs dry {dry_step}"
        );
    }
}
