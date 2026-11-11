//! Where fixtures hang and how each one is tuned (roadmap [32]).
//!
//! * [`Placement`]: a fixture's place on a top-down floor plan (0..1 across
//!   and from the back to the front of the stage), its rotation and height.
//!   Unplaced fixtures get a default spot ([`positions`]): movers on the
//!   back truss, bars and effects in the middle, washes at the front.
//! * [`stage_order`]: the order of fixtures from left to right, used by
//!   chases, sweeps and the phase spread of effects.
//! * [`Tune`]: per-fixture overrides applied when a look is rendered to
//!   DMX: pan and tilt ranges, tilt invert, a colour calibration and a
//!   dimmer curve. They act before the safety limits, which still hold.

use serde::{Deserialize, Serialize};

use crate::fixture::{FixtureRole, Patch};

/// Grid of the floor plan: positions snap to 1/[`GRID`] of the stage.
pub const GRID: f32 = 20.0;

/// A fixture's place on the floor plan, seen from above with the audience
/// at the bottom.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    /// 0 = stage left edge, 1 = right edge (as the audience sees it).
    pub x: f32,
    /// 0 = the back of the stage, 1 = the front edge.
    pub y: f32,
    /// Degrees clockwise; 0 points at the audience.
    #[serde(default)]
    pub rotation: f32,
    /// Height above the floor, metres.
    #[serde(default = "default_height")]
    pub height: f32,
}

fn default_height() -> f32 {
    3.0
}

impl Placement {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            rotation: 0.0,
            height: default_height(),
        }
    }

    /// Snapped to the grid.
    pub fn snapped(mut self) -> Self {
        self.x = snap(self.x);
        self.y = snap(self.y);
        self
    }

    /// Rotation kept in 0..360.
    pub fn rotate(&mut self, degrees: f32) {
        self.rotation = (self.rotation + degrees).rem_euclid(360.0);
    }

    /// Pointing away from the audience (a bar's cells then run the other
    /// way across the stage).
    pub fn reversed(&self) -> bool {
        let r = self.rotation.rem_euclid(360.0);
        r > 90.0 && r < 270.0
    }
}

/// A coordinate snapped to the floor plan grid.
pub fn snap(v: f32) -> f32 {
    ((v.clamp(0.0, 1.0) * GRID).round() / GRID).clamp(0.0, 1.0)
}

/// Where each fixture stands on the floor plan: its placement, or the
/// default spot for its role (movers on the back truss, bars and effects
/// in the middle, washes at the front, spread evenly across).
pub fn positions(patch: &Patch) -> Vec<(f32, f32)> {
    let row = |r: FixtureRole| match r {
        FixtureRole::Mover => 0,
        FixtureRole::Bar | FixtureRole::Effect => 1,
        FixtureRole::Wash => 2,
    };
    let mut counts = [0usize; 3];
    for f in &patch.fixtures {
        counts[row(f.def.role())] += 1;
    }
    let mut seen = [0usize; 3];
    patch
        .fixtures
        .iter()
        .map(|f| {
            let r = row(f.def.role());
            let i = seen[r];
            seen[r] += 1;
            match f.place {
                Some(p) => (p.x, p.y),
                None => ((i as f32 + 0.5) / counts[r] as f32, [0.18, 0.5, 0.82][r]),
            }
        })
        .collect()
}

/// For each fixture its rank from left to right among the fixtures for
/// which `same` holds (stage position; equal places keep the patch order).
pub fn stage_rank(patch: &Patch, same: impl Fn(usize, usize) -> bool) -> Vec<usize> {
    let pos = positions(patch);
    (0..patch.fixtures.len())
        .map(|i| {
            (0..patch.fixtures.len())
                .filter(|&j| j != i && same(i, j))
                .filter(|&j| pos[j].0 < pos[i].0 || (pos[j].0 == pos[i].0 && j < i))
                .count()
        })
        .collect()
}

/// Fixture indices from stage left to stage right.
pub fn stage_order(patch: &Patch) -> Vec<usize> {
    let rank = stage_rank(patch, |_, _| true);
    let mut order: Vec<usize> = (0..patch.fixtures.len()).collect();
    order.sort_by_key(|&i| rank[i]);
    order
}

/// How a fixture's dimmer follows the level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimmerCurve {
    /// As sent.
    #[default]
    Linear,
    /// Darker low end (level squared): for fixtures that jump up early.
    Square,
    /// Brighter low end (square root): for fixtures that stay dark too long.
    Root,
    /// Soft at both ends (smoothstep).
    Soft,
}

