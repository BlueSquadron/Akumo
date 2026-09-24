//! Telemetry & detection output (FR-I). Every technique carries its **expected telemetry signature**
//! (FR-I1), and Akumo emits **Sigma-aligned** candidate detections (ADR-0028) so findings
//! interoperate with the detection ecosystem out of the box. v1 emits expected signatures only; live
//! observed-vs-expected correlation is v2 (ADR-0008).

use serde::{Deserialize, Serialize};

use akumo_domain::impact::ImpactLevel;
use akumo_dsl::schema::{ExpectedTelemetry, Technique};

/// A Sigma-compatible detection rule (a practical subset of the Sigma schema).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SigmaRule {
    /// Rule title.
    pub title: String,
    /// A stable rule id.
    pub id: String,
    /// Rule status (candidate detections are `experimental`).
    pub status: String,
    /// What the rule detects.
    pub description: String,
    /// MITRE ATT&CK tags (`attack.tXXXX`).
    pub tags: Vec<String>,
    /// Where the events come from.
    pub logsource: SigmaLogSource,
    /// The detection body.
    pub detection: SigmaDetection,
    /// Severity.
    pub level: String,
}

/// The Sigma `logsource`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SigmaLogSource {
    /// e.g. `aws`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    /// e.g. `cloudtrail`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
}

/// The Sigma `detection` block: a `selection` map plus a `condition`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SigmaDetection {
    /// Field/value matches.
    pub selection: serde_json::Map<String, serde_json::Value>,
    /// The condition expression (v1 is always `selection`).
    pub condition: String,
}

impl SigmaRule {
    /// Render the rule as Sigma YAML.
    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(self).unwrap_or_default()
    }
}

/// Generate one candidate Sigma rule per expected-telemetry signature of a technique (FR-I3).
pub fn sigma_rules_for(technique: &Technique) -> Vec<SigmaRule> {
    technique
        .metadata
        .expected_telemetry
        .iter()
        .map(|sig| sigma_rule(technique, sig))
        .collect()
}

fn sigma_rule(technique: &Technique, sig: &ExpectedTelemetry) -> SigmaRule {
    let mut selection = serde_json::Map::new();
    selection.insert(
        "eventName".to_string(),
        serde_json::Value::String(sig.event_name.clone()),
    );
    for (k, v) in &sig.fields {
        selection.insert(k.clone(), v.clone());
    }

    let tags = technique
        .metadata
        .mitre
        .iter()
        .map(|t| format!("attack.{}", t.to_lowercase()))
        .collect();

    SigmaRule {
        title: format!("{} — {}", technique.metadata.name, sig.event_name),
        id: format!(
            "akumo-{}-{}",
            slug(&technique.metadata.id),
            slug(&sig.event_name)
        ),
        status: "experimental".to_string(),
        description: technique.metadata.description.clone(),
        tags,
        logsource: SigmaLogSource {
            product: Some(technique.metadata.provider.clone()),
            service: Some(sig.source.clone()),
        },
        detection: SigmaDetection {
            selection,
            condition: "selection".to_string(),
        },
        level: level_for(technique.metadata.impact).to_string(),
    }
}

fn level_for(impact: ImpactLevel) -> &'static str {
    match impact {
        ImpactLevel::Read => "low",
        ImpactLevel::MutatingReversible => "medium",
        ImpactLevel::MutatingIrreversible => "high",
        ImpactLevel::Destructive => "high",
    }
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_dsl::parse_technique;

    const TECH: &str = r#"
metadata:
  id: aws.iam.create-access-key
  name: Create IAM Access Key
  description: Creates a long-term access key for a target IAM user.
  version: "1.0.0"
  provider: aws
  mitre: ["T1098"]
  impact: mutating-reversible
  expected_telemetry:
    - source: cloudtrail
      event_name: CreateAccessKey
      fields:
        eventSource: iam.amazonaws.com
"#;

    #[test]
    fn generates_a_sigma_rule_per_signature() {
        let technique = parse_technique(TECH).unwrap();
        let rules = sigma_rules_for(&technique);
        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert!(rule.title.contains("CreateAccessKey"));
        assert_eq!(rule.tags, vec!["attack.t1098".to_string()]);
        assert_eq!(rule.logsource.service.as_deref(), Some("cloudtrail"));
        assert_eq!(rule.level, "medium");
        assert_eq!(
            rule.detection.selection.get("eventName"),
            Some(&serde_json::Value::String("CreateAccessKey".to_string()))
        );
        assert_eq!(
            rule.detection.selection.get("eventSource"),
            Some(&serde_json::Value::String("iam.amazonaws.com".to_string()))
        );
    }

    #[test]
    fn renders_to_sigma_yaml() {
        let technique = parse_technique(TECH).unwrap();
        let yaml = sigma_rules_for(&technique)[0].to_yaml();
        assert!(yaml.contains("logsource"));
        assert!(yaml.contains("detection"));
        assert!(yaml.contains("eventName"));
    }
}
