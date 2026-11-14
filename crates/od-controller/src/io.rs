//! Real MIDI I/O through `midir` (CoreMIDI on macOS, ALSA on Linux).
//!
//! [`ControllerManager`] owns a controller thread. It periodically rescans
//! MIDI ports (hot-plug), connects ports matching a mapping, turns incoming
//! messages into [`ControllerEvent::Action`]s, and renders feedback pushed
//! with [`ControllerManager::send_feedback`] into diffed, rate-limited MIDI.
//! All midir objects live on the controller thread.

use crate::action::ControlAction;
use crate::feedback::ControlFeedback;
use crate::learn::LearnSession;
use crate::mapping::Mapping;
use crate::monitor::{Direction, MidiMonitor};
use crate::session::ControllerSession;
use crossbeam_channel::{Receiver, Sender};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const CLIENT: &str = "OpenDJDecks";

/// CoreMIDI tells a process about devices plugged in or out through the run
/// loop of the thread that created the process's first MIDI client; until
/// that run loop runs, every port list stays as it was at that first client.
/// The controller thread creates the first client and runs its run loop
/// before each rescan, so a controller plugged in later is found.
#[cfg(target_os = "macos")]
mod run_loop {
    use std::ffi::c_void;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopDefaultMode: *const c_void;
        fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source: u8) -> i32;
    }

    /// Handles what is pending on this thread's run loop, without waiting.
    pub(super) fn pump() {
        // SAFETY: the mode is a CoreFoundation constant; a zero-second run of
        // the current thread's run loop has no other preconditions.
        unsafe {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.0, 0);
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod run_loop {
    pub(super) fn pump() {}
}

/// MIDI port names currently present.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PortList {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

pub fn list_ports() -> Result<PortList, String> {
    let mi = MidiInput::new(CLIENT).map_err(|e| e.to_string())?;
    let mo = MidiOutput::new(CLIENT).map_err(|e| e.to_string())?;
    Ok(PortList {
        inputs: mi
            .ports()
            .iter()
            .filter_map(|p| mi.port_name(p).ok())
            .collect(),
        outputs: mo
            .ports()
            .iter()
            .filter_map(|p| mo.port_name(p).ok())
            .collect(),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControllerEvent {
    Connected {
        device: String,
        port: String,
        usb: Option<(u16, u16)>,
        /// The mapping has physical channel faders (their positions are
        /// unknown until they move).
        channel_faders: bool,
        /// Decks the controller has (its mapping's `device.decks`).
        decks: u8,
        /// The mapping asks the unit for its control positions on connecting
        /// (`[[init]]` or `[[keepalive]]`): the faders report without moving.
        asks_positions: bool,
    },
    Disconnected {
        device: String,
        port: String,
    },
    /// `timestamp_us` is the backend timestamp of the triggering message.
    Action {
        device: String,
        action: ControlAction,
        timestamp_us: u64,
    },
    /// A note played on a MIDI input that no mapping knows (roadmap [82]:
    /// a keyboard for CREATE). Such ports are open only while
    /// [`ControllerManager::set_open_unmapped`] is on; a port with a
    /// mapping (a DJ controller, or one given to the lights) never sends
    /// this, its notes are its mapping's.
    Note {
        port: String,
        note: u8,
        velocity: u8,
        timestamp_us: u64,
    },
    /// A MIDI input no mapping knows appeared (once until it is unplugged):
    /// it can be set up with the guided setup or MIDI learn.
    Unmapped { port: String },
    Error(String),
}

#[derive(Clone, Debug)]
pub struct ManagerConfig {
    pub rescan_interval: Duration,
    /// Output poll / loop period.
    pub tick: Duration,
    /// Also open input ports without a mapping (for the monitor and learn).
    pub monitor_unmapped_ports: bool,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            rescan_interval: Duration::from_secs(2),
            tick: Duration::from_millis(5),
            monitor_unmapped_ports: false,
        }
    }
}

struct RawIn {
    port: String,
    stamp: u64,
    bytes: Vec<u8>,
}

struct Connected {
    _input: MidiInputConnection<()>,
    output: Option<MidiOutputConnection>,
    session: Option<ControllerSession>,
    device: String,
}

pub struct ControllerManager {
    events: Receiver<ControllerEvent>,
    feedback: Sender<(Option<String>, ControlFeedback)>,
    lights: Sender<(Option<String>, bool)>,
    monitor: MidiMonitor,
    learn: Arc<Mutex<Option<LearnSession>>>,
    stop: Arc<AtomicBool>,
    /// Open ports without a mapping too (MIDI learn, the guided setup);
    /// switched at run time, the next scan follows at once.
    unmapped: Arc<AtomicBool>,
    rescan_now: Arc<AtomicBool>,
    /// Ports given a mapping by hand: (port name, mapping id).
    assigned: Arc<Mutex<Vec<(String, String)>>>,
    thread: Option<JoinHandle<()>>,
}

impl ControllerManager {
    pub fn start(mappings: Vec<Arc<Mapping>>, config: ManagerConfig) -> Self {
        let (ev_tx, ev_rx) = crossbeam_channel::unbounded();
        let (fb_tx, fb_rx) = crossbeam_channel::unbounded();
        let (lights_tx, lights_rx) = crossbeam_channel::unbounded();
        let monitor = MidiMonitor::default();
        let learn = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let unmapped = Arc::new(AtomicBool::new(config.monitor_unmapped_ports));
        let rescan_now = Arc::new(AtomicBool::new(false));
        let assigned = Arc::new(Mutex::new(Vec::new()));
        let (ready_tx, ready_rx) = crossbeam_channel::bounded::<()>(1);
        let thread = {
            let monitor = monitor.clone();
            let learn = learn.clone();
            let stop = stop.clone();
            let flags = (unmapped.clone(), rescan_now.clone(), assigned.clone());
            std::thread::Builder::new()
                .name("od-controller".into())
                .spawn(move || {
                    // The process's first MIDI client, kept while the thread
                    // runs: plug events then reach this thread (`run_loop`).
                    let _first = MidiInput::new(CLIENT).ok();
                    let _ = ready_tx.send(());
                    run(mappings, config, ev_tx, fb_rx, lights_rx, monitor, learn, stop, flags)
                })
                .expect("spawn controller thread")
        };
        // No other thread lists ports before the first client exists.
        let _ = ready_rx.recv_timeout(Duration::from_secs(2));
        Self {
            events: ev_rx,
            feedback: fb_tx,
            lights: lights_tx,
            monitor,
            learn,
            stop,
            unmapped,
            rescan_now,
            assigned,
            thread: Some(thread),
        }
    }

    /// Gives the MIDI input `port` the mapping `mapping` (by id), or back
    /// to the mapping that matches it by name (`None`). A port already
    /// open is opened again with its new mapping. Used to give one
    /// controller to the lights while another drives the decks.
    pub fn assign(&self, port: &str, mapping: Option<&str>) {
        let mut a = self.assigned.lock().unwrap();
        let before = a.clone();
        a.retain(|(p, _)| p != port);
        if let Some(id) = mapping.filter(|_| !port.is_empty()) {
            a.push((port.to_string(), id.to_string()));
        }
        if *a != before {
            self.rescan_now.store(true, Ordering::Relaxed);
        }
    }

    /// The ports given a mapping by hand.
    pub fn assigned(&self) -> Vec<(String, String)> {
        self.assigned.lock().unwrap().clone()
    }

    /// Opens (or closes again) the MIDI inputs that no mapping knows, so
    /// MIDI learn and the guided setup hear a controller without a mapping.
    /// Closed by default: on some systems an open port is not available to
    /// other programs.
    pub fn set_open_unmapped(&self, on: bool) {
        if self.unmapped.swap(on, Ordering::Relaxed) != on {
            self.rescan_now.store(true, Ordering::Relaxed);
        }
    }

    pub fn events(&self) -> &Receiver<ControllerEvent> {
        &self.events
    }

    /// Pushes feedback to one device (by mapping id) or to all (`None`).
    pub fn send_feedback(&self, device: Option<&str>, fb: ControlFeedback) {
        let _ = self.feedback.send((device.map(str::to_string), fb));
    }

    pub fn monitor(&self) -> &MidiMonitor {
        &self.monitor
    }

    /// Self-test: switches every LED the mapping drives on, or off again
    /// (after which the LEDs show the app state again). One device (by
    /// mapping id) or all (`None`).
    pub fn light_all(&self, device: Option<&str>, on: bool) {
        let _ = self.lights.send((device.map(str::to_string), on));
    }

    /// Starts a MIDI learn session; incoming messages are fed to it.
    pub fn start_learn(&self) {
        *self.learn.lock().unwrap() = Some(LearnSession::new());
    }

    /// Ends the learn session and returns it for [`LearnSession::suggest`].
    pub fn finish_learn(&self) -> Option<LearnSession> {
        self.learn.lock().unwrap().take()
    }
}

impl Drop for ControllerManager {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mappings: Vec<Arc<Mapping>>,
    config: ManagerConfig,
    events: Sender<ControllerEvent>,
    feedback: Receiver<(Option<String>, ControlFeedback)>,
    lights: Receiver<(Option<String>, bool)>,
    monitor: MidiMonitor,
    learn: Arc<Mutex<Option<LearnSession>>>,
    stop: Arc<AtomicBool>,
    (unmapped, rescan_now, assigned): (
        Arc<AtomicBool>,
        Arc<AtomicBool>,
        Arc<Mutex<Vec<(String, String)>>>,
    ),
) {
    let (raw_tx, raw_rx) = crossbeam_channel::unbounded::<RawIn>();
    let mut conns: HashMap<String, Connected> = HashMap::new();
    let mut announced: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut last_scan: Option<Instant> = None;

    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        if rescan_now.swap(false, Ordering::Relaxed)
            || last_scan.is_none_or(|t| now.duration_since(t) >= config.rescan_interval)
        {
            last_scan = Some(now);
            run_loop::pump();
            let open_unmapped = unmapped.load(Ordering::Relaxed);
            if !open_unmapped {
                conns.retain(|_, c| c.session.is_some());
            }
            let assigned = assigned.lock().unwrap().clone();
            rescan(
                &mappings,
                &assigned,
                open_unmapped,
                &mut conns,
                &mut announced,
                &raw_tx,
                &events,
                &monitor,
            );
        }

        // Input.
        while let Ok(raw) = raw_rx.try_recv() {
            if let Some(l) = learn.lock().unwrap().as_mut() {
                l.feed(&raw.bytes);
            }
            let Some(c) = conns.get_mut(&raw.port) else {
                continue;
            };
            match c.session.as_mut() {
                Some(s) => {
                    let p = s.handle_midi(&raw.bytes);
                    monitor.record(&raw.port, Direction::In, &raw.bytes, p.hits);
                    for action in p.actions {
                        let _ = events.send(ControllerEvent::Action {
                            device: c.device.clone(),
                            action,
                            timestamp_us: raw.stamp,
                        });
                    }
                }
                None => {
                    monitor.record(&raw.port, Direction::In, &raw.bytes, Vec::new());
                    if let Some(e) = unmapped_note(&raw.port, raw.stamp, &raw.bytes) {
                        let _ = events.send(e);
                    }
                }
            }
        }

        // Feedback.
        while let Ok((dev, fb)) = feedback.try_recv() {
            for c in conns.values_mut() {
                if dev.as_ref().is_none_or(|d| *d == c.device)
                    && let Some(s) = c.session.as_mut()
                {
                    s.feedback(&fb);
                }
            }
        }

        // Self-test: all lights on or off.
        while let Ok((dev, on)) = lights.try_recv() {
            for (port, c) in conns.iter_mut() {
                if dev.as_ref().is_some_and(|d| *d != c.device) {
                    continue;
                }
                let (Some(s), Some(out)) = (c.session.as_mut(), c.output.as_mut()) else {
                    continue;
                };
                for msg in crate::hardware::led_messages(s.mapping(), on) {
                    if out.send(&msg).is_err() {
                        break;
                    }
                    monitor.record(port, Direction::Out, &msg, Vec::new());
                }
                if !on {
                    s.output_mut().reset();
                }
            }
        }

        // Output.
        for (port, c) in conns.iter_mut() {
            let (Some(s), Some(out)) = (c.session.as_mut(), c.output.as_mut()) else {
                continue;
            };
            for msg in s.poll_output(now) {
                if let Err(e) = out.send(&msg) {
                    let _ = events.send(ControllerEvent::Error(format!("{port}: {e}")));
                    break;
                }
                monitor.record(port, Direction::Out, &msg, Vec::new());
            }
        }

        std::thread::sleep(config.tick);
    }
}

/// What a message from a port without a mapping sends on: a note-on as
/// [`ControllerEvent::Note`] (a note-on at velocity 0 is a note-off, and
/// like every other message it sends nothing).
pub fn unmapped_note(port: &str, stamp: u64, bytes: &[u8]) -> Option<ControllerEvent> {
    match crate::midi::MidiMessage::parse(bytes)? {
        crate::midi::MidiMessage::NoteOn { note, velocity, .. } => Some(ControllerEvent::Note {
            port: port.to_string(),
            note,
            velocity,
            timestamp_us: stamp,
        }),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn rescan(
    mappings: &[Arc<Mapping>],
    assigned: &[(String, String)],
    open_unmapped: bool,
    conns: &mut HashMap<String, Connected>,
    announced: &mut std::collections::HashSet<String>,
    raw_tx: &Sender<RawIn>,
    events: &Sender<ControllerEvent>,
    monitor: &MidiMonitor,
) {
    let ports = match list_ports() {
        Ok(p) => p,
        Err(e) => {
            let _ = events.send(ControllerEvent::Error(format!(
                "MIDI port scan failed: {e}"
            )));
            return;
        }
    };
    // An unplugged unknown input is announced again when it comes back.
    announced.retain(|p| ports.inputs.contains(p));
    // Unplugged ports, and ports whose mapping changed (assigned to the
    // lights or given back): they are opened again below.
    let gone: Vec<String> = conns
        .iter()
        .filter(|(p, c)| {
            let want = Mapping::for_port(mappings, assigned, p).map(|m| m.device().id.as_str());
            !ports.inputs.contains(p) || want.unwrap_or("") != c.device
        })
        .map(|(p, _)| p.clone())
        .collect();
    for p in gone {
        if let Some(c) = conns.remove(&p) {
            tracing::info!("controller port gone: {p}");
            if c.session.is_some() {
                let _ = events.send(ControllerEvent::Disconnected {
                    device: c.device,
                    port: p,
                });
            }
        }
    }
    // New ports.
    let usb = crate::usb::list_usb_devices();
    for name in &ports.inputs {
        if conns.contains_key(name) {
            continue;
        }
        let mapping = Mapping::for_port(mappings, assigned, name).cloned();
        if mapping.is_none() && !name.starts_with(CLIENT) && announced.insert(name.clone()) {
            let _ = events.send(ControllerEvent::Unmapped { port: name.clone() });
        }
        if mapping.is_none() && !open_unmapped {
            continue;
        }
        let input = match connect_input(name, raw_tx.clone()) {
            Ok(i) => i,
            Err(e) => {
                let _ = events.send(ControllerEvent::Error(format!("open {name}: {e}")));
                continue;
            }
        };
        let (session, output, device) = match &mapping {
            Some(m) => {
                let mut session = ControllerSession::new(m.clone());
                let out_name = ports
                    .outputs
                    .iter()
                    .find(|o| *o == name)
                    .or_else(|| ports.outputs.iter().find(|o| m.matches_port(o)));
                let mut output = out_name.and_then(|o| connect_output(o).ok());
                if let Some(out) = output.as_mut() {
                    for msg in session.connect_messages() {
                        let _ = out.send(&msg);
                        monitor.record(name, Direction::Out, &msg, Vec::new());
                    }
                }
                let ids = usb
                    .iter()
                    .find(|d| m.matches_usb(d.vendor_id, d.product_id))
                    .map(|d| (d.vendor_id, d.product_id));
                let device = m.device().id.clone();
                let channel_faders = m.inputs.iter().any(|b| b.template.name == "mixer.fader");
                let _ = events.send(ControllerEvent::Connected {
                    device: device.clone(),
                    port: name.clone(),
                    usb: ids,
                    channel_faders,
                    decks: m.device().decks,
                    asks_positions: !m.file.init.is_empty() || !m.file.keepalive.is_empty(),
                });
                tracing::info!("controller connected: {device} on {name}");
                (Some(session), output, device)
            }
            None => (None, None, String::new()),
        };
        conns.insert(
            name.clone(),
            Connected {
                _input: input,
                output,
                session,
                device,
            },
        );
    }
}

fn connect_input(name: &str, tx: Sender<RawIn>) -> Result<MidiInputConnection<()>, String> {
    let mut mi = MidiInput::new(CLIENT).map_err(|e| e.to_string())?;
    mi.ignore(midir::Ignore::None);
    let port = mi
        .ports()
        .into_iter()
        .find(|p| mi.port_name(p).ok().as_deref() == Some(name))
        .ok_or("port vanished")?;
    let port_name = name.to_string();
    mi.connect(
        &port,
        "opendjdecks-in",
        move |stamp, bytes, _| {
            let _ = tx.send(RawIn {
                port: port_name.clone(),
                stamp,
                bytes: bytes.to_vec(),
            });
        },
        (),
    )
    .map_err(|e| e.to_string())
}

fn connect_output(name: &str) -> Result<MidiOutputConnection, String> {
    connect_output_named(name, "opendjdecks-out")
}

/// Connects the MIDI output port `name` (exact) under the connection name
/// `as_name`.
pub(crate) fn connect_output_named(name: &str, as_name: &str) -> Result<MidiOutputConnection, String> {
    let mo = MidiOutput::new(CLIENT).map_err(|e| e.to_string())?;
    let port = mo
        .ports()
        .into_iter()
        .find(|p| mo.port_name(p).ok().as_deref() == Some(name))
        .ok_or("port vanished")?;
    mo.connect(&port, as_name).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [82] A keyboard without a mapping: its note-ons go on as notes, on
    /// any channel; note-offs (also a note-on at velocity 0), controllers,
    /// clock and broken messages send nothing.
    #[test]
    fn an_unmapped_port_sends_its_note_ons_only() {
        let n = unmapped_note("Keys", 7, &[0x90, 60, 100]);
        assert_eq!(n, Some(ControllerEvent::Note { port: "Keys".into(), note: 60, velocity: 100, timestamp_us: 7 }));
        assert!(matches!(unmapped_note("Keys", 0, &[0x9F, 36, 1]), Some(ControllerEvent::Note { note: 36, .. })), "channel 16");
        for bytes in [&[0x90u8, 60, 0][..], &[0x80, 60, 64], &[0xB0, 7, 100], &[0xF8], &[0xF0, 1, 0xF7], &[0x90, 60], &[]] {
            assert_eq!(unmapped_note("Keys", 0, bytes), None, "{bytes:?}");
        }
    }
}
