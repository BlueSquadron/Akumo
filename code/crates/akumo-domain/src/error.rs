//! The single, typed error surface of the domain. A first-class [`AkumoError::AccessDenied`] lets
//! enumeration degrade gracefully under partial permissions (FR-C6) rather than throwing, and
//! [`AkumoError::OutOfScope`] makes scope refusals explicit (FR-A1). Errors are intentionally not
//! `Clone` (they may wrap `std::io::Error`).

use thiserror::Error;

/// The domain-wide result alias.
pub type Result<T> = std::result::Result<T, AkumoError>;

/// Every fallible domain operation returns this typed error.
#[derive(Debug, Error)]
pub enum AkumoError {
    /// A provider call was denied by permissions. Recorded as a coverage gap, never fatal.
    #[error("access denied: {0}")]
    AccessDenied(String),

    /// An action targeted something outside the engagement's authorized scope.
    #[error("out of scope: {0}")]
    OutOfScope(String),

    /// The provider throttled us; callers back off within the safety caps (NFR-REL3).
    #[error("throttled: {0}")]
    Throttled(String),

    /// A referenced entity (technique, engagement, node) does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// Content or input failed validation (schema, contract, version).
    #[error("validation error: {0}")]
    Validation(String),

    /// A provider adapter reported an error.
    #[error("provider error: {0}")]
    Provider(String),

    /// The persistence layer (event store) reported an error.
    #[error("persistence error: {0}")]
    Persistence(String),

    /// A capability is not supported (e.g. v2-only telemetry collection).
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// The event chain or state integrity check failed (tamper-evidence, NFR-OBS3).
    #[error("integrity error: {0}")]
    Integrity(String),

    /// A serialization/deserialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// An underlying I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A catch-all for errors that do not (yet) have a dedicated variant.
    #[error("{0}")]
    Message(String),
}

impl From<serde_json::Error> for AkumoError {
    fn from(e: serde_json::Error) -> Self {
        AkumoError::Serialization(e.to_string())
    }
}

impl AkumoError {
    /// Whether this error represents a permission/visibility gap rather than a hard failure —
    /// enumeration treats these as coverage gaps (FR-C6), not engagement failures.
    pub fn is_access_gap(&self) -> bool {
        matches!(self, AkumoError::AccessDenied(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_denied_is_a_gap() {
        assert!(AkumoError::AccessDenied("iam:ListUsers".into()).is_access_gap());
        assert!(!AkumoError::OutOfScope("acct".into()).is_access_gap());
    }

    #[test]
    fn json_error_maps_to_serialization() {
        let err: AkumoError = serde_json::from_str::<i32>("not-json").unwrap_err().into();
        assert!(matches!(err, AkumoError::Serialization(_)));
    }
}
