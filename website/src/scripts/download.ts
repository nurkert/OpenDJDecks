// The download page: shows the file for the visitor's computer in the recommendation plate
// at the top. Without a usable detection the plate stays hidden and the full list remains.
import { detectArch, detectOs } from './platform';

const box = document.querySelector<HTMLElement>('[data-reco]');

(async () => {
  if (!box) return;
  const os = detectOs();
  if (!os) return;
  let items: HTMLElement[] = [];
  if (os === 'macos') {
    const arch = await detectArch();
    const arm = box.querySelector<HTMLElement>('[data-reco-item="dmgArm"]');
    const intel = box.querySelector<HTMLElement>('[data-reco-item="dmgIntel"]');
    if (arch === 'arm' && arm) items = [arm];
    else if (arch === 'x86' && intel) items = [intel];
    else items = [arm, intel].filter((el): el is HTMLElement => Boolean(el));
    box.querySelector<HTMLElement>('[data-reco-chip]')?.toggleAttribute('hidden', items.length < 2);
  } else if (os === 'windows') {
    items = [...box.querySelectorAll<HTMLElement>('[data-reco-item="zip"]')];
  } else {
    items = [...box.querySelectorAll<HTMLElement>('[data-reco-item="debAmd"], [data-reco-item="appimageX86"]')];
  }
  if (items.length === 0) return;
  for (const item of box.querySelectorAll<HTMLElement>('[data-reco-item]')) item.hidden = !items.includes(item);
  box.hidden = false;
})();
