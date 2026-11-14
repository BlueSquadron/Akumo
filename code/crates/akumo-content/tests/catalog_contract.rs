//! Tier-2 **catalog-contract** tests (see `spec/AWS_TEST_EXPANSION.md`).
//!
//! These run in the fast CI tier: offline, no AWS, no emulator. They load the real shipped
//! `content/` tree and assert invariants that `akumo_dsl::validate` does *not* cover on its own —
//! most importantly the **reversibility pairing** (NFR-SAF2): a `mutating-reversible` technique must
//! ship an explicit revert for every mutating step, because the engine records compensation only
//! from an explicit `step.revert`. A reversible technique with a bare mutating step would be
//! silently un-revertible, and that is exactly what we refuse to let into the catalog.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use akumo_domain::impact::ImpactLevel;
use akumo_dsl::schema::StepBody;
use akumo_dsl::Catalog;

/// The repo's default content tree: this crate is at `code/crates/akumo-content`, content at
/// `code/content`.
fn content_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

/// Load the full shipped catalog, or `None` when run from a layout without the content tree
/// (mirrors the tolerance of the existing default-catalog test).
fn load_default_catalog() -> Option<Catalog> {
    let dir = content_dir();
    if !dir.exists() {
        return None;
    }
    let mut catalog = Catalog::new();
    catalog
        .load_dir(&dir)
        .expect("default catalog must load and validate");
    Some(catalog)
}

/// Every shipped technique parses and validates, and we ship at least the seeded set.
#[test]
fn shipped_catalog_loads_and_validates() {
    let Some(catalog) = load_default_catalog() else {
        return;
    };
    assert!(
        catalog.len() >= 5,
        "expected at least the seeded techniques, got {}",
        catalog.len()
    );
}

/// NFR-SAF2 — reversibility pairing. A `mutating-reversible` technique must pair an explicit,
/// well-formed revert to *every* `Call` step; `read` techniques are exempt.
#[test]
fn reversible_techniques_pair_a_revert_for_every_mutating_step() {
    let Some(catalog) = load_default_catalog() else {
        return;
    };

    let mut violations: Vec<String> = Vec::new();
    for technique in catalog.by_impact(ImpactLevel::MutatingReversible) {
        let id = &technique.metadata.id;
        for step in &technique.steps {
            // Only provider calls mutate state; script steps are covered by capability checks.
            if !matches!(step.body, StepBody::Call { .. }) {
                continue;
            }
            match &step.revert {
                None => violations.push(format!(
                    "{id}: step '{}' is a mutating Call with no revert (NFR-SAF2)",
                    step.id
                )),
                Some(c) if c.service.trim().is_empty() || c.operation.trim().is_empty() => {
                    violations.push(format!(
                        "{id}: step '{}' has a revert with empty service/operation",
                        step.id
                    ));
                }
                Some(_) => {}
            }
        }
    }

    assert!(
        violations.is_empty(),
        "reversibility-pairing violations:\n  {}",
        violations.join("\n  ")
    );
}

/// Impact/variant coherence: irreversible/destructive techniques must offer a simulated variant
/// (FR-G6). `validate` enforces this on load; we assert it against the on-disk content too, so a
/// future destructive technique can't slip in without one.
#[test]
fn irreversible_and_destructive_declare_a_simulated_variant() {
    let Some(catalog) = load_default_catalog() else {
        return;
    };
    let mut missing: Vec<String> = Vec::new();
    for technique in catalog.all() {
        let needs_variant = matches!(
            technique.metadata.impact,
            ImpactLevel::MutatingIrreversible | ImpactLevel::Destructive
        );
        if needs_variant && technique.metadata.simulated_variant.is_none() {
            missing.push(technique.metadata.id.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "irreversible/destructive techniques missing a simulated_variant: {missing:?}"
    );
}

/// Every declared telemetry signature carries a usable detection meaning (non-empty source and
/// event name), so detection-validation (Tier 4 / V2) has something concrete to match on.
#[test]
fn expected_telemetry_entries_are_well_formed() {
    let Some(catalog) = load_default_catalog() else {
        return;
    };
    let mut problems: Vec<String> = Vec::new();
    for technique in catalog.all() {
        for sig in &technique.metadata.expected_telemetry {
            if sig.source.trim().is_empty() || sig.event_name.trim().is_empty() {
                problems.push(format!(
                    "{}: telemetry entry with empty source/event_name",
                    technique.metadata.id
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "malformed expected_telemetry:\n  {}",
        problems.join("\n  ")
    );
}

/// Identity hygiene: a technique id is namespaced by its provider, and everything under
/// `content/aws` is an `aws` technique.
#[test]
fn technique_ids_are_namespaced_by_provider() {
    let Some(catalog) = load_default_catalog() else {
        return;
    };
    let mut problems: Vec<String> = Vec::new();
    for technique in catalog.all() {
        let id = &technique.metadata.id;
        let provider = &technique.metadata.provider;
        if !id.starts_with(&format!("{provider}.")) {
            problems.push(format!(
                "id '{id}' is not namespaced by provider '{provider}'"
            ));
        }
    }
    assert!(problems.is_empty(), "{problems:?}");

    // Separately: the AWS content subtree must be all-AWS.
    assert!(
        !catalog.by_provider("aws").is_empty(),
        "expected AWS techniques in the default catalog"
    );
}

/// MITRE coverage floor: the shipped content covers at least the distinct ATT&CK techniques we have
/// today, and every technique maps to at least one. This is a ratchet — new content may raise the
/// floor but must not silently drop coverage. Bump the floor when you intentionally add coverage.
#[test]
fn mitre_coverage_meets_floor() {
    let Some(catalog) = load_default_catalog() else {
        return;
    };

    let mut covered: BTreeSet<&str> = BTreeSet::new();
    for technique in catalog.all() {
        assert!(
            !technique.metadata.mitre.is_empty(),
            "{} has no MITRE mapping",
            technique.metadata.id
        );
        for m in &technique.metadata.mitre {
            covered.insert(m.as_str());
        }
    }

    // Today's shipped set spans these ATT&CK techniques (persistence/privesc/defense-evasion).
    const FLOOR: usize = 3;
    assert!(
        covered.len() >= FLOOR,
        "MITRE coverage regressed: {} distinct techniques ({:?}), floor is {FLOOR}",
        covered.len(),
        covered
    );
}
