//! Roadmap [24]: the touch layout beyond the kiosk.

use super::harness::{Fixture, Rect, Ui, volumes_lock};
use slint::ComponentHandle;

/// Turns the touch layout on as Preferences does.
pub(crate) fn touch_on(ui: &Ui, on: bool) {
    ui.app.state.borrow_mut().change_settings(|s| s.view.touch_layout = on);
    let (settings, lite) = {
        let st = ui.app.state.borrow();
        (st.settings.clone(), st.config.lite)
    };
    crate::shell::apply_theme(&ui.app.ui.global::<crate::Theme>(), &settings, lite);
    crate::shell::apply_browser_style(&ui.app.ui, &settings, on);
    crate::touch::applied(&ui.app);
    ui.pump(400);
}

/// Controls under 44 px on an edge (a list's rows go with the list).
fn small(ui: &Ui) -> Vec<String> {
    crate::help::controls(ui.app.ui.window())
        .into_iter()
        .filter(|c| !c.part && (c.rect.2 < 43.5 || c.rect.3 < 43.5))
        .map(|c| format!("`{}` \"{}\" {:.0}×{:.0} at {:.0},{:.0}", c.id, c.name, c.rect.2, c.rect.3, c.rect.0, c.rect.1))
        .collect()
}

fn views() -> Vec<(&'static str, fn(&Ui))> {
    vec![
        ("perform (decks, mixer, pads, browser)", |u| {
            u.app.ui.invoke_set_mode("perform".into());
            crate::engine::ensure_engine(&u.app);
            u.wait_until(30.0, |u| u.has("mixer.ch1.fader"));
            u.pump(400);
        }),
        ("perform with the FX panel", |u| {
            if !u.app.ui.get_fx_panel() {
                u.app.ui.invoke_toggle_fx_panel();
            }
            u.pump(400);
        }),
        ("perform with the lighting strip", |u| {
            if u.app.ui.get_fx_panel() {
                u.app.ui.invoke_toggle_fx_panel();
            }
            u.app.ui.global::<crate::Lighting>().set_panel(true);
            u.pump(400);
        }),
        ("lighting", |u| {
            u.app.ui.global::<crate::Lighting>().set_panel(false);
            u.app.ui.invoke_set_mode("lighting".into());
            u.pump(400);
        }),
        ("create", |u| {
            u.app.ui.invoke_set_mode("create".into());
            u.pump(400);
        }),
        ("create (a note picked)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        ("create (pads far from the note)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage_far(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        ("create (a scale chosen)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage_scale(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        ("create (a full melody)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage_melody(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        ("create (rendered, READY)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage_rendered(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        // [77] Sent to Perform (SENT), and the MELODY pads with notes and
        // without.
        ("create (sent, SENT)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage_sent(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        // [78] What fits: the chord keys in FITS and THEORY.
        ("create (what fits)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "melody");
            crate::create::stage_theory(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        ("create theory (what fits)", |u| {
            u.app.ui.invoke_set_mode("create".into());
            crate::create::set_view(&u.app, "theory");
            crate::create::stage_theory(&u.app, u.fx.tracks[0].0);
            u.pump(400);
        }),
        ("perform (MELODY pads)", |u| {
            crate::create::stage_melody_pads(&u.app, true);
            u.pump(400);
        }),
        ("perform (MELODY pads, nothing sent)", |u| {
            crate::create::stage_melody_pads(&u.app, false);
            u.pump(400);
        }),
        // [76] The CREATED list in Perform's browser (PREPARE's top bar is
        // not part of the touch layout).
        ("perform with the CREATED list", |u| {
            crate::create::stage_created(&u.app, u.fx.tracks[0].0);
            u.app.ui.invoke_set_mode("perform".into());
            u.pump(400);
        }),
    ]
}

/// With the touch layout on, every control of the decks, mixer, pads, FX,
/// browser and lighting is at least 44 px on both edges, at 1440 × 900 and
/// at 1280 × 800.
#[test]
fn touch_layout_gives_every_control_44_px() {
    let _lock = volumes_lock();
    let ui = Ui::start(Fixture::new(), (1440, 900), false);
    touch_on(&ui, true);
    assert!(ui.app.ui.global::<crate::Theme>().get_touch(), "touch layout");
    assert_eq!(ui.app.ui.global::<crate::Theme>().get_row_height(), 44.0, "touch rows");
    let mut report = Vec::new();
    for size in [(1440, 900), (1280, 800)] {
        ui.resize(size);
        for (view, setup) in views() {
            setup(&ui);
            for s in small(&ui) {
                report.push(format!("{}×{} {view}: {s}", size.0, size.1));
            }
        }
    }
    assert!(report.is_empty(), "{} controls under 44 px:\n{}", report.len(), report.join("\n"));
    // Off again: the desktop deck is back.
    touch_on(&ui, false);
    ui.app.ui.invoke_set_mode("perform".into());
    ui.pump(400);
    assert!(!ui.app.ui.global::<crate::Theme>().get_touch());
}

/// A touch offers the touch layout once (not in the kiosk), and the
/// answer is kept.
#[test]
fn a_touch_offers_the_touch_layout_once() {
    let _lock = volumes_lock();
    let ui = Ui::open((1440, 900));
    crate::touch::touched(&ui.app);
    ui.pump(200);
    assert!(ui.has("app.touch-offer"), "the offer shows");
    ui.click("app.touch-offer.use");
    assert!(ui.wait_until(5.0, |u| !u.has("app.touch-offer")));
    assert!(ui.app.state.borrow().settings.view.touch_layout, "touch layout on");
    ui.pump(300);
    crate::touch::touched(&ui.app);
    ui.pump(200);
    assert!(!ui.has("app.touch-offer"), "offered once");
    // The help key and the decks' transport are touch keys.
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    assert!(ui.wait_until(30.0, |u| u.has("deck1.play")));
    assert!(ui.wait_until(5.0, |u| u.app.ui.global::<crate::Theme>().get_touch_mode()), "touch keys in Perform");
    let play = Rect::of(&ui.get("deck1.play"));
    assert!(play.w >= 44.0 && play.h >= 44.0, "{play:?}");
}
