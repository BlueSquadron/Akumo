//! The attack graph as a **ledger projection** (ADR-0015/0016/0017; spec §2.2–2.4). The graph is
//! never a separate store — it is folded from `FactAsserted` / `AccessDenied` events (SPEC-D3), so
//! it is always reconstructable and never a competing source of truth.
//!
//! - **G4.2** — [`GraphProjection`] folds events into an [`AttackGraph`].
//! - **G4.3** — fact/predicate views (`can_assume`, `has_credential`, …) compile to graph lookups;
//!   they are the ergonomic surface the DSL and planner query (fuller CEL surface in G8/G12).
//! - **G4.4** — the coverage map (blind spots) is folded from `AccessDenied` events; every
//!   attack-relevant assertion keeps its categorical epistemic status ("unknown ≠ absent").

use std::collections::HashMap;

use akumo_domain::epistemic::{Epistemic, EpistemicStatus};
use akumo_domain::event::{event_type, EventEnvelope};
use akumo_domain::graph::{
    Assertion, CoverageGap, EdgeKind, GraphEdge, GraphNode, NodeId, NodeKind,
};

use crate::projection::Projection;

/// The provider-neutral attack graph: nodes keyed by id, edges, and the coverage map of blind spots.
#[derive(Clone, Debug, Default)]
pub struct AttackGraph {
    nodes: HashMap<NodeId, GraphNode>,
    edges: Vec<GraphEdge>,
    coverage: Vec<CoverageGap>,
}

impl AttackGraph {
    /// Build a graph directly from a set of assertions (applying the same merge rules as the
    /// projection). Handy for callers that already hold assertions and for tests; the normal path
    /// is to fold the ledger via [`GraphProjection`].
    pub fn from_assertions(assertions: impl IntoIterator<Item = Assertion>) -> Self {
        let mut graph = Self::default();
        for assertion in assertions {
            graph.apply_assertion(assertion);
        }
        graph
    }

    /// Look up a node by id.
    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// Iterate all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    /// Iterate nodes of a given kind.
    pub fn nodes_by_kind(&self, kind: NodeKind) -> impl Iterator<Item = &GraphNode> + '_ {
        self.nodes.values().filter(move |n| n.kind == kind)
    }

    /// All edges.
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// Edges originating at `from`.
    pub fn edges_from<'a>(&'a self, from: &str) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        let from = from.to_string();
        self.edges.iter().filter(move |e| e.from == from)
    }

    /// Edges of a given kind.
    pub fn edges_of_kind(&self, kind: EdgeKind) -> impl Iterator<Item = &GraphEdge> + '_ {
        self.edges.iter().filter(move |e| e.kind == kind)
    }

    /// The recorded blind spots (FR-C6).
    pub fn coverage_gaps(&self) -> &[CoverageGap] {
        &self.coverage
    }

    /// Node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Edge count.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    // ---- fact/predicate views (G4.3) ----------------------------------------------------------

    /// The epistemic status of a specific directed edge, if it exists. This is the primitive the
    /// named predicates below build on.
    pub fn edge_status(&self, from: &str, to: &str, kind: EdgeKind) -> Option<Epistemic> {
        self.edges
            .iter()
            .find(|e| e.from == from && e.to == to && e.kind == kind)
            .map(|e| e.epistemic)
    }

    /// `can_assume(principal, role)` — does an assume edge exist, and how certain is it?
    pub fn can_assume(&self, principal: &str, role: &str) -> Option<Epistemic> {
        self.edge_status(principal, role, EdgeKind::CanAssume)
    }

    /// `has_credential(principal, credential)`.
    pub fn has_credential(&self, principal: &str, credential: &str) -> Option<Epistemic> {
        self.edge_status(principal, credential, EdgeKind::HasCredential)
    }

    /// `has_permission(principal, permission_set)`.
    pub fn has_permission(&self, principal: &str, permission_set: &str) -> Option<Epistemic> {
        self.edge_status(principal, permission_set, EdgeKind::HasPermission)
    }

    /// `can_access(principal, resource)` — the derived access edge.
    pub fn can_access(&self, principal: &str, resource: &str) -> Option<Epistemic> {
        self.edge_status(principal, resource, EdgeKind::CanAccess)
    }

    // ---- mutation (fold-only; called by the projection) ---------------------------------------

    fn apply_assertion(&mut self, assertion: Assertion) {
        match assertion {
            Assertion::Node(node) => self.upsert_node(node),
            Assertion::Edge(edge) => self.upsert_edge(edge),
        }
    }

    fn upsert_node(&mut self, incoming: GraphNode) {
        use std::collections::hash_map::Entry;
        match self.nodes.entry(incoming.id.clone()) {
            Entry::Vacant(slot) => {
                slot.insert(incoming);
            }
            Entry::Occupied(mut slot) => {
                let existing = slot.get_mut();
                let upgrade = certainty_rank(incoming.epistemic.status)
                    > certainty_rank(existing.epistemic.status);
                // Merge provider attributes; keep the more-certain epistemic status (never
                // downgrade a PROVEN node because a later call could not see it).
                for (k, v) in incoming.attributes {
                    existing.attributes.insert(k, v);
                }
                if upgrade {
                    existing.epistemic = incoming.epistemic;
                    existing.provenance = incoming.provenance;
                }
            }
        }
    }

    fn upsert_edge(&mut self, incoming: GraphEdge) {
        let pos = self.edges.iter().position(|e| {
            e.from == incoming.from && e.to == incoming.to && e.kind == incoming.kind
        });
        match pos {
            Some(i) => {
                let existing = &mut self.edges[i];
                if certainty_rank(incoming.epistemic.status)
                    > certainty_rank(existing.epistemic.status)
                {
                    existing.epistemic = incoming.epistemic;
                    existing.enabling_permission = incoming.enabling_permission;
                    existing.provenance = incoming.provenance;
                }
            }
            None => self.edges.push(incoming),
        }
    }

    fn add_gap(&mut self, gap: CoverageGap) {
        self.coverage.push(gap);
    }
}

