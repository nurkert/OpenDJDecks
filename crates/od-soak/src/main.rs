//! Long-run measurement of the OpenDJDecks engine (roadmap item [51]).
//!
//! Runs the real engine in real time for hours: two or four decks play and
//! loop real audio with keylock, sync, loops, beat jumps, stems (from the
//! stem cache), tempo FX on the master and on one part, channel FX, and the
//! recorder writes the master to WAV. Every minute one JSON line is written
//! with deadline misses, render time against the buffer period, latency,
//! memory and the machine's load average.
//!
//! No audio device is opened and nothing is audible. A dedicated thread with
//! the same scheduling class Core Audio gives its I/O thread on macOS (a
//! time-constraint thread) calls the engine once per buffer period, paced by
//! the clock. A callback counts as a deadline miss when it finishes later
//! than one buffer period after its scheduled start, which is when a device
//! would have needed the buffer. What this does not cover: the audio
//! driver's own work, and the UI drawing at the same time.
//!
//! ```text
//! cargo run --profile soak -p od-soak --bin soak -- --decks 2 --hours 4 \
//!     --track a.mp3=<content hash> --track b.mp3 --stems-dir <dir> \
//!     --record-dir <dir> --out soak-2.jsonl
//! ```

use anyhow::{Context, Result, bail};
use od_core::{ContentHash, TrackId};
use od_engine::*;
use od_recorder::Recorder;
use od_stems::StemCache;
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Histogram resolution.
const BUCKET_NS: u64 = 5_000;
/// 5 µs buckets up to 50 ms; longer times land in the last bucket (the
/// exact maximum is kept separately).
const BUCKETS: usize = 10_000;

struct Args {
    decks: usize,
    duration: Duration,
    buffer: usize,
    rate: u32,
    tracks: Vec<(PathBuf, Option<ContentHash>)>,
    stems_dir: Option<PathBuf>,
    record_dir: PathBuf,
    out: Option<PathBuf>,
    label: String,
}

