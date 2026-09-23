//! Technique validation (FR-F2, G7.5). A technique that fails validation is not loaded. Validation
//! is structural and declarative — it never executes step bodies.

use std::collections::HashSet;

use akumo_domain::impact::ImpactLevel;

use crate::schema::{StepBody, Technique};

/// Validate a technique, returning all problems found (empty `Ok` means valid).
pub fn validate(technique: &Technique) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();
    let meta = &technique.metadata;

    // Mandatory metadata (FR-F2).
    if meta.id.trim().is_empty() {
        errors.push("metadata.id is required".to_string());
    }
    if meta.name.trim().is_empty() {
        errors.push("metadata.name is required".to_string());
    }
    if meta.description.trim().is_empty() {
        errors.push("metadata.description is required".to_string());
    }
    if meta.version.trim().is_empty() {
        errors.push("metadata.version is required".to_string());
    }
    if meta.provider.trim().is_empty() {
        errors.push("metadata.provider is required".to_string());
    }
    if meta.mitre.is_empty() {
        errors.push("at least one MITRE ATT&CK mapping is required (FR-F2)".to_string());
    }
    if meta.expected_telemetry.is_empty() {
        errors.push("expected_telemetry is required — every action carries its detection meaning (FR-I1)".to_string());
    }

    // Irreversible/destructive techniques must offer a simulated variant (FR-G6).
    if matches!(
        meta.impact,
        ImpactLevel::MutatingIrreversible | ImpactLevel::Destructive
    ) && meta.simulated_variant.is_none()
    {
        errors.push(format!(
            "impact '{}' requires a simulated_variant (FR-G6)",
            meta.impact
        ));
    }

    // Steps: unique non-empty ids; script steps must declare capabilities (NFR-SEC5).
    let mut seen_ids: HashSet<&str> = HashSet::new();
    for step in &technique.steps {
        if step.id.trim().is_empty() {
            errors.push("a step is missing an id".to_string());
        } else if !seen_ids.insert(step.id.as_str()) {
            errors.push(format!("duplicate step id '{}'", step.id));
        }
        if let StepBody::Script { capabilities, .. } = &step.body {
            if capabilities.is_empty() {
                errors.push(format!(
                    "script step '{}' must declare the capabilities it needs (NFR-SEC5)",
                    step.id
                ));
            }
        }
    }

    // Contract predicate names must be present.
    for pre in &technique.contract.preconditions {
        if pre.predicate.name.trim().is_empty() {
            errors.push("a precondition has an empty predicate name".to_string());
        }
    }
    for eff in &technique.contract.effects {
        if eff.predicate.name.trim().is_empty() {
            errors.push("an effect has an empty predicate name".to_string());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
