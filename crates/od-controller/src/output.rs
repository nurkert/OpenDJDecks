//! Feedback → MIDI rendering with diffing, software blinking and rate limits.
//!
//! The application pushes [`ControlFeedback`] whenever it likes; nothing is
//! sent immediately. [`OutputEngine::poll`] is called periodically (e.g.
//! every 5–10 ms) and returns only the messages whose byte value actually
//! changed since it was last sent, honouring the per-output minimum interval
//! (meters, jog display positions) and a global per-poll budget. A 14-bit
//! value output is one unit: when either byte changes, MSB and LSB both go
//! out, MSB first.

use crate::feedback::{ControlFeedback, FeedbackValue, LedState};
use crate::mapping::{Mapping, OutputBinding, OutputMode};
use crate::midi::MidiAddr;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Render {
    Static(u8),
    Blink { on: u8, off: u8, period_ms: u32 },
    /// MSB and LSB of a 14-bit value.
    Pair(u8, u8),
}

#[derive(Clone, Copy, Debug)]
struct AddrState {
    desired: Render,
    min_interval: Duration,
    /// Level and value outputs are lower priority than LEDs within the poll
    /// budget.
    is_level: bool,
    /// LSB address of a 14-bit value.
    lsb: Option<MidiAddr>,
    /// Last sent byte (14-bit: MSB << 7 | LSB).
    sent: Option<u16>,
    sent_at: Option<Instant>,
}

pub struct OutputEngine {
    mapping: Arc<Mapping>,
    /// Last value per output binding (kept even while hidden by a layer).
    values: Vec<Option<FeedbackValue>>,
    addrs: BTreeMap<MidiAddr, AddrState>,
    /// Active deck per side (software layers).
    active: HashMap<u8, u8>,
    epoch: Instant,
}

impl OutputEngine {
    pub fn new(mapping: Arc<Mapping>) -> Self {
        let n = mapping.outputs.len();
        let mut active = HashMap::new();
        if let Some(l) = &mapping.file.layers {
            for (i, side) in l.sides.iter().enumerate() {
                active.insert(i as u8 + 1, side[0]);
            }
        }
        Self {
            mapping,
            values: vec![None; n],
            addrs: BTreeMap::new(),
            active,
            epoch: Instant::now(),
        }
    }

    /// Sets the blink phase reference (for deterministic tests).
    pub fn set_epoch(&mut self, t: Instant) {
        self.epoch = t;
    }

    fn is_visible(&self, b: &OutputBinding) -> bool {
        match b.active_when {
            Some((side, deck)) => self.active.get(&side) == Some(&deck),
            None => true,
        }
    }

    fn render(&self, b: &OutputBinding, v: Option<FeedbackValue>) -> Render {
        let opts = self.mapping.options();
        let led = |state: LedState, on: u8| match state {
            LedState::Off => Render::Static(b.off),
            LedState::On => Render::Static(on),
            LedState::Dim => Render::Static(b.dim.unwrap_or(b.off)),
            LedState::Blink | LedState::BlinkFast => match b.blink {
                Some(hw) => Render::Static(hw),
                None => Render::Blink {
                    on,
                    off: b.off,
                    period_ms: if state == LedState::Blink {
                        opts.blink_ms
                    } else {
                        opts.blink_fast_ms
                    },
                },
            },
        };
        let lit = |v: FeedbackValue| match v {
            FeedbackValue::Led(s) => s != LedState::Off,
            FeedbackValue::Level(l) => l >= 0.5,
            FeedbackValue::Color { color, state } => color.is_some() && state != LedState::Off,
            FeedbackValue::Number(n) => n != 0,
        };
        match (b.mode, v) {
            (OutputMode::Value14, None) => Render::Pair(b.off, b.off),
            (_, None) => Render::Static(b.off),
            (OutputMode::Value, Some(FeedbackValue::Number(n))) => {
                let hi = b.max.min(0x7F).max(b.min) as i32;
                Render::Static(n.saturating_add(b.offset).clamp(b.min as i32, hi) as u8)
            }
            (OutputMode::Value14, Some(FeedbackValue::Number(n))) => {
                let v = n.saturating_add(b.offset).clamp(0, 0x3FFF);
                Render::Pair((v >> 7) as u8, (v & 0x7F) as u8)
            }
            // Anything else on a value output shows as on / off (e.g. a
            // hidden marker).
            (OutputMode::Value, Some(v)) => Render::Static(if lit(v) { b.on } else { b.off }),
            (OutputMode::Value14, Some(v)) => {
                let x = if lit(v) { b.on } else { b.off };
                Render::Pair(x, x)
            }
            (_, Some(FeedbackValue::Number(n))) => led(LedState::from_bool(n != 0), b.on),
            (OutputMode::Level, Some(FeedbackValue::Level(l))) => {
                let l = l.clamp(0.0, 1.0);
                Render::Static((b.min as f32 + l * (b.max as f32 - b.min as f32)).round() as u8)
            }
            (OutputMode::Level, Some(FeedbackValue::Led(s))) => led(s, b.max),
            (_, Some(FeedbackValue::Level(l))) => led(LedState::from_bool(l >= 0.5), b.on),
            (_, Some(FeedbackValue::Led(s))) => led(s, b.on),
            (_, Some(FeedbackValue::Color { color, state })) => match color {
                None => Render::Static(b.off),
                Some(c) => {
                    let on = match &b.palette {
                        Some(p) if !p.is_empty() => {
                            let colors: Vec<_> = p.iter().map(|(c, _)| *c).collect();
                            p[c.nearest(&colors)].1
                        }
                        _ => b.on,
                    };
                    led(state, on)
                }
            },
        }
    }

