#!/bin/sh
# Installs OpenDJDecks as a kiosk on Raspberry Pi OS Lite (64-bit),
# Raspberry Pi 4 or 5. Run from the unpacked bundle as root:
#   sudo ./install.sh [user]
set -eu
USER_NAME=${1:-dj}
HERE=$(cd "$(dirname "$0")" && pwd)

apt-get update
apt-get install -y --no-install-recommends \
  librubberband2 libasound2 libinput10 libgbm1 libdrm2 libegl1 libgles2 libfontconfig1 libxkbcommon0 \
  udisks2 exfatprogs dosfstools

id "$USER_NAME" >/dev/null 2>&1 || useradd -m -s /bin/bash "$USER_NAME"
usermod -aG audio,video,input,render,plugdev "$USER_NAME"

install -d /opt/opendjdecks
install -m755 "$HERE/opendjdecks" "$HERE/od-cli" "$HERE/opendjdecks-mcp" /opt/opendjdecks/
rm -rf /opt/opendjdecks/mappings && cp -R "$HERE/mappings" /opt/opendjdecks/mappings

ln -sf /opt/opendjdecks/opendjdecks /usr/local/bin/opendjdecks
# Launcher entry and icons (for a desktop session next to the kiosk).
APP_ID=de.opendjdecks.OpenDJDecks
install -Dm644 "$HERE/$APP_ID.desktop" /usr/share/applications/$APP_ID.desktop
install -Dm644 "$HERE/icons/$APP_ID.svg" /usr/share/icons/hicolor/scalable/apps/$APP_ID.svg
for s in 48 128 256 512; do
  install -Dm644 "$HERE/icons/$APP_ID-$s.png" /usr/share/icons/hicolor/${s}x${s}/apps/$APP_ID.png
done
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q /usr/share/icons/hicolor || true

# Real-time audio for the audio group.
cat > /etc/security/limits.d/95-opendjdecks-audio.conf <<LIMITS
@audio - rtprio 95
@audio - memlock unlimited
LIMITS

# USB sticks are mounted under /media/opendjdecks/<device> automatically and
# may be ejected by the kiosk user.
install -m644 "$HERE/99-opendjdecks-usb.rules" /etc/udev/rules.d/
install -d /etc/polkit-1/rules.d
install -m644 "$HERE/50-opendjdecks-udisks.rules" /etc/polkit-1/rules.d/
install -d /media/opendjdecks

# Kiosk service: starts at boot straight into Perform.
sed "s/@USER@/$USER_NAME/g" "$HERE/opendjdecks-kiosk.service" > /etc/systemd/system/opendjdecks-kiosk.service
systemctl daemon-reload
systemctl enable opendjdecks-kiosk.service
systemctl set-default multi-user.target
# The kiosk owns the display: no login prompt on tty1.
systemctl disable getty@tty1.service || true

# Performance CPU governor (audio stability).
if [ -w /etc/default/cpufrequtils ] || [ ! -e /etc/default/cpufrequtils ]; then
  echo 'GOVERNOR="performance"' > /etc/default/cpufrequtils
fi
udevadm control --reload
echo "Installed. Reboot to start OpenDJDecks."
