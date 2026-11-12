//! Roadmap [79]: the built-in drum sounds start at once, stay in range,
//! ring out, follow PITCH, DECAY and TONE, follow the sample rate, and
//! render bit for bit alike on every platform.

use od_engine::drums::{self, KITS, Params, Voice};

const RATE: u32 = 48_000;

fn peak(s: &[f32]) -> f32 {
    s.iter().fold(0.0f32, |m, x| m.max(x.abs()))
}

/// Zero crossings per second.
fn crossings(s: &[f32], rate: u32) -> f32 {
    let n = s.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    n as f32 * rate as f32 / s.len() as f32
}

/// Where the sound last rises over -40 dB of its peak, seconds.
fn ring(s: &[f32], rate: u32) -> f32 {
    let floor = peak(s) * 0.01;
    s.iter().rposition(|x| x.abs() > floor).unwrap_or(0) as f32 / rate as f32
}

/// FNV-1a over the 16-bit samples.
fn hash(samples: &[i16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for s in samples {
        for b in s.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

#[test]
fn every_voice_sounds_stays_in_range_and_ends_on_silence() {
    for kit in KITS {
        for (v, p) in Voice::ALL.into_iter().zip(kit.pads) {
            let s = drums::render(v, p, RATE);
            assert_eq!(s.len(), drums::frames(v, p, RATE), "{} {}", kit.id, v.id());
            let pk = peak(&s);
            assert!(pk > 0.3, "{} {} sounds ({pk})", kit.id, v.id());
            assert!(pk <= 0.9 + 1e-6, "{} {} under full scale ({pk})", kit.id, v.id());
            assert!(s.iter().all(|x| x.is_finite()));
            // It starts at once: the loudest part is in the first 30 ms
            // (the clap's bursts) or the first fifth.
            let at = s.iter().position(|x| x.abs() >= pk * 0.5).unwrap();
            assert!(at < (RATE as usize * 30 / 1000).max(s.len() / 5), "{} {} starts at once ({at})", kit.id, v.id());
            // It rings out: the last 10 ms are quiet, the last sample silent.
            let tail = &s[s.len() - RATE as usize / 100..];
            assert!(peak(tail) < pk * 0.05, "{} {} rings out ({})", kit.id, v.id(), peak(tail));
            assert_eq!(*s.last().unwrap(), 0.0);
        }
    }
    // The extremes stay in range too.
    for v in Voice::ALL {
        for p in [Params { pitch: -12, decay: 0, tone: 0 }, Params { pitch: 12, decay: 100, tone: 100 }] {
            let s = drums::render(v, p, RATE);
            assert!(peak(&s) > 0.3 && peak(&s) <= 0.9 + 1e-6, "{} at {p:?}", v.id());
            assert_eq!(*s.last().unwrap(), 0.0);
        }
    }
}

#[test]
fn pitch_decay_and_tone_change_the_sound_the_way_they_say() {
    let mid = Params::default();
    for v in Voice::ALL {
        let low = drums::render(v, Params { pitch: -7, ..mid }, RATE);
        let high = drums::render(v, Params { pitch: 7, ..mid }, RATE);
        assert_ne!(low, high, "{}: PITCH changes it", v.id());
        // Higher up, it crosses zero more often.
        assert!(crossings(&high, RATE) > crossings(&low, RATE) * 1.02, "{}: higher PITCH, higher pitch ({} vs {})", v.id(), crossings(&high, RATE), crossings(&low, RATE));
        let short = drums::render(v, Params { decay: 20, ..mid }, RATE);
        let long = drums::render(v, Params { decay: 80, ..mid }, RATE);
        assert!(long.len() > short.len() * 2, "{}: longer DECAY, longer sound", v.id());
        assert!(ring(&long, RATE) > ring(&short, RATE) * 1.5, "{}: and a longer tail", v.id());
        let dark = drums::render(v, Params { tone: 0, ..mid }, RATE);
        let bright = drums::render(v, Params { tone: 100, ..mid }, RATE);
        assert_ne!(dark, bright, "{}: TONE changes it", v.id());
        assert!(crossings(&bright, RATE) > crossings(&dark, RATE), "{}: brighter TONE, more highs", v.id());
    }
}

#[test]
fn lengths_follow_the_sample_rate() {
    for kit in KITS {
        for (v, p) in Voice::ALL.into_iter().zip(kit.pads) {
            let a = drums::render(v, p, 44_100).len() as f64;
            let b = drums::render(v, p, 48_000).len() as f64;
            assert!((a / b - 44_100.0 / 48_000.0).abs() < 0.002, "{} {}: {a} vs {b}", kit.id, v.id());
            // The same pitch at both rates.
            // (After the click at the start, whose noise follows the rate.)
            let body = |rate: u32| {
                let s = drums::render(v, p, rate);
                crossings(&s[rate as usize * 30 / 1000..], rate)
            };
            let (ca, cb) = (body(44_100), body(48_000));
            if matches!(v, Voice::Kick | Voice::Tom1 | Voice::Tom2) {
                assert!((ca / cb - 1.0).abs() < 0.03, "{} {}: {ca} vs {cb}", kit.id, v.id());
            }
        }
    }
}

/// The cross-platform proof: each kit at 48 kHz, as 16-bit samples, hashes
/// to the pinned value on every platform CI runs on.
#[test]
fn every_kit_renders_bit_for_bit_alike() {
    let pinned = [("tight", 3_896_515_965_657_404_735u64), ("warm", 12_352_533_416_259_377_728), ("dusty", 15_739_562_963_780_203_800)];
    let mut got = Vec::new();
    for kit in KITS {
        let mut all = Vec::new();
        for (v, p) in Voice::ALL.into_iter().zip(kit.pads) {
            all.extend(drums::to_i16(&drums::render(v, p, RATE)));
        }
        // Twice the same in one run.
        let again: Vec<i16> =
            Voice::ALL.into_iter().zip(kit.pads).flat_map(|(v, p)| drums::to_i16(&drums::render(v, p, RATE))).collect();
        assert_eq!(all, again, "{}", kit.id);
        got.push((kit.id, hash(&all)));
    }
    assert_eq!(got, pinned, "the kits' hashes");
}

#[test]
fn kits_are_found_by_id() {
    assert_eq!(KITS.map(|k| k.id), ["tight", "warm", "dusty"]);
    assert_eq!(drums::kit("warm").map(|k| k.id), Some("warm"));
    assert!(drums::kit("nope").is_none());
    for k in KITS {
        assert!(k.pads.iter().all(|p| p.clamped() == *p), "{}: in range", k.id);
    }
}
