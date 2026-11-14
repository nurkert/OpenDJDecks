//! Browsing from a controller (SPEC §9): the browse encoder moves through
//! everything in the lists column, top to bottom as it is drawn (the
//! collection, playlists, the Explorer's folders, similar tracks, sticks,
//! histories, recordings, the tag list, created tracks and AUTO SORT) or,
//! once a list is entered, through its tracks. Folders stay closed while
//! the encoder passes over them. A press or a deck's LOAD in the column
//! goes one level in (opens a closed folder and steps to its first entry,
//! closes an open one, or enters the track list); in the track list a press
//! loads the track. BACK goes one level out every time: from the tracks to
//! their list, from an entry to the folder holding it (which closes), an
//! open folder closes, and from the top level on to the rail of sources at
//! the column's left edge (all tracks, playlists, similar, sampler, files,
//! sticks, histories, recordings, tag list), where the encoder steps through
//! the sources and a press goes back in at the source's first entry. VIEW
//! enlarges the browser and SIMILAR opens the similar tracks. The part with
//! focus is marked in the window (`browse-focus`, `browse-rail`).

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

/// The sources of the rail at the left edge of the lists column, in the
/// order they are drawn (`ui/sidebar.slint`, `rail-items`): all tracks,
/// playlists, similar tracks, the sampler, the Explorer, sticks, histories,
/// recordings and the tag list.
pub(crate) const RAIL: [&str; 9] = [
    "All tracks",
    "Playlists",
    "Similar tracks",
    "Sampler",
    "Explorer",
    "Devices",
    "Histories",
    "Recordings",
    "Tag list",
];
const RAIL_SAMPLER: usize = 3;

/// Everything in the lists column, in the order it is drawn
/// (`ui/sidebar.slint`), in groups with the source of the rail each belongs
/// to (created tracks and AUTO SORT, which have none, go with the tag list).
fn groups(app: &App) -> Vec<(usize, Vec<Entry>)> {
    let tree: Vec<crate::TreeRow> = app.ui.get_tree().iter().collect();
    let stick = |r: &crate::TreeRow| r.kind == "stick" || r.id <= -1_000_000;
    let mut devices: Vec<Entry> = app
        .ui
        .get_devices()
        .iter()
        .map(|d| Entry::Device {
            id: d.id,
            name: d.name.to_string(),
        })
        .collect();
    devices.extend(tree.iter().filter(|r| stick(r)).map(node));
    let mut more = vec![
        special(super::sources::TAG_LIST, "Tag list"),
        special(super::sources::CREATED, "Created"),
    ];
    more.extend(app.ui.get_auto_sort().iter().map(|r| node(&r)));
    vec![
        (0, tree.iter().filter(|r| r.kind == "all").map(node).collect()),
        (
            1,
            tree.iter()
                .filter(|r| r.kind != "all" && !stick(r) && r.id > -100)
                .map(node)
                .collect(),
        ),
        (
            4,
            app.ui
                .get_explorer()
                .iter()
                .map(|f| Entry::Folder {
                    id: f.id,
                    name: f.name.to_string(),
                    depth: f.depth,
                    has_children: f.has_children,
                    expanded: f.expanded,
                })
                .collect(),
        ),
        (2, vec![special(super::sources::SIMILAR, "Similar tracks")]),
        (5, devices),
        (6, app.ui.get_histories().iter().map(|r| node(&r)).collect()),
        (7, vec![Entry::Recordings]),
        (8, more),
    ]
}

/// Everything in the lists column, in the order it is drawn.
pub(crate) fn entries(app: &App) -> Vec<Entry> {
    groups(app).into_iter().flat_map(|(_, g)| g).collect()
}

/// The source of the rail entry `i` belongs to (pure).
pub(crate) fn rail_of(groups: &[(usize, usize)], i: usize) -> usize {
    let mut start = 0;
    for &(rail, len) in groups {
        if i < start + len {
            return rail;
        }
        start += len;
    }
    0
}

