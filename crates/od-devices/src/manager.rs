//! Background service that watches volumes and keeps registered sticks in
//! sync automatically.
//!
//! The manager owns its own library connection and runs on its own thread.
//! It polls the volume list and the library change log; when a connected
//! stick is out of date and the library has been quiet for the debounce
//! interval, it syncs the stick in the background.

use crate::sync::{FormatWriter, NativeWriter, Progress, SyncOptions, SyncReport, sync_device};
use crate::volumes::{self, Volume};
use crossbeam_channel::{Receiver, Sender, select, tick, unbounded};
use od_core::DeviceId;
use od_library::{Device, DeviceFormats, Library, NodeId};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceState {
    Disconnected,
    InSync,
    OutOfDate { pending: usize },
    Syncing(Progress),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStatus {
    pub device: Device,
    pub volume: Option<Volume>,
    pub state: DeviceState,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// Current status of every registered device.
    Status(Vec<DeviceStatus>),
    /// Mounted removable volumes that are not registered.
    UnregisteredVolumes(Vec<Volume>),
    SyncFinished {
        device: DeviceId,
        report: SyncReport,
    },
    SyncFailed {
        device: DeviceId,
        error: String,
    },
    Ejected {
        device: DeviceId,
    },
    EjectFailed {
        device: DeviceId,
        error: String,
    },
    Registered {
        device: DeviceId,
    },
    /// Changes made on the stick by players (cues, history) to review.
    Proposals {
        device: DeviceId,
        proposals: Vec<crate::club_usb::Proposal>,
    },
}

#[derive(Clone, Debug)]
pub enum Command {
    /// Registers the volume at `mount_point` as a device with the given
    /// content and syncs it.
    Register {
        mount_point: PathBuf,
        name: String,
        nodes: Vec<NodeId>,
        formats: DeviceFormats,
    },
    SyncNow(DeviceId),
    /// Syncs every connected registered stick at once, each on its own
    /// thread. With `eject` a stick is ejected as soon as its own sync has
    /// finished. A stick that fails shows its error; the others carry on.
    SyncAll {
        eject: bool,
    },
    /// Rewrites everything on the stick.
    Rebuild(DeviceId),
    Eject(DeviceId),
    SetAutoSync(bool),
    /// Re-read volumes and states now.
    Refresh,
    Shutdown,
}

/// Produces the format writers for a device's enabled formats.
pub type WriterFactory =
    Arc<dyn Fn(&DeviceFormats) -> Vec<Box<dyn FormatWriter + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct ManagerConfig {
    pub library_path: PathBuf,
    pub poll_interval: Duration,
    /// Quiet time after the last library change before an automatic sync.
    pub debounce: Duration,
    pub auto_sync: bool,
    pub writers: WriterFactory,
}

impl ManagerConfig {
    pub fn new(library_path: PathBuf) -> Self {
        Self {
            library_path,
            poll_interval: Duration::from_millis(1000),
            debounce: Duration::from_secs(2),
            auto_sync: true,
            writers: Arc::new(|f: &DeviceFormats| {
                let mut w: Vec<Box<dyn FormatWriter + Send>> = Vec::new();
                if f.opendjdecks {
                    w.push(Box::new(NativeWriter::default()));
                }
                if f.club_usb {
                    w.push(Box::new(crate::club_usb::ClubUsbWriter::default()));
                }
                w
            }),
        }
    }
}

/// Handle to the background device service.
pub struct DeviceManager {
    commands: Sender<Command>,
    events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DeviceManager {
    pub fn start(config: ManagerConfig) -> od_library::Result<Self> {
        let library = Library::open(&config.library_path)?;
        let (cmd_tx, cmd_rx) = unbounded();
        let (ev_tx, ev_rx) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker = Worker {
            library,
            auto_sync: config.auto_sync,
            config,
            events: ev_tx,
            cancel: cancel.clone(),
            last_seq: -1,
            last_change: Instant::now(),
            errors: Default::default(),
            last_status: Vec::new(),
            last_unregistered: Vec::new(),
        };
        let thread = std::thread::Builder::new()
            .name("od-devices".into())
            .spawn(move || worker.run(cmd_rx))
            .map_err(od_library::LibraryError::Io)?;
        Ok(Self {
            commands: cmd_tx,
            events: ev_rx,
            cancel,
            thread: Some(thread),
        })
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.commands.send(cmd);
    }

    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }

    /// Cancels a running sync (the stick stays consistent; the next sync is
    /// a full one).
    pub fn cancel_sync(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for DeviceManager {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.commands.send(Command::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Worker {
    library: Library,
    config: ManagerConfig,
    auto_sync: bool,
    events: Sender<Event>,
    cancel: Arc<AtomicBool>,
    last_seq: i64,
    last_change: Instant,
    errors: std::collections::HashMap<DeviceId, String>,
    last_status: Vec<DeviceStatus>,
    last_unregistered: Vec<Volume>,
}

fn new_device_uuid(mount_point: &Path) -> String {
    let mut h = blake3::Hasher::new();
    h.update(mount_point.as_os_str().as_encoded_bytes());
    h.update(&std::process::id().to_le_bytes());
    h.update(
        &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_le_bytes(),
    );
    h.finalize().to_hex()[..32].to_string()
}

impl Worker {
    fn run(mut self, commands: Receiver<Command>) {
        let ticker = tick(self.config.poll_interval);
        self.poll();
        loop {
            select! {
                recv(commands) -> cmd => match cmd {
                    Ok(Command::Shutdown) | Err(_) => break,
                    Ok(cmd) => self.handle(cmd),
                },
                recv(ticker) -> _ => self.poll(),
            }
        }
    }

    fn emit(&self, e: Event) {
        let _ = self.events.send(e);
    }

    /// Registered devices joined with mounted volumes.
    fn connected(&self) -> (Vec<(Device, Option<Volume>)>, Vec<Volume>) {
        let devices = self.library.devices().unwrap_or_default();
        let mut joined: Vec<(Device, Option<Volume>)> =
            devices.into_iter().map(|d| (d, None)).collect();
        let mut unregistered = Vec::new();
        for v in volumes::list() {
            let marker = od_format_native::read_marker(&v.mount_point).ok().flatten();
            match marker.and_then(|m| {
                joined
                    .iter_mut()
                    .find(|(d, _)| d.volume_uuid == m.device_uuid)
            }) {
                Some(slot) => slot.1 = Some(v),
                None => unregistered.push(v),
            }
        }
        (joined, unregistered)
    }

    fn poll(&mut self) {
        let seq = self.library.change_seq().unwrap_or(0);
        if seq != self.last_seq {
            self.last_seq = seq;
            self.last_change = Instant::now();
        }
        let quiet = self.last_change.elapsed() >= self.config.debounce;
        let (joined, unregistered) = self.connected();
        let mut statuses = Vec::with_capacity(joined.len());
        let mut to_sync = Vec::new();
        for (device, volume) in joined {
            let state = match &volume {
                None => DeviceState::Disconnected,
                Some(_) => match self.errors.get(&device.id) {
                    Some(e) => DeviceState::Error(e.clone()),
                    None => match self.library.pending_changes(device.id) {
                        Ok(p) if p.is_empty() => DeviceState::InSync,
                        Ok(p) => {
                            if self.auto_sync && quiet {
                                to_sync.push(device.id);
                            }
                            DeviceState::OutOfDate { pending: p.count }
                        }
                        Err(e) => DeviceState::Error(e.to_string()),
                    },
                },
            };
            if volume.is_none() {
                // Errors are cleared when the stick is unplugged.
                self.errors.remove(&device.id);
            }
            statuses.push(DeviceStatus {
                device,
                volume,
                state,
            });
        }
        if statuses != self.last_status {
            self.last_status = statuses.clone();
            self.emit(Event::Status(statuses));
        }
        if unregistered != self.last_unregistered {
            self.last_unregistered = unregistered.clone();
            self.emit(Event::UnregisteredVolumes(unregistered));
        }
        for id in to_sync {
            self.sync(id, false);
        }
    }

    fn volume_of(&self, id: DeviceId) -> Option<Volume> {
        self.connected()
            .0
            .into_iter()
            .find(|(d, _)| d.id == id)
            .and_then(|(_, v)| v)
    }

    fn set_state(&mut self, id: DeviceId, state: DeviceState) {
        if let Some(s) = self.last_status.iter_mut().find(|s| s.device.id == id) {
            s.state = state;
        }
        self.emit(Event::Status(self.last_status.clone()));
    }

    fn sync(&mut self, id: DeviceId, full: bool) {
        let Some(volume) = self.volume_of(id) else {
            return;
        };
        let Ok(device) = self.library.device(id) else {
            return;
        };
        self.cancel.store(false, Ordering::Relaxed);
        let writers = (self.config.writers)(&device.formats);
        let events = self.events.clone();
        let mut status = self.last_status.clone();
        let result = run_sync(
            &mut self.library,
            writers,
            id,
            &volume.mount_point,
            full,
            &self.cancel,
            &mut |state| {
                if let Some(s) = status.iter_mut().find(|s| s.device.id == id) {
                    s.state = state;
                }
                let _ = events.send(Event::Status(status.clone()));
            },
        );
        let (state, error) = finish_sync(&mut self.library, &self.events, id, result);
        match error {
            Some(e) => self.errors.insert(id, e),
            None => self.errors.remove(&id),
        };
        self.set_state(id, state);
        // Force the next poll to re-evaluate.
        self.last_status.clear();
    }

    /// Syncs every connected stick in parallel; returns when all are done.
    fn sync_all(&mut self, eject: bool) {
        let sticks: Vec<(Device, Volume)> = self
            .connected()
            .0
            .into_iter()
            .filter_map(|(d, v)| v.map(|v| (d, v)))
            .collect();
        if sticks.is_empty() {
            return;
        }
        self.cancel.store(false, Ordering::Relaxed);
        let status = Mutex::new(self.last_status.clone());
        let ids: Vec<DeviceId> = sticks.iter().map(|(d, _)| d.id).collect();
        for id in &ids {
            self.errors.remove(id);
        }
        let (config, events, cancel) = (&self.config, &self.events, &self.cancel);
        let outcomes: Vec<Option<String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = sticks
                .into_iter()
                .map(|(device, volume)| {
                    let status = &status;
                    scope.spawn(move || {
                        sync_stick(config, device, volume, eject, events, status, cancel)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|_| Some("sync crashed".to_string())))
                .collect()
        });
        for (id, error) in ids.into_iter().zip(outcomes) {
            if let Some(e) = error {
                self.errors.insert(id, e);
            }
        }
        // Force the next poll to re-evaluate.
        self.last_status.clear();
        self.poll();
    }

    fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::Register {
                mount_point,
                name,
                nodes,
                formats,
            } => {
                let existing = od_format_native::read_marker(&mount_point).ok().flatten();
                let uuid = existing
                    .map(|m| m.device_uuid)
                    .unwrap_or_else(|| new_device_uuid(&mount_point));
                let marker = od_format_native::DeviceMarker {
                    format_version: od_format_native::FORMAT_VERSION,
                    device_uuid: uuid.clone(),
                    name: name.clone(),
                    created_by: format!("OpenDJDecks {}", env!("CARGO_PKG_VERSION")),
                };
                let result = od_format_native::write_marker(&mount_point, &marker)
                    .map_err(|e| e.to_string())
                    .and_then(|_| {
                        self.library
                            .register_device(&uuid, &name, formats)
                            .map_err(|e| e.to_string())
                    })
                    .and_then(|id| {
                        self.library
                            .set_device_nodes(id, &nodes)
                            .map(|_| id)
                            .map_err(|e| e.to_string())
                    });
                match result {
                    Ok(id) => {
                        self.emit(Event::Registered { device: id });
                        self.last_status.clear();
                        self.poll();
                        self.sync(id, true);
                    }
                    Err(e) => tracing::warn!(error = %e, "registering device failed"),
                }
            }
            Command::SyncNow(id) => {
                self.errors.remove(&id);
                self.sync(id, false);
            }
            Command::SyncAll { eject } => self.sync_all(eject),
            Command::Rebuild(id) => {
                self.errors.remove(&id);
                let _ = self.library.reset_device_sync(id);
                self.sync(id, true);
            }
            Command::Eject(id) => match self.volume_of(id) {
                Some(v) => match volumes::eject(&v) {
                    Ok(()) => {
                        self.emit(Event::Ejected { device: id });
                        self.last_status.clear();
                        self.poll();
                    }
                    Err(e) => self.emit(Event::EjectFailed {
                        device: id,
                        error: e.to_string(),
                    }),
                },
                None => self.emit(Event::Ejected { device: id }),
            },
            Command::SetAutoSync(on) => self.auto_sync = on,
            Command::Refresh => {
                self.last_status.clear();
                self.last_unregistered.clear();
                self.poll();
            }
            Command::Shutdown => {}
        }
    }
}

/// One sync with its progress reported (at most ~10 times per second).
fn run_sync(
    lib: &mut Library,
    mut writers: Vec<Box<dyn FormatWriter + Send>>,
    id: DeviceId,
    root: &Path,
    full: bool,
    cancel: &AtomicBool,
    state: &mut dyn FnMut(DeviceState),
) -> crate::sync::Result<SyncReport> {
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    sync_device(
        lib,
        id,
        root,
        &mut writers,
        &SyncOptions { force_full: full },
        cancel,
        |p| {
            // Rate-limit progress events to ~10 per second.
            if last_emit.elapsed() < Duration::from_millis(100)
                && matches!(p, Progress::Copying { .. })
            {
                return;
            }
            last_emit = Instant::now();
            state(DeviceState::Syncing(p));
        },
    )
}

/// Events for a finished sync. Returns the stick's new state and the error
/// to keep showing on it.
fn finish_sync(
    lib: &mut Library,
    events: &Sender<Event>,
    id: DeviceId,
    result: crate::sync::Result<SyncReport>,
) -> (DeviceState, Option<String>) {
    match result {
        Ok(report) => {
            if !report.proposals.is_empty() {
                let _ = events.send(Event::Proposals {
                    device: id,
                    proposals: report.proposals.clone(),
                });
            }
            tracing::info!(device = %id, copied = report.copied, updated = report.updated, "sync finished");
            let _ = events.send(Event::SyncFinished { device: id, report });
            (DeviceState::InSync, None)
        }
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!(device = %id, error = %msg, "sync failed");
            let _ = lib.mark_device_error(id, &msg);
            let kept = (!matches!(e, crate::SyncError::Cancelled)).then(|| msg.clone());
            let _ = events.send(Event::SyncFailed {
                device: id,
                error: msg.clone(),
            });
            (DeviceState::Error(msg), kept)
        }
    }
}

/// One stick of a parallel sync, on its own thread with its own library
/// connection. Returns the error to keep showing on the stick.
fn sync_stick(
    config: &ManagerConfig,
    device: Device,
    volume: Volume,
    eject: bool,
    events: &Sender<Event>,
    status: &Mutex<Vec<DeviceStatus>>,
    cancel: &AtomicBool,
) -> Option<String> {
    let id = device.id;
    let publish = |state: DeviceState| {
        let mut all = status.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = all.iter_mut().find(|s| s.device.id == id) {
            s.state = state;
        }
        let _ = events.send(Event::Status(all.clone()));
    };
    let mut lib = match Library::open(&config.library_path) {
        Ok(lib) => lib,
        Err(e) => {
            let msg = e.to_string();
            let _ = events.send(Event::SyncFailed {
                device: id,
                error: msg.clone(),
            });
            publish(DeviceState::Error(msg.clone()));
            return Some(msg);
        }
    };
    let writers = (config.writers)(&device.formats);
    let mut on_state = |state| publish(state);
    let result = run_sync(
        &mut lib,
        writers,
        id,
        &volume.mount_point,
        false,
        cancel,
        &mut on_state,
    );
    let synced = result.is_ok();
    let (state, error) = finish_sync(&mut lib, events, id, result);
    publish(state);
    if eject && synced {
        // Only a stick whose sync finished is ejected; a failed one stays
        // mounted so its error can be read and the sync retried.
        match volumes::eject(&volume) {
            Ok(()) => {
                let _ = events.send(Event::Ejected { device: id });
            }
            Err(e) => {
                let _ = events.send(Event::EjectFailed {
                    device: id,
                    error: e.to_string(),
                });
            }
        }
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::SyncContext;
    use od_core::{Cue, TrackTags};
    use od_library::{NewTrack, NodeKind};
    use std::sync::atomic::AtomicUsize;

    /// The tests here share the OPENDJDECKS_VOLUMES_DIR test hook; one at a
    /// time.
    static VOLUMES_ENV: Mutex<()> = Mutex::new(());

    // Uses the OPENDJDECKS_VOLUMES_DIR test hook; the tests touching the
    // environment in this crate hold `VOLUMES_ENV`.
    #[test]
    fn registers_and_auto_syncs() {
        let _env = VOLUMES_ENV.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let volumes_dir = dir.path().join("volumes");
        let stick = volumes_dir.join("STICK");
        std::fs::create_dir_all(&stick).unwrap();
        // SAFETY: tests in this crate do not read this variable concurrently
        // except through this manager.
        unsafe { std::env::set_var("OPENDJDECKS_VOLUMES_DIR", &volumes_dir) };

        let lib_path = dir.path().join("library.sqlite");
        let mut lib = Library::open(&lib_path).unwrap();
        let file = dir.path().join("a.mp3");
        std::fs::write(&file, vec![1u8; 4096]).unwrap();
        let a = lib
            .add_track(&NewTrack {
                content_hash: od_decode::content_hash(&file).unwrap(),
                file_size: 4096,
                path: file,
                format: None,
                duration: 1.0,
                sample_rate: 44_100,
                channels: 2,
                bit_rate: None,
                bits_per_sample: None,
                tags: TrackTags {
                    title: "A".into(),
                    ..Default::default()
                },
                artwork: None,
            })
            .unwrap();
        let pl = lib.create_node(None, NodeKind::Playlist, "P").unwrap();
        lib.add_to_node(pl, &[a], None).unwrap();

        let mut cfg = ManagerConfig::new(lib_path);
        cfg.poll_interval = Duration::from_millis(50);
        cfg.debounce = Duration::from_millis(100);
        let mgr = DeviceManager::start(cfg).unwrap();
        let wait_for = |pred: &dyn Fn(&Event) -> bool| {
            let deadline = Instant::now() + Duration::from_secs(30);
            while Instant::now() < deadline {
                if let Ok(e) = mgr.events().recv_timeout(Duration::from_millis(100)) {
                    if pred(&e) {
                        return e;
                    }
                }
            }
            panic!("timed out waiting for event");
        };
        wait_for(&|e| matches!(e, Event::UnregisteredVolumes(v) if v.len() == 1));
        mgr.send(Command::Register {
            mount_point: stick.clone(),
            name: "Stick".into(),
            nodes: vec![pl],
            formats: DeviceFormats::default(),
        });
        wait_for(&|e| matches!(e, Event::SyncFinished { report, .. } if report.copied == 1));

        // Edit a cue in the app's own connection; the manager picks it up.
        lib.set_cues(a, vec![Cue::hot(1, 4.0)]).unwrap();
        wait_for(
            &|e| matches!(e, Event::SyncFinished { report, .. } if report.updated == 1 && report.copied == 0),
        );
        let s = od_format_native::Stick::open(&stick).unwrap().unwrap();
        assert_eq!(s.library.cues(a).unwrap()[0].start, 4.0);
    }

    /// Sticks sync at the same time: the writer waits until both are inside it.
    struct MeetWriter {
        inside: Arc<AtomicUsize>,
    }

    impl FormatWriter for MeetWriter {
        fn name(&self) -> &'static str {
            "meet"
        }
        fn write(&mut self, _ctx: &SyncContext) -> Result<(), String> {
            self.inside.fetch_add(1, Ordering::SeqCst);
            let deadline = Instant::now() + Duration::from_secs(20);
            while self.inside.load(Ordering::SeqCst) < 2 {
                if Instant::now() > deadline {
                    return Err("the other stick never started".into());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(())
        }
    }

    struct FailingWriter;

    impl FormatWriter for FailingWriter {
        fn name(&self) -> &'static str {
            "broken"
        }
        fn write(&mut self, _ctx: &SyncContext) -> Result<(), String> {
            Err("this stick is broken".into())
        }
    }

    #[test]
    fn sync_all_runs_sticks_together_and_a_failure_stays_on_its_stick() {
        let _env = VOLUMES_ENV.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let volumes_dir = dir.path().join("volumes");
        // SAFETY: `VOLUMES_ENV` keeps the tests in this crate from racing.
        unsafe { std::env::set_var("OPENDJDECKS_VOLUMES_DIR", &volumes_dir) };

        let lib_path = dir.path().join("library.sqlite");
        let mut lib = Library::open(&lib_path).unwrap();
        let file = dir.path().join("a.mp3");
        std::fs::write(&file, vec![1u8; 4096]).unwrap();
        let a = lib
            .add_track(&NewTrack {
                content_hash: od_decode::content_hash(&file).unwrap(),
                file_size: 4096,
                path: file,
                format: None,
                duration: 1.0,
                sample_rate: 44_100,
                channels: 2,
                bit_rate: None,
                bits_per_sample: None,
                tags: TrackTags::default(),
                artwork: None,
            })
            .unwrap();
        let pl = lib.create_node(None, NodeKind::Playlist, "P").unwrap();
        lib.add_to_node(pl, &[a], None).unwrap();
        // The club format selects the broken writer in the factory below.
        let mut sticks = Vec::new();
        for (name, club_usb) in [("ONE", false), ("TWO", false), ("BAD", true)] {
            let stick = volumes_dir.join(name);
            std::fs::create_dir_all(&stick).unwrap();
            let uuid = format!("uuid-{name}");
            let marker = od_format_native::DeviceMarker {
                format_version: od_format_native::FORMAT_VERSION,
                device_uuid: uuid.clone(),
                name: name.into(),
                created_by: "test".into(),
            };
            od_format_native::write_marker(&stick, &marker).unwrap();
            let id = lib
                .register_device(
                    &uuid,
                    name,
                    DeviceFormats {
                        club_usb,
                        opendjdecks: !club_usb,
                        ..Default::default()
                    },
                )
                .unwrap();
            lib.set_device_nodes(id, &[pl]).unwrap();
            sticks.push(id);
        }
        let (one, two, bad) = (sticks[0], sticks[1], sticks[2]);

        let inside = Arc::new(AtomicUsize::new(0));
        let inside2 = inside.clone();
        let mut cfg = ManagerConfig::new(lib_path);
        cfg.poll_interval = Duration::from_millis(50);
        cfg.auto_sync = false;
        cfg.writers = Arc::new(move |f: &DeviceFormats| {
            let w: Box<dyn FormatWriter + Send> = if f.club_usb {
                Box::new(FailingWriter)
            } else {
                Box::new(MeetWriter {
                    inside: inside2.clone(),
                })
            };
            vec![w]
        });
        let mgr = DeviceManager::start(cfg).unwrap();
        // All three sticks seen as connected.
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if let Ok(Event::Status(s)) = mgr.events().recv_timeout(Duration::from_millis(100)) {
                if s.iter().filter(|d| d.volume.is_some()).count() == 3 {
                    break;
                }
            }
        }
        mgr.send(Command::SyncAll { eject: true });

        let (mut finished, mut failed, mut ejected) = (Vec::new(), Vec::new(), Vec::new());
        let mut last_status: Vec<DeviceStatus> = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(60);
        while Instant::now() < deadline && (finished.len() < 2 || failed.is_empty() || ejected.len() < 2) {
            match mgr.events().recv_timeout(Duration::from_millis(100)) {
                Ok(Event::SyncFinished { device, .. }) => finished.push(device),
                Ok(Event::SyncFailed { device, error }) => failed.push((device, error)),
                Ok(Event::Ejected { device }) => ejected.push(device),
                Ok(Event::Status(s)) => last_status = s,
                _ => {}
            }
        }
        finished.sort();
        ejected.sort();
        let mut expected = vec![one, two];
        expected.sort();
        assert_eq!(finished, expected, "both good sticks finished together");
        assert_eq!(ejected, expected, "each finished stick is ejected");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, bad);
        assert!(failed[0].1.contains("broken"), "{}", failed[0].1);
        // The error stays visible on the stick that failed, and only there.
        let deadline = Instant::now() + Duration::from_secs(10);
        let state_of = |all: &[DeviceStatus], id| {
            all.iter()
                .find(|d| d.device.id == id)
                .map(|d| d.state.clone())
        };
        while !matches!(state_of(&last_status, bad), Some(DeviceState::Error(_))) {
            assert!(Instant::now() < deadline, "error shown on the failed stick");
            if let Ok(Event::Status(s)) = mgr.events().recv_timeout(Duration::from_millis(100)) {
                last_status = s;
            }
        }
        assert!(
            !matches!(state_of(&last_status, one), Some(DeviceState::Error(_))),
            "the other sticks do not show the error"
        );
    }
}
