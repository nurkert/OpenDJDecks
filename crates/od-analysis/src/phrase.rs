//! Phrase (section) detection, phrase confidence and phrase mood.
//!
//! Works on bars taken from the beatgrid (downbeat to downbeat):
//!
//! 1. **Features per bar** from a short-time spectrum: energy, the energy
//!    in eight bands from sub bass to air (a coarse timbre), kick strength
//!    (sub-band flux peaks), sustained sub bass, onset density (hits per
//!    beat above 150 Hz), brightness (spectral centroid) and a chroma vector
//!    (harmony).
//! 2. **Self-similarity:** every bar is compared with every other bar
//!    (distance of the features, each scaled by a fixed step it must change
//!    by to count, see [`STEPS`]). A checkerboard kernel slid along the
//!    diagonal at four sizes (1, 2, 4 and 8 bars a side) gives a novelty
//!    curve: high where the bars before differ from the bars after.
//! 3. **Boundaries by dynamic programming** on the novelty peaks: each
//!    boundary earns its novelty minus a threshold; each phrase pays for its
//!    length, nothing for 8, 16 or 32 bars, a little for other multiples of
//!    four, more for other lengths ([`length_cost`]), and phrases under four
//!    bars are not placed; a phrase that does not start on a 4-bar group
//!    pays a little too. Builds, which grow gradually out of a breakdown,
//!    are then split off its end ([`split_builds`]).
//! 4. **Labels** from the musical role of each phrase: Intro and Outro are
//!    the beat-led ends (kick without bass, whatever comes before the first
//!    beat, a beatless tail, and always the first and last phrase); Down is
//!    a breakdown (no kick, or kick without bass); Up is a build (rising
//!    energy, hit density or brightness) right before the beat comes back; a
//!    Chorus (the drop) has kick and bass, comes out of a breakdown or build
//!    and is near the loudest part of the track; phrases that sound like a
//!    drop (by self-similarity) are drops too. Other beat-led phrases are
//!    Verse before the first drop and Bridge after it. A track without a
//!    breakdown or build has no drop.
//! 5. **Confidence** per phrase: the weaker of how clear its boundaries
//!    are (novelty against the threshold) and how clearly its features pass
//!    the tests that gave its type.
//! 6. The **mood** (High / Mid / Low) follows from tempo, share of Chorus
//!    bars and bass weight.
//!
//! The thresholds are stated in musical units (decibels, octaves, hits per
//! beat, bars) and chosen from what a listener would call a change, not
//! fitted per track; see the constants below.

use crate::dsp::stft_map_pairs;
use od_core::{Beatgrid, Phrase, PhraseKind, PhraseMood};

/// FFT size and hop of the analysis spectrum (≈ 93 ms and 23 ms at 11 kHz).
const N_FFT: usize = 1024;
const HOP: usize = 256;
/// Band edges in Hz: sub, kick, low bass, low mids, mids, high mids,
/// presence, air.
const BAND_EDGES: [f64; 9] = [
    20.0,
    90.0,
    150.0,
    300.0,
    600.0,
    1200.0,
    2400.0,
    4000.0,
    f64::INFINITY,
];
const N_BANDS: usize = 8;
/// Bands below this are the low end (kick and bass).
const LOW_HZ: f64 = 150.0;
/// Kick and bass are judged in the sub band (30–100 Hz), where pads, snares
/// and risers put little energy.
const SUB_HZ: (f64, f64) = (30.0, 100.0);

/// Feature dimensions compared between bars: band levels (8), kick,
/// sustained sub bass, onset density, brightness, chroma (12).
const DIMS: usize = N_BANDS + 4 + 12;
/// The change of each feature that counts as one step in the similarity
/// distance: 3 dB for levels, 0.5 hits per beat, a quarter octave of
/// brightness; chroma (unit vectors) is weighted so a change of chord set
/// counts like a layer coming in.
const STEPS: [f64; 4] = [3.0, 0.5, 0.25, 0.5];

/// A boundary must stand out by this much novelty (in steps) to be worth a
/// phrase of its own: about one layer coming in or going out.
const THRESHOLD: f64 = 1.0;

/// Kick: present when its sub-band flux peaks are within 9 dB of the
/// track's strong kick bars (90th percentile); confidence grows over 6 dB
/// either side.
const KICK_MARGIN_DB: f64 = 9.0;
/// Bass: present when the sustained sub band is within 12 dB of the
/// track's loud bass bars.
const BASS_MARGIN_DB: f64 = 12.0;
/// A drop is within this many dB of the loudest beat-led phrase.
const DROP_WITHIN_DB: f64 = 3.0;
/// A beat-led phrase after the first drop that does not sound like a drop
/// is a Bridge, unless it is this much louder than the drops (then the drop
/// goes on in a new form).
const BRIDGE_BELOW_DB: f64 = 1.5;
/// A build rises by at least this (energy dB, or the same in steps of hit
/// density and brightness) from its first to its last quarter.
const RISE_DB: f64 = 3.0;

