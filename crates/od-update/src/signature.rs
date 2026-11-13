//! The signature on `SHA256SUMS.txt` (roadmap [42]).
//!
//! Every release carries `SHA256SUMS.txt.sig`: the raw 64-byte Ed25519
//! signature of the checksum list, made with the project's release key
//! (`packaging/sign-release.sh`). The public half is
//! `packaging/signing/release-key.pub.pem`; it is built in here, so an update
//! is only installed when the list was signed with that key, whatever the
//! download host serves. Anyone can check the same with OpenSSL alone, see
//! `docs/updates.md`.

use crate::Error;
use ring::signature::{ED25519, UnparsedPublicKey};

/// The signature asset next to the checksum list.
pub const SIG_NAME: &str = "SHA256SUMS.txt.sig";

/// The raw Ed25519 public key of `packaging/signing/release-key.pub.pem`.
pub const RELEASE_KEY: [u8; 32] = [
    0x92, 0xed, 0x66, 0x01, 0x8a, 0xab, 0xe0, 0x88, 0x76, 0x90, 0x31, 0xae, 0x41, 0x10, 0x57, 0xb9,
    0xc1, 0xb2, 0xc8, 0x21, 0xf9, 0xbe, 0x3e, 0xca, 0xb6, 0x7b, 0x64, 0xe7, 0xe6, 0x5f, 0x3e, 0x50,
];

/// Checks `sig` (raw, 64 bytes) over `sums` with `key`.
pub fn verify(key: &[u8; 32], sums: &[u8], sig: &[u8]) -> Result<(), Error> {
    if sig.len() != 64 {
        return Err(Error::Signature(format!(
            "{SIG_NAME} has {} bytes, not 64",
            sig.len()
        )));
    }
    UnparsedPublicKey::new(&ED25519, key)
        .verify(sums, sig)
        .map_err(|_| Error::Signature(format!("{SIG_NAME} does not match the release key")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    /// A fixed test key pair (never the release key).
    pub(crate) fn test_pair() -> Ed25519KeyPair {
        Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap()
    }

    pub(crate) fn test_key() -> [u8; 32] {
        test_pair().public_key().as_ref().try_into().unwrap()
    }

    pub(crate) fn sign(data: &[u8]) -> Vec<u8> {
        test_pair().sign(data).as_ref().to_vec()
    }

    #[test]
    fn accepts_its_own_signature_only() {
        let sums = b"abc  OpenDJDecks-0.2.0-x86_64.AppImage\n";
        let sig = sign(sums);
        verify(&test_key(), sums, &sig).unwrap();
        // One changed byte in the list, in the signature, or another key.
        let mut changed = sums.to_vec();
        changed[0] = b'b';
        assert!(matches!(
            verify(&test_key(), &changed, &sig),
            Err(Error::Signature(_))
        ));
        let mut bad = sig.clone();
        bad[10] ^= 1;
        assert!(verify(&test_key(), sums, &bad).is_err());
        assert!(verify(&RELEASE_KEY, sums, &sig).is_err());
        assert!(verify(&test_key(), sums, &sig[..63]).is_err());
    }

    /// The built-in key is the one in the PEM file people verify with.
    #[test]
    fn release_key_matches_the_published_pem() {
        let pem = include_str!("../../../packaging/signing/release-key.pub.pem");
        let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
        let der = base64_decode(&b64);
        // SubjectPublicKeyInfo for Ed25519: a fixed 12-byte prefix, then the key.
        assert_eq!(der.len(), 44);
        assert_eq!(
            &der[..12],
            &[
                0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00
            ]
        );
        assert_eq!(der[12..], RELEASE_KEY);
    }

    fn base64_decode(s: &str) -> Vec<u8> {
        const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        let (mut acc, mut bits) = (0u32, 0);
        for c in s.bytes().filter(|&c| c != b'=') {
            acc = acc << 6 | ABC.iter().position(|&a| a == c).unwrap() as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        out
    }
}
