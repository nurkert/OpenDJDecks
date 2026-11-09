#!/usr/bin/env bash
# Repository rules that keep OpenDJDecks an independent project (CONTRIBUTING.md, "Independence").
# Looks at tracked files only and fails on the first group of findings it prints.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

failed=0
report() {
  if [ -n "$2" ]; then
    printf '%s\n%s\n\n' "FAIL: $1" "$2" >&2
    failed=1
  fi
}

report "third-party program files or exported vendor data are tracked" \
  "$(git ls-files | grep -iE '\.(dll|dylib|so|exe|sys|efi|fw|dmg|pkg|msi)$|(^|/)(exportExt|export)\.pdb$|(^|/)ANLZ[0-9]+\.(DAT|EXT|2EX)$|(^|/)master\.db$|(^|/)(DJM|DEV)?MYSETTING2?\.DAT$' || true)"

report "a vendor name is part of a file or crate name" \
  "$(git ls-files | grep -iE 'pioneer|alphatheta|rekordbox|cdj' || true)"

report "a vendor product name is used as a name in the interface" \
  "$(git grep -n -E '\bCDJ\b' -- 'crates/od-ui/ui' 'crates/od-ui/lang' 'website/src/i18n' 'website/src/components' || true)"

report "notes on analysing third-party programs" \
  "$(git grep -n -iE 'disassembl|decompil|ghidra|hopper provider|reverse.engineer' -- . ':!CONTRIBUTING.md' ':!.github/pull_request_template.md' ':!packaging/ci/check-legal.sh' ':!website/package-lock.json' ':!Cargo.lock' || true)"

report "a fixed database key in the import code" \
  "$(git grep -n -iE '"[0-9a-f]{64}"|deobfuscat' -- 'crates/od-import' || true)"

if [ "$failed" -ne 0 ]; then
  exit 1
fi
echo "Repository rules OK."
