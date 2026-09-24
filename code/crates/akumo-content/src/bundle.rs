//! The content bundle (ADR-0031 / SPEC-D19): a **versioned, signed** manifest over a set of
//! techniques. The engine ships a default catalog but loads techniques from a bundle that is
//! versioned (NFR-MNT3) and signed (NFR-SEC5), updatable independently of the engine and
//! offline-capable. Signatures are Ed25519 over the canonical manifest bytes; each technique is
//! pinned by its SHA-256 so a swapped technique fails verification.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use akumo_domain::error::{AkumoError, Result};
use akumo_domain::hash::to_hex;
use akumo_dsl::schema::Technique;

/// One technique's entry in the manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleEntry {
    /// Technique id.
    pub id: String,
    /// Technique content version.
    pub version: String,
    /// SHA-256 (hex) of the technique's canonical serialization.
    pub sha256: String,
}

/// The signed manifest describing a content bundle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Bundle name.
    pub name: String,
    /// Bundle version (semver).
    pub version: String,
    /// The engine/content-model compatibility range this bundle targets.
    pub engine_compat: String,
    /// One entry per technique.
    pub entries: Vec<BundleEntry>,
}

impl BundleManifest {
    /// Deterministic bytes to hash/sign (JSON with sorted object keys).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// SHA-256 (hex) of a technique's canonical serialization.
pub fn technique_hash(technique: &Technique) -> Result<String> {
    let bytes = serde_json::to_vec(technique)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(to_hex(&hasher.finalize()))
}

/// Build a manifest pinning each technique by content hash.
pub fn build_manifest(
    name: impl Into<String>,
    version: impl Into<String>,
    engine_compat: impl Into<String>,
    techniques: &[Technique],
) -> Result<BundleManifest> {
    let mut entries = Vec::with_capacity(techniques.len());
    for technique in techniques {
        entries.push(BundleEntry {
            id: technique.metadata.id.clone(),
            version: technique.metadata.version.clone(),
            sha256: technique_hash(technique)?,
        });
    }
    Ok(BundleManifest {
        name: name.into(),
        version: version.into(),
        engine_compat: engine_compat.into(),
        entries,
    })
}

/// Sign a manifest, returning the detached signature as hex.
pub fn sign_manifest(manifest: &BundleManifest, signing_key: &SigningKey) -> Result<String> {
    let signature = signing_key.sign(&manifest.canonical_bytes()?);
    Ok(to_hex(&signature.to_bytes()))
}

/// Verify a manifest's detached hex signature against a trusted verifying key.
pub fn verify_manifest(
    manifest: &BundleManifest,
    signature_hex: &str,
    verifying_key: &VerifyingKey,
) -> Result<()> {
    let bytes = from_hex(signature_hex)?;
    let array: [u8; 64] = bytes
        .try_into()
        .map_err(|_| AkumoError::Integrity("signature must be 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&array);
    verifying_key
        .verify(&manifest.canonical_bytes()?, &signature)
        .map_err(|e| AkumoError::Integrity(format!("bundle signature invalid: {e}")))
}

/// Verify a full bundle: every technique's hash matches its manifest entry, and the manifest
/// signature is valid (NFR-SEC5).
pub fn verify_bundle(
    manifest: &BundleManifest,
    techniques: &[Technique],
    signature_hex: &str,
    verifying_key: &VerifyingKey,
) -> Result<()> {
    if manifest.entries.len() != techniques.len() {
        return Err(AkumoError::Integrity(format!(
            "bundle has {} techniques but the manifest lists {}",
            techniques.len(),
            manifest.entries.len()
        )));
    }
    for technique in techniques {
        let hash = technique_hash(technique)?;
        let entry = manifest
            .entries
            .iter()
            .find(|e| e.id == technique.metadata.id)
            .ok_or_else(|| {
                AkumoError::Integrity(format!(
                    "technique '{}' is not in the manifest",
                    technique.metadata.id
                ))
            })?;
        if entry.sha256 != hash {
            return Err(AkumoError::Integrity(format!(
                "technique '{}' hash does not match the manifest (tampered?)",
                technique.metadata.id
            )));
        }
    }
    verify_manifest(manifest, signature_hex, verifying_key)
}

/// Encode a trusted verifying key as hex (for distribution/pinning).
pub fn verifying_key_to_hex(verifying_key: &VerifyingKey) -> String {
    to_hex(&verifying_key.to_bytes())
}

/// Parse a trusted verifying key from hex.
pub fn verifying_key_from_hex(hex: &str) -> Result<VerifyingKey> {
    let bytes = from_hex(hex)?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| AkumoError::Validation("verifying key must be 32 bytes".to_string()))?;
    VerifyingKey::from_bytes(&array)
        .map_err(|e| AkumoError::Validation(format!("invalid verifying key: {e}")))
}

fn from_hex(hex: &str) -> Result<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return Err(AkumoError::Validation(
            "hex string has odd length".to_string(),
        ));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| AkumoError::Validation(format!("invalid hex: {e}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_dsl::parse_technique;

    const SAMPLE: &str = r#"
metadata:
  id: aws.iam.create-access-key
  name: Create IAM Access Key
  description: Creates a long-term access key.
  version: "1.0.0"
  provider: aws
  mitre: ["T1098"]
  impact: mutating-reversible
  expected_telemetry:
    - source: cloudtrail
      event_name: CreateAccessKey
steps:
  - id: create
    body:
      type: call
      service: iam
      operation: CreateAccessKey
      params: {}
    revert:
      service: iam
      operation: DeleteAccessKey
      params: {}
"#;

    #[test]
    fn sign_and_verify_roundtrip() {
        let technique = parse_technique(SAMPLE).unwrap();
        let manifest = build_manifest(
            "aws-default",
            "1.0.0",
            ">=0",
            std::slice::from_ref(&technique),
        )
        .unwrap();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let signature = sign_manifest(&manifest, &signing_key).unwrap();
        verify_manifest(&manifest, &signature, &verifying_key).unwrap();
        verify_bundle(
            &manifest,
            std::slice::from_ref(&technique),
            &signature,
            &verifying_key,
        )
        .unwrap();

        // Key round-trips through hex.
        let vk = verifying_key_from_hex(&verifying_key_to_hex(&verifying_key)).unwrap();
        verify_manifest(&manifest, &signature, &vk).unwrap();
    }

    #[test]
    fn tampered_technique_fails_verification() {
        let technique = parse_technique(SAMPLE).unwrap();
        let manifest = build_manifest(
            "aws-default",
            "1.0.0",
            ">=0",
            std::slice::from_ref(&technique),
        )
        .unwrap();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let signature = sign_manifest(&manifest, &signing_key).unwrap();

        let mut tampered = technique.clone();
        tampered.metadata.description = "malicious change".to_string();
        assert!(verify_bundle(&manifest, &[tampered], &signature, &verifying_key).is_err());
    }

    #[test]
    fn wrong_key_fails_verification() {
        let technique = parse_technique(SAMPLE).unwrap();
        let manifest = build_manifest(
            "aws-default",
            "1.0.0",
            ">=0",
            std::slice::from_ref(&technique),
        )
        .unwrap();
        let signature = sign_manifest(&manifest, &SigningKey::from_bytes(&[7u8; 32])).unwrap();
        let other = SigningKey::from_bytes(&[9u8; 32]).verifying_key();
        assert!(verify_manifest(&manifest, &signature, &other).is_err());
    }
}
