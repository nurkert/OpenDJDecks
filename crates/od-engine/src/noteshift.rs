//! Pitching one note to another (CREATE, roadmap [72]): a picked note,
//! with its fades, rendered at any pitch as a new buffer that starts and
//! ends cleanly. Runs off the real-time thread (it allocates); the result
//! is played as a sample.
//!
//! Two methods, as in the CREATE concept:
//!
//! - **Resampling** (the load-time polyphase resampler of [`crate::dsp`])
//!   plays the note faster or slower. The pitch is exact and the attack
//!   keeps its shape, only the note gets shorter going up and longer going
//!   down, by the same ratio.
//! - **Stretching** (the keylock stretcher of [`crate::keylock`], the same
//!   one KEY SHIFT and keylock use on the decks, with its pitch scale)
//!   keeps the length. It costs more and softens a sharp attack a little.
//!
//! [`method`] picks one: see [`SHORT_NOTE`] and [`MAX_DRIFT`].

use crate::config::Profile;
use crate::dsp::resample::resample_stereo;
use crate::keylock::{MAX_FEED, Stretcher};

/// Shifts larger than this many semitones (up or down) carry a warning:
/// they start to sound unnatural, and a source note nearer the target
/// sounds better.
pub const WARN_SEMITONES: i32 = 7;

/// A note this short (seconds) is mostly attack: it is always resampled,
/// since keeping the attack matters more than its length (a pluck an
/// octave down is 0.5 s instead of 0.25 s).
pub const SHORT_NOTE: f64 = 0.25;

/// How much (seconds) resampling may change a longer note's length. Half a
/// sixteenth at 128 BPM: a note played on a step still ends in its step.
/// A longer change is stretched instead, so the note keeps its length.
pub const MAX_DRIFT: f64 = 0.06;

/// The click guard at the start, seconds: short enough to keep the attack.
const FADE_IN: f64 = 0.002;

/// How a note was pitched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftMethod {
    /// No shift: the note as it is.
    Same,
    Resample,
    Stretch,
}

/// The method for a note of `seconds` shifted by `semitones`.
///
/// The rule: no shift is no work; a short note ([`SHORT_NOTE`]) or a shift
/// that changes the length by at most [`MAX_DRIFT`] is resampled (exact
/// pitch, attack kept); anything else is stretched to keep its length.
/// Without the stretcher compiled in, everything is resampled.
pub fn method(seconds: f64, semitones: f64) -> ShiftMethod {
    if semitones.abs() < 0.005 {
        return ShiftMethod::Same;
    }
    let drift = seconds * (2f64.powf(-semitones / 12.0) - 1.0).abs();
    if seconds <= SHORT_NOTE || drift <= MAX_DRIFT || !Stretcher::AVAILABLE {
        ShiftMethod::Resample
    } else {
        ShiftMethod::Stretch
    }
}

/// Whether a pad `semitones` from the source carries the warning.
pub fn far(semitones: i32) -> bool {
    semitones.abs() > WARN_SEMITONES
}

/// A note rendered at another pitch.
#[derive(Clone, Debug)]
pub struct ShiftedNote {
    /// Interleaved stereo at the source rate.
    pub samples: Vec<f32>,
    pub method: ShiftMethod,
}

/// Renders `note` (interleaved stereo at `rate`, with its fades) shifted
/// by `semitones`, with a clean start and a `fade` seconds tail (see
/// [`clean_edges`]).
pub fn shift_note(note: &[f32], rate: u32, semitones: f64, fade: f64) -> ShiftedNote {
    let frames = note.len() / 2;
    let seconds = frames as f64 / f64::from(rate.max(1));
    let ratio = 2f64.powf(semitones / 12.0);
    let mut m = method(seconds, semitones);
    let mut samples = match m {
        ShiftMethod::Same => note[..frames * 2].to_vec(),
        ShiftMethod::Resample => resample(note, ratio),
        ShiftMethod::Stretch => match stretch(note, rate, ratio) {
            Some(s) => s,
            None => {
                m = ShiftMethod::Resample;
                resample(note, ratio)
            }
        },
    };
    clean_edges(&mut samples, rate, fade);
    ShiftedNote { samples, method: m }
}

/// Plays the note `ratio` times faster: read at the source rate, it sounds
/// `ratio` higher and lasts `1 / ratio` as long.
fn resample(note: &[f32], ratio: f64) -> Vec<f32> {
    // The resampler takes whole rates: a fine one keeps the ratio exact to
    // well under a cent.
    const FINE: f64 = 1_048_576.0;
    let from = (ratio * FINE).round().clamp(1.0, f64::from(u32::MAX)) as u32;
    resample_stereo(note, from, FINE as u32)
}