#[derive(Clone, Copy, Debug, Default)]
struct FrameFeat {
    bands: [f32; N_BANDS],
    /// Energy and positive (linear) flux in the sub band.
    sub: f32,
    flux_sub: f32,
    chroma: [f32; 12],
    /// Positive flux above [`LOW_HZ`] (hats, snares, chords).
    flux_high: f32,
    /// Spectral centroid, Hz.
    centroid: f32,
}

/// Features of one bar.
#[derive(Clone, Copy, Debug)]
struct BarFeat {
    /// Total energy, dB.
    energy: f64,
    bands: [f64; N_BANDS],
    /// Sustained energy in the sub band (its 30th percentile over the
    /// bar's frames, so a kick alone, loud only on its hits, does not
    /// count as bass), dB.
    low: f64,
    /// Kick strength: the 90th percentile of the sub-band flux over the
    /// bar's frames (the hits, not the average), dB.
    kick: f64,
    /// Hits per beat above [`LOW_HZ`].
    onsets: f64,
    /// Spectral centroid, octaves above 100 Hz.
    bright: f64,
    chroma: [f64; 12],
    /// Linear mean power (for the audible range).
    power: f64,
}

fn db(p: f64) -> f64 {
    10.0 * (p + 1e-12).log10()
}

fn percentile(v: &[f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s[((s.len() - 1) as f64 * q).round() as usize]
}

/// Per-frame spectral features.
fn frame_features(samples: &[f32], fs: f64) -> Vec<FrameFeat> {
    let bins = N_FFT / 2 + 1;
    let hz = |k: usize| k as f64 * fs / N_FFT as f64;
    let band_of: Vec<Option<usize>> = (0..bins)
        .map(|k| {
            BAND_EDGES
                .windows(2)
                .position(|w| hz(k) >= w[0] && hz(k) < w[1])
        })
        .collect();
    let chroma_of: Vec<Option<usize>> = (0..bins)
        .map(|k| {
            let f = hz(k);
            (100.0..2000.0).contains(&f).then(|| {
                let midi = 69.0 + 12.0 * (f / 440.0).log2();
                (midi.round() as i64).rem_euclid(12) as usize
            })
        })
        .collect();
    let low_bins = (0..bins).take_while(|&k| hz(k) < LOW_HZ).count();
    let sub = |k: usize| hz(k) >= SUB_HZ.0 && hz(k) < SUB_HZ.1;
    stft_map_pairs(
        samples,
        N_FFT,
        HOP,
        |_| {},
        |_, prev, cur| {
            let mut f = FrameFeat::default();
            let (mut num, mut den) = (0f32, 0f32);
            for k in 1..bins {
                let p = cur[k] * cur[k];
                if let Some(b) = band_of[k] {
                    f.bands[b] += p;
                }
                if let Some(c) = chroma_of[k] {
                    f.chroma[c] += cur[k];
                }
                num += p * hz(k) as f32;
                den += p;
                if sub(k) {
                    f.sub += p;
                }
                if let Some(prev) = prev {
                    // Sub band: linear flux, so faint noise (risers,
                    // snare rolls) does not look like a kick.
                    if sub(k) && cur[k] > prev[k] {
                        f.flux_sub += cur[k] - prev[k];
                    }
                    let d = (1.0 + cur[k]).ln() - (1.0 + prev[k]).ln();
                    if d > 0.0 && k >= low_bins {
                        f.flux_high += d;
                    }
                }
            }
            f.centroid = if den > 0.0 { num / den } else { 0.0 };
            f
        },
    )
}

/// Bars of the grid as (first beat index, start sample, end sample).
fn grid_bars(grid: &Beatgrid, fs: f64, n_samples: usize) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (d, beat) in grid.beats.iter().enumerate() {
        if beat.bar_position != 1 {
            continue;
        }
        let Some(end_beat) = grid.beats.get(d + 4) else {
            break;
        };
        let a = ((beat.time * fs).max(0.0) as usize).min(n_samples);
        let b = ((end_beat.time * fs) as usize).min(n_samples);
        if b <= a {
            break;
        }
        out.push((d, a, b));
    }
    out
}

