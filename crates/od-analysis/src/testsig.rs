//! Deterministic synthetic test signals (test builds only).

use std::f64::consts::PI;

/// Tiny deterministic noise source (LCG).
pub struct Noise(u64);

impl Noise {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1))
    }
    pub fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    }
}

/// Synthetic kick: 150 → 45 Hz pitch sweep with exponential decay.
pub fn kick(fs: u32, amp: f64) -> Vec<f32> {
    let n = (fs as f64 * 0.25) as usize;
    let mut phase = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f64 / fs as f64;
            let f = 45.0 + 105.0 * (-t / 0.03).exp();
            phase += 2.0 * PI * f / fs as f64;
            (amp * phase.sin() * (-t / 0.08).exp()) as f32
        })
        .collect()
}

/// Short noise burst (click / hat / snare depending on length).
pub fn burst(fs: u32, amp: f64, decay: f64, seed: u64) -> Vec<f32> {
    let n = (fs as f64 * decay * 6.0) as usize;
    let mut noise = Noise::new(seed);
    let mut prev = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f64 / fs as f64;
            let w = noise.next();
            let hp = w - prev; // crude high-pass for brightness
            prev = w;
            (amp * hp * 0.7 * (-t / decay).exp()) as f32
        })
        .collect()
}

pub fn add(buf: &mut [f32], at: usize, sound: &[f32]) {
    for (i, &s) in sound.iter().enumerate() {
        if let Some(b) = buf.get_mut(at + i) {
            *b += s;
        }
    }
}

fn beat_times(fs: u32, bpm: f64, offset: f64, secs: f64) -> Vec<(usize, f64)> {
    let period = 60.0 / bpm;
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        let t = offset + i as f64 * period;
        if t > secs - 0.3 {
            break;
        }
        let idx = (t * fs as f64).round() as usize;
        out.push((idx, idx as f64 / fs as f64));
        i += 1;
    }
    out
}

/// Click on every beat. Returns the signal and the exact onset times.
pub fn click_track(fs: u32, bpm: f64, offset: f64, secs: f64) -> (Vec<f32>, Vec<f64>) {
    let mut x = vec![0f32; (fs as f64 * secs) as usize];
    let click = burst(fs, 0.9, 0.004, 7);
    let beats = beat_times(fs, bpm, offset, secs);
    for &(i, _) in &beats {
        add(&mut x, i, &click);
    }
    (x, beats.into_iter().map(|b| b.1).collect())
}

/// Kick track. With `four_on_floor` a kick on every beat plus quiet
/// off-beat hats; otherwise kicks on even beats and equally loud snares on
/// odd beats (drum & bass style). Returns all beat onset times.
pub fn kick_track(
    fs: u32,
    bpm: f64,
    offset: f64,
    secs: f64,
    four_on_floor: bool,
) -> (Vec<f32>, Vec<f64>) {
    let mut x = vec![0f32; (fs as f64 * secs) as usize];
    let k = kick(fs, 0.8);
    let hat = burst(fs, 0.15, 0.01, 3);
    let snare = burst(fs, 0.6, 0.05, 5);
    let half = ((60.0 / bpm / 2.0) * fs as f64).round() as usize;
    let beats = beat_times(fs, bpm, offset, secs);
    for (n, &(i, _)) in beats.iter().enumerate() {
        if four_on_floor || n % 2 == 0 {
            add(&mut x, i, &k);
        } else {
            add(&mut x, i, &snare);
        }
        if four_on_floor {
            add(&mut x, i + half, &hat);
        }
    }
    (x, beats.into_iter().map(|b| b.1).collect())
}

/// Kick track whose kicks on beats `first_down + 4k` are twice as loud and
/// carry a bass note change. Returns all beat times.
pub fn accented_track(
    fs: u32,
    bpm: f64,
    offset: f64,
    secs: f64,
    first_down: usize,
) -> (Vec<f32>, Vec<f64>) {
    let mut x = vec![0f32; (fs as f64 * secs) as usize];
    let loud = kick(fs, 0.9);
    let soft = kick(fs, 0.45);
    let beats = beat_times(fs, bpm, offset, secs);
    for (n, &(i, _)) in beats.iter().enumerate() {
        let down = n >= first_down && (n - first_down).is_multiple_of(4);
        add(&mut x, i, if down { &loud } else { &soft });
    }
    (x, beats.into_iter().map(|b| b.1).collect())
}

