---
title: Bibliothek, Playlists und Smart-Listen
description: Tracks finden, sortieren und taggen. Playlists, Smart-Listen, Tags, Auto-Sortierung, die MATCH-Spalte, passende Tracks und der Explorer.
keywords: [bibliothek, sammlung, playlist, playlists, smart liste, smart playlist, ordner, suche, suchen, filter, tag, tags, bewertung, sterne, farbe, kommentar, sortieren, auto sortierung, match, match spalte, passend, duplikate, doppelt, verschoben, fehlende dateien, explorer, verlauf, spalten, rückgängig, backup, heute gespielt, schon gespielt, mehrere bearbeiten, sammelbearbeitung, gemischt, nicht spielen, suchgeschwindigkeit]
features: [browser-do-not-play, browser-search-speed, browser-played-tonight, browser-bulk-tags, browser-fuzzy-search, browser-filter-chips, browser-rails-and-areas, browser-key-sort, browser-collection-table, browser-column-choice, browser-artwork, browser-tree-nodes, browser-tree-reorder, browser-add-to-playlist, browser-playlist-reorder, browser-drag-and-drop, browser-search, browser-filter, browser-rating, browser-track-colour, browser-comments, browser-tag-editing, browser-user-tags, browser-tag-list, browser-similar-tracks, browser-auto-sort, browser-match-column, browser-track-suggestion, browser-smart-playlists, browser-own-history, browser-loaded-indicator, browser-duplicate-check, browser-relocate, browser-consolidate, browser-undo-redo, browser-explorer, browser-explorer-subfolders, browser-explorer-import-on-load, browser-source-rail, browser-column-reorder-multisort, browser-row-context-menu, browser-info-panel, browser-sub-browser, browser-library-backups, browser-library-archive, browser-played-mark, browser-load-gestures, browser-drag-to-deck, browser-calm-table]
---

Der Browser ist in Prepare und Perform derselbe. Links liegen deine Quellen und Listen, in der Mitte die Tracks, rechts eine Leiste mit Panels.

![Der Browser mit Trackliste, MATCH-Spalte und dem Listenbaum](../img/library.png)

## Die Quellen links

Die schmale Leiste schaltet, was der Baum zeigt: **ALLE** (die ganze Sammlung), **LISTEN** (Playlists, Sets und Smart-Listen), **ÄHNLICH** (passende Tracks), **SAMPLER** (das Sampler-Panel in Perform), **DATEIEN** (der Explorer), **USB** (deine Sticks), **VERLAUF** (Verläufe), **AUFNAHMEN** (Aufnahmen) und **TAG-LISTE** (die Tag-Liste).

Die Wörter bleiben ganz, wo Platz ist. In einem schmalen Fenster, oder wenn ein Panel neben der Liste offen ist, zeigen beide Leisten stattdessen ein Zeichen: eine Platte für die Sammlung, Zeilen für die Listen, zwei Ringe für passende Tracks, vier Pads für den Sampler, einen Ordner, einen Stick, eine Uhr für die Verläufe, ein Mikrofon für die Aufnahmen und ein Lesezeichen für die Tag-Liste; rechts ein Etikett, zwei Ringe, vier Pads, ein „i“ für Info und zwei übereinanderliegende Flächen für den Sub-Browser. Halte den Zeiger über ein Zeichen, um seinen Namen zu lesen; auch Screenreader lesen den Namen.

Die Taste mit dem Pfeil neben **LISTEN FILTERN** (**LISTEN EINKLAPPEN**) klappt die Listen bis auf die Leiste ein, damit die Tracks den Platz bekommen. Der Pfeil unten an der Leiste (**LISTEN ZEIGEN**) oder jede Quelle der Leiste holt sie zurück.

Die Listen, die Trackliste und die Panel-Leiste liegen jeweils auf einer eigenen Fläche mit etwas Abstand dazwischen, so siehst du immer, in welchem Bereich du bist.

## Nach Tonart sortieren (ab Version 0.6)

Klicke die Spalte **KEY**, um deine Tracks nach Tonart zu sortieren. Mit einem Rechtsklick auf die Spaltenüberschrift wählst du, wie:

- **Tonarten nach Quintenzirkel sortieren** geht in musikalischer Reihenfolge rund um das Rad: 1A, 1B, 2A, 2B und so weiter. Tracks nebeneinander lassen sich harmonisch mischen. Das ist die Voreinstellung.
- **Tonarten alphabetisch sortieren** sortiert einfach nach dem angezeigten Tonarten-Text.

Deine Wahl bleibt gespeichert. Die Spalte nutzt die Tonart-Schreibweise, die du unter **Einstellungen › Analyse** gewählt hast.

