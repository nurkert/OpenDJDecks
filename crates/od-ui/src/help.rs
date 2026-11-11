//! The help overlay (roadmap [23]): every control on screen named.
//!
//! F12, `?` (where no shortcut takes it) or the ? key in the top bar show
//! it. Rust walks the window's element tree for the controls a person can
//! operate (their spoken name, tooltip and place), numbers them in reading
//! order, and lays the numbered callouts out so none overlaps another or
//! the list. The list beside them names every control with its shortcut. A
//! click or tap explains the control under it instead of pressing it.
//!
//! The names are the controls' accessible names and the texts their
//! tooltips (`a11y.rs`), so the overlay, the tooltips and screen readers
//! say the same.

use crate::app::{App, handler};
use i_slint_core::accessibility::AccessibleStringProperty as A;
use i_slint_core::item_tree::ItemRc;
use i_slint_core::items::{AccessibleRole, Clip};
use i_slint_core::window::WindowInner;
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::rc::Rc;

/// Roles a person operates.
pub(crate) fn operable(r: AccessibleRole) -> bool {
    matches!(
        r,
        AccessibleRole::Button
            | AccessibleRole::Checkbox
            | AccessibleRole::Combobox
            | AccessibleRole::Slider
            | AccessibleRole::Spinbox
            | AccessibleRole::Tab
            | AccessibleRole::TextInput
            | AccessibleRole::Switch
            | AccessibleRole::RadioButton
    )
}

/// A control found in the element tree.
pub(crate) struct Found {
    pub rc: ItemRc,
    pub id: String,
    pub name: String,
    pub description: String,
    /// x, y, width, height in window coordinates.
    pub rect: (f32, f32, f32, f32),
    /// Part of another control (the ‹ › of a stepper) or of a list row.
    pub part: bool,
}

fn prop(rc: &ItemRc, p: A) -> String {
    rc.accessible_string_property(p)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// The visible, enabled controls of a window, in tree order.
pub(crate) fn controls(window: &slint::Window) -> Vec<Found> {
    let inner = WindowInner::from_pub(window);
    let size = window.size().to_logical(window.scale_factor());
    let root = ItemRc::new_root(inner.component());
    let mut out = Vec::new();
    walk(&root, (0.0, 0.0, size.width, size.height), false, &mut out);
    out
}

fn walk(rc: &ItemRc, clip: (f32, f32, f32, f32), part: bool, out: &mut Vec<Found>) {
    if !rc.is_visible() {
        return;
    }
    if let Some(c) = rc.downcast::<Clip>() {
        let c = c.as_pin_ref();
        if c.is_visibility_clip() && c.clip() {
            return;
        }
    }
    let g = rc.geometry();
    let p = rc.map_to_window(g.origin);
    let rect = (p.x, p.y, g.width(), g.height());
    let ix = (rect.0 + rect.2).min(clip.0 + clip.2) - rect.0.max(clip.0);
    let iy = (rect.1 + rect.3).min(clip.1 + clip.3) - rect.1.max(clip.1);
    let role = rc.accessible_role();
    let id = prop(rc, A::Id);
    // The overlay's own parts are not explained by it.
    if id.starts_with("app.help") {
        return;
    }
    // Controls inside a list row (a rating) go with the row.
    let part = part || (id.starts_with("browser.row") && id.matches('.').count() >= 2);
    if operable(role)
        && rect.2 > 0.5
        && rect.3 > 0.5
        && ix > 0.5
        && iy > 0.5
        && prop(rc, A::Enabled) != "false"
    {
        out.push(Found {
            rc: rc.clone(),
            name: prop(rc, A::Label),
            description: prop(rc, A::Description),
            id,
            rect,
            part,
        });
    }
    let part = part || operable(role) || role == AccessibleRole::ListItem;
    let child_clip = if rc.borrow().as_ref().clips_children() {
        let x = rect.0.max(clip.0);
        let y = rect.1.max(clip.1);
        let r = (rect.0 + rect.2).min(clip.0 + clip.2);
        let b = (rect.1 + rect.3).min(clip.1 + clip.3);
        (x, y, (r - x).max(0.0), (b - y).max(0.0))
    } else {
        clip
    };
    let mut c = rc.first_child();
    while let Some(ch) = c {
        walk(&ch, child_clip, part, out);
        c = ch.next_sibling();
    }
}

/// One entry of the overlay.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    pub id: String,
    pub name: String,
    pub keys: String,
    pub tip: String,
    pub rect: (f32, f32, f32, f32),
    /// The callout: x, y, width (16 px high); `None` where none fits.
    pub badge: Option<(f32, f32, f32)>,
}

pub(crate) const BADGE_H: f32 = 16.0;

/// The callout width for a number.
pub(crate) fn badge_w(n: usize) -> f32 {
    8.0 + 6.0 * n.to_string().len() as f32
}

