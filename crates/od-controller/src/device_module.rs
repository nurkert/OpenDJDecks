//! Optional per-device Rust hooks for behaviour a TOML mapping cannot express.
//!
//! A mapping selects a module with `device.module = "<id>"`. The module sees
//! every raw message before the mapping (and may consume it), and every
//! action after the mapping (and may rewrite, drop or add actions).

use crate::action::{ControlAction, DeckAction, JogMode};
use crate::mapping::Mapping;
use std::collections::HashMap;

/// Jog steps per platter revolution the application expects
/// (`DeckAction::JogTurn` ticks): every jog module scales its unit to this.
pub const JOG_TICKS_PER_REVOLUTION: u32 = 720;

/// Context passed to module hooks.
pub struct ModuleContext<'a> {
    pub mapping: &'a Mapping,
}

pub trait DeviceModule: Send {
    fn id(&self) -> &'static str;

    /// Raw input hook. Return `true` to consume the message (the mapping
    /// will not see it). Actions pushed to `out` are emitted.
    fn on_raw(
        &mut self,
        _ctx: &ModuleContext,
        _bytes: &[u8],
        _out: &mut Vec<ControlAction>,
    ) -> bool {
        false
    }

    /// Action hook: push the (possibly transformed) action(s) to `out`.
    fn on_action(
        &mut self,
        _ctx: &ModuleContext,
        action: ControlAction,
        out: &mut Vec<ControlAction>,
    ) {
        out.push(action);
    }
}

/// Module that does nothing; used when a mapping names none.
#[derive(Debug, Default)]
pub struct NoModule;

impl DeviceModule for NoModule {
    fn id(&self) -> &'static str {
        "none"
    }
}

/// Shared behaviour for jog-wheel controllers.
///
/// - Tempo centre detent: positions within `tempo_detent` of the centre snap
///   to exactly 0 so "tempo 0 %" is reachable despite 14-bit jitter.
/// - Jog resolution: the unit's steps per revolution
///   (`options.jog_ticks_per_revolution`) are scaled to
///   [`JOG_TICKS_PER_REVOLUTION`], so one turn of the platter moves the track
///   by one revolution on every controller. Remainders carry over per deck and
///   kind of turn, so slow turns are not lost.
/// - Jog search acceleration: shift-jog search ticks are multiplied by
///   `search_gain`, and bend ticks are scaled by `bend_gain`.
#[derive(Debug)]
pub struct JogControllerModule {
    pub tempo_detent: f32,
    pub search_gain: i32,
    pub bend_gain: f32,
    remainder: HashMap<(u8, JogMode), f32>,
}

impl Default for JogControllerModule {
    fn default() -> Self {
        Self {
            tempo_detent: 0.004,
            // The application speeds search up itself.
            search_gain: 1,
            bend_gain: 1.0,
            remainder: HashMap::new(),
        }
    }
}

impl DeviceModule for JogControllerModule {
    fn id(&self) -> &'static str {
        "jog-controller"
    }

    fn on_action(
        &mut self,
        ctx: &ModuleContext,
        action: ControlAction,
        out: &mut Vec<ControlAction>,
    ) {
        let per_rev = ctx.mapping.options().jog_ticks_per_revolution.max(1);
        let unit = JOG_TICKS_PER_REVOLUTION as f32 / per_rev as f32;
        match action {
            ControlAction::Deck {
                deck,
                action: DeckAction::Tempo(v),
            } => {
                let v = if v.abs() < self.tempo_detent { 0.0 } else { v };
                out.push(ControlAction::Deck {
                    deck,
                    action: DeckAction::Tempo(v),
                });
            }
            ControlAction::Deck {
                deck,
                action: DeckAction::JogTurn { ticks, mode },
            } => {
                let gain = match mode {
                    JogMode::Search => self.search_gain as f32,
                    JogMode::Bend => self.bend_gain,
                    JogMode::Scratch => 1.0,
                };
                let rem = self.remainder.entry((deck, mode)).or_default();
                let f = ticks as f32 * gain * unit + *rem;
                let t = f.trunc();
                *rem = f - t;
                let ticks = t as i32;
                if ticks != 0 {
                    out.push(ControlAction::Deck {
                        deck,
                        action: DeckAction::JogTurn { ticks, mode },
                    });
                }
            }
            a => out.push(a),
        }
    }
}

/// Instantiates the module a mapping asks for.
pub fn module_for(id: Option<&str>) -> Box<dyn DeviceModule> {
    match id {
        Some("jog-controller") => Box::new(JogControllerModule::default()),
        Some(other) => {
            tracing::warn!("unknown device module `{other}`, using none");
            Box::new(NoModule)
        }
        None => Box::new(NoModule),
    }
}
