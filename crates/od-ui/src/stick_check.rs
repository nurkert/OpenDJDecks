//! The stick check before a gig on the Devices page (roadmap [53]): reads
//! the selected stick back on a worker thread and shows a verdict and a row
//! per track.

use crate::app::{App, handler};
use crate::{StickCheck, StickCheckRow};
use crossbeam_channel::Receiver;
use od_core::DeviceId;
use od_devices::check::{Mark, StickCheck as Result_};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;

/// A check under way.
#[derive(Default)]
pub(crate) struct Running {
    rx: Option<(DeviceId, Receiver<Result<Result_, String>>)>,
}

fn mark(m: Mark) -> &'static str {
    match m {
        Mark::Ok => "ok",
        Mark::Missing => "missing",
        Mark::Differs => "differs",
        Mark::Unreadable => "unreadable",
        Mark::None => "none",
    }
}

/// The summary line and the whole-stick problems.
pub(crate) fn texts(c: &Result_) -> (String, String) {
    let mut summary = od_i18n::trf(
        "{} of {} tracks in order",
        &[&c.tracks_ok(), &c.tracks.len()],
    );
    let lists_ok = c.playlists - c.playlist_problems.len().min(c.playlists);
    summary.push_str(" · ");
    summary.push_str(&od_i18n::trf(
        "{} of {} playlists as in the library",
        &[&lists_ok, &c.playlists],
    ));
    let mut formats = Vec::new();
    if c.native {
        formats.push("OpenDJDecks");
    }
    if c.club_usb {
        formats.push("Club player USB");
    }
    if !formats.is_empty() {
        summary.push_str(" · ");
        summary.push_str(&formats.join(" + "));
    }
    let mut problems = Vec::new();
    if c.interrupted {
        problems.push(od_i18n::tr("A sync did not finish. Sync the stick again.").to_string());
    }
    for f in &c.format_problems {
        problems.push(od_i18n::trf("Not found on the stick: {}", &[f]));
    }
    if !c.playlist_problems.is_empty() {
        let names: Vec<&str> = c.playlist_problems.iter().map(|p| p.name.as_str()).collect();
        problems.push(od_i18n::trf(
            "Playlists not as in the library: {}. Sync the stick.",
            &[&names.join(", ")],
        ));
    }
    if c.tracks.iter().any(|t| {
        [t.file, t.cues, t.grid].contains(&Mark::Differs)
    }) && c.playlist_problems.is_empty()
    {
        problems.push(od_i18n::tr("Some tracks changed since the last sync. Sync the stick.").to_string());
    }
    (summary, problems.join("\n"))
}

fn show(app: &App, c: &Result_) {
    let g = app.ui.global::<StickCheck>();
    let (summary, problems) = texts(c);
    g.set_ready(c.ready());
    g.set_summary(summary.into());
    g.set_problems(problems.into());
    g.set_rows(ModelRc::new(VecModel::from(
        c.tracks
            .iter()
            .map(|t| StickCheckRow {
                title: t.title.clone().into(),
                artist: t.artist.clone().into(),
                file: mark(t.file).into(),
                cues: mark(t.cues).into(),
                grid: mark(t.grid).into(),
            })
            .collect::<Vec<_>>(),
    )));
    g.set_running(false);
}

fn fail(app: &App, why: String) {
    let g = app.ui.global::<StickCheck>();
    g.set_running(false);
    g.set_ready(false);
    g.set_summary("".into());
    g.set_problems(why.into());
    g.set_rows(ModelRc::new(VecModel::from(Vec::<StickCheckRow>::new())));
}

/// Starts the check of `device` (it must be plugged in).
pub(crate) fn run(app: &App, device: DeviceId) {
    let g = app.ui.global::<StickCheck>();
    g.set_visible(true);
    let (root, library) = {
        let st = app.state.borrow();
        let root = st
            .devices
            .statuses
            .iter()
            .find(|s| s.device.id == device)
            .and_then(|s| s.volume.as_ref())
            .map(|v| v.mount_point.clone());
        (root, st.config.library_path.clone())
    };
    let Some(root) = root else {
        fail(app, od_i18n::tr("Plug the stick in to check it.").to_string());
        return;
    };
    g.set_running(true);
    g.set_rows(ModelRc::new(VecModel::from(Vec::<StickCheckRow>::new())));
    let (tx, rx) = crossbeam_channel::bounded(1);
    std::thread::Builder::new()
        .name("od-stick-check".into())
        .spawn(move || {
            let r = od_library::Library::open(&library)
                .and_then(|lib| od_devices::check::check_stick(&lib, device, &root))
                .map_err(|e| e.to_string());
            let _ = tx.send(r);
        })
        .expect("stick check thread");
    app.stick_check.borrow_mut().rx = Some((device, rx));
}

