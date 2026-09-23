//! The event-chain hash (ADR-0014 / SPEC-D3, spec §2.7). Each event's hash is
//! `H(prev_hash ‖ canonical(payload))`, giving an immutable, tamper-evident chain (NFR-OBS3).
//!
//! `canonical(payload)` is the compact JSON serialization of the payload. `serde_json`'s object
//! map is a `BTreeMap` (the `preserve_order` feature is **not** enabled), so keys serialize in
//! sorted order and the bytes are deterministic across runs — exactly what a content hash needs.

use sha2::{Digest, Sha256};

use crate::ids::EventHash;

/// Compute an event hash from the predecessor hash and the canonical payload bytes.
pub fn hash_event(prev: Option<&EventHash>, canonical_payload: &[u8]) -> EventHash {
    let mut hasher = Sha256::new();
    if let Some(prev) = prev {
        hasher.update(prev.as_str().as_bytes());
    }
    hasher.update(canonical_payload);
    let digest = hasher.finalize();
    EventHash::new(to_hex(&digest))
}

/// Lowercase hex encoding, dependency-free (avoids pulling in a `hex` crate for the leaf).
pub fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // Writing to a String is infallible.
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encoding_is_lowercase_and_padded() {
        assert_eq!(to_hex(&[0x00, 0x0f, 0xff]), "000fff");
    }

    #[test]
    fn hash_is_deterministic_and_chains_on_prev() {
        let payload = br#"{"a":1}"#;
        let h1 = hash_event(None, payload);
        let h1_again = hash_event(None, payload);
        assert_eq!(h1, h1_again, "same inputs must hash identically");

        let h2 = hash_event(Some(&h1), payload);
        assert_ne!(h1, h2, "chaining on prev must change the hash");
        // SHA-256 hex is 64 chars.
        assert_eq!(h1.as_str().len(), 64);
    }
}
