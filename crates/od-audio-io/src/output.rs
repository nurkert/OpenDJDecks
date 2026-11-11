//! Output stream running the engine.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, OutputCallbackInfo, SampleFormat, SizedSample};
use od_engine::{EngineProcessor, MAX_DECKS, Outputs};

use crate::devices::{Backend, device_name, find_device, host};
use crate::{AudioError, Result};

/// Engine render scratch size; device callbacks larger than this are split.
const SCRATCH_FRAMES: usize = 8192;

/// Which device channels (0-based) receive master, headphone and per-deck
/// stereo ([67]). A channel may carry only one of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputRouting {
    /// `None`: the master is not on this device (an external mixer takes
    /// the decks, or split cue on one stereo output carries it in the
    /// headphone pair).
    pub master: Option<[u16; 2]>,
    /// `None` disables the headphone output (or it plays on its own
    /// device, [`AudioConfig::cue_device`]).
    pub headphones: Option<[u16; 2]>,
    /// Each deck on its own pair (external mixer, [`Outputs::decks`]).
    pub decks: [Option<[u16; 2]>; MAX_DECKS],
}

impl Default for OutputRouting {
    fn default() -> Self {
        Self::auto(2)
    }
}

impl OutputRouting {
    /// Master on 1/2; headphones on 3/4 if the device has at least four
    /// output channels (the layout of 4-output controllers).
    pub fn auto(device_channels: u16) -> Self {
        Self {
            master: Some([0, 1]),
            headphones: (device_channels >= 4).then_some([2, 3]),
            decks: [None; MAX_DECKS],
        }
    }

    /// Pair `n` (0 = outputs 1/2, 1 = 3/4, …).
    pub fn pair(n: u16) -> [u16; 2] {
        [n * 2, n * 2 + 1]
    }

    /// Split cue on one stereo output: the headphone pair (cue mono left,
    /// master mono right, see `MixerCommand::SetSplitCue`) on outputs 1/2,
    /// no separate master.
    pub fn split_on_one() -> Self {
        Self {
            master: None,
            headphones: Some([0, 1]),
            decks: [None; MAX_DECKS],
        }
    }

    /// Every routed pair, named.
    fn routes(&self) -> impl Iterator<Item = (Route, [u16; 2])> + '_ {
        self.master
            .map(|p| (Route::Master, p))
            .into_iter()
            .chain(self.headphones.map(|p| (Route::Headphones, p)))
            .chain(
                self.decks
                    .iter()
                    .enumerate()
                    .filter_map(|(d, p)| p.map(|p| (Route::Deck(d), p))),
            )
    }

    pub fn channels_needed(&self) -> u16 {
        self.routes()
            .map(|(_, p)| p[0].max(p[1]) + 1)
            .max()
            .unwrap_or(2)
    }

    /// Drops the routes the device cannot take (beyond its channels, or a
    /// channel already taken by an earlier route: master, headphones, then
    /// decks 1–4) and returns what was dropped.
    pub fn fit(&mut self, device_channels: u16) -> Vec<Route> {
        let mut used: Vec<u16> = Vec::new();
        let mut dropped = Vec::new();
        let mut keep = |r: Route, p: Option<[u16; 2]>| -> Option<[u16; 2]> {
            let p = p?;
            let ok = p[0] < device_channels
                && p[1] < device_channels
                && p[0] != p[1]
                && !used.contains(&p[0])
                && !used.contains(&p[1]);
            if ok {
                used.extend(p);
                Some(p)
            } else {
                dropped.push(r);
                None
            }
        };
        self.master = keep(Route::Master, self.master);
        self.headphones = keep(Route::Headphones, self.headphones);
        for d in 0..MAX_DECKS {
            self.decks[d] = keep(Route::Deck(d), self.decks[d]);
        }
        dropped
    }

    fn has_decks(&self) -> bool {
        self.decks.iter().any(Option::is_some)
    }
}

/// One routed signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Master,
    Headphones,
    /// Deck index (0-based).
    Deck(usize),
}

/// What to open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioConfig {
    pub backend: Backend,
    /// Device name or id substring (case-insensitive); `None` = default output.
    pub device: Option<String>,
    /// Requested sample rate; `None` = device default (48 kHz preferred).
    /// [`AudioHost::start`] always uses the engine's rate.
    pub sample_rate: Option<u32>,
    /// Requested buffer size in frames; `None` = backend default.
    pub buffer_frames: Option<u32>,
    /// `None` = [`OutputRouting::auto`].
    pub routing: Option<OutputRouting>,
    /// Headphones on a second device ([67]): its pair `cue_pair` (0 = 1/2)
    /// plays the headphone bus, fed from the main device's callback through
    /// a lock-free ring. Both run at the engine's rate; their clocks drift
    /// apart slowly, which the ring absorbs by dropping or repeating a few
    /// frames (a quiet click on the cue now and then, never on the master).
    pub cue_device: Option<String>,
    pub cue_pair: u16,
    /// If the requested device is missing or cannot be opened, use the
    /// default output for master only (headphones disabled).
    pub fallback_to_default: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Default,
            device: None,
            sample_rate: None,
            buffer_frames: None,
            routing: None,
            cue_device: None,
            cue_pair: 0,
            fallback_to_default: true,
        }
    }
}

/// The configuration a device will actually run with.
#[derive(Clone, Debug, PartialEq)]
pub struct Negotiated {
    pub backend: Backend,
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: Option<u32>,
    pub sample_format: String,
    pub routing: OutputRouting,
    /// Routes asked for that this device cannot take.
    pub dropped: Vec<Route>,
    /// True if the requested device was unavailable and the default was used.
    pub fell_back: bool,
    /// The second device playing the headphones, once it runs.
    pub cue_device: Option<String>,
    /// Why the cue device did not start, if it did not.
    pub cue_error: Option<String>,
}

