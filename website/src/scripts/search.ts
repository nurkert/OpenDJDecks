// The wiki search: loads MiniSearch and the language's index on first focus, then answers
// instantly as you type. `/` focuses the field, arrows move through the results, Enter opens.
import { dedupeByPage, miniSearchOptions } from '../lib/search-core.mjs';

type MiniSearchType = import('minisearch').default<{ id: number }>;

interface Stored {
  title: string;
  heading: string;
  href: string;
  group: string;
  excerpt: string;
}

for (const root of document.querySelectorAll<HTMLElement>('[data-search]')) setup(root);

// A single global `/` key for the first visible search field of the page.
window.addEventListener('keydown', (event) => {
  if (event.key !== '/' || event.ctrlKey || event.metaKey || event.altKey) return;
  const target = event.target as HTMLElement | null;
  if (target?.isContentEditable || (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))) return;
  const input = [...document.querySelectorAll<HTMLInputElement>('[data-search] input')].find((el) => el.offsetParent !== null);
  if (input) {
    event.preventDefault();
    input.focus();
  }
});

function setup(root: HTMLElement) {
  const input = root.querySelector<HTMLInputElement>('input')!;
  const list = root.querySelector<HTMLElement>('[data-search-results]')!;
  const status = root.querySelector<HTMLElement>('[data-search-status]')!;
  const lang = root.dataset.lang ?? 'de';
  let index: MiniSearchType | null = null;
  let loading: Promise<void> | null = null;
  let active = -1;

  const load = () => {
    if (index || loading) return loading;
    status.textContent = root.dataset.loading ?? '';
    loading = Promise.all([import('minisearch'), fetch(`/search/${lang}.json`).then((r) => r.json())])
      .then(([module, docs]: [typeof import('minisearch'), Record<string, unknown>[]]) => {
        const built = new module.default<{ id: number }>(miniSearchOptions() as never);
        built.addAll(docs as never);
        index = built;
        status.textContent = '';
        if (input.value.trim()) render();
      })
      .catch(() => {
        status.textContent = root.dataset.failed ?? '';
      });
    return loading;
  };

  input.addEventListener('focus', () => void load());
  input.addEventListener('input', () => void (index ? render() : load()));

  function render() {
    const query = input.value.trim();
    list.replaceChildren();
    active = -1;
    if (!index || query.length < 2) {
      list.hidden = true;
      input.setAttribute('aria-expanded', 'false');
      input.removeAttribute('aria-activedescendant');
      status.textContent = '';
      return;
    }
    const results = dedupeByPage(index.search(query) as unknown as (Stored & { id: number })[], 10);
    if (results.length === 0) {
      list.hidden = true;
      input.setAttribute('aria-expanded', 'false');
      status.textContent = root.dataset.empty ?? '';
      return;
    }
    status.textContent = '';
    results.forEach((result, position) => {
      const item = document.createElement('li');
      item.id = `${root.id}-r${position}`;
      item.setAttribute('role', 'option');
      const link = document.createElement('a');
      link.href = result.href;
      const title = document.createElement('span');
      title.className = 's__title';
      title.textContent = result.heading ? `${result.title} › ${result.heading}` : result.title;
      const context = document.createElement('span');
      context.className = 's__context';
      context.textContent = result.group;
      const excerpt = document.createElement('span');
      excerpt.className = 's__excerpt';
      excerpt.textContent = result.excerpt;
      link.append(context, title, excerpt);
      item.append(link);
      list.append(item);
    });
    list.hidden = false;
    input.setAttribute('aria-expanded', 'true');
  }

  function move(step: number) {
    const items = [...list.querySelectorAll<HTMLElement>('[role=option]')];
    if (items.length === 0) return;
    active = (active + step + items.length) % items.length;
    items.forEach((item, position) => item.classList.toggle('is-active', position === active));
    input.setAttribute('aria-activedescendant', items[active].id);
    items[active].scrollIntoView({ block: 'nearest' });
  }

  input.addEventListener('keydown', (event) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      move(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      move(-1);
    } else if (event.key === 'Enter') {
      const items = [...list.querySelectorAll<HTMLAnchorElement>('[role=option] a')];
      const target = items[active === -1 ? 0 : active];
      if (target) {
        event.preventDefault();
        location.href = target.href;
      }
    } else if (event.key === 'Escape') {
      input.value = '';
      list.hidden = true;
      status.textContent = '';
      input.setAttribute('aria-expanded', 'false');
    }
  });

  // Clicking outside closes the list; the query stays so the visitor can come back to it.
  document.addEventListener('click', (event) => {
    if (!root.contains(event.target as Node)) list.hidden = true;
  });
  input.addEventListener('focus', () => {
    if (list.children.length > 0) list.hidden = false;
  });
}
