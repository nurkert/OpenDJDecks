---
title: Raspberry Pi standalone
description: A Raspberry Pi 4 or 5 with a touchscreen on your controller boots straight into Perform and plays the sticks you prepare at home.
keywords: [raspberry, raspberry pi, pi, pi 4, pi 5, standalone, kiosk, touchscreen, embedded, no laptop, lite, linux]
features: [shell-kiosk, shell-performance-profiles]
---

OpenDJDecks can run without a laptop on the booth: a Raspberry Pi 4 or 5 with a touchscreen, attached to a compatible controller, starts straight into Perform. The controller drives everything, including the browser.

> [!NOTE]
> This setup is for people comfortable with installing Linux. The Pi build is tested automatically, but it has not yet been verified on a real Raspberry Pi, so expect rough edges.

## What you need

- Raspberry Pi 4 or 5
- 64-bit Raspberry Pi OS Lite (trixie or newer)
- A 7" or 10" touchscreen
- A compatible DJ controller

## Install

On the Pi, in a terminal:

```sh
git clone https://github.com/nurkert/OpenDJDecks && cd OpenDJDecks
packaging/linux/install-build-deps.sh
packaging/raspberry-pi/build-bundle.sh
tar -xzf target/dist/opendjdecks-pi-*.tar.gz -C /tmp
sudo /tmp/opendjdecks-pi-*/install.sh dj
sudo reboot
```

Or download the `opendjdecks-pi-*.tar.gz` bundle from the [download page](/en/download/#pi) and run its `install.sh`. The installer sets up the app as a full-screen service without a desktop, real-time priority for audio, and automatic mounting of USB sticks.

## Your music

Prepare sticks on your Mac or Linux machine. A registered stick carries the OpenDJDecks format, and the Pi plays it directly, including the [stems](stems.md#stems-on-a-stick-and-on-the-raspberry-pi) you separated on the laptop.

The Pi runs the **Lite** performance profile, tuned to stay reliable on small hardware. It is the same app with the same library format.

## Developers

`packaging/raspberry-pi/README.md` covers building an SD card image with pi-gen.
