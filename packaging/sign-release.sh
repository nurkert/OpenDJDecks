#!/usr/bin/env bash
# Signs a published release's checksum list and attaches the signature
# (roadmap [42], docs/updates.md). Run by the maintainer after the release
# workflow has published the release; the app installs an update in place only
# once SHA256SUMS.txt.sig is there.
#
#   packaging/sign-release.sh v0.11.0 /path/to/release-signing-key.pem
#
# The private key never enters the repository or GitHub. Needs OpenSSL 3 or
# newer (Ed25519 with -rawin) and the GitHub CLI.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

tag="${1:?usage: sign-release.sh <tag> <private-key.pem>}"
key="${2:?usage: sign-release.sh <tag> <private-key.pem>}"
repo=nurkert/OpenDJDecks
pub=packaging/signing/release-key.pub.pem

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# The private key must belong to the published public key.
openssl pkey -in "$key" -pubout | cmp -s - "$pub" \
  || { echo "the key does not match $pub" >&2; exit 1; }

gh release download "$tag" --repo "$repo" --pattern SHA256SUMS.txt --dir "$work"
openssl pkeyutl -sign -rawin -inkey "$key" \
  -in "$work/SHA256SUMS.txt" -out "$work/SHA256SUMS.txt.sig"
# The same check anyone can run (docs/updates.md).
openssl pkeyutl -verify -rawin -pubin -inkey "$pub" \
  -in "$work/SHA256SUMS.txt" -sigfile "$work/SHA256SUMS.txt.sig"

gh release upload "$tag" "$work/SHA256SUMS.txt.sig" --repo "$repo" --clobber
echo "signed $tag"
