//! The phrase detector as it was before the self-similarity rewrite (test
//! builds only). Kept unchanged so the evaluation in `phrase::eval` can
//! report old and new results side by side.
#![allow(dead_code)]

use crate::dsp::{Biquad, energy_prefix, filtered, mean_square};
use od_core::{Beatgrid, Phrase, PhraseKind, PhraseMood};
use std::f64::consts::FRAC_1_SQRT_2;

/// Allowed regular phrase lengths in bars.
const LENGTHS: [usize; 3] = [8, 16, 32];

#[derive(Clone, Copy, Debug)]
struct BarFeat {
    all: f64,
    low: f64,
    mid: f64,
    high: f64,
    /// Linear mean square of the full band.
    power: f64,
}

impl BarFeat {
    fn dist(&self, o: &BarFeat) -> f64 {
        (self.all - o.all).abs()
            + (self.low - o.low).abs()
            + (self.mid - o.mid).abs()
            + (self.high - o.high).abs()
    }
}

fn mean_feat(f: &[BarFeat]) -> BarFeat {
    let n = f.len().max(1) as f64;
    let mut m = BarFeat {
        all: 0.0,
        low: 0.0,
        mid: 0.0,
        high: 0.0,
        power: 0.0,
    };
    for b in f {
        m.all += b.all / n;
        m.low += b.low / n;
        m.mid += b.mid / n;
        m.high += b.high / n;
        m.power += b.power / n;
    }
    m
}