/// House-like loop: four-on-the-floor kick, off-beat open hat, 16th closed
/// hats and an off-beat bass. Returns kick times.
pub fn house_track(fs: u32, bpm: f64, offset: f64, secs: f64) -> (Vec<f32>, Vec<f64>) {
    let mut x = vec![0f32; (fs as f64 * secs) as usize];
    let k = kick(fs, 0.8);
    let open = burst(fs, 0.25, 0.04, 11);
    let closed = burst(fs, 0.08, 0.008, 13);
    let period = 60.0 / bpm;
    let beats = beat_times(fs, bpm, offset, secs);
    let bass_len = (period * 0.4 * fs as f64) as usize;
    for (n, &(i, t)) in beats.iter().enumerate() {
        add(&mut x, i, &k);
        let off = ((t + period / 2.0) * fs as f64).round() as usize;
        add(&mut x, off, &open);
        for s in [1, 3] {
            let at = ((t + period * s as f64 / 4.0) * fs as f64).round() as usize;
            add(&mut x, at, &closed);
        }
        let f = if (n / 8) % 2 == 0 { 55.0 } else { 49.0 };
        let bass: Vec<f32> = (0..bass_len)
            .map(|j| {
                let tt = j as f64 / fs as f64;
                let env = (1.0 - j as f64 / bass_len as f64).min(tt / 0.005).min(1.0);
                (0.3 * env * (2.0 * PI * f * tt).sin()) as f32
            })
            .collect();
        add(&mut x, off, &bass);
    }
    (x, beats.into_iter().map(|b| b.1).collect())
}

/// Adds `sound` scaled by `gain` at `at`.
pub fn add_scaled(buf: &mut [f32], at: usize, sound: &[f32], gain: f32) {
    if gain <= 0.0 {
        return;
    }
    for (i, &s) in sound.iter().enumerate() {
        if let Some(b) = buf.get_mut(at + i) {
            *b += s * gain;
        }
    }
}

/// Drum pattern of a generated arrangement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Groove {
    /// Kick on every beat, off-beat open hat, clap on 2 and 4 (house, techno).
    FourFloor,
    /// Broken kick (1, the "and" of 2, 3-and), snare on 2 and 4.
    Breakbeat,
    /// Drum & bass two-step: kick on 1 and the "and" of 3, snare on 2 and 4.
    TwoStep,
    /// Half-time: kick on 1, snare on 3.
    HalfTime,
}

/// Levels (0..=1) of the layers of an arrangement section.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mix {
    pub kick: f32,
    pub bass: f32,
    pub hats: f32,
    /// Snare or clap on the backbeat, with a fill every eighth bar.
    pub snare: f32,
    /// Snare roll whose rate doubles every quarter of the section (builds).
    pub roll: f32,
    pub pad: f32,
    pub lead: f32,
    /// Noise and an upward sweep whose level grows over the section.
    pub riser: f32,
}

impl Mix {
    fn lerp(&self, o: &Mix, t: f32) -> Mix {
        let l = |a: f32, b: f32| a + (b - a) * t;
        Mix {
            kick: l(self.kick, o.kick),
            bass: l(self.bass, o.bass),
            hats: l(self.hats, o.hats),
            snare: l(self.snare, o.snare),
            roll: l(self.roll, o.roll),
            pad: l(self.pad, o.pad),
            lead: l(self.lead, o.lead),
            riser: l(self.riser, o.riser),
        }
    }
}

/// One section of a generated arrangement: its length in bars, the layer
/// levels at its start and end (faded linearly in between) and the chord
/// progression it plays (0 or 1).
#[derive(Clone, Copy, Debug)]
pub struct Section {
    pub bars: usize,
    pub from: Mix,
    pub to: Mix,
    pub harmony: usize,
}