## Playlists, Sets, Smart-Listen und Ordner

Nimm **+ SET**, **+ SMART** und **+ ORDNER** unter dem Baum oder die Tasten: neue Playlist `⌘N`, Set `⇧⌘N`, Ordner `⇧⌘F`, Smart-Liste `⇧⌘M`.

- Klicke den Namen einer Liste an, um sie umzubenennen.
- Ziehe eine Liste auf einen Ordner, um sie hineinzuschieben, oder auf eine andere Liste, um sie davor einzusortieren. Jede Änderung lässt sich rückgängig machen.
- Um Tracks hinzuzufügen, wähle sie aus und drücke **+** neben der Liste, oder ziehe die Zeilen hinüber.
- Ziehe Zeilen zwischen die Zeilen einer nach **#** sortierten Liste, um sie umzuordnen.
- **LÖSCHEN** entfernt die geöffnete Liste (`⌘⌫`). Ausgewählte Tracks entfernst du mit `⌦` oder über das Zeilenmenü aus der Liste.

Für die Planung eines Abends nimm ein [Set](preparing-sets.md) statt einer Playlist: Es zeigt den Verlauf von Tempo, Tonart und Energie.

## Smart-Listen

Eine Smart-Liste füllt sich nach Regeln, die du eintippst. Bedingungen stehen durch Kommas getrennt; ein Track muss alle erfüllen. Lege eine mit **+ SMART** an, tippe in das Regelfeld und drücke Return:

```text
bpm 120-126, key 8A~, genre house, rating >= 3, added < 30d
```

- `bpm`, `rating`, `year`, `plays` und `duration` nehmen einen Wert (`128`), einen Bereich (`120-126`) oder einen Vergleich (`>= 3`).
- `key 8A` trifft genau; `key 8A~` trifft Tonarten, die gut passen (ein Schritt auf dem Kreis oder die Paralleltonart).
- `title`, `artist`, `album`, `genre`, `label`, `comment` und `format` treffen, wenn der Text das Wort enthält. Ein führendes `-` schließt aus (`-genre techno`).
- `added < 30d` vergleicht das Alter: `d` Tage, `w` Wochen, `m` Monate, `y` Jahre.

## Suchen und filtern

Drücke `⌘F` und tippe; die Liste wird beim Tippen enger. Du kannst auch einfach lostippen, nachdem du eine Zeile angeklickt hast: Buchstaben und Ziffern gehen direkt in die Suche. Dass die Liste die Tasten hat, zeigt eine helle Linie an ihrem linken Rand; `Esc` gibt die Tasten an die Decks zurück.

Die Suche verzeiht Tippfehler und findet Wörter in Titel, Artist und Album, auch übergreifend (`nova drive` findet *Night Drive* von *Nova Lines*), dazu in Genre, Label, Remixer und Kommentar so, wie sie getippt sind. In Wörtern mit vier bis sieben Buchstaben ist ein Tippfehler erlaubt, in längeren zwei. Die besten Treffer stehen oben: ein Treffer im Titel vor einem im Artist, ein ganzes Wort vor einem Wortteil. Ein Klick auf einen Spaltenkopf sortiert die Treffer stattdessen nach dieser Spalte.

- `↓` und `↑` in der Suche bewegen sich durch die Treffer.
- `Return` lädt den besten Treffer oder die Zeile, zu der du gegangen bist: auf ein freies Deck in Perform, in den Player in Prepare, wo er gleich spielt. In Perform gehen die Deck-Tasten direkt nach dem Laden wieder.

Die Suche hält beim Tippen Schritt, auch in einer großen Sammlung: Die durchsuchte Liste und der vereinfachte Text jedes Tracks bleiben von einem Tastendruck zum nächsten erhalten, so bewertet, filtert und zeichnet ein Tastendruck nur noch. In einer erzeugten Sammlung mit 50 000 Tracks brauchte ein Tastendruck in einem Entwicklungs-Build etwa 0,1 s; die veröffentlichte App ist ein optimierter Build.

Die Filtertaste über der Liste (`⌘J`) öffnet eine Leiste mit BPM-Bereich, **Tonart passt zum Master**, **BPM ±%** um das Master-Deck (doppeltes und halbes Tempo zählen), einer Mindest-**BEWERTUNG**, einem **TAGS**-Wert und **Heute nicht gespielt**, das auslässt, was in dieser Session schon lief.

Jede gesetzte Bedingung steht als Chip über der Liste, auch wenn die Leiste zu ist: **GEFILTERT NACH** **BPM 120–128**, **BEWERTUNG ≥ 3** und so weiter. Ein Klick auf einen Chip entfernt diese Bedingung; **ZURÜCKSETZEN** neben den Chips entfernt alle.

## Heute gespielt