/// The list panel in window coordinates: x, y, width, height (help.slint's
/// `panel`, in an overlay that starts `top` px down: under a menu bar that
/// is part of the window).
pub(crate) fn panel_rect(win: (f32, f32), left: bool) -> (f32, f32, f32, f32) {
    panel_rect_at(win, left, overlay_top())
}

fn panel_rect_at(win: (f32, f32), left: bool, top: f32) -> (f32, f32, f32, f32) {
    let w = (win.0 * 0.3).clamp(240.0, 340.0);
    let x = if left { 8.0 } else { win.0 - w - 8.0 };
    (x, top + 8.0, w, win.1 - top - 16.0)
}

thread_local! {
    static TOP: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

fn overlay_top() -> f32 {
    TOP.with(|t| t.get())
}

/// Where the overlay starts in the window (its origin), found in the tree.
fn find_origin(rc: &ItemRc) -> Option<(f32, f32)> {
    if prop(rc, A::Id) == "app.help" {
        let p = rc.map_to_window(rc.geometry().origin);
        return Some((p.x, p.y));
    }
    let mut c = rc.first_child();
    while let Some(ch) = c {
        if let Some(o) = find_origin(&ch) {
            return Some(o);
        }
        c = ch.next_sibling();
    }
    None
}

fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32), gap: f32) -> bool {
    a.0 < b.0 + b.2 + gap && b.0 < a.0 + a.2 + gap && a.1 < b.1 + b.3 + gap && b.1 < a.1 + a.3 + gap
}

/// Orders the entries for reading (rows top to bottom, left to right
/// within a row) and places each callout where it overlaps no other
/// callout and not the list: straddling the control's top-left corner,
/// else another corner, inside a corner, its centre, beside it, or the
/// nearest free spot.
pub(crate) fn lay_out(entries: &mut Vec<Entry>, win: (f32, f32), panel: (f32, f32, f32, f32)) {
    lay_out_in(entries, (0.0, 0.0, win.0, win.1), panel)
}

