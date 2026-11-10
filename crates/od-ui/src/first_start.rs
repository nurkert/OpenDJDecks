//! The guided first start (roadmap [7]): a checklist of four steps that ticks
//! itself from what the app already knows, and points at the real control
//! for the current step (`FirstStart.target`, its accessible label). Sample
//! tracks and a test tone are offered so a first mix needs no music of one's
//! own and no guesswork about the outputs.

use crate::app::{App, handler};
use crate::samples;
use od_engine::{Command, MixerCommand};
use slint::{ComponentHandle, Model};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// How long the test tone runs.
const TONE: Duration = Duration::from_secs(4);

/// What the guide reads from the app each tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Facts {
    pub tracks: u64,
    pub unanalysed: usize,
    pub analysing: bool,
    pub samples_busy: bool,
    pub mode: String,
    /// The empty-library screen with its ADD MUSIC FOLDER key is showing.
    pub empty_screen: bool,
    pub has_selection: bool,
    pub loaded: [bool; 2],
    pub playing: [bool; 2],
    pub sync: [bool; 2],
}

/// The guide's state: which steps have been reached (a step once done stays
/// ticked).
#[derive(Default)]
pub(crate) struct Guide {
    done: [bool; 4],
    samples_busy: bool,
    tone_until: Option<Instant>,
}

/// The step in progress (0..=4), the hint key and the control to point at.
pub(crate) fn plan(f: &Facts, done: &mut [bool; 4]) -> (usize, &'static str, String) {
    done[0] |= f.tracks > 0;
    done[1] |= done[0] && f.unanalysed == 0 && !f.analysing && !f.samples_busy;
    done[2] |= done[1] && f.loaded[0] && f.loaded[1];
    done[3] |= done[2] && f.playing[0] && f.playing[1] && (f.sync[0] || f.sync[1]);
    let step = done.iter().position(|d| !d).unwrap_or(4);
    let (hint, target) = match step {
        0 if f.samples_busy => ("samples", String::new()),
        0 if f.mode != "prepare" => ("add-music", "topbar.mode.prepare".into()),
        0 if f.empty_screen => ("add-music", "browser.empty.add-music".into()),
        0 => ("add-music", String::new()),
        1 => ("analysing", "status.activity".into()),
        2 if f.mode != "perform" => ("go-perform", "topbar.mode.perform".into()),
        2 if !f.loaded[0] && !f.has_selection => ("select-track", "browser.list".into()),
        2 if !f.loaded[0] => ("load-1", "deck1.load".into()),
        2 => ("load-2", "deck2.load".into()),
        3 if !f.playing[0] => ("play-1", "deck1.play".into()),
        3 if !f.sync[1] => ("sync-2", "deck2.beat-sync".into()),
        3 => ("play-2", "deck2.play".into()),
        _ => ("done", "global.xfader".into()),
    };
    (step, hint, target)
}

/// Facts from the running app.
fn facts(app: &App, guide: &Guide) -> Facts {
    let state = app.state.borrow();
    let tracks = state.lib.track_count().unwrap_or(0);
    let unanalysed = state
        .lib
        .tracks_needing_analysis(od_analysis::ANALYSIS_VERSION)
        .map(|t| t.len())
        .unwrap_or(0);
    let mut f = Facts {
        tracks,
        unanalysed,
        analysing: state.analysis.1 > 0,
        samples_busy: guide.samples_busy,
        mode: app.ui.get_mode().to_string(),
        empty_screen: app.ui.get_node_kind() == "all"
            && app.ui.get_section() == "library"
            && slint::Model::row_count(&app.ui.get_tracks()) == 0,
        has_selection: app.ui.get_has_selection(),
        ..Default::default()
    };
    let decks = app.ui.get_decks();
    for i in 0..2 {
        if let Some(d) = decks.row_data(i) {
            f.loaded[i] = d.loaded;
            f.playing[i] = d.playing;
            f.sync[i] = d.sync;
        }
    }
    f
}

fn show(app: &App, visible: bool) {
    app.ui.global::<crate::FirstStart>().set_visible(visible);
    if !visible {
        stop_tone(app);
    }
}

fn stop_tone(app: &App) {
    let mut g = app.first_start.borrow_mut();
    if g.tone_until.take().is_some() {
        crate::engine::send(app, Command::Mixer(MixerCommand::SetTestTone(false)));
        app.ui.global::<crate::FirstStart>().set_tone_on(false);
    }
}

