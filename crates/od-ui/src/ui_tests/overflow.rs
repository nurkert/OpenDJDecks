//! Overflow audit: every view, popup and dialog at several screen sizes,
//! interface scales and both languages.
//!
//! The app is laid out in logical pixels: a screen of W × H physical pixels
//! at an interface scale `s` (the app's own scale setting times the
//! operating system's factor, gig.rs) gives a window of W/s × H/s. A window
//! cannot be smaller than the app's minimum (`min-width` / `min-height` of
//! app.slint, 800 × 640): below it the window manager keeps the minimum and
//! the window runs past the screen, so the audit clamps to it. That makes the
//! minimum the worst case of every small screen at a high scale.
//!
//! The main window gets its scale through the app's setting (as the GIG menu
//! and Preferences set it); the Preferences and booth windows through the
//! window's scale factor (as the operating system sets it). Screens 800 × 640
//! (the minimum), 1280 × 800, 1440 × 900 and 1920 × 1080 at 100, 125, 150 and
//! 200 % give ten distinct window sizes; every state is audited at each of
//! them, in German and in English.
//!
//! For each case the real element tree (the window and its open popups) is
//! walked through `i_slint_core` and three things are reported:
//!
//! - **clipped**: an element that draws (text, a filled or outlined
//!   rectangle, an image, a path, or a control) and leaves its clipping
//!   parent, its popup or the window by more than 0.5 px. Content of a
//!   scrolling area may leave it along the axis it scrolls. Only the
//!   outermost offender of a subtree is reported.
//! - **text**: a text that needs more room than its box while it neither
//!   elides nor wraps (or a wrapping text taller than its box), or whose
//!   glyphs run past the key or panel it sits on.
//! - **overlap**: two controls of the same layer that overlap while neither
//!   holds the other.
//!
//! The report (`target/overflow/report-<group>.md`) lists view, screen size,
//! scale, language, the element (label, type and text) and the rects. The
//! test fails on any finding that is not on the allowlist ([`ALLOW`]).

use super::harness::{Fixture, Ui};
use crate::settings::{META_SETTINGS, Settings};
use i_slint_core::item_tree::ItemRc;
use i_slint_core::items::{
    AccessibleRole, BasicBorderRectangle, BorderRectangle, Clip, ClippedImage, ComplexText,
    Flickable, ImageItem, Opacity, Orientation, Path, Rectangle, SimpleText, StyledTextItem,
    TextInput, TextOverflow, TextWrap,
};
use i_slint_core::window::{PopupWindowLocation, WindowInner};
use slint::ComponentHandle;
use slint::platform::{Key, WindowEvent};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

