//! The Tier-2/3 escape-hatch **host seam** (ADR-0020 / SPEC-D9). A per-step `Script` body drops out
//! of the Tier-1 expression language into Starlark (Tier-2) or WASM (Tier-3) for computed values or
//! branching the DSL can't express. Crucially, the technique **contract stays declarative Tier-0**,
//! so the planner and validator never need to understand these languages.
//!
//! This module defines the seam a script runtime plugs into. The runtimes themselves (`starlark`,
//! `wasmtime`) are wired behind this trait as a focused, separately-verified step; the default host
//! reports them unsupported so the pipeline is complete and safe by default. A script only ever
//! receives the capabilities the host grants it (NFR-SEC5).

use akumo_domain::error::{AkumoError, Result};

use crate::expr::{Env, Value};
use crate::schema::ScriptLang;

/// Runs a per-step escape-hatch script in a sandbox, returning a value to bind into facts.
pub trait ScriptHost: Send + Sync {
    /// Execute `source` in `lang`, granting only `capabilities`, over the current `env`.
    fn execute(
        &self,
        lang: ScriptLang,
        source: &str,
        capabilities: &[String],
        env: &Env,
    ) -> Result<Value>;
}

/// The default host: reports Tier-2/3 as unsupported. Techniques that stay in the Tier-1 DSL never
/// touch this; a build that enables the Starlark/WASM runtimes replaces it behind [`ScriptHost`].
pub struct UnsupportedScriptHost;

impl ScriptHost for UnsupportedScriptHost {
    fn execute(
        &self,
        lang: ScriptLang,
        _source: &str,
        _capabilities: &[String],
        _env: &Env,
    ) -> Result<Value> {
        Err(AkumoError::Unsupported(format!(
            "escape-hatch runtime for {lang:?} is not enabled in this build (see tasks G8.3/G8.4)"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_host_reports_unsupported() {
        let host = UnsupportedScriptHost;
        let result = host.execute(ScriptLang::Starlark, "x = 1", &["cap".into()], &Env::new());
        assert!(matches!(result, Err(AkumoError::Unsupported(_))));
    }
}
