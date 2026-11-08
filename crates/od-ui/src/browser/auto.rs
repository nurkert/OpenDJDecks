//! AUTO SORT in the tree: the collection classified offline by tempo,
//! energy, mood, character and genre (`od_library::autosort`). The lists are
//! computed off the UI thread whenever the library's change log moves
//! (imports, analysis, tag edits); profiles of tracks whose analysis did not
//! change are kept, so a refresh after one analysed track is quick.

use super::Special;
use crate::TreeRow;
use crate::app::App;
use crossbeam_channel::{Receiver, TryRecvError, bounded};
use od_core::TrackId;
use od_library::autosort::{self, AutoGroup, AutoInput, AutoList, TrackProfile};
use od_library::{Library, SortKey};
use std::collections::{HashMap, HashSet};

/// Tree ids: group `g` is `AUTO_BASE - g * STRIDE`, its list `i` one below
/// plus `i` (between the stick roots and the histories).
pub(crate) const AUTO_BASE: i32 = -1200;
const STRIDE: i32 = 100;

type Profiles = HashMap<TrackId, (u32, TrackProfile)>;

/// A finished classification.
pub(crate) struct Done {
    seq: i64,
    profiles: Profiles,
    lists: Vec<AutoList>,
}

#[derive(Default)]
pub(crate) struct AutoState {
    pub lists: Vec<AutoList>,
    profiles: Profiles,
    /// Change-log head the lists were made at.
    seq: Option<i64>,
    /// Running classification: the change-log head it started at.
    job: Option<(i64, Receiver<Done>)>,
    /// Groups opened in the tree.
    pub expanded: HashSet<AutoGroup>,
}

/// A tree id of AUTO SORT.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AutoNode {
    Group(AutoGroup),
    List(String),
}

fn group_index(g: AutoGroup) -> i32 {
    AutoGroup::ALL.iter().position(|x| *x == g).unwrap_or(0) as i32
}

pub(crate) fn group_id(g: AutoGroup) -> i32 {
    AUTO_BASE - group_index(g) * STRIDE
}

/// The tree id of the list `key` (its place in its group).
pub(crate) fn list_id(lists: &[AutoList], key: &str) -> Option<i32> {
    let l = lists.iter().find(|l| l.key == key)?;
    let i = lists
        .iter()
        .filter(|x| x.group == l.group)
        .position(|x| x.key == key)? as i32;
    (i < STRIDE - 1).then(|| group_id(l.group) - 1 - i)
}

/// What a tree id of AUTO SORT stands for.
pub(crate) fn decode(id: i32, lists: &[AutoList]) -> Option<AutoNode> {
    if id > AUTO_BASE || id <= AUTO_BASE - STRIDE * AutoGroup::ALL.len() as i32 {
        return None;
    }
    let off = AUTO_BASE - id;
    let g = *AutoGroup::ALL.get((off / STRIDE) as usize)?;
    match off % STRIDE {
        0 => Some(AutoNode::Group(g)),
        i => lists
            .iter()
            .filter(|l| l.group == g)
            .nth(i as usize - 1)
            .map(|l| AutoNode::List(l.key.clone())),
    }
}

/// Classifies the library (reading only profiles not in `known`).
fn compute(lib: &Library, mut profiles: Profiles) -> od_library::Result<Done> {
    let seq = lib.change_seq()?;
    let known: HashMap<TrackId, u32> = profiles.iter().map(|(id, (v, _))| (*id, *v)).collect();
    profiles.extend(lib.auto_profiles(&known)?);
    let tracks = lib.tracks(SortKey::DateAdded, true)?;
    let present: HashSet<TrackId> = tracks.iter().map(|t| t.id).collect();
    profiles.retain(|id, _| present.contains(id));
    let inputs: Vec<AutoInput> = tracks
        .into_iter()
        .filter(|t| !t.missing)
        .map(|t| AutoInput {
            id: t.id,
            bpm: t.bpm,
            key: t.key,
            genre: t.tags.genre,
            profile: profiles.get(&t.id).map(|p| p.1).unwrap_or_default(),
        })
        .collect();
    Ok(Done {
        seq,
        lists: autosort::classify(&inputs),
        profiles,
    })
}

