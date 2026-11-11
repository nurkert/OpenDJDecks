//! Documentation screenshots for roadmap block F (`OPENDJDECKS_DEMO_PROOF`,
//! comma separated). Everything is staged in memory; only `dj-recording`
//! records for real, into the recordings folder of the library in use.
//!
//! - `dj-recording`: Prepare with the full browser on the recordings page
//!   while recording (a marker set, the voice track on), the decks playing
//!   from `OPENDJDECKS_AUTOLOAD`.
//! - `dj-tracklist`: the tracklist editor on a made-up mix of the library's
//!   tracks with a marker.
//! - `dj-emergency`: the USB screen with an emergency stick ready.
//! - `dj-queue`: Perform with the queue open: tracks, a matched and an open
//!   request, a do-not-play track, auto-play mixing.
//! - `dj-dead-air`: Perform with the dead-air warning over the list.
//! - `dj-bulk`: Prepare with three tracks selected and the info panel.

use crate::app::App;
use crate::models::model;
use od_core::TrackId;
use slint::ComponentHandle;
use std::rc::Rc;
use std::time::Duration;

pub(crate) fn run(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let parts: Vec<String> = v.split(',').map(str::to_string).collect();
    if !parts.iter().any(|p| p.starts_with("dj-")) {
        return;
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(Duration::from_secs(4), move || {
        let Some(a) = weak.upgrade() else { return };
        for p in &parts {
            match p.as_str() {
                "dj-recording" => recording(&a),
                "dj-tracklist" => tracklist(&a),
                "dj-emergency" => emergency(&a),
                "dj-queue" => queue(&a),
                "dj-dead-air" => dead_air(&a),
                "dj-bulk" => bulk(&a),
                _ => {}
            }
        }
    });
}

/// The library's first tracks (title, artist).
fn tracks(app: &App, n: usize) -> Vec<(TrackId, String, String)> {
    let st = app.state.borrow();
    let mut v: Vec<_> = st
        .lib
        .tracks(od_library::SortKey::default(), false)
        .unwrap_or_default()
        .into_iter()
        .map(|t| (t.id, t.tags.title, t.tags.artist))
        .collect();
    v.sort_by_key(|t| t.0.0);
    v.truncate(n);
    v
}

fn browser_page(app: &App, section: &str) {
    app.ui.invoke_set_mode("prepare".into());
    app.ui.invoke_menu_action("view.layout.browser".into());
    app.ui.invoke_select_section(section.into());
}

fn recording(app: &Rc<App>) {
    app.state.borrow_mut().settings.recording.voice = true;
    browser_page(app, "recordings");
    if !app.ui.get_recording() {
        app.ui.invoke_toggle_recording();
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(Duration::from_secs(8), move || {
        if let Some(a) = weak.upgrade() {
            a.ui.global::<crate::Rec>().invoke_mark();
        }
    });
}

fn tracklist(app: &Rc<App>) {
    let t = tracks(app, 5);
    let mut list: Vec<od_recorder::TracklistEntry> = t
        .iter()
        .enumerate()
        .map(|(i, (id, title, artist))| od_recorder::TracklistEntry {
            offset: i as f64 * 318.0 + if i > 0 { 12.0 } else { 0.0 },
            track: Some(*id),
            title: title.clone(),
            artist: artist.clone(),
            marker: false,
        })
        .collect();
    list.insert(
        3,
        od_recorder::TracklistEntry {
            offset: 905.0,
            track: None,
            title: "Shout-out to the crowd".into(),
            artist: String::new(),
            marker: true,
        },
    );
    let dir = app.state.borrow().settings.recordings_dir();
    crate::recorder::demo_edit(app, dir.join("Mix 2026-11-07 21-30-05.wav"), 1602.0, list);
    browser_page(app, "recordings");
}

fn emergency(app: &Rc<App>) {
    use crate::emergency::{Emergency, Stage};
    app.state.borrow_mut().devices.emergency = Some(Emergency {
        mount: std::path::PathBuf::from("/Volumes/SPARE"),
        stick: "SPARE".into(),
        device: None,
        node: od_library::NodeId(0),
        tracks: 12,
        stage: Stage::Done,
        progress: 1.0,
        detail: od_i18n::trf("{} tracks, ready to play", &[&12]),
        formats: od_library::DeviceFormats { club_usb: true, opendjdecks: true, club_usb_compat: true },
    });
    crate::emergency::project(app);
    browser_page(app, "devices");
}

fn queue(app: &Rc<App>) {
    use crate::queue::{Item, STATE};
    let t = tracks(app, 10);
    if t.len() < 6 {
        return;
    }
    STATE.with(|s| {
        s.borrow_mut().items = vec![
            Item { track: Some(t[4].0), request: String::new() },
            Item { track: Some(t[5].0), request: format!("{} – {}", t[5].2, t[5].1) },
            Item { track: Some(t[2].0), request: String::new() },
            Item { track: None, request: "The one with the long vocal from last summer".into() },
            Item { track: Some(t[9.min(t.len() - 1)].0), request: String::new() },
        ];
    });
    crate::queue::project(app);
    // The last one tagged do not play, in the picture only.
    let g = app.ui.global::<crate::Queue>();
    let mut rows: Vec<crate::QueueRow> = slint::Model::iter(&g.get_rows()).collect();
    if let Some(r) = rows.last_mut() {
        r.dnp = true;
    }
    g.set_rows(model(rows));
    g.set_auto(true);
    g.set_mixing(od_i18n::trf("MIXING DECK {} › DECK {} · {} BEATS", &[&1, &2, &16]).into());
    g.set_can_load(true);
    crate::queue::HOLD_ALERT.with(|h| h.set(true));
    app.ui.invoke_set_mode("perform".into());
    // The browse layout: lanes over the list, so the queue has the height.
    app.ui.global::<crate::PerformLayout>().set_layout("browse".into());
    app.ui.invoke_select_section("library".into());
    app.ui.set_queue_open(true);
}

fn dead_air(app: &Rc<App>) {
    crate::queue::HOLD_ALERT.with(|h| h.set(true));
    let g = app.ui.global::<crate::Queue>();
    g.set_alert("dead-air".into());
    g.set_alert_title(od_i18n::trf("Dead air in {}", &[&"0:24"]).into());
    g.set_alert_detail(
        od_i18n::trf("Deck {} runs out and no deck is cued. LOAD NEXT takes the queue's next track.", &[&1]).into(),
    );
    g.set_can_load(true);
    app.ui.invoke_set_mode("perform".into());
    app.ui.invoke_select_section("library".into());
}

fn bulk(app: &Rc<App>) {
    browser_page(app, "library");
    app.ui.invoke_menu_action("view.layout.player".into());
    app.ui.invoke_select_node(-1);
    app.ui.set_info_open(true);
    app.ui.invoke_row_clicked(1, false, false);
    app.ui.invoke_row_clicked(3, true, false);
    app.ui.invoke_row_clicked(5, true, false);
}
