//! Mixer channel strip and fader/crossfader curves.

use crate::tempofx::BeatClock;
use crate::channelfx::ChannelFx;
use crate::command::{ChannelCommand, CrossfaderCurve, FaderCurve, SwapAt, XfAssign};
use crate::dsp::db_to_gain;
use crate::dsp::eq::Eq3;
use crate::dsp::meter::Meter;
use crate::dsp::smooth::Smoothed;
use crate::state::ChannelState;

/// Channel fader curve: fader position 0..=1 → linear gain.
pub fn fader_gain(x: f32, curve: FaderCurve) -> f32 {
    let x = x.clamp(0.0, 1.0);
    match curve {
        FaderCurve::Linear => x,
        FaderCurve::Smooth => x * x,
        FaderCurve::Sharp => {
            let y = (x * 4.0).min(1.0);
            1.0 - (1.0 - y) * (1.0 - y)
        }
    }
}

/// Crossfader gains `(side A, side B)` for a crossfader position −1..=1.
pub fn crossfader_gains(x: f32, curve: CrossfaderCurve) -> (f32, f32) {
    let p = ((x.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
    match curve {
        CrossfaderCurve::ConstantPower => {
            let a = p * std::f32::consts::FRAC_PI_2;
            (a.cos(), a.sin())
        }
        CrossfaderCurve::Linear => (1.0 - p, p),
        CrossfaderCurve::Cut => {
            const CUT: f32 = 0.04;
            (((1.0 - p) / CUT).min(1.0), (p / CUT).min(1.0))
        }
    }
}

/// The master-clock beat an EQ swap requested at `beat` happens on: the
/// next whole beat, or the next downbeat when `bar_position` (1..=4, the
/// position of the current beat in its bar) is known. `None` for
/// [`SwapAt::Now`].
pub fn swap_beat(beat: f64, bar_position: Option<u8>, at: SwapAt) -> Option<f64> {
    let whole = beat.floor();
    match at {
        SwapAt::Now => None,
        SwapAt::Beat => Some(whole + 1.0),
        SwapAt::Bar => {
            let pos = bar_position.unwrap_or(1).clamp(1, 4);
            Some(whole + (5 - pos) as f64)
        }
    }
}

/// Level knob 0..=1 → gain (square law, 1.0 = 0 dB).
pub fn level_gain(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.5);
    x * x
}

pub(crate) struct Channel {
    pub trim_db: f32,
    pub auto_gain: bool,
    pub eq: Eq3,
    pub eq_knobs: [f32; 3],
    pub color: ChannelFx,
    pub fader: f32,
    pub fader_curve: FaderCurve,
    pub xf: XfAssign,
    pub cue: bool,
    /// Pre-fader gain (trim + auto gain).
    pub pre_gain: Smoothed,
    /// Post-fader gain to the master bus (fader × crossfader).
    pub post_gain: Smoothed,
    pub meter: Meter,
    /// Smoothed RMS contributed to the master (audible-track logic).
    pub contribution: f32,
    pub gain_to_master: f32,
}

impl Channel {
    pub fn new(sample_rate: u32, seed: u32) -> Self {
        let ramp = (sample_rate as f32 * 0.01) as u32;
        Self {
            trim_db: 0.0,
            auto_gain: true,
            eq: Eq3::new(sample_rate),
            eq_knobs: [0.0; 3],
            color: ChannelFx::new(sample_rate, seed),
            fader: 1.0,
            fader_curve: FaderCurve::default(),
            xf: XfAssign::Thru,
            cue: false,
            pre_gain: Smoothed::new(1.0, ramp),
            post_gain: Smoothed::new(1.0, ramp),
            meter: Meter::new(sample_rate),
            contribution: 0.0,
            gain_to_master: 1.0,
        }
    }

    pub fn apply(&mut self, cmd: ChannelCommand) {
        match cmd {
            ChannelCommand::SetTrim(db) => self.trim_db = db.clamp(-60.0, 12.0),
            ChannelCommand::SetAutoGain(on) => self.auto_gain = on,
            ChannelCommand::SetEq { band, value } => {
                if band < 3 {
                    self.eq_knobs[band] = value.clamp(-1.0, 1.0);
                    self.eq.set_band(band, value);
                }
            }
            ChannelCommand::SetEqMode(m) => self.eq.set_mode(m),
            ChannelCommand::SetEqKill { band, on } => self.eq.set_kill(band, on),
            ChannelCommand::SetChannelFx(k) => self.color.set_kind(k),
            ChannelCommand::SetChannelFxAmount(a) => self.color.set_amount(a),
            ChannelCommand::SetFader(v) => self.fader = v.clamp(0.0, 1.0),
            ChannelCommand::SetFaderCurve(c) => self.fader_curve = c,
            ChannelCommand::SetCrossfaderAssign(x) => self.xf = x,
            ChannelCommand::SetCue(on) => self.cue = on,
        }
    }

    /// Updates gain targets for this block.
    pub fn update_gains(&mut self, auto_gain_db: f32, xf: (f32, f32)) {
        let db = self.trim_db + if self.auto_gain { auto_gain_db } else { 0.0 };
        self.pre_gain.set(if self.trim_db <= -60.0 {
            0.0
        } else {
            db_to_gain(db)
        });
        let x = match self.xf {
            XfAssign::A => xf.0,
            XfAssign::B => xf.1,
            XfAssign::Thru => 1.0,
        };
        self.post_gain
            .set(fader_gain(self.fader, self.fader_curve) * x);
    }

    /// Trim → EQ → channel FX, in place; then meters (pre-fader).
    pub fn process_pre(&mut self, buf: &mut [f32], clock: &BeatClock) {
        for f in buf.as_chunks_mut::<2>().0 {
            let g = self.pre_gain.next();
            f[0] *= g;
            f[1] *= g;
        }
        if !self.eq.is_flat() {
            self.eq.process(buf);
        }
        self.color.process(buf, clock);
        self.meter.process(buf);
    }

    /// The knob and kill of EQ band `band`.
    pub fn eq_band(&self, band: usize) -> (f32, bool) {
        (self.eq_knobs[band], self.eq.kills()[band])
    }

    /// Sets the knob and kill of EQ band `band` (one step of a swap).
    pub fn set_eq_band(&mut self, band: usize, (knob, kill): (f32, bool)) {
        if band < 3 {
            self.eq_knobs[band] = knob;
            self.eq.set_band(band, knob);
            self.eq.set_kill(band, kill);
        }
    }

    pub fn state(&self) -> ChannelState {
        ChannelState {
            meter: self.meter.reading(),
            fader: self.fader,
            trim_db: self.trim_db,
            auto_gain: self.auto_gain,
            eq: self.eq_knobs,
            eq_kill: self.eq.kills(),
            channel_fx: self.color.kind(),
            channel_fx_amount: self.color.amount(),
            xf_assign: self.xf,
            cue: self.cue,
            hold_fx: Default::default(),
            gain_to_master: self.gain_to_master,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossfader_curves() {
        let (a, b) = crossfader_gains(-1.0, CrossfaderCurve::ConstantPower);
        assert!((a - 1.0).abs() < 1e-6 && b.abs() < 1e-6);
        let (a, b) = crossfader_gains(0.0, CrossfaderCurve::ConstantPower);
        assert!((a * a + b * b - 1.0).abs() < 1e-5, "constant power");
        let (a, b) = crossfader_gains(0.0, CrossfaderCurve::Linear);
        assert!((a - 0.5).abs() < 1e-6 && (b - 0.5).abs() < 1e-6);
        let (a, b) = crossfader_gains(0.0, CrossfaderCurve::Cut);
        assert_eq!((a, b), (1.0, 1.0));
        let (a, b) = crossfader_gains(-0.95, CrossfaderCurve::Cut);
        assert_eq!(a, 1.0);
        assert!(b < 1.0);
        let (_, b) = crossfader_gains(-1.0, CrossfaderCurve::Cut);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn fader_curves_monotonic() {
        for c in [FaderCurve::Linear, FaderCurve::Smooth, FaderCurve::Sharp] {
            let mut last = -1.0;
            for i in 0..=100 {
                let g = fader_gain(i as f32 / 100.0, c);
                assert!(g >= last);
                last = g;
            }
            assert_eq!(fader_gain(0.0, c), 0.0);
            assert!((fader_gain(1.0, c) - 1.0).abs() < 1e-6);
        }
        assert!(fader_gain(0.25, FaderCurve::Sharp) > 0.99);
    }

    #[test]
    fn swap_lands_on_the_next_beat_or_downbeat() {
        assert_eq!(swap_beat(10.3, Some(2), SwapAt::Now), None);
        assert_eq!(swap_beat(10.3, Some(2), SwapAt::Beat), Some(11.0));
        // Beat 10 is the 2 of its bar: the next 1 is three beats on.
        assert_eq!(swap_beat(10.3, Some(2), SwapAt::Bar), Some(13.0));
        assert_eq!(swap_beat(10.0, Some(4), SwapAt::Bar), Some(11.0));
        assert_eq!(swap_beat(8.9, Some(1), SwapAt::Bar), Some(12.0));
        // Without a grid position the bar counts from the current beat.
        assert_eq!(swap_beat(3.5, None, SwapAt::Bar), Some(7.0));
    }

    #[test]
    fn eq_band_round_trips_knob_and_kill() {
        let mut a = Channel::new(48_000, 1);
        let mut b = Channel::new(48_000, 2);
        a.apply(ChannelCommand::SetEq { band: 0, value: -1.0 });
        b.apply(ChannelCommand::SetEqKill { band: 0, on: true });
        let (x, y) = (a.eq_band(0), b.eq_band(0));
        a.set_eq_band(0, y);
        b.set_eq_band(0, x);
        assert_eq!(a.state().eq[0], 0.0);
        assert_eq!(a.state().eq_kill, [true, false, false]);
        assert_eq!(b.state().eq[0], -1.0);
        assert_eq!(b.state().eq_kill, [false; 3]);
    }
}