/// Aggregates frame features into bar features.
fn bar_features(frames: &[FrameFeat], bars: &[(usize, usize, usize)]) -> Vec<BarFeat> {
    // Onsets: local maxima of the high-band flux above 1.3 × its local
    // mean (±8 frames) plus a floor of a tenth of the track's median.
    let flux: Vec<f64> = frames.iter().map(|f| f.flux_high as f64).collect();
    let floor = 0.1 * percentile(&flux, 0.5);
    let n = flux.len();
    let mut prefix = vec![0.0; n + 1];
    for i in 0..n {
        prefix[i + 1] = prefix[i] + flux[i];
    }
    let onset: Vec<bool> = (0..n)
        .map(|i| {
            let (a, b) = (i.saturating_sub(8), (i + 9).min(n));
            let local = (prefix[b] - prefix[a]) / (b - a) as f64;
            i > 0
                && i + 1 < n
                && flux[i] > flux[i - 1]
                && flux[i] >= flux[i + 1]
                && flux[i] > 1.3 * local + floor
        })
        .collect();
    bars.iter()
        .map(|&(_, a, b)| {
            // Frames centred inside the bar.
            let fa = a.div_ceil(HOP).min(n - 1);
            let fb = b.div_ceil(HOP).clamp(fa + 1, n);
            let fr = &frames[fa..fb];
            let m = fr.len().max(1) as f64;
            let mut bands = [0f64; N_BANDS];
            let mut chroma = [0f64; 12];
            let (mut num, mut den) = (0.0, 0.0);
            for f in fr {
                for (s, &v) in bands.iter_mut().zip(&f.bands) {
                    *s += v as f64 / m;
                }
                for (s, &v) in chroma.iter_mut().zip(&f.chroma) {
                    *s += v as f64;
                }
                let p: f64 = f.bands.iter().map(|&v| v as f64).sum();
                num += f.centroid as f64 * p;
                den += p;
            }
            let power: f64 = bands.iter().sum();
            let norm = chroma.iter().map(|c| c * c).sum::<f64>().sqrt();
            if norm > 0.0 {
                chroma.iter_mut().for_each(|c| *c /= norm);
            }
            let hits = (fa..fb).filter(|&i| onset[i]).count() as f64;
            let lows: Vec<f64> = fr.iter().map(|f| f.sub as f64).collect();
            let kicks: Vec<f64> = fr.iter().map(|f| f.flux_sub as f64).collect();
            BarFeat {
                energy: db(power),
                bands: bands.map(db),
                low: db(percentile(&lows, 0.3)),
                kick: 2.0 * db(percentile(&kicks, 0.9)),
                onsets: hits / 4.0,
                bright: if den > 0.0 {
                    (num / den / 100.0).max(1.0).log2()
                } else {
                    0.0
                },
                chroma,
                power,
            }
        })
        .collect()
}

/// Bar feature vectors in steps (see [`STEPS`]), levels clipped 60 dB
/// under the loudest bar.
fn vectors(feats: &[BarFeat]) -> Vec<[f64; DIMS]> {
    let top = feats.iter().map(|f| f.energy).fold(f64::MIN, f64::max);
    let kick_top = feats.iter().map(|f| f.kick).fold(f64::MIN, f64::max);
    feats
        .iter()
        .map(|f| {
            let mut v = [0f64; DIMS];
            for (i, b) in f.bands.iter().enumerate() {
                v[i] = b.max(top - 60.0) / STEPS[0];
            }
            v[N_BANDS] = f.kick.max(kick_top - 30.0) / STEPS[0];
            v[N_BANDS + 1] = f.low.max(top - 60.0) / STEPS[0];
            v[N_BANDS + 2] = f.onsets / STEPS[1];
            v[N_BANDS + 3] = f.bright / STEPS[2];
            for (i, c) in f.chroma.iter().enumerate() {
                v[N_BANDS + 4 + i] = c / STEPS[3];
            }
            v
        })
        .collect()
}

/// Bar-to-bar distance matrix.
fn distances(v: &[[f64; DIMS]]) -> Vec<Vec<f64>> {
    let n = v.len();
    let mut d = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in i + 1..n {
            let s: f64 = v[i].iter().zip(&v[j]).map(|(a, b)| (a - b) * (a - b)).sum();
            d[i][j] = s.sqrt();
            d[j][i] = d[i][j];
        }
    }
    d
}

/// Novelty at every bar boundary `b` (between bar `b - 1` and `b`, 0 at the
/// ends): checkerboard kernels of 1, 2, 4 and 8 bars a side on the distance
/// matrix (mean distance across the boundary minus mean distance within
/// each side), averaged. Kernels are cut at the track ends.
fn novelty(d: &[Vec<f64>]) -> Vec<f64> {
    let n = d.len();
    (0..=n)
        .map(|b| {
            if b == 0 || b >= n {
                return 0.0;
            }
            let mut sum = 0.0;
            let mut count = 0.0;
            for half in [1usize, 2, 4, 8] {
                let l = half.min(b).min(n - b);
                if l == 0 {
                    continue;
                }
                let (mut cross, mut within) = (0.0, 0.0);
                for i in b - l..b {
                    for j in b..b + l {
                        cross += d[i][j];
                    }
                }
                let mut pairs = 0.0;
                for (lo, hi) in [(b - l, b), (b, b + l)] {
                    for i in lo..hi {
                        for j in i + 1..hi {
                            within += d[i][j];
                            pairs += 1.0;
                        }
                    }
                }
                let cross = cross / (l * l) as f64;
                let within = if pairs > 0.0 { within / pairs } else { 0.0 };
                sum += (cross - within).max(0.0);
                count += 1.0;
            }
            if count > 0.0 { sum / count } else { 0.0 }
        })
        .collect()
}

/// What a phrase of `len` bars costs: nothing for 8, 16 and 32 bars, a
/// little for the other 4-bar multiples DJs meet (4, 12, 24, 48, 64), more
/// for other multiples of four, more again for other even lengths and most
/// for odd ones and anything under four bars.
fn length_cost(len: usize) -> f64 {
    match len {
        8 | 16 | 32 => 0.0,
        4 | 12 | 24 | 48 | 64 => 0.4,
        l if l % 4 == 0 => 0.8,
        l if l < 4 => 2.5,
        l if l % 2 == 0 => 1.4,
        _ => 1.8,
    }
}

