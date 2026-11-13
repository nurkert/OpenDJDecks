# Updates

OpenDJDecks looks for new versions on GitHub Releases 10 seconds after it starts and every six hours while it runs. It reads only the public release list. After a failed check it tries again after 15 minutes, and Preferences > Advanced shows the reason.

When a new version is found, a dialog asks once per version, and never while a deck plays or a mix is being recorded. The bar under the top bar keeps offering it. After **UPDATE NOW** the app downloads the file, checks the release's `SHA256SUMS.txt` against its signature `SHA256SUMS.txt.sig`, verifies the file against that list, prepares it next to the installed copy, and restarts, unless a deck plays or you are in Perform. Then **RESTART** finishes it, and otherwise the update is applied when you quit.

| Install | What happens | Falls back to |
|---|---|---|
| macOS app | The new bundle replaces the old one after quit | Opens the verified disk image if the folder is not writable or the app runs from a disk image or was translocated |
| Windows zip | The program folder is swapped after quit | Release page if the folder's parent is not writable |
| Linux AppImage | The file is replaced after quit | Release page if the folder is not writable |
| Linux tar.gz | The program folder is swapped after quit | Release page if the folder's parent is not writable |
| Debian package | Installed through the system's password prompt | Release page if `pkexec` is missing |

A release without a valid signature is never installed in place: the app opens its release page instead. A swap keeps the old version until the new one is in place and restores it on failure. Library and settings live outside the program folder.

`cargo test -p od-update --test live -- --ignored` runs the whole path against the newest real release.

## Signed releases

The checksum list of every release is signed with the project's Ed25519 release key. The public key is `packaging/signing/release-key.pub.pem`, built into the app (`od-update/src/signature.rs`, a test keeps both the same) and published on the website as `/release-key.pub.pem`, so a download can be checked without trusting GitHub; the commands are in the getting-started guide under "Checking a download". The private key never enters the repository or GitHub.

After the release workflow has published a release, the maintainer signs it:

```sh
packaging/sign-release.sh v0.11.0 /path/to/release-signing-key.pem
```

The script checks that the key belongs to the published public key, signs `SHA256SUMS.txt` with OpenSSL, verifies the signature the way users do and attaches `SHA256SUMS.txt.sig` to the release. Until then the app offers the release page instead of installing in place. `cargo run -p od-update --example check -- --as 0.9.0 --sig` checks a published release's signature with the app's own code.