/// The app's smallest window (app.slint `min-width` / `min-height`).
const MIN: (f32, f32) = (800.0, 640.0);
/// Preferences window: preferred and smallest size (preferences.slint).
const PREFS_PREFERRED: (f32, f32) = (900.0, 680.0);
const PREFS_MIN: (f32, f32) = (760.0, 520.0);
/// Screens (physical pixels at the operating system's 100 %).
const SCREENS: [(f32, f32); 4] = [(800.0, 640.0), (1280.0, 800.0), (1440.0, 900.0), (1920.0, 1080.0)];
/// Interface scales.
const SCALES: [f32; 4] = [1.0, 1.25, 1.5, 2.0];
const TOL: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct R {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl R {
    const INF: R = R { x: -1e7, y: -1e7, w: 2e7, h: 2e7 };
    fn right(&self) -> f32 {
        self.x + self.w
    }
    fn bottom(&self) -> f32 {
        self.y + self.h
    }
    fn intersect(&self, o: &R) -> R {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        let r = self.right().min(o.right());
        let b = self.bottom().min(o.bottom());
        R { x, y, w: (r - x).max(0.0), h: (b - y).max(0.0) }
    }
    fn contains(&self, o: &R, tol: f32) -> bool {
        o.x >= self.x - tol && o.y >= self.y - tol && o.right() <= self.right() + tol && o.bottom() <= self.bottom() + tol
    }
    fn overlap(&self, o: &R) -> f32 {
        let i = self.intersect(o);
        if i.w > TOL && i.h > TOL { i.w * i.h } else { 0.0 }
    }
    fn is_inf(&self) -> bool {
        self.w > 1e6 || self.h > 1e6
    }
    fn show(&self) -> String {
        if self.is_inf() {
            return "∞".into();
        }
        format!("{:.1},{:.1} {:.1}×{:.1}", self.x, self.y, self.w, self.h)
    }
    /// How far `o` sticks out of `self` on each side (left, top, right, bottom).
    fn excess(&self, o: &R) -> f32 {
        (self.x - o.x).max(self.y - o.y).max(o.right() - self.right()).max(o.bottom() - self.bottom())
    }
}

/// One walked item.
struct Node {
    rc: ItemRc,
    parent: Option<usize>,
    rect: R,
    /// The clip that applies to it (window or popup, clipping ancestors).
    clip: R,
    clip_by: String,
    /// What is really visible of it (every clip, scroll areas too).
    vis: R,
    /// Nearest ancestor that paints a background (a key, a panel).
    plate: Option<usize>,
    draws: bool,
    fill: bool,
    interactive: bool,
    layer: usize,
    label: String,
    type_name: String,
    text: Option<String>,
}

struct Walk {
    nodes: Vec<Node>,
    wa: Rc<dyn i_slint_core::window::WindowAdapter>,
}

fn type_name(rc: &ItemRc) -> String {
    rc.element_type_names_and_ids(0)
        .and_then(|v| {
            v.iter()
                .find(|(t, _)| !t.is_empty() && !t.starts_with("Native"))
                .or(v.first())
                .map(|(t, id)| if id.is_empty() { t.to_string() } else { format!("{t}#{id}") })
        })
        .unwrap_or_default()
}

fn transparent(b: &i_slint_core::Brush) -> bool {
    b.is_transparent()
}

fn text_of(rc: &ItemRc) -> Option<String> {
    if let Some(t) = rc.downcast::<ComplexText>() {
        return Some(t.as_pin_ref().text().to_string());
    }
    if let Some(t) = rc.downcast::<SimpleText>() {
        return Some(t.as_pin_ref().text().to_string());
    }
    if let Some(t) = rc.downcast::<TextInput>() {
        return Some(t.as_pin_ref().text().to_string());
    }
    None
}

fn interactive_role(r: AccessibleRole) -> bool {
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

impl Walk {
    fn new(window: &slint::Window) -> Self {
        let inner = WindowInner::from_pub(window);
        let wa = inner.window_adapter();
        let mut w = Walk { nodes: Vec::new(), wa };
        let size = window.size().to_logical(window.scale_factor());
        let win = R { x: 0.0, y: 0.0, w: size.width, h: size.height };
        let root = ItemRc::new_root(inner.component());
        root.visit_descendants(|_| std::ops::ControlFlow::<()>::Continue(()));
        w.walk(&root, None, win, win, "the window", None, 1.0, (0.0, 0.0), 0, "");
        let popups: Vec<_> = inner
            .active_popups()
            .iter()
            .map(|p| (p.component.clone(), match &p.location {
                PopupWindowLocation::ChildWindow(pt) => (pt.x, pt.y),
                PopupWindowLocation::TopLevel(_) => (0.0, 0.0),
            }))
            .collect();
        for (i, (tree, off)) in popups.into_iter().enumerate() {
            let root = ItemRc::new_root(tree);
            root.visit_descendants(|_| std::ops::ControlFlow::<()>::Continue(()));
            let g = root.geometry();
            let pr = R { x: off.0, y: off.1, w: g.width(), h: g.height() };
            // The popup and its content stay inside the popup and the window.
            w.walk(&root, None, pr.intersect(&win), pr.intersect(&win), "its popup in the window", None, 1.0, off, i + 1, "popup");
        }
        w
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        rc: &ItemRc,
        parent: Option<usize>,
        clip: R,
        vis: R,
        clip_by: &str,
        plate: Option<usize>,
        opacity: f32,
        off: (f32, f32),
        layer: usize,
        label: &str,
    ) {
        if let Some(c) = rc.downcast::<Clip>() {
            let c = c.as_pin_ref();
            if c.is_visibility_clip() && c.clip() {
                return;
            }
        }
        let mut opacity = opacity;
        if let Some(o) = rc.downcast::<Opacity>() {
            opacity *= o.as_pin_ref().opacity();
        }
        if opacity <= 0.01 {
            return;
        }
        let g = rc.geometry();
        let p = rc.map_to_window(g.origin);
        let rect = R { x: p.x + off.0, y: p.y + off.1, w: g.width(), h: g.height() };
        // Entirely outside its clip: not shown (scrolled away, folded).
        let seen = rect.intersect(&clip);
        if !clip.is_inf() && (seen.w <= 0.0 || seen.h <= 0.0) && (rect.w > 0.0 || rect.h > 0.0) {
            return;
        }
        let own_label = rc
            .accessible_string_property(i_slint_core::accessibility::AccessibleStringProperty::Label)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let label = if own_label.is_empty() { label.to_string() } else { own_label };
        let role = rc.accessible_role();
        let (draws, fill) = if let Some(r) = rc.downcast::<Rectangle>() {
            let f = !transparent(&r.as_pin_ref().background());
            (f, f)
        } else if let Some(r) = rc.downcast::<BasicBorderRectangle>() {
            let r = r.as_pin_ref();
            let f = !transparent(&r.background());
            (f || (r.border_width().get() > 0.0 && !transparent(&r.border_color())), f)
        } else if let Some(r) = rc.downcast::<BorderRectangle>() {
            let r = r.as_pin_ref();
            let f = !transparent(&r.background());
            (f || (r.border_width().get() > 0.0 && !transparent(&r.border_color())), f)
        } else {
            let t = rc.downcast::<ImageItem>().is_some()
                || rc.downcast::<ClippedImage>().is_some()
                || rc.downcast::<Path>().is_some()
                || rc.downcast::<ComplexText>().is_some()
                || rc.downcast::<SimpleText>().is_some()
                || rc.downcast::<StyledTextItem>().is_some()
                || rc.downcast::<TextInput>().is_some();
            (t, false)
        };
        let text = text_of(rc);
        let interactive = interactive_role(role) && rect.w > 0.5 && rect.h > 0.5;
        let idx = self.nodes.len();
        self.nodes.push(Node {
            rc: rc.clone(),
            parent,
            rect,
            clip,
            clip_by: clip_by.to_string(),
            vis,
            plate,
            draws: draws && rect.w > 0.5 && rect.h > 0.5,
            fill,
            interactive,
            layer,
            label: label.clone(),
            type_name: type_name(rc),
            text,
        });
        // What the children are clipped by.
        let (child_clip, child_by) = if let Some(f) = rc.downcast::<Flickable>() {
            let f = f.as_pin_ref();
            // Scrolls along an axis whose content is larger than the view.
            // (or is scrolled there right now, settling back after a resize).
            let sx = f.content_width().get() > g.width() + TOL || f.content_x().get().abs() > TOL;
            let sy = f.content_height().get() > g.height() + TOL || f.content_y().get().abs() > TOL;
            let mut c = clip;
            let (x0, x1) = if sx { (-1e7, 1e7) } else { (rect.x.max(clip.x), rect.right().min(clip.right())) };
            let (y0, y1) = if sy { (-1e7, 1e7) } else { (rect.y.max(clip.y), rect.bottom().min(clip.bottom())) };
            c.x = x0;
            c.w = (x1 - x0).max(0.0);
            c.y = y0;
            c.h = (y1 - y0).max(0.0);
            (c, format!("scroll area `{}` {}", self.name(idx), rect.show()))
        } else if rc.borrow().as_ref().clips_children() {
            (clip.intersect(&rect), format!("`{}` (clip) {}", self.name(idx), rect.show()))
        } else {
            (clip, clip_by.to_string())
        };
        let child_vis = if rc.borrow().as_ref().clips_children() { vis.intersect(&rect) } else { vis };
        let child_plate = if fill && rect.w > 0.5 && rect.h > 0.5 { Some(idx) } else { plate };
        let mut child = rc.first_child();
        while let Some(c) = child {
            self.walk(&c, Some(idx), child_clip, child_vis, &child_by, child_plate, opacity, off, layer, &label);
            child = c.next_sibling();
        }
    }

    fn name(&self, i: usize) -> String {
        let n = &self.nodes[i];
        let mut s = String::new();
        if !n.label.is_empty() {
            s.push_str(&n.label);
            s.push(' ');
        }
        s.push_str(&n.type_name);
        // The components it sits in (nearest first).
        let mut ctx: Vec<String> = Vec::new();
        let mut p = n.parent;
        while let Some(q) = p {
            let t = self.nodes[q].type_name.split(['#', ':']).next().unwrap_or("").to_string();
            if !t.is_empty() && t.chars().next().is_some_and(|c| c.is_uppercase()) && !["Rectangle", "Empty", "Text", "TouchArea", "Clip", "Opacity", "Image", "Flickable", "HorizontalLayout", "VerticalLayout", "GridLayout"].contains(&t.as_str()) && !ctx.contains(&t) {
                ctx.push(t);
                if ctx.len() == 2 {
                    break;
                }
            }
            p = self.nodes[q].parent;
        }
        if !ctx.is_empty() {
            let _ = write!(s, " in {}", ctx.join(" < "));
        }
        if let Some(t) = &n.text {
            let t: String = t.chars().take(40).collect();
            let _ = write!(s, " \"{}\"", t.replace('\n', " "));
        }
        s
    }

    fn is_ancestor(&self, a: usize, mut b: usize) -> bool {
        while let Some(p) = self.nodes[b].parent {
            if p == a {
                return true;
            }
            b = p;
        }
        false
    }

    /// Width and height the text needs (one line unless it wraps).
    fn text_need(&self, n: &Node) -> Option<(f32, f32, f32, TextOverflow, TextWrap, bool, bool)> {
        let item = n.rc.borrow();
        let (overflow, wrap, h_align, v_align) = if let Some(t) = n.rc.downcast::<ComplexText>() {
            let t = t.as_pin_ref();
            (t.overflow(), t.wrap(), t.horizontal_alignment(), t.vertical_alignment())
        } else if let Some(t) = n.rc.downcast::<SimpleText>() {
            let t = t.as_pin_ref();
            (TextOverflow::Clip, TextWrap::NoWrap, t.horizontal_alignment(), t.vertical_alignment())
        } else {
            return None;
        };
        let hi = item.as_ref().layout_info(Orientation::Horizontal, -1.0, &self.wa, &n.rc);
        let vi = item.as_ref().layout_info(Orientation::Vertical, n.rect.w, &self.wa, &n.rc);
        use i_slint_core::items::{TextHorizontalAlignment as H, TextVerticalAlignment as V};
        Some((hi.preferred, hi.min, vi.preferred, overflow, wrap, matches!(h_align, H::Center), matches!(v_align, V::Center)))
    }
}

#[derive(Clone, Debug)]
struct Finding {
    kind: &'static str,
    element: String,
    detail: String,
}

/// Intentional clipping and overlaps: (kind, element substring, detail
/// substring, why). Each entry says why the clip or overlap is meant.
const ALLOW: &[(&str, &str, &str, &str)] = &[
    // The EQ legends under the mid and low knobs are their kill keys; the
    // knob keeps a 24 px target over its legend on purpose (DESIGN.md,
    // mixer), and the legend takes the clicks where they meet.
    ("overlap", ".eq-mid Knob", "", "kill key under its knob"),
    ("overlap", ".eq-low Knob", "", "kill key under its knob"),
    // The filter key sits in the empty left part of the right-aligned `#`
    // column header and is painted above it.
    ("overlap", "browser.header.position HeaderCell", "", "filter key over the # header"),
    // A dashed rule repeats its dash and clips the last one at its own end.
    ("clipped", "Rectangle in HLine", "in HLine", "dash pattern cut at the rule's end"),
    ("clipped", "Rectangle in VLine", "in VLine", "dash pattern cut at the rule's end"),
];

fn allowed(f: &Finding) -> bool {
    ALLOW.iter().any(|(k, e, d, _)| *k == f.kind && f.element.contains(e) && f.detail.contains(d))
}

fn audit(window: &slint::Window) -> Vec<Finding> {
    let w = Walk::new(window);
    let mut out = Vec::new();
    let mut reported = vec![false; w.nodes.len()];
    for (i, n) in w.nodes.iter().enumerate() {
        let parent_reported = n.parent.is_some_and(|p| reported[p]);
        if parent_reported {
            reported[i] = true;
            continue;
        }
        // Clipped by its clipping parent, its popup or the window.
        if (n.draws || n.interactive) && !n.clip.contains(&n.rect, TOL) {
            out.push(Finding {
                kind: "clipped",
                element: w.name(i),
                detail: format!("{} leaves {} by {:.1} px", n.rect.show(), n.clip_by, n.clip.excess(&n.rect)),
            });
            reported[i] = true;
            continue;
        }
        // Text that does not fit.
        if let Some(t) = &n.text
            && !t.trim().is_empty()
            && let Some((need_w, need_word, need_h, overflow, wrap, centred, _)) = w.text_need(n)
        {
            let elides = overflow == TextOverflow::Elide;
            let wraps = wrap != TextWrap::NoWrap;
            if !elides && !wraps && need_w > n.rect.w + TOL {
                out.push(Finding {
                    kind: "text",
                    element: w.name(i),
                    detail: format!("needs {need_w:.1} px, box {} (neither elides nor wraps)", n.rect.show()),
                });
                reported[i] = true;
                continue;
            }
            if wrap == TextWrap::WordWrap && !elides && need_word > n.rect.w + TOL {
                out.push(Finding {
                    kind: "text",
                    element: w.name(i),
                    detail: format!("a word needs {need_word:.1} px, box {} (it breaks inside the word)", n.rect.show()),
                });
                reported[i] = true;
                continue;
            }
            if wraps && !elides && need_h > n.rect.h + 2.0 {
                out.push(Finding {
                    kind: "text",
                    element: w.name(i),
                    detail: format!("wraps to {need_h:.1} px, box {}", n.rect.show()),
                });
                reported[i] = true;
                continue;
            }
            // Glyphs on a key or panel: they stay on it.
            if let Some(p) = n.plate {
                let plate = &w.nodes[p];
                let gw = need_w.min(n.rect.w);
                let gx = if centred { n.rect.x + (n.rect.w - gw) / 2.0 } else {
                    use i_slint_core::items::TextHorizontalAlignment as H;
                    let right = n.rc.downcast::<ComplexText>().is_some_and(|t| t.as_pin_ref().horizontal_alignment() == H::Right)
                        || n.rc.downcast::<SimpleText>().is_some_and(|t| t.as_pin_ref().horizontal_alignment() == H::Right);
                    if right { n.rect.right() - gw } else { n.rect.x }
                };
                // Only what is drawn: a scrolled-away part is hidden.
                let glyphs = R { x: gx, y: n.rect.y, w: gw, h: n.rect.h }.intersect(&n.vis);
                let room = plate.rect;
                if glyphs.w > TOL && (glyphs.x < room.x - TOL || glyphs.right() > room.right() + TOL) {
                    out.push(Finding {
                        kind: "text",
                        element: w.name(i),
                        detail: format!("glyphs {} run past `{}` {}", glyphs.show(), w.name(p), plate.rect.show()),
                    });
                    reported[i] = true;
                    continue;
                }
            }
        }
    }
    // Overlapping controls of one layer.
    let ctl: Vec<usize> = (0..w.nodes.len()).filter(|&i| w.nodes[i].interactive).collect();
    for (a_pos, &a) in ctl.iter().enumerate() {
        for &b in &ctl[a_pos + 1..] {
            let (na, nb) = (&w.nodes[a], &w.nodes[b]);
            if na.layer != nb.layer {
                continue;
            }
            let (ra, rb) = (na.rect.intersect(&na.vis), nb.rect.intersect(&nb.vis));
            if ra.overlap(&rb) <= 0.0 || w.is_ancestor(a, b) || w.is_ancestor(b, a) {
                continue;
            }
            if ra.contains(&rb, TOL) || rb.contains(&ra, TOL) {
                continue;
            }
            // `b` is painted later: skip when a branch of `b` that does not
            // hold `a` covers `a` (an overlay, a dialog).
            let mut covered = false;
            let mut c = b;
            while let Some(p) = w.nodes[c].parent {
                if w.is_ancestor(p, a) {
                    break;
                }
                c = p;
            }
            if c != b && w.nodes[c].fill && w.nodes[c].rect.contains(&ra, TOL) {
                covered = true;
            }
            if covered {
                continue;
            }
            out.push(Finding {
                kind: "overlap",
                element: format!("{} × {}", w.name(a), w.name(b)),
                detail: format!("{} and {}", ra.show(), rb.show()),
            });
        }
    }
    out
}

// ----- cases -------------------------------------------------------------

#[derive(Clone, Debug)]
struct Case {
    /// Screen and scale combinations this window size stands for.
    combos: Vec<((f32, f32), f32)>,
    logical: (f32, f32),
    scale: f32,
}

impl Case {
    fn show(&self) -> String {
        let combos: Vec<String> = self
            .combos
            .iter()
            .map(|((w, h), s)| format!("{w:.0}×{h:.0} @{:.0} %", s * 100.0))
            .collect();
        format!("{:.0}×{:.0} logical ({})", self.logical.0, self.logical.1, combos.join(", "))
    }
}

/// Distinct window sizes of the screen × scale grid for a window with
/// `min` and, when the window does not fill the screen, `preferred`.
fn cases(min: (f32, f32), preferred: Option<(f32, f32)>) -> Vec<Case> {
    let mut out: Vec<Case> = Vec::new();
    for screen in SCREENS {
        for s in SCALES {
            let (mut w, mut h) = (screen.0 / s, screen.1 / s);
            if let Some(p) = preferred {
                w = w.min(p.0);
                h = h.min(p.1);
            }
            let logical = ((w.max(min.0)).round(), (h.max(min.1)).round());
            if let Some(c) = out.iter_mut().find(|c| c.logical == logical) {
                c.combos.push((screen, s));
                c.scale = c.scale.max(s);
            } else {
                out.push(Case { combos: vec![(screen, s)], logical, scale: s });
            }
        }
    }
    out
}

/// Sets the interface scale (the app's setting, as the top bar's GIG menu
/// does) and the window to `logical` at that scale.
fn apply_case(ui: &Ui, window: &slint::Window, case: &Case, main: bool) {
    if main {
        let s = case.scale;
        ui.app.state.borrow_mut().change_settings(|st| st.view.scale = s);
        crate::gig::apply(&ui.app);
    } else {
        window.dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: case.scale });
    }
    let (w, h) = case.logical;
    window.set_size(slint::PhysicalSize::new((w * case.scale).round() as u32, (h * case.scale).round() as u32));
    window.dispatch_event(WindowEvent::Resized { size: slint::LogicalSize::new(w, h) });
    ui.pump(300);
}

