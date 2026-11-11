---
title: Einstellungen und Tastenkürzel
description: Audiogeräte, Sprache und Darstellung, Wellenformen, Deck-Standards, alle Tastenkürzel und wie du sie änderst.
keywords: [einstellungen, optionen, audio, ausgabegerät, puffer, abtastrate, routing, ausgänge, externes mischpult, split cue, midi clock, taktsync, sprache, deutsch, englisch, theme, darstellung, dunkel, hell, farbenblind, kontrast, skalierung, zoom, wellenform, tastenkürzel, kürzel, tastatur, tasten, belegen, belegung, shortcut]
features: [audio-output-routing, audio-cue-device, audio-split-cue-one-output, mixer-external-mode, midi-clock-out, analysis-bpm-ranges, prefs-window, prefs-live-preview, prefs-audio-device, prefs-display, prefs-waveform-options, prefs-deck-defaults, prefs-browser-font, analysis-waveform-styles, shell-ui-scaling, shell-colour-blind-contrast, shell-keyboard-layouts, browser-keyboard-shortcuts, audio-coreaudio, audio-linux-backends, shell-motion-language, shell-keyboard-focus, shell-screen-reader-names, shell-no-hue-alone]
---

Öffne die **Einstellungen** mit dem Zahnrad oben, oder `⌘,` auf dem Mac und `Strg+,` auf Windows und Linux. `Esc` schließt das Fenster. **EINSTELLUNG FINDEN** oben durchsucht alle Seiten.

![Die Einstellungen auf der Seite Ansicht, oben die Live-Vorschau](../img/preferences.png)

## Audio

Das **Ausgabegerät** ist standardmäßig das Audio-Interface des Controllers, falls einer angeschlossen ist. Die **Puffergröße** reicht von 64 bis 1024 Samples: kleiner reagiert schneller, größer ist auf einem langsamen Rechner sicherer. Gerät, Puffer und Routing gelten sofort; eine neue **Abtastrate** wartet auf den nächsten Start, markiert mit **GILT NACH NEUSTART**.

### Routing

- **Master** und **Kopfhörer (Cue)** wählen die Ausgänge des Geräts: **1/2**, **3/4**, **5/6** oder **7/8** (Kopfhörer **AUTOMATIK** nehmen 3/4, wenn das Gerät vier Ausgänge hat). Ein Ausgangspaar trägt nur ein Signal, und ein Paar, das das Gerät nicht hat, bleibt weg; **Jetzt** unten auf der Seite sagt, was wo läuft und was weggelassen wurde.
- **Kopfhörer-Gerät** spielt den Kopfhörer auf einem zweiten Audiogerät (sein Paar aus **Kopfhörer (Cue)**). Die beiden Geräte laufen mit eigenen Takten; OpenDJDecks hält den Kopfhörer im Gleichschritt, indem es ab und zu einen einzelnen Sample-Frame auslässt oder wiederholt. Das kann als leises Klicken auf dem Kopfhörer zu hören sein, nie auf dem Master.
- **Ein Stereo-Ausgang** ist Split Cue auf einem einzigen Stereo-Ausgang, etwa der Kopfhörerbuchse eines Laptops: der Kopfhörer (Cue) in Mono links und der Master in Mono rechts auf den Ausgängen 1/2, für ein Y-Kabel zu den Boxen und deinem Kopfhörer. **SPLIT** am Mixer geht damit an.
- **Mixer: EXTERN** überlässt das Mischen einem externen Mischpult: Jedes Deck spielt auf einem eigenen Paar (**Deck 1 auf** … **Deck 4 auf**), so wie es vom Deck kommt, vor Trim, EQ, Fadern und Crossfader. Die Mixer-Spalte in Perform zeigt dann **EXTERNES MISCHPULT** und den Ausgang jedes Decks statt der Kanalzüge. Sampler und Mikrofon bleiben auf dem Master-Paar, und der Recorder nimmt das Master-Paar auf; nimm also am externen Mischpult auf.

![Einstellungen › Audio mit externem Mischpult: Deck 1 auf 3/4, Deck 2 auf 7/8, und die MIDI-Clock](../img/preferences-routing.png)

### Takt

**MIDI-Clock-Ausgang** sendet das Tempo des Master-Decks als MIDI-Beat-Clock (24 Pulse pro Schlag) an einen MIDI-Ausgang, mit Start auf seinem nächsten Schlag und Stopp, wenn es stoppt, damit Drumcomputer, Sequencer oder ein Lichtpult dem Mix folgen. Tempowechsel werden sofort übernommen; ein Sprung (ein Cue, ein Loop, ein anderes Master-Deck) ab dem nächsten Puls. **Jetzt** zeigt den Port und die gesendeten Pulse. OpenDJDecks sendet nur Clock; einer eingehenden Clock folgt es nicht.

