//! The emergency stick ([28]): one press copies the open set or playlist
//! and the tracks on the decks to a plugged-in stick, in the formats club
//! players (with WAV for the older ones) and OpenDJDecks play, with its
//! progress and a clear result.
//!
//! The tracks go into a new playlist ("Emergency stick 2026-11-11 22:41").
//! A stick that is not registered yet is registered with that playlist; a
//! registered one gets the playlist beside what it already carries. Then the
//! device service syncs it as any other stick, and the progress and result
//! come from its events.

use crate::app::{App, handler};
use od_core::{DeviceId, TrackId};
use od_devices::{Command, DeviceState, Event};
use od_library::{DeviceFormats, NodeId, NodeKind};
use slint::ComponentHandle;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// How long registering may take before it counts as failed.
const REGISTER_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Stage {
    /// Waiting for the device service to register the stick.
    Registering(Instant),
    Copying,
    Done,
    Failed(String),
}

#[derive(Clone, Debug)]
pub(crate) struct Emergency {
    pub mount: PathBuf,
    pub stick: String,
    pub device: Option<DeviceId>,
    pub node: NodeId,
    pub tracks: usize,
    pub stage: Stage,
    pub progress: f32,
    pub detail: String,
    pub formats: DeviceFormats,
}

/// The formats an emergency stick is written in: Club player USB with WAV
/// for older players, and OpenDJDecks.
pub(crate) fn playable_formats(current: Option<DeviceFormats>) -> DeviceFormats {
    match current {
        // A registered stick keeps what it carries and gains Club player USB.
        Some(f) => DeviceFormats {
            club_usb: true,
            club_usb_compat: f.club_usb_compat || !f.club_usb,
            opendjdecks: f.opendjdecks,
        },
        None => DeviceFormats {
            club_usb: true,
            opendjdecks: true,
            club_usb_compat: true,
        },
    }
}

/// The tracks to copy: the open list (a playlist, set or smart list, or a
/// history or tag list), then the tracks on the decks that are not in it.
/// The whole collection is never copied (pure).
pub(crate) fn tracks_to_copy(list: &[TrackId], loaded: &[TrackId]) -> Vec<TrackId> {
    let mut out: Vec<TrackId> = Vec::new();
    for t in list.iter().chain(loaded) {
        if !out.contains(t) {
            out.push(*t);
        }
    }
    out
}

fn list_tracks(app: &App) -> Vec<TrackId> {
    let st = app.state.borrow();
    if st.read_only() || st.explorer.open.is_some() {
        return Vec::new();
    }
    match (&st.browser.special, st.browser.node) {
        (Some(_), _) => st.browser.rows.iter().map(|t| t.id).collect(),
        (None, Some(n)) => st
            .lib
            .node_tracks(n)
            .map(|v| v.into_iter().map(|t| t.id).collect())
            .unwrap_or_default(),
        (None, None) => Vec::new(),
    }
}

fn loaded_tracks(app: &App) -> Vec<TrackId> {
    let d = app.decks.borrow();
    if d.source_is_stick() {
        return Vec::new();
    }
    let mut v: Vec<(TrackId, usize)> = d.loaded_decks();
    v.sort_by_key(|(_, deck)| *deck);
    v.into_iter().map(|(t, _)| t).collect()
}

