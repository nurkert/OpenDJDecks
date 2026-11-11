//! The touch layout (roadmap [24]): chosen in Preferences › View › Touch,
//! and offered once when a touch arrives on a screen while it is off.
//!
//! With it on, `Theme.touch` and `Theme.touch-mode` are set: the decks use
//! the touch decks, the browser its 44 px rows, and every key, knob, fader,
//! pad, field and tab at least 44 px on both edges.

use crate::app::{App, handler};
use slint::{ComponentHandle, SharedString};
use std::rc::Rc;

/// A touch arrived (winit touch event): offer the touch layout once.
pub(crate) fn touched(app: &App) {
    let (on, offered, kiosk) = {
        let st = app.state.borrow();
        (st.settings.view.touch_layout, st.settings.view.touch_offered, st.config.kiosk)
    };
    if !on && !offered && !kiosk {
        app.ui.global::<crate::TouchLayout>().set_offer(true);
    }
}

/// The setting changed: the theme and the rows follow (the frame tick
/// sets `Theme.touch`).
pub(crate) fn applied(app: &App) {
    app.ui.global::<crate::TouchLayout>().set_offer(false);
    // The artwork column steps out on touch (as in the kiosk).
    crate::browser::refresh_columns(app);
}

fn answer(app: &Rc<App>, what: &str) {
    let on = what == "use";
    app.state.borrow_mut().change_settings(|s| {
        s.view.touch_offered = true;
        if on {
            s.view.touch_layout = true;
        }
    });
    let (settings, lite, kiosk) = {
        let st = app.state.borrow();
        (st.settings.clone(), st.config.lite, st.config.kiosk)
    };
    crate::shell::apply_theme(&app.ui.global::<crate::Theme>(), &settings, lite);
    crate::shell::apply_browser_style(&app.ui, &settings, kiosk || settings.view.touch_layout);
    applied(app);
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::TouchLayout>();
    g.on_answer(handler!(app, |a, w: SharedString| answer(&a, &w)));
    // Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=touch` turns the
    // touch layout on without storing it; `touch-offer` shows the offer.
    if let Some(v) = od_core::env::var("DEMO_PROOF") {
        let parts: Vec<&str> = v.split(',').collect();
        if parts.contains(&"touch") {
            app.state.borrow_mut().settings.view.touch_layout = true;
            let (settings, lite) = {
                let st = app.state.borrow();
                (st.settings.clone(), st.config.lite)
            };
            crate::shell::apply_theme(&app.ui.global::<crate::Theme>(), &settings, lite);
            crate::shell::apply_browser_style(&app.ui, &settings, true);
        }
        if parts.contains(&"touch-offer") {
            g.set_offer(true);
        }
    }
}