fn fixture(lang: &str) -> Fixture {
    let fx = Fixture::new();
    let mut lib = fx.lib();
    let mut settings = Settings::default();
    settings.view.language = lang.into();
    settings.recording.folder = fx.root.join("recordings").display().to_string();
    lib.set_meta(META_SETTINGS, &settings.to_json()).unwrap();
    drop(lib);
    fx
}

/// A state of the main window: set up from any other state of the list.
struct State {
    id: &'static str,
    setup: fn(&Ui),
    /// Popups and dialogs: undone after the audit.
    teardown: Option<fn(&Ui)>,
}

fn perform(ui: &Ui, decks: i32, level: &str) {
    let w = &ui.app.ui;
    crate::engine::ensure_engine(&ui.app);
    w.invoke_set_mode("perform".into());
    w.invoke_set_deck_count(decks);
    w.global::<crate::PerformLayout>().set_layout("horizontal".into());
    if w.global::<crate::Club>().get_enabled() {
        w.global::<crate::Club>().invoke_set_enabled(false);
    }
    crate::deck_layout::set_level(&ui.app, level);
    for (on, f) in [
        (w.get_fx_panel(), "fx"),
        (w.get_sampler_panel(), "sampler"),
        (w.get_mix_preview_panel(), "mix"),
    ] {
        if on && level != "full" {
            match f {
                "fx" => w.invoke_toggle_fx_panel(),
                "sampler" => w.invoke_toggle_sampler_panel(),
                _ => w.invoke_menu_action("perform.mix-preview.panel".into()),
            }
        }
    }
    w.global::<crate::Lighting>().set_panel(false);
    if w.get_rec_panel() {
        w.invoke_menu_action("view.panel.rec".into());
    }
    ui.pump(200);
}

