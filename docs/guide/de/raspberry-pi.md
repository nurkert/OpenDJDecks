---
title: Raspberry Pi Standalone
description: Ein Raspberry Pi 4 oder 5 mit Touchscreen am Controller startet direkt in Perform und spielt die Sticks, die du zu Hause vorbereitest.
keywords: [raspberry, raspberry pi, pi, pi 4, pi 5, standalone, kiosk, touchscreen, ohne laptop, eigenständig, lite, linux]
features: [shell-kiosk, shell-performance-profiles]
---

OpenDJDecks läuft auch ohne Laptop im Booth: ein Raspberry Pi 4 oder 5 mit Touchscreen, an einem kompatiblen Controller, startet direkt in Perform. Der Controller steuert alles, auch den Browser.

> [!NOTE]
> Das ist etwas für Leute, die sich mit der Linux-Installation wohlfühlen. Der Pi-Build wird automatisch getestet, ist aber noch nicht auf einem echten Raspberry Pi verifiziert; rechne mit rauen Stellen.

## Was du brauchst

- Raspberry Pi 4 oder 5
- 64-Bit Raspberry Pi OS Lite (trixie oder neuer)
- Einen 7"- oder 10"-Touchscreen
- Einen kompatiblen DJ-Controller

## Installieren

Auf dem Pi, im Terminal:

```sh
git clone https://github.com/nurkert/OpenDJDecks && cd OpenDJDecks
packaging/linux/install-build-deps.sh
packaging/raspberry-pi/build-bundle.sh
tar -xzf target/dist/opendjdecks-pi-*.tar.gz -C /tmp
sudo /tmp/opendjdecks-pi-*/install.sh dj
sudo reboot
```

Oder lade das Paket `opendjdecks-pi-*.tar.gz` von der [Download-Seite](/download/#pi) und führe sein `install.sh` aus. Der Installer richtet die App als Vollbild-Dienst ohne Desktop ein, mit Echtzeit-Priorität fürs Audio und automatischem Einhängen von USB-Sticks.

## Deine Musik

Bereite Sticks auf deinem Mac oder Linux-Rechner vor. Ein registrierter Stick trägt das OpenDJDecks-Format, und der Pi spielt es direkt, auch die [Stems](stems.md#stems-auf-dem-stick-und-am-raspberry-pi), die du auf dem Laptop getrennt hast.

Der Pi läuft im Leistungsprofil **Lite**, das auf kleiner Hardware zuverlässig bleibt. Es ist dieselbe App mit demselben Bibliotheksformat. Im Lite-Profil ist der [CPU-Sparmodus](preferences.md#ansicht) immer an: 30 Bilder pro Sekunde, Wellenformen in Schritten, keine Animationen und ein automatischer Audiopuffer von 256 Frames.

Wie viel Prozessor der Pi braucht, wurde auf einem echten Raspberry Pi noch nicht gemessen.

## Entwickler

`packaging/raspberry-pi/README.md` beschreibt, wie man mit pi-gen ein SD-Karten-Image baut.