/// Starts a classification when the library changed since the last one
/// (called with every tree refresh; one query when nothing changed).
pub(crate) fn check(app: &App) {
    let (seq, path, profiles) = {
        let st = app.state.borrow();
        let a = &st.browser.auto;
        let Ok(seq) = st.lib.change_seq() else { return };
        if a.job.is_some() || a.seq == Some(seq) {
            return;
        }
        (
            seq,
            st.lib.path().map(|p| p.to_path_buf()),
            a.profiles.clone(),
        )
    };
    let Some(path) = path else {
        // A library in memory: classify here.
        let done = compute(&app.state.borrow().lib, profiles);
        if let Ok(done) = done {
            store(app, done);
        }
        return;
    };
    let (tx, rx) = bounded(1);
    let spawned = std::thread::Builder::new()
        .name("od-autosort".into())
        .spawn(move || {
            let r = Library::open(&path).and_then(|lib| compute(&lib, profiles));
            match r {
                Ok(done) => {
                    let _ = tx.send(done);
                }
                Err(e) => tracing::warn!(error = %e, "auto sort failed"),
            }
        });
    if spawned.is_ok() {
        app.state.borrow_mut().browser.auto.job = Some((seq, rx));
    }
}

/// Keeps a finished classification; true when the lists changed.
fn store(app: &App, done: Done) -> bool {
    let mut st = app.state.borrow_mut();
    let a = &mut st.browser.auto;
    a.seq = Some(done.seq);
    a.profiles = done.profiles;
    let changed = a.lists != done.lists;
    a.lists = done.lists;
    changed
}

/// Frame tick: a finished classification goes into the tree (and the list,
/// when an AUTO SORT list is open).
pub(crate) fn poll(app: &App) {
    let r = {
        let st = app.state.borrow();
        match &st.browser.auto.job {
            Some((seq, rx)) => (*seq, rx.try_recv()),
            None => return,
        }
    };
    match r {
        (_, Ok(done)) => {
            app.state.borrow_mut().browser.auto.job = None;
            if !store(app, done) {
                // Same lists (the change did not move a track): no refresh,
                // which would rebuild an open menu; a newer change gets its
                // own run.
                check(app);
                return;
            }
        }
        (_, Err(TryRecvError::Empty)) => return,
        (seq, Err(TryRecvError::Disconnected)) => {
            // Failed: do not retry until the library changes again.
            let mut st = app.state.borrow_mut();
            st.browser.auto.job = None;
            st.browser.auto.seq = Some(seq);
            return;
        }
    }
    // Only the AUTO SORT rows (a full tree refresh would rebuild the row
    // menu's playlist targets under an open menu).
    refresh_rows(app);
    let open = matches!(app.state.borrow().browser.special, Some(Special::Auto(_)));
    if open {
        app.state.borrow_mut().load_rows();
        super::refresh_tracks(app);
    }
}

/// Projects the AUTO SORT rows into the sidebar (and keeps an open list
/// selected there).
pub(crate) fn refresh_rows(app: &App) {
    let st = app.state.borrow();
    let b = &st.browser;
    app.ui.set_auto_sort(crate::models::model(rows(
        &b.auto.lists,
        &b.auto.expanded,
        &b.tree_filter,
    )));
    if let (Some(Special::Auto(key)), None) = (&b.special, st.explorer.open.as_ref()) {
        if let Some(id) = list_id(&b.auto.lists, key) {
            app.ui.set_selected_node(id);
        }
    }
}

/// The tracks of list `key` in its order.
pub(crate) fn tracks(lists: &[AutoList], key: &str) -> Vec<TrackId> {
    lists
        .iter()
        .find(|l| l.key == key)
        .map(|l| l.tracks.clone())
        .unwrap_or_default()
}

/// Name of list `key` in the UI language.
pub(crate) fn list_name(lists: &[AutoList], key: &str) -> String {
    lists
        .iter()
        .find(|l| l.key == key)
        .map(|l| od_i18n::tr(&l.name).to_string())
        .unwrap_or_default()
}

