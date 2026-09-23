//! # akumo-content
//!
//! The technique **content bundle** (ADR-0031): a versioned, signed, independently-updatable
//! catalog the engine loads from a content path. The engine ships with a default catalog (under the
//! repo's `content/` tree) but can load newer bundles without an engine release (NFR-EXT6), staying
//! offline-capable.
//!
//! - [`bundle`] — the signed manifest: build, sign (Ed25519), and verify (with per-technique
//!   SHA-256 pinning).
//! - Loading and validating techniques from YAML is provided by [`akumo_dsl::Catalog`].

#![forbid(unsafe_code)]

pub mod bundle;

pub use akumo_domain as domain;
pub use bundle::{
    build_manifest, sign_manifest, technique_hash, verify_bundle, verify_manifest,
    verifying_key_from_hex, verifying_key_to_hex, BundleEntry, BundleManifest,
};

#[cfg(test)]
mod tests {
    use akumo_dsl::Catalog;

    #[test]
    fn ships_a_valid_default_catalog() {
        // The default catalog lives at <repo>/code/content; this crate is at code/crates/akumo-content.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        if !dir.exists() {
            return; // tolerate running from an unusual layout
        }
        let mut catalog = Catalog::new();
        let loaded = catalog.load_dir(&dir).expect("default catalog must load and validate");
        assert!(loaded >= 5, "expected at least the seeded techniques, got {loaded}");
        // Every technique carries a MITRE mapping and expected telemetry (validated on load).
        assert!(catalog.by_provider("aws").len() >= 5);
    }
}
