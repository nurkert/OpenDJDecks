//! [47] Set Flow chart, [48] Preferences previews and the stick screen,
//! [22] rails and double-click, [26] filters, room, fuzzy search and
//! type-then-Enter: driven through the real window.

use super::harness::*;
use super::session::{prefs, prefs_el};
use i_slint_backend_testing::ElementHandle;
use od_library::NodeKind;
use slint::platform::Key;
use slint::{ComponentHandle, Model};

fn row(ui: &Ui, title: &str) -> ElementHandle {
    ui.by_value("browser.row", title)
        .unwrap_or_else(|| panic!("row `{title}`"))
        .1
}

fn titles(ui: &Ui) -> Vec<String> {
    ui.app.ui.get_tracks().iter().map(|r| r.title.to_string()).collect()
}

/// A set of the three fixture tracks: 120 BPM 8A, 121 BPM 9A, then a jump
/// to 132 BPM in 3B (tempo and key warn on the second transition).
fn set_with_a_jump(ui: &Ui) -> od_library::NodeId {
    let mut lib = ui.fx.lib();
    let ids: Vec<_> = ui.fx.tracks.iter().map(|t| t.0).collect();
    for (id, (bpm, key)) in ids.iter().zip([(120.0, "8A"), (121.0, "9A"), (132.0, "3B")]) {
        let d = lib.track(*id).unwrap().duration;
        lib.set_beatgrid(*id, od_core::Beatgrid::constant(bpm, 0.0, 1, d)).unwrap();
        lib.set_key(*id, od_core::MusicalKey::parse(key)).unwrap();
    }
    let set = lib.create_node(None, NodeKind::Set, "Friday").unwrap();
    lib.add_to_node(set, &ids, None).unwrap();
    drop(lib);
    ui.app.refresh_library();
    ui.app.ui.invoke_select_node(set.0 as i32);
    ui.pump(300);
    set
}

#[test]
fn set_flow_warns_with_reasons_and_selects_the_pair() {
    let ui = Ui::open((1440, 900));
    set_with_a_jump(&ui);
    assert!(ui.wait_until(10.0, |u| u.has("browser.flow")), "the chart shows for a set");
    assert_eq!(ui.value("browser.flow"), "1", "one transition warns");
    assert_eq!(ui.value("browser.flow.transition0"), "good");
    let reason = ui.value("browser.flow.transition1");
    assert!(reason.contains("TEMPO +9.1 %") && reason.contains("KEY CLASH 9A → 3B"), "{reason}");
    let chip = ui.value("browser.flow.warning0");
    assert!(chip.starts_with("2 → 3") && chip.contains("KEY CLASH"), "{chip}");

    // A click on the warning selects both tracks of the transition.
    ui.click("browser.flow.warning0");
    ui.pump(100);
    let st = ui.app.state.borrow();
    assert_eq!(st.browser.selection.iter().copied().collect::<Vec<_>>(), [1, 2]);
    assert_eq!((st.browser.flow_pair, st.browser.flow_selected), (Some(1), Some(1)));
    drop(st);
    assert_eq!(ui.app.ui.get_flow_pair(), 1);
    // So does its marker; a position alone clears the pair.
    ui.click("browser.flow.transition0");
    assert_eq!(ui.app.state.borrow().browser.selection.len(), 2);
    assert_eq!(ui.app.ui.get_flow_pair(), 0);
    ui.click("browser.flow.point2");
    assert_eq!(ui.app.ui.get_flow_pair(), -1);
    assert_eq!(ui.app.ui.get_flow_selected(), 2);
}

#[test]
fn fuzzy_search_ranks_and_forgives_typos() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_search("brvao".into());
    ui.pump(100);
    assert_eq!(titles(&ui), ["Bravo Tune"], "two swapped letters in a short word");
    ui.app.ui.invoke_search("tune charlie".into());
    ui.pump(100);
    assert_eq!(titles(&ui)[0], "Charlie Tune", "the best match first");
    assert_eq!(titles(&ui).len(), 1, "every word has to match");
    ui.app.ui.invoke_search("test artst".into());
    ui.pump(100);
    assert_eq!(titles(&ui).len(), 3, "across title and artist, with a typo");
    // A list searches the same way.
    ui.app.ui.invoke_select_node(ui.fx.playlist.0 as i32);
    ui.app.ui.invoke_search("alpah".into());
    ui.pump(100);
    assert_eq!(titles(&ui), ["Alpha Tune"]);
}

