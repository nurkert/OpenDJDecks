//! The guided controller setup (roadmap [13]): a band under the top bar
//! that walks through the core controls of a controller class
//! (`od_controller::guided`), learns each from the messages it sends and
//! saves a mapping file. Opened from Preferences › Controller and from the
//! View menu.
//!
//! A step is taken by itself once its control has been moved and then left
//! alone for [`QUIET`]; a control that an earlier step already uses is
//! refused with the number of that step.

use crate::app::{App, handler};
use crossbeam_channel::Receiver;
use od_controller::guided::{Expect, GuidedSetup, Refusal, Template};
use od_controller::monitor::{Direction, MidiMonitor, MonitorEvent};
use slint::{ComponentHandle, SharedString, VecModel};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// How long a control has to rest before its step is taken.
pub(crate) const QUIET: Duration = Duration::from_millis(700);

#[derive(Default)]
pub(crate) struct Runtime {
    /// The controller layer's monitor (tests put their own here).
    pub monitor: Option<MidiMonitor>,
    rx: Option<Receiver<MonitorEvent>>,
    pub visible: bool,
    pub class_index: usize,
    pub setup: Option<GuidedSetup>,
    /// Last message that counted for the current step.
    last: Option<Instant>,
    /// Port of the first message seen (named on the first screen).
    pub port: String,
    /// The earlier step (1-based) that uses the control just moved, 0 none.
    taken: usize,
    pub saved: Option<PathBuf>,
    timer: Option<slint::Timer>,
}

impl Runtime {
    /// The setup is open (its ports must be listened to).
    pub fn active(&self) -> bool {
        self.visible
    }

    /// Takes the new messages; true when something changed.
    fn poll(&mut self, now: Instant) -> bool {
        let Some(rx) = &self.rx else { return false };
        let events: Vec<MonitorEvent> = rx.try_iter().collect();
        let mut changed = false;
        for e in events {
            if e.direction != Direction::In {
                continue;
            }
            if self.port.is_empty() {
                self.port = e.port.clone();
                changed = true;
            }
            if let Some(s) = self.setup.as_mut()
                && s.feed(&e.port, &e.bytes)
            {
                self.last = Some(now);
                self.taken = 0;
                changed = true;
            }
        }
        // A control that has come to rest is taken.
        if let (Some(s), Some(t)) = (self.setup.as_mut(), self.last)
            && now.duration_since(t) >= QUIET
            && !s.done()
        {
            self.last = None;
            match s.accept() {
                Ok(()) => self.taken = 0,
                Err(Refusal::Taken(i)) => self.taken = i + 1,
                Err(Refusal::Nothing) => {}
            }
            changed = true;
        }
        changed
    }
}

/// The class names the first screen offers, in the templates' order.
fn class_label(id: &str) -> String {
    od_i18n::tr(match id {
        "two-deck-entry" => "2 DECKS · ENTRY LEVEL",
        "two-deck-battle" => "2 DECKS · BATTLE",
        "four-deck" => "4 DECKS",
        _ => "OTHER",
    })
    .to_string()
}

/// Opens the setup at its first screen (not in the kiosk).
pub(crate) fn open(app: &Rc<App>) {
    if app.state.borrow().config.kiosk {
        return;
    }
    {
        let mut r = app.controller_setup.borrow_mut();
        if r.monitor.is_none() {
            r.monitor = crate::controller::monitor(app);
        }
        if r.monitor.is_none() {
            drop(r);
            app.status("No controller layer running — connect a controller and restart");
            return;
        }
        r.visible = true;
        r.setup = None;
        r.saved = None;
        r.taken = 0;
        r.last = None;
        if r.rx.is_none() {
            let m = r.monitor.clone().unwrap();
            m.set_enabled(true);
            r.rx = Some(m.subscribe());
        }
    }
    crate::controller::refresh_unmapped(app);
    start_timer(app);
    refresh(app);
}

fn start_timer(app: &Rc<App>) {
    let weak = Rc::downgrade(app);
    let t = slint::Timer::default();
    t.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            let Some(a) = weak.upgrade() else { return };
            let changed = a.controller_setup.borrow_mut().poll(Instant::now());
            if changed {
                refresh(&a);
            }
        },
    );
    app.controller_setup.borrow_mut().timer = Some(t);
}

fn choose(app: &App, i: usize) {
    app.controller_setup.borrow_mut().class_index = i.min(Template::all().len().saturating_sub(1));
    refresh(app);
}

/// START: the first control of the chosen class, in Perform so the matching
/// on-screen control can be pointed at.
fn start(app: &App) {
    let templates = Template::all();
    {
        let mut r = app.controller_setup.borrow_mut();
        let Some(t) = templates.get(r.class_index).cloned() else {
            return;
        };
        let mut s = GuidedSetup::new(t);
        s.port = r.port.clone();
        r.setup = Some(s);
        r.taken = 0;
        r.last = None;
        // Messages from before START do not count.
        if let Some(rx) = &r.rx {
            let _ = rx.try_iter().count();
        }
    }
    if app.ui.get_mode() != "perform" {
        app.ui.set_mode("perform".into());
    }
    refresh(app);
}

