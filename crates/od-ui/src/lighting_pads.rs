//! Own looks and chases on pads, group masters and AUTO steering
//! (roadmap [15], [17], [18]), and connecting interfaces (port picker,
//! Art-Net node discovery, test pattern and the guided setup, [20]), for
//! the LIGHTING view and the lighting panel in Perform and Club.
//!
//! The rig file keeps the looks, group masters, palette and phrase table;
//! which pads are latched or held, HOLD and the test pattern are live state
//! and never saved.

use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use od_lighting::looks::{self, STEP_BEATS};
use od_lighting::{
    ArtNode, AutoPick, DeckMusic, GroupSettings, Layer, MAX_UNIVERSES, Mode, OutputKind, PAGE_PADS,
    PAGES, PadAction, PadLook, Palette, PhraseSlot, Program, TestPattern,
};
use slint::{Color, ModelRc, SharedString, VecModel};

use crate::app::{App, handler};
use crate::lighting::{LightingRuntime, SWATCHES, each, edit, program_label, refresh, rgb};
use crate::{LightAutoRow, LightLookGroupRow, LightMasterRow, LightPadView};

/// "1.3": page and pad, both from 1.
pub fn pad_number(pad: u16) -> String {
    format!(
        "{}.{}",
        pad as usize / PAGE_PADS + 1,
        pad as usize % PAGE_PADS + 1
    )
}

/// "PAD 1.3 RED WASH" or a program name: what an AUTO pick plays.
pub fn pick_label(pick: AutoPick, pads: &[PadLook]) -> String {
    match pick {
        AutoPick::Program(p) => program_label(p).to_string(),
        AutoPick::Pad(n) => match looks::find(pads, n) {
            Some(l) => format!("{} {}", pad_number(n), l.name.to_uppercase()),
            None => od_i18n::trf("{} EMPTY", &[&pad_number(n)]),
        },
    }
}

fn slot_label(s: PhraseSlot) -> String {
    match s {
        PhraseSlot::Intro => od_i18n::tr("INTRO"),
        PhraseSlot::Verse => od_i18n::tr("VERSE"),
        PhraseSlot::BuildUp => od_i18n::tr("BUILD-UP"),
        PhraseSlot::Drop => od_i18n::tr("DROP"),
        PhraseSlot::Breakdown => od_i18n::tr("BREAKDOWN"),
        PhraseSlot::Outro => od_i18n::tr("OUTRO"),
    }
    .to_string()
}

fn palette_label(p: Palette) -> String {
    match p {
        Palette::Track => od_i18n::tr("TRACK KEY"),
        Palette::Warm => od_i18n::tr("WARM"),
        Palette::Cool => od_i18n::tr("COOL"),
        Palette::Sunset => od_i18n::tr("SUNSET"),
        Palette::Ocean => od_i18n::tr("OCEAN"),
        Palette::White => od_i18n::tr("WHITE"),
        Palette::Custom => od_i18n::tr("OWN"),
    }
    .to_string()
}

fn step_label(beats: f32) -> String {
    match beats {
        b if b <= 0.25 => od_i18n::tr("1/4 BEAT").to_string(),
        b if b <= 0.5 => od_i18n::tr("1/2 BEAT").to_string(),
        b if b <= 1.0 => od_i18n::tr("1 BEAT").to_string(),
        b if b <= 2.0 => od_i18n::tr("2 BEATS").to_string(),
        b if b <= 4.0 => od_i18n::tr("1 BAR").to_string(),
        _ => od_i18n::tr("2 BARS").to_string(),
    }
}

/// Preview colour of one group's settings (`None` = colour from the music).
fn group_color(g: &GroupSettings) -> Option<[f32; 3]> {
    let dark = g.program == Some(Program::Blackout) || g.intensity.is_some_and(|v| v <= 0.0);
    if dark {
        return Some([0.11, 0.11, 0.105]);
    }
    let level = g.intensity.unwrap_or(1.0).clamp(0.35, 1.0);
    g.color.map(|c| c.map(|v| v * level))
}

/// What a look shows, one colour per group it sets in the rig's group
/// order; a chase shows the first colour of each step.
pub fn preview_colors(look: &PadLook, pads: &[PadLook], groups: &[String]) -> Vec<[f32; 3]> {
    // Groups the music colours (or that are not patched) show bone.
    const MUSIC: [f32; 3] = [0.74, 0.72, 0.67];
    let of = |l: &PadLook| -> Vec<[f32; 3]> {
        let mut names: Vec<&String> = groups
            .iter()
            .filter(|g| l.groups.contains_key(*g))
            .collect();
        let mut rest: Vec<&String> = l.groups.keys().filter(|g| !groups.contains(g)).collect();
        rest.sort();
        names.extend(rest);
        names
            .into_iter()
            .map(|g| group_color(&l.groups[g]).unwrap_or(MUSIC))
            .collect()
    };
    if look.is_chase() {
        look.steps
            .iter()
            .filter_map(|s| looks::find(pads, *s))
            .filter(|l| !l.is_chase())
            .filter_map(|l| of(l).first().copied())
            .take(8)
            .collect()
    } else {
        of(look).into_iter().take(8).collect()
    }
}