    fn refresh_binding(&mut self, idx: usize) {
        let b = &self.mapping.outputs[idx];
        if !self.is_visible(b) {
            return;
        }
        let desired = self.render(b, self.values[idx]);
        let min_interval = Duration::from_millis(b.min_interval_ms as u64);
        let is_level = matches!(
            b.mode,
            OutputMode::Level | OutputMode::Value | OutputMode::Value14
        );
        let st = self.addrs.entry(b.addr).or_insert(AddrState {
            desired,
            min_interval,
            is_level,
            lsb: b.lsb,
            sent: None,
            sent_at: None,
        });
        st.desired = desired;
        st.min_interval = min_interval;
        st.is_level = is_level;
        st.lsb = b.lsb;
    }

    /// Applies one feedback message.
    pub fn apply(&mut self, fb: &ControlFeedback) {
        match fb {
            ControlFeedback::Set { target, value } => {
                let idxs: Vec<usize> = self.mapping.outputs_for(target).to_vec();
                for i in idxs {
                    self.values[i] = Some(*value);
                    self.refresh_binding(i);
                }
            }
            ControlFeedback::Parameter { .. } => {}
            ControlFeedback::ResendAll => {
                for st in self.addrs.values_mut() {
                    st.sent = None;
                    st.sent_at = None;
                }
            }
            ControlFeedback::AllOff => {
                for v in self.values.iter_mut() {
                    *v = None;
                }
                for i in 0..self.mapping.outputs.len() {
                    self.refresh_binding(i);
                }
            }
        }
    }

    /// Initialises every output to "off" so the first poll clears the
    /// controller's LEDs.
    pub fn reset(&mut self) {
        self.addrs.clear();
        for i in 0..self.mapping.outputs.len() {
            self.refresh_binding(i);
        }
    }

    /// A button at `addr` was pressed or released on the unit. Some units
    /// light or darken a pressed button themselves; its LED is sent again on
    /// the next poll so it shows the application's state, not the unit's.
    pub fn touched(&mut self, addr: MidiAddr) {
        if let Some(st) = self.addrs.get_mut(&addr) {
            st.sent = None;
            st.sent_at = None;
        }
    }

    /// Software deck layer switch: `side` now shows `deck`.
    pub fn set_active_deck(&mut self, side: u8, deck: u8) {
        if self.active.insert(side, deck) == Some(deck) {
            return;
        }
        for i in 0..self.mapping.outputs.len() {
            if self.mapping.outputs[i].active_when == Some((side, deck)) {
                self.refresh_binding(i);
            }
        }
    }

