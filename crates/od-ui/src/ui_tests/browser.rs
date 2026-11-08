//! Behaviour tests of the browser features of `tests/parity/features.toml`:
//! column choice / order / secondary sort, the quick filter, Tags, the tag
//! list, similar tracks, histories, drag and drop into the tree and row /
//! node reordering. Each drives the real window and asserts the library,
//! the settings or the list on screen.

use super::harness::*;
use od_library::NodeKind;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model};

/// Titles of the list on screen, in order.
fn titles(ui: &Ui) -> Vec<String> {
    ui.app
        .ui
        .get_tracks()
        .iter()
        .map(|r| r.title.to_string())
        .collect()
}

fn row(ui: &Ui, title: &str) -> i_slint_backend_testing::ElementHandle {
    ui.by_value("browser.row", title)
        .unwrap_or_else(|| panic!("row `{title}`"))
        .1
}

fn stored(ui: &Ui) -> crate::settings::Settings {
    crate::settings::Settings::from_json(
        &ui.fx
            .lib()
            .meta(crate::settings::META_SETTINGS)
            .unwrap()
            .unwrap(),
    )
}

/// Holds ⇧ while running `f` (modifiers come from key events).
fn with_shift(ui: &Ui, f: impl FnOnce()) {
    let shift: slint::SharedString = Key::Shift.into();
    ui.app.ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: shift.clone(),
    });
    f();
    ui.app
        .ui
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text: shift });
}

/// Header right-click opens the column menu; ticking ALBUM shows the
/// column and stores the choice; Default columns resets it.
#[test]
fn column_menu_chooses_columns() {
    let ui = Ui::open((1440, 900));
    assert!(!ui.has("browser.header.album"));
    ui.right_click_el(&ui.get("browser.header.title"));
    assert!(ui.click_menu_item("ALBUM"), "column menu lists ALBUM");
    assert!(
        ui.wait_until(5.0, |u| u.has("browser.header.album")),
        "album column shown"
    );
    assert!(
        stored(&ui).browser.columns.contains(&"album".to_string()),
        "choice stored in the settings"
    );
    // The columns button opens the same menu: hide GENRE.
    ui.click("browser.header.columns");
    assert!(ui.click_menu_item("GENRE"));
    assert!(
        ui.wait_until(5.0, |u| !u.has("browser.header.genre")),
        "genre column hidden"
    );
    ui.click("browser.header.columns");
    assert!(ui.click_menu_item("Default columns"));
    assert!(ui.wait_until(5.0, |u| u.has("browser.header.genre")
        && !u.has("browser.header.album")));
    assert!(
        stored(&ui).browser.columns.is_empty(),
        "defaults stored as the empty list"
    );
}

/// Dragging a header cell onto another moves the column there (stored);
/// ⇧-click adds a secondary sort (stored) and a plain click clears it.
#[test]
fn column_drag_and_secondary_sort() {
    let ui = Ui::open((1440, 900));
    let key = Rect::of(&ui.get("browser.header.key"));
    let artist = Rect::of(&ui.get("browser.header.artist"));
    assert!(key.x > artist.x);
    ui.drag_points(
        key.centre(),
        slint::LogicalPosition::new(artist.x + 4.0, artist.centre().y),
    );
    assert!(
        ui.wait_until(5.0, |u| Rect::of(&u.get("browser.header.key")).x
            < Rect::of(&u.get("browser.header.artist")).x),
        "key moved before artist"
    );
    let cols = stored(&ui).browser.columns;
    let pos = |c: &str| cols.iter().position(|x| x == c).unwrap();
    assert!(pos("key") < pos("artist"), "order stored: {cols:?}");
    // Primary: title (click); secondary: rating (⇧-click).
    ui.click("browser.header.title");
    with_shift(&ui, || ui.click("browser.header.rating"));
    let then = stored(&ui).browser.sort_then;
    assert_eq!(then.len(), 1, "secondary sort stored: {then:?}");
    assert_eq!(then[0].column, "rating");
    let ranks: Vec<(String, i32)> = ui
        .app
        .ui
        .get_columns()
        .iter()
        .map(|c| (c.id.to_string(), c.rank))
        .collect();
    assert!(
        ranks.contains(&("title".into(), 1)) && ranks.contains(&("rating".into(), 2)),
        "{ranks:?}"
    );
    ui.click("browser.header.artist");
    assert!(
        stored(&ui).browser.sort_then.is_empty(),
        "a plain click sorts by one column again"
    );
}

