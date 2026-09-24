//! The Engagement Manager (FR-A, NFR-COMP): named, isolated engagements with their own scope,
//! credentials, state, and audit trail — all recorded as ledger events (ADR-0014). It provides the
//! authorization gate every operation passes through:
//!
//! - **Lifecycle (G5.1):** `open` (records the authorization affirmation, NFR-COMP1) / `amend_scope`
//!   / `close`. State lives in the ledger, so switching engagements needs no restart (FR-A6).
//! - **Scope (G5.2):** [`EngagementContext::ensure_in_scope`] refuses out-of-scope targets (FR-A1).
//! - **Consent (G5.3):** [`ConsentPolicy`] + [`consent_satisfied`] gate mutating steps (FR-A3).
//! - **Kill-switch (G5.4):** `invoke_kill_switch` marks the engagement halted (revert wiring: G9).
//! - **Auth check (G5.5):** [`authcheck`] validates credentials + scope without enumerating.

use serde::{Deserialize, Serialize};

use akumo_domain::error::{AkumoError, Result};
use akumo_domain::event::event_type;
use akumo_domain::ids::{Actor, EngagementId, ProviderId, Region};
use akumo_domain::impact::ImpactLevel;
use akumo_domain::ports::{EventStore, Provider};
use akumo_domain::principal::Principal;
use akumo_domain::scope::Scope;

use crate::journal::Journal;
use crate::projection::Projection;

