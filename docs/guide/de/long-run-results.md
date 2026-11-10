---
title: Ergebnisse von Langläufen
description: Wie lange OpenDJDecks ohne Aussetzer spielt, über Stunden gemessen mit zwei und vier Decks, Stems, Effekten und Aufnahme, und wie du die Messung auf deinem Rechner wiederholst.
keywords: [aussetzer, dropout, latenz, stabilität, langlauf, soak, zuverlässigkeit, leistung, cpu, speicher, puffer]
features: []
---

Das sind Messungen, keine Versprechen. Jeder Lauf spielt echte Tracks auf zwei oder vier Decks über Stunden und schreibt eine Zeile pro Minute: wie oft ein Puffer zu spät kam, wie lange die Engine pro Puffer gebraucht hat, die Latenz, den Speicher und wie beschäftigt der Rechner war. Hier stehen nur Zahlen, die gemessen wurden.

## Was läuft

- Zwei oder vier Decks spielen und loopen echte Tracks mit Keylock, Sync auf jedem Deck nach dem ersten, 8-Beat-Loops, Loop halbieren, Beat Jumps und Tempowechseln.
- Stems auf jedem Deck, aus dem Stem-Cache (vorher von OpenDJDecks getrennt), mit wechselnden Stem-Pegeln während des Laufs.
- Echo auf dem Master, Hall auf den Drums von Deck 1, Kanal-FX auf jedem Kanal.
- Alle drei Minuten wird ein Deck neu geladen (Dekodieren und Stems, wie in der App).
- Der Master wird die ganze Zeit als WAV aufgenommen. Alle zehn Minuten wird die Datei geschlossen und eine neue begonnen, damit stundenlange Aufnahmen die Platte nicht füllen.

## Wie gemessen wird und was nicht abgedeckt ist

Es wird kein Audiogerät geöffnet, und nichts ist zu hören. Ein Thread mit der Scheduling-Klasse, die das System dem Thread eines Audiogeräts gibt (auf macOS ein Time-Constraint-Thread), holt pro Pufferdauer einen Puffer von der Engine, getaktet von der Uhr. Ein Puffer zählt als **verpasste Frist** (ein Aussetzer), wenn er später fertig ist als eine Pufferdauer nach Beginn seines Takts, also dann, wenn ein Audiogerät ihn gebraucht hätte. **Rechenzeit** ist, wie lange die Engine für einen Puffer gebraucht hat, auch als Anteil an der Pufferdauer. **Latenz** ist die Pufferlänge: 256 Frames bei 48 kHz sind 5,3 ms.

Nicht abgedeckt: die Arbeit des Audiotreibers, die Wandler des Interfaces, das gleichzeitige Zeichnen des Fensters und Eingaben vom Controller. Die echte App macht das zusätzlich. Andererseits liefen die Messungen unten auf einem Rechner, der gleichzeitig benutzt wurde; deshalb steht die Systemlast in jeder Zeile.

## Ergebnisse