/// Filter bar: minimum rating and a BPM range narrow the list; CLEAR
/// brings every track back.
#[test]
fn filter_bar_narrows_the_list() {
    let fx = Fixture::new();
    {
        let mut lib = fx.lib();
        let mut tags = lib.track(fx.tracks[1].0).unwrap().tags;
        tags.rating = 4;
        lib.set_tags(fx.tracks[1].0, tags).unwrap();
    }
    let ui = Ui::start(fx, (1440, 900), false);
    assert_eq!(titles(&ui).len(), 3);
    ui.click("browser.header.filter");
    assert!(ui.has("browser.filter.rating3"), "filter bar open");
    ui.click("browser.filter.rating3");
    assert!(
        ui.wait_until(5.0, |u| titles(u) == [u.fx.tracks[1].1.clone()]),
        "only the 4-star track: {:?}",
        titles(&ui)
    );
    ui.click("browser.filter.clear");
    assert_eq!(titles(&ui).len(), 3);
    ui.click("browser.filter.bpm-min");
    ui.type_text("300");
    assert!(
        ui.wait_until(5.0, |u| titles(u).is_empty()),
        "no track at 300 BPM or more"
    );
    ui.click("browser.filter.clear");
    assert!(ui.wait_until(5.0, |u| titles(u).len() == 3));
    // The filter button closes the bar again.
    ui.click("browser.header.filter");
    assert!(!ui.has("browser.filter.rating3"));
}

/// Tags panel: typing "Mood: Peak" tags the selected track (undoable,
/// shown in the TAGS column and offered by the filter); clicking the value
/// untags it.
#[test]
fn user_tag_panel_tags_tracks() {
    let ui = Ui::open((1440, 900));
    let (id, title) = ui.fx.tracks[0].clone();
    ui.click_el(&row(&ui, &title));
    ui.click("browser.panel.user-tag");
    assert!(
        ui.wait_until(5.0, |u| u.has("user_tag.add")),
        "Tags panel open"
    );
    ui.click("user_tag.add");
    ui.type_text("Mood: Peak");
    ui.key(Key::Return);
    let peak = od_library::UserTag::new("Mood", "Peak");
    assert!(
        ui.wait_until(5.0, |u| u.fx.lib().user_tags(id).unwrap() == [peak.clone()]),
        "tag stored"
    );
    let shown = ui
        .app
        .ui
        .get_tracks()
        .iter()
        .find(|r| r.title == title.as_str())
        .map(|r| r.user_tag.to_string());
    assert_eq!(shown.as_deref(), Some("Peak"), "TAGS cell value");
    assert!(
        ui.app
            .ui
            .get_user_tag_options()
            .iter()
            .any(|o| o == "Mood: Peak"),
        "offered by the filter"
    );
    assert!(
        ui.app
            .ui
            .get_user_tag_chips()
            .iter()
            .any(|c| c.value == "Peak" && c.on)
    );
    ui.click("user_tag.value0");
    assert!(
        ui.wait_until(5.0, |u| u.fx.lib().user_tags(id).unwrap().is_empty()),
        "untagged"
    );
}

