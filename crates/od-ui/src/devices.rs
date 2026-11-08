//! Devices: registered sticks, their sync state, new volumes, changes made
//! and the browsable stick libraries.

use crate::app::{App, handler};
use crate::browser;
use crate::models::model;
use crate::{DeviceRow, NodeChoice, VolumeRow, format};
use od_core::DeviceId;
use od_devices::{Command, DeviceState, DeviceStatus, Event, Volume};
use od_library::{Library, NodeId, NodeKind};
use slint::SharedString;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct DevicesState {
    pub selected: Option<DeviceId>,
    pub statuses: Vec<DeviceStatus>,
    pub volumes: Vec<Volume>,
}

/// Selects the first registered stick when none is selected.
pub(crate) fn select_default(app: &App) {
    let first = app
        .state
        .borrow()
        .lib
        .devices()
        .ok()
        .and_then(|d| d.first().map(|d| d.id));
    let mut st = app.state.borrow_mut();
    if st.devices.selected.is_none() {
        st.devices.selected = first;
    }
}

/// State, detail line and progress of a stick (detail untranslated).
pub(crate) fn device_line(s: &DeviceStatus) -> (&'static str, String, f32) {
    match &s.state {
        DeviceState::Disconnected => ("disconnected", "Not connected".to_string(), 0.0),
        DeviceState::InSync => (
            "in-sync",
            match s.device.last_sync.as_deref() {
                Some(t) => format!("In sync · {}", t.get(11..16).unwrap_or(t)),
                None => "In sync".to_string(),
            },
            1.0,
        ),
        DeviceState::OutOfDate { pending } => (
            "pending",
            format!(
                "{pending} change{} waiting",
                if *pending == 1 { "" } else { "s" }
            ),
            0.0,
        ),
        DeviceState::Syncing(p) => match p {
            od_devices::Progress::Copying {
                done_bytes,
                total_bytes,
                current,
            } => (
                "syncing",
                format!("Copying {current}"),
                if *total_bytes > 0 {
                    *done_bytes as f32 / *total_bytes as f32
                } else {
                    0.0
                },
            ),
            od_devices::Progress::Writing { format } => {
                ("syncing", format!("Writing {format} data"), 0.95)
            }
            _ => ("syncing", "Preparing".to_string(), 0.02),
        },
        DeviceState::Error(e) => ("error", e.clone(), 0.0),
    }
}

fn space(free: u64, total: u64) -> String {
    od_i18n::trf(
        "{} FREE OF {}",
        &[&format::bytes(free), &format::bytes(total)],
    )
}

pub(crate) fn refresh(app: &App) {
    let st = app.state.borrow();
    let ui = &app.ui;
    let mut rows: Vec<DeviceRow> = st
        .devices
        .statuses
        .iter()
        .map(|s| {
            let (state, detail, progress) = device_line(s);
            DeviceRow {
                id: s.device.id.0 as i32,
                name: s.device.name.clone().into(),
                state: state.into(),
                detail: od_i18n::translate_message(&detail).into(),
                progress,
                space: s
                    .volume
                    .as_ref()
                    .map(|v| space(v.free_bytes, v.total_bytes))
                    .unwrap_or_default()
                    .into(),
                connected: s.volume.is_some(),
            }
        })
        .collect();
    // OPENDJDECKS_DEMO_SYNC=0.4 shows the first connected stick mid-sync
    // (documentation screenshots and design review).
    if let Some(p) = od_core::env::var("DEMO_SYNC")
        .and_then(|v| v.parse::<f32>().ok())
    {
        if let Some(r) = rows.iter_mut().find(|r| r.connected) {
            r.state = "syncing".into();
            r.progress = p.clamp(0.0, 1.0);
            r.detail = od_i18n::translate_message("Copying {}")
                .replace("{}", "…")
                .into();
        }
    }
    ui.set_devices(model(rows));
    let vols: Vec<VolumeRow> = st
        .devices
        .volumes
        .iter()
        .enumerate()
        .map(|(i, v)| VolumeRow {
            index: i as i32,
            name: v.name.clone().into(),
            space: space(v.free_bytes, v.total_bytes).into(),
        })
        .collect();
    ui.set_volumes(model(vols));

    // Device detail.
    let Some(id) = st.devices.selected else {
        ui.set_device_selected(-1);
        return;
    };
    let Ok(device) = st.lib.device(id) else {
        ui.set_device_selected(-1);
        return;
    };
    ui.set_device_selected(id.0 as i32);
    ui.set_device_name(device.name.clone().into());
    let assigned: HashSet<NodeId> = st
        .lib
        .device_nodes(id)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut choices = Vec::new();
    fn walk(
        lib: &Library,
        parent: Option<NodeId>,
        depth: i32,
        assigned: &HashSet<NodeId>,
        out: &mut Vec<NodeChoice>,
    ) {
        for n in lib.children(parent).unwrap_or_default() {
            out.push(NodeChoice {
                id: n.id.0 as i32,
                name: n.name.clone().into(),
                depth,
                checked: assigned.contains(&n.id),
            });
            if n.kind == NodeKind::Folder {
                walk(lib, Some(n.id), depth + 1, assigned, out);
            }
        }
    }
    walk(&st.lib, None, 0, &assigned, &mut choices);
    ui.set_device_content(model(choices));
}