/// Certainty ordering for merges: definitive observations (`Proven`/`Absent`) outrank `Inferred`,
/// which outranks `Unknown`. Used so repeated assertions never downgrade what we already proved.
fn certainty_rank(status: EpistemicStatus) -> u8 {
    match status {
        EpistemicStatus::Proven | EpistemicStatus::Absent => 3,
        EpistemicStatus::Inferred => 2,
        EpistemicStatus::Unknown => 1,
    }
}

/// Folds the event ledger into an [`AttackGraph`].
pub struct GraphProjection;

impl Projection for GraphProjection {
    type State = AttackGraph;

    fn apply(state: &mut AttackGraph, event: &EventEnvelope) {
        match event.event_type.as_str() {
            event_type::FACT_ASSERTED => {
                if let Ok(assertion) = serde_json::from_value::<Assertion>(event.payload.clone()) {
                    state.apply_assertion(assertion);
                }
            }
            event_type::ACCESS_DENIED => {
                if let Ok(gap) = serde_json::from_value::<CoverageGap>(event.payload.clone()) {
                    state.add_gap(gap);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::event::EventEnvelope;
    use akumo_domain::graph::Provenance;
    use akumo_domain::ids::{Actor, EngagementId, EventHash, Seq, Timestamp};

    /// Build a sealed `FactAsserted` event carrying an assertion.
    fn fact_event(seq: u64, prev: Option<EventHash>, assertion: &Assertion) -> EventEnvelope {
        EventEnvelope::seal(
            format!("evt-{seq}"),
            EngagementId::new("eng-graph"),
            Seq(seq),
            Timestamp::from_millis(seq),
            Actor::new("enum"),
            prev,
            event_type::FACT_ASSERTED,
            serde_json::to_value(assertion).unwrap(),
        )
        .unwrap()
    }

    fn node(id: &str, kind: NodeKind, ep: Epistemic) -> GraphNode {
        GraphNode {
            id: id.into(),
            kind,
            attributes: serde_json::Map::new(),
            epistemic: ep,
            provenance: Provenance::new("test", Timestamp::from_millis(0)),
        }
    }

    fn edge(from: &str, to: &str, kind: EdgeKind, ep: Epistemic) -> GraphEdge {
        GraphEdge {
            from: from.into(),
            to: to.into(),
            kind,
            enabling_permission: Some("sts:AssumeRole".into()),
            epistemic: ep,
            provenance: Provenance::new("test", Timestamp::from_millis(0)),
        }
    }

    #[test]
    fn folds_nodes_edges_and_fact_views() {
        let assertions = [
            Assertion::Node(node("p", NodeKind::Principal, Epistemic::proven())),
            Assertion::Node(node("r", NodeKind::Principal, Epistemic::proven())),
            Assertion::Edge(edge("p", "r", EdgeKind::CanAssume, Epistemic::proven())),
        ];
        let mut prev = None;
        let mut events = Vec::new();
        for (i, a) in assertions.iter().enumerate() {
            let e = fact_event(i as u64, prev.clone(), a);
            prev = Some(e.hash.clone());
            events.push(e);
        }

        let g = GraphProjection::replay(events.iter());
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.nodes_by_kind(NodeKind::Principal).count(), 2);
        assert_eq!(g.can_assume("p", "r"), Some(Epistemic::proven()));
        assert_eq!(g.can_assume("p", "missing"), None);
    }

    #[test]
    fn merge_never_downgrades_proven() {
        let e_proven = fact_event(
            0,
            None,
            &Assertion::Node(node("p", NodeKind::Principal, Epistemic::proven())),
        );
        let e_unknown = fact_event(
            1,
            Some(e_proven.hash.clone()),
            &Assertion::Node(node("p", NodeKind::Principal, Epistemic::unknown())),
        );
        let g = GraphProjection::replay([&e_proven, &e_unknown]);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.node("p").unwrap().epistemic.status, EpistemicStatus::Proven);
    }

    #[test]
    fn access_denied_becomes_a_coverage_gap() {
        let gap = CoverageGap {
            scope: "iam:ListRoles".into(),
            reason: "access denied".into(),
            observed_at: Timestamp::from_millis(0),
        };
        let e = EventEnvelope::seal(
            "evt-gap",
            EngagementId::new("eng-graph"),
            Seq(0),
            Timestamp::from_millis(0),
            Actor::new("enum"),
            None,
            event_type::ACCESS_DENIED,
            serde_json::to_value(&gap).unwrap(),
        )
        .unwrap();
        let g = GraphProjection::replay([&e]);
        assert_eq!(g.coverage_gaps().len(), 1);
        assert_eq!(g.node_count(), 0, "a blind spot is not a node (unknown != absent)");
    }
}
