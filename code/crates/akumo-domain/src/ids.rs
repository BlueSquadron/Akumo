//! Identifier and scalar newtypes shared across the domain.

use serde::{Deserialize, Serialize};

/// Declares a transparent `String` newtype id with the usual conversions and `Display`.
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Construct from anything string-like.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrow the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

string_id!(
    /// Opaque, unique identifier of an engagement/session.
    EngagementId
);
string_id!(
    /// Identifier of a provider adapter (e.g. `aws`, `mock`).
    ProviderId
);
string_id!(
    /// Identifier of a technique in the catalog.
    TechniqueId
);
string_id!(
    /// A provider region/partition identifier (e.g. `us-east-1`).
    Region
);
string_id!(
    /// Who performed an action (operator handle, CI actor, `system`).
    Actor
);
string_id!(
    /// A hex-encoded hash used in the event chain (full scheme in Inc. 2 / G2.1).
    EventHash
);

/// Monotonic per-engagement event sequence number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seq(pub u64);

impl Seq {
    /// The first sequence number in a stream.
    pub const ZERO: Seq = Seq(0);

    /// The next sequence number.
    pub fn next(self) -> Seq {
        Seq(self.0 + 1)
    }
}

/// A point in time, as Unix epoch milliseconds. Kept dependency-free on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Construct from Unix epoch milliseconds.
    pub fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// The Unix epoch milliseconds value.
    pub fn as_millis(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_id_roundtrips_through_json() {
        let id = EngagementId::new("eng-1");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"eng-1\"");
        let back: EngagementId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
        assert_eq!(id.to_string(), "eng-1");
    }

    #[test]
    fn seq_is_monotonic() {
        assert_eq!(Seq::ZERO.next(), Seq(1));
        assert!(Seq(1) < Seq(2));
    }
}