/// Tag list: + collects the selected track, the list shows it, Delete
/// removes it again, SAVE AS PLAYLIST makes a playlist of it.
#[test]
fn tag_list_collects_and_saves() {
    let ui = Ui::open((1440, 900));
    let (a, ta) = ui.fx.tracks[0].clone();
    let (b, tb) = ui.fx.tracks[2].clone();
    ui.click_el(&row(&ui, &ta));
    let add = ui
        .scroll_into_view("browser.tree", |u| u.find("browser.tree.tag-list.add"))
        .expect("tag list + in the tree");
    ui.click_el(&add);
    ui.right_click_el(&row(&ui, &tb));
    assert!(ui.click_menu_item("Add to tag list"));
    assert_eq!(ui.app.ui.get_tag_list_count(), 2);
    let list = ui
        .scroll_into_view("browser.tree", |u| u.find("browser.tree.tag-list"))
        .expect("tag list row in the tree");
    ui.click_el(&list);
    assert!(
        ui.wait_until(5.0, |u| titles(u) == [ta.clone(), tb.clone()]),
        "tag list in order of adding: {:?}",
        titles(&ui)
    );
    assert_eq!(ui.app.ui.get_node_kind(), "taglist");
    ui.click_el(&row(&ui, &tb));
    ui.key(Key::Delete);
    assert!(
        ui.wait_until(5.0, |u| titles(u) == [ta.clone()]),
        "removed from the tag list"
    );
    ui.click("browser.tag-list.save");
    let lib = ui.fx.lib();
    let saved = lib
        .all_nodes()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == NodeKind::Playlist && n.name == "Tag list")
        .expect("playlist created");
    assert_eq!(
        lib.entries(saved.id)
            .unwrap()
            .iter()
            .map(|e| e.track)
            .collect::<Vec<_>>(),
        [a]
    );
    let _ = b;
    // The tag list persists in the library.
    assert_eq!(crate::browser::load_tag_list(&lib), [a]);
}

/// Similar tracks: the list follows the selected track (no deck loaded)
/// and shows only tracks with a compatible key; the Similar panel lists
/// the same suggestions.
#[test]
fn similar_tracks_follow_the_reference() {
    let fx = Fixture::new();
    {
        let mut lib = fx.lib();
        for ((id, _), key) in fx.tracks.iter().zip(["8A", "9A", "3B"]) {
            lib.set_key(*id, od_core::MusicalKey::parse(key)).unwrap();
            lib.conn()
                .execute("UPDATE track SET bpm = 120.0 WHERE id = ?1", [id.0])
                .unwrap();
        }
    }
    let ui = Ui::start(fx, (1440, 900), false);
    let (a, b) = (ui.fx.tracks[0].1.clone(), ui.fx.tracks[1].1.clone());
    ui.click_el(&row(&ui, &a));
    ui.click("browser.panel.similar");
    assert!(
        ui.wait_until(5.0, |u| u
            .app
            .ui
            .get_suggestions()
            .iter()
            .any(|s| s.title == b.as_str())),
        "panel suggests {b}"
    );
    assert!(
        ui.app
            .ui
            .get_suggestions()
            .iter()
            .all(|s| s.title != ui.fx.tracks[2].1.as_str()),
        "3B does not fit after 8A"
    );
    ui.click("similar.open-list");
    assert_eq!(ui.app.ui.get_node_kind(), "similar");
    assert!(
        ui.wait_until(5.0, |u| titles(u) == [b.clone()]),
        "Similar list: {:?}",
        titles(&ui)
    );
    assert!(
        ui.has("browser.similar.master") && ui.has("browser.similar.deck4"),
        "deck source selector"
    );
    // The tree row opens the same view.
    ui.app.ui.invoke_select_node(-1);
    ui.pump(100);
    assert_eq!(ui.app.ui.get_node_kind(), "all");
    ui.click("browser.rail.similar");
    assert_eq!(ui.app.ui.get_node_kind(), "similar");
}

