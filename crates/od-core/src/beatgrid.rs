use serde::{Deserialize, Serialize};

/// One beat of a beatgrid.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Beat {
    /// Position in seconds.
    pub time: f64,
    /// Tempo from this beat to the next one.
    pub bpm: f64,
    /// Position inside the bar, 1..=4 (1 is the downbeat).
    pub bar_position: u8,
}

/// A beatgrid as an explicit list of beats. Supports variable tempo.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Beatgrid {
    pub beats: Vec<Beat>,
    /// A locked grid is never replaced by analysis.
    pub locked: bool,
}

impl Beatgrid {
    /// Builds a constant-tempo grid. `first_beat` is the time of a beat with
    /// bar position `first_bar_position`; beats are extended backwards to the
    /// start of the track and forwards to `duration`.
    pub fn constant(bpm: f64, first_beat: f64, first_bar_position: u8, duration: f64) -> Self {
        assert!(bpm > 0.0, "bpm must be positive");
        let period = 60.0 / bpm;
        let steps_back = (first_beat / period).floor() as i64;
        let start = first_beat - steps_back as f64 * period;
        let start_bar = (first_bar_position as i64 - 1 - steps_back).rem_euclid(4);
        let mut beats = Vec::new();
        let mut i: i64 = 0;
        loop {
            let time = start + i as f64 * period;
            if time > duration {
                break;
            }
            beats.push(Beat {
                time,
                bpm,
                bar_position: ((start_bar + i).rem_euclid(4) + 1) as u8,
            });
            i += 1;
        }
        Self {
            beats,
            locked: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.beats.is_empty()
    }

    /// Tempo at the first beat, or `None` for an empty grid.
    pub fn first_bpm(&self) -> Option<f64> {
        self.beats.first().map(|b| b.bpm)
    }

    /// Most common tempo of the grid weighted by duration.
    pub fn dominant_bpm(&self) -> Option<f64> {
        if self.beats.len() < 2 {
            return self.first_bpm();
        }
        let mut buckets: Vec<(i64, f64)> = Vec::new();
        for w in self.beats.windows(2) {
            let key = (w[0].bpm * 100.0).round() as i64;
            let dur = w[1].time - w[0].time;
            match buckets.iter_mut().find(|(k, _)| *k == key) {
                Some((_, d)) => *d += dur,
                None => buckets.push((key, dur)),
            }
        }
        buckets
            .into_iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k as f64 / 100.0)
    }

    /// Index of the last beat at or before `time`.
    pub fn beat_index_at(&self, time: f64) -> Option<usize> {
        match self.beats.partition_point(|b| b.time <= time) {
            0 => None,
            n => Some(n - 1),
        }
    }

    /// Fractional beat number at `time` (0.0 at the first beat). Extrapolates
    /// outside the grid with the nearest tempo.
    pub fn beat_position(&self, time: f64) -> Option<f64> {
        let first = self.beats.first()?;
        match self.beat_index_at(time) {
            None => Some((time - first.time) * first.bpm / 60.0),
            Some(i) => {
                let b = &self.beats[i];
                let period = match self.beats.get(i + 1) {
                    Some(next) => next.time - b.time,
                    None => 60.0 / b.bpm,
                };
                Some(i as f64 + (time - b.time) / period)
            }
        }
    }

    /// Time of a fractional beat number, inverse of [`Self::beat_position`].
    pub fn time_of_beat(&self, beat: f64) -> Option<f64> {
        let first = self.beats.first()?;
        if beat < 0.0 {
            return Some(first.time + beat * 60.0 / first.bpm);
        }
        let i = beat.floor() as usize;
        match self.beats.get(i) {
            Some(b) => {
                let period = match self.beats.get(i + 1) {
                    Some(next) => next.time - b.time,
                    None => 60.0 / b.bpm,
                };
                Some(b.time + beat.fract() * period)
            }
            None => {
                let last = self.beats.last()?;
                let extra = beat - (self.beats.len() - 1) as f64;
                Some(last.time + extra * 60.0 / last.bpm)
            }
        }
    }

    /// Moves every beat by `offset` seconds.
    pub fn shift(&mut self, offset: f64) {
        for b in &mut self.beats {
            b.time += offset;
        }
    }

    /// Index of the beat nearest to `time` (pure).
    pub fn nearest_beat(&self, time: f64) -> Option<usize> {
        let n = self.beats.len();
        if n == 0 {
            return None;
        }
        let i = self.beats.partition_point(|b| b.time < time);
        let candidates = [i.checked_sub(1), (i < n).then_some(i)];
        candidates
            .into_iter()
            .flatten()
            .min_by(|&a, &b| {
                (self.beats[a].time - time)
                    .abs()
                    .total_cmp(&(self.beats[b].time - time).abs())
            })
    }

    /// Moves the whole grid so that the beat nearest `time` sits exactly at
    /// `time` (tempo, markers and bar numbers stay). Returns that beat.
    pub fn align_beat(&mut self, time: f64) -> Option<usize> {
        let i = self.nearest_beat(time)?;
        let d = time - self.beats[i].time;
        self.shift(d);
        Some(i)
    }

