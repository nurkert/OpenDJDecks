---
title: Sets mit einer KI vorbereiten
description: Der MCP-Server lässt eine KI deine Bibliothek durchsuchen, Tonarten und Tempi prüfen und Sets bauen, während du jede Änderung in der App siehst.
keywords: [mcp, ki, ai, assistent, claude, chatgpt, llm, automatisierung, künstliche intelligenz, mit ki vorbereiten, werkzeuge]
features: [mcp-set-preparation]
---

`opendjdecks-mcp` ist ein kleines Programm, das eine KI wie Claude mit deiner OpenDJDecks-Bibliothek verbindet. Du beschreibst das gewünschte Set; die KI durchsucht deine Tracks, prüft Tonarten, Tempo und Energie, baut das Set und schreibt Übergangsnotizen. Jede Änderung erscheint sofort in der App und lässt sich dort rückgängig machen. Auch die [Lichtshow](light-show.md) kann sie verwalten.

> [!TIP]
> Lass die App offen, während du mit der KI arbeitest: Du siehst jede Änderung ankommen.

## Einrichten

Das Programm wird mit der App installiert:

| Installation | Pfad |
| --- | --- |
| macOS-App | `/Applications/OpenDJDecks.app/Contents/MacOS/opendjdecks-mcp` |
| Debian-/Ubuntu-Paket | `/usr/bin/opendjdecks-mcp` |
| Linux-Archiv | neben `opendjdecks` im entpackten Ordner |
| Raspberry-Pi-Paket | `/opt/opendjdecks/opendjdecks-mcp` |

In **Claude Desktop** öffnest du Settings › Developer › Edit Config und ergänzt:

```json
{
  "mcpServers": {
    "opendjdecks": {
      "command": "/Applications/OpenDJDecks.app/Contents/MacOS/opendjdecks-mcp",
      "args": []
    }
  }
}
```

Starte Claude Desktop neu, dann erscheinen die OpenDJDecks-Werkzeuge. In **Claude Code**: `claude mcp add opendjdecks -- "/Applications/OpenDJDecks.app/Contents/MacOS/opendjdecks-mcp"`. Andere Clients und die Optionen stehen in der [MCP-Server-Referenz](../../mcp.md).

## Was du fragen kannst

- "Bau aus meinem Deep House ein Warm-up-Set für eine Stunde mit langsam steigender Energie und schreib zu jedem Übergang eine Notiz."
- "Welche Tracks in meiner Bibliothek passen nach diesem, um 128 BPM?"
- "Verschieb die Tracks mit wenig Energie in eine neue Playlist."

Jede Änderung ist in der App ein Rückgängig-Schritt mit dem Namen "MCP: …" und geht wie jede andere auf deine USB-Sticks.
