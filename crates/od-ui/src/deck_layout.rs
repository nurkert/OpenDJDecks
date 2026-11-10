//! Deck layouts (roadmap [6]): Simple, Standard and Full, groups that can be
//! hidden as a whole, and named layouts saved in the settings.
//!
//! - **Simple**: a calm deck of about 25 large controls (`SimpleControls`
//!   in `perform.slint`); setup keys, the phase strip and the effects are
//!   hidden, the pads always play hot cues.
//! - **Standard**: the deck as it has always been; nothing hidden.
//! - **Full**: every group, with the Tempo FX panel open.
//!
//! A hidden group is not built at all in the UI (`if` in Slint), so it can
//! neither be seen nor focused. The mixer column is the top bar's MIXER
//! switch (`perform.mixer_panel`); the other groups are in
//! `perform.hidden`.

use crate::app::{App, handler};
use crate::settings::SavedLayout;
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::rc::Rc;

/// The three layouts.
pub(crate) const LEVELS: [&str; 3] = ["simple", "standard", "full"];

/// Groups kept in `perform.hidden` (the mixer has its own switch).
pub(crate) const HIDEABLE: [&str; 6] = ["pads", "loops", "stems", "fx", "phase", "setup"];

/// What choosing a layout sets: hidden groups, mixer column, Tempo FX panel.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Preset {
    pub hidden: Vec<String>,
    pub mixer: bool,
    pub fx_panel: bool,
}

pub(crate) fn preset(level: &str) -> Preset {
    match level {
        "simple" => Preset {
            hidden: vec!["fx".into(), "phase".into(), "setup".into()],
            mixer: true,
            fx_panel: false,
        },
        "full" => Preset {
            hidden: Vec::new(),
            mixer: true,
            fx_panel: true,
        },
        // Standard: today's default, nothing hidden and no panel opened.
        _ => Preset {
            hidden: Vec::new(),
            mixer: true,
            fx_panel: false,
        },
    }
}

/// `hidden` with `group` switched (shown ↔ hidden), sorted.
pub(crate) fn toggled(hidden: &[String], group: &str) -> Vec<String> {
    let mut h: Vec<String> = hidden.to_vec();
    if let Some(i) = h.iter().position(|g| g == group) {
        h.remove(i);
    } else if HIDEABLE.contains(&group) {
        h.push(group.to_string());
    }
    h.sort();
    h
}

/// `saved` with `layout` stored under its name (an existing one of the same
/// name, ignoring case, is replaced in place).
pub(crate) fn with_saved(saved: &[SavedLayout], layout: SavedLayout) -> Vec<SavedLayout> {
    let mut v = saved.to_vec();
    match v
        .iter()
        .position(|l| l.name.eq_ignore_ascii_case(&layout.name))
    {
        Some(i) => v[i] = layout,
        None => v.push(layout),
    }
    v
}

/// Shows the layout the settings hold.
pub(crate) fn apply(app: &App) {
    let p = app.state.borrow().settings.perform.clone();
    let g = app.ui.global::<crate::DeckLayout>();
    g.set_level(p.layout.clone().into());
    let shown = |name: &str| !p.hidden.iter().any(|h| h == name);
    g.set_pads(shown("pads"));
    g.set_loops(shown("loops"));
    g.set_stems(shown("stems"));
    g.set_fx(shown("fx"));
    g.set_phase(shown("phase"));
    g.set_setup(shown("setup"));
    let names: Vec<SharedString> = p
        .saved_layouts
        .iter()
        .map(|l| l.name.clone().into())
        .collect();
    let same = {
        let cur = g.get_saved();
        cur.row_count() == names.len() && cur.iter().zip(&names).all(|(a, b)| &a == b)
    };
    if !same {
        g.set_saved(Rc::new(VecModel::from(names)).into());
    }
    g.set_current(p.saved_current.clone().into());
    crate::shell::apply_panels(app);
    app.ui.set_fx_panel(p.fx_panel);
}

