# OpenDJDecks on a Raspberry Pi

Standalone setup: a Raspberry Pi 4 or 5 (64-bit Raspberry Pi OS Lite, trixie or newer) with a
touchscreen, attached to a compatible DJ controller. The Pi boots straight into
Perform; the controller drives everything, including the library browser.

## Install on an existing Raspberry Pi OS Lite (64-bit)

```sh
# on the Pi (or an arm64 build machine)
git clone https://github.com/nurkert/OpenDJDecks && cd opendjdecks
packaging/linux/install-build-deps.sh
packaging/raspberry-pi/build-bundle.sh
tar -xzf target/dist/opendjdecks-pi-*.tar.gz -C /tmp
sudo /tmp/opendjdecks-pi-*/install.sh dj
sudo reboot
```

## Build an SD card image

Use [pi-gen](https://github.com/RPi-Distro/pi-gen) (arm64 branch): copy
`stage-opendjdecks` into the pi-gen tree, put the bundle into
`stage-opendjdecks/files/`, and set `STAGE_LIST="stage0 stage1 stage2 stage-opendjdecks"`.

## What the installer sets up

- `/opt/opendjdecks` with the app, `od-cli` and the controller mappings
- `opendjdecks-kiosk.service`: full screen via `linuxkms` (no desktop), Lite
  profile, real-time priority for audio, restarts automatically
- USB sticks mount at `/media/opendjdecks/<device>` and can be ejected from the app
- performance CPU governor

## Music

Prepare sticks on your Mac or Linux machine: every registered stick carries
the OpenDJDecks format, and the Pi plays it directly.
For a library on the Pi itself, copy music to the Pi and run
`/opt/opendjdecks/od-cli import <folder> && /opt/opendjdecks/od-cli analyze --lite`.
