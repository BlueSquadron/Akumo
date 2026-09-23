//! The planner (FR-E): compute ranked, explainable attack paths toward an objective over the
//! attack graph. This is Akumo's attack-graph-native differentiator. v1 is **deterministic** only
//! (ADR-0007; the AI planner is v2).
//!
//! Design (spec §5):
//! - **Objectives (G12.1 / ADR-0027):** goal predicates — `ReachAdmin`, `ReachResource`.
//! - **Hybrid search (G12.2 / ADR-0024):** graph reachability over control-transfer edges **plus**
//!   technique-as-action expansion for state-creating steps (a [`PlanningAction`] whose
//!   preconditions hold produces new control). Techniques (G7) will map their contracts to
//!   `PlanningAction`; the planner stays decoupled from the DSL.
//! - **Uncertainty (G12.3 / ADR-0025):** dual-mode — proven paths and labeled candidate paths; a
//!   strict-proven and a full-optimistic mode are also available.
//! - **Search (G12.4 / ADR-0026):** bounded best-first (uniform-cost) guided by the ranking cost,
//!   with an opt-in exhaustive mode.
//! - **Ranking (G12.5 / ADR-0011):** composite `confidence → length → detectability` by default,
//!   with presets; reversibility/impact is always carried as a safety annotation.
//! - **Explainability (G12.6 / FR-E3):** each step names the principal, the edge/technique, the
//!   enabling fact + epistemic status, and the produced effect.
//!
//! The planner is pure (no ledger). A driving layer records a `PathComputed` event.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap, HashSet};

use akumo_domain::epistemic::Epistemic;
use akumo_domain::graph::{EdgeKind, NodeId};
use akumo_domain::impact::ImpactLevel;

use crate::graph::AttackGraph;

/// A goal predicate the planner plans toward (v1 objective classes, OQ-3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Objective {
    /// Reach a principal that satisfies the "administrative" predicate.
    ReachAdmin,
    /// Reach control of a principal that can access the named resource.
    ReachResource {
        /// The target resource node id.
        resource: NodeId,
    },
}

/// How the planner treats unproven (`UNKNOWN`/`INFERRED`) edges (ADR-0025).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlanMode {
    /// Only traverse `PROVEN` edges.
    StrictProven,
    /// Traverse proven + unproven; paths using unproven edges are labeled `Candidate` (default).
    #[default]
    HonestDefault,
    /// Traverse everything non-`ABSENT` (candidates still labeled by confidence).
    FullOptimistic,
}

/// Ranking preset (ADR-0011). The composite default orders confidence → length → detectability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RankPreset {
    /// Confidence → length → detectability.
    #[default]
    Composite,
    /// Fewest steps.
    Shortest,
    /// Least detectable.
    Stealthiest,
    /// Lowest impact / most reversible.
    Safest,
    /// Highest confidence.
    MostReliable,
}

/// Planner bounds and behavior (ADR-0026).
#[derive(Clone, Debug)]
pub struct PlannerConfig {
    /// Uncertainty handling.
    pub mode: PlanMode,
    /// Ranking preset (also the search cost).
    pub preset: RankPreset,
    /// Maximum path length.
    pub max_depth: usize,
    /// Maximum states expanded (safety bound on large graphs, NFR-PERF3).
    pub max_expansions: usize,
    /// Maximum paths returned (ignored when `exhaustive`).
    pub max_results: usize,
    /// Explore beyond `max_results` up to `max_expansions`.
    pub exhaustive: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            mode: PlanMode::default(),
            preset: RankPreset::default(),
            max_depth: 12,
            max_expansions: 10_000,
            max_results: 8,
            exhaustive: false,
        }
    }
}