/// Simple pads always play hot cues (their mode keys are hidden).
fn hot_cue_pads(app: &App) {
    for d in 0..crate::decks::DECKS {
        if app.decks.borrow().pad_mode[d] != od_controller::PadMode::HotCue {
            crate::decks::set_pad_mode(app, d, od_controller::PadMode::HotCue);
        }
    }
}

/// Simple, Standard or Full: the layout's own groups and panels.
pub(crate) fn set_level(app: &App, level: &str) {
    if !LEVELS.contains(&level) {
        return;
    }
    let p = preset(level);
    let level = level.to_string();
    app.state.borrow_mut().change_settings(|s| {
        s.perform.layout = level.clone();
        s.perform.hidden = p.hidden.clone();
        s.perform.mixer_panel = p.mixer;
        s.perform.fx_panel = p.fx_panel;
        s.perform.saved_current.clear();
    });
    if level == "simple" {
        hot_cue_pads(app);
    }
    apply(app);
}

/// Hides or shows a whole group ("mixer" is the mixer column).
pub(crate) fn toggle_group(app: &App, group: &str) {
    if group == "mixer" {
        crate::shell::toggle_panel(app, "mixer");
    } else if HIDEABLE.contains(&group) {
        let hidden = toggled(&app.state.borrow().settings.perform.hidden, group);
        app.state
            .borrow_mut()
            .change_settings(|s| s.perform.hidden = hidden.clone());
    } else {
        return;
    }
    app.state
        .borrow_mut()
        .change_settings(|s| s.perform.saved_current.clear());
    apply(app);
}

/// Saves the layout in use under `name` (replacing one of the same name).
pub(crate) fn save(app: &App, name: &str) {
    let name = name.trim().to_string();
    if name.is_empty() {
        return;
    }
    let layout = {
        let p = &app.state.borrow().settings.perform;
        SavedLayout {
            name: name.clone(),
            layout: p.layout.clone(),
            hidden: p.hidden.clone(),
            mixer_panel: p.mixer_panel,
            fx_panel: p.fx_panel,
        }
    };
    let saved = with_saved(&app.state.borrow().settings.perform.saved_layouts, layout);
    app.state.borrow_mut().change_settings(|s| {
        s.perform.saved_layouts = saved.clone();
        s.perform.saved_current = name.clone();
    });
    app.status(od_i18n::trf("Layout saved: {}", &[&name]));
    apply(app);
}

/// Switches to a saved layout.
pub(crate) fn apply_saved(app: &App, name: &str) {
    let Some(l) = app
        .state
        .borrow()
        .settings
        .perform
        .saved_layouts
        .iter()
        .find(|l| l.name == name)
        .cloned()
    else {
        return;
    };
    app.state.borrow_mut().change_settings(|s| {
        s.perform.layout = l.layout.clone();
        s.perform.hidden = l.hidden.clone();
        s.perform.mixer_panel = l.mixer_panel;
        s.perform.fx_panel = l.fx_panel;
        s.perform.saved_current = l.name.clone();
    });
    if l.layout == "simple" {
        hot_cue_pads(app);
    }
    apply(app);
}