/// Mounted sticks with an OpenDJDecks library, sorted and de-duplicated.
fn browsable_sticks(statuses: &[DeviceStatus], volumes: &[Volume]) -> Vec<(String, PathBuf)> {
    let mut sticks: Vec<(String, PathBuf)> = statuses
        .iter()
        .filter_map(|s| {
            s.volume
                .as_ref()
                .map(|v| (s.device.name.clone(), v.mount_point.clone()))
        })
        .chain(volumes.iter().map(|v| {
            let name = od_format_native::read_marker(&v.mount_point)
                .ok()
                .flatten()
                .map(|m| m.name)
                .unwrap_or_else(|| v.name.clone());
            (name, v.mount_point.clone())
        }))
        .filter(|(_, root)| od_format_native::library_path(root).exists())
        .collect();
    sticks.sort();
    sticks.dedup_by(|a, b| a.1 == b.1);
    sticks
}

/// Recomputes the browsable sticks from connected volumes.
fn update_sticks(app: &App) {
    let changed = {
        let mut st = app.state.borrow_mut();
        let sticks = browsable_sticks(&st.devices.statuses, &st.devices.volumes);
        if sticks == st.browser.sticks {
            false
        } else {
            // Leave a stick view whose stick went away.
            let b = &mut st.browser;
            let active_root = b
                .stick
                .as_ref()
                .map(|(i, _)| b.sticks.get(*i).map(|s| s.1.clone()));
            b.sticks = sticks;
            if let Some(root) = active_root.flatten() {
                match b.sticks.iter().position(|s| s.1 == root) {
                    Some(i) => {
                        if let Some(s) = b.stick.as_mut() {
                            s.0 = i;
                        }
                    }
                    None => {
                        b.stick = None;
                        b.node = None;
                        b.selection.clear();
                        st.player.current = None;
                    }
                }
            }
            true
        }
    };
    if changed {
        // Standalone: a stick becomes the source as soon as it is plugged in.
        let auto = {
            let st = app.state.borrow();
            st.browser.stick.is_none()
                && !st.browser.sticks.is_empty()
                && (st.config.kiosk || st.lib.track_count().unwrap_or(0) == 0)
        };
        if auto {
            app.ui.invoke_select_node(browser::stick_root_id(0));
        }
        app.refresh_library();
    }
}

/// Device manager events; returns true when the library needs a refresh.
pub(crate) fn poll(app: &App) -> bool {
    let mut need_refresh = false;
    let mut changed = false;
    for e in app.device_manager.events().try_iter() {
        match e {
            Event::Status(s) => {
                app.state.borrow_mut().devices.statuses = s;
                changed = true;
            }
            Event::UnregisteredVolumes(v) => {
                app.state.borrow_mut().devices.volumes = v;
                changed = true;
            }
            Event::SyncFinished { device, report } => {
                let name = app
                    .state
                    .borrow()
                    .lib
                    .device(device)
                    .map(|d| d.name)
                    .unwrap_or_default();
                app.status(format!(
                    "{name} synced: {} copied, {} updated, {} removed{}",
                    report.copied,
                    report.updated,
                    report.removed,
                    if report.skipped_missing.is_empty() {
                        String::new()
                    } else {
                        format!(", {} missing files skipped", report.skipped_missing.len())
                    }
                ));
            }
            Event::SyncFailed { error, .. } => app.status(format!("Sync failed: {error}")),
            Event::Ejected { device } => {
                let name = app
                    .state
                    .borrow()
                    .lib
                    .device(device)
                    .map(|d| d.name)
                    .unwrap_or_default();
                app.status(format!("{name} can be removed safely"));
            }
            Event::EjectFailed { error, .. } => app.status(format!("Eject failed: {error}")),
            Event::Registered { .. } => need_refresh = true,
        }
    }
    if changed {
        update_sticks(app);
        refresh(app);
    }
    need_refresh
}

