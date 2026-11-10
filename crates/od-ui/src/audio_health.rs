//! Audio health (roadmap [10]): the permanent device, latency and dropout
//! line in the status band, the alert when the device is lost, recovery
//! onto the same device or another one without a restart, and a device
//! change from Preferences that applies at once.
//!
//! When the device goes, the engine's processor comes back from the audio
//! layer and waits ("parked"): decks, positions, loops and the mixer stay as
//! they were and go on when an output runs it again.

use crate::app::{App, handler};
use od_audio_io::{AudioHost, Health};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// A dropout keeps the line in its attention state this long.
const DROPOUT_HOLD: Duration = Duration::from_secs(20);
/// How often a lost device is looked for again.
const PROBE_EVERY: Duration = Duration::from_secs(1);

#[derive(Default)]
struct Watch {
    last_check: Option<Instant>,
    lost: Option<Lost>,
    /// Dropouts counted on outputs before the current one.
    earlier_dropouts: u64,
    shown_dropouts: u64,
    last_dropout: Option<Instant>,
}

struct Lost {
    device: String,
    /// The lost output was the virtual one (headless).
    virtual_out: bool,
    last_probe: Instant,
    /// The device was missing from the device list at least once: when it
    /// shows up again it was plugged back in.
    seen_absent: bool,
}

thread_local! {
    static WATCH: RefCell<Watch> = RefCell::new(Watch::default());
}

/// An output started (first start, recovery, device change).
pub(crate) fn started(app: &App) {
    WATCH.with(|w| {
        let mut w = w.borrow_mut();
        w.lost = None;
        w.last_check = None;
    });
    refresh(app);
}

/// True while the device is lost and the engine waits.
pub(crate) fn is_lost() -> bool {
    WATCH.with(|w| w.borrow().lost.is_some())
}

/// Total dropouts since the app started (all outputs).
pub(crate) fn dropouts(app: &App) -> u64 {
    let current = app
        .engine
        .borrow()
        .host
        .as_ref()
        .map_or(0, |h| h.stats().dropouts());
    WATCH.with(|w| w.borrow().earlier_dropouts) + current
}

/// Output latency of the running device, if known.
pub(crate) fn latency(host: &AudioHost) -> Option<Duration> {
    host.stats()
        .output_latency
        .or_else(|| host.info().nominal_latency())
}

/// "5.3 ms".
pub(crate) fn latency_text(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

/// Frame tick: notices a lost device at once (checked every 100 ms) and
/// looks for it again while it is gone.
pub(crate) fn watch(app: &App) {
    let due = WATCH.with(|w| {
        let mut w = w.borrow_mut();
        let due = w
            .last_check
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(100));
        if due {
            w.last_check = Some(Instant::now());
        }
        due
    });
    if !due {
        return;
    }
    if is_lost() {
        probe(app);
        return;
    }
    let health = app.engine.borrow().host.as_ref().map(|h| h.health());
    if health == Some(Health::Lost) {
        lose(app);
    }
    refresh(app);
}

/// The device is gone: stop the stream, keep the processor, raise the alert.
fn lose(app: &App) {
    let host = app.engine.borrow_mut().host.take();
    let Some(host) = host else { return };
    let device = host.info().device_name.clone();
    let virtual_out = host.is_virtual();
    let before = host.stats().dropouts();
    let processor = host.stop();
    tracing::error!(device = %device, kept = processor.is_some(), "audio device lost");
    {
        let mut h = app.engine.borrow_mut();
        if processor.is_none() {
            // The backend kept it: the engine cannot go on. The next load
            // starts a new one (and the session restore has the decks).
            h.engine = None;
        }
        h.parked = processor;
        h.message = od_i18n::trf("Audio output lost: {}", &[&device]);
    }
    WATCH.with(|w| {
        let mut w = w.borrow_mut();
        w.earlier_dropouts += before;
        w.lost = Some(Lost {
            device: device.clone(),
            virtual_out,
            last_probe: Instant::now(),
            seen_absent: false,
        });
    });
    app.status(od_i18n::trf("Audio output lost: {}", &[&device]));
    crate::engine::show_message(app);
    refresh(app);
}