/// Frame tick: picks up a finished check.
pub(crate) fn poll(app: &App) {
    let done = {
        let r = app.stick_check.borrow();
        r.rx.as_ref().and_then(|(_, rx)| rx.try_recv().ok())
    };
    let Some(result) = done else { return };
    app.stick_check.borrow_mut().rx = None;
    match result {
        Ok(c) => show(app, &c),
        Err(e) => fail(app, od_i18n::trf("The stick could not be read: {}", &[&e])),
    }
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=stick-check` runs
/// the check of the first plugged-in stick; `self-test[:<y>]` opens the
/// controller self-test (scrolled to `y`) with a few controls ticked as if
/// they had been moved.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(4), move || {
        let Some(a) = weak.upgrade() else { return };
        for part in v.split(',') {
            if part == "stick-check" {
                let id = a
                    .state
                    .borrow()
                    .devices
                    .statuses
                    .iter()
                    .find(|s| s.volume.is_some())
                    .map(|s| s.device.id);
                if let Some(id) = id {
                    a.ui.invoke_open_device(id.0 as i32);
                    run(&a, id);
                }
            }
            if let Some(rest) = part.strip_prefix("self-test") {
                crate::preferences::open(&a);
                let list = crate::controller::test_mappings(&a);
                {
                    let mut p = a.prefs.borrow_mut();
                    if p.midi.test_mappings.is_empty() {
                        p.midi.test_mappings = list;
                    }
                    if p.midi.selftest.is_none() {
                        let i = p.midi.test_index;
                        p.midi.select_test(i);
                    }
                    if let Some(t) = p.midi.selftest.as_mut() {
                        let msgs: Vec<[u8; 3]> = t
                            .mapping()
                            .inputs
                            .iter()
                            .take(40)
                            .step_by(2)
                            .map(|b| b.addr.message(0x7F))
                            .collect();
                        for m in msgs {
                            t.feed(&m);
                        }
                    }
                    p.midi.testing = true;
                }
                let y: f32 = rest.trim_start_matches(':').parse().unwrap_or(0.0);
                crate::preferences::demo_page(&a, "controller", y);
            }
        }
    });
}

pub(crate) fn wire(app: &Rc<App>) {
    demo(app);
    let g = app.ui.global::<StickCheck>();
    g.on_run(handler!(app, |a, id: i32| run(&a, DeviceId(id as i64))));
    g.on_close(handler!(app, |a| {
        a.ui.global::<StickCheck>().set_visible(false);
        a.stick_check.borrow_mut().rx = None;
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_devices::check::{PlaylistProblem, TrackCheck};

    fn track(file: Mark) -> TrackCheck {
        TrackCheck {
            track: od_core::TrackId(1),
            title: "A".into(),
            artist: "B".into(),
            file,
            cues: Mark::Ok,
            grid: Mark::None,
        }
    }

    #[test]
    fn the_summary_counts_tracks_lists_and_formats() {
        let c = Result_ {
            tracks: vec![track(Mark::Ok), track(Mark::Missing)],
            playlists: 2,
            playlist_problems: vec![PlaylistProblem {
                name: "Gig".into(),
                mark: Mark::Differs,
            }],
            native: true,
            club_usb: true,
            ..Default::default()
        };
        let (s, p) = texts(&c);
        assert_eq!(
            s,
            "1 of 2 tracks in order · 1 of 2 playlists as in the library · OpenDJDecks + Club player USB"
        );
        assert_eq!(p, "Playlists not as in the library: Gig. Sync the stick.");
        let mut c = c;
        c.interrupted = true;
        c.playlist_problems.clear();
        assert!(texts(&c).1.starts_with("A sync did not finish."));
    }
}