/// A technique modeled as a planning action (FR-F6): if the attacker controls all
/// `requires_control` principals, applying it grants control of `grants_control`. Techniques (G7)
/// produce these from their precondition/effect contracts.
#[derive(Clone, Debug)]
pub struct PlanningAction {
    /// Technique id.
    pub id: String,
    /// Principals that must be controlled for the action to apply.
    pub requires_control: Vec<NodeId>,
    /// Principals the action grants control of.
    pub grants_control: Vec<NodeId>,
    /// The action's impact (safety annotation).
    pub impact: ImpactLevel,
    /// A detectability weight (higher = noisier), from expected telemetry.
    pub detectability: u64,
}

/// One hop in a path.
#[derive(Clone, Debug, PartialEq)]
pub enum StepKind {
    /// Traversal of an existing graph edge.
    Edge(EdgeKind),
    /// Application of a technique (state-creating).
    Technique(String),
}

/// An explainable step (FR-E3): who, how, why (with certainty), and what it produced.
#[derive(Clone, Debug, PartialEq)]
pub struct PathStep {
    /// The principal acting at this hop.
    pub principal: NodeId,
    /// Edge traversal or technique application.
    pub kind: StepKind,
    /// The enabling permission/trust/technique.
    pub enabling: String,
    /// How certain this hop is.
    pub epistemic: Epistemic,
    /// What the hop produced.
    pub produced: String,
}

/// A path's overall confidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathConfidence {
    /// Every hop is proven.
    Proven,
    /// At least one hop relies on an unproven edge — needs validation.
    Candidate,
}

/// A computed attack path.
#[derive(Clone, Debug)]
pub struct AttackPath {
    /// Ordered, explainable steps (empty means the start already satisfies the objective).
    pub steps: Vec<PathStep>,
    /// Proven vs candidate.
    pub confidence: PathConfidence,
    /// Search cost under the active preset (lower ranks higher).
    pub cost: u64,
    /// The most impactful action on the path (safety annotation, always surfaced — OQ-10).
    pub max_impact: ImpactLevel,
    /// Whether every action on the path is reversible.
    pub reversible: bool,
}

impl AttackPath {
    /// Number of hops.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the path is empty (start already at the objective).
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// A human-readable rendering (FR-E3): one line per hop, then the safety annotation.
    pub fn explain(&self) -> String {
        let mut out = String::new();
        for (i, step) in self.steps.iter().enumerate() {
            let how = match &step.kind {
                StepKind::Edge(kind) => edge_label(*kind).to_string(),
                StepKind::Technique(id) => format!("technique {id}"),
            };
            out.push_str(&format!(
                "{}. {} --[{}: {}]--> {} ({:?})\n",
                i + 1,
                step.principal,
                how,
                step.enabling,
                step.produced,
                step.epistemic.status,
            ));
        }
        out.push_str(&format!(
            "confidence: {:?} · reversible: {} · max impact: {}",
            self.confidence, self.reversible, self.max_impact
        ));
        out
    }
}

/// The deterministic path planner.
pub struct Planner;

impl Planner {
    /// Compute ranked, explainable paths from the `start` principals toward `objective`, using the
    /// graph's control-transfer edges plus the supplied technique actions.
    pub fn plan(
        graph: &AttackGraph,
        start: &[NodeId],
        objective: &Objective,
        actions: &[PlanningAction],
        config: &PlannerConfig,
    ) -> Vec<AttackPath> {
        let start_set: BTreeSet<NodeId> = start.iter().cloned().collect();
        let mut heap: BinaryHeap<Reverse<SearchNode>> = BinaryHeap::new();
        let mut visited: HashSet<BTreeSet<NodeId>> = HashSet::new();
        let mut tie: u64 = 0;
        let mut results: Vec<AttackPath> = Vec::new();
        let mut expansions: usize = 0;

        heap.push(Reverse(SearchNode {
            cost: 0,
            tie,
            state: SearchState {
                controlled: start_set,
                steps: Vec::new(),
                any_unproven: false,
                max_impact: ImpactLevel::Read,
                reversible: true,
            },
        }));
        tie += 1;

        while let Some(Reverse(node)) = heap.pop() {
            if !config.exhaustive && results.len() >= config.max_results {
                break;
            }
            if expansions >= config.max_expansions {
                break;
            }
            if !visited.insert(node.state.controlled.clone()) {
                continue;
            }
            expansions += 1;

            if objective_satisfied(graph, &node.state, objective, config.mode) {
                results.push(to_path(&node.state, node.cost));
                continue; // a goal state is a leaf; do not expand past it
            }

            if node.state.steps.len() >= config.max_depth {
                continue;
            }

            for (step_cost, succ) in expand(graph, actions, &node.state, config) {
                heap.push(Reverse(SearchNode { cost: node.cost + step_cost, tie, state: succ }));
                tie += 1;
            }
        }

        results
    }
}

