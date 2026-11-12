//! Roadmap [25]: keyboard focus and readable names.
//!
//! - Every interactive control of each main view is reached by Tab, and the
//!   focused control shows the focus ring (2px in `Theme.focus`).
//! - Every interactive control has a readable accessible name: never empty,
//!   never a test id such as `deck1.stem.vocal`.
//!
//! The element tree is walked through `i_slint_core` (as the overflow audit
//! does), which also tells which item has the keyboard focus.

use super::harness::{Fixture, Ui, volumes_lock};
use i_slint_core::accessibility::AccessibleStringProperty as A;
use i_slint_core::item_tree::{ItemRc, ParentItemTraversalMode};
use i_slint_core::items::{BasicBorderRectangle, BorderRectangle};
use i_slint_core::window::WindowInner;
use slint::ComponentHandle;
use slint::platform::{Key, WindowEvent};

use crate::help::operable;

pub(crate) struct Control {
    pub rc: ItemRc,
    pub id: String,
    pub name: String,
    pub kind: String,
    pub rect: (f32, f32, f32, f32),
    /// Part of another control (the ‹ › of a stepper) or of a list row:
    /// reached through that control or the list, not a Tab stop of its own.
    pub part: bool,
}

impl Control {
    pub fn show(&self) -> String {
        format!(
            "`{}` ({}) name \"{}\" at {:.0},{:.0} {:.0}×{:.0}",
            self.id, self.kind, self.name, self.rect.0, self.rect.1, self.rect.2, self.rect.3
        )
    }
}

fn prop(rc: &ItemRc, p: A) -> String {
    rc.accessible_string_property(p)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn kind(rc: &ItemRc) -> String {
    rc.element_type_names_and_ids(0)
        .and_then(|v| {
            v.iter()
                .find(|(t, _)| !t.is_empty() && !t.starts_with("Native"))
                .or(v.first())
                .map(|(t, _)| t.to_string())
        })
        .unwrap_or_default()
}

/// The visible, enabled controls of a window (the help overlay's walk).
pub(crate) fn controls(window: &slint::Window) -> Vec<Control> {
    crate::help::controls(window)
        .into_iter()
        .map(|f| Control {
            kind: kind(&f.rc),
            rc: f.rc,
            id: f.id,
            name: f.name,
            rect: f.rect,
            part: f.part,
        })
        .collect()
}

/// The control the keyboard focus is in: the outermost operable ancestor
/// of the focus item that carries a test id, else the nearest operable one.
fn focused(window: &slint::Window) -> Option<ItemRc> {
    let inner = WindowInner::from_pub(window);
    let f = inner.focus_item.borrow().upgrade()?;
    let mut best: Option<ItemRc> = None;
    let mut cur = Some(f);
    while let Some(c) = cur {
        if operable(c.accessible_role()) {
            let has_id = !prop(&c, A::Id).is_empty();
            if best.is_none() || has_id {
                best = Some(c.clone());
            }
            if has_id {
                break;
            }
        }
        cur = c.parent_item(ParentItemTraversalMode::StopAtPopups);
    }
    best
}

/// Whether a focus ring (a 2px border in the focus colour) is drawn in the
/// control.
fn ring_shown(rc: &ItemRc, focus: slint::Color) -> bool {
    let border = |c: &ItemRc| -> bool {
        let (w, col) = if let Some(r) = c.downcast::<BorderRectangle>() {
            let r = r.as_pin_ref();
            (r.border_width().get(), r.border_color())
        } else if let Some(r) = c.downcast::<BasicBorderRectangle>() {
            let r = r.as_pin_ref();
            (r.border_width().get(), r.border_color())
        } else {
            return false;
        };
        w >= 1.5 && col.color() == focus
    };
    fn any(rc: &ItemRc, f: &dyn Fn(&ItemRc) -> bool) -> bool {
        if f(rc) {
            return true;
        }
        let mut c = rc.first_child();
        while let Some(ch) = c {
            if ch.is_visible() && any(&ch, f) {
                return true;
            }
            c = ch.next_sibling();
        }
        false
    }
    // The ring of a control drawn beside it (a field's text input).
    any(rc, &border) || rc.parent_item(ParentItemTraversalMode::StopAtPopups).is_some_and(|p| any(&p, &border))
}

/// Presses Tab until the focus comes round again; returns what was missed
/// and the focused controls that showed no ring.
pub(crate) fn tab_walk(ui: &Ui) -> (Vec<String>, Vec<String>) {
    let window = ui.app.ui.window();
    let all = controls(window);
    let focus = ui.app.ui.global::<crate::Theme>().get_focus();
    let mut reached = vec![false; all.len()];
    let mut ringless = Vec::new();
    let mut first: Option<ItemRc> = None;
    let mut rounds = 0;
    for _ in 0..(all.len() * 3 + 60) {
        let s: slint::SharedString = Key::Tab.into();
        window.dispatch_event(WindowEvent::KeyPressed { text: s.clone() });
        window.dispatch_event(WindowEvent::KeyReleased { text: s });
        ui.pump(16);
        let Some(f) = focused(window) else { continue };
        if first.as_ref() == Some(&f) {
            rounds += 1;
            if rounds >= 1 {
                break;
            }
        }
        if first.is_none() {
            first = Some(f.clone());
        }
        if let Some(i) = all.iter().position(|c| c.rc == f) {
            if !reached[i] && !ring_shown(&f, focus) {
                ringless.push(all[i].show());
            }
            reached[i] = true;
        }
    }
    let missed = all
        .iter()
        .zip(&reached)
        .filter(|(c, r)| !**r && !c.part)
        .map(|(c, _)| c.show())
        .collect();
    (missed, ringless)
}

/// Names that are empty or look like a test id.
pub(crate) fn bad_names(window: &slint::Window) -> Vec<String> {
    controls(window)
        .into_iter()
        .filter(|c| c.name.trim().is_empty() || crate::a11y::id_like(&c.name) || (c.name == c.id && !c.id.is_empty()))
        .map(|c| c.show())
        .collect()
}

/// The main views of the tests: (name, setup).
fn views() -> Vec<(&'static str, fn(&Ui))> {
    vec![
        ("prepare", |u| {
            u.app.ui.invoke_set_mode("prepare".into());
            u.app.ui.invoke_select_section("library".into());
            u.app.ui.invoke_row_clicked(0, false, false);
            u.pump(300);
        }),
        ("perform", |u| {
            u.app.ui.invoke_set_mode("perform".into());
            crate::engine::ensure_engine(&u.app);
            u.wait_until(30.0, |u| u.has("mixer.ch1.fader"));
            u.pump(300);
        }),
        ("perform.full", |u| {
            u.app.ui.invoke_set_mode("perform".into());
            crate::engine::ensure_engine(&u.app);
            crate::deck_layout::set_level(&u.app, "full");
            u.wait_until(30.0, |u| u.has("mixer.ch1.fader"));
            u.pump(300);
        }),
        ("lighting", |u| {
            u.app.ui.invoke_set_mode("lighting".into());
            u.pump(300);
        }),
        ("create", |u| {
            u.app.ui.invoke_set_mode("create".into());
            u.pump(300);
        }),
        ("create (a note picked)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage(&u.app, u.fx.tracks[0].0);
            u.pump(300);
        }),
        ("create (pads far from the note)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage_far(&u.app, u.fx.tracks[0].0);
            u.pump(300);
        }),
    ]
}

