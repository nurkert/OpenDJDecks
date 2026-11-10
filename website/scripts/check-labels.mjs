#!/usr/bin/env node
// The guides name the controls as the app shows them: every bold label that looks like a
// control (an all-caps word or phrase in **…**, e.g. **KEYLOCK**, **ALL FX OFF**) must still
// exist in the app.
//
// - English guides (docs/guide/en): the label is a UI text of the app, a string of the .slint
//   files or of the Rust code (crates/*/src).
// - German guides (docs/guide/de): the label is a msgstr of the German catalogue
//   (crates/od-ui/lang/de/LC_MESSAGES/od-ui.po), a German text of od-i18n, or a text the app
//   shows untranslated.
//
// A label counts when a legend of the app (a UI text without lower-case letters) contains it
// as whole words, or when a legend with placeholders (`LOAD TO: DECK {}`) produces it. Keyboard shortcuts, file names and labels without letters
// are exempt; RUNTIME lists the few labels the app assembles at run time.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';

import { guideRoot, repoRoot } from '../src/lib/docs-catalog.mjs';

/** Labels the app builds at run time from parts (each with where it comes from). */
const RUNTIME = new Map([
  ['FX1', 'fx.slint: "FX" + the unit number'],
  ['FX2', 'fx.slint: "FX" + the unit number'],
]);

function walk(dir, ext, skip = () => false) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = path.join(dir, name);
    if (skip(p)) continue;
    const st = statSync(p);
    if (st.isDirectory()) out.push(...walk(p, ext, skip));
    else if (name.endsWith(ext)) out.push(p);
  }
  return out;
}

/** String literals of a source file (Slint and Rust share the double-quoted form). */
function literals(src) {
  const out = [];
  const re = /"((?:[^"\\\n]|\\.)*)"/g;
  let m;
  while ((m = re.exec(src))) out.push(m[1].replace(/\\"/g, '"').replace(/\\n/g, '\n').replace(/\\\\/g, '\\'));
  return out;
}

/** msgstr values of a .po file (multi-line strings joined). */
function msgstrs(src) {
  const out = [];
  let cur = null;
  for (const line of src.split('\n')) {
    if (line.startsWith('msgstr ')) {
      if (cur !== null) out.push(cur);
      cur = JSON.parse(line.slice(7));
    } else if (cur !== null && line.startsWith('"')) {
      cur += JSON.parse(line);
    } else if (cur !== null) {
      out.push(cur);
      cur = null;
    }
  }
  if (cur !== null) out.push(cur);
  return out.filter((s) => s !== '');
}

const LETTER = 'A-Za-zÄÖÜäöüß';
const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** Matchers of a set of UI texts: whole-word containment, and templates with `{}`. */
function matcher(texts) {
  // Only legends count: texts without lower-case letters (a tooltip that still
  // mentions an old key does not keep the key alive).
  const plain = [...new Set(texts)].filter((t) => !/[a-zäöüß]/.test(t.replace(/\{\d*\}/g, '')));
  // A template produces the whole label; it needs letters of its own (`{}` alone
  // would produce anything).
  const templates = plain
    .filter((t) => /\{\d*\}/.test(t) && /[A-ZÄÖÜ]{2}/.test(t.replace(/\{\d*\}/g, '')))
    .map((t) => new RegExp(`^${t.split(/\{\d*\}/).map(escape).join('.+?')}$`));
  return (label) => {
    const word = new RegExp(`(^|[^${LETTER}0-9])${escape(label)}($|[^${LETTER}0-9])`);
    return plain.some((t) => word.test(t)) || templates.some((re) => re.test(label));
  };
}

/** A bold label that names a control: letters, all of them upper case. */
function isControlLabel(label) {
  if (!/[A-ZÄÖÜ]/.test(label)) return false;
  if (/[a-zäöüß]/.test(label)) return false;
  // Keyboard shortcuts and keys.
  if (/[⌘⌥⇧⌃]/.test(label) || /^(F\d{1,2}|ESC|TAB|ENTER|RETURN|SPACE|SHIFT|CTRL|ALT|CMD|DEL|BACKSPACE)$/.test(label)) return false;
  if (/\b(CTRL|ALT|SHIFT|CMD)\s*\+/.test(label)) return false;
  // File names and paths.
  if (/\.[A-Za-z0-9]{2,5}$/.test(label) || label.includes('/') && /\.[A-Za-z]/.test(label)) return false;
  return true;
}

/**
 * Whether the app shows `label`. The guides write a few controls in a short notation:
 * `‹ FILTER ›` is the stepper FILTER, `MOVE:` a lead-in, `◀1 / 1▶` several keys, and
 * `MEMORY +` or `BUILD 8` a legend with its glyph or value.
 */
function known(has, label) {
  const core = label.replace(/[:.,]$/, '').replace(/^‹\s*(.*?)\s*›$/, '$1').trim();
  if (has(core)) return true;
  const parts = core.split(' / ').map((p) => p.trim());
  return parts.every((p) => {
    if (!/[A-ZÄÖÜ]/.test(p) || has(p) || RUNTIME.has(p)) return true;
    // A legend followed by its glyph or a value.
    const stem = p.replace(/(\s+[^A-ZÄÖÜ\s]+)+$/, '');
    return stem !== p && has(stem);
  });
}

function boldLabels(md) {
  const out = [];
  // Code spans and blocks are not prose.
  const text = md.replace(/```[\s\S]*?```/g, '').replace(/`[^`\n]*`/g, '');
  const re = /\*\*([^*\n]+?)\*\*/g;
  let m;
  while ((m = re.exec(text))) out.push(m[1].trim());
  return out;
}

const skipTests = (p) => p.includes(`${path.sep}ui_tests`) || p.includes(`${path.sep}tests${path.sep}`) || p.includes(`${path.sep}target${path.sep}`);
const slint = walk(path.join(repoRoot, 'crates/od-ui/ui'), '.slint').flatMap((f) => literals(readFileSync(f, 'utf8')));
const rust = readdirSync(path.join(repoRoot, 'crates'))
  .map((c) => path.join(repoRoot, 'crates', c, 'src'))
  .filter((d) => { try { return statSync(d).isDirectory(); } catch { return false; } })
  .flatMap((d) => walk(d, '.rs', skipTests))
  .flatMap((f) => literals(readFileSync(f, 'utf8')));
const english = [...slint, ...rust];
const po = msgstrs(readFileSync(path.join(repoRoot, 'crates/od-ui/lang/de/LC_MESSAGES/od-ui.po'), 'utf8'));

const inApp = { en: matcher(english), de: matcher([...po, ...english]) };

const problems = [];
let checked = 0;
for (const lang of ['en', 'de']) {
  const dir = path.join(guideRoot, lang);
  for (const file of readdirSync(dir).filter((f) => f.endsWith('.md')).sort()) {
    const md = readFileSync(path.join(dir, file), 'utf8');
    const seen = new Set();
    for (const label of boldLabels(md)) {
      if (!isControlLabel(label) || seen.has(label)) continue;
      seen.add(label);
      checked++;
      if (RUNTIME.has(label) || known(inApp[lang], label)) continue;
      problems.push(`${lang}/${file}: **${label}** is not a text of the app${lang === 'de' ? ' (nor a msgstr of od-ui.po)' : ''}`);
    }
  }
}

if (problems.length) {
  console.error(`Guide labels FAILED (${problems.length}):\n  ${problems.join('\n  ')}`);
  process.exit(1);
}
console.log(`Guide labels OK: ${checked} control labels in the guides exist in the app.`);
