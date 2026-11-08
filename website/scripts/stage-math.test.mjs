import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { clamp01, ease, glowAt, heroAt, liveMinHeight, liveMinWidth, liveQuery, poseAt, progress, scrollRange, transformOf, windows } from '../src/lib/stage-math.mjs';

test('progress is clamped to the pinned stretch', () => {
  assert.equal(progress(0, 100, 500), 0);
  assert.equal(progress(100, 100, 500), 0);
  assert.equal(progress(350, 100, 500), 0.5);
  assert.equal(progress(9999, 100, 500), 1);
  assert.equal(progress(10, 0, 0), 0);
});

test('ease starts and ends flat and stays in range', () => {
  assert.equal(ease(0), 0);
  assert.equal(ease(1), 1);
  assert.ok(ease(0.25) < 0.25, 'slow start');
  assert.ok(ease(0.75) > 0.75, 'slow end');
  for (let t = 0; t <= 1; t += 0.02) {
    assert.ok(ease(t) >= 0 && ease(t) <= 1);
    assert.ok(ease(t + 0.02) >= ease(t) - 1e-9, 'monotonic');
  }
});

test('all three windows exist', () => {
  assert.deepEqual(windows, ['perform', 'prepare', 'club']);
});

test('the side windows end as mirror images and stay behind the front window', () => {
  const left = poseAt('prepare', 1, 1000, 600);
  const right = poseAt('club', 1, 1000, 600);
  assert.ok(Math.abs(left.x + right.x) < 1e-6, 'x mirrored');
  assert.ok(Math.abs(left.ry + right.ry) < 1e-6, 'yaw mirrored');
  assert.ok(left.x < -100, 'ends left');
  assert.ok(right.x > 100, 'ends right');
  for (let t = 0; t <= 1; t += 0.05) {
    for (const name of ['prepare', 'club']) assert.ok(poseAt(name, t, 1000, 600).z < 0, `${name} behind the front window`);
  }
});

test('the front window ends flat, facing the viewer', () => {
  const start = poseAt('perform', 0, 1000, 600);
  const end = poseAt('perform', 1, 1000, 600);
  assert.ok(start.rx > 5, 'tilted on the floor at the start');
  assert.equal(end.rx, 0);
  assert.equal(end.ry, 0);
  assert.ok(end.s >= start.s);
});

test('the stylesheet goes live under the same condition as the script', () => {
  const css = readFileSync(new URL('../src/styles/home.css', import.meta.url), 'utf8');
  assert.ok(liveQuery.includes(`(min-width: ${liveMinWidth}px)`));
  assert.ok(liveQuery.includes(`(min-height: ${liveMinHeight}px)`));
  assert.ok(css.includes(`@media ${liveQuery} {`));
  assert.ok(css.includes(`height: ${(1 + scrollRange) * 100}svh;`));
});

test('hero leaves in the first half, the glow opens with the scroll', () => {
  assert.equal(heroAt(0).o, 1);
  assert.equal(clamp01(heroAt(0.6).o), 0);
  assert.ok(heroAt(0.25).y < 0);
  assert.ok(glowAt(1) > glowAt(0));
});

test('a transform is a single translate3d + rotations + scale string', () => {
  const css = transformOf(poseAt('perform', 0.5, 1000, 600));
  assert.match(css, /^translate\(-50%, -50%\) translate3d\([^)]+px\) rotateX\([^)]+deg\) rotateY\([^)]+deg\) scale\([^)]+\)$/);
});