/// Histories: every session is a node; opening it lists the plays in order.
#[test]
fn histories_list_play_sessions() {
    let fx = Fixture::new();
    {
        let mut lib = fx.lib();
        lib.record_play(fx.tracks[2].0, "Session 2026-11-01 22:00", "opendjdecks")
            .unwrap();
        lib.record_play(fx.tracks[0].0, "Session 2026-11-01 22:00", "opendjdecks")
            .unwrap();
    }
    let ui = Ui::start(fx, (1440, 900), false);
    assert_eq!(
        ui.value("browser.tree.history0"),
        "Session 2026-11-01 22:00"
    );
    ui.click("browser.tree.history0");
    let want = [ui.fx.tracks[2].1.clone(), ui.fx.tracks[0].1.clone()];
    assert!(
        ui.wait_until(5.0, |u| titles(u) == want),
        "plays in order: {:?}",
        titles(&ui)
    );
    assert_eq!(ui.app.ui.get_node_kind(), "history");
}

/// Dragging a row onto a playlist in the tree adds it; inside the playlist
/// dragging a row onto another reorders the entries (one undo step).
#[test]
fn drag_rows_to_tree_and_reorder() {
    let ui = Ui::open((1440, 900));
    let (pl, (b, tb)) = (ui.fx.playlist, ui.fx.tracks[1].clone());
    let a = ui.fx.tracks[0].0;
    let target = Rect::of(&ui.get(&format!("browser.tree.node{}", pl.0))).centre();
    ui.drag_el(&row(&ui, &tb), target);
    assert!(
        ui.wait_until(5.0, |u| u.fx.lib().entries(pl).unwrap().len() == 2),
        "dropped track added"
    );
    assert!(!ui.app.ui.global::<crate::DragState>().get_active());
    ui.click(&format!("browser.tree.node{}", pl.0));
    assert!(ui.wait_until(5.0, |u| titles(u).len() == 2));
    assert!(
        ui.app.ui.get_reorderable(),
        "playlist in its own order can be reordered"
    );
    let first = Rect::of(&row(&ui, &ui.fx.tracks[0].1));
    ui.drag_el(
        &row(&ui, &tb),
        slint::LogicalPosition::new(first.centre().x, first.y + 3.0),
    );
    let order = |u: &Ui| {
        u.fx.lib()
            .entries(pl)
            .unwrap()
            .iter()
            .map(|e| e.track)
            .collect::<Vec<_>>()
    };
    assert!(
        ui.wait_until(5.0, |u| order(u) == [b, a]),
        "reordered: {:?}",
        order(&ui)
    );
    ui.app.undo();
    assert_eq!(order(&ui), [a, b], "undo restores the order");
}

trait FolderFixture {
    fn lib_mut_create_folder(&self) -> od_library::NodeId;
}

impl FolderFixture for Fixture {
    fn lib_mut_create_folder(&self) -> od_library::NodeId {
        let mut lib = self.lib();
        lib.create_node(None, NodeKind::Folder, "Crates").unwrap()
    }
}

/// Dragging a tree node onto a folder moves it into the folder.
#[test]
fn drag_tree_node_into_folder() {
    let fx = Fixture::new();
    let folder = fx.lib_mut_create_folder();
    let ui = Ui::start(fx, (1440, 900), false);
    let pl = ui.fx.playlist;
    let to = Rect::of(&ui.get(&format!("browser.tree.node{}", folder.0))).centre();
    ui.drag_el(&ui.get(&format!("browser.tree.node{}", pl.0)), to);
    assert!(
        ui.wait_until(5.0, |u| u.fx.lib().node(pl).unwrap().parent == Some(folder)),
        "playlist moved into the folder"
    );
}