/// Information about a running stream.
pub type StreamInfo = Negotiated;

impl Negotiated {
    /// Nominal output latency from the buffer size (excluding converter /
    /// device latency, see [`AudioStats::output_latency`]).
    pub fn nominal_latency(&self) -> Option<Duration> {
        self.buffer_frames
            .map(|b| Duration::from_secs_f64(b as f64 / self.sample_rate as f64))
    }
}

struct SharedStats {
    /// When the host was created; callback times are counted from here.
    epoch: Instant,
    /// Microseconds from `epoch` to the end of the last callback (0 = none yet).
    last_callback_us: AtomicU64,
    callbacks: AtomicU64,
    xruns: AtomicU64,
    overruns: AtomicU64,
    last_frames: AtomicU32,
    max_frames: AtomicU32,
    latency_ns: AtomicU64,
    load_ppm: AtomicU32,
    peak_load_ppm: AtomicU32,
    device_lost: AtomicBool,
    /// Cue-device callbacks that found the ring short (silence filled in).
    cue_underruns: AtomicU64,
    /// Frames the cue ring dropped or repeated to follow the clock drift.
    cue_adjusts: AtomicU64,
}

impl SharedStats {
    fn new() -> Self {
        Self {
            epoch: Instant::now(),
            last_callback_us: AtomicU64::new(0),
            callbacks: AtomicU64::new(0),
            xruns: AtomicU64::new(0),
            overruns: AtomicU64::new(0),
            last_frames: AtomicU32::new(0),
            max_frames: AtomicU32::new(0),
            latency_ns: AtomicU64::new(0),
            load_ppm: AtomicU32::new(0),
            peak_load_ppm: AtomicU32::new(0),
            device_lost: AtomicBool::new(false),
            cue_underruns: AtomicU64::new(0),
            cue_adjusts: AtomicU64::new(0),
        }
    }

    /// What the backend's error callback does when the device goes away.
    fn report_lost(&self) {
        self.device_lost.store(true, Ordering::Relaxed);
    }
}

/// How long the device may stay silent (no callback) before it counts as
/// lost. Backends do not always report a removed device; a stream that
/// stops calling back is the other sign.
pub const STALL_LIMIT: Duration = Duration::from_millis(1500);

/// What the output is doing, as far as the host can tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    /// Opened, no callback yet.
    Starting,
    Running,
    /// The backend reported the device gone, or callbacks stopped for longer
    /// than [`STALL_LIMIT`].
    Lost,
}

/// Runtime statistics of the output stream.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AudioStats {
    pub callbacks: u64,
    /// Underruns/overruns reported by the backend.
    pub xruns: u64,
    /// Callbacks whose processing took longer than the buffer duration.
    pub overruns: u64,
    /// Frames of the last callback.
    pub callback_frames: u32,
    pub max_callback_frames: u32,
    /// Callback-to-playback delay reported by the backend (actual output
    /// latency), if available.
    pub output_latency: Option<Duration>,
    /// Engine processing time / buffer duration of the last callback (0..1+).
    pub load: f32,
    /// Highest load seen since start.
    pub peak_load: f32,
    pub device_lost: bool,
    /// Time since the last callback ended (`None` before the first).
    pub since_last_callback: Option<Duration>,
    /// Headphones on a second device: callbacks that ran short.
    pub cue_underruns: u64,
    /// … and frames dropped or repeated to follow the clock drift.
    pub cue_adjusts: u64,
}

impl AudioStats {
    /// Dropouts the DJ can hear: backend under/overruns plus callbacks that
    /// took longer than their buffer.
    pub fn dropouts(&self) -> u64 {
        self.xruns + self.overruns
    }

    pub fn health(&self) -> Health {
        match self.since_last_callback {
            _ if self.device_lost => Health::Lost,
            None => Health::Starting,
            Some(d) if d > STALL_LIMIT => Health::Lost,
            Some(_) => Health::Running,
        }
    }
}

struct CallbackState {
    /// Always `Some` until the state is dropped, when it goes back to the
    /// host (see [`AudioHost::stop`]).
    processor: Option<EngineProcessor>,
    give_back: Sender<EngineProcessor>,
    master: Vec<f32>,
    phones: Vec<f32>,
    /// Per-deck outputs, `MAX_DECKS` pairs per frame.
    decks: Vec<f32>,
    /// The headphone bus for a second device, if one plays it.
    cue_tx: Option<rtrb::Producer<f32>>,
    channels: usize,
    routing: OutputRouting,
    sample_rate: f64,
    stats: Arc<SharedStats>,
}

impl Drop for CallbackState {
    fn drop(&mut self) {
        if let Some(p) = self.processor.take() {
            let _ = self.give_back.send(p);
        }
    }
}

/// Writes one rendered block into the device's interleaved frames: every
/// channel silent, then each routed pair. No allocation.
fn route_frames<T: SizedSample + FromSample<f32>>(
    data: &mut [T],
    ch: usize,
    r: &OutputRouting,
    master: &[f32],
    phones: &[f32],
    decks: Option<&[f32]>,
    silence: T,
) {
    let m = r.master.map(|p| p.map(|c| c as usize));
    let hp = r.headphones.map(|p| p.map(|c| c as usize));
    let dp = r.decks.map(|d| d.map(|p| p.map(|c| c as usize)));
    for (i, frame) in data.chunks_exact_mut(ch).enumerate() {
        frame.iter_mut().for_each(|s| *s = silence);
        if let Some([l, rr]) = m {
            frame[l] = T::from_sample(master[i * 2]);
            frame[rr] = T::from_sample(master[i * 2 + 1]);
        }
        if let Some([l, rr]) = hp {
            frame[l] = T::from_sample(phones[i * 2]);
            frame[rr] = T::from_sample(phones[i * 2 + 1]);
        }
        if let Some(d) = decks {
            for (k, p) in dp.iter().enumerate() {
                if let Some([l, rr]) = *p {
                    let j = (i * MAX_DECKS + k) * 2;
                    frame[l] = T::from_sample(d[j]);
                    frame[rr] = T::from_sample(d[j + 1]);
                }
            }
        }
    }
}