fn with_setup(app: &App, f: impl FnOnce(&mut GuidedSetup)) {
    {
        let mut r = app.controller_setup.borrow_mut();
        if let Some(s) = r.setup.as_mut() {
            f(s);
        }
        r.taken = 0;
        r.last = None;
    }
    refresh(app);
}

/// Where a mapping called `name` is written: next to the library, in
/// `mappings/`, under a file name that is not taken yet.
pub(crate) fn mapping_path(library: &Path, name: &str) -> PathBuf {
    let dir = library.parent().unwrap_or(Path::new(".")).join("mappings");
    let base = od_controller::guided::slug(name);
    let mut p = dir.join(format!("{base}.toml"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base}-{n}.toml"));
        n += 1;
    }
    p
}

/// SAVE MAPPING: writes the file (only if it loads), makes it the own
/// mapping file, and for a battle-style class sets the CUT crossfader curve.
pub(crate) fn save(app: &App, name: &str) {
    let (text, curve, loads) = {
        let r = app.controller_setup.borrow();
        let Some(s) = r.setup.as_ref() else { return };
        if s.learned_count() == 0 {
            return;
        }
        (
            s.to_toml(name),
            s.template.crossfader_curve.clone(),
            s.mapping(name).map(|_| ()),
        )
    };
    if let Err(e) = loads {
        app.status(od_i18n::trf("The mapping could not be saved: {}", &[&e]));
        return;
    }
    let path = mapping_path(&app.state.borrow().config.library_path, name);
    let written = path
        .parent()
        .map(std::fs::create_dir_all)
        .unwrap_or(Ok(()))
        .and_then(|_| std::fs::write(&path, &text));
    if let Err(e) = written {
        app.status(od_i18n::trf("The mapping could not be saved: {}", &[&e]));
        return;
    }
    let file = path.display().to_string();
    app.state.borrow_mut().change_settings(|s| {
        s.controller.mapping_file = file.clone();
        if curve == "cut" {
            s.mixer.crossfader_curve = "cut".into();
        }
    });
    if curve == "cut" {
        crate::engine::send(
            app,
            od_engine::Command::Mixer(od_engine::MixerCommand::SetCrossfaderCurve(
                od_engine::CrossfaderCurve::Cut,
            )),
        );
    }
    app.status(od_i18n::trf(
        "Mapping saved to {} — used from the next start",
        &[&path.display()],
    ));
    app.controller_setup.borrow_mut().saved = Some(path);
    refresh(app);
}

pub(crate) fn close(app: &App) {
    {
        let mut r = app.controller_setup.borrow_mut();
        r.visible = false;
        r.setup = None;
        r.rx = None;
        r.timer = None;
        r.saved = None;
    }
    crate::controller::refresh_unmapped(app);
    refresh(app);
}

/// "A" for hot cue slot 0 (the prompt names the pad).
fn args_text(s: &od_controller::guided::Step) -> String {
    if let Some(b) = s.args.get("band").and_then(|v| v.as_str()) {
        return match b {
            "low" => "LOW",
            "mid" => "MID",
            _ => "HI",
        }
        .into();
    }
    match s.args.get("slot").and_then(|v| v.as_integer()) {
        Some(i) => crate::pads::hot_letter(i as usize),
        None => String::new(),
    }
}