    /// First index of the tempo segment (run of beats at one tempo) that
    /// holds beat `i`, and one past its last beat.
    fn segment(&self, i: usize) -> (usize, usize) {
        let same = |a: usize, b: usize| (self.beats[a].bpm - self.beats[b].bpm).abs() < 1e-9;
        let mut s = i;
        while s > 0 && same(s - 1, i) {
            s -= 1;
        }
        let mut e = i + 1;
        while e < self.beats.len() && same(e, i) {
            e += 1;
        }
        (s, e)
    }

    /// Changes the tempo of the segment you look at: the beat nearest
    /// `pivot` stays where it is (and keeps its place in the bar), the
    /// other beats of its tempo segment follow `bpm`. Other segments
    /// (tempo markers) keep their beats; bars are counted on from the
    /// pivot. `duration` bounds a last segment. A grid without beats gets
    /// a constant one through `pivot`.
    pub fn set_bpm_about(&mut self, pivot: f64, bpm: f64, duration: f64) {
        if !(bpm > 0.0) {
            return;
        }
        let Some(k) = self.nearest_beat(pivot) else {
            *self = Beatgrid {
                locked: self.locked,
                ..Beatgrid::constant(bpm, pivot.max(0.0), 1, duration.max(pivot + 1.0))
            };
            return;
        };
        let (s, e) = self.segment(k);
        let period = 60.0 / bpm;
        let anchor = self.beats[k];
        // Bounds of the segment: the beat before it (a marker's
        // predecessor) and the next marker; the track otherwise.
        let lo = if s > 0 { self.beats[s - 1].time + period * 0.5 } else { 0.0 };
        let hi = if e < self.beats.len() {
            self.beats[e].time - period * 0.5
        } else {
            duration.max(anchor.time)
        };
        let back = ((anchor.time - lo) / period).floor().max(0.0) as i64;
        let fwd = ((hi - anchor.time) / period).floor().max(0.0) as i64;
        let mut beats: Vec<Beat> = self.beats[..s].to_vec();
        let pivot_new = beats.len() + back as usize;
        for j in -back..=fwd {
            beats.push(Beat {
                time: anchor.time + j as f64 * period,
                bpm,
                bar_position: 1,
            });
        }
        beats.extend_from_slice(&self.beats[e..]);
        // Bars counted from the pivot for its segment and everything after.
        let base = anchor.bar_position as i64 - 1;
        for (j, b) in beats.iter_mut().enumerate().skip(s) {
            b.bar_position = ((j as i64 - pivot_new as i64 + base).rem_euclid(4) + 1) as u8;
        }
        self.beats = beats;
    }

    /// A grid through pins: each pin `(time, beat)` says beat number
    /// `beat` (counted on the current grid) sits exactly at `time`. Between
    /// two pins the tempo is constant (beats / seconds); before the first
    /// and after the last the neighbouring tempo goes on, a single pin
    /// uses `bpm`. The pin with the lowest beat number has bar position
    /// `first_bar`. `None` without pins or for pins out of order.
    pub fn through_pins(pins: &[(f64, i64)], first_bar: u8, bpm: f64, duration: f64) -> Option<Beatgrid> {
        let mut pins = pins.to_vec();
        pins.sort_by_key(|p| p.1);
        pins.dedup_by_key(|p| p.1);
        let first = *pins.first()?;
        // Period of each span between pins.
        let mut periods = Vec::new();
        for w in pins.windows(2) {
            let p = (w[1].0 - w[0].0) / (w[1].1 - w[0].1) as f64;
            if !(p > 0.0) {
                return None;
            }
            periods.push(p);
        }
        if periods.is_empty() {
            if !(bpm > 0.0) {
                return None;
            }
            periods.push(60.0 / bpm);
        }
        let last = *pins.last()?;
        // Time of beat number `n` and the period that starts there.
        let at = |n: i64| -> (f64, f64) {
            if n <= first.1 {
                let p = periods[0];
                return (first.0 + (n - first.1) as f64 * p, p);
            }
            for (i, w) in pins.windows(2).enumerate() {
                if n < w[1].1 {
                    return (w[0].0 + (n - w[0].1) as f64 * periods[i], periods[i]);
                }
            }
            let p = *periods.last().unwrap_or(&periods[0]);
            (last.0 + (n - last.1) as f64 * p, p)
        };
        let back = (first.0 / periods[0]).floor() as i64;
        let mut n = first.1 - back.max(0);
        let mut beats = Vec::new();
        let base = first_bar.clamp(1, 4) as i64 - 1;
        loop {
            let (t, p) = at(n);
            if t > duration.max(last.0) {
                break;
            }
            if t >= -1e-9 {
                beats.push(Beat {
                    time: t,
                    bpm: 60.0 / p,
                    bar_position: ((n - first.1 + base).rem_euclid(4) + 1) as u8,
                });
            }
            n += 1;
        }
        Some(Beatgrid { beats, locked: false })
    }