fn parse_args() -> Result<Args> {
    let mut a = Args {
        decks: 2,
        duration: Duration::from_secs(120),
        buffer: 256,
        rate: 48_000,
        tracks: Vec::new(),
        stems_dir: None,
        record_dir: std::env::temp_dir().join("od-soak-rec"),
        out: None,
        label: String::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().with_context(|| format!("{k} needs a value"));
        match k.as_str() {
            "--decks" => a.decks = v()?.parse()?,
            "--hours" => a.duration = Duration::from_secs_f64(v()?.parse::<f64>()? * 3600.0),
            "--minutes" => a.duration = Duration::from_secs_f64(v()?.parse::<f64>()? * 60.0),
            "--buffer" => a.buffer = v()?.parse()?,
            "--rate" => a.rate = v()?.parse()?,
            "--track" => {
                let s = v()?;
                let (p, h) = match s.rsplit_once('=') {
                    Some((p, h)) if !h.is_empty() => (p.to_string(), Some(ContentHash(h.into()))),
                    _ => (s, None),
                };
                a.tracks.push((p.into(), h));
            }
            "--stems-dir" => a.stems_dir = Some(v()?.into()),
            "--record-dir" => a.record_dir = v()?.into(),
            "--out" => a.out = Some(v()?.into()),
            "--label" => a.label = v()?,
            "-h" | "--help" => {
                println!(
                    "soak --decks 2|4 (--hours H | --minutes M) [--buffer 256] [--rate 48000] \
                     --track PATH[=CONTENT_HASH]... [--stems-dir DIR] [--record-dir DIR] \
                     [--out FILE.jsonl] [--label TEXT]"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument {other}"),
        }
    }
    if a.decks != 2 && a.decks != 4 {
        bail!("--decks must be 2 or 4");
    }
    if a.tracks.is_empty() {
        bail!("give at least one --track");
    }
    if !(16..=2048).contains(&a.buffer) {
        bail!("--buffer must be 16..=2048");
    }
    Ok(a)
}

/// Shared between the audio thread and the reporter (atomics only).
struct Stats {
    render: Vec<AtomicU32>,
    wake: Vec<AtomicU32>,
    render_max_ns: AtomicU64,
    wake_max_ns: AtomicU64,
    callbacks: AtomicU64,
    misses: AtomicU64,
    overruns: AtomicU64,
}

impl Stats {
    fn new() -> Self {
        Self {
            render: (0..BUCKETS).map(|_| AtomicU32::new(0)).collect(),
            wake: (0..BUCKETS).map(|_| AtomicU32::new(0)).collect(),
            render_max_ns: AtomicU64::new(0),
            wake_max_ns: AtomicU64::new(0),
            callbacks: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            overruns: AtomicU64::new(0),
        }
    }
}

fn bucket(ns: u64) -> usize {
    ((ns / BUCKET_NS) as usize).min(BUCKETS - 1)
}

/// Takes the histogram (resetting it) and returns (p50, p99) in µs, as the
/// upper edge of the bucket the percentile falls in.
fn take_percentiles(h: &[AtomicU32]) -> (f64, f64, u64) {
    let counts: Vec<u64> = h.iter().map(|c| c.swap(0, Ordering::Relaxed) as u64).collect();
    let total: u64 = counts.iter().sum();
    let pct = |p: f64| -> f64 {
        if total == 0 {
            return 0.0;
        }
        let want = ((total as f64) * p).ceil() as u64;
        let mut acc = 0;
        for (i, c) in counts.iter().enumerate() {
            acc += c;
            if acc >= want {
                return ((i as u64 + 1) * BUCKET_NS) as f64 / 1000.0;
            }
        }
        (BUCKETS as u64 * BUCKET_NS) as f64 / 1000.0
    };
    (pct(0.50), pct(0.99), total)
}

/// Gives the calling thread the scheduling class of a Core Audio I/O
/// thread (macOS). Returns false where that is not available.
#[cfg(target_os = "macos")]
#[allow(deprecated)] // libc points to mach2 for these; the calls are stable.
fn make_realtime(period: Duration) -> bool {
    unsafe {
        let mut tb = libc::mach_timebase_info { numer: 0, denom: 0 };
        if libc::mach_timebase_info(&mut tb) != 0 || tb.numer == 0 {
            return false;
        }
        let abs = |d: Duration| (d.as_nanos() as u64 * tb.denom as u64 / tb.numer as u64) as u32;
        let mut policy = libc::thread_time_constraint_policy {
            period: abs(period),
            computation: abs(period / 2),
            constraint: abs(period),
            preemptible: 1,
        };
        let r = libc::thread_policy_set(
            libc::mach_thread_self(),
            libc::THREAD_TIME_CONSTRAINT_POLICY as libc::thread_policy_flavor_t,
            &mut policy as *mut _ as libc::thread_policy_t,
            libc::THREAD_TIME_CONSTRAINT_POLICY_COUNT,
        );
        r == 0
    }
}

#[cfg(not(target_os = "macos"))]
fn make_realtime(_period: Duration) -> bool {
    false
}

/// The paced "device": one engine callback per buffer period.
fn spawn_audio(
    mut processor: EngineProcessor,
    buffer: usize,
    rate: u32,
    stats: Arc<Stats>,
    stop: Arc<AtomicBool>,
    rt_ok: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("od-soak-audio".into())
        .spawn(move || {
            let period = Duration::from_secs_f64(buffer as f64 / rate as f64);
            rt_ok.store(make_realtime(period), Ordering::Relaxed);
            let mut master = vec![0.0f32; buffer * 2];
            let mut phones = vec![0.0f32; buffer * 2];
            let mut sched = Instant::now() + period;
            while !stop.load(Ordering::Relaxed) {
                let now = Instant::now();
                if sched > now {
                    std::thread::sleep(sched - now);
                }
                let start = Instant::now();
                let wake = start.saturating_duration_since(sched).as_nanos() as u64;
                processor.process(
                    buffer,
                    &mut Outputs {
                        master: &mut master,
                        headphones: Some(&mut phones),
                        booth: None,
                    },
                );
                let end = Instant::now();
                let render = end.duration_since(start).as_nanos() as u64;
                stats.render[bucket(render)].fetch_add(1, Ordering::Relaxed);
                stats.wake[bucket(wake)].fetch_add(1, Ordering::Relaxed);
                stats.render_max_ns.fetch_max(render, Ordering::Relaxed);
                stats.wake_max_ns.fetch_max(wake, Ordering::Relaxed);
                stats.callbacks.fetch_add(1, Ordering::Relaxed);
                if render > period.as_nanos() as u64 {
                    stats.overruns.fetch_add(1, Ordering::Relaxed);
                }
                if end > sched + period {
                    // The buffer was late: a device would have played a gap.
                    // Like a device, carry on from now.
                    stats.misses.fetch_add(1, Ordering::Relaxed);
                    sched = end;
                } else {
                    sched += period;
                }
            }
        })
        .expect("audio thread")
}

fn rss_mb() -> Option<f64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    let kb: f64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    Some((kb / 1024.0 * 10.0).round() / 10.0)
}

fn loadavg() -> [f64; 3] {
    let mut l = [0f64; 3];
    #[cfg(unix)]
    unsafe {
        libc::getloadavg(l.as_mut_ptr(), 3);
    }
    l.map(|v| (v * 100.0).round() / 100.0)
}

fn cmd_out(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

struct Library {
    tracks: Vec<(PathBuf, Option<ContentHash>, od_core::Beatgrid)>,
    cache: StemCache,
    rate: u32,
    next_id: i64,
}

impl Library {
    /// Decodes track `i` (and its cached stems) the way the app loads a deck.
    fn load(&mut self, i: usize) -> Result<Arc<TrackData>> {
        let (path, hash, grid) = &self.tracks[i % self.tracks.len()];
        self.next_id += 1;
        Ok(od_stems::load_track_data(
            &self.cache,
            TrackId(self.next_id),
            path,
            hash.as_ref(),
            self.rate,
            grid.clone(),
            &[],
            0.0,
        )?)
    }
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let mut out: Box<dyn Write> = match &args.out {
        Some(p) => Box::new(std::io::BufWriter::new(std::fs::File::create(p)?)),
        None => Box::new(std::io::stdout()),
    };
    let decks = args.decks;
    let rate = args.rate;

    // Analyse once (beat grids); decoding happens at every load.
    let mut tracks = Vec::new();
    for (p, h) in &args.tracks {
        let a = od_analysis::analyze_file(p, &Default::default())
            .with_context(|| format!("analysing {}", p.display()))?;
        tracks.push((p.clone(), h.clone(), a.beatgrid));
    }
    let cache = StemCache::new(
        args.stems_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("od-soak-no-stems")),
    );
    let mut lib = Library {
        tracks,
        cache,
        rate,
        next_id: 0,
    };

    let (mut engine, processor) = Engine::new(EngineConfig {
        sample_rate: rate,
        max_block: args.buffer.max(512),
        decks,
        profile: Profile::Full,
    });
    let mut recorder = Recorder::new(engine.take_record_tap().context("record tap")?, rate);

    let mut stems_on = Vec::new();
    for d in 0..decks {
        let t = lib.load(d)?;
        stems_on.push(t.has_stems());
        engine.deck(d, DeckCommand::Load(t))?;
    }
    let track_info: Vec<_> = lib
        .tracks
        .iter()
        .map(|(p, h, _)| {
            let stems = h.as_ref().is_some_and(|h| lib.cache.exists(h));
            json!({"file": p.file_name().map(|n| n.to_string_lossy().to_string()), "stems": stems})
        })
        .collect();

    // Mixer and effects.
    engine.send(Command::SetRecordTap(true))?;
    for d in 0..decks {
        engine.deck(d, DeckCommand::SetKeylock(true))?;
        engine.deck(d, DeckCommand::SetTempoFader(0.04 - d as f32 * 0.02))?;
        if d > 0 {
            engine.deck(d, DeckCommand::SetSync(true))?;
        }
        // Stem levels below unity keep the stem path running.
        engine.deck(d, DeckCommand::SetStemLevel { stem: Stem::Vocals, level: 0.6 })?;
        engine.send(Command::Channel(d, ChannelCommand::SetFader(0.8)))?;
        engine.send(Command::Channel(d, ChannelCommand::SetChannelFxAmount(0.25)))?;
        engine.deck(d, DeckCommand::Play)?;
    }
    for (unit, kind, target) in [
        (0, TempoFxKind::Echo, FxTarget::Master),
        (1, TempoFxKind::Reverb, FxTarget::Part(0, Stem::Drums)),
    ] {
        engine.send(Command::TempoFx(unit, TempoFxCommand::SetKind(kind)))?;
        engine.send(Command::TempoFx(unit, TempoFxCommand::SetTarget(target)))?;
        engine.send(Command::TempoFx(unit, TempoFxCommand::SetLevel(0.4)))?;
        engine.send(Command::TempoFx(unit, TempoFxCommand::SetOn(true)))?;
    }

    let stats = Arc::new(Stats::new());
    let stop = Arc::new(AtomicBool::new(false));
    let rt_ok = Arc::new(AtomicBool::new(false));
    let audio = spawn_audio(
        processor,
        args.buffer,
        rate,
        stats.clone(),
        stop.clone(),
        rt_ok.clone(),
    );
    std::fs::create_dir_all(&args.record_dir)?;
    let mut recorded_s = 0.0;
    recorder.start(&args.record_dir, "soak")?;
    std::thread::sleep(Duration::from_millis(100));

    let period_ms = args.buffer as f64 / rate as f64 * 1000.0;
    let header = json!({
        "type": "header",
        "label": args.label,
        "started_unix": unix_now(),
        "decks": decks,
        "sample_rate": rate,
        "buffer_frames": args.buffer,
        "period_ms": period_ms,
        "latency_ms": period_ms,
        "planned_minutes": args.duration.as_secs_f64() / 60.0,
        "output": "paced: no audio device, one engine callback per buffer period on a time-constraint thread",
        "rt_policy": rt_ok.load(Ordering::Relaxed),
        "build": if cfg!(debug_assertions) { "debug" } else { "optimised" },
        "machine": cmd_out("sysctl", &["-n", "hw.model"]),
        "cpu": cmd_out("sysctl", &["-n", "machdep.cpu.brand_string"]),
        "memory_bytes": cmd_out("sysctl", &["-n", "hw.memsize"]).parse::<u64>().ok(),
        "os": format!("{} {}", cmd_out("sw_vers", &["-productName"]), cmd_out("sw_vers", &["-productVersion"])),
        "git": cmd_out("git", &["rev-parse", "--short", "HEAD"]),
        "tracks": track_info,
        "decks_with_stems": stems_on.iter().filter(|s| **s).count(),
        "load": "keylock on, sync on decks 2+, 8-beat loops, beat jumps, tempo changes, stem levels, echo on master, reverb on deck 1 drums, channel FX, a deck reloaded every 3 minutes, recording to WAV (rotated every 10 minutes)",
    });
    writeln!(out, "{header}")?;
    out.flush()?;

    let start = Instant::now();
    let mut minute = 0u64;
    let mut next_minute = Duration::from_secs(60);
    let mut tick = 0u64;
    let mut reloads = 0u64;
    let mut next_load = 4usize;
    let mut last_rotate = Instant::now();
    let mut misses_total = 0u64;
    let mut overruns_total = 0u64;
    let mut callbacks_total = 0u64;
    let mut worst_render_us = 0f64;
    let mut worst_p99_us = 0f64;
    while start.elapsed() < args.duration {
        std::thread::sleep(Duration::from_millis(50));
        tick += 1;
        engine.events().for_each(drop);
        engine.collect_garbage();
        let st = *engine.state();
        // Every second: keep the decks playing (start over near the end).
        if tick % 20 == 0 {
            for d in 0..decks {
                let s = &st.decks[d];
                if s.at_end || s.position > s.duration - 3.0 {
                    engine.deck(d, DeckCommand::Seek { seconds: 0.0 })?;
                    engine.deck(d, DeckCommand::Play)?;
                }
            }
        }
        // Every 10 s, one deck changes: loops, jumps, stems, tempo.
        if tick % 200 == 0 {
            let step = tick / 200;
            let d = (step as usize) % decks;
            match step % 6 {
                0 => engine.deck(d, DeckCommand::AutoLoop { beats: 8.0 })?,
                1 => engine.deck(d, DeckCommand::LoopHalve)?,
                2 => engine.deck(d, DeckCommand::LoopExit)?,
                3 => engine.deck(d, DeckCommand::BeatJump { beats: 16.0 })?,
                4 => engine.deck(
                    d,
                    DeckCommand::SetStemLevel {
                        stem: Stem::Drums,
                        level: if step % 12 == 4 { 0.0 } else { 1.0 },
                    },
                )?,
                _ => engine.deck(
                    0,
                    DeckCommand::SetTempoFader(((step % 7) as f32 - 3.0) / 50.0),
                )?,
            }
        }
        // Every 3 minutes: a fresh load on one deck (decode + stems).
        if tick % 3600 == 1800 {
            let d = reloads as usize % decks;
            let t = lib.load(next_load)?;
            next_load += 1;
            engine.deck(d, DeckCommand::Load(t))?;
            engine.deck(d, DeckCommand::Play)?;
            reloads += 1;
        }
        // Every 10 minutes: close the recording and start a new one, so
        // the disk does not fill during hours of recording.
        if last_rotate.elapsed() >= Duration::from_secs(600) {
            let r = recorder.stop()?;
            recorded_s += r.duration;
            for p in [&r.path, &r.cue_path, &r.text_path] {
                let _ = std::fs::remove_file(p);
            }
            recorder.start(&args.record_dir, "soak")?;
            last_rotate = Instant::now();
        }
        if start.elapsed() >= next_minute {
            next_minute += Duration::from_secs(60);
            minute += 1;
            let line = report(
                &stats,
                minute,
                period_ms,
                &mut misses_total,
                &mut overruns_total,
                &mut callbacks_total,
                recorded_s + recorder.elapsed(),
                reloads,
                &mut worst_render_us,
                &mut worst_p99_us,
            );
            writeln!(out, "{line}")?;
            out.flush()?;
        }
    }
    stop.store(true, Ordering::Relaxed);
    let _ = audio.join();
    let r = recorder.stop()?;
    recorded_s += r.duration;
    for p in [&r.path, &r.cue_path, &r.text_path] {
        let _ = std::fs::remove_file(p);
    }
    let result = json!({
        "type": "result",
        "minutes": minute,
        "seconds": start.elapsed().as_secs_f64().round(),
        "callbacks": callbacks_total,
        "misses": misses_total,
        "overruns": overruns_total,
        "worst_render_us": worst_render_us,
        "worst_minute_p99_us": worst_p99_us,
        "recorded_s": recorded_s.round(),
        "reloads": reloads,
        "rss_mb": rss_mb(),
        "ended_unix": unix_now(),
    });
    writeln!(out, "{result}")?;
    out.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn report(
    stats: &Stats,
    minute: u64,
    period_ms: f64,
    misses_total: &mut u64,
    overruns_total: &mut u64,
    callbacks_total: &mut u64,
    recorded_s: f64,
    reloads: u64,
    worst_render_us: &mut f64,
    worst_p99_us: &mut f64,
) -> serde_json::Value {
    let callbacks = stats.callbacks.swap(0, Ordering::Relaxed);
    let misses = stats.misses.swap(0, Ordering::Relaxed);
    let overruns = stats.overruns.swap(0, Ordering::Relaxed);
    let (r50, r99, _) = take_percentiles(&stats.render);
    let (_, w99, _) = take_percentiles(&stats.wake);
    let rmax = stats.render_max_ns.swap(0, Ordering::Relaxed) as f64 / 1000.0;
    let wmax = stats.wake_max_ns.swap(0, Ordering::Relaxed) as f64 / 1000.0;
    *misses_total += misses;
    *overruns_total += overruns;
    *callbacks_total += callbacks;
    *worst_render_us = worst_render_us.max(rmax);
    *worst_p99_us = worst_p99_us.max(r99);
    let pct = |us: f64| (us / 1000.0 / period_ms * 1000.0).round() / 10.0;
    json!({
        "type": "minute",
        "t": minute,
        "callbacks": callbacks,
        "misses": misses,
        "misses_total": *misses_total,
        "overruns": overruns,
        "render_us": {"p50": r50, "p99": r99, "max": (rmax * 10.0).round() / 10.0},
        "load_pct": {"p50": pct(r50), "p99": pct(r99), "max": pct(rmax)},
        "wake_late_us": {"p99": w99, "max": (wmax * 10.0).round() / 10.0},
        "rss_mb": rss_mb(),
        "loadavg": loadavg(),
        "recorded_s": recorded_s.round(),
        "reloads": reloads,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_use_bucket_edges() {
        let h: Vec<AtomicU32> = (0..BUCKETS).map(|_| AtomicU32::new(0)).collect();
        for _ in 0..98 {
            h[bucket(12_000)].fetch_add(1, Ordering::Relaxed);
        }
        h[bucket(400_000)].fetch_add(2, Ordering::Relaxed);
        let (p50, p99, n) = take_percentiles(&h);
        assert_eq!(n, 100);
        assert_eq!(p50, 15.0);
        assert_eq!(p99, 405.0);
        // Taking resets.
        assert_eq!(take_percentiles(&h).2, 0);
    }

    #[test]
    fn long_times_land_in_the_last_bucket() {
        assert_eq!(bucket(10_000_000_000), BUCKETS - 1);
        assert_eq!(bucket(0), 0);
    }
}