/// Keeps the length and scales the pitch by `ratio` with the keylock
/// stretcher (its real-time mode, fed in blocks): the start pad is fed
/// first and the start delay dropped, so the output lines up with the
/// input; the end is flushed with silence. `None` without the stretcher.
fn stretch(note: &[f32], rate: u32, ratio: f64) -> Option<Vec<f32>> {
    let frames = note.len() / 2;
    let mut st = Stretcher::new(rate, Profile::Full)?;
    st.set_pitch_scale(ratio);
    let pad = st.preferred_start_pad();
    let delay = st.start_delay();
    let (l, r): (Vec<f32>, Vec<f32>) = note.chunks_exact(2).map(|f| (f[0], f[1])).unzip();
    let zeros = vec![0f32; MAX_FEED];
    let (mut bl, mut br) = (vec![0f32; MAX_FEED], vec![0f32; MAX_FEED]);
    let (mut ol, mut or) = (Vec::with_capacity(frames + delay), Vec::with_capacity(frames + delay));
    let mut drain = |st: &mut Stretcher, ol: &mut Vec<f32>, or: &mut Vec<f32>| {
        loop {
            let n = st.available().min(MAX_FEED);
            if n == 0 {
                break;
            }
            let got = st.retrieve(&mut bl[..n], &mut br[..n]);
            if got == 0 {
                break;
            }
            ol.extend_from_slice(&bl[..got]);
            or.extend_from_slice(&br[..got]);
        }
    };
    let mut left = pad;
    while left > 0 {
        let n = left.min(MAX_FEED);
        st.process(&zeros[..n], &zeros[..n]);
        drain(&mut st, &mut ol, &mut or);
        left -= n;
    }
    for (cl, cr) in l.chunks(MAX_FEED).zip(r.chunks(MAX_FEED)) {
        st.process(cl, cr);
        drain(&mut st, &mut ol, &mut or);
    }
    // Flush: silence until the whole note is out (bounded).
    let want = delay + frames;
    let mut rounds = 0;
    while ol.len() < want && rounds < 64 {
        st.process(&zeros, &zeros);
        drain(&mut st, &mut ol, &mut or);
        rounds += 1;
    }
    let mut out = vec![0f32; frames * 2];
    for i in 0..frames {
        let j = delay + i;
        if j < ol.len() {
            out[i * 2] = ol[j];
            out[i * 2 + 1] = or[j];
        }
    }
    Some(out)
}

/// A clean start and tail for interleaved stereo at `rate`: no DC offset,
/// a short raised-cosine click guard at the start (the attack stays), and
/// `fade` seconds (at most a quarter of the note) down to exactly zero at
/// the end, so whatever rings after the note is gone.
pub fn clean_edges(samples: &mut [f32], rate: u32, fade: f64) {
    let frames = samples.len() / 2;
    if frames == 0 {
        return;
    }
    let rate = f64::from(rate.max(1));
    let fin = ((FADE_IN * rate) as usize).clamp(1, (frames / 4).max(1));
    let fout = ((fade * rate) as usize).clamp(1, (frames / 4).max(1));
    let cosine = |x: f32| 0.5 - 0.5 * (std::f32::consts::PI * x).cos();
    let gain = |i: usize| {
        let mut g = 1.0;
        if i < fin {
            g *= cosine(i as f32 / fin as f32);
        }
        let left = frames - 1 - i;
        if left < fout {
            g *= cosine(left as f32 / fout as f32);
        }
        g
    };
    // The offset taken out is weighted by the fades, so the faded note
    // itself has none.
    let total: f64 = (0..frames).map(|i| f64::from(gain(i))).sum::<f64>().max(1e-9);
    for c in 0..2 {
        let dc = (0..frames).map(|i| f64::from(gain(i)) * f64::from(samples[i * 2 + c])).sum::<f64>() / total;
        for i in 0..frames {
            samples[i * 2 + c] = (samples[i * 2 + c] - dc as f32) * gain(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rule_resamples_short_notes_and_small_drifts() {
        assert_eq!(method(0.5, 0.0), ShiftMethod::Same);
        assert_eq!(method(0.2, 12.0), ShiftMethod::Resample, "short: all attack");
        assert_eq!(method(0.2, -12.0), ShiftMethod::Resample);
        // Half a second: a semitone or two moves the end by under 60 ms.
        assert_eq!(method(0.5, 2.0), ShiftMethod::Resample);
        assert_eq!(method(0.5, -1.0), ShiftMethod::Resample);
        let long = if Stretcher::AVAILABLE { ShiftMethod::Stretch } else { ShiftMethod::Resample };
        assert_eq!(method(0.5, 3.0), long);
        assert_eq!(method(1.5, -3.0), long);
        assert_eq!(method(2.0, 0.5), ShiftMethod::Resample, "a quarter tone: 29 ms");
    }

    #[test]
    fn warning_beyond_seven_semitones() {
        assert!(!far(7) && !far(-7) && !far(0));
        assert!(far(8) && far(-8) && far(12));
    }

    #[test]
    fn edges_start_and_end_at_zero_without_dc() {
        let mut x = vec![0.3f32; 2 * 1000];
        clean_edges(&mut x, 1000, 0.1);
        assert_eq!(x.iter().copied().fold(0f32, |a, b| a.max(b.abs())), 0.0, "a constant is all DC");
        let mut x: Vec<f32> = (0..1000).flat_map(|i| {
            let v = 0.2 + (i as f32 * 0.3).sin();
            [v, v]
        }).collect();
        clean_edges(&mut x, 1000, 0.1);
        assert_eq!((x[0], x[1998], x[1999]), (0.0, 0.0, 0.0));
        let mean = x.iter().step_by(2).sum::<f32>() / 1000.0;
        assert!(mean.abs() < 1e-4, "no DC: {mean}");
    }
}