/// Deletes a saved layout (the layout in use stays as it is).
pub(crate) fn remove(app: &App, name: &str) {
    let name = name.to_string();
    app.state.borrow_mut().change_settings(|s| {
        s.perform.saved_layouts.retain(|l| l.name != name);
        if s.perform.saved_current == name {
            s.perform.saved_current.clear();
        }
    });
    apply(app);
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=layout-simple` (or
/// `layout-full`) shows that layout without storing it in the library;
/// `layout-menu` opens the view menu once the window is up.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else {
        return;
    };
    if v.split(',').any(|p| p == "layout-menu") {
        let weak = Rc::downgrade(app);
        slint::Timer::single_shot(std::time::Duration::from_secs(4), move || {
            if let Some(a) = weak.upgrade() {
                a.ui.global::<crate::DeckLayout>().set_show_menu(true);
            }
        });
    }
    let Some(level) = v
        .split(',')
        .find_map(|p| p.strip_prefix("layout-"))
        .filter(|l| LEVELS.contains(l))
    else {
        return;
    };
    let p = preset(level);
    {
        let mut st = app.state.borrow_mut();
        st.settings.perform.layout = level.to_string();
        st.settings.perform.hidden = p.hidden;
        st.settings.perform.mixer_panel = p.mixer;
        st.settings.perform.fx_panel = p.fx_panel;
    }
    if level == "simple" {
        hot_cue_pads(app);
    }
}

pub(crate) fn wire(app: &Rc<App>) {
    demo(app);
    let g = app.ui.global::<crate::DeckLayout>();
    g.on_set_level(handler!(app, |a, l: SharedString| set_level(&a, &l)));
    g.on_toggle(handler!(app, |a, g: SharedString| toggle_group(&a, &g)));
    g.on_save(handler!(app, |a, n: SharedString| save(&a, &n)));
    g.on_apply(handler!(app, |a, n: SharedString| apply_saved(&a, &n)));
    g.on_remove(handler!(app, |a, n: SharedString| remove(&a, &n)));
    apply(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_hide_the_right_groups() {
        assert!(preset("standard").hidden.is_empty() && !preset("standard").fx_panel);
        assert!(preset("full").hidden.is_empty() && preset("full").fx_panel);
        let s = preset("simple");
        assert_eq!(s.hidden, ["fx", "phase", "setup"]);
        assert!(s.mixer && !s.fx_panel);
        // Anything unknown is Standard.
        assert_eq!(preset("huge"), preset("standard"));
    }

    #[test]
    fn groups_toggle_and_unknown_ones_are_ignored() {
        let h = toggled(&[], "pads");
        assert_eq!(h, ["pads"]);
        let h = toggled(&h, "stems");
        assert_eq!(h, ["pads", "stems"]);
        assert_eq!(toggled(&h, "pads"), ["stems"]);
        assert_eq!(toggled(&h, "mixer"), h, "the mixer has its own switch");
        assert_eq!(toggled(&h, "nonsense"), h);
    }

    #[test]
    fn saving_under_a_known_name_replaces_it() {
        let a = SavedLayout {
            name: "Club".into(),
            layout: "full".into(),
            ..Default::default()
        };
        let b = SavedLayout {
            name: "Home".into(),
            layout: "simple".into(),
            ..Default::default()
        };
        let v = with_saved(&with_saved(&[], a), b);
        assert_eq!(v.len(), 2);
        let c = SavedLayout {
            name: "club".into(),
            layout: "standard".into(),
            ..Default::default()
        };
        let v = with_saved(&v, c);
        assert_eq!(v.len(), 2);
        assert_eq!(
            (v[0].name.as_str(), v[0].layout.as_str()),
            ("club", "standard")
        );
    }

    #[test]
    fn layouts_survive_the_settings_round_trip_and_bad_values_fall_back() {
        let mut s = crate::settings::Settings::default();
        assert_eq!(s.perform.layout, "standard");
        s.perform.layout = "simple".into();
        s.perform.hidden = vec!["setup".into(), "bogus".into(), "phase".into()];
        s.perform.saved_layouts = vec![
            SavedLayout {
                name: "Warm-up".into(),
                layout: "simple".into(),
                hidden: vec!["pads".into()],
                ..Default::default()
            },
            SavedLayout {
                name: "  ".into(),
                ..Default::default()
            },
        ];
        let back = crate::settings::Settings::from_json(&s.to_json());
        assert_eq!(back.perform.layout, "simple");
        assert_eq!(back.perform.hidden, ["phase", "setup"]);
        assert_eq!(back.perform.saved_layouts.len(), 1);
        assert_eq!(back.perform.saved_layouts[0].hidden, ["pads"]);
        let bad = crate::settings::Settings::from_json(r#"{"perform": {"layout": "huge"}}"#);
        assert_eq!(bad.perform.layout, "standard");
    }
}
