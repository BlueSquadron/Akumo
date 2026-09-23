//! The scriptable synthetic environment behind the Mock provider (ADR-0030). It is authored
//! directly in terms of the attack-core vocabulary (principals, graph assertions), scripted
//! enumeration outcomes (including denials, to exercise partial-permission handling, FR-C6), and
//! action results — deliberately *not* an AWS simulator.

use std::collections::HashMap;

use akumo_domain::graph::Assertion;
use akumo_domain::ids::{ProviderId, Region};
use akumo_domain::principal::Principal;
use akumo_domain::seam::ActionResult;

/// What a scripted enumeration call returns.
#[derive(Clone, Debug)]
pub enum EnumOutcome {
    /// The graph assertions this call reveals.
    Assertions(Vec<Assertion>),
    /// The call is denied (recorded as a coverage gap by enumeration, FR-C6).
    Denied(String),
}

/// A synthetic environment: a current principal, regions, scripted enumerations keyed by
/// `service.operation`, and scripted action results keyed the same way.
#[derive(Clone, Debug)]
pub struct MockEnvironment {
    pub(crate) provider_id: ProviderId,
    pub(crate) current_principal: Principal,
    pub(crate) regions: Vec<Region>,
    pub(crate) enumerations: HashMap<String, EnumOutcome>,
    pub(crate) actions: HashMap<String, ActionResult>,
}

impl MockEnvironment {
    /// Start building an environment for `provider_id` whose credentials resolve to
    /// `current_principal`.
    pub fn builder(
        provider_id: impl Into<ProviderId>,
        current_principal: Principal,
    ) -> MockEnvironmentBuilder {
        MockEnvironmentBuilder {
            env: MockEnvironment {
                provider_id: provider_id.into(),
                current_principal,
                regions: Vec::new(),
                enumerations: HashMap::new(),
                actions: HashMap::new(),
            },
        }
    }
}

/// Builder for a [`MockEnvironment`].
pub struct MockEnvironmentBuilder {
    env: MockEnvironment,
}

impl MockEnvironmentBuilder {
    /// Add an available region.
    pub fn region(mut self, region: impl Into<Region>) -> Self {
        self.env.regions.push(region.into());
        self
    }

    /// Script a successful enumeration for `service.operation`, revealing `assertions`.
    pub fn enumeration(
        mut self,
        service: &str,
        operation: &str,
        assertions: Vec<Assertion>,
    ) -> Self {
        self.env
            .enumerations
            .insert(format!("{service}.{operation}"), EnumOutcome::Assertions(assertions));
        self
    }

    /// Script a denied enumeration for `service.operation` (drives partial-permission handling).
    pub fn denied_enumeration(mut self, service: &str, operation: &str, reason: &str) -> Self {
        self.env
            .enumerations
            .insert(format!("{service}.{operation}"), EnumOutcome::Denied(reason.to_string()));
        self
    }

    /// Script the result of an action `service.operation`.
    pub fn action(mut self, service: &str, operation: &str, result: ActionResult) -> Self {
        self.env.actions.insert(format!("{service}.{operation}"), result);
        self
    }

    /// Finish building.
    pub fn build(self) -> MockEnvironment {
        self.env
    }
}
