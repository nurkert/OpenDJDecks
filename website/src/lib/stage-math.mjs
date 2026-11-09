// The numbers of the landing page's stage, free of any browser API so they can be tested
// (scripts/stage-math.test.mjs). A frame only ever writes transform and opacity.

export const clamp01 = (t) => Math.min(1, Math.max(0, t));
export const mix = (a, b, t) => a + (b - a) * t;

/** Smooth ease with zero slope at both ends. */
export const ease = (t) => {
  const x = clamp01(t);
  return x * x * x * (x * (6 * x - 15) + 10);
};

/** Below this width the stage does not pin; it shows the calm, static arrangement. */
export const liveMinWidth = 880;
export const liveMinHeight = 560;

export const liveQuery = `(min-width: ${liveMinWidth}px) and (min-height: ${liveMinHeight}px) and (prefers-reduced-motion: no-preference) and (scripting: enabled)`;

/** Length of the pinned scroll, as a share of the viewport height. */
export const scrollRange = 1.25;

/** Below the live size, without reduced motion: the windows open up on a normal scroll. */
export const softQuery = `(max-width: ${liveMinWidth - 1}px) and (prefers-reduced-motion: no-preference)`;

/**
 * Poses of the three windows, in fractions of the front window's width (W): x offset, z depth,
 * tilts in degrees, scale s. All three share one vertical line.
 * Hero: the front window leans back below the copy, the two others hide far behind it.
 * Flight: the side windows swing out with a strong yaw while the front window comes forward.
 * End: everything is flat, the front window centred, the two others fanned out to the sides.
 */
const poses = {
  perform: {
    hero: { x: 0, z: -0.18, rx: 20, s: 0.9, o: 1 },
    end: { x: 0, z: 0, rx: 0, s: 1, o: 1 },
    swing: 0,
    lift: 0.14,
  },
  prepare: {
    hero: { x: -0.05, z: -1.1, rx: 20, s: 0.86, o: 0.15 },
    end: { x: -1, z: -0.3, rx: 0, s: 0.94, o: 0.85 },
    swing: 42,
    lift: 0,
  },
  club: {
    hero: { x: 0.05, z: -1.1, rx: 20, s: 0.86, o: 0.15 },
    end: { x: 1, z: -0.3, rx: 0, s: 0.94, o: 0.85 },
    swing: -42,
    lift: 0,
  },
};

export const windows = Object.keys(poses);

/** Share of the hero's scale at which the front window starts, for placing it under the copy. */
export const heroScale = poses.perform.hero.s;

/**
 * Motion over the pinned stretch: a soft start, but the windows are still moving when the pin
 * ends, so the page hands over to normal scrolling without a moment of nothing happening.
 */
export const travel = (t) => (clamp01(t) + ease(t)) / 2;

/**
 * How far the side windows spread, in W: as far as the screen allows with a small margin, so a
 * wide screen is used in full.
 */
export function sideSpread(viewportWidth, W) {
  const margin = Math.max(16, viewportWidth * 0.02);
  return Math.min(0.62, Math.max(0.27, (viewportWidth / 2 - margin - 0.42 * W) / (0.98 * W)));
}

/**
 * The pose of a window at scroll progress t (0..1), in pixels for a front window of W wide.
 * startY and endY are the vertical offsets from the stage's centre at both ends, spread the
 * end position of the side windows in W.
 */
export function poseAt(name, t, W, startY = 0, endY = 0, spread = 0.27) {
  const p = poses[name];
  const k = travel(t);
  const arc = Math.sin(Math.PI * k);
  return {
    x: mix(p.hero.x, p.end.x * spread, k) * W,
    y: mix(startY, endY, k),
    z: (mix(p.hero.z, p.end.z, k) + p.lift * arc) * W,
    ry: p.swing * arc,
    rx: mix(p.hero.rx, p.end.rx, k),
    s: mix(p.hero.s, p.end.s, k),
    o: mix(p.hero.o, p.end.o, k),
  };
}

/** Transform of a window on a phone at scroll progress t: tucked in, then fanned open. */
export function softAt(name, t) {
  const k = ease(t);
  if (name === 'perform') return `translate3d(0, ${mix(26, 0, k).toFixed(1)}px, 0) scale(${mix(0.93, 1, k).toFixed(4)})`;
  const side = name === 'prepare' ? -1 : 1;
  return `translate3d(${(side * mix(-20, 0, k)).toFixed(2)}%, ${mix(14, 0, k).toFixed(1)}px, 0) rotate(${(side * mix(0, 6, k)).toFixed(2)}deg) scale(${mix(0.82, 0.92, k).toFixed(4)})`;
}

/** The transform of a pose, centred on the window's own position. */
export const transformOf = (p) =>
  `translate(-50%, -50%) translate3d(${p.x.toFixed(1)}px, ${p.y.toFixed(1)}px, ${p.z.toFixed(1)}px) rotateX(${p.rx.toFixed(2)}deg) rotateY(${p.ry.toFixed(2)}deg) scale(${p.s.toFixed(4)})`;

/** Scroll progress within the pinned stretch of the stage. */
export const progress = (scrollY, top, range) => (range <= 0 ? 0 : clamp01((scrollY - top) / range));

/** Hero copy leaves in the first part of the scroll. */
export const heroAt = (t) => {
  const k = ease(t / 0.4);
  return { o: 1 - k, y: -60 * k };
};

/** The glow behind the stage opens up as the windows come forward. */
export const glowAt = (t) => 0.55 + 0.45 * ease(t);
