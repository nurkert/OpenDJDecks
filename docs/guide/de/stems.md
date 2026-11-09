---
title: Stems
description: Tracks auf deinem eigenen Rechner in Vocals, Instrumente, Drums und Bass trennen und jeden Part stummschalten, einpegeln oder mit Effekten belegen.
keywords: [stems, stem, trennung, trennen, teilen, vocals, gesang, acapella, instrumental, drums, schlagzeug, bass, parts, stem fx, stumm, karaoke]
features: [stems-separation, stems-mute-ui, stems-levels, stems-batch-prepare, stems-model-download, stems-model-setup, stems-status-on-deck, stems-pi-playback, pad-mode-stems, analysis-vocal-position]
---

OpenDJDecks kann einen Track in vier Stems trennen: Vocals, Instrumente, Drums und Bass. Das passiert auf deinem eigenen Rechner, im Hintergrund, und nichts verlässt den Computer. Beim ersten Mal wird einmalig ein Modell geladen; danach geht es offline.

![Die Stems eines Decks: VOCAL, INST, DRUMS und BASS](../img/stems.png)

## Tracks trennen

Es gibt zwei Wege:

- **Beim Laden:** Sobald das [Modell installiert ist](#der-einmalige-download-des-modells), lade einen Track auf ein Deck. Er spielt wie immer; die Leiste über den Pads zeigt **SPLIT** und einen Prozentwert, während die Stems entstehen. Die Stem-Tasten bleiben an ihrem Platz, gedimmt, bis die Stems da sind, und du kannst die ganze Zeit spielen.
- **Im Voraus:** Wähle in Prepare Tracks, Rechtsklick, **Stems trennen** (`⌥⇧⌘S`). Das läuft im Hintergrund, und das Band unten zeigt den Fortschritt.

## Die Stems am Deck benutzen

Jedes Deck hat **VOCAL**, **INST**, **DRUMS** und **BASS** unter der Wellenform.

- Ein Klick schaltet den Part stumm oder laut. Leuchtet heißt hörbar.
- Das Mausrad über einer Taste stellt den Pegel des Stems ein; die Linie unter der Taste zeigt ihn.

Zum Mischen drücke **STEM** an einem Mixer-Kanal: **HI** wird zu den Vocals, **MID** zu den Instrumenten und **LOW** zu den Drums. Der Pad-Modus **STEMS** legt dir die Stem-Mutes und Stem-Effekte auf die Pads.

Auf der Übersicht sind Stellen, an denen gesungen wird, als Band oben gezeichnet, damit du siehst, wo du mischen kannst.

Einen Effekt nur auf einen Stem legen: siehe [Stem FX](effects.md#stem-fx).

## Stems auf dem Stick und am Raspberry Pi

Nutzt ein Stick das OpenDJDecks-Format, werden die Stems beim Sync mit kopiert. Tracks von diesem Stick benutzen sie, und so spielt der Raspberry Pi Stems, die auf dem Laptop getrennt wurden, ohne die Arbeit selbst zu machen.

## Der einmalige Download des Modells

Ab Version 0.7 fragt OpenDJDecks, bevor etwas heruntergeladen wird. Das Trennmodell (etwa 158 MB) ist nicht Teil der App. Wenn du ohne Modell zum ersten Mal eine Stem-Taste drückst, einen Mixer-Kanal auf **STEM** stellst oder **Stems trennen** wählst, sagt dir ein Dialog, was gebraucht wird, und bietet **JETZT LADEN** und **JETZT NICHT** an. Deine Musik bleibt auf deinem Computer; nur das Modell kommt einmalig herunter, danach funktionieren Stems ohne Internet.

Während ein Deck spielt oder ein Mix aufgenommen wird, deckt kein Dialog den Bildschirm zu. Ein schmaler Hinweis unter der oberen Leiste bietet den Download an, der Dialog folgt, sobald die Musik stoppt. **JETZT NICHT** gilt für den Rest der Sitzung: Von selbst fragt nichts mehr, und ein Druck auf eine Stem-Taste bringt den Hinweis mit dem Download zurück.

Der Download läuft im Hintergrund, der Fortschritt steht im selben Hinweis, und du kannst weiterspielen. **ABBRECHEN** stoppt ihn, das nächste **HERUNTERLADEN** macht an dieser Stelle weiter. Die Prüfsumme des Modells wird geprüft, bevor es benutzt wird. Alles Weitere steht unter **Einstellungen › Analyse › Stems**, auch die Größe auf der Festplatte und **MODELL ENTFERNEN**. Das Entfernen des Modells entfernt bereits getrennte Stems nicht.

Der Raspberry Pi und der Kiosk laden das Modell nicht herunter; ihre Stems werden auf einem Desktop-Computer getrennt und kommen über den Stick.

## Der Download ist abgebrochen und Stems funktionieren nicht

Ab Version 0.7 lässt ein abgebrochener Download Stems nicht dauerhaft kaputt zurück, und du musst nichts von Hand tun. Der Hinweis unter der oberen Leiste sagt, was passiert ist, und bietet **NOCHMAL** und **HILFE** an. Ist er weg oder gehen Stems weiterhin nicht:

1. Öffne **Einstellungen › Analyse**. Im Abschnitt **Stems** steht als Status **Unvollständig** oder **Beschädigt**.
2. Bei **Unvollständig** drücke **HERUNTERLADEN**: Der Download macht an der Stelle weiter, an der er stehen blieb.
3. Hilft das nicht oder steht dort **Beschädigt**, drücke **LÖSCHEN UND NEU LADEN**. Das entfernt zuerst jede unvollständige, beschädigte oder alte Modelldatei und beginnt von vorn.
4. Warte, bis als Status **Installiert** steht, und drücke dann noch einmal eine Stem-Taste. Tracks, die schon auf einem Deck liegen, werden automatisch getrennt.

Stand im Hinweis **Keine Internetverbindung**, prüfe zuerst dein Netzwerk. Stand dort, dass **nicht genug Platz** auf der Festplatte ist, schaffe Platz und drücke **NOCHMAL**. Das Entfernen oder erneute Laden des Modells rührt bereits getrennte Stems nie an.