/// Pushes the state into the band.
pub(crate) fn refresh(app: &App) {
    let g = app.ui.global::<crate::ControllerSetup>();
    let r = app.controller_setup.borrow();
    g.set_visible(r.visible);
    let templates = Template::all();
    let classes: Vec<SharedString> = templates
        .iter()
        .map(|t| class_label(&t.id).into())
        .collect();
    if slint::Model::row_count(&g.get_classes()) != classes.len() {
        g.set_classes(Rc::new(VecModel::from(classes)).into());
    }
    g.set_class_index(r.class_index as i32);
    g.set_port(r.port.clone().into());
    g.set_saved_path(
        r.saved
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
            .into(),
    );
    g.set_taken(
        if r.taken > 0 {
            r.taken.to_string()
        } else {
            String::new()
        }
        .into(),
    );
    let Some(s) = r.setup.as_ref() else {
        g.set_stage("class".into());
        g.set_target("".into());
        g.set_heard("".into());
        return;
    };
    g.set_total(s.template.steps.len() as i32);
    g.set_index(s.index as i32);
    g.set_learned(s.learned_count() as i32);
    g.set_curve(s.template.crossfader_curve.clone().into());
    match s.step() {
        Some(step) => {
            g.set_stage("learn".into());
            g.set_action(step.action.clone().into());
            g.set_deck(step.deck.unwrap_or(0) as i32);
            g.set_args(args_text(step).into());
            g.set_expect(
                match step.expect {
                    Expect::Button => "button",
                    Expect::Fader => "fader",
                    Expect::Wheel => "wheel",
                }
                .into(),
            );
            g.set_target(step.screen.clone().unwrap_or_default().into());
            g.set_heard(
                s.pending()
                    .map(|l| crate::midi_tools::describe(&l))
                    .unwrap_or_default()
                    .into(),
            );
        }
        None => {
            g.set_stage("done".into());
            g.set_target("".into());
            g.set_heard("".into());
        }
    }
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=controller-setup`
/// opens the setup for a two-deck entry-level unit, with PLAY, CUE and
/// SYNC of deck 1 learned and the jog wheel skipped, at the tempo fader
/// with a 14-bit fader heard. The messages are made up here; no unit is
/// involved.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else {
        return;
    };
    if !v.split(',').any(|p| p == "controller-setup") {
        return;
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(Duration::from_secs(3), move || {
        let Some(a) = weak.upgrade() else { return };
        {
            let mut r = a.controller_setup.borrow_mut();
            r.monitor = Some(MidiMonitor::new(16));
        }
        open(&a);
        start(&a);
        let port = "MIDI controller";
        {
            let mut r = a.controller_setup.borrow_mut();
            r.port = port.into();
            r.rx = None;
            let Some(s) = r.setup.as_mut() else { return };
            s.port = port.into();
            for note in [0x0Bu8, 0x0C, 0x58] {
                s.feed(port, &[0x90, note, 0x7F]);
                s.feed(port, &[0x90, note, 0x00]);
                let _ = s.accept();
            }
            // No jog wheel on this made-up unit.
            s.skip();
            s.skip();
            for v in [40u8, 52, 64] {
                s.feed(port, &[0xB0, 0x00, v]);
                s.feed(port, &[0xB0, 0x20, 0x00]);
            }
        }
        refresh(&a);
    });
}

pub(crate) fn wire(app: &Rc<App>) {
    demo(app);
    let g = app.ui.global::<crate::ControllerSetup>();
    g.on_open(handler!(app, |a| open(&a)));
    g.on_choose(handler!(app, |a, i: i32| choose(&a, i.max(0) as usize)));
    g.on_start(handler!(app, |a| start(&a)));
    g.on_skip(handler!(app, |a| with_setup(&a, |s| s.skip())));
    g.on_back(handler!(app, |a| with_setup(&a, |s| s.back())));
    g.on_retry(handler!(app, |a| with_setup(&a, |s| s.retry())));
    g.on_save(handler!(app, |a, n: SharedString| save(&a, &n)));
    g.on_close(handler!(app, |a| close(&a)));
    refresh(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resting_control_is_taken_and_a_used_one_refused() {
        let m = MidiMonitor::new(64);
        m.set_enabled(true);
        let mut r = Runtime {
            rx: Some(m.subscribe()),
            setup: Some(GuidedSetup::new(
                Template::bundled("two-deck-entry").unwrap(),
            )),
            ..Default::default()
        };
        let t0 = Instant::now();
        m.record("PORT", Direction::In, &[0x90, 0x0B, 0x7F], Vec::new());
        m.record("PORT", Direction::In, &[0x90, 0x0B, 0x00], Vec::new());
        assert!(r.poll(t0));
        assert_eq!(r.port, "PORT");
        // Still moving: not yet taken.
        assert!(!r.poll(t0 + Duration::from_millis(300)));
        assert_eq!(r.setup.as_ref().unwrap().index, 0);
        assert!(r.poll(t0 + QUIET + Duration::from_millis(10)));
        assert_eq!(r.setup.as_ref().unwrap().index, 1);
        // The same key for CUE: refused, step 1 named.
        let t1 = t0 + Duration::from_secs(2);
        m.record("PORT", Direction::In, &[0x90, 0x0B, 0x7F], Vec::new());
        r.poll(t1);
        r.poll(t1 + QUIET);
        assert_eq!(r.taken, 1);
        assert_eq!(r.setup.as_ref().unwrap().index, 1);
        // Output to the unit does not count.
        m.record("PORT", Direction::Out, &[0x90, 0x0C, 0x7F], Vec::new());
        assert!(!r.poll(t1 + Duration::from_secs(1)));
    }

    #[test]
    fn mapping_files_never_overwrite_one_another() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("library.sqlite");
        let a = mapping_path(&lib, "My Unit");
        assert!(a.ends_with("mappings/my-unit.toml"));
        std::fs::create_dir_all(a.parent().unwrap()).unwrap();
        std::fs::write(&a, "x").unwrap();
        assert!(mapping_path(&lib, "My Unit").ends_with("mappings/my-unit-2.toml"));
    }
}