/// The eight pads of the current page (pure).
pub fn pad_views(rt: &LightingRuntime) -> Vec<LightPadView> {
    let s = &rt.config.settings;
    let groups = rt.config.patch.groups();
    let top = rt.pads.top();
    (0..PAGE_PADS)
        .map(|slot| {
            let pad = (rt.page * PAGE_PADS + slot) as u16;
            let look = looks::find(&s.pads, pad);
            let colors: Vec<Color> = look
                .map(|l| preview_colors(l, &s.pads, &groups))
                .unwrap_or_default()
                .into_iter()
                .map(rgb)
                .collect();
            LightPadView {
                slot: slot as i32,
                number: pad_number(pad).into(),
                name: look.map(|l| l.name.as_str()).unwrap_or("").into(),
                chase: look.is_some_and(PadLook::is_chase),
                flash: look.is_some_and(|l| l.action == PadAction::Flash),
                colors: ModelRc::new(VecModel::from(colors)),
                on: look.is_some() && (top == Some(pad) || rt.pads.latched == Some(pad)),
                selected: rt.editing && rt.edit_pad == Some(pad),
            }
        })
        .collect()
}

/// Who is in control, as a key for the UI and a line of text.
pub fn layer_text(rt: &LightingRuntime, decks: &[DeckMusic]) -> (String, String) {
    let s = &rt.config.settings;
    let m = rt.momentary;
    let name = |p: u16| {
        looks::find(&s.pads, p)
            .map(|l| l.name.to_uppercase())
            .unwrap_or_default()
    };
    match looks::layer(s.blackout, m.flash || m.strobe, &rt.pads, &s.pads) {
        Layer::Blackout => ("blackout".into(), od_i18n::tr("BLACKOUT").to_string()),
        Layer::Momentary => (
            "momentary".into(),
            if m.flash {
                od_i18n::tr("FLASH KEY HELD").to_string()
            } else {
                od_i18n::tr("STROBE KEY HELD").to_string()
            },
        ),
        Layer::FlashPad(p) => (
            "flash-pad".into(),
            od_i18n::trf("PAD {} · {} · HELD", &[&pad_number(p), &name(p)]),
        ),
        Layer::Latched(p) => (
            "latched".into(),
            od_i18n::trf("PAD {} · {} · LATCHED", &[&pad_number(p), &name(p)]),
        ),
        Layer::Hold => (
            "hold".into(),
            od_i18n::trf(
                "AUTO HELD · {}",
                &[&rt
                    .held
                    .map(|p| pick_label(p, &s.pads))
                    .unwrap_or_else(|| "…".into())],
            ),
        ),
        Layer::Base => (
            "base".into(),
            crate::lighting::status_text(s.mode, decks, &s.auto, &s.pads),
        ),
    }
}

/// The phrase of the deck you hear most.
fn now_slot(decks: &[DeckMusic]) -> Option<PhraseSlot> {
    decks
        .iter()
        .filter(|d| d.playing && d.weight > 0.01)
        .max_by(|a, b| a.weight.total_cmp(&b.weight))
        .and_then(|d| PhraseSlot::of(d.phrase))
}

/// Pad rows equal for the UI (the colour models compared by content).
fn same_pad(a: &LightPadView, b: &LightPadView) -> bool {
    use slint::Model;
    a.slot == b.slot
        && a.number == b.number
        && a.name == b.name
        && a.chase == b.chase
        && a.flash == b.flash
        && a.on == b.on
        && a.selected == b.selected
        && a.colors.row_count() == b.colors.row_count()
        && a.colors.iter().zip(b.colors.iter()).all(|(x, y)| x == y)
}

fn auto_rows(rt: &LightingRuntime) -> Vec<LightAutoRow> {
    let s = &rt.config.settings;
    PhraseSlot::ALL
        .iter()
        .map(|slot| LightAutoRow {
            label: slot_label(*slot).into(),
            pick: pick_label(s.auto.table.get(*slot), &s.pads).into(),
            now: s.mode == Mode::Auto && rt.now_slot == Some(*slot),
        })
        .collect()
}

/// Per frame while lighting is in view: who is in control, the pads and
/// the phrase AUTO is on. Only what changed is written.
pub(crate) fn sync_live(app: &App, decks: &[DeckMusic]) {
    let changed = {
        let mut rt = app.lighting.borrow_mut();
        rt.now_slot = now_slot(decks);
        let layer = layer_text(&rt, decks);
        let changed = layer != rt.layer;
        rt.layer = layer;
        rt.pad_rows.sync(pad_views(&rt), same_pad);
        rt.auto_rows.sync_eq(auto_rows(&rt));
        changed.then(|| rt.layer.clone())
    };
    if let Some((key, text)) = changed {
        each(app, |w| {
            w.set_layer(key.as_str().into());
            w.set_layer_text(text.as_str().into());
        });
    }
}

