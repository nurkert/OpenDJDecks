---
title: Preparing sets with an AI assistant
description: The MCP server lets an AI assistant search your library, check keys and tempos and build sets, while you watch every change in the app.
keywords: [mcp, ai, assistant, claude, chatgpt, llm, automation, artificial intelligence, prepare with ai, tools]
features: [mcp-set-preparation]
---

`opendjdecks-mcp` is a small program that connects an AI assistant, such as Claude, to your OpenDJDecks library. You describe the set you want; the assistant searches your tracks, checks keys, tempo and energy, builds the set and writes transition notes. Every change appears in the app as it happens and can be undone there. It can also manage the [light show](light-show.md).

> [!TIP]
> Keep the app open while you work with the assistant: you see each change arrive.

## Set it up

The program is installed with the app:

| Install | Path |
| --- | --- |
| macOS app | `/Applications/OpenDJDecks.app/Contents/MacOS/opendjdecks-mcp` |
| Debian/Ubuntu package | `/usr/bin/opendjdecks-mcp` |
| Linux archive | next to `opendjdecks` in the unpacked folder |
| Raspberry Pi bundle | `/opt/opendjdecks/opendjdecks-mcp` |

In **Claude Desktop**, open Settings › Developer › Edit Config and add:

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

Restart Claude Desktop, and the OpenDJDecks tools appear. In **Claude Code**: `claude mcp add opendjdecks -- "/Applications/OpenDJDecks.app/Contents/MacOS/opendjdecks-mcp"`. Other clients and the options are in the [MCP server reference](../../mcp.md).

## What you can ask

- "Build a one-hour warm-up set from my deep house with a slow energy climb, and write a note for each transition."
- "Which tracks in my library work after this one, around 128 BPM?"
- "Move the tracks with low energy to a new playlist."

Every write is one undo step in the app, named "MCP: …", and goes to your USB sticks like any other change.