/// Starts an emergency copy.
pub(crate) fn start(app: &App) {
    if app.guard_read_only() {
        return;
    }
    if app
        .state
        .borrow()
        .devices
        .emergency
        .as_ref()
        .is_some_and(|e| matches!(e.stage, Stage::Copying | Stage::Registering(_)))
    {
        return;
    }
    let tracks = tracks_to_copy(&list_tracks(app), &loaded_tracks(app));
    if tracks.is_empty() {
        app.status("Nothing to copy: open a set or playlist, or load a track to a deck");
        return;
    }
    // A stick that is not registered yet first, else a registered one.
    let target = {
        let st = app.state.borrow();
        st.devices
            .volumes
            .first()
            .map(|v| (v.mount_point.clone(), v.name.clone(), None))
            .or_else(|| {
                st.devices.statuses.iter().find_map(|s| {
                    s.volume.as_ref().map(|v| {
                        (v.mount_point.clone(), s.device.name.clone(), Some((s.device.id, s.device.formats)))
                    })
                })
            })
    };
    let Some((mount, stick, registered)) = target else {
        app.status("Plug in a USB stick for the emergency copy");
        return;
    };
    let name = {
        let now = app.state.borrow().lib.now().unwrap_or_default();
        let t = now.trim_end_matches('Z').replace('T', " ");
        od_i18n::trf("Emergency stick {}", &[&t.get(..16).unwrap_or(&t)])
    };
    let node = {
        let r = app.state.borrow_mut().lib.create_node(None, NodeKind::Playlist, &name);
        let Some(node) = app.report(r) else { return };
        let r = app.state.borrow_mut().lib.add_to_node(node, &tracks, None);
        if app.report(r).is_none() {
            return;
        }
        node
    };
    let formats = playable_formats(registered.map(|r| r.1));
    let stage = match registered {
        None => {
            app.device_manager.send(Command::Register {
                mount_point: mount.clone(),
                name: stick.clone(),
                nodes: vec![node],
                formats,
            });
            Stage::Registering(Instant::now())
        }
        Some((id, _)) => {
            let mut nodes = app.state.borrow().lib.device_nodes(id).unwrap_or_default();
            nodes.push(node);
            let r = app.state.borrow_mut().lib.set_device_nodes(id, &nodes);
            if app.report(r).is_none() {
                return;
            }
            let r = app.state.borrow_mut().lib.update_device(id, &stick, formats);
            if app.report(r).is_none() {
                return;
            }
            app.device_manager.send(Command::SyncNow(id));
            Stage::Copying
        }
    };
    app.state.borrow_mut().devices.emergency = Some(Emergency {
        mount,
        stick: stick.clone(),
        device: registered.map(|r| r.0),
        node,
        tracks: tracks.len(),
        stage,
        progress: 0.0,
        detail: od_i18n::trf("{} tracks into {}", &[&tracks.len(), &name]),
        formats,
    });
    app.status(format!("Emergency stick: copying {} tracks to {stick}", tracks.len()));
    crate::browser::refresh_tree(app);
    project(app);
}

/// Follows the device service's events for the emergency copy.
pub(crate) fn event(app: &App, e: &Event) {
    let mut done: Option<String> = None;
    {
        let mut st = app.state.borrow_mut();
        let Some(em) = st.devices.emergency.as_mut() else { return };
        match e {
            Event::Registered { device } if matches!(em.stage, Stage::Registering(_)) => {
                em.device = Some(*device);
                em.stage = Stage::Copying;
            }
            Event::SyncFinished { device, report } if Some(*device) == em.device && em.stage == Stage::Copying => {
                em.stage = Stage::Done;
                em.progress = 1.0;
                let mut detail = od_i18n::trf("{} tracks, ready to play", &[&report.tracks]);
                if !report.skipped_missing.is_empty() {
                    detail += &od_i18n::trf(" · {} missing files left out", &[&report.skipped_missing.len()]);
                }
                if !report.conversion_failed.is_empty() {
                    detail += &od_i18n::trf(" · {} not playable on older club players", &[&report.conversion_failed.len()]);
                }
                em.detail = detail;
                done = Some(format!(
                    "Emergency stick ready: {} tracks on {}",
                    report.tracks, em.stick
                ));
            }
            Event::SyncFailed { device, error } if Some(*device) == em.device => {
                em.stage = Stage::Failed(error.clone());
                em.detail = error.clone();
            }
            Event::Status(statuses) => {
                if let Some(s) = statuses.iter().find(|s| Some(s.device.id) == em.device) {
                    match &s.state {
                        DeviceState::Syncing(_) => {
                            let (_, detail, p) = crate::devices::device_line(s);
                            em.stage = Stage::Copying;
                            em.progress = p;
                            em.detail = od_i18n::translate_message(&detail);
                        }
                        DeviceState::Error(err) if em.stage == Stage::Copying => {
                            em.stage = Stage::Failed(err.clone());
                            em.detail = err.clone();
                        }
                        _ => {}
                    }
                } else if em.device.is_some() && em.stage == Stage::Copying
                    && !statuses.iter().any(|s| Some(s.device.id) == em.device && s.volume.is_some())
                {
                    // Pulled out before it was done.
                    let msg = od_i18n::tr("The stick was pulled out before the copy was done").to_string();
                    em.stage = Stage::Failed(msg.clone());
                    em.detail = msg;
                }
            }
            _ => {}
        }
    }
    if let Some(msg) = done {
        app.status(msg);
    }
}