Ein Track, der in dieser Session insgesamt 30 Sekunden lief, bekommt in der Liste einen grünen Haken, wo sonst die Decknummer steht. Die Spalte **HEUTE** (Zeilenmenü der Spaltenköpfe) zeigt, wann er lief, etwa *22:41*, und **Heute nicht gespielt** in der Filterleiste blendet diese Tracks aus, so läuft ein Wunsch nie versehentlich zweimal.

## Die MATCH-Spalte

Neben der Tonart bewertet **MATCH** jede Zeile gegen das, was gerade läuft: das Master-Deck, sonst ein spielendes Deck, sonst den ausgewählten Track. Bis zu drei Linien leuchten für Tonart und Tempo, und beim Überfahren erfährst du, warum. Sortierst du danach, stehen die besten Treffer oben; die Spalte folgt dem Master-Deck, während du spielst.

## Auto-Sortierung

**AUTO-SORTIERUNG** im Baum gruppiert deine analysierten Tracks nach **Tempo**, **Energie**, **Stimmung**, **Charakter** und **Genre**, berechnet auf deinem eigenen Rechner. Öffne eine Liste wie jede andere; sie lässt sich sortieren, laden und in Playlists ziehen.

## Passende Tracks

Das Panel **PASSEND** rechts zeigt, was nach dem gewählten Track passt: innerhalb ±6 % Tempo und mit passender Tonart. Doppelklick lädt auf ein freies Deck, **ALS LISTE ZEIGEN** öffnet dieselbe Auswahl als Liste. Im Baum macht **Passende Tracks** dasselbe für das Master-Deck oder ein gewähltes Deck 1 bis 4.

## Tags, Bewertung und Farbe

![Drei Tracks ausgewählt: das Info-Panel zeigt GEMISCHT, wo sie sich unterscheiden](../img/library-bulk-tags.png)

