#!/bin/sh
# Builds the Raspberry Pi bundle (run on aarch64 Linux, e.g. a Pi 5 or an
# arm64 CI runner): target/dist/opendjdecks-pi-<version>.tar.gz
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release --locked -p opendjdecks -p od-cli -p od-mcp --features opendjdecks/kiosk
OUT=target/dist/opendjdecks-pi-$VERSION
rm -rf "$OUT" && mkdir -p "$OUT"
cp target/release/opendjdecks target/release/od-cli target/release/opendjdecks-mcp "$OUT/"
cp -R mappings "$OUT/mappings"
cp packaging/raspberry-pi/install.sh packaging/raspberry-pi/opendjdecks-kiosk.service \
   packaging/raspberry-pi/99-opendjdecks-usb.rules packaging/raspberry-pi/50-opendjdecks-udisks.rules "$OUT/"
APP_ID=de.opendjdecks.OpenDJDecks
cp "packaging/linux/$APP_ID.desktop" "$OUT/"
mkdir -p "$OUT/icons"
cp packaging/assets/icon.svg "$OUT/icons/$APP_ID.svg"
for s in 48 128 256 512; do cp "packaging/assets/icon-$s.png" "$OUT/icons/$APP_ID-$s.png"; done
tar -C target/dist -czf "$OUT.tar.gz" "opendjdecks-pi-$VERSION"
echo "$OUT.tar.gz"