fn prepare(ui: &Ui, layout: &str) {
    let w = &ui.app.ui;
    w.invoke_set_mode("prepare".into());
    w.invoke_select_section("library".into());
    w.invoke_select_node(-1);
    w.invoke_row_clicked(0, false, false);
    w.invoke_menu_action(layout.into());
    ui.pump(200);
}

/// A set of the fixture's tracks with a tempo jump and a key clash (made
/// once per fixture).
fn flow_set(ui: &Ui) -> i32 {
    let mut lib = ui.fx.lib();
    if let Some(n) = lib.all_nodes().unwrap().into_iter().find(|n| n.name == "Flow audit") {
        return n.id.0 as i32;
    }
    let ids: Vec<_> = ui.fx.tracks.iter().map(|t| t.0).collect();
    for (id, (bpm, key)) in ids.iter().zip([(120.0, "8A"), (132.0, "3B"), (133.0, "3B")]) {
        let d = lib.track(*id).unwrap().duration;
        lib.set_beatgrid(*id, od_core::Beatgrid::constant(bpm, 0.0, 1, d)).unwrap();
        lib.set_key(*id, od_core::MusicalKey::parse(key)).unwrap();
    }
    let set = lib.create_node(None, od_library::NodeKind::Set, "Flow audit").unwrap();
    lib.add_to_node(set, &ids, None).unwrap();
    drop(lib);
    ui.app.refresh_library();
    set.0 as i32
}