/// Every model of the pads, masters, AUTO and connection parts.
pub(crate) fn sync(app: &App) {
    let mut rt = app.lighting.borrow_mut();
    rt.layer = layer_text(&rt, &[]);
    let rt = &*rt;
    let s = &rt.config.settings;
    rt.pad_rows.sync(pad_views(rt), same_pad);
    let groups = rt.config.patch.groups();
    rt.master_rows.sync_eq(
        groups
            .iter()
            .map(|g| LightMasterRow {
                name: g.as_str().into(),
                level: s.group_level(g),
            })
            .collect(),
    );
    rt.auto_rows.sync_eq(auto_rows(rt));
    let edit = rt.edit_pad.and_then(|p| looks::find(&s.pads, p));
    rt.edit_rows.sync_eq(
        groups
            .iter()
            .map(|g| {
                let gs = edit.and_then(|l| l.groups.get(g));
                LightLookGroupRow {
                    name: g.as_str().into(),
                    set: gs.is_some(),
                    program: match gs {
                        None => "—".into(),
                        Some(GroupSettings { program: None, .. }) => od_i18n::tr("MUSIC").into(),
                        Some(GroupSettings {
                            program: Some(p), ..
                        }) => program_label(*p).into(),
                    },
                    color: gs
                        .and_then(|g| g.color)
                        .and_then(|c| SWATCHES.iter().position(|s| *s == c))
                        .map_or(-1, |i| i as i32),
                    level: match gs.and_then(|g| g.intensity) {
                        Some(v) => format!("{} %", (v * 100.0).round() as i32).into(),
                        None => "—".into(),
                    },
                }
            })
            .collect(),
    );
    let edit_number: SharedString = if rt.editing {
        rt.edit_pad.map(pad_number).unwrap_or_default().into()
    } else {
        SharedString::new()
    };
    let steps: Vec<SharedString> = edit
        .map(|l| {
            l.steps
                .iter()
                .map(|p| {
                    SharedString::from(match looks::find(&s.pads, *p) {
                        Some(x) => format!("{} {}", pad_number(*p), x.name),
                        None => od_i18n::trf("{} EMPTY", &[&pad_number(*p)]),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let steps = ModelRc::new(VecModel::from(steps));
    let palettes: Vec<SharedString> = Palette::ALL
        .iter()
        .map(|p| SharedString::from(palette_label(*p)))
        .collect();
    let palettes = ModelRc::new(VecModel::from(palettes));
    let palette = Palette::ALL
        .iter()
        .position(|p| *p == s.auto.palette)
        .unwrap_or(0) as i32;
    let custom: Vec<bool> = SWATCHES.iter().map(|c| s.auto.custom.contains(c)).collect();
    let custom = ModelRc::new(VecModel::from(custom));
    let ports: Vec<SharedString> = rt.ports.iter().map(|p| p.as_str().into()).collect();
    let ports = ModelRc::new(VecModel::from(ports));
    let nodes: Vec<SharedString> = rt.nodes.iter().map(|n| n.label().into()).collect();
    let nodes = ModelRc::new(VecModel::from(nodes));
    let g = &rt.guide;
    each(app, |w| {
        w.set_page(rt.page as i32);
        w.set_editing(rt.editing);
        w.set_adding_steps(rt.adding_steps);
        w.set_edit_number(edit_number.clone());
        w.set_edit_has(edit.is_some());
        w.set_edit_name(edit.map(|l| l.name.as_str()).unwrap_or("").into());
        w.set_edit_chase(edit.is_some_and(PadLook::is_chase));
        w.set_edit_flash(edit.is_some_and(|l| l.action == PadAction::Flash));
        w.set_edit_steps(steps.clone());
        w.set_edit_step_beats(step_label(edit.map_or(1.0, |l| l.step_beats)).into());
        w.set_hold(rt.pads.hold);
        w.set_layer(rt.layer.0.as_str().into());
        w.set_layer_text(rt.layer.1.as_str().into());
        w.set_palettes(palettes.clone());
        w.set_palette(palette);
        w.set_custom_colors(custom.clone());
        w.set_ports(ports.clone());
        w.set_nodes(nodes.clone());
        w.set_finding(rt.finder.is_some());
        w.set_find_text(rt.find_text.as_str().into());
        w.set_test(
            match rt.test {
                None => "",
                Some(TestPattern::Each) => "each",
                Some(TestPattern::Colours) => "colours",
                Some(TestPattern::Full) => "full",
            }
            .into(),
        );
        w.set_guide_step(g.step);
        w.set_guide_kind(g.kind.as_str().into());
        w.set_guide_target(g.target.as_str().into());
        w.set_guide_universe(g.universe.to_string().into());
        w.set_guide_note(g.note.as_str().into());
    });
}

/// Binds the models and callbacks of one window's Lighting state.
pub(crate) fn wire(app: &Rc<App>, g: &crate::Lighting<'_>) {
    {
        let rt = app.lighting.borrow();
        g.set_pads(rt.pad_rows.handle());
        g.set_masters(rt.master_rows.handle());
        g.set_auto_rows(rt.auto_rows.handle());
        g.set_edit_groups(rt.edit_rows.handle());
    }
    g.on_pad_pressed(handler!(app, |a, slot: i32, pressed: bool| {
        let pad = (a.lighting.borrow().page * PAGE_PADS) as u16 + slot.clamp(0, 7) as u16;
        let editing = a.lighting.borrow().editing;
        if editing {
            if pressed {
                open_pad(&a, pad);
            }
        } else {
            play(&a, pad, pressed);
        }
    }));
    g.on_choose_page(handler!(app, |a, p: i32| set_page(&a, p.max(0) as usize)));
    g.on_set_editing(handler!(app, |a, on: bool| {
        {
            let mut rt = a.lighting.borrow_mut();
            rt.editing = on;
            rt.adding_steps = false;
        }
        refresh(&a);
    }));
    g.on_pad_new(handler!(app, |a, chase: bool| new_look(&a, chase, false)));
    g.on_pad_capture(handler!(app, |a| new_look(&a, false, true)));
    g.on_pad_rename(handler!(app, |a, name: SharedString| {
        let name = name.trim().to_string();
        if !name.is_empty() {
            edit_look(&a, |l, _| l.name = name);
        }
    }));
    g.on_pad_flash(handler!(app, |a, on: bool| edit_look(&a, |l, _| {
        l.action = if on {
            PadAction::Flash
        } else {
            PadAction::Latch
        }
    })));
    g.on_pad_delete(handler!(app, |a| delete_look(&a)));
    g.on_pad_group_program(handler!(app, |a, gi: i32, d: i32| edit_group(
        &a,
        gi,
        |gs| {
            // MUSIC, then the programs, round.
            let n = Program::ALL.len() as i32 + 1;
            let at = gs
                .program
                .and_then(|p| Program::ALL.iter().position(|x| *x == p))
                .map_or(0, |i| i as i32 + 1);
            let next = (at + d).rem_euclid(n);
            gs.program = (next > 0).then(|| Program::ALL[next as usize - 1]);
        }
    )));
    g.on_pad_group_color(handler!(app, |a, gi: i32, ci: i32| edit_group(
        &a,
        gi,
        |gs| gs.color = SWATCHES.get(ci.max(-1) as usize).copied()
    )));
    g.on_pad_group_level(handler!(app, |a, gi: i32, d: i32| edit_group(
        &a,
        gi,
        |gs| {
            // 0 %, 10 % … 100 %, then — (the program decides).
            let at = gs.intensity.map_or(11, |v| (v * 10.0).round() as i32);
            let next = (at + d).clamp(0, 11);
            gs.intensity = (next <= 10).then_some(next as f32 / 10.0);
        }
    )));
    g.on_pad_group_clear(handler!(app, |a, gi: i32| {
        let Some(name) = a
            .lighting
            .borrow()
            .config
            .patch
            .groups()
            .get(gi as usize)
            .cloned()
        else {
            return;
        };
        edit_look(&a, |l, _| {
            l.groups.remove(&name);
        });
    }));
    g.on_pad_add_steps(handler!(app, |a, on: bool| {
        a.lighting.borrow_mut().adding_steps = on;
        refresh(&a);
    }));
    g.on_pad_step_remove(handler!(app, |a, i: i32| edit_look(&a, |l, _| {
        if (i as usize) < l.steps.len() {
            l.steps.remove(i as usize);
        }
    })));
    g.on_pad_step_beats(handler!(app, |a, d: i32| edit_look(&a, |l, _| {
        let at = STEP_BEATS
            .iter()
            .position(|b| *b >= l.step_beats - 1e-6)
            .unwrap_or(2) as i32;
        l.step_beats = STEP_BEATS[(at + d).clamp(0, STEP_BEATS.len() as i32 - 1) as usize];
    })));
    g.on_group_master(handler!(app, |a, gi: i32, v: f32| group_master(
        &a,
        gi.max(0) as usize,
        v
    )));
    g.on_set_hold(handler!(app, |a, on: bool| set_hold(&a, on)));
    g.on_release(handler!(app, |a| release(&a)));
    g.on_choose_palette(handler!(app, |a, i: i32| edit(&a, |rt| {
        if let Some(p) = Palette::ALL.get(i.max(0) as usize) {
            rt.config.settings.auto.palette = *p;
        }
    })));
    g.on_custom_color(handler!(app, |a, i: i32| edit(&a, |rt| {
        if let Some(c) = SWATCHES.get(i.max(0) as usize) {
            let custom = &mut rt.config.settings.auto.custom;
            if let Some(at) = custom.iter().position(|x| x == c) {
                custom.remove(at);
            } else {
                custom.push(*c);
            }
        }
    })));
    g.on_auto_step(handler!(app, |a, row: i32, d: i32| edit(&a, |rt| {
        let Some(slot) = PhraseSlot::ALL.get(row.max(0) as usize).copied() else {
            return;
        };
        let s = &mut rt.config.settings;
        let mut options: Vec<AutoPick> =
            Program::ALL.iter().map(|p| AutoPick::Program(*p)).collect();
        let mut pads: Vec<u16> = s.pads.iter().map(|l| l.pad).collect();
        pads.sort();
        options.extend(pads.into_iter().map(AutoPick::Pad));
        let at = options
            .iter()
            .position(|o| *o == s.auto.table.get(slot))
            .unwrap_or(0) as i32;
        let next = (at + d).rem_euclid(options.len() as i32) as usize;
        s.auto.table.set(slot, options[next]);
    })));
    // Connecting interfaces.
    g.on_refresh_ports(handler!(app, |a| refresh_ports(&a)));
    g.on_use_port(handler!(app, |a, i: i32, universe: SharedString| {
        let Some(device) = a.lighting.borrow().ports.get(i.max(0) as usize).cloned() else {
            return;
        };
        match crate::lighting::parse_universe(&universe) {
            Ok(u) => {
                crate::lighting::set_output(&a, u.unwrap_or(0), OutputKind::EnttecPro { device })
            }
            Err(e) => a.status(e),
        }
    }));
    g.on_find_nodes(handler!(app, |a| find_nodes(&a)));
    g.on_use_node(handler!(app, |a, i: i32, universe: SharedString| {
        let Some(node) = a.lighting.borrow().nodes.get(i.max(0) as usize).cloned() else {
            return;
        };
        match crate::lighting::parse_universe(&universe) {
            Ok(u) => {
                let u = u.unwrap_or(0);
                crate::lighting::set_output(&a, u, node_output(&node, u));
            }
            Err(e) => a.status(e),
        }
    }));
    g.on_set_test(handler!(app, |a, t: SharedString| set_test(
        &a,
        match t.as_str() {
            "each" => Some(TestPattern::Each),
            "colours" => Some(TestPattern::Colours),
            "full" => Some(TestPattern::Full),
            _ => None,
        }
    )));
    g.on_guide(handler!(app, |a, action: SharedString| guide(
        &a,
        action.as_str()
    )));
}

/// Art-Net output to a node for app universe `u`: the node's port with the
/// same Port-Address when it has one, else its first port.
pub fn node_output(node: &ArtNode, u: u16) -> OutputKind {
    let port = if node.universes.contains(&u) {
        u
    } else {
        node.universes.first().copied().unwrap_or(u)
    };
    OutputKind::ArtNet {
        target: node.target(),
        universe: port,
    }
}

fn set_page(app: &App, page: usize) {
    app.lighting.borrow_mut().page = page.min(PAGES - 1);
    refresh(app);
}

/// Plays a pad (screen, deck pads, controller): latch or flash.
pub(crate) fn play(app: &App, pad: u16, pressed: bool) -> bool {
    let found = {
        let mut rt = app.lighting.borrow_mut();
        let pads = rt.config.settings.pads.clone();
        let found = rt.pads.press(&pads, pad, pressed);
        if found {
            rt.runner.pads(rt.pads.clone());
        }
        found
    };
    if found {
        refresh(app);
    }
    found
}

/// A pad clicked in PADS while editing: opens it, or adds it as a step of
/// the chase being edited.
fn open_pad(app: &App, pad: u16) {
    let adding = {
        let rt = app.lighting.borrow();
        rt.adding_steps
            && rt
                .edit_pad
                .and_then(|p| looks::find(&rt.config.settings.pads, p))
                .is_some_and(|l| l.is_chase() && l.pad != pad)
    };
    if adding {
        let ok = looks::find(&app.lighting.borrow().config.settings.pads, pad)
            .is_some_and(|l| !l.is_chase());
        if ok {
            edit_look(app, |l, _| l.steps.push(pad));
        } else {
            app.status(od_i18n::tr(
                "Lighting: a chase steps through looks; that pad holds none",
            ));
        }
        return;
    }
    {
        let mut rt = app.lighting.borrow_mut();
        rt.edit_pad = Some(pad);
        rt.adding_steps = false;
    }
    refresh(app);
}

/// A new look (white and steady on every group), chase, or a look from the
/// PRO group settings, on the pad in the editor.
fn new_look(app: &App, chase: bool, from_pro: bool) {
    edit(app, |rt| {
        let Some(pad) = rt.edit_pad else { return };
        let s = &mut rt.config.settings;
        if looks::find(&s.pads, pad).is_some() {
            return;
        }
        let n = s.pads.iter().filter(|l| l.is_chase() == chase).count() + 1;
        let name = if chase {
            od_i18n::trf("Chase {}", &[&n])
        } else {
            od_i18n::trf("Look {}", &[&n])
        };
        let mut look = PadLook::new(pad, &name);
        if !chase {
            let pro: Vec<(String, GroupSettings)> = s
                .groups
                .iter()
                .filter(|(_, g)| **g != GroupSettings::default())
                .map(|(k, g)| (k.clone(), g.clone()))
                .collect();
            if from_pro && !pro.is_empty() {
                look.groups.extend(pro);
            } else {
                for g in rt.config.patch.groups() {
                    look.groups.insert(
                        g,
                        GroupSettings {
                            program: Some(Program::Static),
                            color: Some([1.0; 3]),
                            ..Default::default()
                        },
                    );
                }
            }
        } else {
            look.chase = true;
            rt.adding_steps = true;
        }
        rt.config.settings.pads.push(look);
    });
}

/// Changes the look in the editor and saves the rig.
fn edit_look(app: &App, f: impl FnOnce(&mut PadLook, &[String])) {
    edit(app, |rt| {
        let Some(pad) = rt.edit_pad else { return };
        let groups = rt.config.patch.groups();
        if let Some(l) = rt.config.settings.pads.iter_mut().find(|l| l.pad == pad) {
            f(l, &groups);
        }
        let pads = rt.config.settings.pads.clone();
        rt.pads.retain(&pads);
        rt.runner.pads(rt.pads.clone());
    });
}

/// Changes group `gi` of the look in the editor (the look then sets it).
fn edit_group(app: &App, gi: i32, f: impl FnOnce(&mut GroupSettings)) {
    edit_look(app, |l, groups| {
        if let Some(name) = groups.get(gi.max(0) as usize) {
            f(l.groups.entry(name.clone()).or_default());
        }
    });
}

fn delete_look(app: &App) {
    edit(app, |rt| {
        let Some(pad) = rt.edit_pad else { return };
        let s = &mut rt.config.settings;
        s.pads.retain(|l| l.pad != pad);
        for l in &mut s.pads {
            l.steps.retain(|p| *p != pad);
        }
        s.auto.table.forget_pad(pad);
        let pads = s.pads.clone();
        rt.pads.retain(&pads);
        rt.adding_steps = false;
        rt.runner.pads(rt.pads.clone());
    });
}

pub(crate) fn group_master(app: &App, gi: usize, v: f32) {
    edit(app, |rt| {
        if let Some(name) = rt.config.patch.groups().get(gi).cloned() {
            let v = v.clamp(0.0, 1.0);
            if v >= 0.999 {
                rt.config.settings.group_levels.remove(&name);
            } else {
                rt.config.settings.group_levels.insert(name, v);
            }
        }
    });
}

pub(crate) fn set_hold(app: &App, on: bool) {
    {
        let mut rt = app.lighting.borrow_mut();
        rt.pads.hold = on;
        rt.runner.pads(rt.pads.clone());
    }
    refresh(app);
}

/// Hands the show back to the music: the latched pad and HOLD end.
pub(crate) fn release(app: &App) {
    {
        let mut rt = app.lighting.borrow_mut();
        rt.pads.release();
        rt.runner.pads(rt.pads.clone());
    }
    refresh(app);
}

/// A mapped controller input for the pads, pages, masters and AUTO.
pub(crate) fn controller(app: &App, a: &od_controller::action::LightingAction) {
    use od_controller::action::LightingAction as L;
    match a {
        L::Pad { pad, pressed } => {
            let n = (app.lighting.borrow().page * PAGE_PADS) as u16 + (*pad as u16).min(7);
            play(app, n, *pressed);
        }
        L::Page(p) => set_page(app, *p as usize),
        L::PageNext => {
            let p = app.lighting.borrow().page;
            set_page(app, (p + 1) % PAGES);
        }
        L::PagePrev => {
            let p = app.lighting.borrow().page;
            set_page(app, (p + PAGES - 1) % PAGES);
        }
        L::GroupMaster { group, level } => group_master(app, *group as usize, *level),
        L::Release => release(app),
        L::Hold => {
            let on = !app.lighting.borrow().pads.hold;
            set_hold(app, on);
        }
        _ => {}
    }
}

/// The deck pads in LIGHTING mode: a pad of the current page that holds a
/// look plays it; true when it did.
pub(crate) fn deck_pad(app: &App, index: usize, pressed: bool) -> bool {
    let pad = (app.lighting.borrow().page * PAGE_PADS + index.min(7)) as u16;
    let has = looks::find(&app.lighting.borrow().config.settings.pads, pad).is_some();
    has && play(app, pad, pressed)
}

/// Labels of the deck pads in LIGHTING mode for the looks of the page.
pub(crate) fn deck_pad_views(app: &App, rows: &mut [crate::PadView]) -> usize {
    let rt = app.lighting.borrow();
    let top = rt.pads.top();
    let mut n = 0;
    for r in rows.iter_mut() {
        let pad = (rt.page * PAGE_PADS + r.index as usize) as u16;
        if let Some(l) = looks::find(&rt.config.settings.pads, pad) {
            r.caption = l.name.to_uppercase().into();
            r.sub = if l.is_chase() {
                od_i18n::tr("CHASE")
            } else if l.action == PadAction::Flash {
                od_i18n::tr("FLASH")
            } else {
                od_i18n::tr("LATCH")
            }
            .into();
            r.set = true;
            r.lit = top == Some(pad) || rt.pads.latched == Some(pad);
            n += 1;
        }
    }
    n
}

// ── Connecting interfaces ──

pub(crate) fn refresh_ports(app: &App) {
    {
        let mut rt = app.lighting.borrow_mut();
        if !rt.ports_fixed {
            rt.ports = od_lighting::serial_ports();
        }
    }
    refresh(app);
}

/// How long discovery listens for answers.
pub const FIND_WAIT: Duration = Duration::from_millis(1500);

/// Asks the network for Art-Net nodes on a worker thread; the answer is
/// picked up by [`poll`].
pub(crate) fn find_nodes(app: &App) {
    {
        let mut rt = app.lighting.borrow_mut();
        if rt.finder.is_some() {
            return;
        }
        let targets: Vec<SocketAddr> = rt.poll_targets.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("artnet-discovery".into())
            .spawn(move || {
                let _ = tx.send(od_lighting::discovery::discover(&targets, FIND_WAIT));
            });
        match spawned {
            Ok(_) => {
                rt.finder = Some(rx);
                rt.find_text = od_i18n::tr("ASKING THE NETWORK…").to_string();
            }
            Err(e) => rt.find_text = e.to_string(),
        }
    }
    refresh(app);
}

/// Frame tick: takes the discovery answer when it is there.
pub(crate) fn poll(app: &App) {
    let result = {
        let rt = app.lighting.borrow();
        let Some(rx) = rt.finder.as_ref() else { return };
        match rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => None,
        }
    };
    {
        let mut rt = app.lighting.borrow_mut();
        rt.finder = None;
        match result {
            Some(Ok(nodes)) => {
                rt.find_text = if nodes.is_empty() {
                    od_i18n::tr("NO NODE ANSWERED").to_string()
                } else {
                    od_i18n::trf("{} FOUND", &[&nodes.len()])
                };
                rt.nodes = nodes;
            }
            Some(Err(e)) => rt.find_text = od_i18n::trf("NETWORK: {}", &[&e]),
            None => rt.find_text.clear(),
        }
    }
    refresh(app);
}

pub(crate) fn set_test(app: &App, pattern: Option<TestPattern>) {
    {
        let mut rt = app.lighting.borrow_mut();
        rt.test = pattern;
        rt.runner.test(pattern);
    }
    refresh(app);
}

/// State of the guided connection setup.
#[derive(Clone, Debug)]
pub struct Guide {
    /// 0 closed, 1 kind, 2 interface, 3 universe, 4 test, 5 done.
    pub step: i32,
    /// "artnet" | "sacn" | "usb".
    pub kind: String,
    /// Address or port chosen ("" = none yet).
    pub target: String,
    /// The chosen node's Port-Addresses (Art-Net).
    pub node_ports: Vec<u16>,
    /// Universe, from 1.
    pub universe: u16,
    pub note: String,
}

impl Default for Guide {
    fn default() -> Self {
        Self {
            step: 0,
            kind: String::new(),
            target: String::new(),
            node_ports: Vec::new(),
            universe: 1,
            note: String::new(),
        }
    }
}

/// One step of the guided setup.
pub(crate) fn guide(app: &App, action: &str) {
    let (verb, arg) = action.split_once(':').unwrap_or((action, ""));
    let mut then: Option<fn(&App)> = None;
    let mut output: Option<(u16, OutputKind)> = None;
    let mut test: Option<Option<TestPattern>> = None;
    {
        let mut rt = app.lighting.borrow_mut();
        let rt = &mut *rt;
        let g = &mut rt.guide;
        g.note.clear();
        match verb {
            "open" => {
                *g = Guide {
                    step: 1,
                    ..Guide::default()
                };
            }
            "close" => {
                if (1..=4).contains(&g.step) && rt.test.is_some() {
                    test = Some(None);
                }
                *g = Guide::default();
            }
            "kind" => {
                g.kind = arg.to_string();
                g.target.clear();
                g.node_ports.clear();
                g.step = 2;
                match arg {
                    "usb" => then = Some(refresh_ports),
                    "artnet" => then = Some(find_nodes),
                    _ => g.target = od_i18n::tr("sACN multicast").to_string(),
                }
            }
            "node" => {
                if let Some(n) = arg.parse::<usize>().ok().and_then(|i| rt.nodes.get(i)) {
                    g.target = n.target().to_string();
                    g.node_ports = n.universes.clone();
                    if let Some(p) = n.universes.first() {
                        g.universe = (p + 1).min(MAX_UNIVERSES);
                    }
                }
            }
            "port" => {
                if let Some(p) = arg.parse::<usize>().ok().and_then(|i| rt.ports.get(i)) {
                    g.target = p.clone();
                }
            }
            "target" => match od_lighting::parse_output("artnet", arg) {
                Ok(OutputKind::ArtNet { target, .. }) if !arg.trim().is_empty() => {
                    g.target = target.to_string();
                    g.node_ports.clear();
                }
                _ => g.note = od_i18n::trf("Not an address: {}", &[&arg.trim()]),
            },
            "next" if g.step == 2 && !g.target.is_empty() => g.step = 3,
            "back" => {
                if g.step == 4 {
                    test = Some(None);
                }
                g.step = (g.step - 1).max(1);
            }
            "universe" => {
                let d: i32 = arg.parse().unwrap_or(0);
                g.universe = (g.universe as i32 + d).clamp(1, MAX_UNIVERSES as i32) as u16;
            }
            "apply" if g.step == 3 => {
                let u = g.universe - 1;
                let kind = match g.kind.as_str() {
                    "artnet" => g.target.parse().ok().map(|target| OutputKind::ArtNet {
                        target,
                        universe: if g.node_ports.contains(&u) || g.node_ports.is_empty() {
                            u
                        } else {
                            g.node_ports[0]
                        },
                    }),
                    "usb" => Some(OutputKind::EnttecPro {
                        device: g.target.clone(),
                    }),
                    _ => Some(OutputKind::Sacn { universe: u + 1 }),
                };
                match kind {
                    Some(k) => {
                        output = Some((u, k));
                        test = Some(Some(TestPattern::Each));
                        g.step = 4;
                    }
                    None => g.note = od_i18n::trf("Not an address: {}", &[&g.target]),
                }
            }
            "works" => {
                test = Some(None);
                g.step = 5;
            }
            "fails" => {
                g.note = od_i18n::tr(
                    "Check, in this order: the fixtures are on and set to DMX; their universe and address match the patch table; the cable runs from the interface to the first fixture and on from there, with a terminator at the end if you have one; for a node, its universe matches and it is on this computer’s network. The OUTPUT line above the guide says if the link itself is lost.",
                )
                .to_string();
            }
            _ => {}
        }
    }
    if let Some((u, k)) = output {
        crate::lighting::set_output(app, u, k);
    }
    if let Some(t) = test {
        set_test(app, t);
    }
    refresh(app);
    if let Some(f) = then {
        f(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_labels_and_previews() {
        assert_eq!(pad_number(0), "1.1");
        assert_eq!(pad_number(10), "2.3");
        let mut red = PadLook::new(0, "Red");
        red.groups.insert(
            "Wash".into(),
            GroupSettings {
                color: Some([1.0, 0.0, 0.0]),
                intensity: Some(0.5),
                ..Default::default()
            },
        );
        red.groups.insert(
            "Movers".into(),
            GroupSettings {
                program: Some(Program::Blackout),
                ..Default::default()
            },
        );
        let mut chase = PadLook::new(9, "Run");
        chase.steps = vec![0, 0];
        let pads = vec![red.clone(), chase.clone()];
        let groups = vec!["Movers".to_string(), "Wash".to_string()];
        let c = preview_colors(&red, &pads, &groups);
        assert_eq!(c.len(), 2);
        assert!(c[0][0] < 0.2, "movers dark first, in group order");
        assert_eq!(c[1], [0.5, 0.0, 0.0]);
        assert_eq!(preview_colors(&chase, &pads, &groups).len(), 2);
        assert_eq!(
            pick_label(AutoPick::Pad(0), &pads),
            "1.1 RED",
            "a pad pick names the pad"
        );
        assert_eq!(
            pick_label(AutoPick::Program(Program::Sweep), &pads),
            "SWEEP"
        );
        let node = ArtNode {
            ip: std::net::Ipv4Addr::new(192, 0, 2, 10),
            short_name: "A".into(),
            long_name: String::new(),
            universes: vec![4, 5],
            from: "127.0.0.1:1".parse().unwrap(),
        };
        assert_eq!(
            node_output(&node, 5),
            OutputKind::ArtNet {
                target: "192.0.2.10:6454".parse().unwrap(),
                universe: 5
            }
        );
        assert!(matches!(
            node_output(&node, 0),
            OutputKind::ArtNet { universe: 4, .. }
        ));
    }
}