// ---- internals ---------------------------------------------------------------------------------

#[derive(Clone)]
struct SearchState {
    controlled: BTreeSet<NodeId>,
    steps: Vec<PathStep>,
    any_unproven: bool,
    max_impact: ImpactLevel,
    reversible: bool,
}

/// A heap entry ordered by `(cost, tie)`; the state is carried but not part of the ordering.
struct SearchNode {
    cost: u64,
    tie: u64,
    state: SearchState,
}

impl PartialEq for SearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.tie == other.tie
    }
}
impl Eq for SearchNode {}
impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.cmp(&other.cost).then(self.tie.cmp(&other.tie))
    }
}

fn to_path(state: &SearchState, cost: u64) -> AttackPath {
    AttackPath {
        steps: state.steps.clone(),
        confidence: if state.any_unproven {
            PathConfidence::Candidate
        } else {
            PathConfidence::Proven
        },
        cost,
        max_impact: state.max_impact,
        reversible: state.reversible,
    }
}

fn is_control_transfer(kind: EdgeKind) -> bool {
    matches!(
        kind,
        EdgeKind::CanAssume | EdgeKind::CanEscalateTo | EdgeKind::MemberOf | EdgeKind::FederatedAs
    )
}

fn edge_traversable(epistemic: Epistemic, mode: PlanMode) -> bool {
    use akumo_domain::epistemic::EpistemicStatus::*;
    match mode {
        PlanMode::StrictProven => epistemic.status == Proven,
        PlanMode::HonestDefault | PlanMode::FullOptimistic => epistemic.status != Absent,
    }
}

fn edge_label(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::MemberOf => "member-of",
        EdgeKind::HasPermission => "has-permission",
        EdgeKind::CanAssume => "can-assume",
        EdgeKind::Trusts => "trusts",
        EdgeKind::HasCredential => "has-credential",
        EdgeKind::FederatedAs => "federated-as",
        EdgeKind::CanAccess => "can-access",
        EdgeKind::CanEscalateTo => "can-escalate-to",
    }
}

fn impact_rank(impact: ImpactLevel) -> u64 {
    match impact {
        ImpactLevel::Read => 0,
        ImpactLevel::MutatingReversible => 1,
        ImpactLevel::MutatingIrreversible => 2,
        ImpactLevel::Destructive => 3,
    }
}

fn step_cost(preset: RankPreset, unproven: bool, detectability: u64, impact: ImpactLevel) -> u64 {
    match preset {
        RankPreset::Shortest => 1,
        RankPreset::Stealthiest => 1 + detectability,
        RankPreset::Safest => 1 + impact_rank(impact) * 10,
        RankPreset::MostReliable => 1 + if unproven { 1_000 } else { 0 },
        // Composite: confidence dominates, then length, then detectability/impact.
        RankPreset::Composite => {
            (if unproven { 1_000_000 } else { 0 }) + 1_000 + detectability + impact_rank(impact)
        }
    }
}

