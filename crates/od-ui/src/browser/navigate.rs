//! Browsing from a controller (SPEC §9): the browse encoder moves through
//! everything in the lists column, top to bottom as it is drawn (the
//! collection, playlists, the Explorer's folders, similar tracks, sticks,
//! histories, recordings, the tag list, created tracks and AUTO SORT) or,
//! once a list is entered, through its tracks. A press or a deck's LOAD in
//! the column goes one level in (opens a folder and steps to its first
//! entry, or enters the track list); in the track list a press loads the
//! track. BACK goes one level out every time: from the tracks to their list,
//! from an entry to the folder holding it, and an open folder closes. VIEW
//! enlarges the browser and SIMILAR opens the similar tracks. The part with
//! focus is marked in the window (`browse-focus`).

use crate::app::App;
use crate::decks;
use od_controller::BrowseAction;
use slint::{Model, SharedString};

/// One entry of the lists column, as the encoder steps through it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Entry {
    /// A library node (the collection, lists, folders, specials, sessions,
    /// stick lists, AUTO SORT groups).
    Node {
        id: i32,
        name: String,
        depth: i32,
        has_children: bool,
        expanded: bool,
    },
    /// A folder of the Explorer.
    Folder {
        id: i32,
        name: String,
        depth: i32,
        has_children: bool,
        expanded: bool,
    },
    /// A registered stick.
    Device { id: i32, name: String },
    /// The recordings.
    Recordings,
}

impl Entry {
    fn depth(&self) -> i32 {
        match self {
            Entry::Node { depth, .. } | Entry::Folder { depth, .. } => *depth,
            _ => 0,
        }
    }

    /// An entry with entries below it, and whether they are shown.
    fn folder(&self) -> Option<bool> {
        match self {
            Entry::Node {
                has_children: true,
                expanded,
                ..
            }
            | Entry::Folder {
                has_children: true,
                expanded,
                ..
            } => Some(*expanded),
            _ => None,
        }
    }

    fn name(&self) -> String {
        match self {
            Entry::Node { name, .. } | Entry::Folder { name, .. } | Entry::Device { name, .. } => {
                name.clone()
            }
            Entry::Recordings => od_i18n::tr("Recordings").to_string(),
        }
    }

    /// Same kind of entry (a node's parent is a node, a folder's a folder).
    fn same_kind(&self, other: &Entry) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

fn node(r: &crate::TreeRow) -> Entry {
    Entry::Node {
        id: r.id,
        name: r.name.to_string(),
        depth: r.depth,
        has_children: r.has_children,
        expanded: r.expanded,
    }
}

fn special(id: i32, name: &str) -> Entry {
    Entry::Node {
        id,
        name: od_i18n::tr(name).to_string(),
        depth: 0,
        has_children: false,
        expanded: false,
    }
}

/// Everything in the lists column, in the order it is drawn
/// (`ui/sidebar.slint`).
pub(crate) fn entries(app: &App) -> Vec<Entry> {
    let tree: Vec<crate::TreeRow> = app.ui.get_tree().iter().collect();
    let stick = |r: &crate::TreeRow| r.kind == "stick" || r.id <= -1_000_000;
    let mut out: Vec<Entry> = tree.iter().filter(|r| r.kind == "all").map(node).collect();
    out.extend(
        tree.iter()
            .filter(|r| r.kind != "all" && !stick(r) && r.id > -100)
            .map(node),
    );
    out.extend(app.ui.get_explorer().iter().map(|f| Entry::Folder {
        id: f.id,
        name: f.name.to_string(),
        depth: f.depth,
        has_children: f.has_children,
        expanded: f.expanded,
    }));
    out.push(special(super::sources::SIMILAR, "Similar tracks"));
    out.extend(app.ui.get_devices().iter().map(|d| Entry::Device {
        id: d.id,
        name: d.name.to_string(),
    }));
    out.extend(tree.iter().filter(|r| stick(r)).map(node));
    out.extend(app.ui.get_histories().iter().map(|r| node(&r)));
    out.push(Entry::Recordings);
    out.push(special(super::sources::TAG_LIST, "Tag list"));
    out.push(special(super::sources::CREATED, "Created"));
    out.extend(app.ui.get_auto_sort().iter().map(|r| node(&r)));
    out
}

/// Where the encoder stands in `entries`: the open recordings, stick, or
/// Explorer folder, else the open node.
fn current(app: &App, entries: &[Entry]) -> Option<usize> {
    let section = app.ui.get_section();
    let wanted = |e: &Entry| match e {
        Entry::Recordings => section == "recordings",
        Entry::Device { id, .. } => section == "devices" && app.ui.get_device_selected() == *id,
        Entry::Folder { id, .. } => {
            section == "library"
                && app.ui.get_node_kind() == "explorer"
                && app.ui.get_explorer_selected() == *id
        }
        Entry::Node { id, .. } => {
            section == "library"
                && app.ui.get_node_kind() != "explorer"
                && app.ui.get_selected_node() == *id
        }
    };
    entries.iter().position(wanted)
}

/// The index `delta` steps from `at` (clamped at both ends; an unknown
/// position counts as the first entry; pure).
pub(crate) fn step(len: usize, at: Option<usize>, delta: i32) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let i = at.unwrap_or(0) as i64;
    Some((i + delta as i64).clamp(0, len as i64 - 1) as usize)
}

