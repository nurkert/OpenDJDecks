// The stage: a pinned section in which the three windows rise from below the copy to a
// centred fan. One rAF loop runs only while the scroll position changes. Below the
// size limit, or with reduced motion, no script runs and the CSS arrangement stays.
import { glowAt, heroAt, heroScale, liveQuery, poseAt, progress, transformOf, windows } from '../lib/stage-math.mjs';

const scene = document.querySelector<HTMLElement>('[data-scene]');
const front = scene?.querySelector<HTMLElement>('[data-win="perform"]');

if (scene && front) start(scene, front);

function start(sceneEl: HTMLElement, frontEl: HTMLElement) {
  const elements = new Map(windows.map((name) => [name, sceneEl.querySelector<HTMLElement>(`[data-win="${name}"]`)!]));
  const hero = sceneEl.querySelector<HTMLElement>('[data-hero]');
  const glow = sceneEl.querySelector<HTMLElement>('[data-glow]');
  const media = window.matchMedia(liveQuery);
  let W = 0;
  let startY = 0;
  let endY = 0;
  let top = 0;
  let range = 0;
  let live = false;
  let scheduled = false;

  const measure = () => {
    live = media.matches;
    if (!live) {
      elements.forEach((el) => el.removeAttribute('style'));
      hero?.removeAttribute('style');
      glow?.removeAttribute('style');
      return;
    }
    W = frontEl.offsetWidth;
    const stageH = (frontEl.offsetParent as HTMLElement).offsetHeight;
    const heroBottom = hero ? hero.offsetTop + hero.offsetHeight : stageH * 0.4;
    // Under the copy, but always with a good part of the window in view.
    const windowTop = Math.min(heroBottom + stageH * 0.045, stageH * 0.66);
    startY = windowTop + (frontEl.offsetHeight * heroScale) / 2 - stageH / 2;
    // Centred in the space below the navigation bar.
    endY = (hero?.offsetTop ?? 0) / 2;
    const rect = sceneEl.getBoundingClientRect();
    top = rect.top + window.scrollY;
    range = rect.height - window.innerHeight;
    paint();
    sceneEl.setAttribute('data-stage-ready', '');
  };

  function paint() {
    scheduled = false;
    if (!live) return;
    const t = progress(window.scrollY, top, range);
    for (const name of windows) {
      const pose = poseAt(name, t, W, startY, endY);
      const el = elements.get(name)!;
      el.style.transform = transformOf(pose);
      el.style.opacity = String(pose.o);
    }
    if (hero) {
      const h = heroAt(t);
      hero.style.opacity = String(h.o);
      hero.style.transform = `translate3d(0, ${h.y.toFixed(1)}px, 0)`;
      hero.style.visibility = h.o <= 0.01 ? 'hidden' : '';
    }
    if (glow) glow.style.opacity = String(glowAt(t));
  }

  const schedule = () => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(paint);
  };

  measure();
  window.addEventListener('scroll', schedule, { passive: true });
  window.addEventListener('resize', measure);
  media.addEventListener('change', measure);
  // Layout shifts from the fonts arriving must not leave the numbers stale.
  document.fonts?.ready.then(measure);
}