/// As [`lay_out`], with the callouts kept inside `area` (x0, y0, x1, y1):
/// the overlay, which starts under a menu bar that is part of the window.
pub(crate) fn lay_out_in(entries: &mut Vec<Entry>, area: (f32, f32, f32, f32), panel: (f32, f32, f32, f32)) {
    entries.sort_by(|a, b| {
        let ra = (a.rect.1 / 24.0).floor();
        let rb = (b.rect.1 / 24.0).floor();
        ra.partial_cmp(&rb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.rect.0.partial_cmp(&b.rect.0).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut placed: Vec<(f32, f32, f32, f32)> = Vec::new();
    // A callout never covers another control's middle: a tap there
    // explains that control.
    let centres: Vec<(f32, f32)> = entries
        .iter()
        .map(|e| (e.rect.0 + e.rect.2 / 2.0, e.rect.1 + e.rect.3 / 2.0))
        .collect();
    for (i, e) in entries.iter_mut().enumerate() {
        let w = badge_w(i + 1);
        let h = BADGE_H;
        let (x, y, cw, ch) = e.rect;
        // Straddling a corner first: the legend in the middle stays readable.
        let mut cands = vec![
            (x - w / 2.0, y - h / 2.0),
            (x + cw - w / 2.0, y - h / 2.0),
            (x - w / 2.0, y + ch - h / 2.0),
            (x + cw - w / 2.0, y + ch - h / 2.0),
            (x + 1.0, y + 1.0),
            (x + cw - w - 1.0, y + 1.0),
            (x + 1.0, y + ch - h - 1.0),
            (x + cw - w - 1.0, y + ch - h - 1.0),
            (x + (cw - w) / 2.0, y + (ch - h) / 2.0),
            (x, y - h - 1.0),
            (x - w - 1.0, y),
            (x + cw + 1.0, y),
            (x, y + ch + 1.0),
        ];
        // Further out, nearest first (a leader line joins it to its control).
        for r in 1..=16 {
            let d = r as f32 * 6.0;
            for (dx, dy) in [(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d), (-d, -d), (d, -d), (-d, d), (d, d)] {
                cands.push((x + 1.0 + dx, y + 1.0 + dy));
            }
        }
        e.badge = cands.into_iter().find_map(|(bx, by)| {
            let b = (bx, by, w, h);
            let inside = bx >= area.0 && by >= area.1 && bx + w <= area.2 && by + h <= area.3;
            let covers = centres.iter().enumerate().any(|(k, c)| {
                k != i && c.0 >= bx - 2.0 && c.0 <= bx + w + 2.0 && c.1 >= by - 2.0 && c.1 <= by + h + 2.0
            });
            (inside && !covers && !overlaps(b, panel, 0.0) && !placed.iter().any(|p| overlaps(b, *p, 1.0)))
                .then_some((bx, by, w))
        });
        if let Some((bx, by, w)) = e.badge {
            placed.push((bx, by, w, h));
        }
    }
}

/// The overlay's entries for the controls of the main window.
pub(crate) fn entries(app: &App) -> Vec<Entry> {
    let keymap = app.state.borrow().keymap.clone();
    controls(app.ui.window())
        .into_iter()
        .filter(|c| !c.part && !c.name.trim().is_empty())
        .map(|c| {
            let (tip, mut keys) = crate::a11y::split_description(&c.description);
            if keys.is_empty() && crate::keymap::action(&c.id).is_some() {
                keys = crate::shell::shortcut_text(&keymap, &c.id);
            }
            Entry {
                id: c.id,
                name: c.name,
                keys,
                tip,
                rect: c.rect,
                badge: None,
            }
        })
        .collect()
}

thread_local! {
    static SHOWN: std::cell::RefCell<Vec<Entry>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn window_size(app: &App) -> (f32, f32) {
    let w = app.ui.window();
    let s = w.size().to_logical(w.scale_factor());
    (s.width, s.height)
}

/// Collects the controls and lays the overlay out (keeps the selection).
pub(crate) fn refresh(app: &App) {
    let help = app.ui.global::<crate::Help>();
    let win = window_size(app);
    let root = ItemRc::new_root(WindowInner::from_pub(app.ui.window()).component());
    let origin = find_origin(&root).unwrap_or((0.0, 0.0));
    TOP.with(|t| t.set(origin.1));
    help.set_origin_x(origin.0);
    help.set_origin_y(origin.1);
    let mut list = entries(app);
    let left = help.get_panel_left();
    let panel = panel_rect(win, left);
    lay_out_in(&mut list, (origin.0, origin.1, win.0, win.1), panel);
    let selected_id = SHOWN.with(|s| {
        let s = s.borrow();
        usize::try_from(help.get_selected())
            .ok()
            .and_then(|i| s.get(i))
            .map(|e| e.id.clone())
    });
    let rows: Vec<crate::HelpItem> = list
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (bx, by, bw) = e.badge.unwrap_or((0.0, 0.0, 0.0));
            crate::HelpItem {
                n: i as i32 + 1,
                name: e.name.clone().into(),
                keys: e.keys.clone().into(),
                tip: e.tip.clone().into(),
                x: e.rect.0,
                y: e.rect.1,
                w: e.rect.2,
                h: e.rect.3,
                badge: e.badge.is_some(),
                bx,
                by,
                bw,
            }
        })
        .collect();
    let sel = selected_id
        .and_then(|id| list.iter().position(|e| !id.is_empty() && e.id == id))
        .map(|i| i as i32)
        .unwrap_or(-1);
    SHOWN.with(|s| *s.borrow_mut() = list);
    help.set_items(Rc::new(VecModel::from(rows)).into());
    help.set_selected(sel);
}

/// Shows or hides the overlay.
pub(crate) fn toggle(app: &Rc<App>) {
    let help = app.ui.global::<crate::Help>();
    if help.get_on() {
        close(app);
        return;
    }
    app.ui.global::<crate::TooltipState>().invoke_dismiss();
    // The list goes to the side with fewer controls under it.
    let win = window_size(app);
    let all = controls(app.ui.window());
    let count = |left: bool| {
        let p = panel_rect(win, left);
        all.iter()
            .filter(|c| overlaps(c.rect, p, 0.0))
            .count()
    };
    help.set_panel_left(count(true) < count(false));
    help.set_selected(-1);
    SHOWN.with(|s| s.borrow_mut().clear());
    help.set_on(true);
    refresh(app);
    // Once the overlay is built its place in the window is known (under a
    // menu bar that is part of the window): lay out again.
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_millis(30), move || {
        if let Some(a) = weak.upgrade()
            && a.ui.global::<crate::Help>().get_on()
        {
            refresh(&a);
        }
    });
    // The keys go to the window (a focused slider keeps no arrows).
    app.ui.invoke_focus_keys();
    // Kept in step with the view while it is open.
    let weak = Rc::downgrade(app);
    TIMER.with(|t| {
        t.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(700), move || {
            if let Some(a) = weak.upgrade()
                && a.ui.global::<crate::Help>().get_on()
            {
                refresh(&a);
            }
        })
    });
}

thread_local! {
    static TIMER: slint::Timer = slint::Timer::default();
}

pub(crate) fn close(app: &App) {
    let help = app.ui.global::<crate::Help>();
    help.set_on(false);
    help.set_selected(-1);
    help.set_items(Rc::new(VecModel::<crate::HelpItem>::default()).into());
    TIMER.with(|t| t.stop());
}