/// While lost: when the device was gone from the list and is back, it was
/// plugged in again, and audio goes on there.
fn probe(app: &App) {
    let check = WATCH.with(|w| {
        let mut w = w.borrow_mut();
        let l = w.lost.as_mut()?;
        if l.virtual_out || l.last_probe.elapsed() < PROBE_EVERY {
            return None;
        }
        l.last_probe = Instant::now();
        Some(l.device.clone())
    });
    let Some(device) = check else { return };
    let present = od_audio_io::list_devices(od_audio_io::Backend::Default)
        .map(|d| d.iter().any(|d| d.name == device))
        .unwrap_or(false);
    let back = WATCH.with(|w| {
        let mut w = w.borrow_mut();
        let Some(l) = w.lost.as_mut() else { return false };
        if !present {
            l.seen_absent = true;
            false
        } else {
            l.seen_absent
        }
    });
    if back {
        let mut cfg = configured(app);
        cfg.device = Some(device);
        cfg.fallback_to_default = false;
        reopen(app, Target::Device(cfg));
    }
}

/// Where to run the engine next.
pub(crate) enum Target {
    Device(od_audio_io::AudioConfig),
    Virtual(od_audio_io::VirtualOutput),
}

/// The audio configuration from Preferences (the device chosen there).
fn configured(app: &App) -> od_audio_io::AudioConfig {
    let st = app.state.borrow();
    crate::engine::audio_config(&st.settings.audio, st.config.lite)
}

/// Moves the engine onto `target`: the running output stops (its processor
/// comes back), the new one starts with the same engine. True when audio
/// runs afterwards.
pub(crate) fn reopen(app: &App, target: Target) -> bool {
    let running = app.engine.borrow_mut().host.take();
    if let Some(host) = running {
        let before = host.stats().dropouts();
        WATCH.with(|w| w.borrow_mut().earlier_dropouts += before);
        let p = host.stop();
        app.engine.borrow_mut().parked = p;
    }
    let Some(processor) = app.engine.borrow_mut().parked.take() else {
        // No engine to move: start a fresh one on the next load.
        app.engine.borrow_mut().engine = None;
        refresh(app);
        return false;
    };
    let result = match target {
        Target::Virtual(v) => Ok(AudioHost::start_virtual(&v, processor)),
        Target::Device(cfg) => AudioHost::start_or_return(&cfg, processor),
    };
    match result {
        Ok(host) => {
            let line = crate::engine::device_line(host.info());
            let name = host.info().device_name.clone();
            let was_lost = is_lost();
            {
                let mut h = app.engine.borrow_mut();
                h.message = line;
                h.host = Some(host);
            }
            started(app);
            crate::engine::show_message(app);
            if was_lost {
                app.status(od_i18n::trf("Audio back on {}", &[&name]));
            } else {
                app.status(od_i18n::trf("Audio output: {}", &[&name]));
            }
            true
        }
        Err(e) => {
            tracing::warn!(error = %e, "audio output could not start");
            app.engine.borrow_mut().parked = e.processor;
            app.status(od_i18n::trf("Audio output could not start: {}", &[&e.error.to_string()]));
            refresh(app);
            false
        }
    }
}

/// TRY AGAIN: the lost device (or the virtual output) once more.
pub(crate) fn retry(app: &App) {
    let lost = WATCH.with(|w| {
        w.borrow()
            .lost
            .as_ref()
            .map(|l| (l.device.clone(), l.virtual_out))
    });
    let target = match lost {
        Some((name, true)) => Target::Virtual(od_audio_io::VirtualOutput::new(name, 48_000)),
        Some((name, false)) => {
            let mut cfg = configured(app);
            cfg.device = Some(name);
            cfg.fallback_to_default = false;
            Target::Device(cfg)
        }
        None => Target::Device(configured(app)),
    };
    reopen(app, target);
}

