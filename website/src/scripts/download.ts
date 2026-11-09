// The download page: opens the visitor's system, switches between systems through the quiet
// tabs and, where the browser says which chip a Mac has, leaves one button instead of two.
// Without JavaScript every system stays open, one below the other.
import { detectArch, detectOs } from './platform';

const root = document.querySelector<HTMLElement>('[data-dl]');
const nav = root?.querySelector<HTMLElement>('[data-tabs]');

if (root && nav) void start(root, nav);

async function start(rootEl: HTMLElement, navEl: HTMLElement) {
  const tabs = [...navEl.querySelectorAll<HTMLAnchorElement>('[data-tab]')];
  const panels = [...rootEl.querySelectorAll<HTMLElement>('[data-panel]')];
  const ids = tabs.map((tab) => tab.dataset.tab!);
  const os = detectOs();
  const home = os || ids[0];
  const fromHash = () => {
    const id = location.hash.slice(1);
    return ids.includes(id) ? id : home;
  };

  const place = () => {
    const tab = tabs.find((el) => el.hasAttribute('aria-current'));
    if (!tab) return;
    navEl.style.setProperty('--gx', `${tab.offsetLeft}px`);
    navEl.style.setProperty('--gw', `${tab.offsetWidth}px`);
  };

  const select = (id: string) => {
    for (const tab of tabs) {
      if (tab.dataset.tab === id) tab.setAttribute('aria-current', 'true');
      else tab.removeAttribute('aria-current');
    }
    for (const panel of panels) panel.hidden = panel.dataset.panel !== id;
    place();
  };

  if (os === 'macos') narrowMac(rootEl, await detectArch());
  rootEl.querySelector<HTMLElement>('[data-unknown]')?.toggleAttribute('hidden', Boolean(os));

  select(fromHash());
  // A link to #windows must not leave the page scrolled to where that section used to be.
  if (location.hash) window.scrollTo(0, 0);
  rootEl.setAttribute('data-ready', '');

  for (const tab of tabs) {
    tab.addEventListener('click', (event) => {
      event.preventDefault();
      rootEl.setAttribute('data-switched', '');
      history.pushState(null, '', `#${tab.dataset.tab}`);
      select(tab.dataset.tab!);
    });
  }
  window.addEventListener('popstate', () => select(fromHash()));
  window.addEventListener('hashchange', () => select(fromHash()));
  window.addEventListener('resize', place);
  document.fonts?.ready.then(place);
}

/** One button for the chip the browser reports; the other version moves under "Other versions". */
function narrowMac(rootEl: HTMLElement, arch: string) {
  const keep = arch === 'arm' ? 'dmgArm' : arch === 'x86' ? 'dmgIntel' : '';
  const buttons = [...rootEl.querySelectorAll<HTMLElement>('#macos .get__btn[data-kind]')];
  if (!keep || !buttons.some((button) => button.dataset.kind === keep)) return;
  for (const button of buttons) button.hidden = button.dataset.kind !== keep;
  rootEl.querySelector<HTMLElement>('[data-chip]')?.toggleAttribute('hidden', true);
  rootEl.querySelector<HTMLElement>('[data-more="macos"]')?.toggleAttribute('hidden', false);
}