/// A registered stick that is not plugged in (its row, badges and state).
fn usb_stick(ui: &Ui) {
    let lib = ui.fx.lib();
    if lib.devices().unwrap_or_default().is_empty() {
        let mut lib = lib;
        lib.register_device(
            "uuid-overflow-audit",
            "AUDIT STICK WITH A LONG NAME",
            od_library::DeviceFormats { club_usb: true, opendjdecks: true, club_usb_compat: true, ..Default::default() },
        )
        .unwrap();
        ui.app.device_manager.send(od_devices::Command::Refresh);
        ui.wait_until(10.0, |u| !u.app.state.borrow().devices.statuses.is_empty());
    }
}

fn click(ui: &Ui, label: &str) {
    if let Some(e) = ui.find(label) {
        e.mock_single_click(slint::platform::PointerEventButton::Left);
        ui.pump(150);
    }
}

fn escape(ui: &Ui) {
    ui.key(Key::Escape);
    ui.pump(100);
    ui.app.ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: slint::LogicalPosition::new(1.0, 1.0),
        button: slint::platform::PointerEventButton::Left,
    });
    ui.app.ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: slint::LogicalPosition::new(1.0, 1.0),
        button: slint::platform::PointerEventButton::Left,
    });
    ui.pump(100);
}

fn main_states() -> Vec<State> {
    vec![
        State { id: "prepare.1-player", setup: |u| prepare(u, "view.layout.player"), teardown: None },
        State { id: "prepare.2-players", setup: |u| prepare(u, "view.layout.two"), teardown: None },
        State { id: "prepare.dual-player", setup: |u| prepare(u, "view.layout.dual"), teardown: None },
        State { id: "prepare.simple-player", setup: |u| prepare(u, "view.layout.simple"), teardown: None },
        State { id: "prepare.full-browser", setup: |u| prepare(u, "view.layout.browser"), teardown: None },
        State {
            id: "prepare.grid-edit",
            setup: |u| {
                prepare(u, "view.layout.player");
                click(u, "player1.tab.grid");
                click(u, "player1.grid.edit");
            },
            teardown: Some(|u| escape(u)),
        },
        State {
            id: "prepare.info-and-sub-browser",
            setup: |u| {
                prepare(u, "view.layout.player");
                u.app.ui.set_info_open(true);
                if !u.app.ui.get_sub_browser_open() {
                    u.app.ui.invoke_toggle_sub_browser();
                }
                u.pump(200);
            },
            teardown: Some(|u| {
                u.app.ui.set_info_open(false);
                if u.app.ui.get_sub_browser_open() {
                    u.app.ui.invoke_toggle_sub_browser();
                }
            }),
        },
        State {
            id: "prepare.recording-bar",
            setup: |u| {
                prepare(u, "view.layout.player");
                if !u.app.ui.get_rec_panel() {
                    u.app.ui.invoke_menu_action("view.panel.rec".into());
                }
                u.pump(200);
            },
            teardown: Some(|u| {
                if u.app.ui.get_rec_panel() {
                    u.app.ui.invoke_menu_action("view.panel.rec".into());
                }
            }),
        },
        State {
            id: "prepare.set-flow (warnings, a transition selected)",
            setup: |u| {
                prepare(u, "view.layout.player");
                let set = flow_set(u);
                u.app.ui.invoke_select_node(set);
                u.pump(200);
                u.app.ui.invoke_flow_select_pair(0);
                u.pump(200);
            },
            teardown: None,
        },
        State {
            id: "prepare.filter-chips, search and lists folded",
            setup: |u| {
                prepare(u, "view.layout.player");
                let w = &u.app.ui;
                w.invoke_filter_edit("bpm-min".into(), "100".into());
                w.invoke_filter_edit("bpm-max".into(), "140".into());
                w.invoke_filter_edit("rating".into(), "1".into());
                w.invoke_filter_edit("key".into(), "1".into());
                w.set_search_text("tune".into());
                w.invoke_search("tune".into());
                w.set_lists_collapsed(true);
                u.pump(200);
            },
            teardown: Some(|u| {
                let w = &u.app.ui;
                w.invoke_filter_edit("clear".into(), "".into());
                w.set_search_text("".into());
                w.invoke_search("".into());
                w.set_lists_collapsed(false);
            }),
        },
        State {
            id: "prepare.usb-sticks",
            setup: |u| {
                prepare(u, "view.layout.player");
                usb_stick(u);
                u.app.ui.invoke_select_section("devices".into());
                u.pump(200);
            },
            teardown: Some(|u| u.app.ui.invoke_select_section("library".into())),
        },
        State { id: "perform.2.simple", setup: |u| perform(u, 2, "simple"), teardown: None },
        State { id: "perform.2.standard", setup: |u| perform(u, 2, "standard"), teardown: None },
        State { id: "perform.2.full", setup: |u| perform(u, 2, "full"), teardown: None },
        State { id: "perform.4.simple", setup: |u| perform(u, 4, "simple"), teardown: None },
        State { id: "perform.4.standard", setup: |u| perform(u, 4, "standard"), teardown: None },
        State { id: "perform.4.full", setup: |u| perform(u, 4, "full"), teardown: None },
        State {
            id: "perform.2.vertical",
            setup: |u| {
                perform(u, 2, "standard");
                u.app.ui.global::<crate::PerformLayout>().set_layout("vertical".into());
                u.pump(200);
            },
            teardown: None,
        },
        State {
            id: "perform.4.vertical",
            setup: |u| {
                perform(u, 4, "standard");
                u.app.ui.global::<crate::PerformLayout>().set_layout("vertical".into());
                u.pump(200);
            },
            teardown: None,
        },
        State {
            id: "perform.browse",
            setup: |u| {
                perform(u, 2, "standard");
                u.app.ui.global::<crate::PerformLayout>().set_layout("browse".into());
                u.pump(200);
            },
            teardown: None,
        },
        State {
            id: "perform.panels (sampler, mix preview, recording, lighting strip)",
            setup: |u| {
                perform(u, 2, "standard");
                let w = &u.app.ui;
                w.invoke_toggle_sampler_panel();
                w.invoke_menu_action("perform.mix-preview.panel".into());
                w.invoke_menu_action("view.panel.rec".into());
                w.global::<crate::Lighting>().set_panel(true);
                u.pump(300);
            },
            teardown: None,
        },
        State {
            id: "club.2",
            setup: |u| {
                perform(u, 2, "standard");
                u.app.ui.global::<crate::Club>().invoke_set_enabled(true);
                u.pump(300);
            },
            teardown: Some(|u| u.app.ui.global::<crate::Club>().invoke_set_enabled(false)),
        },
        State {
            id: "club.4",
            setup: |u| {
                perform(u, 4, "standard");
                u.app.ui.global::<crate::Club>().invoke_set_enabled(true);
                u.pump(300);
            },
            teardown: Some(|u| u.app.ui.global::<crate::Club>().invoke_set_enabled(false)),
        },
        State { id: "lighting.stage+rig", setup: |u| lighting(u, "stage", "rig"), teardown: None },
        State { id: "lighting.pads+output", setup: |u| lighting(u, "pads", "output"), teardown: None },
        State { id: "lighting.patch+safety", setup: |u| lighting(u, "patch", "safety"), teardown: None },
        State { id: "lighting.channels", setup: |u| lighting(u, "channels", "rig"), teardown: None },
    ]
}

