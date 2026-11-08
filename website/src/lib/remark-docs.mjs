// Prepare the Markdown tree of the wiki, without changing what the files say:
// - relative links to another guide or developer doc point at its wiki page, keeping the anchor;
// - links to other files of the repository point at GitHub (blob, or tree for folders);
// - image paths get a `./` so the asset pipeline resolves them as local files and optimises them;
// - HTML comments disappear, as on GitHub.
import { statSync } from 'node:fs';
import path from 'node:path';

import { devDocForFile, devHref, docsRoot, githubBlob, githubTree, guideHref, guideRoot, repoRoot } from './docs-catalog.mjs';

/** Where the link of a document at `filePath` really goes; everything non-relative stays. */
export function rewriteDocLink(url, filePath) {
  if (!url || /^(?:[a-z][a-z0-9+.-]*:|#|\/)/i.test(url)) return url;
  const hashAt = url.indexOf('#');
  const beforeHash = hashAt === -1 ? url : url.slice(0, hashAt);
  const hash = hashAt === -1 ? '' : url.slice(hashAt);
  const queryAt = beforeHash.indexOf('?');
  const target = queryAt === -1 ? beforeHash : beforeHash.slice(0, queryAt);
  if (!target) return url;
  const resolved = path.resolve(path.dirname(filePath), decodeURI(target));

  const insideGuide = resolved.startsWith(guideRoot + path.sep);
  if (/\.md$/i.test(target) && insideGuide) {
    const [lang, file] = path.relative(guideRoot, resolved).split(path.sep);
    return guideHref(file.replace(/\.md$/i, ''), lang) + hash;
  }
  if (/\.md$/i.test(target) && resolved.startsWith(docsRoot + path.sep)) {
    const doc = devDocForFile(path.relative(docsRoot, resolved));
    if (doc) {
      // Same-language URL in the guides; the developer docs render twice and are localised later.
      const fromGuideLang = filePath.startsWith(guideRoot + path.sep)
        ? path.relative(guideRoot, filePath).split(path.sep)[0]
        : 'de';
      return devHref(doc.slug, fromGuideLang) + hash;
    }
  }

  const repoPath = path.relative(repoRoot, resolved).split(path.sep).join('/');
  if (repoPath.startsWith('..')) return url;
  let isDirectory = target.endsWith('/');
  try {
    isDirectory = statSync(resolved).isDirectory();
  } catch {
    // A file that does not exist (yet): treat it as a file and let the link check report it.
  }
  return `${isDirectory ? githubTree : githubBlob}${repoPath}${hash}`;
}

/** The walker from unist-util-visit, in a few lines (no extra dependency). */
function walk(node, visit, parent = null) {
  visit(node, parent);
  if (Array.isArray(node.children)) for (const child of [...node.children]) walk(child, visit, node);
}

export default function remarkDocs() {
  return (tree, file) => {
    const filePath = file.path ?? file.history?.[0];
    if (!filePath) return;
    const directory = path.dirname(filePath);
    walk(tree, (node, parent) => {
      if ((node.type === 'link' || node.type === 'definition') && node.url) {
        node.url = rewriteDocLink(node.url, filePath);
      }
      if (node.type === 'image' && node.url && !/^(?:[a-z][a-z0-9+.-]*:|\/)/i.test(node.url)) {
        // Inside docs/ the picture is a local asset; outside, the build cannot use it.
        const resolved = path.resolve(directory, decodeURI(node.url));
        if (resolved.startsWith(docsRoot + path.sep) && !node.url.startsWith('./') && !node.url.startsWith('../')) {
          node.url = `./${node.url}`;
        }
      }
      if (node.type === 'html' && /^\s*<!--[\s\S]*?-->\s*$/.test(node.value) && parent) {
        parent.children.splice(parent.children.indexOf(node), 1);
      }
    });
  };
}