/// PREVIEW and ARTWORK columns: every analysed row shows its waveform
/// thumbnail; a track with cover art shows it (decoded off the UI thread),
/// the others stay empty.
#[test]
fn preview_and_artwork_columns() {
    let fx = Fixture::new();
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        64,
        64,
        image::Rgba([200, 40, 40, 255]),
    ))
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .unwrap();
    let covered = fx.tracks[1].0;
    assert!(
        fx.lib()
            .set_artwork_if_missing(covered, &png, "image/png")
            .unwrap()
    );
    let ui = Ui::start(fx, (1440, 900), false);
    assert!(
        ui.has("browser.header.artwork"),
        "ARTWORK column shown by default"
    );
    let shown = |u: &Ui| {
        u.app.ui.get_tracks().iter().all(|r| {
            r.analysed
                && r.preview.size().width > 0
                && (r.artwork.size().width > 0) == (r.id as i64 == covered.0)
        })
    };
    assert!(
        ui.wait_until(60.0, shown),
        "rows: {:?}",
        ui.app
            .ui
            .get_tracks()
            .iter()
            .map(|r| (
                r.title.to_string(),
                r.analysed,
                r.preview.size(),
                r.artwork.size()
            ))
            .collect::<Vec<_>>()
    );
}

/// Kiosk lists keep title and artist readable on the 800 px touch screen.
#[test]
fn kiosk_list_keeps_title_and_artist_readable() {
    let ui = Ui::start(Fixture::new(), (800, 480), true);
    ui.app.ui.set_browser_open(true);
    ui.pump(300);
    for (col, min) in [("title", 120.0), ("artist", 80.0)] {
        let w = Rect::of(&ui.get(&format!("browser.header.{col}"))).w;
        assert!(w >= min, "{col} column is {w} px wide");
    }
}

/// Tracks without a cover get one in the background from a cover image in
/// their folder; it is stored and shows in the list without a refresh.
#[test]
fn covers_are_filled_in_from_the_folder() {
    let fx = Fixture::new();
    let track = fx.lib().track(fx.tracks[0].0).unwrap();
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(32, 32, image::Rgba([10, 200, 40, 255])))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    std::fs::write(track.path.parent().unwrap().join("Cover.png"), &png).unwrap();
    let ui = Ui::start(fx, (1440, 900), false);
    let id = ui.fx.tracks[0].0;
    assert!(ui.wait_until(20.0, |u| u.fx.lib().artwork(id).unwrap().is_some()), "cover stored");
    assert!(
        // All fixture tracks share the folder, so all of them get it.
        ui.wait_until(30.0, |u| u.app.ui.get_tracks().iter().all(|r| r.artwork.size().width > 0)),
        "every track of the folder shows it"
    );
}

/// An Explorer folder follows the chosen columns, and files already in the
/// collection show their library data there (rating, genre, …).
#[test]
fn explorer_shows_chosen_columns_with_library_data() {
    let ui = Ui::open((1440, 900));
    let (id, title) = ui.fx.tracks[0].clone();
    let lib = ui.fx.lib();
    let mut tags = lib.track(id).unwrap().tags;
    tags.rating = 4;
    tags.genre = "Electro".into();
    let mut lib = ui.fx.lib();
    lib.set_tags(id, tags).unwrap();
    let folder = lib.track(id).unwrap().path.parent().unwrap().to_path_buf();
    // Choose RATING (it is not in the Explorer's reduced set).
    ui.right_click_el(&ui.get("browser.header.title"));
    assert!(ui.click_menu_item("ALBUM"));
    crate::explorer::open_folder(&ui.app, folder);
    assert!(
        ui.wait_until(10.0, |u| u.has("browser.header.album") && u.has("browser.header.rating") && u.has("browser.header.genre")),
        "the chosen columns show in the Explorer"
    );
    assert!(
        ui.wait_until(10.0, |u| u.app.ui.get_tracks().iter().any(|r| r.title == title && r.rating == 4 && r.genre == "Electro")),
        "library data of a file in the collection"
    );
}