// ---- event payloads (private; serialized into the ledger) --------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EngagementOpenedPayload {
    scope: Scope,
    provider: ProviderId,
    credential_ref: String,
    authorization_affirmed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ScopeAmendedPayload {
    scope: Scope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ConsentRecordedPayload {
    impact: ImpactLevel,
    granted: bool,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct KillSwitchPayload {
    reason: String,
}

// ---- projected state ---------------------------------------------------------------------------

/// The lifecycle status of an engagement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngagementStatus {
    /// Open and usable.
    Open,
    /// Halted by the kill-switch.
    Killed,
    /// Closed.
    Closed,
}

/// The current projected state of an engagement (folded from its event stream).
#[derive(Clone, Debug, PartialEq)]
pub struct EngagementState {
    /// The engagement id.
    pub id: EngagementId,
    /// The authorized scope.
    pub scope: Scope,
    /// The provider in use.
    pub provider: ProviderId,
    /// Lifecycle status.
    pub status: EngagementStatus,
    /// The highest impact level for which consent has been recorded (interactive mode).
    pub max_consent: Option<ImpactLevel>,
}

/// Folds engagement lifecycle events into an [`EngagementState`].
pub struct EngagementView;

impl Projection for EngagementView {
    type State = Option<EngagementState>;

    fn apply(state: &mut Option<EngagementState>, event: &akumo_domain::event::EventEnvelope) {
        match event.event_type.as_str() {
            event_type::ENGAGEMENT_OPENED => {
                if let Ok(p) =
                    serde_json::from_value::<EngagementOpenedPayload>(event.payload.clone())
                {
                    *state = Some(EngagementState {
                        id: event.engagement_id.clone(),
                        scope: p.scope,
                        provider: p.provider,
                        status: EngagementStatus::Open,
                        max_consent: None,
                    });
                }
            }
            event_type::SCOPE_AMENDED => {
                if let (Some(s), Ok(p)) = (
                    state.as_mut(),
                    serde_json::from_value::<ScopeAmendedPayload>(event.payload.clone()),
                ) {
                    s.scope = p.scope;
                }
            }
            event_type::CONSENT_RECORDED => {
                if let (Some(s), Ok(p)) = (
                    state.as_mut(),
                    serde_json::from_value::<ConsentRecordedPayload>(event.payload.clone()),
                ) {
                    if p.granted {
                        s.max_consent = Some(s.max_consent.map_or(p.impact, |m| m.max(p.impact)));
                    }
                }
            }
            event_type::KILL_SWITCH_INVOKED => {
                if let Some(s) = state.as_mut() {
                    s.status = EngagementStatus::Killed;
                }
            }
            event_type::ENGAGEMENT_CLOSED => {
                if let Some(s) = state.as_mut() {
                    s.status = EngagementStatus::Closed;
                }
            }
            _ => {}
        }
    }
}

// ---- ambient engagement context ----------------------------------------------------------------

/// The ambient context every provider-touching operation consults: the authorized scope and the
/// live status. Built from [`EngagementState`]; used by enumeration (G6) and execution (G9) to
/// refuse out-of-scope or post-kill-switch actions (spec §1.7).
#[derive(Clone, Debug)]
pub struct EngagementContext {
    /// The engagement id.
    pub engagement_id: EngagementId,
    /// The authorized scope.
    pub scope: Scope,
    /// Lifecycle status.
    pub status: EngagementStatus,
}

impl EngagementContext {
    /// Derive a context from projected state.
    pub fn from_state(state: &EngagementState) -> Self {
        Self {
            engagement_id: state.id.clone(),
            scope: state.scope.clone(),
            status: state.status,
        }
    }

    /// Refuse if the engagement is not active (killed or closed).
    pub fn ensure_active(&self) -> Result<()> {
        match self.status {
            EngagementStatus::Open => Ok(()),
            EngagementStatus::Killed => Err(AkumoError::Message(format!(
                "engagement {} is halted by the kill-switch",
                self.engagement_id
            ))),
            EngagementStatus::Closed => Err(AkumoError::Message(format!(
                "engagement {} is closed",
                self.engagement_id
            ))),
        }
    }

    /// Refuse if `kind`/`value` is outside the authorized scope (FR-A1, NFR-COMP2).
    pub fn ensure_in_scope(&self, kind: &str, value: &str) -> Result<()> {
        if self.scope.contains(kind, value) {
            Ok(())
        } else {
            Err(AkumoError::OutOfScope(format!(
                "{kind}:{value} is outside the authorized scope"
            )))
        }
    }
}

// ---- consent -----------------------------------------------------------------------------------

/// How consent is granted for the two operating contexts (FR-K), spec §4.2.
#[derive(Clone, Copy, Debug)]
pub enum ConsentPolicy {
    /// Interactive: a step needs a recorded consent at or above its impact.
    Interactive,
    /// CI / unattended: steps at or below the declared ceiling are pre-authorized; anything above
    /// halts rather than proceeding unattended.
    CiCeiling(ImpactLevel),
}

/// Whether a step at `impact` may proceed under `policy`, given the highest recorded consent so far.
pub fn consent_satisfied(
    policy: ConsentPolicy,
    max_recorded: Option<ImpactLevel>,
    impact: ImpactLevel,
) -> bool {
    if !impact.requires_consent() {
        return true; // read-only never needs consent
    }
    match policy {
        ConsentPolicy::CiCeiling(ceiling) => impact <= ceiling,
        ConsentPolicy::Interactive => max_recorded.is_some_and(|m| m >= impact),
    }
}

// ---- dry authorization check (G5.5) ------------------------------------------------------------

/// The result of a dry authorization check: who we are and where we can operate, with no
/// enumeration and no ledger writes (FR-A5).
#[derive(Clone, Debug)]
pub struct AuthCheckReport {
    /// The resolved current principal.
    pub principal: Principal,
    /// The provider id.
    pub provider: ProviderId,
    /// Available regions/partitions.
    pub regions: Vec<Region>,
}

/// Validate credentials + provider metadata without enumerating or mutating anything (FR-A5).
pub async fn authcheck(provider: &dyn Provider) -> Result<AuthCheckReport> {
    let principal = provider.identity().resolve_current_principal().await?;
    let meta = provider.metadata();
    Ok(AuthCheckReport {
        principal,
        provider: meta.provider_id(),
        regions: meta.regions(),
    })
}

// ---- the manager -------------------------------------------------------------------------------

/// Manages engagement lifecycle over an [`EventStore`].
pub struct EngagementManager<'a> {
    store: &'a dyn EventStore,
}

impl<'a> EngagementManager<'a> {
    /// Create a manager over an event store.
    pub fn new(store: &'a dyn EventStore) -> Self {
        Self { store }
    }

