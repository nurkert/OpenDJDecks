#!/bin/sh
# Builds "OpenDJDecks.app" and a styled disk image in target/dist:
#   target/dist/OpenDJDecks-<version>-macos-<arch>.dmg
# Requirements: Rust, Homebrew rubberband + dylibbundler, Python 3.
# Signing: ad-hoc by default; set CODESIGN_IDENTITY for a Developer ID.
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=$(uname -m)
cargo build --release --locked -p opendjdecks -p od-cli -p od-mcp

DIST=target/dist
APP="$DIST/OpenDJDecks.app"
rm -rf "$APP" && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp target/release/opendjdecks target/release/od-cli target/release/opendjdecks-mcp "$APP/Contents/MacOS/"
cp -R mappings "$APP/Contents/Resources/mappings"
cp packaging/assets/OpenDJDecks.icns "$APP/Contents/Resources/OpenDJDecks.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>OpenDJDecks</string>
  <key>CFBundleDisplayName</key><string>OpenDJDecks</string>
  <key>CFBundleIdentifier</key><string>de.opendjdecks.OpenDJDecks</string>
  <key>CFBundleExecutable</key><string>opendjdecks</string>
  <key>CFBundleIconFile</key><string>OpenDJDecks</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>GPL-3.0-or-later</string>
</dict></plist>
PLIST

# Bundle non-system libraries (Rubber Band, libsamplerate, …).
dylibbundler -od -b -cd -ns \
  -x "$APP/Contents/MacOS/opendjdecks" -x "$APP/Contents/MacOS/od-cli" \
  -d "$APP/Contents/Frameworks/" -p @executable_path/../Frameworks/ >/dev/null
if otool -L "$APP/Contents/MacOS/opendjdecks" | grep -E '/opt/homebrew|/usr/local/(opt|Cellar)'; then
  echo "unbundled Homebrew library left in the binary" >&2
  exit 1
fi

# Sign inside-out. The hardened runtime needs matching Team IDs on every
# library, so it is only enabled with a real Developer ID.
if [ -n "${CODESIGN_IDENTITY:-}" ]; then
  SIGN="--options runtime --timestamp --sign $CODESIGN_IDENTITY"
else
  SIGN="--sign -"
fi
for lib in "$APP"/Contents/Frameworks/*.dylib; do
  codesign --force $SIGN "$lib"
done
codesign --force $SIGN "$APP/Contents/MacOS/od-cli"
codesign --force $SIGN "$APP/Contents/MacOS/opendjdecks-mcp"
codesign --force $SIGN "$APP"
codesign --verify --deep --strict "$APP"

# A fresh venv outside target/ (CI caches target/, a cached venv can point
# at a Python that no longer exists).
VENV="${TMPDIR:-/tmp}/opendjdecks-dmgbuild"
rm -rf "$VENV" "$DIST/.dmgbuild"
python3 -m venv "$VENV" >/dev/null
"$VENV/bin/pip" install --quiet "dmgbuild==1.6.5"
OUT="$DIST/OpenDJDecks-${VERSION}-macos-${ARCH}.dmg"
rm -f "$OUT"
"$VENV/bin/dmgbuild" -s packaging/macos/dmg-settings.py \
  -D app="$APP" -D background=packaging/assets/dmg-background.tiff -D icon=packaging/assets/OpenDJDecks.icns \
  "OpenDJDecks" "$OUT"
echo "$OUT"