/// Dragging a header cell's right edge resizes the column (stored), and a
/// double-click on the edge gives it back its default width.
#[test]
fn columns_resize_by_dragging_their_edge() {
    let ui = Ui::open((1440, 900));
    let bpm = Rect::of(&ui.get("browser.header.bpm"));
    let edge = slint::LogicalPosition::new(bpm.right() + 1.0, bpm.centre().y);
    ui.drag_points(edge, slint::LogicalPosition::new(edge.x + 60.0, edge.y));
    assert!(
        ui.wait_until(5.0, |u| (Rect::of(&u.get("browser.header.bpm")).w - (bpm.w + 60.0)).abs() < 2.0),
        "BPM is 60 px wider: {} → {}",
        bpm.w,
        Rect::of(&ui.get("browser.header.bpm")).w
    );
    let width = stored(&ui).browser.widths.get("bpm").copied().unwrap_or(0.0);
    assert!((width - (bpm.w + 60.0)).abs() < 2.0, "width stored: {width}");
    // A row cell follows the header.
    let title = Rect::of(&ui.get("browser.header.title"));
    assert!(title.w > 50.0);
    let now = Rect::of(&ui.get("browser.header.bpm"));
    ui.double_click_at(slint::LogicalPosition::new(now.right() + 1.0, now.centre().y));
    assert!(
        ui.wait_until(5.0, |u| (Rect::of(&u.get("browser.header.bpm")).w - bpm.w).abs() < 1.0),
        "double-click restores the default width"
    );
    assert!(!stored(&ui).browser.widths.contains_key("bpm"));
}

/// Dragging the edge of a shared-width column moves that edge only: the
/// columns to its left stay where they are.
#[test]
fn column_drag_moves_only_the_dragged_edge() {
    let ui = Ui::open((1440, 900));
    let title = Rect::of(&ui.get("browser.header.title"));
    let artist = Rect::of(&ui.get("browser.header.artist"));
    let edge = slint::LogicalPosition::new(artist.right() + 1.0, artist.centre().y);
    ui.drag_points(edge, slint::LogicalPosition::new(edge.x + 80.0, edge.y));
    assert!(
        ui.wait_until(5.0, |u| (Rect::of(&u.get("browser.header.artist")).right() - (artist.right() + 80.0)).abs() < 2.0),
        "the edge follows the pointer: {} → {}",
        artist.right(),
        Rect::of(&ui.get("browser.header.artist")).right()
    );
    let now = Rect::of(&ui.get("browser.header.title"));
    assert!((now.x - title.x).abs() < 1.0 && (now.w - title.w).abs() < 1.0, "TITLE stays: {title:?} → {now:?}");
    assert!((Rect::of(&ui.get("browser.header.artist")).x - artist.x).abs() < 1.0, "ARTIST's left edge stays");
}

/// A track on air past the mark gets the played tick in the list; the
/// others don't.
#[test]
fn played_tracks_are_ticked_in_the_list() {
    let fx = Fixture::new();
    let id = fx.tracks[0].0;
    let ui = Ui::start(fx, (1440, 900), false);
    assert!(!ui.has("browser.row.played"));
    ui.app.recorder.borrow_mut().played.insert(id);
    crate::browser::refresh_tracks(&ui.app);
    assert!(ui.wait_until(2.0, |u| u.has("browser.row.played")), "tick shown");
    let ticked: Vec<_> = ui.app.ui.get_tracks().iter().filter(|r| r.played).map(|r| r.id as i64).collect();
    assert_eq!(ticked, vec![id.0]);
}