fn focus_window(ui: &Ui) {
    // Start from the window, as after a click on an empty spot.
    ui.app.ui.window().dispatch_event(WindowEvent::KeyPressed { text: Key::Escape.into() });
    ui.app.ui.window().dispatch_event(WindowEvent::KeyReleased { text: Key::Escape.into() });
    ui.pump(50);
}

/// Every operable control of Prepare, Perform (Standard and Full),
/// Lighting and Create is a Tab stop and shows the focus ring when it has the focus.
#[test]
fn tab_reaches_every_control_with_a_visible_ring() {
    let _lock = volumes_lock();
    let ui = Ui::start(Fixture::new(), (1440, 900), false);
    let mut report = Vec::new();
    for (view, setup) in views() {
        setup(&ui);
        focus_window(&ui);
        let (missed, ringless) = tab_walk(&ui);
        for m in missed {
            report.push(format!("{view}: not reached by Tab: {m}"));
        }
        for m in ringless {
            report.push(format!("{view}: no focus ring: {m}"));
        }
    }
    assert!(report.is_empty(), "{} problems:\n{}", report.len(), report.join("\n"));
}

/// Every operable control of the main views and of Preferences has a
/// readable name.
#[test]
fn every_control_has_a_readable_name() {
    let _lock = volumes_lock();
    let ui = Ui::start(Fixture::new(), (1440, 900), false);
    let mut report = Vec::new();
    for (view, setup) in views() {
        setup(&ui);
        for b in bad_names(ui.app.ui.window()) {
            report.push(format!("{view}: {b}"));
        }
    }
    ui.app.ui.invoke_open_preferences();
    ui.pump(300);
    let prefs = ui.app.prefs.borrow().window.as_ref().map(|w| w.clone_strong()).expect("preferences window");
    for page in ["audio", "view", "analysis", "controller", "keyboard", "recording", "advanced"] {
        prefs.set_page(page.into());
        ui.pump(300);
        for b in bad_names(prefs.window()) {
            report.push(format!("prefs.{page}: {b}"));
        }
    }
    assert!(report.is_empty(), "{} unreadable names:\n{}", report.len(), report.join("\n"));
}
