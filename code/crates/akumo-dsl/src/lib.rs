//! # akumo-dsl
//!
//! The declarative technique content model (FR-F): the YAML technique **schema** (metadata +
//! contract + steps + revert), **validation**, and a discoverable **catalog** loader.
//!
//! This increment (G7) delivers the Tier-0 (metadata + contract) and the Tier-1 step *structure*,
//! plus loading, validation, and cataloging. CEL evaluation and the Starlark/WASM escape-hatch
//! hosts are wired in G8; execution through the safe lifecycle is G9.
//!
//! The crate is a provider-agnostic consumer of [`akumo_domain`]; techniques reach the cloud only
//! via host-brokered descriptors, never a cloud SDK.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod expr;
pub mod schema;
pub mod script;
pub mod validate;

pub use akumo_domain as domain;
pub use catalog::{parse_technique, Catalog};
pub use expr::{eval, eval_array, eval_bool, resolve_params, resolve_value, Env, Value};
pub use schema::{
    Binding, Compensation, Contract, Effect, ExpectedTelemetry, FactPredicate, ForEach, Input,
    InputType, Metadata, Precondition, ScriptLang, Step, StepBody, Technique,
};
pub use script::{ScriptHost, UnsupportedScriptHost};
pub use validate::validate;