/// The receiving end of the headphone ring on a second device. Keeps the
/// ring near `target` samples: it waits until that much is there before it
/// plays, fills silence when the ring runs dry, and drops a frame when the
/// main device runs ahead (more than twice the target waiting) or repeats
/// one when it falls behind (less than half). Never allocates.
pub(crate) struct CueBridge {
    rx: rtrb::Consumer<f32>,
    target: usize,
    primed: bool,
    last: [f32; 2],
    stats: Arc<SharedStats>,
}

impl CueBridge {
    fn new(rx: rtrb::Consumer<f32>, target_frames: usize, stats: Arc<SharedStats>) -> Self {
        Self {
            rx,
            target: target_frames.max(64) * 2,
            primed: false,
            last: [0.0; 2],
            stats,
        }
    }

    /// Fills `out` (interleaved, `ch` channels; the pair `pair` gets the
    /// headphones, the rest silence).
    fn pull<T: SizedSample + FromSample<f32>>(&mut self, out: &mut [T], ch: usize, pair: [usize; 2]) {
        let silence = T::from_sample(0.0f32);
        let frames = out.len() / ch.max(1);
        let waiting = self.rx.slots();
        if !self.primed {
            if waiting < self.target {
                out.iter_mut().for_each(|s| *s = silence);
                return;
            }
            self.primed = true;
        }
        // Clock drift: one frame per callback either way.
        if waiting > self.target * 2 + frames * 2 {
            let _ = self.rx.pop();
            let _ = self.rx.pop();
            self.stats.cue_adjusts.fetch_add(1, Ordering::Relaxed);
        }
        let repeat = self.rx.slots() < self.target / 2;
        if repeat {
            self.stats.cue_adjusts.fetch_add(1, Ordering::Relaxed);
        }
        let mut short = false;
        for (i, frame) in out.chunks_exact_mut(ch.max(1)).enumerate() {
            frame.iter_mut().for_each(|s| *s = silence);
            let lr = if repeat && i == 0 {
                Some(self.last)
            } else {
                match (self.rx.pop(), self.rx.pop()) {
                    (Ok(l), Ok(r)) => Some([l, r]),
                    _ => None,
                }
            };
            match lr {
                Some(v) => {
                    self.last = v;
                    if pair[1] < frame.len() {
                        frame[pair[0]] = T::from_sample(v[0]);
                        frame[pair[1]] = T::from_sample(v[1]);
                    }
                }
                None => short = true,
            }
        }
        if short {
            self.stats.cue_underruns.fetch_add(1, Ordering::Relaxed);
            self.primed = false;
        }
    }
}

impl CallbackState {
    fn render<T: SizedSample + FromSample<f32>>(
        &mut self,
        data: &mut [T],
        info: &OutputCallbackInfo,
    ) {
        let ts = info.timestamp();
        let lat = ts.playback.saturating_duration_since(ts.callback);
        self.render_frames(data, Some(lat));
    }

    fn render_frames<T: SizedSample + FromSample<f32>>(
        &mut self,
        data: &mut [T],
        latency: Option<Duration>,
    ) {
        let t0 = Instant::now();
        let Some(processor) = self.processor.as_mut() else {
            return;
        };
        let ch = self.channels;
        let frames = data.len() / ch;
        let silence = T::from_sample(0.0f32);
        let r = self.routing;
        let hp_enabled = r.headphones.is_some() || self.cue_tx.is_some();
        let decks_enabled = r.has_decks();
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(SCRATCH_FRAMES);
            {
                let mut out = Outputs {
                    master: &mut self.master[..n * 2],
                    headphones: if hp_enabled {
                        Some(&mut self.phones[..n * 2])
                    } else {
                        None
                    },
                    booth: None,
                    decks: if decks_enabled {
                        Some(&mut self.decks[..n * 2 * MAX_DECKS])
                    } else {
                        None
                    },
                };
                processor.process(n, &mut out);
            }
            route_frames(
                &mut data[done * ch..(done + n) * ch],
                ch,
                &r,
                &self.master[..n * 2],
                &self.phones[..n * 2],
                decks_enabled.then(|| &self.decks[..n * 2 * MAX_DECKS]),
                silence,
            );
            if let Some(tx) = self.cue_tx.as_mut() {
                // A full ring (the cue device stopped) just drops.
                let _ = tx.push_partial_slice(&self.phones[..n * 2]);
            }
            done += n;
        }
        // Statistics (atomics only).
        let s = &self.stats;
        s.callbacks.fetch_add(1, Ordering::Relaxed);
        s.last_frames.store(frames as u32, Ordering::Relaxed);
        s.max_frames.fetch_max(frames as u32, Ordering::Relaxed);
        if let Some(lat) = latency {
            s.latency_ns.store(lat.as_nanos() as u64, Ordering::Relaxed);
        }
        let budget = frames as f64 / self.sample_rate;
        if budget > 0.0 {
            let load = t0.elapsed().as_secs_f64() / budget;
            let ppm = (load * 1e6) as u32;
            s.load_ppm.store(ppm, Ordering::Relaxed);
            s.peak_load_ppm.fetch_max(ppm, Ordering::Relaxed);
            if load > 1.0 {
                s.overruns.fetch_add(1, Ordering::Relaxed);
            }
        }
        let us = s.epoch.elapsed().as_micros() as u64;
        s.last_callback_us.store(us.max(1), Ordering::Relaxed);
    }
}