    /// Makes the beat at index `i` a downbeat and renumbers all bars.
    pub fn set_downbeat(&mut self, i: usize) {
        for (j, b) in self.beats.iter_mut().enumerate() {
            b.bar_position = ((j as i64 - i as i64).rem_euclid(4) + 1) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_grid_extends_backwards() {
        let g = Beatgrid::constant(120.0, 1.25, 1, 10.0);
        assert!((g.beats[0].time - 0.25).abs() < 1e-9);
        assert_eq!(g.beats[0].bar_position, 3);
        assert_eq!(g.beats[2].bar_position, 1);
        assert!((g.beats[2].time - 1.25).abs() < 1e-9);
        assert_eq!(g.beats.len(), 20);
    }

    #[test]
    fn beat_position_roundtrip() {
        let g = Beatgrid::constant(128.0, 0.1, 1, 60.0);
        for t in [0.0, 0.1, 3.3, 59.0, 70.0] {
            let b = g.beat_position(t).unwrap();
            assert!((g.time_of_beat(b).unwrap() - t).abs() < 1e-9, "t={t}");
        }
        assert_eq!(g.dominant_bpm(), Some(128.0));
    }

    #[test]
    fn set_downbeat_renumbers() {
        let mut g = Beatgrid::constant(120.0, 0.0, 1, 4.0);
        g.set_downbeat(1);
        let bars: Vec<u8> = g.beats.iter().map(|b| b.bar_position).collect();
        assert_eq!(&bars[..6], &[4, 1, 2, 3, 4, 1]);
    }
    #[test]
    fn set_bpm_about_keeps_the_pivot_beat() {
        let mut g = Beatgrid::constant(120.0, 0.25, 3, 60.0);
        let k = g.nearest_beat(30.1).unwrap();
        let (t, bar) = (g.beats[k].time, g.beats[k].bar_position);
        g.set_bpm_about(30.1, 121.0, 60.0);
        let k2 = g.nearest_beat(t).unwrap();
        assert!((g.beats[k2].time - t).abs() < 1e-12, "pivot stays");
        assert_eq!(g.beats[k2].bar_position, bar, "and keeps its place in the bar");
        assert!((g.beats[k2 + 1].time - g.beats[k2].time - 60.0 / 121.0).abs() < 1e-9);
        assert!(g.beats.iter().all(|b| b.bpm == 121.0));
        assert!(g.beats[0].time >= 0.0 && g.beats[0].time < 60.0 / 121.0);
        // Bars count on through the whole grid.
        for w in g.beats.windows(2) {
            assert_eq!(w[1].bar_position, w[0].bar_position % 4 + 1);
        }
    }

    #[test]
    fn set_bpm_about_leaves_other_tempo_segments() {
        // 120 BPM up to 10 s, then a marker at 100 BPM.
        let a = Beatgrid::constant(120.0, 0.0, 1, 9.9);
        let marker = a.beats.last().unwrap().time + 0.5;
        let b = Beatgrid::constant(100.0, marker, 1, 30.0);
        let mut g = Beatgrid {
            beats: a.beats.iter().chain(b.beats.iter().filter(|x| x.time >= marker)).copied().collect(),
            locked: false,
        };
        let after: Vec<f64> = g.beats.iter().filter(|x| x.time >= marker).map(|x| x.time).collect();
        g.set_bpm_about(4.0, 121.0, 30.0);
        let now: Vec<f64> = g.beats.iter().filter(|x| x.time >= marker - 1e-9).map(|x| x.time).collect();
        assert_eq!(now, after, "the marker and its beats survive");
        assert!(g.beats.iter().any(|x| (x.time - 4.0).abs() < 1e-12), "pivot fixed");
    }

    #[test]
    fn through_pins_hits_the_pins() {
        let pins = [(1.234, 0), (40.5, 84), (90.0, 190)];
        let g = Beatgrid::through_pins(&pins, 1, 0.0, 120.0).unwrap();
        for (t, _) in pins {
            assert!(g.beats.iter().any(|b| (b.time - t).abs() < 1e-9), "pin at {t}");
        }
        let at = |t: f64| g.beats.iter().position(|b| (b.time - t).abs() < 1e-9).unwrap();
        assert_eq!(at(40.5) - at(1.234), 84, "beat count between pins kept");
        assert_eq!(g.beats[at(1.234)].bar_position, 1);
        assert!((g.beats[at(1.234)].bpm - 60.0 * 84.0 / (40.5 - 1.234)).abs() < 1e-9);
        // One pin: a constant grid at the given tempo through it.
        let one = Beatgrid::through_pins(&[(2.0, 5)], 3, 120.0, 10.0).unwrap();
        let i = one.nearest_beat(2.0).unwrap();
        assert!((one.beats[i].time - 2.0).abs() < 1e-12);
        assert_eq!(one.beats[i].bar_position, 3);
        assert!(Beatgrid::through_pins(&[], 1, 120.0, 10.0).is_none());
    }

    #[test]
    fn align_beat_moves_the_nearest_beat_onto_the_time() {
        let mut g = Beatgrid::constant(120.0, 0.0, 1, 20.0);
        assert_eq!(g.align_beat(4.02), Some(8));
        assert!((g.beats[8].time - 4.02).abs() < 1e-12);
        assert!((g.beats[0].time - 0.02).abs() < 1e-12);
    }
}