<!-- soak-results:start (generated: cargo run -p od-soak --bin soak-report -- docs/soak/*.jsonl) -->

Rechner: MacBookPro18,2 (Apple M1 Max), 32 GB, macOS 27.0.1. Build: optimiert (`--profile soak`).

### Kurzer Probelauf: 4 Decks, kein Langlauf (2 min)

| Messgröße | Wert |
|---|---|
| Beginn | 2026-11-10 06:32 UTC |
| Gemessene Minuten | 2 |
| Puffer, Rate, Latenz | 256 Frames · 48000 Hz · 5,33 ms |
| Engine-Callbacks | 22533 |
| Verpasste Fristen (Aussetzer) | **12** in 2 der Minuten |
| Rechenzeit länger als der Puffer | 12 |
| Rechenzeit, Median der Minuten-Mediane | 2,21 ms (42 %) |
| Rechenzeit p99, typische / schlechteste Minute | 4,62 ms (87 %) / 4,62 ms (87 %) |
| Längster einzelner Callback | 6,97 ms (131 %) |
| Speicher (RSS) erste / letzte / höchste Minute | 1083 / 1283 / 1283 MB |
| Last des Rechners (1 min), Median / höchste | 11,0 / 11,0 |
| Als WAV aufgenommen | 2 min |
| Neu geladene Decks während des Laufs | 1 |
| Log | `docs/soak/smoke-4-decks-2min.jsonl` |

### 2 Decks, 4 Stunden

| Messgröße | Wert |
|---|---|
| Beginn | 2026-11-10 06:35 UTC |
| Gemessene Minuten | 240 |
| Puffer, Rate, Latenz | 256 Frames · 48000 Hz · 5,33 ms |
| Engine-Callbacks | 2700026 |
| Verpasste Fristen (Aussetzer) | **7** in 5 der Minuten |
| Rechenzeit länger als der Puffer | 6 |
| Rechenzeit, Median der Minuten-Mediane | 2,56 ms (48 %) |
| Rechenzeit p99, typische / schlechteste Minute | 2,96 ms (55 %) / 3,57 ms (67 %) |
| Längster einzelner Callback | 5,82 ms (109 %) |
| Speicher (RSS) erste / letzte / höchste Minute | 1025 / 936 / 1025 MB |
| Last des Rechners (1 min), Median / höchste | 11,4 / 115,3 |
| Als WAV aufgenommen | 240 min |
| Neu geladene Decks während des Laufs | 70 |
| Log | `docs/soak/soak-2-decks-4h.jsonl` |

Stunde für Stunde:

| Stunde | Aussetzer | Rechenzeit p99, schlechteste Minute | Längster Callback | RSS am Ende | Last (Median) |
|---|---|---|---|---|---|
| 1 | 2 | 3,44 ms | 5,82 ms | 936 MB | 12,4 |
| 2 | 0 | 3,46 ms | 5,22 ms | 620 MB | 11,4 |
| 3 | 4 | 3,57 ms | 5,56 ms | 322 MB | 10,3 |
| 4 | 1 | 3,42 ms | 5,47 ms | 936 MB | 11,8 |

### 4 Decks, 4 Stunden

| Messgröße | Wert |
|---|---|
| Beginn | 2026-11-10 10:35 UTC |
| Gemessene Minuten | 240 |
| Puffer, Rate, Latenz | 256 Frames · 48000 Hz · 5,33 ms |
| Engine-Callbacks | 2700003 |
| Verpasste Fristen (Aussetzer) | **1007** in 105 der Minuten |
| Rechenzeit länger als der Puffer | 947 |
| Rechenzeit, Median der Minuten-Mediane | 1,97 ms (37 %) |
| Rechenzeit p99, typische / schlechteste Minute | 3,27 ms (61 %) / 5,38 ms (101 %) |
| Längster einzelner Callback | 7,37 ms (138 %) |
| Speicher (RSS) erste / letzte / höchste Minute | 1219 / 970 / 1339 MB |
| Last des Rechners (1 min), Median / höchste | 8,2 / 111,2 |
| Als WAV aufgenommen | 240 min |
| Neu geladene Decks während des Laufs | 70 |
| Log | `docs/soak/soak-4-decks-4h.jsonl` |

Stunde für Stunde:

| Stunde | Aussetzer | Rechenzeit p99, schlechteste Minute | Längster Callback | RSS am Ende | Last (Median) |
|---|---|---|---|---|---|
| 1 | 83 | 4,60 ms | 6,95 ms | 592 MB | 13,3 |
| 2 | 158 | 4,76 ms | 5,99 ms | 686 MB | 11,7 |
| 3 | 379 | 5,29 ms | 6,49 ms | 916 MB | 4,4 |
| 4 | 387 | 5,38 ms | 7,37 ms | 970 MB | 3,9 |
<!-- soak-results:end -->

### Was die Zahlen zeigen

- **Zwei Decks:** 7 verspätete Puffer in vier Stunden, bei 2,7 Millionen Puffern. Keine Stunde hatte mehr als 4. Der Speicher stieg und fiel mit dem Neuladen der Decks und wuchs über den Lauf nicht an.
- **Vier Decks:** 1007 verspätete Puffer, und sie kamen nicht gleichmäßig. In 135 der 240 Minuten gab es keinen einzigen. Der Rest kam in Schüben, etwa in den Minuten 165 bis 181 und 231 bis 238. In diesen Minuten hat derselbe Rechner OpenDJDecks kompiliert, die Testsuite laufen lassen und Bildschirmfotos der App gemacht. Die mittlere Rechenzeit der Engine pro Puffer lag die ganzen vier Stunden zwischen 1,7 und 2,6 ms. Die Engine wurde mit der Zeit nicht langsamer; sie hat den Prozessor an andere Arbeit verloren.
- **Was das für dich heißt:** Mit vier Decks, Stems und Effekten bei 5,3 ms Puffer schließt du am besten alles andere auf dem Rechner oder wählst einen größeren Puffer. Zwei Decks liefen auch neben schwerer Arbeit stabil.
- **Noch offen:** Diese Läufe liefen auf einem Rechner, der gleichzeitig benutzt wurde. Ein Lauf auf einem ruhigen Rechner und Läufe über acht Stunden fehlen noch. Sie kommen hier dazu, auf dieselbe Weise gemessen.

## Auf deinem Rechner wiederholen

Du brauchst den Quellcode und die Rust-Toolchain. Baue die Messung optimiert (ein Debug-Build würde unterschätzen, was die Engine kann):

```sh
cargo run --profile soak -p od-soak --bin soak -- --decks 4 --hours 4 \
  --track "/musik/a.mp3" --track "/musik/b.flac" \
  --stems-dir "<OpenDJDecks-Datenordner>/stems" \
  --record-dir /tmp/soak-rec --out soak-4-decks.jsonl
```

`--track PFAD=HASH` hängt die gespeicherten Stems eines Tracks an (der Inhalts-Hash ist der Name der Stem-Datei). `--buffer` und `--rate` ändern Puffergröße und Samplerate. Danach machst du aus den Logs diese Tabellen:

```sh
cargo run -p od-soak --bin soak-report -- soak-4-decks.jsonl
```

Lass immer nur eine Messung laufen; zwei gleichzeitig teilen sich denselben Prozessor. Wenn du uns dein Log schickst, schreib dazu, was der Rechner sonst noch gemacht hat.
