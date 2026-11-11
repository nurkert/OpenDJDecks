//! MIDI clock out ([67]): the master deck's tempo and transport as MIDI
//! beat clock (24 pulses per quarter note, Start, Stop) on a chosen MIDI
//! output, so drum machines, sequencers and lighting desks follow the mix.
//!
//! Only the MIDI clock messages of the MIDI 1.0 specification are used; no
//! network tempo protocol and no third-party SDK.
//!
//! * [`ClockModel`] is the schedule, pure and testable: it is told where
//!   the master deck stands (playing, tempo, beat) and answers when the
//!   next pulse is due. Tempo changes take effect at once; small phase
//!   differences are corrected gently, a jump (cue, loop, another master)
//!   is followed at once. Start goes out on the master's next whole beat,
//!   so the receiver's first pulse is a beat.
//! * [`MidiClockOut`] runs a model on its own thread (never the audio
//!   thread) and sends through a [`ClockSink`]: a MIDI port with the
//!   `midi-io` feature, or anything else (tests).

use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Pulses per quarter note (MIDI beat clock).
pub const PPQN: u32 = 24;
pub const CLOCK: u8 = 0xF8;
pub const START: u8 = 0xFA;
pub const STOP: u8 = 0xFC;

/// Where the master deck stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockSource {
    pub playing: bool,
    /// Tempo in BPM (ignored at or below 20).
    pub bpm: f64,
    /// Beat position (beats since the grid's first beat).
    pub beat: f64,
}

/// Phase differences below this many beats are corrected gently, larger
/// ones (a jump) at once.
const SNAP_BEATS: f64 = 0.25;
/// Share of a small phase difference taken per update.
const PULL: f64 = 0.1;

/// The clock's schedule (time in seconds from any origin).
#[derive(Clone, Debug, Default)]
pub struct ClockModel {
    running: bool,
    /// Start is sent with the next pulse.
    start_pending: bool,
    anchor_t: f64,
    anchor_beat: f64,
    bpm: f64,
    /// Index of the next pulse (beat × [`PPQN`]).
    next: i64,
}

impl ClockModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// The beat the clock stands on at `t`.
    pub fn beat_at(&self, t: f64) -> f64 {
        self.anchor_beat + (t - self.anchor_t) * self.bpm / 60.0
    }

    /// Follows the master at `now`. Returns [`STOP`] when the master stopped
    /// (to send at once); Start goes out with the first pulse
    /// ([`ClockModel::take_pulse`]).
    pub fn update(&mut self, now: f64, src: ClockSource) -> Option<u8> {
        let playing = src.playing && src.bpm > 20.0 && src.bpm.is_finite() && src.beat.is_finite();
        if !playing {
            let was = self.running;
            self.running = false;
            self.start_pending = false;
            return was.then_some(STOP);
        }
        if !self.running {
            self.running = true;
            self.start_pending = true;
            self.anchor_t = now;
            self.anchor_beat = src.beat;
            self.bpm = src.bpm;
            // The first pulse on the next whole beat.
            self.next = (src.beat.ceil() * PPQN as f64) as i64;
            return None;
        }
        // Tempo: re-anchor where the clock stands now, at the new tempo.
        let here = self.beat_at(now);
        self.anchor_t = now;
        self.anchor_beat = here;
        self.bpm = src.bpm;
        let err = src.beat - here;
        if err.abs() > SNAP_BEATS {
            // A jump: follow at once. Pulses continue from the new place
            // (never twice the same index, never a burst).
            self.anchor_beat = src.beat;
            if self.start_pending {
                self.next = (src.beat.ceil() * PPQN as f64) as i64;
            } else {
                self.next = (src.beat * PPQN as f64).ceil() as i64;
            }
        } else {
            self.anchor_beat += err * PULL;
        }
        None
    }

    /// When the next pulse is due, while running.
    pub fn next_due(&self) -> Option<f64> {
        self.running.then(|| {
            self.anchor_t + (self.next as f64 / PPQN as f64 - self.anchor_beat) * 60.0 / self.bpm
        })
    }

    /// The messages of the pulse due now (Start first, if pending) and
    /// moves on. A pulse more than one period late is skipped forward to
    /// `now` rather than sent in a burst.
    pub fn take_pulse(&mut self, now: f64) -> ([u8; 2], usize) {
        let mut out = [CLOCK, 0];
        let mut n = 1;
        if self.start_pending {
            out = [START, CLOCK];
            n = 2;
            self.start_pending = false;
        }
        self.next += 1;
        let period = 60.0 / self.bpm / PPQN as f64;
        if let Some(due) = self.next_due()
            && due < now - period
        {
            self.next = (self.beat_at(now) * PPQN as f64).ceil() as i64;
        }
        (out, n)
    }
}

