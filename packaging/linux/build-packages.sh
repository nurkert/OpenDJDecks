#!/bin/sh
# Builds the Linux release artifacts for the current architecture into
# target/dist: .deb, AppImage and a portable tar.gz.
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DEB_ARCH=$(dpkg --print-architecture)
cargo build --release --locked -p opendjdecks -p od-cli -p od-mcp
mkdir -p target/dist

# .deb
command -v cargo-deb >/dev/null || cargo install --locked cargo-deb
cargo deb --no-build -p opendjdecks -o "target/dist/opendjdecks_${VERSION}_${DEB_ARCH}.deb"

# Portable tar.gz
T="target/dist/opendjdecks-${VERSION}-linux-$(uname -m)"
rm -rf "$T" && mkdir -p "$T"
cp target/release/opendjdecks target/release/od-cli target/release/opendjdecks-mcp "$T/"
cp -R mappings "$T/mappings"
cp README.md LICENSE packaging/linux/de.opendjdecks.OpenDJDecks.desktop "$T/"
cp packaging/assets/icon.svg "$T/de.opendjdecks.OpenDJDecks.svg"
cp packaging/assets/icon-256.png "$T/de.opendjdecks.OpenDJDecks.png"
tar -C target/dist -czf "$T.tar.gz" "$(basename "$T")"

# AppImage
packaging/linux/build-appimage.sh
ls -la target/dist
