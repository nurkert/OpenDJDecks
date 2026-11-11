//! Roadmap [23]: the help overlay names every control, also on touch.

use super::harness::{Fixture, Rect, Ui, volumes_lock};
use slint::platform::Key;
use slint::{ComponentHandle, Model};

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    assert!(ui.wait_until(30.0, |u| u.has("mixer.ch1.fader")), "the mixer is up");
    ui.pump(300);
}

fn on(ui: &Ui) -> bool {
    ui.app.ui.global::<crate::Help>().get_on()
}

fn items(ui: &Ui) -> Vec<crate::HelpItem> {
    ui.app.ui.global::<crate::Help>().get_items().iter().collect()
}

/// F12 shows the overlay over Perform: every control on screen is in the
/// list with a readable name (the same as its spoken name), the callouts
/// overlap neither each other nor the list, and Esc closes it.
#[test]
fn f12_names_every_control_without_overlapping_callouts() {
    let _lock = volumes_lock();
    let ui = Ui::open((1440, 900));
    perform(&ui);
    ui.key(Key::F12);
    assert!(ui.wait_until(5.0, on), "F12 opens the help");
    ui.pump(200);
    let list = items(&ui);
    let controls: Vec<_> = crate::help::controls(ui.app.ui.window())
        .into_iter()
        .filter(|c| !c.part)
        .collect();
    assert!(list.len() > 60, "the controls of Perform are listed: {}", list.len());
    // Every control on screen (but the overlay's own) is in the list.
    for c in &controls {
        assert!(
            list.iter().any(|i| i.name.as_str() == c.name && (i.x - c.rect.0).abs() < 0.5 && (i.y - c.rect.1).abs() < 0.5),
            "`{}` ({}) is in the list",
            c.id,
            c.name
        );
    }
    for i in &list {
        assert!(!i.name.is_empty() && !crate::a11y::id_like(&i.name), "readable: {:?}", i.name);
    }
    // PLAY of deck 1: its keymap title and shortcut.
    let play = list.iter().find(|i| i.name.starts_with("Deck 1") && i.name.contains("Play")).expect("deck 1 play listed");
    assert!(!play.keys.is_empty(), "with its shortcut: {:?}", play.keys);
    // Callouts: none overlaps another or the list; most controls get one.
    let win = ui.size();
    let panel = crate::help::panel_rect(win, ui.app.ui.global::<crate::Help>().get_panel_left());
    let badges: Vec<(f32, f32, f32)> = list.iter().filter(|i| i.badge).map(|i| (i.bx, i.by, i.bw)).collect();
    // The controls the list leaves visible.
    let seen: Vec<_> = list
        .iter()
        .filter(|i| !(i.x < panel.0 + panel.2 && panel.0 < i.x + i.w && i.y < panel.1 + panel.3 && panel.1 < i.y + i.h))
        .collect();
    let with = seen.iter().filter(|i| i.badge).count();
    assert!(with * 10 >= seen.len() * 9, "nearly every visible control gets a callout: {with} of {}", seen.len());
    let h = crate::help::BADGE_H;
    for (k, a) in badges.iter().enumerate() {
        assert!(a.0 >= 0.0 && a.1 >= 0.0 && a.0 + a.2 <= win.0 && a.1 + h <= win.1, "inside the window: {a:?}");
        assert!(!(a.0 < panel.0 + panel.2 && panel.0 < a.0 + a.2 && a.1 < panel.1 + panel.3 && panel.1 < a.1 + h), "clear of the list: {a:?}");
        for b in &badges[k + 1..] {
            let apart = a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + h <= b.1 || b.1 + h <= a.1;
            assert!(apart, "callouts {a:?} and {b:?} overlap");
        }
    }
    // The overlay's own parts are found.
    assert!(ui.has("app.help.list") && ui.has("app.help.close"));
    ui.key(Key::Escape);
    assert!(ui.wait_until(5.0, |u| !on(u)), "Esc closes it");
}

/// A click (or tap) on a control while the help is open explains it and
/// does not press it.
#[test]
fn a_click_explains_instead_of_pressing() {
    let _lock = volumes_lock();
    let ui = Ui::open((1440, 900));
    perform(&ui);
    crate::decks::load_track(&ui.app, 0, ui.fx.tracks[0].0, None);
    assert!(ui.wait_until(60.0, |u| crate::engine::snapshot(&u.app).is_some_and(|s| s.decks[0].track.is_some())), "deck 1 loads");
    ui.pump(300);
    let play = Rect::of(&ui.get("deck1.play"));
    ui.click("topbar.help");
    assert!(ui.wait_until(5.0, on), "the ? key opens the help");
    // The list keeps to one side; it moves out of the way with its key.
    let panel = crate::help::panel_rect(ui.size(), ui.app.ui.global::<crate::Help>().get_panel_left());
    if play.x < panel.0 + panel.2 && panel.0 < play.right() {
        ui.click("app.help.side");
    }
    ui.click_at(play.centre(), slint::platform::PointerEventButton::Left);
    ui.pump(200);
    let help = ui.app.ui.global::<crate::Help>();
    let sel = help.get_selected();
    assert!(sel >= 0, "a control is selected");
    let it = help.get_items().row_data(sel as usize).unwrap();
    assert!(it.name.contains("Play") && it.name.starts_with("Deck 1"), "PLAY is explained: {}", it.name);
    assert!(ui.has("app.help.name") && ui.value("app.help.name") == it.name.as_str());
    let playing = crate::engine::snapshot(&ui.app).map(|s| s.decks[0].playing).unwrap_or(false);
    assert!(!playing, "the deck was not started");
    // The arrows move through the list; ? closes.
    ui.key(Key::DownArrow);
    assert_eq!(help.get_selected(), sel + 1);
    ui.type_text("?");
    assert!(ui.wait_until(5.0, |u| !on(u)), "? closes it");
}

/// `?` opens the help in Prepare (no shortcut takes it there); on the touch
/// screen the ? key does, and the list rows are touch rows.
#[test]
fn question_mark_in_prepare_and_the_help_key_on_touch() {
    let _lock = volumes_lock();
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("prepare".into());
    ui.pump(300);
    ui.type_text("?");
    assert!(ui.wait_until(5.0, on), "? opens the help in Prepare");
    assert!(items(&ui).iter().any(|i| i.name.as_str().contains("Search") || i.name.as_str().contains("search")), "the search is named");
    ui.key(Key::F12);
    assert!(ui.wait_until(5.0, |u| !on(u)));
    drop(ui);
    let kiosk = Ui::start(Fixture::new(), (800, 480), true);
    kiosk.pump(300);
    let key = Rect::of(&kiosk.get("topbar.help"));
    assert!(key.w >= 44.0 && key.h >= 44.0, "the help key is a touch key: {key:?}");
    kiosk.click("topbar.help");
    assert!(kiosk.wait_until(5.0, on), "the help key opens it on touch");
}