/// Frame tick: the registration timeout and the projection.
pub(crate) fn tick(app: &App) {
    {
        let mut st = app.state.borrow_mut();
        if let Some(em) = st.devices.emergency.as_mut()
            && let Stage::Registering(at) = em.stage
            && at.elapsed() > REGISTER_TIMEOUT
        {
            let msg = od_i18n::tr("The stick could not be registered: is it writable?").to_string();
            em.stage = Stage::Failed(msg.clone());
            em.detail = msg;
        }
    }
    project(app);
}

/// The `Emergency` global from the state.
pub(crate) fn project(app: &App) {
    let g = app.ui.global::<crate::Emergency>();
    let st = app.state.borrow();
    let available = !st.devices.volumes.is_empty()
        || st.devices.statuses.iter().any(|s| s.volume.is_some());
    if g.get_available() != available {
        g.set_available(available);
    }
    let (state, progress, stick, detail, formats) = match &st.devices.emergency {
        None => ("", 0.0, String::new(), String::new(), DeviceFormats::default()),
        Some(e) => (
            match e.stage {
                Stage::Registering(_) | Stage::Copying => "copying",
                Stage::Done => "done",
                Stage::Failed(_) => "failed",
            },
            e.progress,
            e.stick.clone(),
            e.detail.clone(),
            e.formats,
        ),
    };
    if g.get_stage() != state {
        g.set_stage(state.into());
    }
    if (g.get_progress() - progress).abs() > 1e-4 {
        g.set_progress(progress);
    }
    if g.get_stick() != stick.as_str() {
        g.set_stick(stick.into());
    }
    if g.get_detail() != detail.as_str() {
        g.set_detail(detail.into());
    }
    g.set_club(formats.club_usb);
    g.set_compat(formats.club_usb && formats.club_usb_compat);
    g.set_native(formats.opendjdecks);
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::Emergency>();
    g.on_start(handler!(app, |a| start(&a)));
    g.on_dismiss(handler!(app, |a| {
        a.state.borrow_mut().devices.emergency = None;
        project(&a);
    }));
    g.on_eject(handler!(app, |a| {
        let device = a.state.borrow().devices.emergency.as_ref().and_then(|e| e.device);
        if let Some(d) = device {
            a.device_manager.send(Command::Eject(d));
            a.status("Ejecting…");
        }
    }));
    project(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_then_the_decks_without_doubles() {
        let t = |i| TrackId(i);
        assert_eq!(tracks_to_copy(&[t(3), t(1)], &[t(1), t(7)]), vec![t(3), t(1), t(7)]);
        assert!(tracks_to_copy(&[], &[]).is_empty());
    }

    #[test]
    fn playable_formats_always_include_club_usb() {
        let f = playable_formats(None);
        assert!(f.club_usb && f.club_usb_compat && f.opendjdecks);
        let only_native = DeviceFormats { club_usb: false, opendjdecks: true, club_usb_compat: false };
        let f = playable_formats(Some(only_native));
        assert!(f.club_usb && f.club_usb_compat && f.opendjdecks, "gains Club player USB with WAV");
        let club = DeviceFormats { club_usb: true, opendjdecks: false, club_usb_compat: false };
        assert_eq!(playable_formats(Some(club)), club, "a club stick stays as it is");
    }
}