fn is_admin(graph: &AttackGraph, principal: &str, mode: PlanMode) -> bool {
    if let Some(node) = graph.node(principal) {
        if node.attributes.get("admin").and_then(|v| v.as_bool()).unwrap_or(false) {
            return true;
        }
    }
    graph.edges_from(principal).any(|edge| {
        edge.kind == EdgeKind::HasPermission
            && edge_traversable(edge.epistemic, mode)
            && graph
                .node(&edge.to)
                .and_then(|n| n.attributes.get("admin"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
    })
}

fn objective_satisfied(
    graph: &AttackGraph,
    state: &SearchState,
    objective: &Objective,
    mode: PlanMode,
) -> bool {
    match objective {
        Objective::ReachAdmin => state.controlled.iter().any(|p| is_admin(graph, p, mode)),
        Objective::ReachResource { resource } => state.controlled.iter().any(|p| {
            graph
                .can_access(p, resource)
                .is_some_and(|ep| edge_traversable(ep, mode))
        }),
    }
}

fn expand(
    graph: &AttackGraph,
    actions: &[PlanningAction],
    state: &SearchState,
    config: &PlannerConfig,
) -> Vec<(u64, SearchState)> {
    let mut out: Vec<(u64, SearchState)> = Vec::new();

    // Static control-transfer edges (graph reachability).
    for principal in &state.controlled {
        for edge in graph.edges_from(principal) {
            if !is_control_transfer(edge.kind)
                || state.controlled.contains(&edge.to)
                || !edge_traversable(edge.epistemic, config.mode)
            {
                continue;
            }
            let unproven = !edge.epistemic.is_proven();
            let mut next = state.clone();
            next.controlled.insert(edge.to.clone());
            next.any_unproven |= unproven;
            next.steps.push(PathStep {
                principal: principal.clone(),
                kind: StepKind::Edge(edge.kind),
                enabling: edge
                    .enabling_permission
                    .clone()
                    .unwrap_or_else(|| edge_label(edge.kind).to_string()),
                epistemic: edge.epistemic,
                produced: format!("control {}", edge.to),
            });
            let cost = step_cost(config.preset, unproven, 0, ImpactLevel::Read);
            out.push((cost, next));
        }
    }

    // Technique-as-action expansion (state-creating steps).
    for action in actions {
        let applicable = action
            .requires_control
            .iter()
            .all(|r| state.controlled.contains(r));
        let grants_new = action
            .grants_control
            .iter()
            .any(|g| !state.controlled.contains(g));
        if !applicable || !grants_new {
            continue;
        }
        let mut next = state.clone();
        for g in &action.grants_control {
            next.controlled.insert(g.clone());
        }
        next.max_impact = next.max_impact.max(action.impact);
        next.reversible &= action.impact.is_reversible();
        next.steps.push(PathStep {
            principal: action.requires_control.first().cloned().unwrap_or_default(),
            kind: StepKind::Technique(action.id.clone()),
            enabling: format!("technique {}", action.id),
            epistemic: Epistemic::proven(),
            produced: format!("control {}", action.grants_control.join(", ")),
        });
        let cost = step_cost(config.preset, false, action.detectability, action.impact);
        out.push((cost, next));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::epistemic::Epistemic;
    use akumo_domain::graph::{Assertion, GraphEdge, GraphNode, NodeKind, Provenance};
    use akumo_domain::ids::Timestamp;

    fn prov() -> Provenance {
        Provenance::new("test", Timestamp::from_millis(0))
    }

    fn principal(id: &str, admin: bool) -> Assertion {
        let mut attributes = serde_json::Map::new();
        if admin {
            attributes.insert("admin".into(), serde_json::Value::Bool(true));
        }
        Assertion::Node(GraphNode {
            id: id.into(),
            kind: NodeKind::Principal,
            attributes,
            epistemic: Epistemic::proven(),
            provenance: prov(),
        })
    }

    fn resource(id: &str) -> Assertion {
        Assertion::Node(GraphNode {
            id: id.into(),
            kind: NodeKind::Resource,
            attributes: serde_json::Map::new(),
            epistemic: Epistemic::proven(),
            provenance: prov(),
        })
    }

    fn edge(from: &str, to: &str, kind: EdgeKind, ep: Epistemic) -> Assertion {
        Assertion::Edge(GraphEdge {
            from: from.into(),
            to: to.into(),
            kind,
            enabling_permission: None,
            epistemic: ep,
            provenance: prov(),
        })
    }

    #[test]
    fn reaches_admin_over_a_static_edge() {
        let graph = AttackGraph::from_assertions(vec![
            principal("f", false),
            principal("a", true),
            edge("f", "a", EdgeKind::CanAssume, Epistemic::proven()),
        ]);
        let paths = Planner::plan(
            &graph,
            &["f".to_string()],
            &Objective::ReachAdmin,
            &[],
            &PlannerConfig::default(),
        );
        assert!(!paths.is_empty());
        assert_eq!(paths[0].len(), 1);
        assert_eq!(paths[0].confidence, PathConfidence::Proven);
    }

    #[test]
    fn reports_no_path_when_unreachable() {
        let graph = AttackGraph::from_assertions(vec![principal("f", false), principal("a", true)]);
        let paths = Planner::plan(
            &graph,
            &["f".to_string()],
            &Objective::ReachAdmin,
            &[],
            &PlannerConfig::default(),
        );
        assert!(paths.is_empty());
    }

    #[test]
    fn unknown_edge_is_strict_hidden_but_honest_candidate() {
        let graph = AttackGraph::from_assertions(vec![
            principal("f", false),
            principal("a", true),
            edge("f", "a", EdgeKind::CanAssume, Epistemic::unknown()),
        ]);

        let strict = Planner::plan(
            &graph,
            &["f".to_string()],
            &Objective::ReachAdmin,
            &[],
            &PlannerConfig { mode: PlanMode::StrictProven, ..PlannerConfig::default() },
        );
        assert!(strict.is_empty(), "strict mode must not use an unproven edge");

        let honest = Planner::plan(
            &graph,
            &["f".to_string()],
            &Objective::ReachAdmin,
            &[],
            &PlannerConfig::default(),
        );
        assert_eq!(honest.len(), 1);
        assert_eq!(honest[0].confidence, PathConfidence::Candidate);
    }

    #[test]
    fn technique_action_creates_a_reachable_path() {
        // No static edge from f to the admin t; only a technique bridges them.
        let graph = AttackGraph::from_assertions(vec![principal("f", false), principal("t", true)]);
        let action = PlanningAction {
            id: "create_key_and_assume".to_string(),
            requires_control: vec!["f".to_string()],
            grants_control: vec!["t".to_string()],
            impact: ImpactLevel::MutatingReversible,
            detectability: 1,
        };
        let paths = Planner::plan(
            &graph,
            &["f".to_string()],
            &Objective::ReachAdmin,
            std::slice::from_ref(&action),
            &PlannerConfig::default(),
        );
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 1);
        assert!(matches!(paths[0].steps[0].kind, StepKind::Technique(_)));
        assert_eq!(paths[0].max_impact, ImpactLevel::MutatingReversible);
        assert!(paths[0].reversible);
    }

    #[test]
    fn reaches_resource_after_escalation() {
        let graph = AttackGraph::from_assertions(vec![
            principal("f", false),
            principal("a", false),
            resource("r"),
            edge("f", "a", EdgeKind::CanAssume, Epistemic::proven()),
            edge("a", "r", EdgeKind::CanAccess, Epistemic::proven()),
        ]);
        let paths = Planner::plan(
            &graph,
            &["f".to_string()],
            &Objective::ReachResource { resource: "r".to_string() },
            &[],
            &PlannerConfig::default(),
        );
        assert!(!paths.is_empty());
        assert_eq!(paths[0].len(), 1); // assume f->a; then a can-access r satisfies the goal
        assert!(paths[0].explain().contains("can-access") || paths[0].explain().contains("can-assume"));
    }
}