/// A running output stream driving the engine. Dropping it stops audio;
/// [`AudioHost::stop`] stops it and hands the engine processor back, so the
/// same engine (decks, positions, mixer) can go on on another device.
///
/// Note: on some backends the underlying stream must be dropped on the
/// thread that created it; keep the `AudioHost` on that thread.
pub struct AudioHost {
    driver: Driver,
    /// The headphones' own device, if they play on one.
    #[allow(dead_code)]
    cue_stream: Option<cpal::Stream>,
    info: StreamInfo,
    stats: Arc<SharedStats>,
    returned: Receiver<EngineProcessor>,
}

enum Driver {
    // Held so the stream keeps running; dropping it stops audio.
    Cpal(#[allow(dead_code)] cpal::Stream),
    Virtual(VirtualThread),
}

/// A failed start, with the processor when it could be recovered.
pub struct StartError {
    pub error: AudioError,
    pub processor: Option<EngineProcessor>,
}

impl std::fmt::Debug for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.error)
    }
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

/// A device without hardware: a thread that runs the engine in real time
/// and discards what it renders. Used where no audio is wanted (headless
/// tests, machines without a device) and to test device loss: its
/// [`FaultHook`] makes it vanish the way an unplugged interface does.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualOutput {
    pub name: String,
    pub sample_rate: u32,
    /// Frames per callback.
    pub block: usize,
    pub channels: u16,
    /// `None` = [`OutputRouting::auto`].
    pub routing: Option<OutputRouting>,
}

impl VirtualOutput {
    pub fn new(name: impl Into<String>, sample_rate: u32) -> Self {
        Self {
            name: name.into(),
            sample_rate,
            block: 480,
            channels: 4,
            routing: None,
        }
    }
}

struct VirtualThread {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    unplugged: Arc<AtomicBool>,
}

impl Drop for VirtualThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Injects device faults into a running host (tests and diagnostics).
#[derive(Clone)]
pub struct FaultHook {
    stats: Arc<SharedStats>,
    unplugged: Option<Arc<AtomicBool>>,
}

impl FaultHook {
    /// The device disappears: the backend's "device not available" report
    /// is raised and, on a virtual device, callbacks stop as they do when
    /// the cable is pulled. A real stream keeps running until the app
    /// reacts to the report.
    pub fn lose_device(&self) {
        if let Some(u) = &self.unplugged {
            u.store(true, Ordering::Relaxed);
        }
        self.stats.report_lost();
    }

    /// Callbacks stop without any report (a stalled driver). Only a
    /// virtual device can do this; the stall shows as [`Health::Lost`]
    /// after [`STALL_LIMIT`].
    pub fn stall(&self) {
        if let Some(u) = &self.unplugged {
            u.store(true, Ordering::Relaxed);
        }
    }

    /// One reported xrun (a dropout).
    pub fn xrun(&self) {
        self.stats.xruns.fetch_add(1, Ordering::Relaxed);
    }
}

fn pick_device(cfg: &AudioConfig) -> Result<(cpal::Device, bool)> {
    let h = host(cfg.backend)?;
    if let Some(name) = cfg.device.as_ref().filter(|n| !n.trim().is_empty()) {
        if let Some(d) = find_device(&h, name, true) {
            return Ok((d, false));
        }
        if !cfg.fallback_to_default {
            return Err(AudioError::DeviceNotFound(name.clone()));
        }
        tracing::warn!(device = %name, "audio device not found, falling back to default output");
        let d = h
            .default_output_device()
            .ok_or_else(|| AudioError::DeviceNotFound("default".into()))?;
        return Ok((d, true));
    }
    let d = h
        .default_output_device()
        .ok_or_else(|| AudioError::DeviceNotFound("default".into()))?;
    Ok((d, false))
}

fn format_rank(f: SampleFormat) -> u8 {
    match f {
        SampleFormat::F32 => 0,
        SampleFormat::I32 => 1,
        SampleFormat::I24 => 2,
        SampleFormat::I16 => 3,
        SampleFormat::F64 => 4,
        SampleFormat::U16 => 5,
        _ => 9,
    }
}

fn negotiate_on(
    cfg: &AudioConfig,
    device: &cpal::Device,
    fell_back: bool,
    force_rate: Option<u32>,
) -> Result<(Negotiated, cpal::StreamConfig, SampleFormat)> {
    let name = device_name(device);
    let ranges: Vec<_> = device.supported_output_configs()?.collect();
    let max_ch = ranges.iter().map(|r| r.channels()).max().unwrap_or(0);
    let mut routing = if fell_back {
        OutputRouting {
            master: Some([0, 1]),
            headphones: None,
            decks: [None; MAX_DECKS],
        }
    } else {
        cfg.routing.unwrap_or_else(|| OutputRouting::auto(max_ch))
    };
    // Routes beyond the device's outputs (e.g. headphones on 3/4 of a
    // stereo device) are left out and reported.
    let dropped = routing.fit(max_ch);
    if !dropped.is_empty() {
        tracing::warn!(device = %name, ?dropped, "device has {max_ch} outputs; routes left out");
    }
    if routing.master.is_none() && routing.headphones.is_none() && !routing.has_decks() {
        // Nothing left: at least the master on 1/2.
        routing.master = Some([0, 1]);
    }
    let needed = routing.channels_needed().max(2);
    let default_rate = device.default_output_config().ok().map(|c| c.sample_rate());
    let rate = force_rate
        .or(cfg.sample_rate)
        .or(default_rate.filter(|r| *r == 44_100 || *r == 48_000))
        .unwrap_or(48_000);
    let best = ranges
        .iter()
        .filter(|r| {
            r.channels() >= needed && r.contains_rate(rate) && format_rank(r.sample_format()) < 9
        })
        .min_by_key(|r| (r.channels(), format_rank(r.sample_format())))
        .ok_or_else(|| AudioError::UnsupportedConfig {
            device: name.clone(),
            sample_rate: rate,
            channels: needed,
        })?;
    let buffer = match (cfg.buffer_frames, best.buffer_size()) {
        (Some(b), cpal::SupportedBufferSize::Range { min, max }) => Some(b.clamp(*min, *max)),
        (Some(b), _) => Some(b),
        (None, _) => None,
    };
    let stream_cfg = cpal::StreamConfig {
        channels: best.channels(),
        sample_rate: rate,
        buffer_size: buffer.map_or(cpal::BufferSize::Default, cpal::BufferSize::Fixed),
    };
    let resolved_backend = cfg.backend;
    Ok((
        Negotiated {
            backend: resolved_backend,
            device_name: name,
            sample_rate: rate,
            channels: best.channels(),
            buffer_frames: buffer,
            sample_format: format!("{:?}", best.sample_format()),
            routing,
            dropped,
            fell_back,
            cue_device: None,
            cue_error: None,
        },
        stream_cfg,
        best.sample_format(),
    ))
}