#[test]
fn typing_in_the_list_searches_and_return_plays_the_top_hit_in_prepare() {
    let ui = Ui::open((1440, 900));
    ui.click_el(&row(&ui, "Alpha Tune"));
    ui.pump(100);
    assert_eq!(ui.value("browser.list"), "focused", "a click gives the list the keys");
    ui.type_text("charl");
    ui.pump(100);
    assert_eq!(ui.app.ui.get_search_text(), "charl", "the letters went to the search");
    assert_eq!(titles(&ui), ["Charlie Tune"]);
    ui.key(Key::Return);
    let charlie = ui.fx.tracks[2].0;
    assert!(
        ui.wait_until(20.0, |u| u.app.decks.borrow().loaded_id(crate::decks::PREVIEW_DECK) == Some(charlie)),
        "Return plays the top hit in the player"
    );
    assert!(ui.app.state.borrow().player.previewing);
    // A double-click on a track that already plays keeps it playing.
    ui.double_click_el(&row(&ui, "Charlie Tune"));
    ui.pump(200);
    assert!(ui.app.state.borrow().player.previewing, "never pauses");
}

#[test]
fn arrows_from_the_search_and_return_load_to_the_free_deck_in_perform() {
    let ui = Ui::open((1440, 900));
    ui.app.ui.invoke_set_mode("perform".into());
    ui.pump(300);
    ui.click_el(&row(&ui, "Alpha Tune"));
    ui.type_text("tune");
    ui.pump(100);
    assert_eq!(titles(&ui).len(), 3);
    // Down lands on the top hit, the next Down on the second.
    ui.key(Key::DownArrow);
    ui.key(Key::DownArrow);
    ui.pump(100);
    let second = titles(&ui)[1].clone();
    let id = ui.fx.tracks.iter().find(|t| t.1 == second).unwrap().0;
    ui.key(Key::Return);
    assert!(
        ui.wait_until(20.0, |u| u.app.decks.borrow().loaded_id(0) == Some(id)),
        "Return loads the row moved to on the free deck"
    );
    // The deck keys work again after the load.
    assert_ne!(ui.value("browser.list"), "focused");
}

#[test]
fn active_filters_show_as_removable_chips() {
    let ui = Ui::open((1440, 900));
    assert!(!ui.has("browser.filter.chips"));
    ui.app.ui.invoke_filter_edit("bpm-min".into(), "200".into());
    ui.app.ui.invoke_filter_edit("rating".into(), "2".into());
    ui.pump(100);
    assert!(ui.has("browser.filter.chips"), "visible with the bar closed");
    assert_eq!(ui.value("browser.filter.chip0"), "BPM ≥ 200");
    assert_eq!(titles(&ui).len(), 0);
    ui.click("browser.filter.chip0");
    ui.pump(100);
    assert_eq!(ui.value("browser.filter.chip0"), "RATING ≥ 2", "only that condition went");
    ui.click("browser.filter.clear-all");
    ui.pump(100);
    assert!(!ui.has("browser.filter.chips"));
    assert_eq!(titles(&ui).len(), 3);
}

#[test]
fn the_lists_column_folds_to_its_rail() {
    let ui = Ui::open((1440, 900));
    let wide = Rect::of(&ui.get("browser.list")).w;
    ui.click("browser.lists.toggle");
    ui.pump(200);
    assert!(Rect::of(&ui.get("browser.lists")).w < 100.0, "only the rail stays");
    assert!(Rect::of(&ui.get("browser.list")).w > wide + 150.0, "the list takes the room");
    ui.click("browser.lists.toggle");
    ui.pump(200);
    assert!((Rect::of(&ui.get("browser.list")).w - wide).abs() < 1.0, "the key at the rail's foot unfolds");
    // Folded again, a source of the rail unfolds the lists too.
    ui.click("browser.lists.toggle");
    ui.pump(200);
    ui.click("browser.rail.playlists");
    ui.pump(200);
    assert!((Rect::of(&ui.get("browser.list")).w - wide).abs() < 1.0, "a source opens its lists");
}

