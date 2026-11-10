---
title: Sicher durch das Set
description: Die Audiozeile in der Statusleiste, was passiert, wenn das Audiointerface verloren geht, und der Check vor dem Gig.
keywords: [absturz, wiederherstellen, abgestürzt, beendet, stromausfall, sitzung, audio, ausgang, gerät, interface, latenz, aussetzer, xrun, verloren, ausgesteckt, getrennt, soundkarte, gig-check, vor dem gig, speicherplatz, fehlende dateien, sicherheit]
features: [shell-audio-health, shell-gig-check, shell-crash-restore]
---

OpenDJDecks behält die Dinge im Blick, die ein Set vorzeitig beenden können. Diese Seite erklärt, was es zeigt und was du tun kannst.

## Die Audiozeile

Die Statusleiste unten rechts zeigt immer **AUDIO**, den Ausgang, der spielt, seine Latenz in Millisekunden und die bisherigen Aussetzer. Die Linie davor zeigt den Zustand auf einen Blick:

| Linie | Bedeutung |
|---|---|
| Durchgehend grün | Der Ton läuft ohne Aussetzer |
| Gestrichelt gelb | Ein Aussetzer in den letzten 20 Sekunden |
| Durchgestrichen | Das Gerät ist verloren |
| Halbe Höhe | Der Ton läuft noch nicht; er startet mit dem ersten Track |

Kommen immer wieder Aussetzer, erhöhe die Puffergröße unter **Einstellungen › Audio**. Gerät, Puffergröße und Kopfhörerausgang gelten dort sofort, ohne Neustart. Nur eine neue Samplerate wartet auf den nächsten Start.

## Wenn das Audiointerface verloren geht

Wird das Interface ausgesteckt oder hört es auf zu arbeiten, erscheint sofort ein rotes Band unter der oberen Leiste und nennt das Gerät. Die Decks warten, wo sie waren: Tracks, Positionen, Loops und der Mixer bleiben, wie sie sind.

![Das Band, wenn der Audioausgang verloren ist](../img/audio-lost.png)

- Steck das Interface wieder ein: Der Ton kommt von selbst auf demselben Gerät zurück.
- **AUSGANG WECHSELN** spielt sofort auf dem eigenen Ausgang des Rechners weiter.
- **NOCHMAL VERSUCHEN** öffnet das verlorene Gerät noch einmal.

Ein Gerät gilt als verloren, wenn das System es als weg meldet oder wenn es anderthalb Sekunden lang keinen Ton mehr angefordert hat.

Getestet wurde das mit einem simulierten Gerät, das so ausfällt wie ein ausgestecktes Interface. Mit einem echten Interface, das auf jedem System ausgesteckt wird, ist es noch nicht geprüft.

## Der Gig-Check

Öffne das Menü neben **GIG** in der oberen Leiste und drücke **GIG-CHECK STARTEN**, oder wähle **Ansicht › Gig-Check** (`⌥⇧⌘C`, unter Linux und Windows `Strg+Alt+Umschalt+C`). Ein Band zeigt vier Ergebnisse:

- **AUDIOAUSGANG**: das Gerät, seine Latenz, ob Master und Kopfhörer beide einen Ausgang haben, und die bisherigen Aussetzer.
- **CONTROLLER**: der verbundene Controller.
- **PLATZ FÜR AUFNAHME**: freier Platz dort, wo Aufnahmen landen, und wie viele Stunden Aufnahme etwa hineinpassen.
- **TRACKDATEIEN**: Tracks der Bibliothek, deren Datei fehlt.

Eine durchgehend grüne Linie ist bereit, eine gestrichelte ist einen Blick wert, eine durchgestrichene musst du vor dem Spielen beheben. **ERNEUT PRÜFEN** prüft noch einmal, **SCHLIESSEN** blendet das Band aus.

## Nach einem Absturz

Während du spielst, schreibt OpenDJDecks den Zustand der Decks einmal pro Sekunde neben deine Bibliothek: die Tracks, ihre Positionen, Tempo und Bereich, Keylock, Tonartverschiebung, Sync, Loops und den Mixer. Es schreibt eine neue Kopie und tauscht sie dann aus, sodass ein Absturz in jedem Moment einen vollständigen Zustand hinterlässt.

Wurde die App nicht richtig beendet, bietet der nächste Start ihn in einem Band unter der oberen Leiste wieder an:

![Das Angebot zum Wiederherstellen nach einem Absturz](../img/restore.png)

- **DECKS WIEDERHERSTELLEN** lädt die Tracks dorthin, wo sie waren, mit Tempo, Loops und Mixer, und öffnet Perform. Die Decks warten auf **PLAY**.
- **LEER STARTEN** lässt die Decks leer.

Eine laufende Aufnahme bleibt in jedem Fall erhalten. Ihre Datei wird bis zum Moment des Absturzes vervollständigt und mit der bisherigen Trackliste unter Aufnahmen aufgeführt. Das Band sagt, wie lang sie ist.
