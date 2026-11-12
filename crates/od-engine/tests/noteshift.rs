//! [72] Pitching a picked note: the target pitch as the detector of [71]
//! reads it, a clean start and tail, and the method per the rule.

use od_analysis::pitch::detect_pitch;
use od_engine::keylock::Stretcher;
use od_engine::noteshift::{ShiftMethod, clean_edges, method, shift_note};

const RATE: u32 = 48_000;
const A3: f64 = 220.0;

/// A plucked-ish A3: three harmonics, a hard attack and an exponential
/// decay, `secs` long, with the fades a picked note has (click guard, a
/// quarter of the note at most 60 ms at OUT).
fn note(secs: f64, decay: f64) -> (Vec<f32>, f64) {
    let n = (secs * f64::from(RATE)) as usize;
    let mut x: Vec<f32> = (0..n)
        .flat_map(|i| {
            let t = i as f64 / f64::from(RATE);
            let w = std::f64::consts::TAU * A3 * t;
            let v = (0.5 * w.sin() + 0.25 * (2.0 * w).sin() + 0.12 * (3.0 * w).sin()) * (-t / decay).exp();
            [v as f32, v as f32]
        })
        .collect();
    let fade = (secs * 0.25).clamp(0.005, 0.06);
    clean_edges(&mut x, RATE, fade);
    (x, fade)
}

fn mono(x: &[f32]) -> Vec<f32> {
    x.chunks_exact(2).map(|f| (f[0] + f[1]) * 0.5).collect()
}

fn cents(hz: f64, want: f64) -> f64 {
    1200.0 * (hz / want).log2()
}

/// First frame above a tenth of the peak.
fn onset(x: &[f32]) -> usize {
    let m = mono(x);
    let peak = m.iter().fold(0f32, |a, &b| a.max(b.abs()));
    m.iter().position(|v| v.abs() > peak * 0.1).unwrap_or(0)
}

/// Checks pitch, start and tail of `out` as `semitones` from A3.
fn check(out: &[f32], semitones: f64, label: &str) -> f64 {
    let want = A3 * 2f64.powf(semitones / 12.0);
    let e = detect_pitch(&mono(out), RATE);
    let hz = e.hz.unwrap_or_else(|| panic!("{label}: no pitch ({:?})", e.outcome));
    let c = cents(hz, want);
    assert!(c.abs() < 5.0, "{label}: {hz:.2} Hz for {want:.2} Hz, {c:+.2} ct");
    // A clean start: from zero, no step steeper than the note's own.
    assert_eq!((out[0], out[1]), (0.0, 0.0), "{label}: starts at zero");
    let m = mono(out);
    let guard = (0.002 * f64::from(RATE)) as usize;
    let slope = |s: &[f32]| s.windows(2).fold(0f32, |a, w| a.max((w[1] - w[0]).abs()));
    let start = slope(&m[..guard + 1]);
    let body = slope(&m[guard..]);
    assert!(start <= body * 1.05 + 1e-6, "{label}: a click at the start ({start} > {body})");
    // A clean tail: down to zero, nothing ringing on.
    let n = m.len();
    assert_eq!(*m.last().unwrap(), 0.0, "{label}: ends at zero");
    let tail = m[n - 48..].iter().fold(0f32, |a, &b| a.max(b.abs()));
    assert!(tail < 0.02, "{label}: the last millisecond is quiet ({tail})");
    let mean = m.iter().sum::<f32>() / n as f32;
    assert!(mean.abs() < 1e-3, "{label}: no DC ({mean})");
    c
}

const SHIFTS: [f64; 6] = [-12.0, -7.0, -3.0, 3.0, 7.0, 12.0];

/// A short note is resampled at every shift: the pitch lands within a few
/// cents, the attack stays where it was, the length follows the ratio.
#[test]
fn short_notes_are_resampled_to_the_target() {
    let (x, fade) = note(0.2, 0.15);
    let frames = x.len() / 2;
    for s in SHIFTS {
        let out = shift_note(&x, RATE, s, fade);
        assert_eq!(out.method, ShiftMethod::Resample, "{s:+}: resampled");
        let c = check(&out.samples, s, &format!("resample {s:+}"));
        let want = (frames as f64 / 2f64.powf(s / 12.0)).round() as usize;
        assert!((out.samples.len() / 2).abs_diff(want) <= 1, "{s:+}: length by the ratio");
        let r = 2f64.powf(s / 12.0);
        // Within the click guard (2 ms) of where the ratio puts it.
        let (a, b) = (onset(&x) as f64 / r, onset(&out.samples) as f64);
        assert!(b <= a + 0.002 * f64::from(RATE), "{s:+}: the attack stays at the start ({b} for {a:.0})");
        eprintln!("resample {s:+}: {c:+.2} ct");
    }
}

/// A long note keeps its length through the keylock stretcher: the pitch
/// lands within a few cents and the note starts where it did.
#[test]
fn long_notes_are_stretched_and_keep_their_length() {
    if !Stretcher::AVAILABLE {
        return;
    }
    let (x, fade) = note(1.5, 1.2);
    let frames = x.len() / 2;
    for s in SHIFTS {
        assert_eq!(method(1.5, s), ShiftMethod::Stretch);
        let out = shift_note(&x, RATE, s, fade);
        assert_eq!(out.method, ShiftMethod::Stretch, "{s:+}: stretched");
        assert_eq!(out.samples.len() / 2, frames, "{s:+}: same length");
        let c = check(&out.samples, s, &format!("stretch {s:+}"));
        // Lined up: the start delay is gone, the attack is where it was.
        let (a, b) = (onset(&x), onset(&out.samples));
        assert!(b.abs_diff(a) < (0.01 * f64::from(RATE)) as usize, "{s:+}: onset {b} for {a}");
        // And the body is there to the end (not cut short by the delay).
        let m = mono(&out.samples);
        let late = m[frames * 3 / 4..frames * 7 / 8].iter().fold(0f32, |acc, &v| acc.max(v.abs()));
        assert!(late > 0.05, "{s:+}: the note lasts ({late})");
        eprintln!("stretch {s:+}: {c:+.2} ct");
    }
}

/// No shift: the note as picked, edges cleaned.
#[test]
fn no_shift_is_the_note_itself() {
    let (x, fade) = note(0.5, 0.4);
    let out = shift_note(&x, RATE, 0.0, fade);
    assert_eq!(out.method, ShiftMethod::Same);
    assert_eq!(out.samples.len(), x.len());
    check(&out.samples, 0.0, "same");
}
