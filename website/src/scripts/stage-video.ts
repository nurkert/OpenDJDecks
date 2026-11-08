// The animated front window: its video starts only when the stage is visible, stops offscreen,
// and is skipped for reduced motion or Save-Data. Without files the poster picture stays.
const video = document.querySelector<HTMLVideoElement>('[data-stage-video]');

if (video) {
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const saveData = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection?.saveData;
  if (!reduced && !saveData && video.dataset.webm && video.dataset.mp4) {
    const picture = video.parentElement?.querySelector<HTMLImageElement>('img');
    if (picture?.currentSrc) video.poster = picture.currentSrc;
    for (const [src, type] of [
      [video.dataset.webm, 'video/webm'],
      [video.dataset.mp4, 'video/mp4'],
    ]) {
      const source = document.createElement('source');
      source.src = src;
      source.type = type;
      video.append(source);
    }
    let started = false;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          if (!started) {
            started = true;
            video.load();
          }
          void video.play().catch(() => undefined);
        } else {
          video.pause();
        }
      },
      { threshold: 0.2 },
    );
    video.addEventListener('playing', () => video.setAttribute('data-playing', ''));
    observer.observe(video);
  } else {
    video.remove();
  }
}
