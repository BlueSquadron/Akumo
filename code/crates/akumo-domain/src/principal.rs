//! Principals — the identities the attack graph reasons about. This is the provider-neutral
//! attack-core view (ADR-0015); provider-specific inventory attaches as attributes at the graph
//! layer (Inc. 2 / G4), not here.

use serde::{Deserialize, Serialize};

use crate::ids::ProviderId;

/// The kind of an identity in the attack-core ontology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrincipalKind {
    /// A human/user identity.
    User,
    /// An assumable role.
    Role,
    /// A non-human service identity.
    ServiceIdentity,
    /// A group of principals.
    Group,
    /// A federated/external identity (IdP).
    ExternalIdentity,
}

/// A principal: an identity that can hold permissions and act.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Principal {
    /// Provider-native stable identifier (e.g. an ARN). Opaque to the core.
    pub id: String,
    /// The attack-core kind of this identity.
    pub kind: PrincipalKind,
    /// A human-friendly display name, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The provider this principal belongs to.
    pub provider: ProviderId,
}

impl Principal {
    /// Construct a principal.
    pub fn new(id: impl Into<String>, kind: PrincipalKind, provider: ProviderId) -> Self {
        Self { id: id.into(), kind, display_name: None, provider }
    }

    /// Attach a display name (builder-style).
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_display_name() {
        let p = Principal::new("arn:aws:iam::1:role/x", PrincipalKind::Role, ProviderId::new("aws"))
            .with_display_name("x");
        assert_eq!(p.display_name.as_deref(), Some("x"));
        assert_eq!(p.kind, PrincipalKind::Role);
    }
}
