//! The booth view (roadmap [58]): the decks of Perform drawn large in their
//! own window, like the detached LIGHTING view, for a screen in the booth
//! or the laptop in full screen. It shares the deck models of the main
//! window, so it shows exactly what Perform shows, frame by frame. A
//! double-click switches full screen.

use crate::app::App;
use slint::ComponentHandle;
use std::rc::Rc;

/// Opens the booth window (or brings it forward).
pub(crate) fn open(app: &Rc<App>) {
    if let Some(w) = app.booth_window.borrow().as_ref() {
        let _ = w.show();
        return;
    }
    let Ok(w) = crate::BoothWindow::new() else {
        return;
    };
    apply_theme_to(app, &w);
    {
        let d = app.decks.borrow();
        w.set_decks(d.views.all.handle());
        w.set_lane_motion(d.lane_motion());
    }
    let weak = w.as_weak();
    w.global::<crate::Booth>().on_toggle_full_screen(move || {
        if let Some(w) = weak.upgrade() {
            let full = w.window().is_fullscreen();
            w.window().set_fullscreen(!full);
        }
    });
    let _ = w.show();
    *app.booth_window.borrow_mut() = Some(w);
    // The booth shows Perform; make sure its decks are being drawn.
    if app.ui.get_mode() != "perform" {
        app.ui.invoke_set_mode("perform".into());
    }
}

fn apply_theme_to(app: &App, w: &crate::BoothWindow) {
    let st = app.state.borrow();
    crate::shell::apply_theme(&w.global::<crate::Theme>(), &st.settings, st.config.lite);
}

/// Appearance, contrast or palette changed.
pub(crate) fn apply_theme(app: &App) {
    if let Some(w) = app.booth_window.borrow().as_ref() {
        apply_theme_to(app, w);
    }
}

pub(crate) fn wire(app: &Rc<App>) {
    if od_core::env::var("DEMO_SAFETY").is_some_and(|v| v.split(',').any(|p| p == "booth")) {
        let weak = Rc::downgrade(app);
        slint::Timer::single_shot(std::time::Duration::from_secs(3), move || {
            if let Some(a) = weak.upgrade() {
                open(&a);
                // Screenshots: the booth window at the main window's size.
                let size = a.ui.window().size().to_logical(a.ui.window().scale_factor());
                if let Some(w) = a.booth_window.borrow().as_ref() {
                    w.window().set_size(size);
                }
            }
        });
    }
}