fn lighting(ui: &Ui, centre: &str, tab: &str) {
    let w = &ui.app.ui;
    w.invoke_set_mode("lighting".into());
    let l = w.global::<crate::Lighting>();
    l.set_centre(centre.into());
    l.set_setup_tab(tab.into());
    ui.pump(300);
}

fn popup_states() -> Vec<State> {
    let close: Option<fn(&Ui)> = Some(|u| escape(u));
    vec![
        State { id: "popup.gig", setup: |u| { perform(u, 2, "standard"); click(u, "topbar.gig.menu"); }, teardown: close },
        State { id: "popup.view-menu", setup: |u| { perform(u, 2, "standard"); click(u, "topbar.layout"); }, teardown: close },
        State {
            id: "popup.record-section",
            setup: |u| {
                perform(u, 2, "standard");
                click(u, "topbar.toggle.rec");
            },
            teardown: Some(|u| {
                if u.app.ui.get_rec_panel() {
                    u.app.ui.invoke_menu_action("view.panel.rec".into());
                }
            }),
        },
        State {
            id: "popup.fx-type",
            setup: |u| {
                perform(u, 2, "full");
                click(u, "fx.unit1.slot1.type");
            },
            teardown: close,
        },
        State {
            id: "popup.channel-fx",
            setup: |u| {
                perform(u, 2, "full");
                click(u, "fx.channel-fx.type");
            },
            teardown: close,
        },
        State { id: "popup.pad-mode", setup: |u| { perform(u, 2, "standard"); click(u, "deck1.pad-mode"); }, teardown: close },
        State { id: "popup.bpm", setup: |u| { perform(u, 2, "standard"); click(u, "deck1.bpm-set"); }, teardown: close },
        State {
            id: "dialog.gig-check",
            setup: |u| {
                perform(u, 2, "standard");
                u.app.ui.global::<crate::GigCheck>().invoke_run();
                u.wait_until(5.0, |u| u.app.ui.global::<crate::GigCheck>().get_result() != "");
            },
            teardown: Some(|u| u.app.ui.global::<crate::GigCheck>().set_visible(false)),
        },
        State {
            id: "dialog.controller-setup",
            setup: |u| {
                perform(u, 2, "standard");
                u.app.ui.global::<crate::ControllerSetup>().invoke_open();
                u.pump(300);
            },
            teardown: Some(|u| u.app.ui.global::<crate::ControllerSetup>().invoke_close()),
        },
        State {
            id: "dialog.first-start",
            setup: |u| {
                prepare(u, "view.layout.player");
                u.app.ui.global::<crate::FirstStart>().invoke_reopen();
                u.pump(300);
            },
            teardown: Some(|u| u.app.ui.global::<crate::FirstStart>().invoke_close()),
        },
        State {
            id: "dialog.restore",
            setup: |u| {
                perform(u, 2, "standard");
                let r = u.app.ui.global::<crate::Restore>();
                r.set_when("21:42".into());
                r.set_summary(od_i18n::tr("2 decks, the recording").into());
                r.set_has_decks(true);
                r.set_recording("Set 2026-11-07 21-30.wav".into());
                r.set_visible(true);
                u.pump(200);
            },
            teardown: Some(|u| u.app.ui.global::<crate::Restore>().set_visible(false)),
        },
        State {
            id: "dialog.lock",
            setup: |u| {
                perform(u, 2, "standard");
                if !u.app.ui.global::<crate::PerformLock>().get_locked() {
                    u.app.ui.global::<crate::PerformLock>().invoke_toggle();
                }
                u.pump(200);
            },
            teardown: Some(|u| {
                if u.app.ui.global::<crate::PerformLock>().get_locked() {
                    u.app.ui.global::<crate::PerformLock>().invoke_toggle();
                }
            }),
        },
        State {
            id: "dialog.stems-prompt",
            setup: |u| {
                prepare(u, "view.layout.player");
                u.app.ui.set_stems_model_prompt_title(od_i18n::tr("Separate stems on this computer?").into());
                u.app.ui.set_stems_model_size(180);
                u.app.ui.set_stems_model_prompt(true);
                u.pump(200);
            },
            teardown: Some(|u| u.app.ui.set_stems_model_prompt(false)),
        },
        State {
            id: "dialog.update-prompt",
            setup: |u| {
                prepare(u, "view.layout.player");
                u.app.ui.set_update_prompt_title("OpenDJDecks 0.9.0".into());
                u.app.ui.set_update_prompt_notes("- Faster analysis\n- The decks keep their place".into());
                u.app.ui.set_update_prompt(true);
                u.pump(200);
            },
            teardown: Some(|u| u.app.ui.set_update_prompt(false)),
        },
        State {
            id: "dialog.import-preview",
            setup: |u| {
                prepare(u, "view.layout.player");
                let g = u.app.ui.global::<crate::ImportReview>();
                g.set_file("rekordbox.xml".into());
                g.set_format("LIBRARY XML".into());
                let rows: Vec<crate::ImportCountRow> = [
                    od_i18n::tr("Tracks"),
                    od_i18n::tr("Playlists"),
                    od_i18n::tr("Hot cues"),
                ]
                .iter()
                .map(|l| crate::ImportCountRow { label: (*l).into(), before: "120".into(), after: "1 482".into(), changed: true })
                .collect();
                g.set_counts(Rc::new(slint::VecModel::from(rows)).into());
                let left: Vec<crate::ImportLeftOutRow> = (0..4)
                    .map(|i| crate::ImportLeftOutRow {
                        item: format!("A rather long track title number {i} – Extended Mix").into(),
                        reason: crate::import::reason_text(&od_import::review::LeftOutReason::NotALocalFile).into(),
                    })
                    .collect();
                g.set_left_out(Rc::new(slint::VecModel::from(left)).into());
                g.set_left_out_more(12);
                g.set_stage("preview".into());
                u.pump(200);
            },
            teardown: Some(|u| u.app.ui.global::<crate::ImportReview>().set_stage("".into())),
        },
        State {
            id: "dialog.encrypted-import",
            setup: |u| {
                prepare(u, "view.layout.player");
                u.app.ui.set_encrypted_import_open(true);
                u.pump(200);
            },
            teardown: Some(|u| u.app.ui.set_encrypted_import_open(false)),
        },
        State {
            id: "dialog.device-lost",
            setup: |u| {
                perform(u, 2, "standard");
                u.app.ui.global::<crate::AudioHealth>().set_lost_device("USB Audio Interface (2 in, 4 out)".into());
                u.app.ui.global::<crate::AudioHealth>().set_fallback("MacBook Pro Speakers".into());
                u.pump(200);
            },
            teardown: Some(|u| u.app.ui.global::<crate::AudioHealth>().set_lost_device("".into())),
        },
    ]
}