/// The control at a window position (the smallest one holding it).
pub(crate) fn pick(app: &App, x: f32, y: f32) {
    let help = app.ui.global::<crate::Help>();
    let items = help.get_items();
    // A callout first; else the control whose centre is nearest among
    // those holding the point (round keys sit in overlapping boxes).
    let mut best: Option<(usize, f32)> = None;
    for (i, it) in items.iter().enumerate() {
        let hit = x >= it.x - 2.0 && x <= it.x + it.w + 2.0 && y >= it.y - 2.0 && y <= it.y + it.h + 2.0;
        let on_badge = it.badge && x >= it.bx && x <= it.bx + it.bw && y >= it.by && y <= it.by + BADGE_H;
        if hit || on_badge {
            let (cx, cy) = (it.x + it.w / 2.0, it.y + it.h / 2.0);
            let d = if on_badge { -1.0 } else { ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() };
            if best.is_none_or(|(_, b)| d < b) {
                best = Some((i, d));
            }
        }
    }
    help.set_selected(best.map(|(i, _)| i as i32).unwrap_or(-1));
}

/// Keys while the overlay is open: F12, ? and Esc close it, the arrows move
/// through the list. Everything else waits (nothing is pressed by accident).
pub(crate) fn key(app: &Rc<App>, text: &str, control: bool, meta: bool) -> bool {
    if control || meta {
        return false;
    }
    const ESC: &str = "\u{1b}";
    const F12: &str = "\u{F70F}";
    const UP: &str = "\u{F700}";
    const DOWN: &str = "\u{F701}";
    match text {
        ESC | F12 | "?" => close(app),
        UP => step(app, -1),
        DOWN => step(app, 1),
        _ => {}
    }
    true
}

pub(crate) fn step(app: &App, d: i32) {
    let help = app.ui.global::<crate::Help>();
    let n = help.get_items().row_count() as i32;
    if n == 0 {
        return;
    }
    let cur = help.get_selected();
    let next = if cur < 0 {
        if d > 0 { 0 } else { n - 1 }
    } else {
        (cur + d).clamp(0, n - 1)
    };
    help.set_selected(next);
}

pub(crate) fn wire(app: &Rc<App>) {
    let help = app.ui.global::<crate::Help>();
    help.on_toggle(handler!(app, |a| toggle(&a)));
    help.on_close(handler!(app, |a| close(&a)));
    help.on_pick(handler!(app, |a, x: f32, y: f32| pick(&a, x, y)));
    help.on_step(handler!(app, |a, d: i32| step(&a, d)));
    help.on_side(handler!(app, |a| {
        let h = a.ui.global::<crate::Help>();
        h.set_panel_left(!h.get_panel_left());
        refresh(&a);
    }));
    demo(app);
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=help` opens the
/// overlay once the window is up, `help-pick` also explains the first
/// deck's PLAY key.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else {
        return;
    };
    let parts: Vec<&str> = v.split(',').collect();
    if !parts.iter().any(|p| *p == "help" || *p == "help-pick") {
        return;
    }
    let pick_play = parts.contains(&"help-pick");
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(6), move || {
        let Some(a) = weak.upgrade() else { return };
        toggle(&a);
        if pick_play {
            let i = SHOWN.with(|s| s.borrow().iter().position(|e| e.id == "deck1.play"));
            if let Some(i) = i {
                a.ui.global::<crate::Help>().set_selected(i as i32);
            }
        }
    });
}

#[allow(dead_code)]
fn _name(s: &SharedString) -> &str {
    s.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(x: f32, y: f32, w: f32, h: f32) -> Entry {
        Entry {
            id: String::new(),
            name: "x".into(),
            keys: String::new(),
            tip: String::new(),
            rect: (x, y, w, h),
            badge: None,
        }
    }

    #[test]
    fn callouts_never_overlap() {
        // A dense grid of small keys, as on a compact deck.
        let mut list: Vec<Entry> = (0..120)
            .map(|i| e((i % 12) as f32 * 22.0 + 10.0, (i / 12) as f32 * 20.0 + 10.0, 20.0, 18.0))
            .collect();
        let win = (800.0, 640.0);
        let panel = panel_rect(win, false);
        lay_out(&mut list, win, panel);
        let placed: Vec<_> = list.iter().filter_map(|e| e.badge).collect();
        assert!(placed.len() > 60, "most get a callout: {}", placed.len());
        for (i, a) in placed.iter().enumerate() {
            let ra = (a.0, a.1, a.2, BADGE_H);
            assert!(!overlaps(ra, panel, 0.0));
            for b in &placed[i + 1..] {
                assert!(!overlaps(ra, (b.0, b.1, b.2, BADGE_H), 0.5), "{a:?} {b:?}");
            }
        }
        // Reading order: rows top to bottom, left to right.
        assert!(list[0].rect.1 <= list[13].rect.1 && list[0].rect.0 < list[1].rect.0);
    }
}