/// The first entry of source `rail`, if it has any (pure).
pub(crate) fn first_of(groups: &[(usize, usize)], rail: usize) -> Option<usize> {
    let mut start = 0;
    for &(r, len) in groups {
        if r == rail && len > 0 {
            return Some(start);
        }
        start += len;
    }
    None
}

/// The groups' sources and sizes, for `rail_of` and `first_of`.
fn sizes(app: &App) -> Vec<(usize, usize)> {
    groups(app).iter().map(|(r, g)| (*r, g.len())).collect()
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

/// Where BACK on `entries[i]` goes in the lists column (pure).
#[derive(Debug, PartialEq)]
pub(crate) enum Out {
    /// The open folder `i` closes, the encoder stays on it.
    Close(usize),
    /// Up to the folder `p` holding the entry, which closes.
    Up(usize),
    /// On to the rail.
    Rail,
}

/// Where BACK on `entries[i]` goes: an open folder closes, an entry inside
/// a folder goes up to it (and it closes), the top level goes to the rail.
pub(crate) fn out_of(entries: &[Entry], i: usize) -> Out {
    if entries.get(i).and_then(Entry::folder) == Some(true) {
        return Out::Close(i);
    }
    match parent(entries, i) {
        Some(p) => Out::Up(p),
        None => Out::Rail,
    }
}

/// Whether LOAD on `e` closes it: an open folder (pure).
pub(crate) fn load_closes(e: &Entry) -> bool {
    e.folder() == Some(true)
}

/// Opens an entry: its tracks show in the list. A folder stays closed (a
/// click opens it, the encoder only passes over it).
fn open(app: &App, e: &Entry) {
    match e {
        Entry::Node { id, .. } => super::select_node_with(app, *id, false),
        Entry::Folder { id, .. } => {
            let path = app.state.borrow().explorer.path_of(*id);
            if let Some(p) = path {
                crate::explorer::open_folder_with(app, p, false);
            }
        }
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
    {
        let mut st = app.state.borrow_mut();
        st.browser.focus_tree = tree;
        st.browser.focus_rail = None;
    }
    app.ui
        .set_browse_focus(SharedString::from(if tree { "tree" } else { "list" }));
    if tree {
        unfold(app);
    }
}

/// The encoder moves to source `i` of the rail (and names it).
fn set_rail(app: &App, i: usize) {
    {
        let mut st = app.state.borrow_mut();
        st.browser.focus_tree = true;
        st.browser.focus_rail = Some(i);
    }
    app.ui.set_browse_rail(i as i32);
    app.ui.set_browse_focus(SharedString::from("rail"));
    unfold(app);
    app.status(od_i18n::tr(RAIL[i]).to_string());
}

/// Lists folded away to the rail come back when the encoder goes into them.
fn unfold(app: &App) {
    if app.ui.get_lists_collapsed() {
        app.ui.set_lists_collapsed(false);
    }
}

fn rail(app: &App) -> Option<usize> {
    app.state.borrow().browser.focus_rail
}

/// Steps to entry `i`, opens it and names it in the status line.
fn go(app: &App, entries: &[Entry], i: usize) {
    let Some(e) = entries.get(i) else { return };
    open(app, e);
    set_focus(app, true);
    app.status(e.name());
}

/// One level in from the lists column: a closed folder opens and its first
/// entry is opened; an open folder closes; anything else enters its track
/// list (the first track selected when none is).
pub(crate) fn enter(app: &App) {
    if let Some(r) = rail(app) {
        enter_source(app, r);
        return;
    }
    let list = entries(app);
    let Some(i) = current(app, &list) else {
        if let Some(i) = step(list.len(), None, 0) {
            go(app, &list, i);
        }
        return;
    };
    let e = list[i].clone();
    if load_closes(&e) {
        toggle(app, &e);
        return;
    }
    match e.folder() {
        Some(_) => {
            toggle(app, &e);
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

/// One level in from the rail: the source's first entry is opened (the
/// sampler shows its panel); a source without entries keeps the focus.
fn enter_source(app: &App, r: usize) {
    if r == RAIL_SAMPLER {
        app.ui.invoke_menu_action("browser.source.sampler".into());
        return;
    }
    if let Some(i) = first_of(&sizes(app), r) {
        go(app, &entries(app), i);
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
/// folder closes, an entry inside a folder steps to that folder and closes
/// it, and an entry at the top level hands over to its source in the rail.
pub(crate) fn back(app: &App) {
    if !app.state.borrow().browser.focus_tree {
        set_focus(app, true);
        return;
    }
    if rail(app).is_some() {
        return;
    }
    let list = entries(app);
    let Some(i) = current(app, &list) else {
        set_rail(app, 0);
        return;
    };
    match out_of(&list, i) {
        Out::Close(i) => toggle(app, &list[i]),
        Out::Up(p) => {
            go(app, &list, p);
            if list[p].folder() == Some(true) {
                toggle(app, &list[p]);
            }
        }
        Out::Rail => set_rail(app, rail_of(&sizes(app), i)),
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
            if let Some(r) = rail(app) {
                if let Some(next) = step(RAIL.len(), Some(r), delta)
                    && next != r
                {
                    set_rail(app, next);
                }
            } else if in_tree {
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

    fn open(e: Entry) -> Entry {
        match e {
            Entry::Node { id, depth, .. } => Entry::Node {
                id,
                name: String::new(),
                depth,
                has_children: true,
                expanded: true,
            },
            Entry::Folder { id, depth, .. } => Entry::Folder {
                id,
                name: String::new(),
                depth,
                has_children: true,
                expanded: true,
            },
            e => e,
        }
    }

    #[test]
    fn back_closes_the_folder_it_is_on_or_in() {
        // An open folder with a list, a closed folder (with sub-folders),
        // then the Explorer: an open root with a sub-folder.
        let closed = Entry::Node {
            id: 3,
            name: String::new(),
            depth: 0,
            has_children: true,
            expanded: false,
        };
        let list = vec![open(n(1, 0)), n(2, 1), closed, open(f(0, 0)), f(1, 1)];
        assert_eq!(out_of(&list, 0), Out::Close(0), "on an open folder: it closes");
        assert_eq!(out_of(&list, 1), Out::Up(0), "inside: up to the folder");
        assert_eq!(out_of(&list, 2), Out::Rail, "a closed top-level folder");
        assert_eq!(out_of(&list, 3), Out::Close(3));
        assert_eq!(out_of(&list, 4), Out::Up(3));
        // LOAD closes an open folder only; a closed one opens.
        assert!(load_closes(&list[0]));
        assert!(load_closes(&list[3]));
        assert!(!load_closes(&list[2]));
        assert!(!load_closes(&list[1]), "a list enters its tracks");
    }

    #[test]
    fn every_entry_has_its_source_in_the_rail() {
        // All tracks, two playlists, no Explorer roots, similar, one stick,
        // no histories, recordings, the tag list with created tracks.
        let groups = [(0, 1), (1, 2), (4, 0), (2, 1), (5, 1), (6, 0), (7, 1), (8, 2)];
        assert_eq!(rail_of(&groups, 0), 0);
        assert_eq!(rail_of(&groups, 2), 1);
        assert_eq!(rail_of(&groups, 3), 2, "similar after the empty Explorer");
        assert_eq!(rail_of(&groups, 4), 5);
        assert_eq!(rail_of(&groups, 6), 8);
        assert_eq!(rail_of(&groups, 7), 8, "created goes with the tag list");
        assert_eq!(first_of(&groups, 0), Some(0));
        assert_eq!(first_of(&groups, 2), Some(3));
        assert_eq!(first_of(&groups, 8), Some(6));
        assert_eq!(first_of(&groups, 4), None, "no Explorer roots");
        assert_eq!(first_of(&groups, 3), None, "the sampler has no entries");
        assert_eq!(RAIL.len(), 9, "one per source of the rail");
    }
}