/// Detects phrases from a mono signal and its beatgrid. Returns no phrases
/// (and [`PhraseMood::Mid`]) when the grid has fewer than eight bars.
pub fn detect_phrases(
    samples: &[f32],
    sample_rate: u32,
    grid: &Beatgrid,
) -> (Vec<Phrase>, PhraseMood) {
    let none = (Vec::new(), PhraseMood::default());
    if samples.is_empty() || sample_rate == 0 || grid.beats.len() < 32 {
        return none;
    }
    let fs = sample_rate as f64;
    let q = FRAC_1_SQRT_2;
    let lp = Biquad::lowpass(fs, 150.0, q);
    let hp_low = Biquad::highpass(fs, 150.0, q);
    let hi_f = 2000.0f64.min(fs * 0.4);
    let lp_high = Biquad::lowpass(fs, hi_f, q);
    let hp_high = Biquad::highpass(fs, hi_f, q);
    let ((p_all, p_low), (p_mid, p_high)) = rayon::join(
        || {
            rayon::join(
                || energy_prefix(samples),
                || energy_prefix(&filtered(samples, &[lp, lp])),
            )
        },
        || {
            rayon::join(
                || energy_prefix(&filtered(samples, &[hp_low, hp_low, lp_high, lp_high])),
                || energy_prefix(&filtered(samples, &[hp_high, hp_high])),
            )
        },
    );

    // Bars: (first beat index, start sample, end sample).
    let downbeats: Vec<usize> = grid
        .beats
        .iter()
        .enumerate()
        .filter(|(_, b)| b.bar_position == 1)
        .map(|(i, _)| i)
        .collect();
    let n_samples = samples.len();
    let mut bars: Vec<(usize, BarFeat)> = Vec::new();
    for &d in &downbeats {
        let Some(end_beat) = grid.beats.get(d + 4) else {
            break;
        };
        let a = ((grid.beats[d].time * fs).max(0.0) as usize).min(n_samples);
        let b = ((end_beat.time * fs) as usize).min(n_samples);
        if b <= a {
            break;
        }
        let db = |p: &[f64]| 10.0 * (mean_square(p, a, b) + 1e-10).log10();
        bars.push((
            d,
            BarFeat {
                all: db(&p_all),
                low: db(&p_low),
                mid: db(&p_mid),
                high: db(&p_high),
                power: mean_square(&p_all, a, b),
            },
        ));
    }
    let max_power = bars.iter().map(|b| b.1.power).fold(0.0, f64::max);
    if bars.len() < 8 || max_power <= 1e-10 {
        return none;
    }
    // Music range: first to last bar within 30 dB of the loudest bar.
    let audible = |f: &BarFeat| f.power > max_power * 1e-3;
    let Some(first) = bars.iter().position(|b| audible(&b.1)) else {
        return none;
    };
    let last = bars
        .iter()
        .rposition(|b| audible(&b.1))
        .unwrap_or(bars.len() - 1);
    let bars = &bars[first..=last];
    let feats: Vec<BarFeat> = bars
        .iter()
        .map(|b| {
            // Clip at −60 dB below the loudest bar so silence doesn't dominate.
            let floor = 10.0 * max_power.log10() - 60.0;
            BarFeat {
                all: b.1.all.max(floor),
                low: b.1.low.max(floor),
                mid: b.1.mid.max(floor),
                high: b.1.high.max(floor),
                power: b.1.power,
            }
        })
        .collect();
    let nb = feats.len();

    let novelty: Vec<f64> = (0..=nb)
        .map(|j| {
            if j == 0 || j >= nb {
                return 0.0;
            }
            let w = 4.min(j).min(nb - j);
            mean_feat(&feats[j - w..j]).dist(&mean_feat(&feats[j..j + w]))
                + 0.5 * feats[j].dist(&feats[j - 1])
        })
        .collect();

    let mut boundaries = segment(&novelty, nb);
    // A trailing fragment shorter than four bars joins the previous phrase.
    if boundaries.len() > 2 && nb - boundaries[boundaries.len() - 2] < 4 {
        boundaries.remove(boundaries.len() - 2);
    }
    let end_beat_of = |bar: usize| -> u32 {
        match bars.get(bar) {
            Some(b) => b.0 as u32,
            None => (bars[nb - 1].0 + 4).min(grid.beats.len()) as u32,
        }
    };

    // Per-phrase statistics.
    struct Stats {
        energy: f64,
        low: f64,
        slope: f64,
    }
    let stats: Vec<Stats> = boundaries
        .windows(2)
        .map(|w| {
            let f = &feats[w[0]..w[1]];
            let m = mean_feat(f);
            let h = f.len() / 2;
            let slope = if h > 0 {
                let a = mean_feat(&f[..h]);
                let b = mean_feat(&f[h..]);
                (b.all - a.all) + (b.high - a.high)
            } else {
                0.0
            };
            Stats {
                energy: m.all,
                low: m.low,
                slope,
            }
        })
        .collect();
    let e_max = stats.iter().map(|s| s.energy).fold(f64::MIN, f64::max);
    let e_min = stats.iter().map(|s| s.energy).fold(f64::MAX, f64::min);
    let low_max = stats.iter().map(|s| s.low).fold(f64::MIN, f64::max);
    let range = e_max - e_min;
    let norm = |e: f64| {
        if range < 3.0 {
            1.0
        } else {
            (e - e_min) / range
        }
    };
    let n = stats.len();
    let hi: Vec<bool> = stats
        .iter()
        .map(|s| norm(s.energy) >= 0.75 && s.low >= low_max - 6.0)
        .collect();
    let mut kinds = Vec::with_capacity(n);
    let mut seen_chorus = false;
    for (i, s) in stats.iter().enumerate() {
        let en = norm(s.energy);
        let bass = s.low >= low_max - 9.0;
        let kind = if n >= 3 && i == 0 {
            PhraseKind::Intro
        } else if n >= 3 && i == n - 1 {
            PhraseKind::Outro
        } else if n >= 4 && i == 1 && kinds[0] == PhraseKind::Intro && en < 0.5 && !hi[i] {
            PhraseKind::Intro
        } else if hi[i] || (kinds.last() == Some(&PhraseKind::Chorus) && en >= 0.6 && bass) {
            // Hysteresis: a chorus stays a chorus until the energy clearly
            // drops, so phrases near the threshold do not flap.
            PhraseKind::Chorus
        } else if hi.get(i + 1).copied().unwrap_or(false) && s.slope > 1.0 {
            // A riser into the drop is Up even without bass.
            PhraseKind::Up
        } else if en < 0.4 || !bass {
            PhraseKind::Down
        } else if seen_chorus {
            PhraseKind::Bridge
        } else {
            PhraseKind::Verse
        };
        seen_chorus |= kind == PhraseKind::Chorus;
        kinds.push(kind);
    }
    // Neighbouring phrases of the same kind are one section. The segmentation
    // cuts wherever the texture shifts, but a drop that loses its hats halfway
    // is still one Chorus, not Chorus, Chorus, Chorus.
    let mut i = 1;
    while i < kinds.len() {
        if kinds[i] == kinds[i - 1] {
            kinds.remove(i);
            boundaries.remove(i);
        } else {
            i += 1;
        }
    }

    let phrases: Vec<Phrase> = od_core::normalize_phrases(
        &boundaries
            .windows(2)
            .zip(&kinds)
            .map(|(w, &kind)| Phrase::new(end_beat_of(w[0]), end_beat_of(w[1]), kind))
            .collect::<Vec<_>>(),
    );

    // Mood.
    let bpm = grid.dominant_bpm().unwrap_or(120.0);
    let chorus_bars: usize = boundaries
        .windows(2)
        .zip(&kinds)
        .filter(|(_, k)| matches!(k, PhraseKind::Chorus | PhraseKind::Up))
        .map(|(w, _)| w[1] - w[0])
        .sum();
    let chorus_share = chorus_bars as f64 / nb as f64;
    let mean_bass = feats.iter().map(|f| f.low - f.all).sum::<f64>() / nb as f64;
    let mood = if bpm >= 118.0 && chorus_share >= 0.3 && mean_bass > -8.0 {
        PhraseMood::High
    } else if bpm < 100.0 || (chorus_share < 0.15 && range >= 3.0) {
        PhraseMood::Low
    } else {
        PhraseMood::Mid
    };
    (phrases, mood)
}

