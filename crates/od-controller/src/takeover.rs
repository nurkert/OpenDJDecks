//! Soft takeover ("pickup") for absolute knobs and faders.
//!
//! When the engine value of a parameter differs from the physical position
//! of its control (after a deck layer switch or a software change) moving the
//! control must not make the value jump. The control is ignored until it comes
//! within `threshold` of the engine value or crosses it; from then on it
//! follows directly.
//!
//! The first position a control reports after connecting is taken as it is:
//! the engine's value then is only what the software had before the
//! controller was there, and a channel fader at the bottom must not be held
//! until it is pushed up to a fader the software left open.
//!
//! All values here are in the normalised 0..1 domain.

use crate::action::ParamTarget;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default)]
struct State {
    engine: Option<f32>,
    caught: bool,
    last_hw: Option<f32>,
    /// The control has a known position: it reported one, or a layer switch
    /// moved it here from another deck.
    known: bool,
}

#[derive(Debug, Default)]
pub struct SoftTakeover {
    threshold: f32,
    states: HashMap<ParamTarget, State>,
}

impl SoftTakeover {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            states: HashMap::new(),
        }
    }

    /// Feeds a new hardware position. Returns `true` if the value should be
    /// passed to the engine.
    pub fn input(&mut self, target: ParamTarget, hw: f32) -> bool {
        let thr = self.threshold;
        let st = self.states.entry(target).or_default();
        let pass = match st.engine {
            None => true,
            Some(_) if !st.known => true,
            Some(_) if st.caught => true,
            Some(e) => {
                let close = (hw - e).abs() <= thr;
                let crossed = st.last_hw.is_some_and(|l| (l - e) * (hw - e) <= 0.0);
                close || crossed
            }
        };
        st.last_hw = Some(hw);
        st.known = true;
        if pass {
            st.caught = true;
            st.engine = Some(hw);
        }
        pass
    }

    /// Reports the engine's current value. If it differs from what the
    /// control last sent, the control has to pick it up again.
    pub fn set_engine(&mut self, target: ParamTarget, value: f32) {
        let thr = self.threshold;
        let st = self.states.entry(target).or_default();
        if st.engine.is_some_and(|e| (e - value).abs() < 1e-4) {
            return;
        }
        st.engine = Some(value);
        st.caught = st.last_hw.is_some_and(|l| (l - value).abs() <= thr);
    }

    /// Forget the physical position of every parameter of `deck`, e.g. after
    /// the hardware switched deck layers.
    pub fn invalidate_deck(&mut self, deck: u8) {
        for (t, st) in self.states.iter_mut() {
            if t.deck() == Some(deck) {
                st.caught = false;
                st.last_hw = None;
                // The physical control sits where it was for the other deck.
                st.known = true;
            }
        }
    }

    /// Whether `target` currently follows the hardware.
    pub fn is_caught(&self, target: ParamTarget) -> bool {
        self.states
            .get(&target)
            .is_none_or(|s| s.engine.is_none() || s.caught)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: ParamTarget = ParamTarget::EqLow { channel: 1 };

    #[test]
    fn unknown_engine_value_passes() {
        let mut s = SoftTakeover::new(0.03);
        assert!(s.input(T, 0.9));
    }

    /// [H1] The app had the fader open before the controller was there; the
    /// physical fader at the bottom sets it at once, without pickup.
    #[test]
    fn first_position_after_connecting_wins() {
        let mut s = SoftTakeover::new(0.03);
        s.set_engine(T, 1.0);
        assert!(s.input(T, 0.0));
        assert!(s.input(T, 0.1));
    }

    #[test]
    fn far_value_is_held_until_crossed() {
        let mut s = SoftTakeover::new(0.03);
        assert!(s.input(T, 0.1)); // the control's position is known
        s.set_engine(T, 0.5); // changed in software
        assert!(!s.input(T, 0.1));
        assert!(!s.input(T, 0.3));
        assert!(s.input(T, 0.55)); // crossed 0.5
        assert!(s.input(T, 0.9)); // follows now
    }

    #[test]
    fn external_change_drops_catch() {
        let mut s = SoftTakeover::new(0.03);
        assert!(s.input(T, 0.2));
        s.set_engine(T, 0.2); // echo of our own value
        assert!(s.input(T, 0.25));
        s.set_engine(T, 0.8); // changed in software
        assert!(!s.input(T, 0.3));
        assert!(s.input(T, 0.79));
    }
}
