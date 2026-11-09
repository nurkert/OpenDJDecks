---
title: Import your library
description: Bring an existing library in once, as a library XML file, with cues, grids, keys and playlists.
keywords: [rekordbox, import, xml, library xml, itunes, apple music, music library, transfer, settings, shortcuts]
features: [browser-xml-itunes-exchange, import-settings, import-encrypted-library]
---

OpenDJDecks can read a library from a **library XML file**, so you start with your cues, beatgrids, keys and playlists in place.

## Create the XML file

In rekordbox, export your collection as an XML file from the File menu and save it somewhere you can find it. If you have rekordbox on this computer, **File › Import rekordbox Preferences and Shortcuts…** (also under **Preferences › Advanced**) carries over its preferences and keyboard shortcuts as well, wherever they do not clash with existing ones.

## Import the file

1. Open **File › Import Library XML…** (`⇧⌥⌘X`), or use **IMPORT LIBRARY** on the empty library screen.
2. Choose the XML file. Tracks, playlists, cues, beatgrids and keys arrive in your library.

Cover art is taken from the files themselves. Your music files stay where they are. If a track can no longer be found afterwards, see [moved files](library.md#moved-files).

## Other formats

- **File › Export Library XML…** writes your library as a file other DJ software can read (`⌥⌘X`).
- **File › Import Music / iTunes Library XML…** reads a `Library.xml` exported from Apple Music or iTunes, with tags, ratings and playlist folders (`⇧⌥⌘M`).

To move your whole OpenDJDecks library to a new computer, use the library archive instead; see [Moving to a new computer](library.md#moving-to-a-new-computer).

## Encrypted library database

From version 0.6, OpenDJDecks can also read an encrypted library database directly, with tracks, cues, beatgrids, keys, playlists, smart playlists (as ordinary playlists with their contents at import time), cover art and the play history. This is optional; the XML route above needs no key.

1. Choose **File › Import an Encrypted Library Database…**.
2. **CHOOSE FILE**: pick the database file of your library yourself. OpenDJDecks never searches your computer for one.
3. Under **ADVANCED**, enter the key for this library.
4. Press **IMPORT**.

You need the key for your own library. OpenDJDecks neither contains nor fetches it, and it cannot help you find it. The key is used for this import only: it is not saved or logged. Your database file is only read, never changed, and the temporary decrypted copy is removed when the import ends, also if it fails. If the key is wrong, you see the message "The key was not accepted", and nothing is imported.

Lists arrive in a **Library import** folder. Running the import again adds nothing twice.