/// Renders an arrangement of `sections` at `bpm` (4/4, first downbeat at
/// t = 0) with kick, bass, hats, snare, snare roll, pad, lead and riser
/// layers. `seed` varies the key and the layer gains (±3 dB) so no two
/// tracks share a mix. Returns the mono signal (one second of tail).
pub fn render_arrangement(
    fs: u32,
    bpm: f64,
    groove: Groove,
    sections: &[Section],
    seed: u64,
) -> Vec<f32> {
    let fsf = fs as f64;
    let period = 60.0 / bpm;
    let bar_len = 4.0 * period;
    let total_bars: usize = sections.iter().map(|s| s.bars).sum();
    let n = ((total_bars as f64 * bar_len + 1.0) * fsf) as usize;
    let mut x = vec![0f32; n];
    let mut rng = Noise::new(seed);
    let mut gain = || 10f64.powf(rng.next() * 3.0 / 20.0) as f32;
    let g = Mix {
        kick: gain(),
        bass: gain(),
        hats: gain(),
        snare: gain(),
        roll: gain(),
        pad: gain(),
        lead: gain(),
        riser: gain(),
    };
    // Key: A1 (55 Hz) moved by up to ±5 semitones.
    let root = 55.0 * 2f64.powf(((seed % 11) as f64 - 5.0) / 12.0);
    // (root semitone, minor) per bar of a four-bar progression.
    const PROGRESSIONS: [[(i32, bool); 4]; 2] = [
        [(0, true), (-4, false), (3, false), (-2, false)],
        [(5, true), (0, true), (-5, true), (-4, false)],
    ];
    // Per bar: the section, its progress at the bar start and end.
    let mut bar_mix: Vec<(Mix, Mix, usize)> = Vec::with_capacity(total_bars);
    for s in sections {
        for b in 0..s.bars {
            let t0 = b as f32 / s.bars as f32;
            let t1 = (b + 1) as f32 / s.bars as f32;
            bar_mix.push((s.from.lerp(&s.to, t0), s.from.lerp(&s.to, t1), s.harmony));
        }
    }
    let mix_at = |bar: usize, frac: f64| -> Mix {
        let (a, b, _) = &bar_mix[bar.min(total_bars - 1)];
        a.lerp(b, frac as f32)
    };

    // Drum hits.
    let kick_s = kick(fs, 0.8);
    let hat = burst(fs, 0.18, 0.012, seed ^ 3);
    let open_hat = burst(fs, 0.22, 0.05, seed ^ 5);
    let snare: Vec<f32> = {
        let noise = burst(fs, 0.5, 0.06, seed ^ 7);
        noise
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let t = i as f64 / fsf;
                v + (0.3 * (2.0 * PI * 190.0 * t).sin() * (-t / 0.05).exp()) as f32
            })
            .collect()
    };
    let at = |beat: f64| (beat * period * fsf).round() as usize;
    for bar in 0..total_bars {
        let fill = (bar + 1) % 8 == 0;
        for step in 0..16 {
            // Sixteenth steps of the bar.
            let beat = bar as f64 * 4.0 + step as f64 / 4.0;
            let m = mix_at(bar, step as f64 / 16.0);
            let i = at(beat);
            let kick_on = match groove {
                Groove::FourFloor => step % 4 == 0,
                Groove::Breakbeat => matches!(step, 0 | 6 | 10),
                Groove::TwoStep => matches!(step, 0 | 10),
                Groove::HalfTime => step == 0 || (step == 14 && bar % 2 == 1),
            };
            if kick_on {
                add_scaled(&mut x, i, &kick_s, m.kick * g.kick);
            }
            let snare_on = match groove {
                Groove::HalfTime => step == 8,
                _ => step == 4 || step == 12,
            } || (fill && step >= 12 && step % 2 == 0 && groove != Groove::HalfTime);
            if snare_on {
                add_scaled(&mut x, i, &snare, 0.8 * m.snare * g.snare);
            }
            let hat_level = match groove {
                Groove::FourFloor if step % 4 == 2 => {
                    add_scaled(&mut x, i, &open_hat, m.hats * g.hats);
                    0.0
                }
                Groove::TwoStep | Groove::Breakbeat => 0.8,
                Groove::HalfTime if step % 2 == 0 => 0.7,
                _ => 0.5,
            };
            add_scaled(&mut x, i, &hat, hat_level * m.hats * g.hats);
        }
        // Snare roll: quarters, eighths, sixteenths, thirty-seconds over
        // the four quarters of its section.
        let (m0, m1, _) = bar_mix[bar];
        if m0.roll > 0.0 || m1.roll > 0.0 {
            let (sec_start, sec_bars) = {
                let mut start = 0;
                let mut found = (0, 1);
                for s in sections {
                    if bar < start + s.bars {
                        found = (start, s.bars);
                        break;
                    }
                    start += s.bars;
                }
                found
            };
            let prog = (bar - sec_start) as f64 / sec_bars as f64;
            let per_beat = if prog < 0.25 {
                1
            } else if prog < 0.5 {
                2
            } else if prog < 0.75 {
                4
            } else {
                8
            };
            for k in 0..4 * per_beat {
                let frac = k as f64 / (4 * per_beat) as f64;
                let m = mix_at(bar, frac);
                let i = at(bar as f64 * 4.0 + 4.0 * frac);
                add_scaled(
                    &mut x,
                    i,
                    &snare,
                    0.6 * m.roll * g.roll * (0.5 + 0.5 * frac as f32),
                );
            }
        }
    }

    // Tonal layers and the riser, sample by sample.
    let mut noise = Noise::new(seed ^ 11);
    let (mut ph_bass, mut ph_sweep) = (0.0f64, 0.0f64);
    let mut ph_pad = [0.0f64; 6];
    let mut ph_lead = 0.0f64;
    let mut prev_noise = 0.0;
    let end = (total_bars as f64 * bar_len * fsf) as usize;
    for (i, out) in x.iter_mut().enumerate().take(end) {
        let t = i as f64 / fsf;
        let beat_pos = t / period;
        let bar = (beat_pos / 4.0) as usize;
        if bar >= total_bars {
            break;
        }
        let in_bar = beat_pos / 4.0 - bar as f64;
        let m = mix_at(bar, in_bar);
        let (chord_root, minor) = PROGRESSIONS[bar_mix[bar].2][bar % 4];
        let f_root = root * 2f64.powf(chord_root as f64 / 12.0);
        let mut v = 0.0f64;
        if m.bass > 0.0 {
            ph_bass += 2.0 * PI * f_root / fsf;
            let beat_frac = beat_pos.fract();
            let env = match groove {
                // Off-beat bass notes.
                Groove::FourFloor | Groove::Breakbeat => {
                    if beat_frac >= 0.5 {
                        ((beat_frac - 0.5) / 0.02).min(1.0) * (1.0 - (beat_frac - 0.5) * 1.6)
                    } else {
                        0.0
                    }
                }
                // Sustained reese.
                Groove::TwoStep => 0.8,
                // Wobble in eighths.
                Groove::HalfTime => 0.55 + 0.45 * (2.0 * PI * beat_pos * 2.0).cos(),
            };
            let tone = ph_bass.sin()
                + 0.35 * (2.0 * ph_bass).sin()
                + if groove == Groove::TwoStep {
                    0.3 * (3.0 * ph_bass * 1.003).sin()
                } else {
                    0.0
                };
            v += 0.3 * (m.bass * g.bass) as f64 * env * tone;
        }
        if m.pad > 0.0 {
            let third = if minor { 3.0 } else { 4.0 };
            let notes = [0.0, third, 7.0];
            let mut s = 0.0;
            for (k, semi) in notes.iter().enumerate() {
                let f = f_root * 4.0 * 2f64.powf(semi / 12.0);
                for (d, det) in [(0, 1.0), (3, 1.004)] {
                    let p = &mut ph_pad[k + d];
                    *p += 2.0 * PI * f * det / fsf;
                    s += p.sin();
                }
            }
            v += 0.035 * (m.pad * g.pad) as f64 * s;
        }
        if m.lead > 0.0 {
            // Eighth-note arpeggio over the chord, two octaves up.
            let step = (beat_pos * 2.0) as usize;
            let third = if minor { 3.0 } else { 4.0 };
            let semi = [0.0, third, 7.0, 12.0, 7.0, third, 12.0, 7.0][step % 8];
            let f = f_root * 16.0 * 2f64.powf(semi / 12.0);
            ph_lead += 2.0 * PI * f / fsf;
            let env = (-(beat_pos * 2.0).fract() * 3.0).exp();
            let saw = ph_lead.sin()
                + 0.5 * (2.0 * ph_lead).sin()
                + 0.33 * (3.0 * ph_lead).sin()
                + 0.25 * (4.0 * ph_lead).sin();
            v += 0.09 * (m.lead * g.lead) as f64 * env * saw;
        }
        if m.riser > 0.0 {
            // Within its section the riser climbs: level and pitch follow
            // the progress through the section.
            let mut start = 0;
            let mut prog = 0.0;
            for s in sections {
                if bar < start + s.bars {
                    prog = ((beat_pos / 4.0 - start as f64) / s.bars as f64).clamp(0.0, 1.0);
                    break;
                }
                start += s.bars;
            }
            let w = noise.next();
            let hp = w - prev_noise;
            prev_noise = w;
            ph_sweep += 2.0 * PI * (300.0 + 2700.0 * prog * prog) / fsf;
            v += (m.riser * g.riser) as f64 * (0.12 * prog * hp + 0.08 * prog * ph_sweep.sin());
        }
        *out += v as f32;
    }
    x
}