/// Where the clock's bytes go.
pub trait ClockSink: Send {
    fn send(&mut self, bytes: &[u8]);
}

impl<F: FnMut(&[u8]) + Send> ClockSink for F {
    fn send(&mut self, bytes: &[u8]) {
        self(bytes)
    }
}

enum Msg {
    Source(ClockSource, Instant),
    Quit,
}

/// MIDI clock on its own thread. Dropping it sends Stop (if running) and
/// ends the thread.
pub struct MidiClockOut {
    tx: Sender<Msg>,
    thread: Option<JoinHandle<()>>,
    pulses: Arc<AtomicU64>,
    port: String,
}

/// Asks the scheduler to treat the clock thread as time-critical
/// (macOS: the user-interactive QoS class; elsewhere the default).
fn timing_priority() {
    #[cfg(target_os = "macos")]
    // SAFETY: sets the calling thread's own QoS class; no pointers.
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
}

/// Time before a pulse the thread stops waiting for messages (a timed
/// wait can wake several milliseconds late) and sleeps in short steps,
/// then waits actively for the last [`SPIN_LAST`].
const SPIN: Duration = Duration::from_micros(5000);
const SPIN_LAST: f64 = 0.0012;

impl MidiClockOut {
    /// Runs the clock into `sink` (named `port` for the views).
    pub fn start(port: impl Into<String>, mut sink: Box<dyn ClockSink>) -> Self {
        let (tx, rx): (Sender<Msg>, Receiver<Msg>) = crossbeam_channel::unbounded();
        let pulses = Arc::new(AtomicU64::new(0));
        let counted = pulses.clone();
        let thread = std::thread::Builder::new()
            .name("od-midi-clock".into())
            .spawn(move || {
                timing_priority();
                let epoch = Instant::now();
                let secs = |t: Instant| t.saturating_duration_since(epoch).as_secs_f64();
                let mut model = ClockModel::new();
                loop {
                    // Sleep until shortly before the next pulse, or a message.
                    let wait = match model.next_due() {
                        Some(due) => {
                            let left = due - secs(Instant::now());
                            Duration::from_secs_f64(left.max(0.0)).saturating_sub(SPIN)
                        }
                        None => Duration::from_millis(100),
                    };
                    let msg = if wait.is_zero() {
                        rx.try_recv().ok()
                    } else {
                        match rx.recv_timeout(wait.min(Duration::from_millis(100))) {
                            Ok(m) => Some(m),
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => None,
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => Some(Msg::Quit),
                        }
                    };
                    match msg {
                        Some(Msg::Quit) => {
                            if model.running() {
                                sink.send(&[STOP]);
                            }
                            return;
                        }
                        Some(Msg::Source(src, at)) => {
                            if let Some(b) = model.update(secs(at), src) {
                                sink.send(&[b]);
                            }
                            continue;
                        }
                        None => {}
                    }
                    let Some(due) = model.next_due() else { continue };
                    // The last stretch: short sleeps, then wait actively for
                    // the exact time (a yield can give the core away for a
                    // whole time slice).
                    loop {
                        let left = due - secs(Instant::now());
                        if left <= 0.0 {
                            break;
                        }
                        if left > SPIN_LAST {
                            std::thread::sleep(Duration::from_micros(300));
                        } else {
                            std::hint::spin_loop();
                        }
                    }
                    let (bytes, n) = model.take_pulse(secs(Instant::now()));
                    for b in &bytes[..n] {
                        sink.send(&[*b]);
                    }
                    counted.fetch_add(1, Ordering::Relaxed);
                }
            })
            .expect("midi clock thread");
        Self {
            tx,
            thread: Some(thread),
            pulses,
            port: port.into(),
        }
    }

    /// Opens the MIDI output `port` (exact name) for the clock.
    #[cfg(feature = "midi-io")]
    pub fn open(port: &str) -> Result<Self, String> {
        let mut conn = crate::io::connect_output_named(port, "opendjdecks-clock")?;
        Ok(Self::start(
            port,
            Box::new(move |b: &[u8]| {
                let _ = conn.send(b);
            }),
        ))
    }