    /// Open a new, isolated engagement. `authorization_affirmed` MUST be `true` (NFR-COMP1) and the
    /// scope MUST be non-empty (fail-closed). Fails if the engagement already exists.
    pub async fn open(
        &self,
        id: EngagementId,
        scope: Scope,
        provider: ProviderId,
        credential_ref: impl Into<String>,
        actor: Actor,
        authorization_affirmed: bool,
    ) -> Result<EngagementId> {
        if !authorization_affirmed {
            return Err(AkumoError::Validation(
                "authorization must be affirmed to open an engagement".to_string(),
            ));
        }
        if scope.is_empty() {
            return Err(AkumoError::Validation(
                "engagement scope must not be empty".to_string(),
            ));
        }
        if self.load(&id).await?.is_some() {
            return Err(AkumoError::Validation(format!(
                "engagement {id} already exists"
            )));
        }
        let payload = serde_json::to_value(EngagementOpenedPayload {
            scope,
            provider,
            credential_ref: credential_ref.into(),
            authorization_affirmed,
        })?;
        Journal::new(self.store)
            .record(&id, actor, event_type::ENGAGEMENT_OPENED, payload)
            .await?;
        Ok(id)
    }

    /// Amend an active engagement's scope.
    pub async fn amend_scope(&self, id: &EngagementId, scope: Scope, actor: Actor) -> Result<()> {
        self.require_active(id).await?;
        let payload = serde_json::to_value(ScopeAmendedPayload { scope })?;
        Journal::new(self.store)
            .record(id, actor, event_type::SCOPE_AMENDED, payload)
            .await?;
        Ok(())
    }

    /// Record a consent decision at a given impact level (FR-A3).
    pub async fn record_consent(
        &self,
        id: &EngagementId,
        actor: Actor,
        impact: ImpactLevel,
        granted: bool,
        note: Option<String>,
    ) -> Result<()> {
        self.require_active(id).await?;
        let payload = serde_json::to_value(ConsentRecordedPayload {
            impact,
            granted,
            note,
        })?;
        Journal::new(self.store)
            .record(id, actor, event_type::CONSENT_RECORDED, payload)
            .await?;
        Ok(())
    }

    /// Invoke the kill-switch: mark the engagement halted (FR-A4). Revert of completed reversible
    /// steps is wired by the execution engine (G9).
    pub async fn invoke_kill_switch(
        &self,
        id: &EngagementId,
        actor: Actor,
        reason: impl Into<String>,
    ) -> Result<()> {
        self.require_exists(id).await?;
        let payload = serde_json::to_value(KillSwitchPayload {
            reason: reason.into(),
        })?;
        Journal::new(self.store)
            .record(id, actor, event_type::KILL_SWITCH_INVOKED, payload)
            .await?;
        Ok(())
    }

    /// Close an engagement.
    pub async fn close(&self, id: &EngagementId, actor: Actor) -> Result<()> {
        self.require_exists(id).await?;
        Journal::new(self.store)
            .record(
                id,
                actor,
                event_type::ENGAGEMENT_CLOSED,
                serde_json::json!({}),
            )
            .await?;
        Ok(())
    }

    /// Load the current projected state of an engagement (`None` if it does not exist).
    pub async fn load(&self, id: &EngagementId) -> Result<Option<EngagementState>> {
        let events = self.store.read_stream(id).await?;
        Ok(EngagementView::replay(events.iter()))
    }

    /// Build the ambient context for an engagement, erroring if it does not exist.
    pub async fn context(&self, id: &EngagementId) -> Result<EngagementContext> {
        match self.load(id).await? {
            Some(state) => Ok(EngagementContext::from_state(&state)),
            None => Err(AkumoError::NotFound(format!("engagement {id}"))),
        }
    }

    async fn require_active(&self, id: &EngagementId) -> Result<()> {
        self.context(id).await?.ensure_active()
    }

