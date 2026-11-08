// HTML tree of the wiki: headings get stable ids (the search links to them) and a permalink;
// wide tables and code blocks scroll inside a frame instead of widening the page; admonition
// blockquotes (`> [!NOTE]`, `> [!TIP]`, `> [!WARNING]`) become plates in the site's style.
import { slugify } from './search-core.mjs';

const isElement = (node, name) => node?.type === 'element' && (!name || node.tagName === name);
const text = (node) =>
  node.type === 'text' ? node.value : Array.isArray(node.children) ? node.children.map(text).join('') : '';

const labels = { note: 'Note', tip: 'Tip', warning: 'Warning', important: 'Important', caution: 'Caution' };

export function headingId(value, seen = new Map()) {
  const base = slugify(value) || 'section';
  const count = seen.get(base) ?? 0;
  seen.set(base, count + 1);
  return count === 0 ? base : `${base}-${count}`;
}

function walk(node, visit) {
  if (!Array.isArray(node.children)) return;
  for (let i = 0; i < node.children.length; i += 1) {
    const child = node.children[i];
    const replaced = visit(child, node, i);
    if (replaced !== undefined) node.children[i] = replaced;
    // A replacement wraps the original node, which has already been handled.
    if (replaced === undefined) walk(child, visit);
  }
}

export default function rehypeDocs() {
  return (tree) => {
    const seen = new Map();
    walk(tree, (node) => {
      if (!isElement(node)) return undefined;
      if (/^h[1-4]$/.test(node.tagName)) {
        const value = text(node).trim();
        node.properties ??= {};
        node.properties.id = headingId(value, seen);
        return undefined;
      }
      if (isElement(node, 'table')) {
        return {
          type: 'element',
          tagName: 'div',
          properties: { className: ['table-frame'], tabIndex: 0, role: 'region', ariaLabel: 'Table' },
          children: [node],
        };
      }
      if (isElement(node, 'blockquote')) {
        const first = node.children.find((c) => isElement(c, 'p'));
        const head = first?.children?.[0];
        const match = head?.type === 'text' ? /^\s*\[!(NOTE|TIP|WARNING|IMPORTANT|CAUTION)\]\s*/.exec(head.value) : null;
        if (!match) return undefined;
        const kind = match[1].toLowerCase();
        head.value = head.value.slice(match[0].length);
        node.tagName = 'aside';
        node.properties = { className: ['note', `note--${kind}`], dataLabel: labels[kind] };
      }
      return undefined;
    });
  };
}