impl AudioHost {
    /// Resolves device, channel count, sample rate and buffer size without
    /// opening a stream. Create the engine with the returned sample rate.
    pub fn negotiate(cfg: &AudioConfig) -> Result<Negotiated> {
        let (dev, fell_back) = pick_device(cfg)?;
        negotiate_on(cfg, &dev, fell_back, None).map(|(n, _, _)| n)
    }

    /// Opens the output device and starts running `processor` in its
    /// callback. The device runs at the engine's sample rate.
    pub fn start(cfg: &AudioConfig, processor: EngineProcessor) -> Result<AudioHost> {
        Self::start_or_return(cfg, processor).map_err(|e| e.error)
    }

    /// [`AudioHost::start`], giving the processor back when the device
    /// cannot be opened (to try another one with the same engine).
    pub fn start_or_return(
        cfg: &AudioConfig,
        processor: EngineProcessor,
    ) -> std::result::Result<AudioHost, StartError> {
        let rate = processor.config().sample_rate;
        let fail = |error: AudioError, processor: Option<EngineProcessor>| StartError { error, processor };
        let picked = match pick_device(cfg) {
            Ok(p) => p,
            Err(e) => return Err(fail(e, Some(processor))),
        };
        let (dev, fell_back) = picked;
        let attempt = negotiate_on(cfg, &dev, fell_back, Some(rate));
        let negotiated = match attempt {
            Ok((n, s, f)) => Ok((n, s, f, dev)),
            Err(e) if cfg.fallback_to_default && !fell_back => {
                tracing::warn!(error = %e, "cannot open requested device, using default output");
                host(cfg.backend)
                    .ok()
                    .and_then(|h| h.default_output_device())
                    .ok_or(e)
                    .and_then(|d| {
                        negotiate_on(cfg, &d, true, Some(rate)).map(|(n, s, f)| (n, s, f, d))
                    })
            }
            Err(e) => Err(e),
        };
        let (info, stream_cfg, fmt, dev) = match negotiated {
            Ok(n) => n,
            Err(e) => return Err(fail(e, Some(processor))),
        };
        let mut info = info;
        let stats = Arc::new(SharedStats::new());
        // Headphones on a second device: opened first, so the ring is there
        // for the main callback.
        let mut cue_stream = None;
        let mut cue_tx = None;
        if let Some(name) = cfg.cue_device.as_ref().filter(|n| !n.trim().is_empty()) {
            let buffer = info.buffer_frames.unwrap_or(512) as usize;
            match start_cue(cfg.backend, name, cfg.cue_pair, rate, buffer, stats.clone()) {
                Ok((stream, tx, opened)) => {
                    cue_stream = Some(stream);
                    cue_tx = Some(tx);
                    info.cue_device = Some(opened);
                    // The headphone bus goes there, not to this device.
                    info.routing.headphones = None;
                }
                Err(e) => {
                    tracing::warn!(device = %name, error = %e, "headphone device did not start");
                    info.cue_error = Some(e.to_string());
                }
            }
        }
        let (give_back, returned) = channel();
        let state = CallbackState {
            processor: Some(processor),
            give_back,
            master: vec![0.0; SCRATCH_FRAMES * 2],
            phones: vec![0.0; SCRATCH_FRAMES * 2],
            decks: vec![0.0; SCRATCH_FRAMES * 2 * MAX_DECKS],
            cue_tx,
            channels: stream_cfg.channels as usize,
            routing: info.routing,
            sample_rate: rate as f64,
            stats: stats.clone(),
        };
        let stream = build(&dev, &stream_cfg, fmt, state, stats.clone())
            .and_then(|s| s.play().map(|_| s).map_err(AudioError::from));
        let stream = match stream {
            Ok(s) => s,
            // The callback state was dropped with the failed stream.
            Err(e) => {
                let p = returned.recv_timeout(Duration::from_secs(2)).ok();
                return Err(fail(e, p));
            }
        };
        tracing::info!(
            device = %info.device_name, rate = info.sample_rate, channels = info.channels,
            buffer = ?info.buffer_frames, format = %info.sample_format, "audio output started"
        );
        Ok(AudioHost {
            driver: Driver::Cpal(stream),
            cue_stream,
            info,
            stats,
            returned,
        })
    }