    /// Where the master deck stands now.
    pub fn update(&self, src: ClockSource) {
        let _ = self.tx.send(Msg::Source(src, Instant::now()));
    }

    /// Pulses sent so far.
    pub fn pulses(&self) -> u64 {
        self.pulses.load(Ordering::Relaxed)
    }

    pub fn port(&self) -> &str {
        &self.port
    }
}

impl Drop for MidiClockOut {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Quit);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn src(playing: bool, bpm: f64, beat: f64) -> ClockSource {
        ClockSource { playing, bpm, beat }
    }

    /// Runs the model in simulated time from `t0` to `t1`, feeding the
    /// source `f(t)` every 16 ms (a UI frame), and returns the pulse times.
    fn simulate(model: &mut ClockModel, t0: f64, t1: f64, f: impl Fn(f64) -> ClockSource) -> Vec<(f64, Vec<u8>)> {
        let mut out = Vec::new();
        let mut next_update = t0;
        let mut t = t0;
        while t < t1 {
            let due = model.next_due().unwrap_or(f64::INFINITY);
            if next_update <= due {
                t = next_update;
                if let Some(b) = model.update(t, f(t)) {
                    out.push((t, vec![b]));
                }
                next_update += 0.016;
            } else {
                t = due;
                let (b, n) = model.take_pulse(t);
                out.push((t, b[..n].to_vec()));
            }
        }
        out
    }

    #[test]
    fn twenty_four_pulses_per_beat_and_start_on_a_beat() {
        let mut m = ClockModel::new();
        // The master plays at 120 BPM from beat 3.4 at t = 0.
        let ev = simulate(&mut m, 0.0, 4.0, |t| src(true, 120.0, 3.4 + t * 2.0));
        let first = &ev[0];
        assert_eq!(first.1, vec![START, CLOCK], "Start, then the first pulse");
        // Beat 4 is 0.3 s after beat 3.4 at 120 BPM.
        assert!((first.0 - 0.3).abs() < 1e-9, "on the next whole beat: {}", first.0);
        let pulses: Vec<f64> = ev.iter().filter(|e| e.1.contains(&CLOCK)).map(|e| e.0).collect();
        // From 0.3 s to 4 s: 3.7 s × 2 beats/s × 24.
        assert_eq!(pulses.len(), (3.7f64 * 2.0 * 24.0).ceil() as usize);
        for w in pulses.windows(2) {
            assert!(((w[1] - w[0]) - 0.5 / 24.0).abs() < 1e-9);
        }
    }

    #[test]
    fn follows_tempo_changes_and_stops() {
        let mut m = ClockModel::new();
        // 120 BPM for 2 s, then 128 BPM, then stop at 4 s.
        let beat = |t: f64| if t < 2.0 { t * 2.0 } else { 4.0 + (t - 2.0) * 128.0 / 60.0 };
        let ev = simulate(&mut m, 0.0, 5.0, |t| src(t < 4.0, if t < 2.0 { 120.0 } else { 128.0 }, beat(t)));
        let pulses: Vec<f64> = ev.iter().filter(|e| e.1.contains(&CLOCK)).map(|e| e.0).collect();
        let late: Vec<f64> = pulses.iter().copied().filter(|t| *t > 2.1 && *t < 3.9).collect();
        for w in late.windows(2) {
            let period = w[1] - w[0];
            assert!((period - 60.0 / 128.0 / 24.0).abs() < 1e-6, "follows 128 BPM: {period}");
        }
        assert_eq!(ev.iter().filter(|e| e.1 == vec![STOP]).count(), 1);
        assert!(pulses.iter().all(|t| *t <= 4.0 + 1e-9), "nothing after Stop");
        // The clock stays on the master's beat across the change.
        let at = late[late.len() / 2];
        let k = pulses.iter().position(|t| *t == at).unwrap() as f64;
        assert!((k / 24.0 - beat(at)).abs() < 0.01, "pulse {k} at beat {}", beat(at));
    }

    /// Reported beats with UI-frame noise (±3 ms): the pulses stay even
    /// (the phase correction is gentle) and on the beat on average.
    #[test]
    fn noisy_reports_keep_the_pulses_even() {
        let mut m = ClockModel::new();
        let noise = |t: f64| ((t * 1000.0).sin() * 0.003) * 2.0; // ±3 ms in beats at 120
        let ev = simulate(&mut m, 0.0, 6.0, |t| src(true, 120.0, t * 2.0 + noise(t)));
        let pulses: Vec<f64> = ev.iter().filter(|e| e.1.contains(&CLOCK)).map(|e| e.0).collect();
        let nominal = 0.5 / 24.0;
        for w in pulses[24..].windows(2) {
            let jitter = (w[1] - w[0] - nominal).abs();
            assert!(jitter < 0.001, "jitter {:.3} ms", jitter * 1000.0);
        }
    }

    #[test]
    fn a_jump_is_followed_without_a_burst() {
        let mut m = ClockModel::new();
        let beat = |t: f64| if t < 1.0 { t * 2.0 } else { 32.0 + (t - 1.0) * 2.0 };
        let ev = simulate(&mut m, 0.0, 2.0, |t| src(true, 120.0, beat(t)));
        let pulses: Vec<f64> = ev.iter().filter(|e| e.1.contains(&CLOCK)).map(|e| e.0).collect();
        for w in pulses.windows(2) {
            assert!(w[1] - w[0] > 0.5 / 24.0 * 0.5, "no burst");
        }
    }

    /// The real thread: 24 ppqn at 120 BPM within a bounded jitter, then
    /// 140 BPM, then Stop.
    #[test]
    fn the_clock_thread_keeps_time() {
        let log: Arc<Mutex<Vec<(Instant, u8)>>> = Arc::default();
        let l2 = log.clone();
        let clock = MidiClockOut::start(
            "test",
            Box::new(move |b: &[u8]| l2.lock().unwrap().push((Instant::now(), b[0]))),
        );
        let t0 = Instant::now();
        let feed = |bpm: f64, beat0: f64, from: Instant, secs: f64| {
            let end = from + Duration::from_secs_f64(secs);
            while Instant::now() < end {
                let t = Instant::now().duration_since(from).as_secs_f64();
                clock.update(src(true, bpm, beat0 + t * bpm / 60.0));
                std::thread::sleep(Duration::from_millis(16));
            }
        };
        feed(120.0, 0.0, t0, 1.5);
        let t1 = Instant::now();
        let beat1 = t1.duration_since(t0).as_secs_f64() * 2.0;
        feed(140.0, beat1, t1, 1.0);
        clock.update(src(false, 140.0, 0.0));
        std::thread::sleep(Duration::from_millis(50));
        let pulses_sent = clock.pulses();
        drop(clock);
        let log = log.lock().unwrap();
        assert_eq!(log.iter().filter(|e| e.1 == START).count(), 1);
        assert_eq!(log.iter().filter(|e| e.1 == STOP).count(), 1);
        let pulses: Vec<Instant> = log.iter().filter(|e| e.1 == CLOCK).map(|e| e.0).collect();
        assert_eq!(pulses.len() as u64, pulses_sent);
        let period = |a: &Instant, b: &Instant| b.duration_since(*a).as_secs_f64();
        let check = |from: Instant, to: Instant, bpm: f64| {
            let p: Vec<&Instant> = pulses.iter().filter(|t| **t > from && **t < to).collect();
            let nominal = 60.0 / bpm / 24.0;
            let mut d: Vec<f64> = p.windows(2).map(|w| (period(w[0], w[1]) - nominal).abs()).collect();
            d.sort_by(f64::total_cmp);
            let median = d[d.len() / 2];
            let p90 = d[d.len() * 9 / 10];
            let worst = d[d.len() - 1];
            eprintln!(
                "{bpm} BPM: {} pulses, jitter median {:.3} ms, 90 % {:.3} ms, worst {:.3} ms",
                p.len(),
                median * 1e3,
                p90 * 1e3,
                worst * 1e3
            );
            // A desktop scheduler now and then holds a thread back for a
            // few milliseconds; the bounds allow that, not a drifting clock.
            assert!(median < 0.001, "median jitter {:.3} ms", median * 1e3);
            assert!(p90 < 0.004, "90 % jitter {:.3} ms", p90 * 1e3);
            assert!(worst < 0.030, "worst jitter {:.3} ms", worst * 1e3);
            // Pulse count over the span ≈ 24 per beat.
            let span = period(p[0], p[p.len() - 1]);
            let expect = span / nominal;
            assert!(((p.len() - 1) as f64 - expect).abs() <= 2.0);
        };
        check(t0 + Duration::from_millis(600), t1, 120.0);
        check(t1 + Duration::from_millis(300), t1 + Duration::from_millis(950), 140.0);
    }
}