    async fn require_exists(&self, id: &EngagementId) -> Result<()> {
        self.context(id).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::event::EventEnvelope;
    use akumo_domain::ids::{EventHash, Seq};
    use akumo_domain::scope::ScopeSelector;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// A minimal in-memory event store for testing the manager without the ledger adapter.
    #[derive(Default)]
    struct MemStore {
        inner: Mutex<HashMap<EngagementId, Vec<EventEnvelope>>>,
    }

    #[async_trait]
    impl EventStore for MemStore {
        async fn append(&self, event: EventEnvelope) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .entry(event.engagement_id.clone())
                .or_default()
                .push(event);
            Ok(())
        }
        async fn read_stream(&self, engagement: &EngagementId) -> Result<Vec<EventEnvelope>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(engagement)
                .cloned()
                .unwrap_or_default())
        }
        async fn last_hash(&self, engagement: &EngagementId) -> Result<Option<EventHash>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(engagement)
                .and_then(|v| v.last())
                .map(|e| e.hash.clone()))
        }
        async fn head(&self, engagement: &EngagementId) -> Result<Option<(Seq, EventHash)>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(engagement)
                .and_then(|v| v.last())
                .map(|e| (e.seq, e.hash.clone())))
        }
    }

    fn scope() -> Scope {
        Scope::new(vec![ScopeSelector::new("account", "123456789012")])
    }

    #[tokio::test]
    async fn open_requires_authorization_and_scope() {
        let store = MemStore::default();
        let mgr = EngagementManager::new(&store);

        let no_auth = mgr
            .open(
                EngagementId::new("e"),
                scope(),
                ProviderId::new("mock"),
                "cred",
                Actor::new("op"),
                false,
            )
            .await;
        assert!(matches!(no_auth, Err(AkumoError::Validation(_))));

        let empty_scope = mgr
            .open(
                EngagementId::new("e"),
                Scope::default(),
                ProviderId::new("mock"),
                "cred",
                Actor::new("op"),
                true,
            )
            .await;
        assert!(matches!(empty_scope, Err(AkumoError::Validation(_))));
    }

    #[tokio::test]
    async fn open_then_load_and_scope_enforcement() {
        let store = MemStore::default();
        let mgr = EngagementManager::new(&store);
        let id = EngagementId::new("eng-open");
        mgr.open(
            id.clone(),
            scope(),
            ProviderId::new("mock"),
            "cred",
            Actor::new("op"),
            true,
        )
        .await
        .unwrap();

        let state = mgr.load(&id).await.unwrap().unwrap();
        assert_eq!(state.status, EngagementStatus::Open);

        let ctx = mgr.context(&id).await.unwrap();
        ctx.ensure_in_scope("account", "123456789012").unwrap();
        assert!(ctx.ensure_in_scope("account", "999999999999").is_err());

        // Opening the same id again is refused.
        assert!(mgr
            .open(
                id.clone(),
                scope(),
                ProviderId::new("mock"),
                "cred",
                Actor::new("op"),
                true
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn kill_switch_deactivates_engagement() {
        let store = MemStore::default();
        let mgr = EngagementManager::new(&store);
        let id = EngagementId::new("eng-kill");
        mgr.open(
            id.clone(),
            scope(),
            ProviderId::new("mock"),
            "cred",
            Actor::new("op"),
            true,
        )
        .await
        .unwrap();

        mgr.invoke_kill_switch(&id, Actor::new("op"), "manual abort")
            .await
            .unwrap();
        let ctx = mgr.context(&id).await.unwrap();
        assert_eq!(ctx.status, EngagementStatus::Killed);
        assert!(ctx.ensure_active().is_err());
    }

    #[tokio::test]
    async fn recorded_consent_raises_the_ceiling() {
        let store = MemStore::default();
        let mgr = EngagementManager::new(&store);
        let id = EngagementId::new("eng-consent");
        mgr.open(
            id.clone(),
            scope(),
            ProviderId::new("mock"),
            "cred",
            Actor::new("op"),
            true,
        )
        .await
        .unwrap();
        mgr.record_consent(
            &id,
            Actor::new("op"),
            ImpactLevel::MutatingReversible,
            true,
            None,
        )
        .await
        .unwrap();
        let state = mgr.load(&id).await.unwrap().unwrap();
        assert_eq!(state.max_consent, Some(ImpactLevel::MutatingReversible));
    }

    #[test]
    fn consent_policy_logic() {
        // Read never needs consent.
        assert!(consent_satisfied(
            ConsentPolicy::Interactive,
            None,
            ImpactLevel::Read
        ));
        // Interactive needs a recorded consent >= impact.
        assert!(!consent_satisfied(
            ConsentPolicy::Interactive,
            None,
            ImpactLevel::MutatingReversible
        ));
        assert!(consent_satisfied(
            ConsentPolicy::Interactive,
            Some(ImpactLevel::Destructive),
            ImpactLevel::MutatingReversible
        ));
        // CI ceiling pre-authorizes up to the ceiling.
        assert!(consent_satisfied(
            ConsentPolicy::CiCeiling(ImpactLevel::MutatingReversible),
            None,
            ImpactLevel::MutatingReversible
        ));
        assert!(!consent_satisfied(
            ConsentPolicy::CiCeiling(ImpactLevel::MutatingReversible),
            None,
            ImpactLevel::Destructive
        ));
    }
}