    /// Runs `processor` on a [`VirtualOutput`] (no hardware).
    pub fn start_virtual(out: &VirtualOutput, processor: EngineProcessor) -> AudioHost {
        let rate = processor.config().sample_rate;
        let stats = Arc::new(SharedStats::new());
        let (give_back, returned) = channel();
        let mut routing = out.routing.unwrap_or_else(|| OutputRouting::auto(out.channels));
        let dropped = routing.fit(out.channels.max(2));
        let channels = out.channels.max(2) as usize;
        let mut state = CallbackState {
            processor: Some(processor),
            give_back,
            master: vec![0.0; SCRATCH_FRAMES * 2],
            phones: vec![0.0; SCRATCH_FRAMES * 2],
            decks: vec![0.0; SCRATCH_FRAMES * 2 * MAX_DECKS],
            cue_tx: None,
            channels,
            routing,
            sample_rate: rate as f64,
            stats: stats.clone(),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let unplugged = Arc::new(AtomicBool::new(false));
        let (stop2, unplugged2) = (stop.clone(), unplugged.clone());
        let block = out.block.clamp(16, SCRATCH_FRAMES);
        let thread = std::thread::Builder::new()
            .name("od-virtual-audio".into())
            .spawn(move || {
                let mut data = vec![0.0f32; block * channels];
                let period = Duration::from_secs_f64(block as f64 / rate.max(1) as f64);
                let mut next = Instant::now();
                while !stop2.load(Ordering::Relaxed) {
                    if unplugged2.load(Ordering::Relaxed) {
                        // Gone: no more callbacks, the processor waits here
                        // until the host is stopped.
                        std::thread::sleep(Duration::from_millis(5));
                        next = Instant::now();
                        continue;
                    }
                    state.render_frames(&mut data, None);
                    next += period;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now;
                    }
                }
                drop(state);
            })
            .expect("virtual audio thread");
        AudioHost {
            driver: Driver::Virtual(VirtualThread {
                stop,
                thread: Some(thread),
                unplugged,
            }),
            info: Negotiated {
                backend: Backend::Default,
                device_name: out.name.clone(),
                sample_rate: rate,
                channels: out.channels,
                buffer_frames: Some(block as u32),
                sample_format: "F32".into(),
                routing,
                dropped,
                fell_back: false,
                cue_device: None,
                cue_error: None,
            },
            cue_stream: None,
            stats,
            returned,
        }
    }

    /// Stops the stream and returns the engine processor (with every deck
    /// as it was), so it can be started on another device. `None` if the
    /// backend did not release it within two seconds.
    pub fn stop(self) -> Option<EngineProcessor> {
        let AudioHost {
            driver,
            returned,
            cue_stream,
            ..
        } = self;
        drop(cue_stream);
        drop(driver);
        returned.recv_timeout(Duration::from_secs(2)).ok()
    }

    /// Fault injection for this host.
    pub fn fault_hook(&self) -> FaultHook {
        FaultHook {
            stats: self.stats.clone(),
            unplugged: match &self.driver {
                Driver::Virtual(v) => Some(v.unplugged.clone()),
                Driver::Cpal(_) => None,
            },
        }
    }

    /// True for a [`VirtualOutput`].
    pub fn is_virtual(&self) -> bool {
        matches!(self.driver, Driver::Virtual(_))
    }

    pub fn info(&self) -> &StreamInfo {
        &self.info
    }

    pub fn health(&self) -> Health {
        self.stats().health()
    }

    pub fn stats(&self) -> AudioStats {
        let s = &self.stats;
        let lat = s.latency_ns.load(Ordering::Relaxed);
        let last = s.last_callback_us.load(Ordering::Relaxed);
        AudioStats {
            callbacks: s.callbacks.load(Ordering::Relaxed),
            xruns: s.xruns.load(Ordering::Relaxed),
            overruns: s.overruns.load(Ordering::Relaxed),
            callback_frames: s.last_frames.load(Ordering::Relaxed),
            max_callback_frames: s.max_frames.load(Ordering::Relaxed),
            output_latency: (lat > 0).then(|| Duration::from_nanos(lat)),
            load: s.load_ppm.load(Ordering::Relaxed) as f32 / 1e6,
            peak_load: s.peak_load_ppm.load(Ordering::Relaxed) as f32 / 1e6,
            device_lost: s.device_lost.load(Ordering::Relaxed),
            cue_underruns: s.cue_underruns.load(Ordering::Relaxed),
            cue_adjusts: s.cue_adjusts.load(Ordering::Relaxed),
            since_last_callback: (last > 0).then(|| {
                s.epoch
                    .elapsed()
                    .saturating_sub(Duration::from_micros(last))
            }),
        }
    }
}