impl DimmerCurve {
    pub const ALL: [DimmerCurve; 4] = [
        DimmerCurve::Linear,
        DimmerCurve::Square,
        DimmerCurve::Root,
        DimmerCurve::Soft,
    ];

    pub fn apply(self, v: f32) -> f32 {
        let v = v.clamp(0.0, 1.0);
        match self {
            DimmerCurve::Linear => v,
            DimmerCurve::Square => v * v,
            DimmerCurve::Root => v.sqrt(),
            DimmerCurve::Soft => v * v * (3.0 - 2.0 * v),
        }
    }
}

fn full_range() -> [f32; 2] {
    [0.0, 1.0]
}

/// Per-fixture programming, applied when a look is rendered.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tune {
    /// The part of the pan travel a look's 0..1 is spread over.
    #[serde(default = "full_range")]
    pub pan: [f32; 2],
    /// The part of the tilt travel a look's 0..1 is spread over.
    #[serde(default = "full_range")]
    pub tilt: [f32; 2],
    /// Mirror tilt (a fixture hung upside down).
    #[serde(default)]
    pub invert_tilt: bool,
    /// Colour calibration: red, green and blue raised or lowered by this
    /// share (−0.5..0.5) so fixtures of different makes match.
    #[serde(default)]
    pub color: [f32; 3],
    #[serde(default)]
    pub curve: DimmerCurve,
}

impl Default for Tune {
    fn default() -> Self {
        Self {
            pan: full_range(),
            tilt: full_range(),
            invert_tilt: false,
            color: [0.0; 3],
            curve: DimmerCurve::Linear,
        }
    }
}

impl Tune {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// A look's pan (0..1) on this fixture.
    pub fn pan(&self, v: f32) -> f32 {
        lerp(self.pan, v)
    }

    /// A look's tilt (0..1) on this fixture.
    pub fn tilt(&self, v: f32) -> f32 {
        let v = if self.invert_tilt { 1.0 - v } else { v };
        lerp(self.tilt, v)
    }

    /// A colour after the calibration.
    pub fn color(&self, c: [f32; 3]) -> [f32; 3] {
        [0, 1, 2].map(|i| (c[i] * (1.0 + self.color[i].clamp(-0.5, 0.5))).clamp(0.0, 1.0))
    }
}

fn lerp(range: [f32; 2], v: f32) -> f32 {
    let (a, b) = (range[0].clamp(0.0, 1.0), range[1].clamp(0.0, 1.0));
    (a + (b - a) * v.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::FixtureDef;

    #[test]
    fn placement_snaps_and_orders_left_to_right() {
        let mut patch = Patch::default();
        let par = FixtureDef::builtin("rgbw-par").unwrap();
        patch.add_many(&par, 3, Some(0), Some(1), "Par", "Wash").unwrap();
        // Default spots spread in patch order.
        assert_eq!(stage_order(&patch), vec![0, 1, 2]);
        // Placed: the first one goes to the right edge.
        patch.fixtures[0].place = Some(Placement::new(0.93, 0.5).snapped());
        assert_eq!(patch.fixtures[0].place.unwrap().x, 0.95);
        assert_eq!(stage_order(&patch), vec![1, 2, 0]);
        let mut p = Placement::new(0.0, 0.0);
        p.rotate(-90.0);
        assert_eq!(p.rotation, 270.0);
        p.rotate(-90.0);
        assert!(p.reversed());
    }

    #[test]
    fn tune_ranges_invert_colour_and_curves() {
        let t = Tune {
            pan: [0.25, 0.75],
            tilt: [0.0, 0.5],
            invert_tilt: true,
            color: [-0.5, 0.0, 0.5],
            curve: DimmerCurve::Square,
        };
        assert_eq!(t.pan(0.0), 0.25);
        assert_eq!(t.pan(1.0), 0.75);
        assert_eq!(t.tilt(1.0), 0.0);
        assert_eq!(t.tilt(0.0), 0.5);
        assert_eq!(t.color([1.0, 1.0, 0.5]), [0.5, 1.0, 0.75]);
        assert_eq!(t.curve.apply(0.5), 0.25);
        assert_eq!(DimmerCurve::Root.apply(0.25), 0.5);
        assert_eq!(DimmerCurve::Soft.apply(1.0), 1.0);
        assert!(Tune::default().is_default());
        // Old rigs without these fields read with the defaults.
        let old: Tune = serde_json::from_str("{}").unwrap();
        assert!(old.is_default());
    }
}