/// The entry holding `entries[i]`: the nearest one above it, of its kind,
/// one level less deep (pure).
pub(crate) fn parent(entries: &[Entry], i: usize) -> Option<usize> {
    let e = entries.get(i)?;
    let depth = e.depth();
    if depth == 0 {
        return None;
    }
    (0..i)
        .rev()
        .find(|&j| entries[j].same_kind(e) && entries[j].depth() < depth)
}

/// Opens an entry: its tracks show in the list.
fn open(app: &App, e: &Entry) {
    match e {
        Entry::Node { id, .. } => app.ui.invoke_select_node(*id),
        Entry::Folder { id, .. } => app.ui.invoke_explorer_select(*id),
        Entry::Device { id, .. } => app.ui.invoke_open_device(*id),
        Entry::Recordings => app.ui.invoke_select_section("recordings".into()),
    }
}

/// Opens or closes a folder entry.
fn toggle(app: &App, e: &Entry) {
    match e {
        Entry::Node { id, .. } => app.ui.invoke_toggle_node(*id),
        Entry::Folder { id, .. } => app.ui.invoke_explorer_toggle(*id),
        _ => {}
    }
}

fn set_focus(app: &App, tree: bool) {
    app.state.borrow_mut().browser.focus_tree = tree;
    app.ui
        .set_browse_focus(SharedString::from(if tree { "tree" } else { "list" }));
}

/// Steps to entry `i`, opens it and names it in the status line.
fn go(app: &App, entries: &[Entry], i: usize) {
    let Some(e) = entries.get(i) else { return };
    open(app, e);
    set_focus(app, true);
    app.status(e.name());
}

/// One level in from the lists column: a closed folder opens and its first
/// entry is opened; an open folder hands over to its first entry; anything
/// else enters its track list (the first track selected when none is).
pub(crate) fn enter(app: &App) {
    let list = entries(app);
    let Some(i) = current(app, &list) else {
        if let Some(i) = step(list.len(), None, 0) {
            go(app, &list, i);
        }
        return;
    };
    let e = list[i].clone();
    match e.folder() {
        Some(expanded) => {
            if !expanded {
                toggle(app, &e);
            }
            // The rows below appear with the toggle; take the first one.
            let list = entries(app);
            let child = list
                .get(i + 1)
                .filter(|c| c.same_kind(&e) && c.depth() > e.depth());
            match child {
                Some(_) => go(app, &list, i + 1),
                // A folder without sub-folders (the Explorer reads them
                // lazily): its own tracks.
                None => enter_list(app),
            }
        }
        None => enter_list(app),
    }
}

fn enter_list(app: &App) {
    set_focus(app, false);
    if app.state.borrow().browser.selection.is_empty() && app.state.borrow().row_count() > 0 {
        super::select_row(app, 0, false, false);
        app.ui.set_focus_row(0);
    }
}