- Klicke die Sterne einer Zeile zum Bewerten; ein erneuter Klick löscht. Das Zeilenmenü hat dasselbe, und **Farbe** setzt eine von acht Markierungen.
- Das Panel **INFO** rechts zeigt alle Tags des gewählten Tracks; klicke ein Feld, tippe und speichere mit Return, auch den **KOMMENTAR**. Der Tab **COVER** zeigt das Cover.
- Mit mehreren ausgewählten Tracks bearbeitet das Panel **INFO** alle: Ein Feld, das sie teilen, zeigt seinen Wert, ein Feld, in dem sie sich unterscheiden, ist leer mit *GEMISCHT · RETURN SETZT ALLE*. Was du in ein Feld tippst und mit Return bestätigst, wird bei jedem ausgewählten Track gesetzt, und ein Rückgängig (`⌘Z`) nimmt es bei allen zurück.
- **Nicht spielen** am Fuß des Panels (oder *Nicht spielen (an / aus)* im Zeilenmenü) markiert die ausgewählten Tracks mit dem Tag-Wert *Play: Do not play*. Sie zeigen in der Liste eine rot durchgestrichene Marke, warnen beim Laden, und Auto-Play lässt sie aus (siehe [Perform](perform-basics.md#die-warteschlange-wuensche-und-auto-play)).
- **TAGS** füllst du selbst: Im Panel tippst du `Kategorie: Wert` (zum Beispiel `Stimmung: Peak`) und drückst Return, um die Auswahl zu taggen, dann klickst du einen Wert zum Taggen oder Enttaggen. Filter und Spalte benutzen dieselben Werte.

## Die Tag-Liste

Sammle beim Stöbern Kandidaten: mit `⌥⌘T`, über das Zeilenmenü oder durch Ziehen auf **TAG-LISTE** im Baum. **ALS PLAYLIST SPEICHERN** macht eine Playlist daraus, **LEEREN** leert sie.

## Der Explorer

**DATEIEN** zeigt Ordner auf deiner Festplatte, ohne etwas zu importieren. Drücke **+** neben **EXPLORER**, um einen Ordner hinzuzufügen. **Unterordner einbeziehen** im Listenkopf zeigt alle Dateien darunter. Ein aus dem Explorer geladener Track kommt in die Sammlung und wird im Hintergrund analysiert.

## Spalten und Sortierung

Klicke eine Spaltenüberschrift zum Sortieren, ein zweiter Klick kehrt um. `⇧`-Klick fügt eine zweite und dritte Sortierspalte hinzu. Ziehe eine Überschrift auf eine andere Spalte, um sie zu verschieben, oder eine Spaltenkante, um die Breite zu ändern. Rechtsklick auf die Kopfzeile oder der Spaltenknopf am rechten Ende wählen die sichtbaren Spalten; **Standardspalten** setzt zurück.

Zeilen zeigen außerdem, wo ein Track geladen ist (eine Decknummer), einen Haken, sobald er in dieser Session insgesamt 30 Sekunden lief, und das Cover in der Spalte **ART**.

### Die Liste lesen

Die Liste ist so gezeichnet, dass der Titel führt: Titel sind heller und kräftiger, Artist und Zahlen kommen danach, die übrigen Spalten treten zurück. Zahlen (**BPM**, **TONART**, **ZEIT**, **JAHR**, **WIEDERGABEN** und die übrigen) stehen rechtsbündig mit gleich breiten Ziffern, so lesen sie sich von oben nach unten wie ein Register. Die ausgewählte Zeile ist heller grau hinterlegt, mit einem kurzen gelben Strich am linken Rand. Ein Track, der auf einem Deck liegt, trägt Linie und Nummer dieses Decks in der Deckfarbe (blau für 1, orange für 2, grün für 3, violett für 4).

Der Trenner zwischen den Listen links und der Trackliste hält dort an, wo die Listen noch passen: die drei **+**-Tasten, ein Name und seine Anzahl. Die Listen zeichnen nie über die Tracks.

## Zeilenmenü

Ein Rechtsklick auf eine Zeile bietet alles Weitere: **Auf Deck laden**, **Analysieren**, **Analyse sperren / entsperren**, **Stems trennen**, die zuletzt benutzten Listen, **Zu Playlist hinzufügen**, **Zur Tag-Liste hinzufügen**, **Tags neu aus der Datei lesen**, **Datei exportieren…**, **Wiedergabezähler zurücksetzen**, **Memory Cues zu Hot Cues**, **Hot Cues nach Zeit sortieren**, **Cues wie bei meinen anderen setzen**, **Passende Tracks zeigen**, **Farbe**, **Bewertung**, **Aus dieser Liste entfernen**, **Info zeigen** und **Im Dateimanager zeigen**.

## Tracks laden

- Doppelklick auf eine Zeile: auf ein freies Deck in Perform, in den Player in Prepare, wo der Track gleich spielt (ein Track, der dort schon spielt, läuft weiter). `Return` macht dasselbe mit der gewählten Zeile, in der Suche mit dem besten Treffer. **LADEN AUF: FREIES DECK** über der Liste sagt, wohin ein Doppelklick in Perform lädt; ein Klick wählt ein festes Deck. **ZEILEN: NORMAL** daneben stellt die Zeilenhöhe der Liste ein.
- Ziehe eine Zeile auf ein Deck oder einen Prepare-Player, aus der Liste, dem Explorer oder aus Finder und Dateimanager.
- `⇧←` / `⇧→` laden die Auswahl auf Deck 1 und 2, `⌥⇧←` / `⌥⇧→` auf Deck 3 und 4, und `Return` auf ein freies Deck.

## Ein zweiter Browser

**SUB-BROWSER** in der Panel-Leiste rechts öffnet einen zweiten Baum und eine Liste darunter, damit du Tracks zwischen Listen ziehen kannst, ohne deine Position zu verlieren.

## Verläufe

**VERLAUF** listet jede gespielte Session mit den Tracks in Reihenfolge. Sessions entstehen automatisch, egal ob du aufnimmst.

## Rückgängig

Jede Änderung lässt sich mit `⌘Z` rückgängig machen und mit `⇧⌘Z` wiederholen. Außerdem sichert die App deine Bibliothek automatisch; siehe **Einstellungen › Erweitert**.

## Verschobene Dateien

Benennst oder verschiebst du Musik auf der Festplatte, zeigen Tracks eine durchgestrichene Linie und oben steht **{n} VERSCHOBEN**. Drücke das (oder `⇧⌘L`) und zeige auf den neuen Ordner; die Tracks werden gefunden und neu verbunden.

## Duplikate und Aufräumen

**Ablage › Doppelte Tracks finden** (`⌥⌘Y`) listet Tracks, die mehrfach vorhanden sind. **AUSGEWÄHLTE ENTFERNEN** oder **JE EINEN BEHALTEN** nehmen Kopien aus der Sammlung; die Dateien bleiben auf der Festplatte.

**Ablage › Bibliothek zusammenführen…** kopiert jeden Track in einen gewählten Ordner (`<Ordner>/<Künstler>/…`) und stellt die Bibliothek auf die Kopien um, die Originale bleiben.

## Umzug auf einen neuen Rechner

**Ablage › Bibliotheksarchiv exportieren…** schreibt alles in eine Datei, mit oder ohne Audio. Auf dem neuen Rechner entpackt **Ablage › Bibliotheksarchiv wiederherstellen…** es in einen gewählten Ordner, der beim nächsten Start geöffnet wird.