/// The output offered instead of the lost one: the system's default
/// output, or nothing when that is the lost device itself.
fn fallback_name(lost: &Lost) -> Option<String> {
    if lost.virtual_out {
        return Some(crate::engine::null_output().name);
    }
    od_audio_io::default_output_device_name(od_audio_io::Backend::Default)
        .filter(|n| *n != lost.device)
}

/// SWITCH OUTPUT: go on on the system's default output.
pub(crate) fn use_fallback(app: &App) {
    let target = WATCH.with(|w| {
        let w = w.borrow();
        let l = w.lost.as_ref()?;
        Some(if l.virtual_out {
            Target::Virtual(crate::engine::null_output())
        } else {
            let mut cfg = configured(app);
            cfg.device = fallback_name(l);
            cfg.routing = None;
            cfg.fallback_to_default = true;
            Target::Device(cfg)
        })
    });
    if let Some(t) = target {
        reopen(app, t);
    }
}

/// Preferences > Audio changed: move the running engine onto the chosen
/// device at once (the sample rate still applies after a restart).
pub(crate) fn apply_settings(app: &App) {
    let running = app
        .engine
        .borrow()
        .host
        .as_ref()
        .map(|h| h.is_virtual());
    match running {
        Some(false) => {
            reopen(app, Target::Device(configured(app)));
            crate::preferences::refresh_live(app);
        }
        // Headless, not started yet, or lost (the next try uses the setting).
        _ => {}
    }
}

/// Test and demo hook: the running output loses its device.
pub(crate) fn inject_loss(app: &App) {
    if let Some(h) = app.engine.borrow().host.as_ref() {
        h.fault_hook().lose_device();
    }
}

/// Projects the state into the status band and the alert.
pub(crate) fn refresh(app: &App) {
    let g = app.ui.global::<crate::AudioHealth>();
    let lost = WATCH.with(|w| {
        w.borrow()
            .lost
            .as_ref()
            .map(|l| (l.device.clone(), fallback_name(l).unwrap_or_default()))
    });
    if let Some((device, fallback)) = lost {
        g.set_state("lost".into());
        g.set_device(device.clone().into());
        g.set_lost_device(device.into());
        g.set_fallback(fallback.into());
        g.set_latency("".into());
        g.set_dropouts(dropouts(app) as i32);
        return;
    }
    g.set_lost_device("".into());
    let h = app.engine.borrow();
    let Some(host) = h.host.as_ref() else {
        g.set_state("off".into());
        g.set_device("".into());
        g.set_latency("".into());
        return;
    };
    let device = host.info().device_name.clone();
    let lat = latency(host).map(latency_text).unwrap_or_default();
    drop(h);
    let total = dropouts(app);
    let recent = WATCH.with(|w| {
        let mut w = w.borrow_mut();
        if total > w.shown_dropouts {
            w.last_dropout = Some(Instant::now());
        }
        w.shown_dropouts = total;
        w.last_dropout.is_some_and(|t| t.elapsed() < DROPOUT_HOLD)
    });
    g.set_state(if recent { "dropouts" } else { "ok" }.into());
    g.set_device(device.into());
    g.set_latency(lat.into());
    g.set_dropouts(total as i32);
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::AudioHealth>();
    g.on_retry(handler!(app, |a| retry(&a)));
    g.on_use_fallback(handler!(app, |a| use_fallback(&a)));
    refresh(app);
    if od_core::env::var("DEMO_SAFETY").is_some_and(|v| v.split(',').any(|p| p == "device-lost")) {
        // Documentation screenshots: the device goes once audio runs.
        let weak = Rc::downgrade(app);
        slint::Timer::single_shot(Duration::from_secs(6), move || {
            if let Some(a) = weak.upgrade() {
                inject_loss(&a);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn latency_is_shown_in_milliseconds() {
        assert_eq!(super::latency_text(std::time::Duration::from_micros(5333)), "5.3 ms");
    }
}