/// Report lines of one group.
#[derive(Default)]
struct Report {
    lines: Vec<String>,
    failures: usize,
    by_view: BTreeMap<String, usize>,
    allowed: usize,
    cases: usize,
}

impl Report {
    fn add(&mut self, view: &str, case: &Case, lang: &str, findings: Vec<Finding>) {
        self.cases += 1;
        // One line per element and kind (the same element in several cases
        // is listed once per case).
        for f in findings {
            if allowed(&f) {
                self.allowed += 1;
                continue;
            }
            self.failures += 1;
            *self.by_view.entry(view.to_string()).or_default() += 1;
            self.lines.push(format!(
                "| {view} | {} | {lang} | {} | {} | {} |",
                case.show(),
                f.kind,
                f.element.replace('|', "/"),
                f.detail.replace('|', "/")
            ));
        }
    }

    fn finish(self, group: &str) {
        let dir = super::views::workspace_root().join("target/overflow");
        let _ = std::fs::create_dir_all(&dir);
        let mut s = format!(
            "# Overflow audit: {group}\n\n{} cases, {} findings, {} allowed.\n\n",
            self.cases, self.failures, self.allowed
        );
        for (v, n) in &self.by_view {
            let _ = writeln!(s, "- {v}: {n}");
        }
        s.push_str("\n| view | window | lang | kind | element | rects |\n|---|---|---|---|---|---|\n");
        for l in &self.lines {
            s.push_str(l);
            s.push('\n');
        }
        let path = dir.join(format!("report-{group}.md"));
        let _ = std::fs::write(&path, &s);
        let shown: Vec<&String> = self.lines.iter().take(80).collect();
        assert!(
            self.failures == 0,
            "{} overflow findings ({}), first ones:\n{}",
            self.failures,
            path.display(),
            shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
        );
    }
}