/// AUTO SORT: the analysed fixture (120 BPM) lands in the 115–124 BPM
/// list; a click on the group opens it, a click on the list shows its
/// tracks in the table (sortable like any list).
#[test]
fn auto_sort_lists_open_in_the_table() {
    let ui = Ui::open((1440, 900));
    assert!(
        ui.wait_until(10.0, |u| u.app.ui.get_auto_sort().row_count() > 0),
        "AUTO SORT computed"
    );
    let group = ui
        .scroll_into_view("browser.tree", |u| {
            u.by_value("browser.tree.auto", "Tempo").map(|(_, e)| e)
        })
        .expect("Tempo group in the tree");
    ui.click_el(&group);
    let list = ui
        .scroll_into_view("browser.tree", |u| {
            u.by_value("browser.tree.auto", "115–124 BPM").map(|(_, e)| e)
        })
        .expect("tempo band list after opening the group");
    ui.click_el(&list);
    let mut want: Vec<String> = ui.fx.tracks.iter().map(|t| t.1.clone()).collect();
    want.sort();
    assert!(
        ui.wait_until(5.0, |u| {
            let mut t = titles(u);
            t.sort();
            t == want
        }),
        "the band holds the fixture: {:?}",
        titles(&ui)
    );
    assert_eq!(ui.app.ui.get_node_kind(), "auto");
    assert_eq!(ui.app.ui.get_node_title(), "115–124 BPM");
    // Sortable like any list.
    ui.click("browser.header.title");
    assert!(ui.wait_until(5.0, |u| titles(u) == want), "sorted by title");
    // Mood, genre and the other groups are there too (keys come from the analysis).
    let groups: Vec<String> = ui
        .app
        .ui
        .get_auto_sort()
        .iter()
        .filter(|r| r.kind == "folder")
        .map(|r| r.name.to_string())
        .collect();
    assert!(groups.contains(&"Energy".to_string()), "{groups:?}");
}

/// MATCH column (on by default): rows are rated against the selected track
/// when no deck plays — same tempo and an adjacent key fill all three
/// lines, a key clash one; sorting puts the best fits first.
#[test]
fn match_column_rates_against_the_reference() {
    let fx = Fixture::new();
    {
        let mut lib = fx.lib();
        for ((id, _), key) in fx.tracks.iter().zip(["8A", "9A", "3B"]) {
            lib.set_key(*id, od_core::MusicalKey::parse(key)).unwrap();
            lib.conn()
                .execute("UPDATE track SET bpm = 120.0 WHERE id = ?1", [id.0])
                .unwrap();
        }
    }
    let ui = Ui::start(fx, (1440, 900), false);
    assert!(ui.has("browser.header.match"), "MATCH is a default column");
    let (a, b, c) = (
        ui.fx.tracks[0].1.clone(),
        ui.fx.tracks[1].1.clone(),
        ui.fx.tracks[2].1.clone(),
    );
    ui.click_el(&row(&ui, &a));
    let score = |u: &Ui, t: &str| {
        u.app
            .ui
            .get_tracks()
            .iter()
            .find(|r| r.title == t)
            .map(|r| r.match_score)
    };
    assert!(
        ui.wait_until(5.0, |u| score(u, &b) == Some(3) && score(u, &c) == Some(1)),
        "8A → 9A fits, 3B clashes: {:?} {:?}",
        score(&ui, &b),
        score(&ui, &c)
    );
    let tip = ui
        .app
        .ui
        .get_tracks()
        .iter()
        .find(|r| r.title == c.as_str())
        .unwrap()
        .match_tip
        .to_string();
    assert!(tip.starts_with("vs SELECTED") && tip.contains("key clash"), "{tip}");
    let cell = ui.get("browser.row0.match");
    assert!(!cell.accessible_value().unwrap_or_default().is_empty());
    ui.click("browser.header.match");
    assert!(
        ui.wait_until(5.0, |u| titles(u).last() == Some(&c)),
        "best fits first, the clash last: {:?}",
        titles(&ui)
    );
}

