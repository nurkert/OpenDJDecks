---
title: Stems
description: Tracks auf deinem eigenen Rechner in Vocals, Instrumente, Drums und Bass trennen und jeden Part stummschalten, einpegeln oder mit Effekten belegen.
keywords: [stems, stem, trennung, trennen, teilen, vocals, gesang, acapella, instrumental, drums, schlagzeug, bass, parts, stem fx, stumm, karaoke]
features: [stems-separation, stems-mute-ui, stems-levels, stems-batch-prepare, stems-model-download, stems-status-on-deck, stems-pi-playback, pad-mode-stems, analysis-vocal-position]
---

OpenDJDecks kann einen Track in vier Parts trennen: Vocals, Instrumente, Drums und Bass. Das passiert auf deinem eigenen Rechner, im Hintergrund, und nichts verlässt den Computer. Beim ersten Mal wird einmalig ein Modell geladen; danach geht es offline.

![Die Parts eines Decks: VOCAL, INST, DRUMS und BASS](../img/stems.png)

## Tracks trennen

Es gibt zwei Wege:

- **Beim Laden:** Lade einen Track auf ein Deck. Er spielt wie immer; die Leiste über den Pads zeigt **SPLIT** und einen Prozentwert, während die Parts entstehen. Die Part-Tasten bleiben an ihrem Platz, gedimmt, bis die Parts da sind, und du kannst die ganze Zeit spielen.
- **Im Voraus:** Wähle in Prepare Tracks, Rechtsklick, **Stems trennen** (`⌥⇧⌘S`). Das läuft im Hintergrund, und das Band unten zeigt den Fortschritt.

## Die Parts am Deck benutzen

Jedes Deck hat **VOCAL**, **INST**, **DRUMS** und **BASS** unter der Wellenform.

- Ein Klick schaltet den Part stumm oder laut. Leuchtet heißt hörbar.
- Das Mausrad über einer Taste stellt den Pegel des Parts ein; die Linie unter der Taste zeigt ihn.

Zum Mischen drücke **STEM** an einem Mixer-Kanal: **HI** wird zu den Vocals, **MID** zu den Instrumenten und **LOW** zu den Drums. Der Pad-Modus **STEMS** legt dir die Part-Mutes und Stem-Effekte auf die Pads.

Auf der Übersicht sind Stellen, an denen gesungen wird, als Band oben gezeichnet, damit du siehst, wo du mischen kannst.

Einen Effekt nur auf einen Part legen: siehe [Stem FX](effects.md#stem-fx).

## Stems auf dem Stick und am Raspberry Pi

Nutzt ein Stick das OpenDJDecks-Format, werden die Parts beim Sync mit kopiert. Tracks von diesem Stick benutzen sie, und so spielt der Raspberry Pi Parts, die auf dem Laptop getrennt wurden, ohne die Arbeit selbst zu machen.