/// The longest phrase the segmentation places (longer sections are split
/// and joined again when their parts get the same type).
const MAX_LEN: usize = 64;
/// What a phrase start off the 4-bar grid (counted from the first bar of
/// music) costs.
const OFF_GROUP_COST: f64 = 0.3;

/// The shortest phrase: shorter fragments would join a neighbour anyway
/// (see [`od_core::normalize_phrases`]).
const MIN_LEN: usize = 4;

/// What a boundary one bar off its novelty peak costs. A change often
/// spills into the bar before it (a fill, or a downbeat a beat off), so the
/// boundary may sit on the regular bar next to the peak.
const OFF_PEAK_COST: f64 = 0.2;

/// Dynamic-programming segmentation of `nb` bars. `novelty[b]` is the
/// novelty at boundary `b`; only its peaks (the highest within two bars
/// either side) and the bars next to them can be boundaries, and a peak
/// earns at most three times [`THRESHOLD`], so one strong change cannot
/// buy a crowd of boundaries around it. Returns the boundary bar indices
/// including 0 and `nb`.
fn segment(novelty: &[f64], nb: usize) -> Vec<usize> {
    if nb <= 8 {
        return vec![0, nb];
    }
    let peak = |b: usize| {
        let lo = b.saturating_sub(2);
        let hi = (b + 2).min(nb);
        (lo..=hi).all(|c| c == b || novelty[c] < novelty[b] || (novelty[c] == novelty[b] && c > b))
    };
    let mut best = vec![f64::NEG_INFINITY; nb + 1];
    let mut prev = vec![usize::MAX; nb + 1];
    best[0] = 0.0;
    for p in 1..=nb {
        let last = p == nb;
        let max_len = if last { nb } else { MAX_LEN.min(p) };
        for len in MIN_LEN..=max_len {
            let q = p - len;
            if !best[q].is_finite() {
                continue;
            }
            // The last phrase ends with the track, wherever that is: it
            // pays at most for a 4-bar multiple.
            let cost = if last {
                length_cost(len).min(length_cost(4))
            } else {
                length_cost(len)
            };
            let gain = if q == 0 {
                0.0
            } else {
                let Some(evidence) = [(q, 0.0), (q - 1, OFF_PEAK_COST), (q + 1, OFF_PEAK_COST)]
                    .into_iter()
                    .filter(|&(c, _)| c > 0 && c < nb && peak(c))
                    .map(|(c, cost)| novelty[c].min(3.0 * THRESHOLD) / THRESHOLD - cost)
                    .max_by(f64::total_cmp)
                else {
                    continue;
                };
                evidence - 1.0 - if q % 4 != 0 { OFF_GROUP_COST } else { 0.0 }
            };
            let v = best[q] + gain - cost;
            if v > best[p] {
                best[p] = v;
                prev[p] = q;
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

/// Features of one phrase candidate.
#[derive(Clone, Copy, Debug)]
struct SegFeat {
    energy: f64,
    low: f64,
    kick: f64,
    /// Rise from the first to the last quarter: energy in dB plus hit
    /// density and brightness in [`RISE_DB`]-equivalent steps.
    rise: f64,
}

fn seg_feat(f: &[BarFeat]) -> SegFeat {
    let n = f.len().max(1);
    let mean = |g: &dyn Fn(&BarFeat) -> f64, s: &[BarFeat]| {
        s.iter().map(g).sum::<f64>() / s.len().max(1) as f64
    };
    // Energies average in the power domain.
    let energy = db(mean(&|b| b.power, f));
    let q = (n / 4).max(1);
    let (head, tail) = (&f[..q.min(f.len())], &f[f.len().saturating_sub(q)..]);
    let de = db(mean(&|b| b.power, tail)) - db(mean(&|b| b.power, head));
    let on_head = mean(&|b| b.onsets, head);
    let on_tail = mean(&|b| b.onsets, tail);
    let dbright = mean(&|b| b.bright, tail) - mean(&|b| b.bright, head);
    let rise =
        de + RISE_DB * ((on_tail - on_head) / 1.0).max(0.0) + RISE_DB * (dbright / 0.5).max(0.0);
    SegFeat {
        energy,
        low: db(mean(&|b| 10f64.powf(b.low / 10.0), f)),
        kick: db(mean(&|b| 10f64.powf(b.kick / 10.0), f)),
        rise,
    }
}

/// The track's reference levels: kick strength and sub-bass level of its
/// strong bars (90th percentile), dB.
fn references(feats: &[BarFeat]) -> (f64, f64) {
    let kicks: Vec<f64> = feats.iter().map(|f| f.kick).collect();
    let lows: Vec<f64> = feats.iter().map(|f| f.low).collect();
    (percentile(&kicks, 0.9), percentile(&lows, 0.9))
}

/// Builds sit at the end of breakdowns and grow gradually, so the novelty
/// often misses where they start. A phrase without kick that leads into
/// one with kick and bass gives up its last 4, 8 or 16 bars as a build when
/// they rise by at least [`RISE_DB`] while the bars before them stay flat
/// (under half of that); of several such splits the one with the highest
/// novelty, less the [`length_cost`] of both parts, wins.
fn split_builds(bounds: &[usize], feats: &[BarFeat], nov: &[f64]) -> Vec<usize> {
    let (kick_ref, low_ref) = references(feats);
    let kick_on = |s: &SegFeat| s.kick >= kick_ref - KICK_MARGIN_DB;
    let body = |s: &SegFeat| kick_on(s) && s.low >= low_ref - BASS_MARGIN_DB;
    let mut out = vec![0];
    for (i, w) in bounds.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let next = bounds.get(i + 2).map(|&c| seg_feat(&feats[b..c]));
        let this = seg_feat(&feats[a..b]);
        if !kick_on(&this) && next.is_some_and(|n| body(&n)) {
            let best = [4usize, 8, 16]
                .into_iter()
                .filter(|&l| b - a >= l + MIN_LEN)
                .filter_map(|l| {
                    let tail = seg_feat(&feats[b - l..b]);
                    let head = seg_feat(&feats[a..b - l]);
                    (tail.rise >= RISE_DB && head.rise < 0.5 * RISE_DB)
                        .then(|| (b - l, nov[b - l] - length_cost(l) - length_cost(b - a - l)))
                })
                .max_by(|x, y| x.1.total_cmp(&y.1));
            if let Some((cut, _)) = best {
                out.push(cut);
            }
        }
        out.push(b);
    }
    out
}

/// A margin mapped to a confidence: 0.5 on the threshold, 1 at `scale`
/// beyond it, 0 at `scale` short of it.
fn sure(margin: f64, scale: f64) -> f64 {
    (0.5 + 0.5 * margin / scale).clamp(0.0, 1.0)
}

/// Labels the segments `bounds` and returns, per segment, its kind and how
/// sure the label is (0..=1).
fn label(bounds: &[usize], feats: &[BarFeat], dist: &[Vec<f64>]) -> Vec<(PhraseKind, f64)> {
    use PhraseKind::*;
    let n = bounds.len() - 1;
    let segs: Vec<SegFeat> = bounds
        .windows(2)
        .map(|w| seg_feat(&feats[w[0]..w[1]]))
        .collect();
    let (kick_ref, low_ref) = references(feats);
    // Margins (dB) beyond the presence thresholds.
    let kick_m: Vec<f64> = segs
        .iter()
        .map(|s| s.kick - (kick_ref - KICK_MARGIN_DB))
        .collect();
    let bass_m: Vec<f64> = segs
        .iter()
        .map(|s| s.low - (low_ref - BASS_MARGIN_DB))
        .collect();
    #[cfg(test)]
    if std::env::var("OD_PHRASE_DEBUG").is_ok() {
        for (i, w) in bounds.windows(2).enumerate() {
            eprintln!(
                "seg {:>3}..{:<3} e {:6.1} kick_m {:6.1} bass_m {:6.1} rise {:5.1}",
                w[0], w[1], segs[i].energy, kick_m[i], bass_m[i], segs[i].rise
            );
        }
    }
    let kick_on = |i: usize| kick_m[i] >= 0.0;
    let body = |i: usize| kick_on(i) && bass_m[i] >= 0.0;
    let mut kinds: Vec<Option<(PhraseKind, f64)>> = vec![None; n];
    if n < 3 {
        // Too little to tell the ends from the middle.
        return (0..n)
            .map(|i| {
                if body(i) {
                    (Verse, 0.4)
                } else if kick_on(i) {
                    (Intro, 0.4)
                } else {
                    (Down, 0.4)
                }
            })
            .collect();
    }
    // Ends: the first and last phrase, and the beat-led phrases (kick
    // without bass) next to them. Before the beat has started at all, a
    // phrase without kick is intro too (there is nothing to break down
    // from), unless it is the build into the first beat. At the end, a
    // tail without any kick is outro.
    let beat_led = |i: usize| kick_on(i) && bass_m[i] < 0.0;
    // An end phrase with bass is intro or outro by its place alone: unsure.
    let end_sure = |i: usize| sure(kick_m[i].min(-bass_m[i]), 6.0).max(0.4);
    kinds[0] = Some((Intro, end_sure(0)));
    let rising_into_body = |i: usize| segs[i].rise >= RISE_DB && i + 1 < n && body(i + 1);
    let mut i = 1;
    while i < n - 1
        && (beat_led(i) || (!kick_on(i) && (0..i).all(|k| !kick_on(k)) && !rising_into_body(i)))
    {
        kinds[i] = Some((Intro, end_sure(i)));
        i += 1;
    }
    let intro_end = i;
    kinds[n - 1] = Some((Outro, end_sure(n - 1)));
    let mut j = n - 1;
    while j > intro_end && (beat_led(j - 1) || (j - 1..n).all(|k| !kick_on(k))) {
        j -= 1;
        kinds[j] = Some((Outro, end_sure(j)));
    }
    let outro_start = j;
    let middle = intro_end..outro_start;
    // Breakdowns and builds: no kick in the middle. A build rises and the
    // beat comes back right after it; a phrase with the kick on counts as a
    // build only when it rises twice as much.
    for i in middle.clone() {
        let next_body = i + 1 < n && body(i + 1);
        let rising = segs[i].rise >= RISE_DB;
        if !kick_on(i) {
            kinds[i] = Some(if next_body && rising {
                (
                    Up,
                    sure(segs[i].rise - RISE_DB, RISE_DB).min(sure(-kick_m[i], 6.0)),
                )
            } else {
                (Down, sure(-kick_m[i], 6.0))
            });
        } else if next_body && segs[i].rise >= 2.0 * RISE_DB && segs[i + 1].energy > segs[i].energy
        {
            kinds[i] = Some((Up, sure(segs[i].rise - 2.0 * RISE_DB, RISE_DB).min(0.7)));
        } else if !body(i) {
            // Kick but no bass in the middle: a thin breakdown.
            kinds[i] = Some((Down, sure(-bass_m[i], 6.0)));
        }
    }
    // A build is the whole rising stretch before the beat: a phrase without
    // kick right before a build that rises as much itself belongs to it.
    for i in middle.clone().rev() {
        if matches!(kinds[i], Some((Up, _)))
            && i > intro_end
            && matches!(kinds[i - 1], Some((Down, _)))
            && segs[i - 1].rise >= RISE_DB
        {
            let s = kinds[i].map_or(0.5, |k| k.1);
            kinds[i - 1] = Some((Up, s.min(sure(segs[i - 1].rise - RISE_DB, RISE_DB))));
        }
    }
    // Drops: beat-led phrases with bass that come out of a breakdown or a
    // build and are near the loudest of them.
    let loudest = middle
        .clone()
        .filter(|&i| body(i))
        .map(|i| segs[i].energy)
        .fold(f64::MIN, f64::max);
    let released = |i: usize| i > 0 && matches!(kinds[i - 1], Some((Down | Up, _)));
    let mut drop = vec![false; n];
    let mut drop_sure = vec![0.0; n];
    for i in middle.clone() {
        if kinds[i].is_none() && body(i) && released(i) {
            let margin = segs[i].energy - (loudest - DROP_WITHIN_DB);
            if margin >= 0.0 {
                drop[i] = true;
                drop_sure[i] = sure(margin, DROP_WITHIN_DB)
                    .min(sure(kick_m[i], 6.0))
                    .min(sure(bass_m[i], 6.0))
                    .min(kinds[i - 1].map_or(0.5, |k| k.1.max(0.5)));
            }
        }
    }
    // Phrases that sound like a drop are drops: mean distance to a drop
    // phrase no larger than the drop's own spread plus one step.
    let seg_dist = |a: usize, b: usize| {
        let (ra, rb) = (bounds[a]..bounds[a + 1], bounds[b]..bounds[b + 1]);
        let mut s = 0.0;
        for x in ra.clone() {
            for y in rb.clone() {
                s += dist[x][y];
            }
        }
        s / (ra.len() * rb.len()).max(1) as f64
    };
    let drops: Vec<usize> = (0..n).filter(|&i| drop[i]).collect();
    for i in middle.clone() {
        if kinds[i].is_some() || drop[i] || !body(i) {
            continue;
        }
        if let Some((d, gap)) = drops
            .iter()
            .map(|&d| (d, seg_dist(i, d) - (seg_dist(d, d) + 1.0)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            && gap <= 0.0
            && segs[i].energy >= loudest - DROP_WITHIN_DB
        {
            drop[i] = true;
            drop_sure[i] = sure(-gap, 1.0).min(drop_sure[d]);
        }
    }
    for i in middle.clone() {
        if drop[i] {
            kinds[i] = Some((Chorus, drop_sure[i]));
        }
    }
    // The other beat-led phrases: Verse before the first drop. After it,
    // they do not sound like a drop (else they would be one by now): a
    // Bridge, unless as loud as the drops, when the drop goes on.
    let first_drop = drops.first().copied();
    let drop_energy = drops
        .iter()
        .map(|&d| segs[d].energy)
        .fold(f64::MIN, f64::max);
    for i in middle {
        if kinds[i].is_some() {
            continue;
        }
        kinds[i] = Some(match first_drop {
            Some(d) if i > d => {
                let gap = drops
                    .iter()
                    .map(|&d| seg_dist(i, d) - (seg_dist(d, d) + 1.0))
                    .fold(f64::MAX, f64::min);
                let below = drop_energy - segs[i].energy;
                if below > -BRIDGE_BELOW_DB {
                    (
                        Bridge,
                        sure(gap, 1.0).max(sure(below - BRIDGE_BELOW_DB, BRIDGE_BELOW_DB)),
                    )
                } else {
                    (
                        Chorus,
                        sure(-below - BRIDGE_BELOW_DB, BRIDGE_BELOW_DB).min(0.6),
                    )
                }
            }
            _ => (Verse, sure(kick_m[i].min(bass_m[i]), 6.0)),
        });
    }
    kinds
        .into_iter()
        .map(|k| k.unwrap_or((Verse, 0.0)))
        .collect()
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
    let bars = grid_bars(grid, fs, samples.len());
    if bars.len() < 8 {
        return none;
    }
    let frames = frame_features(samples, fs);
    if frames.is_empty() {
        return none;
    }
    let all = bar_features(&frames, &bars);
    let max_power = all.iter().map(|b| b.power).fold(0.0, f64::max);
    if max_power <= 1e-10 {
        return none;
    }
    // Music range: first to last bar within 30 dB of the loudest bar.
    let audible = |f: &BarFeat| f.power > max_power * 1e-3;
    let Some(first) = all.iter().position(audible) else {
        return none;
    };
    let last = all.iter().rposition(audible).unwrap_or(all.len() - 1);
    let bars = &bars[first..=last];
    let feats = &all[first..=last];
    let nb = feats.len();
    if nb < 8 {
        return none;
    }
    let dist = distances(&vectors(feats));
    let nov = novelty(&dist);
    let bounds = split_builds(&segment(&nov, nb), feats, &nov);
    #[cfg(test)]
    if std::env::var("OD_PHRASE_DEBUG").is_ok() {
        let line: Vec<String> = nov
            .iter()
            .enumerate()
            .map(|(b, v)| format!("{b}:{v:.1}"))
            .collect();
        eprintln!("novelty {}", line.join(" "));
        for (i, f) in feats.iter().enumerate() {
            eprintln!(
                "bar {i:>3} e {:6.1} low {:6.1} kick {:6.1} on {:4.2} br {:4.2} bands {:?}",
                f.energy,
                f.low,
                f.kick,
                f.onsets,
                f.bright,
                f.bands.map(|b| b.round() as i32)
            );
        }
    }
    let labels = label(&bounds, feats, &dist);

    // Same-kind neighbours become one phrase; its confidence weighs the
    // label certainty by length and takes the weaker of its two outer
    // boundaries.
    let boundary_sure = |b: usize| {
        if b == 0 || b >= nb {
            1.0
        } else {
            let evidence = nov[b]
                .max(nov[b - 1] - OFF_PEAK_COST)
                .max(nov[b + 1] - OFF_PEAK_COST);
            sure(evidence - THRESHOLD, 2.0 * THRESHOLD)
        }
    };
    let mut merged: Vec<(usize, usize, PhraseKind, f64)> = Vec::new();
    for (w, &(kind, s)) in bounds.windows(2).zip(&labels) {
        let len = (w[1] - w[0]) as f64;
        match merged.last_mut() {
            Some(m) if m.2 == kind => {
                m.1 = w[1];
                m.3 += s * len;
            }
            _ => merged.push((w[0], w[1], kind, s * len)),
        }
    }
    let beat_of = |bar: usize| -> u32 {
        match bars.get(bar) {
            Some(b) => b.0 as u32,
            None => (bars[nb - 1].0 + 4).min(grid.beats.len()) as u32,
        }
    };
    let phrases: Vec<Phrase> = od_core::normalize_phrases(
        &merged
            .iter()
            .map(|&(a, b, kind, s)| {
                let label_sure = s / (b - a) as f64;
                let edge_sure = boundary_sure(a).min(boundary_sure(b));
                // As sure as the weaker of the two: a clear type does not
                // make up for a boundary that is a guess, nor the other
                // way round.
                let confidence = label_sure.min(edge_sure).clamp(0.0, 1.0);
                Phrase {
                    confidence: Some(((confidence * 100.0).round() / 100.0) as f32),
                    ..Phrase::new(beat_of(a), beat_of(b), kind)
                }
            })
            .collect::<Vec<_>>(),
    );

    // Mood.
    let bpm = grid.dominant_bpm().unwrap_or(120.0);
    let chorus_bars: usize = merged
        .iter()
        .filter(|m| matches!(m.2, PhraseKind::Chorus | PhraseKind::Up))
        .map(|m| m.1 - m.0)
        .sum();
    let chorus_share = chorus_bars as f64 / nb as f64;
    // Bass weight: the low end (below LOW_HZ) against the whole, in a
    // typical bar (the median, so breakdowns do not count).
    let low_end = |f: &BarFeat| db(10f64.powf(f.bands[0] / 10.0) + 10f64.powf(f.bands[1] / 10.0));
    let weights: Vec<f64> = feats.iter().map(|f| low_end(f) - f.energy).collect();
    let mean_bass = percentile(&weights, 0.5);
    let energies: Vec<f64> = feats.iter().map(|f| f.energy).collect();
    let range = percentile(&energies, 0.9) - percentile(&energies, 0.1);
    let mood = if bpm >= 118.0 && chorus_share >= 0.3 && mean_bass > -8.0 {
        PhraseMood::High
    } else if bpm < 100.0 || (chorus_share < 0.15 && range >= 3.0) {
        PhraseMood::Low
    } else {
        PhraseMood::Mid
    };
    (phrases, mood)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phrase_eval::{FS, cases, render, score, truth};

    #[test]
    fn edm_arrangement_is_segmented_and_labelled() {
        let case = &cases()[0];
        let (x, grid) = render(case, 7);
        let (phrases, mood) = detect_phrases(&x, FS, &grid);
        assert!(!phrases.is_empty());
        // Contiguous and bar aligned.
        for w in phrases.windows(2) {
            assert_eq!(w[0].end_beat, w[1].start_beat);
        }
        for p in &phrases {
            assert_eq!(p.start_beat % 4, 0);
            let c = p.confidence.expect("analysed phrases carry a confidence");
            assert!((0.0..=1.0).contains(&c));
        }
        let s = score(&truth(case), &phrases);
        assert_eq!(s.hits_1, s.true_bounds, "{phrases:?}");
        assert!(s.label_accuracy() > 0.9, "{phrases:?}");
        assert_eq!(s.fake_drops, 0);
        assert_eq!(phrases[0].kind, PhraseKind::Intro);
        assert_eq!(phrases.last().unwrap().kind, PhraseKind::Outro);
        assert_eq!(mood, PhraseMood::High);
    }

    #[test]
    fn a_track_without_breakdown_has_no_drop() {
        for case in cases().iter().filter(|c| c.name.starts_with("nodrop")) {
            let (x, grid) = render(case, 3);
            let (phrases, _) = detect_phrases(&x, FS, &grid);
            assert!(
                phrases.iter().all(|p| p.kind != PhraseKind::Chorus),
                "{}: {phrases:?}",
                case.name
            );
        }
    }

    #[test]
    fn too_short_or_silent() {
        let grid = Beatgrid::constant(128.0, 0.0, 1, 10.0);
        assert!(
            detect_phrases(&vec![0.1; FS as usize * 10], FS, &grid)
                .0
                .is_empty()
        );
        let grid = Beatgrid::constant(128.0, 0.0, 1, 60.0);
        assert!(
            detect_phrases(&vec![0.0; FS as usize * 60], FS, &grid)
                .0
                .is_empty()
        );
        assert!(detect_phrases(&[], FS, &Beatgrid::default()).0.is_empty());
    }

    #[test]
    fn segmentation_prefers_novel_boundaries_and_regular_lengths() {
        let mut nov = vec![0.1; 65];
        nov[24] = 5.0;
        let b = segment(&nov, 64);
        assert!(b.contains(&24), "{b:?}");
        assert_eq!(*b.last().unwrap(), 64);
        // A weak bump off the 8-bar grid does not make a boundary.
        let mut nov = vec![0.1; 65];
        nov[21] = 1.2;
        assert!(!segment(&nov, 64).contains(&21));
        assert!(length_cost(16) < length_cost(12));
        assert!(length_cost(12) < length_cost(20));
        assert!(length_cost(20) < length_cost(6));
        assert!(length_cost(6) < length_cost(2));
    }

    /// Layers that change by only a few dB and a "drop" that is hardly
    /// louder than the groove: what is found is marked unsure.
    #[test]
    fn subtle_changes_are_unsure() {
        use crate::testsig::{Groove, Mix, Section, render_arrangement};
        let groove = Mix {
            kick: 1.0,
            bass: 0.8,
            hats: 0.6,
            snare: 0.5,
            pad: 0.3,
            ..Mix::default()
        };
        let lift = Mix {
            hats: 0.75,
            pad: 0.4,
            ..groove
        };
        let thin = Mix {
            bass: 0.55,
            hats: 0.45,
            ..groove
        };
        let sections = [
            Section {
                bars: 16,
                from: Mix {
                    bass: 0.5,
                    ..groove
                },
                to: groove,
                harmony: 0,
            },
            Section {
                bars: 32,
                from: groove,
                to: groove,
                harmony: 0,
            },
            Section {
                bars: 8,
                from: thin,
                to: thin,
                harmony: 1,
            },
            Section {
                bars: 32,
                from: lift,
                to: lift,
                harmony: 0,
            },
            Section {
                bars: 16,
                from: groove,
                to: thin,
                harmony: 0,
            },
        ];
        let x = render_arrangement(FS, 126.0, Groove::FourFloor, &sections, 9);
        let grid = Beatgrid::constant(126.0, 0.0, 1, x.len() as f64 / FS as f64);
        let (phrases, _) = detect_phrases(&x, FS, &grid);
        if std::env::var("OD_PHRASE_DEBUG").is_ok() {
            eprintln!("{phrases:?}");
        }
        assert!(phrases.iter().any(|p| p.is_unsure()), "{phrases:?}");
        assert!(
            phrases
                .iter()
                .all(|p| p.kind != PhraseKind::Chorus || p.is_unsure()),
            "{phrases:?}"
        );
    }

    /// Lite mode analyses at about 8 kHz: same structure found.
    #[test]
    fn lite_rate_finds_the_same_structure() {
        use crate::testsig::{Section, render_arrangement};
        let case = &cases()[0];
        let sections: Vec<Section> = case
            .sections
            .iter()
            .map(|&(bars, _, from, to, harmony)| Section {
                bars,
                from,
                to,
                harmony,
            })
            .collect();
        let fs = 8_000;
        let x = render_arrangement(fs, case.bpm, case.groove, &sections, 7);
        let grid = Beatgrid::constant(case.bpm, 0.0, 1, x.len() as f64 / fs as f64);
        let (phrases, _) = detect_phrases(&x, fs, &grid);
        let s = score(&truth(case), &phrases);
        assert!(s.hit_rate_1() >= 0.85, "{phrases:?}");
        assert!(s.label_accuracy() >= 0.85, "{phrases:?}");
        assert_eq!(s.fake_drops, 0);
    }

    #[test]
    fn confidence_maps_margins() {
        assert_eq!(sure(0.0, 3.0), 0.5);
        assert_eq!(sure(3.0, 3.0), 1.0);
        assert_eq!(sure(-6.0, 3.0), 0.0);
    }
}