/// Opens `name` for the headphones: pair `pair` at the engine's `rate`,
/// fed through a ring of a few buffers.
fn start_cue(
    backend: Backend,
    name: &str,
    pair: u16,
    rate: u32,
    buffer: usize,
    stats: Arc<SharedStats>,
) -> Result<(cpal::Stream, rtrb::Producer<f32>, String)> {
    let h = host(backend)?;
    let dev = find_device(&h, name, true).ok_or_else(|| AudioError::DeviceNotFound(name.into()))?;
    let opened = device_name(&dev);
    let needed = pair * 2 + 2;
    let best = dev
        .supported_output_configs()?
        .filter(|r| r.channels() >= needed && r.contains_rate(rate) && format_rank(r.sample_format()) < 9)
        .min_by_key(|r| (r.channels(), format_rank(r.sample_format())))
        .ok_or_else(|| AudioError::UnsupportedConfig {
            device: opened.clone(),
            sample_rate: rate,
            channels: needed,
        })?;
    let cfg = cpal::StreamConfig {
        channels: best.channels(),
        sample_rate: rate,
        buffer_size: cpal::BufferSize::Default,
    };
    // A few main buffers of headroom, at least 20 ms.
    let target = buffer.max(64) * 3;
    let target = target.max(rate as usize / 50);
    let (tx, rx) = rtrb::RingBuffer::new(target * 2 * 8);
    let bridge = CueBridge::new(rx, target, stats);
    let ch = cfg.channels as usize;
    let pair = [pair as usize * 2, pair as usize * 2 + 1];
    fn typed<T: SizedSample + FromSample<f32>>(
        dev: &cpal::Device,
        cfg: &cpal::StreamConfig,
        mut bridge: CueBridge,
        ch: usize,
        pair: [usize; 2],
    ) -> Result<cpal::Stream> {
        Ok(dev.build_output_stream::<T, _, _>(
            *cfg,
            move |data: &mut [T], _: &OutputCallbackInfo| bridge.pull(data, ch, pair),
            |e: cpal::Error| tracing::warn!(error = %e, "headphone stream error"),
            None,
        )?)
    }
    let stream = match best.sample_format() {
        SampleFormat::F32 => typed::<f32>(&dev, &cfg, bridge, ch, pair),
        SampleFormat::I32 => typed::<i32>(&dev, &cfg, bridge, ch, pair),
        SampleFormat::I24 => typed::<cpal::I24>(&dev, &cfg, bridge, ch, pair),
        SampleFormat::I16 => typed::<i16>(&dev, &cfg, bridge, ch, pair),
        SampleFormat::F64 => typed::<f64>(&dev, &cfg, bridge, ch, pair),
        SampleFormat::U16 => typed::<u16>(&dev, &cfg, bridge, ch, pair),
        other => Err(AudioError::UnsupportedFormat(format!("{other:?}"))),
    }?;
    stream.play()?;
    Ok((stream, tx, opened))
}