    /// Returns the messages to send now.
    pub fn poll(&mut self, now: Instant) -> Vec<[u8; 3]> {
        let budget = self.mapping.options().max_messages_per_poll;
        let elapsed_ms = now.saturating_duration_since(self.epoch).as_millis() as u64;
        let mut out = Vec::new();
        for pass_levels in [false, true] {
            for (addr, st) in self.addrs.iter_mut() {
                if st.is_level != pass_levels {
                    continue;
                }
                let (value, lsb) = match st.desired {
                    Render::Static(v) => (v, None),
                    Render::Blink { on, off, period_ms } => {
                        let half = (period_ms / 2).max(1) as u64;
                        if (elapsed_ms / half).is_multiple_of(2) {
                            (on, None)
                        } else {
                            (off, None)
                        }
                    }
                    Render::Pair(msb, lsb) => (msb, st.lsb.map(|a| (a, lsb))),
                };
                let key = match lsb {
                    Some((_, l)) => (value as u16) << 7 | l as u16,
                    None => value as u16,
                };
                if st.sent == Some(key) {
                    continue;
                }
                if let (Some(at), Some(_)) = (st.sent_at, st.sent)
                    && now.saturating_duration_since(at) < st.min_interval
                {
                    continue;
                }
                if out.len() + 1 + lsb.is_some() as usize > budget {
                    // Out of budget: stays dirty and goes out on the next poll.
                    continue;
                }
                st.sent = Some(key);
                st.sent_at = Some(now);
                out.push(addr.message(value));
                if let Some((a, l)) = lsb {
                    out.push(a.message(l));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::{DeckElement, FeedbackTarget};

    const MAP: &str = r#"
schema = 1
[device]
id = "test"
name = "Test"
vendor = "Test"
model = "Test"
decks = 2
[device.match]
port_names = ["TEST"]
[channels]
deck = [0, 1]
[[output]]
target = "deck.bpm"
cc = 0x14
mode = "value14"
channel = 15
deck = 1
[[output]]
target = "deck.cue_angle"
cc = 0x1C
lsb = 0x3C
off = 0x7F
channel = 15
deck = 1
[[output]]
target = "deck.time_minutes"
cc = 0x42
channel = 15
deck = 1
[[output]]
target = "deck.key_shift"
cc = 0x4A
offset = 13
channel = "deck"
[[output]]
target = "deck.position_angle"
cc = 0x10
mode = "value14"
min_interval_ms = 33
channel = 15
deck = 1
"#;

    fn engine() -> OutputEngine {
        let mut e = OutputEngine::new(Arc::new(Mapping::from_toml_str(MAP).unwrap()));
        e.reset();
        e
    }

    fn set(e: &mut OutputEngine, element: DeckElement, n: i32) {
        e.apply(&ControlFeedback::number(
            FeedbackTarget::Deck { deck: 1, element },
            n,
        ));
    }

    #[test]
    fn value14_sends_msb_then_lsb() {
        let mut e = engine();
        let t0 = Instant::now();
        // Reset: BPM 0/0, the cue marker hidden (`off` in both bytes).
        let first = e.poll(t0);
        assert!(first.contains(&[0xBF, 0x14, 0x00]) && first.contains(&[0xBF, 0x34, 0x00]));
        let cue = first.iter().position(|m| m == &[0xBF, 0x1C, 0x7F]).unwrap();
        assert_eq!(first[cue + 1], [0xBF, 0x3C, 0x7F]);
        set(&mut e, DeckElement::Bpm, 1280);
        assert_eq!(e.poll(t0), vec![[0xBF, 0x14, 0x0A], [0xBF, 0x34, 0x00]]);
        // Only the LSB changed: both bytes still go out, MSB first.
        set(&mut e, DeckElement::Bpm, 1281);
        assert_eq!(e.poll(t0), vec![[0xBF, 0x14, 0x0A], [0xBF, 0x34, 0x01]]);
        set(&mut e, DeckElement::Bpm, 1281);
        assert!(e.poll(t0).is_empty());
        // Clamped to 14 bits.
        set(&mut e, DeckElement::Bpm, 99_999);
        assert_eq!(e.poll(t0), vec![[0xBF, 0x14, 0x7F], [0xBF, 0x34, 0x7F]]);
        set(&mut e, DeckElement::CueAngle, 359);
        assert_eq!(e.poll(t0), vec![[0xBF, 0x1C, 0x02], [0xBF, 0x3C, 0x67]]);
        // No cue point: hidden again.
        e.apply(&ControlFeedback::led(
            FeedbackTarget::Deck {
                deck: 1,
                element: DeckElement::CueAngle,
            },
            false,
        ));
        assert_eq!(e.poll(t0), vec![[0xBF, 0x1C, 0x7F], [0xBF, 0x3C, 0x7F]]);
    }

    #[test]
    fn value_sends_one_clamped_byte_with_offset() {
        let mut e = engine();
        let t0 = Instant::now();
        e.poll(t0);
        set(&mut e, DeckElement::TimeMinutes, 3);
        assert_eq!(e.poll(t0), vec![[0xBF, 0x42, 0x03]]);
        set(&mut e, DeckElement::TimeMinutes, 300);
        assert_eq!(e.poll(t0), vec![[0xBF, 0x42, 0x7F]]);
        set(&mut e, DeckElement::TimeMinutes, -4);
        assert_eq!(e.poll(t0), vec![[0xBF, 0x42, 0x00]]);
        // Key shift: semitones + 13 (0x0D = no shift).
        set(&mut e, DeckElement::KeyShift, 0);
        assert_eq!(e.poll(t0), vec![[0xB0, 0x4A, 0x0D]]);
        set(&mut e, DeckElement::KeyShift, -12);
        assert_eq!(e.poll(t0), vec![[0xB0, 0x4A, 0x01]]);
    }

    #[test]
    fn value14_honours_the_rate_limit() {
        let mut e = engine();
        let t0 = Instant::now();
        e.poll(t0);
        let t = |ms| t0 + Duration::from_millis(ms);
        set(&mut e, DeckElement::PositionAngle, 10);
        assert_eq!(e.poll(t(40)), vec![[0xBF, 0x10, 0x00], [0xBF, 0x30, 0x0A]]);
        set(&mut e, DeckElement::PositionAngle, 20);
        assert!(e.poll(t(50)).is_empty(), "within 33 ms");
        set(&mut e, DeckElement::PositionAngle, 200);
        assert_eq!(e.poll(t(80)), vec![[0xBF, 0x10, 0x01], [0xBF, 0x30, 0x48]]);
    }
}
