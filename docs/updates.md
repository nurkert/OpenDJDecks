# Updates

OpenDJDecks looks for new versions on GitHub Releases 10 seconds after it starts and every six hours while it runs. It reads only the public release list. After a failed check it tries again after 15 minutes, and Preferences > Advanced shows the reason.

When a new version is found, a dialog asks once per version, and never while a deck plays or a mix is being recorded. The bar under the top bar keeps offering it. After **UPDATE NOW** the app downloads the file, verifies it against the release's `SHA256SUMS.txt`, prepares it next to the installed copy, and restarts, unless a deck plays or you are in Perform. Then **RESTART** finishes it, and otherwise the update is applied when you quit.

| Install | What happens | Falls back to |
|---|---|---|
| macOS app | The new bundle replaces the old one after quit | Opens the verified disk image if the folder is not writable or the app runs from a disk image or was translocated |
| Windows zip | The program folder is swapped after quit | Release page if the folder's parent is not writable |
| Linux AppImage | The file is replaced after quit | Release page if the folder is not writable |
| Linux tar.gz | The program folder is swapped after quit | Release page if the folder's parent is not writable |
| Debian package | Installed through the system's password prompt | Release page if `pkexec` is missing |

A swap keeps the old version until the new one is in place and restores it on failure. Library and settings live outside the program folder.

`cargo test -p od-update --test live -- --ignored` runs the whole path against the newest real release.