fn build(
    dev: &cpal::Device,
    cfg: &cpal::StreamConfig,
    fmt: SampleFormat,
    state: CallbackState,
    stats: Arc<SharedStats>,
) -> Result<cpal::Stream> {
    fn typed<T: SizedSample + FromSample<f32>>(
        dev: &cpal::Device,
        cfg: &cpal::StreamConfig,
        mut state: CallbackState,
        stats: Arc<SharedStats>,
    ) -> Result<cpal::Stream> {
        let err = move |e: cpal::Error| match e.kind() {
            cpal::ErrorKind::Xrun => {
                stats.xruns.fetch_add(1, Ordering::Relaxed);
            }
            cpal::ErrorKind::DeviceNotAvailable => {
                stats.report_lost();
                tracing::error!("audio device disconnected");
            }
            _ => tracing::warn!(error = %e, "audio stream error"),
        };
        Ok(dev.build_output_stream::<T, _, _>(
            *cfg,
            move |data: &mut [T], info: &OutputCallbackInfo| state.render(data, info),
            err,
            None,
        )?)
    }
    match fmt {
        SampleFormat::F32 => typed::<f32>(dev, cfg, state, stats),
        SampleFormat::I32 => typed::<i32>(dev, cfg, state, stats),
        SampleFormat::I24 => typed::<cpal::I24>(dev, cfg, state, stats),
        SampleFormat::I16 => typed::<i16>(dev, cfg, state, stats),
        SampleFormat::F64 => typed::<f64>(dev, cfg, state, stats),
        SampleFormat::U16 => typed::<u16>(dev, cfg, state, stats),
        other => Err(AudioError::UnsupportedFormat(format!("{other:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_routing() {
        assert_eq!(
            OutputRouting::auto(2),
            OutputRouting {
                master: Some([0, 1]),
                headphones: None,
                decks: [None; MAX_DECKS],
            }
        );
        let r = OutputRouting::auto(4);
        assert_eq!(r.headphones, Some([2, 3]));
        assert_eq!(r.channels_needed(), 4);
    }

    #[test]
    fn routes_that_do_not_fit_are_dropped() {
        let mut r = OutputRouting {
            master: Some([0, 1]),
            headphones: Some([2, 3]),
            decks: [Some([4, 5]), Some([2, 3]), Some([8, 9]), None],
        };
        let dropped = r.fit(8);
        // Deck 2 wanted the headphones' pair, deck 3 a pair the device lacks.
        assert_eq!(dropped, vec![Route::Deck(1), Route::Deck(2)]);
        assert_eq!(r.decks, [Some([4, 5]), None, None, None]);
        assert_eq!(r.channels_needed(), 6);
        let mut stereo = OutputRouting::auto(4);
        assert_eq!(stereo.fit(2), vec![Route::Headphones]);
        assert_eq!(stereo.master, Some([0, 1]));
    }

    use od_engine::{
        ChannelCommand, Command, DeckCommand, Engine, EngineConfig, MixerCommand, Profile,
        TrackData,
    };

    fn sine(id: i64, freq: f64, l: f32, r: f32) -> std::sync::Arc<TrackData> {
        let n = 48_000 * 4;
        let samples = (0..n)
            .flat_map(|i| {
                let v = (std::f64::consts::TAU * freq * i as f64 / 48_000.0).sin() as f32;
                [v * l, v * r]
            })
            .collect();
        TrackData::new(
            od_core::TrackId(id),
            od_decode::DecodedAudio {
                sample_rate: 48_000,
                channels: 2,
                samples,
            },
            48_000,
            od_core::Beatgrid::default(),
            &[],
            0.0,
        )
    }

    fn rms_of(data: &[f32], ch: usize, c: usize) -> f32 {
        let v: Vec<f32> = data.chunks_exact(ch).map(|f| f[c]).collect();
        (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    /// A callback state around a fresh engine, two decks playing: deck 1 a
    /// 0.4 sine on both sides, deck 2 a 0.2 sine.
    fn rig(routing: OutputRouting, channels: usize) -> (Engine, CallbackState) {
        let (mut engine, processor) = Engine::new(EngineConfig {
            sample_rate: 48_000,
            max_block: 512,
            decks: 2,
            profile: Profile::Lite,
        });
        engine.send(Command::Mixer(MixerCommand::SetLimiter(false))).unwrap();
        for (d, t) in [(0, sine(1, 440.0, 0.4, 0.4)), (1, sine(2, 1000.0, 0.2, 0.2))] {
            engine.deck(d, DeckCommand::SetAutoCue(false)).unwrap();
            engine.deck(d, DeckCommand::Load(t)).unwrap();
            engine.deck(d, DeckCommand::Play).unwrap();
        }
        let (give_back, _returned) = channel();
        std::mem::forget(_returned);
        let state = CallbackState {
            processor: Some(processor),
            give_back,
            master: vec![0.0; SCRATCH_FRAMES * 2],
            phones: vec![0.0; SCRATCH_FRAMES * 2],
            decks: vec![0.0; SCRATCH_FRAMES * 2 * MAX_DECKS],
            cue_tx: None,
            channels,
            routing,
            sample_rate: 48_000.0,
            stats: Arc::new(SharedStats::new()),
        };
        (engine, state)
    }

    /// [67] An 8-output interface with the external mixer: deck 1 on 3/4,
    /// deck 2 on 5/6, nothing of them on the master pair.
    #[test]
    fn each_deck_on_its_own_device_pair() {
        let routing = OutputRouting {
            master: Some([0, 1]),
            headphones: None,
            decks: [Some([2, 3]), Some([4, 5]), None, None],
        };
        let (mut engine, mut st) = rig(routing, 8);
        engine.send(Command::Mixer(MixerCommand::SetExternalMixer(true))).unwrap();
        let mut data = vec![0.0f32; 4800 * 8];
        st.render_frames(&mut data, None);
        st.render_frames(&mut data, None);
        let s = 0.4 / 2f32.sqrt();
        assert!((rms_of(&data, 8, 2) - s).abs() < 0.01 && (rms_of(&data, 8, 3) - s).abs() < 0.01);
        let s = 0.2 / 2f32.sqrt();
        assert!((rms_of(&data, 8, 4) - s).abs() < 0.01 && (rms_of(&data, 8, 5) - s).abs() < 0.01);
        for c in [0, 1, 6, 7] {
            assert!(rms_of(&data, 8, c) < 1e-9, "channel {c} silent");
        }
    }

    /// [67] Split cue on one stereo output: deck 1 cued (fader down) on the
    /// left in mono, deck 2 on air on the right in mono.
    #[test]
    fn split_cue_on_one_stereo_device() {
        let (mut engine, mut st) = rig(OutputRouting::split_on_one(), 2);
        engine.send(Command::Channel(0, ChannelCommand::SetFader(0.0))).unwrap();
        engine.send(Command::Channel(0, ChannelCommand::SetCue(true))).unwrap();
        engine.send(Command::Mixer(MixerCommand::SetSplitCue(true))).unwrap();
        let mut data = vec![0.0f32; 4800 * 2];
        st.render_frames(&mut data, None);
        st.render_frames(&mut data, None);
        let l: Vec<f32> = data.iter().step_by(2).copied().collect();
        let r: Vec<f32> = data.iter().skip(1).step_by(2).copied().collect();
        let zc = |v: &[f32]| v.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
        assert!((rms_of(&data, 2, 0) - 0.4 / 2f32.sqrt()).abs() < 0.01);
        assert!((rms_of(&data, 2, 1) - 0.2 / 2f32.sqrt()).abs() < 0.01);
        // 0.1 s: 440 Hz → 88 crossings, 1000 Hz → 200.
        assert!((zc(&l) as i32 - 88).abs() <= 2, "the cue on the left");
        assert!((zc(&r) as i32 - 200).abs() <= 2, "the master on the right");
    }

    /// [67] The headphone ring of a second device: primes, plays what the
    /// main callback pushed in order, drops or repeats a frame when the
    /// clocks drift, and fills silence (counted) when it runs dry.
    #[test]
    fn cue_bridge_follows_the_main_device() {
        let stats = Arc::new(SharedStats::new());
        let (mut tx, rx) = rtrb::RingBuffer::new(4096);
        let mut b = CueBridge::new(rx, 128, stats.clone());
        let mut out = vec![1.0f32; 64 * 4];
        // Not primed yet: silence.
        b.pull(&mut out, 4, [2, 3]);
        assert!(out.iter().all(|v| *v == 0.0));
        let ramp: Vec<f32> = (0..200).flat_map(|i| [i as f32, -(i as f32)]).collect();
        assert_eq!(tx.push_partial_slice(&ramp).0.len(), 400);
        b.pull(&mut out, 4, [2, 3]);
        let got: Vec<f32> = out.chunks_exact(4).map(|f| f[2]).collect();
        assert_eq!(got, (0..64).map(|i| i as f32).collect::<Vec<_>>());
        assert!(out.chunks_exact(4).all(|f| f[0] == 0.0 && f[1] == 0.0 && f[3] == -f[2]));
        // Way ahead (main device faster): one frame dropped.
        let more: Vec<f32> = (200..800).flat_map(|i| [i as f32, 0.0]).collect();
        assert_eq!(tx.push_partial_slice(&more).0.len(), 1200);
        b.pull(&mut out, 4, [2, 3]);
        assert_eq!(out[2], 65.0, "frame 64 dropped");
        assert_eq!(stats.cue_adjusts.load(Ordering::Relaxed), 1);
        // Drain until dry: silence and an underrun.
        for _ in 0..40 {
            b.pull(&mut out, 4, [2, 3]);
        }
        assert!(stats.cue_underruns.load(Ordering::Relaxed) >= 1);
        assert!(out.iter().all(|v| *v == 0.0));
    }
}
