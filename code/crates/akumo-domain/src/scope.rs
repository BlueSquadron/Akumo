//! Authorized target scope (FR-A1, NFR-COMP2). An engagement declares its scope up front; every
//! provider call is checked against it, and out-of-scope targets are refused and logged. The scope
//! model is provider-neutral: a selector is a `kind`/`value` pair (e.g. `account` / `123456789012`)
//! that the provider adapter interprets.

use serde::{Deserialize, Serialize};

/// One authorized boundary within a scope.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeSelector {
    /// The selector dimension (e.g. `account`, `organization`, `region`, `resource`).
    pub kind: String,
    /// The value within that dimension.
    pub value: String,
}

impl ScopeSelector {
    /// Build a selector.
    pub fn new(kind: impl Into<String>, value: impl Into<String>) -> Self {
        Self { kind: kind.into(), value: value.into() }
    }
}

/// The declared authorized scope of an engagement.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    /// Allowed boundaries. An empty scope authorizes nothing (fail-closed).
    #[serde(default)]
    pub allowed: Vec<ScopeSelector>,
}

impl Scope {
    /// Construct from a list of selectors.
    pub fn new(allowed: Vec<ScopeSelector>) -> Self {
        Self { allowed }
    }

    /// Whether the scope authorizes nothing (no selectors). Enumeration/execution must refuse.
    pub fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }

    /// Whether a given `kind`/`value` target falls within the declared scope.
    pub fn contains(&self, kind: &str, value: &str) -> bool {
        self.allowed
            .iter()
            .any(|s| s.kind == kind && s.value == value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_scope_authorizes_nothing() {
        let scope = Scope::default();
        assert!(scope.is_empty());
        assert!(!scope.contains("account", "123456789012"));
    }

    #[test]
    fn contains_matches_declared_selectors() {
        let scope = Scope::new(vec![ScopeSelector::new("account", "123456789012")]);
        assert!(scope.contains("account", "123456789012"));
        assert!(!scope.contains("account", "999999999999"));
        assert!(!scope.contains("region", "123456789012"));
    }
}