/// Sidebar rows: each group with lists (a folder row), its lists below
/// when it is open or a tree filter matches them (pure).
pub(crate) fn rows(
    lists: &[AutoList],
    expanded: &HashSet<AutoGroup>,
    filter: &str,
) -> Vec<TreeRow> {
    let longest = lists
        .iter()
        .map(|l| l.tracks.len())
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let mut out = Vec::new();
    for g in AutoGroup::ALL {
        let mine: Vec<(i32, &AutoList)> = lists
            .iter()
            .filter(|l| l.group == g)
            .take((STRIDE - 1) as usize)
            .enumerate()
            .map(|(i, l)| (group_id(g) - 1 - i as i32, l))
            .filter(|(_, l)| {
                filter.is_empty() || od_i18n::tr(&l.name).to_lowercase().contains(filter)
            })
            .collect();
        if mine.is_empty() {
            continue;
        }
        let open = expanded.contains(&g) || !filter.is_empty();
        out.push(TreeRow {
            id: group_id(g),
            name: od_i18n::tr(g.title()).into(),
            kind: "folder".into(),
            depth: 0,
            has_children: true,
            expanded: open,
            ..Default::default()
        });
        if open {
            out.extend(mine.into_iter().map(|(id, l)| TreeRow {
                id,
                name: od_i18n::tr(&l.name).into(),
                kind: "auto".into(),
                depth: 1,
                count: l.tracks.len() as i32,
                length_frac: l.tracks.len() as f32 / longest,
                ..Default::default()
            }));
        }
    }
    out
}

/// A click on a group opens or closes it.
pub(crate) fn toggle(app: &App, g: AutoGroup) {
    {
        let mut st = app.state.borrow_mut();
        let e = &mut st.browser.auto.expanded;
        if !e.remove(&g) {
            e.insert(g);
        }
    }
    super::refresh_tree(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(group: AutoGroup, key: &str, n: usize) -> AutoList {
        AutoList {
            group,
            key: key.into(),
            name: key.into(),
            tracks: (0..n as i64).map(TrackId).collect(),
        }
    }

    #[test]
    fn ids_round_trip_and_stay_clear_of_other_sources() {
        let lists = vec![
            list(AutoGroup::Tempo, "tempo:124-130", 4),
            list(AutoGroup::Tempo, "tempo:130-140", 2),
            list(AutoGroup::Genre, "genre:house", 8),
        ];
        assert_eq!(
            decode(group_id(AutoGroup::Tempo), &lists),
            Some(AutoNode::Group(AutoGroup::Tempo))
        );
        for l in &lists {
            let id = list_id(&lists, &l.key).unwrap();
            assert_eq!(decode(id, &lists), Some(AutoNode::List(l.key.clone())));
            assert_eq!(crate::browser::tree::decode_stick_id(id), None);
            assert_eq!(crate::browser::sources::decode(id, &[]), None);
        }
        assert_eq!(decode(-1, &lists), None);
        assert_eq!(decode(super::super::sources::HISTORY_BASE, &lists), None);
        assert_eq!(
            decode(group_id(AutoGroup::Tempo) - 5, &lists),
            None,
            "no such list"
        );
    }

    #[test]
    fn rows_open_groups_and_follow_the_filter() {
        let lists = vec![
            list(AutoGroup::Tempo, "tempo:124-130", 4),
            list(AutoGroup::Energy, "energy:high", 2),
        ];
        let closed = rows(&lists, &HashSet::new(), "");
        assert_eq!(closed.len(), 2, "two groups, closed");
        assert!(
            closed
                .iter()
                .all(|r| r.kind == "folder" && r.has_children && !r.expanded)
        );
        let open = rows(&lists, &[AutoGroup::Tempo].into(), "");
        assert_eq!(open.len(), 3);
        assert_eq!(
            (open[1].kind.as_str(), open[1].count, open[1].depth),
            ("auto", 4, 1)
        );
        assert_eq!(open[1].length_frac, 1.0);
        let filtered = rows(&lists, &HashSet::new(), "energy");
        assert_eq!(filtered.len(), 2, "only the matching group, opened");
        assert!(filtered[0].expanded);
    }
}
