//! The gig preset (roadmap [3]): one step sets high contrast, the colour-blind
//! palette and a larger interface; switching it off restores what was set.
//! The interface scale applies at once (no restart): the window's scale factor
//! becomes the operating system's factor times the setting.

use crate::app::{App, handler};
use crate::settings::GigRestore;
use slint::ComponentHandle;
use slint::platform::WindowEvent;
use std::cell::Cell;
use std::rc::Rc;

/// Interface scale of the preset (the Raspberry Pi keeps its own size).
const GIG_SCALE: f32 = 1.25;
/// Choices of the top-bar menu.
const SCALES: [f32; 3] = [1.0, 1.25, 1.5];

thread_local! {
    /// The operating system's scale factor of the main window and the factor
    /// last applied, once the window exists.
    static OS_SCALE: Cell<Option<(f32, f32)>> = const { Cell::new(None) };
}

/// A user-set `SLINT_SCALE_FACTOR` wins over the setting.
fn scale_external() -> bool {
    std::env::var_os("SLINT_SCALE_FACTOR").is_some()
}

fn set_window_scale(window: &slint::Window, target: f32) {
    let physical = window.size();
    window.dispatch_event(WindowEvent::ScaleFactorChanged {
        scale_factor: target,
    });
    window.dispatch_event(WindowEvent::Resized {
        size: physical.to_logical(target),
    });
}

/// Applies the scale setting to the windows, keeping the OS factor as base.
pub(crate) fn apply(app: &App) {
    refresh(app);
    if scale_external() {
        return;
    }
    let setting = app.state.borrow().settings.view.scale;
    let window = app.ui.window();
    let os = OS_SCALE
        .with(|c| c.get())
        .map(|(os, _)| os)
        .unwrap_or_else(|| window.scale_factor());
    let target = os * setting;
    if (window.scale_factor() - target).abs() > 1e-3 {
        set_window_scale(window, target);
        crate::app::App::refresh_all(app);
    }
    OS_SCALE.with(|c| c.set(Some((os, target))));
    let w = app.prefs.borrow().window.as_ref().map(|w| w.as_weak());
    if let Some(w) = w.and_then(|w| w.upgrade()) {
        if (w.window().scale_factor() - target).abs() > 1e-3 {
            set_window_scale(w.window(), target);
        }
    }
}

/// Follows the OS factor (another display) and applies the setting over it.
pub(crate) fn tick(app: &App) {
    if scale_external() {
        return;
    }
    let current = app.ui.window().scale_factor();
    let setting = app.state.borrow().settings.view.scale;
    let changed = OS_SCALE.with(|c| match c.get() {
        // The window exists now; the factor it has is the OS factor.
        None => {
            c.set(Some((current, current)));
            true
        }
        // Something else changed the factor: that is the new OS factor.
        Some((_, applied)) if (current - applied).abs() > 1e-3 => {
            c.set(Some((current, current)));
            true
        }
        _ => false,
    });
    if changed && (setting - 1.0).abs() > 1e-3 {
        apply(app);
    }
}

/// Mirrors the preset's state into the top bar.
fn refresh(app: &App) {
    let view = app.state.borrow().settings.view.clone();
    let gig = app.ui.global::<crate::Gig>();
    gig.set_active(view.gig_restore.is_some());
    gig.set_contrast(view.high_contrast);
    gig.set_colorblind(view.colorblind_safe);
    gig.set_scale(
        SCALES
            .iter()
            .position(|f| (f - view.scale).abs() < 0.01)
            .map(|i| i as i32)
            .unwrap_or(-1),
    );
}

/// Top bar and first run: `"toggle"`, `"contrast"`, `"colorblind"` or
/// `"scale:<index>"` (1.0, 1.25, 1.5).
fn act(app: &App, what: &str) {
    let kiosk = app.state.borrow().config.kiosk;
    app.state.borrow_mut().change_settings(|s| match what {
        "toggle" => match s.view.gig_restore.take() {
            Some(old) => {
                s.view.high_contrast = old.high_contrast;
                s.view.colorblind_safe = old.colorblind_safe;
                s.view.scale = old.scale;
            }
            None => {
                s.view.gig_restore = Some(GigRestore {
                    high_contrast: s.view.high_contrast,
                    colorblind_safe: s.view.colorblind_safe,
                    scale: s.view.scale,
                });
                s.view.high_contrast = true;
                s.view.colorblind_safe = true;
                if !kiosk {
                    s.view.scale = GIG_SCALE;
                }
            }
        },
        "contrast" => s.view.high_contrast = !s.view.high_contrast,
        "colorblind" => s.view.colorblind_safe = !s.view.colorblind_safe,
        other => {
            if let Some(i) = other
                .strip_prefix("scale:")
                .and_then(|i| i.parse::<usize>().ok())
                .and_then(|i| SCALES.get(i))
            {
                s.view.scale = *i;
            }
        }
    });
    crate::preferences::refresh_live(app);
}

pub(crate) fn wire(app: &Rc<App>) {
    app.ui
        .global::<crate::Gig>()
        .on_act(handler!(app, |a, what: slint::SharedString| act(&a, &what)));
    refresh(app);
    demo(app);
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=gig-menu` opens the
/// GIG menu of the top bar a few seconds after the start.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else {
        return;
    };
    if v.split(',').any(|p| p == "gig-menu") {
        let weak = Rc::downgrade(app);
        slint::Timer::single_shot(std::time::Duration::from_secs(6), move || {
            if let Some(a) = weak.upgrade() {
                a.ui.global::<crate::Gig>().set_show_menu(true);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_choices_are_the_listed_scales() {
        assert!(SCALES.contains(&GIG_SCALE));
    }
}