Das Routing ist in der Engine und der Geräteschicht mit erzeugten Signalen getestet (jedes Deck auf seinem Paar, Split Cue in Mono links und rechts, der Kopfhörer über den Ring des zweiten Geräts), die Clock mit einem Timing-Test; nichts davon wurde mit einem echten Mehrkanal-Interface, einem externen Mischpult oder einem Clock-Empfänger ausprobiert.

Unter Windows läuft Audio über WASAPI, unter macOS über CoreAudio und unter Linux über ALSA, JACK oder PipeWire, was du hier wählst.

## Ansicht

**GIG** in der oberen Leiste ist ein Preset fürs Spielen: Ein Klick stellt hohen Kontrast, die Palette für Farbenblinde und eine größere Oberfläche zugleich ein, ein zweiter Klick stellt deine Werte wieder her. Der Pfeil daneben stellt die einzelnen Teile ein. Die leere Bibliothek bietet beim ersten Start denselben Knopf **GIG-PRESET**.

Sprache, Tonart-Schreibweise (**Camelot**, **Open Key** oder **musikalisch**), die Darstellung (dunkel, hell oder wie das System), **Skalierung** von 75 bis 200 % (gilt sofort) und die **Wellenform-Farbe**: 3 Bänder, einfarbig oder RGB. Unter **EIGENE FARBEN** wählst du die Farben für Tiefen, Mitten und Höhen selbst; **STANDARDFARBEN** stellt die vorgegebenen wieder her.

**VORSCHAU** oben auf der Seite zeigt, was die Einstellungen darunter ändern, während du sie änderst: eine Wellenform mit deiner **Wellenform-Farbe** und der Einstellung **Ruhige Wellenform**, die vier Deckfarben und die Statuslinien (sie folgen **Palette für Farbenblinde** und **Hoher Kontrast (helles Licht)**), einen Wert in deiner **Skalierung** und einen Plan des **Deck-Layouts**. **Deck-Layout** wählt **EINFACH**, **STANDARD** oder **VOLL** für die Decks in Perform, genau wie **Ansicht › Deck-Layout**.

Die Seiten sehen aus wie der Rest der App: Jeder Abschnitt beginnt auf einem eigenen Band, der gewählte Wert einer Auswahl ist gefüllt, und Schalter sind gefüllte Spuren.

