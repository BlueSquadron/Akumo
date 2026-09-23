//! The technique content schema (FR-F). A technique is one versioned unit with four parts
//! (spec §3.1): **metadata** and **contract** are always declarative Tier-0 (the planner and
//! validator read them, never executing code); **steps** and their **revert** are where the tiers
//! apply. Techniques are authored in YAML (ADR-0018); this module is the deserialization target.
//!
//! Expression fields (step `condition`, `for_each.items`, templated params/bindings) hold **CEL
//! source as strings** here; parsing/evaluation is wired in Increment 3 part 2 (G8). The
//! precondition/effect **contract**, by contrast, is *structured* predicates (SPEC-D5) so the
//! planner can reason over it without understanding CEL.

use serde::{Deserialize, Serialize};

use akumo_domain::epistemic::EpistemicStatus;
use akumo_domain::impact::ImpactLevel;

/// A complete technique definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Technique {
    /// Declarative metadata (Tier-0).
    pub metadata: Metadata,
    /// The precondition/effect contract (Tier-0), shared by execution and the planner (FR-F6).
    #[serde(default)]
    pub contract: Contract,
    /// The ordered steps (Tier-1 DSL, with per-step Tier-2/3 escapes).
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Declarative metadata (FR-F2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    /// Unique technique id.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// What the technique does.
    pub description: String,
    /// Author/attribution.
    #[serde(default)]
    pub author: Option<String>,
    /// Content version (semver).
    pub version: String,
    /// The provider this technique targets (provider-scoped, SPEC-D2).
    pub provider: String,
    /// MITRE ATT&CK technique id(s), e.g. `T1098`.
    #[serde(default)]
    pub mitre: Vec<String>,
    /// Optional OWASP-WSTG cross-map.
    #[serde(default)]
    pub owasp: Vec<String>,
    /// Optional CIS cross-map.
    #[serde(default)]
    pub cis: Vec<String>,
    /// Free-form references (e.g. AWS Threat Technique Catalog entries, research write-ups).
    #[serde(default)]
    pub references: Vec<String>,
    /// Impact classification (gates consent, FR-A3).
    pub impact: ImpactLevel,
    /// Expected telemetry signatures (FR-I1; Sigma-aligned in G13).
    #[serde(default)]
    pub expected_telemetry: Vec<ExpectedTelemetry>,
    /// Required for irreversible/destructive techniques: the id of a simulated variant (FR-G6).
    #[serde(default)]
    pub simulated_variant: Option<String>,
    /// Engine/content-model compatibility range (NFR-MNT3), e.g. `>=0`.
    #[serde(default)]
    pub engine_compat: Option<String>,
}

/// An expected telemetry signature (the detection meaning of the action). Field-level detail and
/// Sigma alignment are elaborated in G13.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExpectedTelemetry {
    /// Log/event source, e.g. `cloudtrail`.
    pub source: String,
    /// The event/operation name, e.g. `CreateAccessKey`.
    pub event_name: String,
    /// Relevant fields an analyst would key on.
    #[serde(default)]
    pub fields: serde_json::Map<String, serde_json::Value>,
    /// A candidate detection reference/expression (Sigma) — G13.
    #[serde(default)]
    pub detection: Option<String>,
}

/// The precondition/effect contract (spec §3.3), the PDDL-shaped action model shared by execution
/// and the planner (FR-F6).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    /// Typed parameters the steps consume.
    #[serde(default)]
    pub inputs: Vec<Input>,
    /// Facts/edges that must hold (with a required epistemic status).
    #[serde(default)]
    pub preconditions: Vec<Precondition>,
    /// Facts/edges asserted on verified success.
    #[serde(default)]
    pub effects: Vec<Effect>,
}

/// A typed technique input parameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    /// Parameter name (referenced in the body as `$name`).
    pub name: String,
    /// Declared type.
    #[serde(rename = "type")]
    pub ty: InputType,
    /// Whether the parameter must be supplied.
    #[serde(default)]
    pub required: bool,
    /// A default value if not supplied.
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

/// The type of an input parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    /// A text value.
    String,
    /// A numeric value.
    Number,
    /// A boolean.
    Bool,
    /// A reference to a principal node.
    Principal,
    /// A reference to a resource node.
    Resource,
}

/// A named predicate over the fact/graph views (SPEC-D5), e.g. `can_assume($p, $role)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactPredicate {
    /// Predicate name (must be a known fact view).
    pub name: String,
    /// Predicate arguments (input refs `$x` or literals; resolved at execution/planning time).
    #[serde(default)]
    pub args: Vec<String>,
}

/// A precondition: a predicate plus the epistemic status it requires (SPEC-D6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Precondition {
    /// The required predicate.
    pub predicate: FactPredicate,
    /// The minimum epistemic status the predicate's evidence must have.
    #[serde(default = "default_min_status")]
    pub min_status: EpistemicStatus,
    /// If true, the precondition may be satisfied by inferred/unknown evidence (optimistic
    /// planning): the planner may treat it as a candidate.
    #[serde(default)]
    pub opportunistic: bool,
}

fn default_min_status() -> EpistemicStatus {
    EpistemicStatus::Proven
}

/// An effect: a fact/edge asserted on verified success.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    /// The asserted predicate.
    pub predicate: FactPredicate,
}

/// One step of a technique body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// Step id (unique within the technique).
    pub id: String,
    /// What the step does (Tier-1 call or Tier-2/3 escape).
    pub body: StepBody,
    /// A CEL run-if predicate (G8); the step runs only if it evaluates true.
    #[serde(default)]
    pub condition: Option<String>,
    /// Bounded iteration over enumerated results (G8).
    #[serde(default)]
    pub for_each: Option<ForEach>,
    /// Bindings from the step result into named facts available to later steps.
    #[serde(default)]
    pub bind: Vec<Binding>,
    /// An explicit compensation override. If absent, the engine derives one from the descriptor's
    /// declared inverse (saga model, SPEC-D8).
    #[serde(default)]
    pub revert: Option<Compensation>,
}

/// The body of a step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepBody {
    /// Tier-1: a host-brokered provider call (a descriptor reference).
    Call {
        /// Provider service.
        service: String,
        /// Operation within the service.
        operation: String,
        /// Static or CEL-templated parameters.
        #[serde(default)]
        params: serde_json::Map<String, serde_json::Value>,
    },
    /// Tier-2/3: a per-step escape hatch (executed in G8). Must declare the capabilities it needs.
    Script {
        /// The escape language.
        language: ScriptLang,
        /// The script source.
        source: String,
        /// The capabilities the host grants this step (NFR-SEC5).
        #[serde(default)]
        capabilities: Vec<String>,
    },
}

/// A per-step escape-hatch language (SPEC-D9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptLang {
    /// Tier-2 Starlark.
    Starlark,
    /// Tier-3 WASM.
    Wasm,
}

/// Bounded iteration over enumerated results.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForEach {
    /// A CEL expression producing the collection to iterate (G8).
    pub items: String,
    /// The loop variable name.
    pub var: String,
    /// An explicit upper bound on iterations (safety).
    #[serde(default)]
    pub max: Option<usize>,
}

/// A binding from a step result into a named fact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    /// The fact name to bind.
    pub name: String,
    /// A CEL/path expression into the step result (G8).
    pub from: String,
}

/// A compensating provider call that undoes a step's effect (saga model, SPEC-D8).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Compensation {
    /// Provider service.
    pub service: String,
    /// The compensating operation.
    pub operation: String,
    /// Parameters (may reference bound facts).
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
}
