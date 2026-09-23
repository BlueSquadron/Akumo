//! Technique impact classification (FR-A3). Ordering matters: consent must be recorded *at or
//! above* a step's impact level, so the enum is declared least-to-most impactful and derives
//! `Ord` from that order.

use serde::{Deserialize, Serialize};

/// How consequential a technique/step is against the target.
///
/// Declared in increasing order so that `>=` comparisons express "at or above this impact".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImpactLevel {
    /// Read-only: observes the target, mutates nothing.
    Read,
    /// Mutates state but ships a tested revert (the safe-by-default case).
    MutatingReversible,
    /// Mutates state with no clean inverse; must be opt-in and prefer a simulated variant.
    MutatingIrreversible,
    /// Destructive impact; opt-in, gated, simulated wherever a real action cannot be reversed.
    Destructive,
}

impl ImpactLevel {
    /// Anything above [`ImpactLevel::Read`] mutates the target and requires consent (FR-A3).
    pub fn is_mutating(self) -> bool {
        self > ImpactLevel::Read
    }

    /// Whether a consent decision is required before executing at this level.
    pub fn requires_consent(self) -> bool {
        self.is_mutating()
    }

    /// Whether the engine can guarantee a clean revert for this level.
    pub fn is_reversible(self) -> bool {
        matches!(self, ImpactLevel::Read | ImpactLevel::MutatingReversible)
    }
}

impl std::fmt::Display for ImpactLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ImpactLevel::Read => "read",
            ImpactLevel::MutatingReversible => "mutating-reversible",
            ImpactLevel::MutatingIrreversible => "mutating-irreversible",
            ImpactLevel::Destructive => "destructive",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_reflects_severity() {
        assert!(ImpactLevel::Read < ImpactLevel::MutatingReversible);
        assert!(ImpactLevel::MutatingReversible < ImpactLevel::MutatingIrreversible);
        assert!(ImpactLevel::MutatingIrreversible < ImpactLevel::Destructive);
    }

    #[test]
    fn read_is_the_only_non_consent_level() {
        assert!(!ImpactLevel::Read.requires_consent());
        assert!(ImpactLevel::MutatingReversible.requires_consent());
        assert!(ImpactLevel::Destructive.requires_consent());
    }

    #[test]
    fn reversibility_matches_classification() {
        assert!(ImpactLevel::MutatingReversible.is_reversible());
        assert!(!ImpactLevel::MutatingIrreversible.is_reversible());
    }

    #[test]
    fn serde_uses_kebab_case() {
        let json = serde_json::to_string(&ImpactLevel::MutatingReversible).unwrap();
        assert_eq!(json, "\"mutating-reversible\"");
    }
}
