//! Loading and indexing techniques (FR-F5, G7.3). Adding a technique is additive: drop a YAML file
//! into the content path and it appears in the catalog — no core changes (NFR-EXT3/EXT6).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use akumo_domain::error::{AkumoError, Result};
use akumo_domain::impact::ImpactLevel;

use crate::schema::Technique;
use crate::validate::validate;

/// Parse and validate a single technique from YAML.
pub fn parse_technique(yaml: &str) -> Result<Technique> {
    let technique: Technique = serde_yaml::from_str(yaml)
        .map_err(|e| AkumoError::Validation(format!("YAML parse error: {e}")))?;
    validate(&technique).map_err(|errs| AkumoError::Validation(errs.join("; ")))?;
    Ok(technique)
}

/// A discoverable, queryable set of validated techniques.
#[derive(Debug, Default)]
pub struct Catalog {
    techniques: Vec<Technique>,
    by_id: HashMap<String, usize>,
}

impl Catalog {
    /// An empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a validated technique, rejecting a duplicate id.
    pub fn insert(&mut self, technique: Technique) -> Result<()> {
        validate(&technique).map_err(|errs| AkumoError::Validation(errs.join("; ")))?;
        let id = technique.metadata.id.clone();
        if self.by_id.contains_key(&id) {
            return Err(AkumoError::Validation(format!("duplicate technique id '{id}'")));
        }
        self.by_id.insert(id, self.techniques.len());
        self.techniques.push(technique);
        Ok(())
    }

    /// Parse, validate, and insert a technique from YAML.
    pub fn insert_yaml(&mut self, yaml: &str) -> Result<()> {
        let technique = parse_technique(yaml)?;
        self.insert(technique)
    }

    /// Recursively load every `.yaml`/`.yml` technique under `dir`, returning how many loaded.
    pub fn load_dir(&mut self, dir: impl AsRef<Path>) -> Result<usize> {
        let mut files = Vec::new();
        collect_yaml(dir.as_ref(), &mut files)?;
        files.sort(); // deterministic load order
        let mut count = 0;
        for path in files {
            let content = fs::read_to_string(&path)?;
            let technique = parse_technique(&content)
                .map_err(|e| AkumoError::Validation(format!("{}: {e}", path.display())))?;
            self.insert(technique)?;
            count += 1;
        }
        Ok(count)
    }

    /// Look up a technique by id.
    pub fn get(&self, id: &str) -> Option<&Technique> {
        self.by_id.get(id).map(|&i| &self.techniques[i])
    }

    /// All techniques.
    pub fn all(&self) -> &[Technique] {
        &self.techniques
    }

    /// Number of techniques.
    pub fn len(&self) -> usize {
        self.techniques.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.techniques.is_empty()
    }

    /// Techniques for a given provider.
    pub fn by_provider(&self, provider: &str) -> Vec<&Technique> {
        self.techniques
            .iter()
            .filter(|t| t.metadata.provider == provider)
            .collect()
    }

    /// Techniques mapped to a given MITRE ATT&CK id.
    pub fn by_mitre(&self, mitre_id: &str) -> Vec<&Technique> {
        self.techniques
            .iter()
            .filter(|t| t.metadata.mitre.iter().any(|m| m == mitre_id))
            .collect()
    }

    /// Techniques of a given impact level.
    pub fn by_impact(&self, impact: ImpactLevel) -> Vec<&Technique> {
        self.techniques
            .iter()
            .filter(|t| t.metadata.impact == impact)
            .collect()
    }
}

/// Recursively collect `.yaml`/`.yml` files under `dir`.
fn collect_yaml(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_yaml(&path, out)?;
        } else if matches!(path.extension().and_then(|e| e.to_str()), Some("yaml" | "yml")) {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
metadata:
  id: aws.iam.create-access-key
  name: Create IAM Access Key
  description: Creates a long-term access key for a target IAM user.
  version: "0.1.0"
  provider: aws
  mitre: ["T1098"]
  impact: mutating-reversible
  expected_telemetry:
    - source: cloudtrail
      event_name: CreateAccessKey
contract:
  inputs:
    - name: user
      type: principal
      required: true
  preconditions:
    - predicate: { name: has_permission, args: ["$foothold", "iam:CreateAccessKey"] }
      min_status: proven
  effects:
    - predicate: { name: has_credential, args: ["$user", "new-key"] }
steps:
  - id: create-key
    body:
      type: call
      service: iam
      operation: CreateAccessKey
      params: { UserName: "$user" }
    bind:
      - name: key
        from: result.AccessKey
    revert:
      service: iam
      operation: DeleteAccessKey
      params: { UserName: "$user", AccessKeyId: "$key.AccessKeyId" }
"#;

    #[test]
    fn parses_and_validates_a_technique() {
        let t = parse_technique(VALID).unwrap();
        assert_eq!(t.metadata.id, "aws.iam.create-access-key");
        assert_eq!(t.metadata.impact, ImpactLevel::MutatingReversible);
        assert_eq!(t.steps.len(), 1);
        assert!(t.steps[0].revert.is_some());
        assert_eq!(t.contract.preconditions.len(), 1);
    }

    #[test]
    fn rejects_missing_mitre() {
        let yaml = r#"
metadata:
  id: x
  name: X
  description: d
  version: "0.1"
  provider: aws
  impact: read
  expected_telemetry:
    - source: cloudtrail
      event_name: GetUser
"#;
        let err = parse_technique(yaml).unwrap_err();
        assert!(matches!(err, AkumoError::Validation(_)));
    }

    #[test]
    fn rejects_irreversible_without_simulated_variant() {
        let yaml = r#"
metadata:
  id: x
  name: X
  description: d
  version: "0.1"
  provider: aws
  mitre: ["T1485"]
  impact: destructive
  expected_telemetry:
    - source: cloudtrail
      event_name: DeleteBucket
"#;
        let err = parse_technique(yaml).unwrap_err();
        assert!(matches!(err, AkumoError::Validation(_)));
    }

    #[test]
    fn rejects_script_step_without_capabilities() {
        let yaml = r#"
metadata:
  id: x
  name: X
  description: d
  version: "0.1"
  provider: aws
  mitre: ["T1078"]
  impact: read
  expected_telemetry:
    - source: cloudtrail
      event_name: GetCallerIdentity
steps:
  - id: compute
    body:
      type: script
      language: starlark
      source: "x = 1"
"#;
        let err = parse_technique(yaml).unwrap_err();
        assert!(matches!(err, AkumoError::Validation(_)));
    }

    #[test]
    fn catalog_indexes_and_queries() {
        let mut catalog = Catalog::new();
        catalog.insert_yaml(VALID).unwrap();
        assert_eq!(catalog.len(), 1);
        assert!(catalog.get("aws.iam.create-access-key").is_some());
        assert_eq!(catalog.by_provider("aws").len(), 1);
        assert_eq!(catalog.by_mitre("T1098").len(), 1);
        assert_eq!(catalog.by_impact(ImpactLevel::MutatingReversible).len(), 1);
        // Duplicate id is refused.
        assert!(catalog.insert_yaml(VALID).is_err());
    }

    #[test]
    fn loads_a_directory() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("akumo-dsl-test-{nanos}"));
        std::fs::create_dir_all(dir.join("aws")).unwrap();
        std::fs::write(dir.join("aws").join("create-key.yaml"), VALID).unwrap();

        let mut catalog = Catalog::new();
        let count = catalog.load_dir(&dir).unwrap();
        assert_eq!(count, 1);
        assert_eq!(catalog.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