/// Dynamic-programming segmentation of `nb` bars into phrases. Returns the
/// boundary bar indices including 0 and `nb`.
fn segment(novelty: &[f64], nb: usize) -> Vec<usize> {
    if nb <= 8 {
        return vec![0, nb];
    }
    let mut at8: Vec<f64> = (8..nb).step_by(8).map(|j| novelty[j]).collect();
    at8.sort_by(f64::total_cmp);
    let lambda = at8.get(at8.len() / 2).copied().unwrap_or(0.0);
    let mut best = vec![f64::NEG_INFINITY; nb + 1];
    let mut prev = vec![usize::MAX; nb + 1];
    best[0] = 0.0;
    for p in 1..=nb {
        let gain = if p == nb { 0.0 } else { novelty[p] - lambda };
        let mut cands: Vec<(usize, f64)> = LENGTHS
            .iter()
            .filter(|&&l| l <= p)
            .map(|&l| (p - l, 0.0))
            .collect();
        if p == nb {
            // The final phrase may have any length up to 32 bars.
            cands.extend(
                (1..=32.min(p))
                    .filter(|l| !LENGTHS.contains(l))
                    .map(|l| (p - l, 0.5 * lambda.abs() + 0.1)),
            );
        }
        for (from, penalty) in cands {
            if best[from].is_finite() {
                let v = best[from] + gain - penalty;
                if v > best[p] {
                    best[p] = v;
                    prev[p] = from;
                }
            }
        }
    }
    let mut out = vec![nb];
    let mut p = nb;
    while p > 0 {
        let q = prev[p];
        if q == usize::MAX {
            return vec![0, nb];
        }
        out.push(q);
        p = q;
    }
    out.reverse();
    out
}
