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

/**
 * Poses of the three windows, in fractions of the front window's width (W) and height (H):
 * x / y offset, z depth, rotations in degrees, scale s.
 * Hero: the front window stands to the right of the headline, the others behind it. End: the
 * front window has come to the viewer, the two others swing out to both sides.
 */
const poses = {
  perform: {
    hero: { x: 0.3, y: 0.36, z: 0, ry: 0, rx: 12, s: 0.96, o: 1 },
    end: { x: 0, y: 0.06, z: 0, ry: 0, rx: 0, s: 1, o: 1 },
  },
  prepare: {
    hero: { x: 0.8, y: 0.1, z: -0.7, ry: -28, rx: 8, s: 0.9, o: 0.5 },
    end: { x: -0.82, y: 0.04, z: -0.72, ry: 38, rx: 0, s: 0.92, o: 0.85 },
  },
  club: {
    hero: { x: 0.68, y: 0.62, z: -0.85, ry: -22, rx: 8, s: 0.82, o: 0.42 },
    end: { x: 0.82, y: 0.04, z: -0.72, ry: -38, rx: 0, s: 0.92, o: 0.85 },
  },
};

export const windows = Object.keys(poses);

/** The pose of a window at scroll progress t (0..1), in pixels for a front window of W×H. */
export function poseAt(name, t, W, H) {
  const p = poses[name];
  const k = ease(t);
  return {
    x: mix(p.hero.x, p.end.x, k) * W,
    y: mix(p.hero.y, p.end.y, k) * H,
    z: mix(p.hero.z, p.end.z, k) * W,
    ry: mix(p.hero.ry, p.end.ry, k),
    rx: mix(p.hero.rx, p.end.rx, k),
    s: mix(p.hero.s, p.end.s, k),
    o: mix(p.hero.o, p.end.o, k),
  };
}

/** The transform of a pose, centred on the window's own position. */
export const transformOf = (p) =>
  `translate(-50%, -50%) translate3d(${p.x.toFixed(1)}px, ${p.y.toFixed(1)}px, ${p.z.toFixed(1)}px) rotateX(${p.rx.toFixed(2)}deg) rotateY(${p.ry.toFixed(2)}deg) scale(${p.s.toFixed(4)})`;

/** Scroll progress within the pinned stretch of the stage. */
export const progress = (scrollY, top, range) => (range <= 0 ? 0 : clamp01((scrollY - top) / range));

/** Hero copy leaves in the first part of the scroll. */
export const heroAt = (t) => {
  const k = ease(t / 0.55);
  return { o: 1 - k, y: -60 * k };
};

/** The glow behind the stage opens up as the windows come forward. */
export const glowAt = (t) => 0.55 + 0.45 * ease(t);