/// Cues like my others: three tracks carry hot cue A ("IN", red) on their
/// first downbeat; the row menu puts the same cue on a fourth track (one
/// undo step).
#[test]
fn cues_like_my_others_from_the_row_menu() {
    let mut fx = Fixture::new();
    let delta = {
        let mut lib = fx.lib();
        let path = fx.root.join("music/track4.wav");
        write_wav(&path, 8.0, 550.0);
        let id = crate::workers::import_file(&mut lib, &path).unwrap();
        let mut tags = lib.track(id).unwrap().tags;
        tags.title = "Delta Tune".into();
        lib.set_tags(id, tags).unwrap();
        id
    };
    fx.tracks.push((delta, "Delta Tune".into()));
    let ui = Ui::start(fx, (1440, 900), false);
    let red = od_core::CueColor::rgb(230, 40, 40);
    let first_downbeat = |lib: &od_library::Library, id: od_core::TrackId| {
        lib.beatgrid(id)
            .unwrap()
            .expect("analysed")
            .beats
            .iter()
            .find(|b| b.bar_position == 1)
            .map(|b| b.time)
            .unwrap()
    };
    {
        let mut lib = ui.fx.lib();
        for (id, _) in &ui.fx.tracks[..3] {
            let t = first_downbeat(&lib, *id);
            let mut cue = od_core::Cue::hot(0, t);
            cue.label = "IN".into();
            cue.color = Some(red);
            lib.set_cues(*id, vec![cue]).unwrap();
        }
    }
    ui.pump(500);
    ui.right_click_el(&row(&ui, "Delta Tune"));
    assert!(ui.click_menu_item("Set cues like my others"));
    let lib = ui.fx.lib();
    let cues = lib.cues(delta).unwrap();
    assert_eq!(cues.len(), 1, "{cues:?}");
    assert_eq!(cues[0].kind, od_core::CueKind::Hot(0));
    assert!((cues[0].start - first_downbeat(&lib, delta)).abs() < 1e-6);
    assert_eq!((cues[0].label.as_str(), cues[0].color), ("IN", Some(red)));
    ui.app.undo();
    assert!(ui.fx.lib().cues(delta).unwrap().is_empty(), "one undo step");
}

/// With a moved file (n MOVED in the list header) and the info
/// panel open, the laptop window still holds every panel.
#[test]
fn info_panel_fits_with_moved_files_on_a_laptop() {
    let fx = Fixture::new();
    let id = fx.tracks[0].0;
    fx.lib().set_track_path(id, std::path::Path::new("/gone/away/track.wav")).unwrap();
    fx.lib().set_missing(id, true).unwrap();
    let ui = Ui::start(fx, (1280, 800), false);
    let w = &ui.app.ui;
    w.invoke_select_node(-1);
    ui.pump(100);
    w.invoke_row_clicked(0, false, false);
    w.set_info_open(true);
    ui.pump(200);
    assert!(ui.wait_until(5.0, |u| u.has("browser.find-moved")), "moved files offered");
    let r = Rect::of(&ui.get("info.panel"));
    assert!(r.right() <= 1280.0 + 0.5, "info panel inside the window: {r:?}");
}

/// A very long file path (deep Windows profile folders) wraps inside the
/// info panel instead of pushing it past the window.
#[test]
fn info_panel_keeps_long_paths_inside_the_window() {
    let fx = Fixture::new();
    let id = fx.tracks[0].0;
    // A real file deep down a long folder chain.
    let old = fx.lib().track(id).unwrap().path;
    let mut dir = old.parent().unwrap().to_path_buf();
    for _ in 0..6 {
        dir.push("AVeryLongFolderNameWithoutAnySpacesInIt");
    }
    std::fs::create_dir_all(&dir).unwrap();
    let long = dir.join("track-with-a-rather-long-file-name-without-any-spaces.wav");
    std::fs::copy(&old, &long).unwrap();
    fx.lib().set_track_path(id, &long).unwrap();
    let ui = Ui::start(fx, (1280, 800), false);
    let w = &ui.app.ui;
    w.invoke_select_node(-1);
    ui.pump(100);
    let row = ui.app.ui.get_tracks().iter().position(|r| r.id as i64 == id.0).unwrap();
    w.invoke_row_clicked(row as i32, false, false);
    w.set_info_open(true);
    ui.pump(200);
    ui.click("info.tab.info");
    assert!(ui.wait_until(2.0, |u| u.has("info.path")));
    for f in ["info.time", "info.path"] {
        let r = Rect::of(&ui.get(f));
        assert!(r.right() <= 1280.0 + 0.5, "{f} inside the window: {r:?}");
    }
}