fn run_main(lang: &str, states: Vec<State>, group: &str) {
    let _lock = super::harness::volumes_lock();
    let ui = Ui::start(fixture(lang), (1440, 900), false);
    let all = cases(MIN, None);
    let mut report = Report::default();
    // Outer loop: window sizes (a popup opens after the window has its size).
    for case in &all {
        apply_case(&ui, ui.app.ui.window(), case, true);
        for st in &states {
            (st.setup)(&ui);
            ui.pump(300);
            let findings = audit(ui.app.ui.window());
            report.add(st.id, case, lang, findings);
            if let Some(t) = st.teardown {
                t(&ui);
            }
        }
    }
    report.finish(group);
}

/// The audits are the heaviest UI tests: one at a time, so they do not
/// starve the timing of the other tests on a small CI runner.
fn heavy() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The language is chosen for the whole process when the app starts, so
/// the German runs go into a child process of this test binary: in this
/// one they would switch the strings of the tests running beside them.
fn in_own_process(test: &str) {
    if std::env::var_os("OD_OVERFLOW_CHILD").is_some() {
        return;
    }
    let _one = heavy();
    let exe = std::env::current_exe().expect("test binary");
    let status = std::process::Command::new(exe)
        .args([test, "--exact", "--ignored", "--test-threads=1", "--nocapture"])
        .env("OD_OVERFLOW_CHILD", "1")
        .status()
        .expect("child test runs");
    assert!(status.success(), "{test} found overflows (see its output above)");
}

#[test]
fn overflow_views_de() {
    in_own_process("ui_tests::overflow::overflow_views_de_run");
}

#[test]
#[ignore = "runs in its own process from overflow_views_de"]
fn overflow_views_de_run() {
    run_main("de", main_states(), "views-de");
}

#[test]
fn overflow_views_en() {
    let _one = heavy();
    run_main("en", main_states(), "views-en");
}

#[test]
fn overflow_popups_and_dialogs_de() {
    in_own_process("ui_tests::overflow::overflow_popups_and_dialogs_de_run");
}

#[test]
#[ignore = "runs in its own process from overflow_popups_and_dialogs_de"]
fn overflow_popups_and_dialogs_de_run() {
    run_main("de", popup_states(), "popups-de");
}

#[test]
fn overflow_popups_and_dialogs_en() {
    let _one = heavy();
    run_main("en", popup_states(), "popups-en");
}

/// Preferences (every page) and the booth window, in their own windows,
/// in both languages (so in a process of its own, see [`in_own_process`]).
#[test]
fn overflow_windows() {
    in_own_process("ui_tests::overflow::overflow_windows_run");
}

#[test]
#[ignore = "runs in its own process from overflow_windows"]
fn overflow_windows_run() {
    let _lock = super::harness::volumes_lock();
    let mut report = Report::default();
    for lang in ["de", "en"] {
        let ui = Ui::start(fixture(lang), (1440, 900), false);
        ui.app.ui.invoke_open_preferences();
        ui.pump(300);
        let prefs = ui.app.prefs.borrow().window.as_ref().map(|w| w.clone_strong()).expect("preferences window");
        for case in cases(PREFS_MIN, Some(PREFS_PREFERRED)) {
            apply_case(&ui, prefs.window(), &case, false);
            for page in ["audio", "view", "analysis", "controller", "keyboard", "recording", "advanced"] {
                prefs.set_page(page.into());
                ui.pump(300);
                report.add(&format!("prefs.{page}"), &case, lang, audit(prefs.window()));
            }
        }
        crate::commands::run(&ui.app, "view.booth");
        ui.pump(300);
        let booth = ui.app.booth_window.borrow().as_ref().map(|w| w.clone_strong()).expect("booth window");
        for case in cases((640.0, 360.0), None) {
            apply_case(&ui, booth.window(), &case, false);
            for decks in [2, 4] {
                ui.app.ui.invoke_set_deck_count(decks);
                ui.pump(300);
                report.add(&format!("booth.{decks}"), &case, lang, audit(booth.window()));
            }
        }
    }
    report.finish("windows");
}