/// A narrow window shows signs on both rails, never cut words; the word
/// stays the accessible description and the tooltip.
#[test]
fn narrow_rails_show_signs_not_cut_words() {
    let ui = Ui::open((1040, 800));
    for label in ["browser.rail.similar", "browser.rail.sampler", "browser.rail.histories", "browser.panel.sub-browser", "browser.panel.similar"] {
        let e = ui.get(label);
        assert!(
            e.query_descendants().match_type_name("Glyph").find_first().is_some(),
            "{label} shows its sign"
        );
        assert!(!e.accessible_description().unwrap_or_default().is_empty(), "{label} keeps its word");
    }
    ui.resize((1600, 900));
    let e = ui.get("browser.rail.similar");
    assert!(e.query_descendants().match_type_name("Glyph").find_first().is_none(), "full words where there is room");
}

#[test]
fn preferences_preview_follows_the_settings() {
    let ui = Ui::open((1440, 900));
    let w = prefs(&ui, "view");
    let wave = || prefs_el(&w, "prefs.view.preview.wave").accessible_value().unwrap_or_default();
    assert_eq!(wave(), "standard");
    let before = w.get_preview_wave();
    prefs_el(&w, "prefs.view.calm-waveform").mock_single_click(slint::platform::PointerEventButton::Left);
    ui.pump(200);
    assert_eq!(wave(), "calm", "calm waveform at once");
    assert!(w.get_preview_wave() != before, "drawn again with the softer colours");

    let colour = || w.global::<crate::Theme>().invoke_deck_color(1);
    let plain = colour();
    prefs_el(&w, "prefs.view.colour-blind").mock_single_click(slint::platform::PointerEventButton::Left);
    ui.pump(200);
    assert_ne!(colour(), plain, "the deck colours follow the palette");

    w.invoke_choose("view.deck-layout".into(), 0);
    ui.pump(200);
    assert_eq!(prefs_el(&w, "prefs.view.preview.layout").accessible_value().unwrap_or_default(), "simple");
    assert_eq!(ui.app.state.borrow().settings.perform.layout, "simple", "and the decks follow");
    w.invoke_choose("view.scale".into(), 3);
    ui.pump(200);
    let scale = w.get_preview_scale();
    assert!((scale - ui.app.state.borrow().settings.view.scale).abs() < 1e-3 && scale > 1.0, "{scale}");
}

/// The USB screen: each stick with its format badges, its sync state in
/// words and its actions.
#[test]
fn stick_screen_shows_badges_state_and_actions() {
    let _volumes = volumes_lock();
    let ui = Ui::open((1440, 900));
    let name = "CRAFT-STICK";
    let stick = test_env().join("volumes").join(name);
    let _ = std::fs::remove_dir_all(&stick);
    std::fs::create_dir_all(&stick).unwrap();
    let uuid = format!("uuid-{name}-{}", std::process::id());
    let marker = od_format_native::DeviceMarker {
        format_version: od_format_native::FORMAT_VERSION,
        device_uuid: uuid.clone(),
        name: name.into(),
        created_by: "test".into(),
    };
    od_format_native::write_marker(&stick, &marker).unwrap();
    let id = ui
        .fx
        .lib()
        .register_device(&uuid, name, od_library::DeviceFormats { club_usb: true, opendjdecks: true, ..Default::default() })
        .unwrap();
    ui.app.device_manager.send(od_devices::Command::Refresh);
    assert!(ui.wait_until(20.0, |u| u.app.state.borrow().devices.statuses.iter().any(|s| s.volume.is_some())));
    ui.app.ui.invoke_select_section("devices".into());
    ui.pump(200);
    let n = id.0;
    assert!(ui.wait_until(10.0, |u| u.has(&format!("devices.stick{n}"))));
    assert_eq!(ui.value(&format!("devices.badge.club{n}")), "CLUB USB");
    assert_eq!(ui.value(&format!("devices.badge.native{n}")), "OPENDJDECKS");
    let state = |u: &Ui| u.value(&format!("devices.state{n}"));
    assert!(
        ui.wait_until(30.0, |u| ["IN SYNC", "CHANGES WAITING"].contains(&state(u).as_str()) || state(u).starts_with("SYNCING")),
        "{}",
        state(&ui)
    );
    for k in ["sync", "check", "eject"] {
        assert!(ui.wait_until(30.0, |u| u.has(&format!("devices.{k}{n}"))), "{k} key");
    }
    ui.click(&format!("devices.check{n}"));
    assert!(ui.wait_until(30.0, |u| u.has("devices.check-panel")), "CHECK opens the stick check");
    let _ = std::fs::remove_dir_all(&stick);
}
