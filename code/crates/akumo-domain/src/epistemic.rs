//! Epistemic status (ADR-0017 / SPEC-D6). Offensive enumeration is almost always *partial*, so
//! every attack-relevant assertion carries an explicit status. Conflating "unseen" with "absent"
//! would produce false negatives (missed real paths) and dishonest reports — unacceptable for an
//! authorized-testing tool.

use serde::{Deserialize, Serialize};

/// What we know about an assertion (a node, edge, or property).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpistemicStatus {
    /// Directly observed to be true.
    Proven,
    /// Directly observed to be false / not present.
    Absent,
    /// Could not be seen (access denied, throttled, out of scope). **Not** the same as `Absent`.
    Unknown,
    /// Derived/deduced rather than directly observed; may carry a confidence score.
    Inferred,
}

/// An epistemic judgement: a [`EpistemicStatus`] plus an optional confidence in `[0.0, 1.0]`
/// (meaningful for [`EpistemicStatus::Inferred`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Epistemic {
    /// The categorical status.
    pub status: EpistemicStatus,
    /// Optional confidence, used mainly for inferred assertions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

impl Epistemic {
    /// A proven assertion.
    pub const fn proven() -> Self {
        Self { status: EpistemicStatus::Proven, confidence: None }
    }

    /// An absent assertion (observed not to exist).
    pub const fn absent() -> Self {
        Self { status: EpistemicStatus::Absent, confidence: None }
    }

    /// An unknown assertion (unseen — a blind spot, never treated as absent).
    pub const fn unknown() -> Self {
        Self { status: EpistemicStatus::Unknown, confidence: None }
    }

    /// An inferred assertion with a confidence in `[0.0, 1.0]` (clamped).
    pub fn inferred(confidence: f32) -> Self {
        Self {
            status: EpistemicStatus::Inferred,
            confidence: Some(confidence.clamp(0.0, 1.0)),
        }
    }

    /// Whether the planner may rely on this as a certainty.
    pub fn is_proven(&self) -> bool {
        self.status == EpistemicStatus::Proven
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inferred_confidence_is_clamped() {
        assert_eq!(Epistemic::inferred(1.5).confidence, Some(1.0));
        assert_eq!(Epistemic::inferred(-0.2).confidence, Some(0.0));
    }

    #[test]
    fn unknown_is_distinct_from_absent() {
        assert_ne!(Epistemic::unknown().status, Epistemic::absent().status);
    }

    #[test]
    fn proven_serializes_without_confidence() {
        let json = serde_json::to_string(&Epistemic::proven()).unwrap();
        assert_eq!(json, "{\"status\":\"proven\"}");
    }
}