pub(crate) fn wire(app: &Rc<App>) {
    let ui = &app.ui;
    ui.on_sync_device(handler!(app, |a, id: i32| a
        .device_manager
        .send(Command::SyncNow(DeviceId(id as i64)))));
    // DEVICES heading: sync every registered stick now (sticks that are not
    // plugged in are skipped by the device manager).
    ui.on_sync_all_devices(handler!(app, |a| {
        let ids: Vec<DeviceId> = a
            .state
            .borrow()
            .lib
            .devices()
            .unwrap_or_default()
            .iter()
            .map(|d| d.id)
            .collect();
        for id in ids {
            a.device_manager.send(Command::SyncNow(id));
        }
        a.status("Syncing sticks…");
    }));
    ui.on_rebuild_device(handler!(app, |a, id: i32| {
        a.device_manager.send(Command::Rebuild(DeviceId(id as i64)));
        a.status("Rebuilding stick…");
    }));
    ui.on_eject_device(handler!(app, |a, id: i32| {
        a.device_manager.send(Command::Eject(DeviceId(id as i64)));
        a.status("Ejecting…");
    }));
    ui.on_open_device(handler!(app, |a, id: i32| {
        a.state.borrow_mut().devices.selected = Some(DeviceId(id as i64));
        a.ui.set_section("devices".into());
        refresh(&a);
    }));
    ui.on_forget_device(handler!(app, |a, id: i32| {
        let r = a.state.borrow_mut().lib.forget_device(DeviceId(id as i64));
        if a.report(r).is_some() {
            a.state.borrow_mut().devices.selected = None;
            a.device_manager.send(Command::Refresh);
            a.status("Stick forgotten. Its files were left untouched.");
        }
    }));
    ui.on_register_volume(handler!(app, |a, i: i32| {
        let (volume, node, name) = {
            let st = a.state.borrow();
            let node = st.browser.node.filter(|_| st.explorer.open.is_none());
            let name = node.and_then(|n| st.lib.node(n).ok()).map(|n| n.name);
            (st.devices.volumes.get(i as usize).cloned(), node, name)
        };
        let (Some(v), Some(n)) = (volume, node) else {
            a.status("Select the playlist, set or folder that should go on the stick first");
            return;
        };
        a.device_manager.send(Command::Register {
            mount_point: v.mount_point.clone(),
            name: v.name.clone(),
            nodes: vec![n],
        });
        a.status(format!(
            "Registering {} with {} — the first sync starts now",
            v.name,
            name.unwrap_or_default()
        ));
    }));
    ui.on_set_device_node(handler!(app, |a, node: i32, on: bool| {
        let id = a.state.borrow().devices.selected;
        if let Some(id) = id {
            let mut nodes = a.state.borrow().lib.device_nodes(id).unwrap_or_default();
            let n = NodeId(node as i64);
            nodes.retain(|x| *x != n);
            if on {
                nodes.push(n);
            }
            let r = a.state.borrow_mut().lib.set_device_nodes(id, &nodes);
            if a.report(r).is_some() {
                refresh(&a);
                browser::refresh_tree(&a);
            }
        }
    }));
    ui.on_rename_device(handler!(app, |a, name: SharedString| {
        let id = a.state.borrow().devices.selected;
        if let Some(id) = id {
            let r = a
                .state
                .borrow_mut()
                .lib
                .update_device(id, name.trim());
            if a.report(r).is_some() {
                a.device_manager.send(Command::Refresh);
            }
        }
    }));

}
