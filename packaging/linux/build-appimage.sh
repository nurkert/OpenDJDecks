#!/bin/sh
# Builds an AppImage in target/dist using linuxdeploy (expects a release build).
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=$(uname -m)
[ -x target/release/opendjdecks ] || cargo build --release --locked -p opendjdecks -p od-cli -p od-mcp
APPDIR=target/dist/AppDir
rm -rf "$APPDIR" && mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/opendjdecks"
cp target/release/opendjdecks target/release/od-cli target/release/opendjdecks-mcp "$APPDIR/usr/bin/"
cp -R mappings "$APPDIR/usr/share/opendjdecks/mappings"
# Icons under the app id (the .desktop Icon= key, the Wayland app_id and
# the X11 WM_CLASS): scalable SVG plus PNGs for desktops without SVG.
APP_ID=de.opendjdecks.OpenDJDecks
for s in 48 128 256 512; do
  install -Dm644 "packaging/assets/icon-$s.png" "$APPDIR/usr/share/icons/hicolor/${s}x${s}/apps/$APP_ID.png"
done
install -Dm644 packaging/assets/icon.svg "target/dist/icon/$APP_ID.svg"
# The tool lives outside target/dist so it never ends up in a release.
TOOL=target/tools/linuxdeploy-$ARCH.AppImage
mkdir -p target/tools
[ -x "$TOOL" ] || { curl -fsSL -o "$TOOL" "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$ARCH.AppImage"; chmod +x "$TOOL"; }
cd target/dist
APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=1 OUTPUT="OpenDJDecks-${VERSION}-${ARCH}.AppImage" "../tools/linuxdeploy-$ARCH.AppImage" \
  --appdir AppDir --executable AppDir/usr/bin/opendjdecks \
  --desktop-file "../../packaging/linux/$APP_ID.desktop" --icon-file "icon/$APP_ID.svg" \
  --output appimage
rm -rf AppDir icon