/// One level out: from the tracks to the lists column; there, an open
/// folder closes, and an entry inside a folder steps to that folder.
pub(crate) fn back(app: &App) {
    if !app.state.borrow().browser.focus_tree {
        set_focus(app, true);
        return;
    }
    let list = entries(app);
    let Some(i) = current(app, &list) else { return };
    if list[i].folder() == Some(true) {
        toggle(app, &list[i]);
        return;
    }
    if let Some(p) = parent(&list, i) {
        go(app, &list, p);
    }
}

/// The track list has the focus (a track was picked with the mouse).
pub(crate) fn to_list(app: &App) {
    if in_lists(app) {
        set_focus(app, false);
    }
}

/// Whether the encoder is in the lists column (a deck's LOAD then goes in
/// instead of loading).
pub(crate) fn in_lists(app: &App) -> bool {
    app.state.borrow().browser.focus_tree
}

/// Handles one browse action of a controller.
pub(crate) fn controller(app: &App, action: &BrowseAction) {
    let kiosk = app.state.borrow().config.kiosk;
    let in_tree = in_lists(app);
    match action {
        BrowseAction::Rotate { delta, shifted } => {
            if kiosk {
                app.ui.set_browser_open(true);
            }
            let delta = if *shifted { delta * 8 } else { *delta };
            if in_tree {
                let list = entries(app);
                let at = current(app, &list);
                if let Some(next) = step(list.len(), at, delta)
                    && Some(next) != at
                {
                    go(app, &list, next);
                }
            } else {
                for _ in 0..delta.unsigned_abs().min(64) {
                    super::step_selection(app, delta < 0);
                }
            }
        }
        BrowseAction::Press { shifted } => {
            if in_tree {
                enter(app);
            } else if *shifted {
                super::sources::add_selected_to_tag_list(app);
            } else {
                if !crate::explorer::load_selection(app, None) {
                    decks::load_selected_into_free_deck(app);
                }
                if kiosk {
                    app.ui.set_browser_open(false);
                }
            }
        }
        BrowseAction::Back => back(app),
        BrowseAction::ViewToggle => {
            if kiosk {
                app.ui.set_browser_open(!app.ui.get_browser_open());
            } else if app.ui.get_mode() == "prepare" {
                let next = if app.ui.get_prepare_layout() == "browser" {
                    "player"
                } else {
                    "browser"
                };
                crate::shell::set_prepare_layout(app, next);
            } else {
                // Perform: the browse layout (small decks, large browser) and back.
                crate::decks::toggle_browse_layout(app);
            }
        }
        BrowseAction::Similar => {
            app.ui.invoke_select_node(super::sources::SIMILAR);
            set_focus(app, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(id: i32, depth: i32) -> Entry {
        Entry::Node {
            id,
            name: String::new(),
            depth,
            has_children: false,
            expanded: false,
        }
    }

    fn f(id: i32, depth: i32) -> Entry {
        Entry::Folder {
            id,
            name: String::new(),
            depth,
            has_children: false,
            expanded: false,
        }
    }

    #[test]
    fn steps_clamp_at_the_ends() {
        assert_eq!(step(4, Some(0), 2), Some(2));
        assert_eq!(step(4, Some(2), -5), Some(0));
        assert_eq!(step(4, Some(3), 3), Some(3));
        assert_eq!(step(4, None, 1), Some(1), "an unknown position counts as the first");
        assert_eq!(step(0, None, 1), None);
    }

    #[test]
    fn the_parent_is_one_level_up_of_the_same_kind() {
        // Collection, a folder with a list and a sub-folder with a list,
        // then the Explorer: a root with a sub-folder.
        let list = vec![n(-1, 0), n(1, 0), n(2, 1), n(3, 1), n(4, 2), f(0, 0), f(1, 1)];
        assert_eq!(parent(&list, 2), Some(1));
        assert_eq!(parent(&list, 4), Some(3));
        assert_eq!(parent(&list, 1), None, "top level");
        assert_eq!(parent(&list, 6), Some(5), "a folder's parent is a folder");
        assert_eq!(parent(&list, 5), None);
    }
}