- **Palette für Farbenblinde** und **Hoher Kontrast (helles Licht)** gelten sofort.
- **Bewegung reduzieren** schaltet jede Animation ab: Tasten, Schalter, Bereiche und die Modus-Reiter wechseln sofort, und das Pulsieren einer scharfen Taste, der Ladestreifen und die Stroboskop-Vorschau stehen still. Die Einstellung ist zunächst aus, weil sich die Systemeinstellung nicht auslesen lässt. Pegel, Wellenformen und Abspielmarken animieren nie: Sie zeigen den Klang immer so, wie er ist.
- **Ruhige Wellenform** (aus in der Voreinstellung) zeichnet auf den großen Wellenformen eine scharfe, dünne Abspielmarke, zarte Beatlinien und sanftere Bandfarben, für lange Sets oder helle Räume. Siehe [Perform](perform-basics.md#ruhige-wellenform).
- **Touch** schaltet das Touch-Layout ein: jedes Bedienelement der Decks, des Mixers, der Pads, der Effekte, des Browsers und des Lichts mindestens 44 px. Siehe [Perform](perform-basics.md#touch-layout).
- **Waveform-Zoom** legt fest, wie viele Sekunden eine Wellenform beim Öffnen eines Tracks zeigt; das Mausrad zoomt weiterhin.
- Unter **Browser** stellst du **Schriftgröße** und **Zeilenabstand** der Liste ein.

## Analyse, Aufnahme

Siehe [Beatgrids](beatgrid.md#wenn-das-tempo-immer-wieder-falsch-ist) für den **BPM-Bereich** und [Aufnahme](recording.md) für die Aufnahmeseite.

Der **BPM-Bereich** (ab Version 0.6) sagt der Tempoanalyse, in welchem Bereich deine Musik liegt. Tracks, die mit halbem oder doppeltem Tempo erkannt würden, werden in den gewählten Bereich verschoben:

- **70–180** ist die Voreinstellung und passt für die meiste Musik.
- **58–115** für langsame Musik wie Reggae oder Downtempo.
- **98–195** für schnellere Genres, bei denen ein Track nicht als halbes Tempo erscheinen soll.
- **118–236** für sehr schnelle Musik wie Drum and Bass oder Hardcore.
- **150–300** für Musik über 200 BPM wie Gabber oder Speedcore; siehe [Beatgrids](beatgrid.md#schnelle-musik-und-halb-oder-doppelt-am-deck).

Der Bereich gilt für alle Tracks, die ab jetzt analysiert werden. Um einen analysierten Track zu ändern, wähle in seinem Menü im Player oder in der Liste **Erneut analysieren**.

## Controller

**Decks** enthält die Standards, die an jedes Deck gehen: Tempobereich, Quantisieren, Sync-Modus (**BEAT**, **NUR TEMPO** oder **PHRASE**, siehe [Phrasen](phrases.md#auf-der-phrase-mixen)), Jog-Modus und Auto Cue. **Mixer** enthält EQ und Faderkurven. Den Rest findest du unter [Controller](controllers.md).

## Tastatur

Die Seite **Tastatur** listet jede Aktion mit ihrer Belegung. Suche nach Aktion, Id oder Taste.

1. Klicke eine Belegung an (oder wähle eine mit `↑` `↓` und drücke Return) und drücke die neuen Tasten. `Esc` bricht ab, Rücktaste entfernt die Belegung.
2. Ist eine Taste belegt, nennt die Seite die andere Aktion. **ERSETZEN** übernimmt die Taste, **ABBRECHEN** lässt beide unverändert.
3. **STANDARD** setzt eine Aktion zurück und **ALLE ZURÜCKSETZEN** alles. **IMPORTIEREN…** und **EXPORTIEREN…** lesen und schreiben die Belegung als Datei.

Die Standards folgen einem Layout, das DJs oft benutzen: Deck 1 und 2 liegen auf der linken und rechten Tastaturhälfte, zum Beispiel Play `Z` und `N`, Cue `A` und `H`, Sync `Q` und `Y`, Hot Cues auf den Zahlenreihen, Loop-In, -Out und -Exit auf `D F G` und `K L ;`. Deck 3 und 4 liegen auf `F1` bis `F11`. Ansichts- und Layout-Wechsel nutzen auf dem Mac `⌥⌘`.

Die vollständige Tabelle, pro Deck und Modus, steht auf der Seite [Keyboard shortcuts](../../keyboard.md).

## Erweitert

Backups, der Speicherort der Bibliothek, Zusammenführen und Neu zuordnen, und die Update-Prüfung. Updates lesen nur die öffentliche Release-Liste auf GitHub; es wird nichts über dich gesendet, geprüft wird nicht, während in Perform ein Deck spielt, und ein Update unterbricht nie eine Performance.

## Barrierefreiheit

- **Tastatur:** `Tab` und `⇧Tab` erreichen jedes Bedienelement in der Reihenfolge des Bildschirms, das fokussierte trägt einen hellen Ring. Return oder Leertaste drücken eine Taste; die Pfeiltasten drehen einen Regler oder bewegen einen Fader in Schritten von fünf Prozent, Return setzt einen Regler auf seinen Standardwert. `Esc` gibt die Tasten an das Fenster zurück.
- **Screenreader:** Jedes Bedienelement hat einen lesbaren Namen: den Namen der Aktion aus der Tastenbelegung, wo es eine gibt („Deck 1: Play / Pause“), sonst seine Beschriftung oder den Anfang seines Tooltips, mit dem Deck, Kanal oder der Effekt-Einheit, zu der es gehört. Seine Beschreibung ist der Tooltip mit dem Kurzbefehl. Die Namen folgen der Sprache der App.
- **Hilfe:** `F12` oder die Taste **?** benennt jedes Bedienelement auf dem Bildschirm ([Perform](perform-basics.md#hilfe-zu-jedem-bedienelement)).
- **Farbe steht nie allein:** Ein Zustand, der in Farbe gezeigt wird, hat auch eine Form oder ein Wort: Linienformen (durchgezogen, gestrichelt, halb, durchgestrichen) für Tonart-Beziehungen, Sticks, Übergänge und die Audiolinie, Wörter wie **CLIP**, **KILL**, IN SYNC und **GESPERRT**, die Decknummer neben jeder Deckfarbe und der Buchstabe neben jeder Cue-Farbe, und der Anfangsbuchstabe jeder Phrase, wo ihr Name nicht passt.
- **Touch** und **Hoher Kontrast (helles Licht)** stehen auf der Seite **Ansicht**.

Diese Teile wurden nur mit den eigenen Tests der App geprüft; mit einem echten Screenreader oder einem echten Touchscreen wurden sie noch nicht ausprobiert.
