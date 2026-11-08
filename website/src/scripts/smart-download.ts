// The hero button names the visitor's system. Without JavaScript, on phones or on an unknown
// system it stays neutral.
import { detectOs } from './platform';

const os = detectOs();
if (os) {
  for (const link of document.querySelectorAll<HTMLElement>('[data-smart-download]')) {
    const label = link.querySelector<HTMLElement>('[data-smart-label]');
    const text = link.dataset[`label${os[0].toUpperCase()}${os.slice(1)}`];
    if (label && text) label.textContent = text;
  }
}