fn toggle_tone(app: &App) {
    let on = app.first_start.borrow().tone_until.is_some();
    if on {
        stop_tone(app);
    } else {
        app.first_start.borrow_mut().tone_until = Some(Instant::now() + TONE);
        crate::engine::send(app, Command::Mixer(MixerCommand::SetTestTone(true)));
        app.ui.global::<crate::FirstStart>().set_tone_on(true);
    }
}

fn add_samples(app: &App) {
    {
        let mut g = app.first_start.borrow_mut();
        if g.samples_busy {
            return;
        }
        g.samples_busy = true;
    }
    app.status("Writing sample tracks");
    let (path, tx, wake) = {
        let s = app.state.borrow();
        (
            s.config.library_path.clone(),
            app.worker_tx.clone(),
            app.analysis.wake.clone(),
        )
    };
    samples::spawn(path, tx, wake);
}

/// The sample tracks are written and added.
pub(crate) fn samples_done(app: &App) {
    app.first_start.borrow_mut().samples_busy = false;
    app.refresh_library();
}

fn close(app: &App) {
    app.state
        .borrow_mut()
        .change_settings(|s| s.view.first_start = "off".into());
    show(app, false);
}

/// View > First-Start Guide: from the start (steps already done tick again).
fn reopen(app: &App) {
    if app.state.borrow().config.kiosk {
        return;
    }
    app.state
        .borrow_mut()
        .change_settings(|s| s.view.first_start = "active".into());
    app.first_start.borrow_mut().done = [false; 4];
    show(app, true);
    tick(app);
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::FirstStart>();
    g.on_add_samples(handler!(app, |a| add_samples(&a)));
    g.on_toggle_tone(handler!(app, |a| toggle_tone(&a)));
    g.on_close(handler!(app, |a| close(&a)));
    g.on_reopen(handler!(app, |a| reopen(&a)));

    // Who gets it: a library that starts empty. An existing library never
    // sees it unless asked for (View menu); the kiosk has its own start.
    let tracks = app.state.borrow().lib.track_count().unwrap_or(0);
    let kiosk = app.state.borrow().config.kiosk;
    let stored = app.state.borrow().settings.view.first_start.clone();
    let visible = !kiosk
        && match stored.as_str() {
            "" => tracks == 0,
            "active" => true,
            _ => false,
        };
    if stored.is_empty() && !kiosk {
        let value = if visible { "active" } else { "off" };
        app.state
            .borrow_mut()
            .change_settings(|s| s.view.first_start = value.into());
    }
    show(app, visible);
    tick(app);
}

pub(crate) fn tick(app: &App) {
    let ui = app.ui.global::<crate::FirstStart>();
    if !ui.get_visible() {
        return;
    }
    if app.first_start.borrow().tone_until.is_some_and(|t| Instant::now() >= t) {
        stop_tone(app);
    }
    let f = facts(app, &app.first_start.borrow());
    let (step, hint, target) = {
        let mut g = app.first_start.borrow_mut();
        plan(&f, &mut g.done)
    };
    ui.set_step(step as i32);
    if ui.get_hint() != hint {
        ui.set_hint(hint.into());
    }
    ui.set_target(target.into());
    ui.set_samples_busy(f.samples_busy);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_tick_in_order_and_stay_ticked() {
        let mut done = [false; 4];
        let mut f = Facts {
            mode: "prepare".into(),
            empty_screen: true,
            ..Default::default()
        };
        assert_eq!(plan(&f, &mut done).2, "browser.empty.add-music");
        f.tracks = 2;
        f.unanalysed = 2;
        assert_eq!(plan(&f, &mut done).1, "analysing");
        f.unanalysed = 0;
        assert_eq!(plan(&f, &mut done).2, "topbar.mode.perform");
        f.mode = "perform".into();
        assert_eq!(plan(&f, &mut done).2, "browser.list");
        f.has_selection = true;
        assert_eq!(plan(&f, &mut done).2, "deck1.load");
        f.loaded[0] = true;
        assert_eq!(plan(&f, &mut done).2, "deck2.load");
        f.loaded[1] = true;
        assert_eq!(plan(&f, &mut done).2, "deck1.play");
        f.playing[0] = true;
        assert_eq!(plan(&f, &mut done).2, "deck2.beat-sync");
        f.sync[1] = true;
        assert_eq!(plan(&f, &mut done).2, "deck2.play");
        f.playing[1] = true;
        assert_eq!(plan(&f, &mut done).0, 4, "done");
        // Stopping the deck afterwards does not untick anything.
        f.playing = [false, false];
        assert_eq!(plan(&f, &mut done).2, "global.xfader");
    }
}
