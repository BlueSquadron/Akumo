//! Akumo developer automation.
//!
//! Subcommands:
//! - `dep-lint` — the release-blocking dependency-direction check (task G1.4, NFR-EXT1): the
//!   provider-blind core (`akumo-domain`, `akumo-core`) must not depend — transitively, via normal
//!   (non-dev, non-build) dependencies — on any adapter crate or any cloud SDK.
//!
//! Later tasks add `docgen` (G18.7), `leak-detector` (G19.2), and `bundle` (G17.3).

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::process::Command;

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let code = match cmd.as_str() {
        "dep-lint" => dep_lint(),
        "docgen" => docgen(),
        "leak-detector" => leak_detector(),
        "" | "help" | "-h" | "--help" => {
            eprintln!(
                "usage: cargo run -p xtask -- <dep-lint | docgen [content_dir] [out_dir] | \
                 leak-detector [state_dir]>"
            );
            2
        }
        other => {
            eprintln!("xtask: unknown subcommand '{other}'");
            2
        }
    };
    std::process::exit(code);
}

/// Crates whose *runtime* dependency closure must stay free of adapters and cloud SDKs.
const GUARDED_CRATES: &[&str] = &["akumo-domain", "akumo-core"];

/// Is `dep` a crate the guarded core must never depend on?
///
/// `guarded` is the crate being checked, so `akumo-domain` (which must be an internal *leaf*) can be
/// held to a stricter rule than `akumo-core`.
fn is_forbidden(guarded: &str, dep: &str) -> bool {
    // Cloud SDKs — must live only inside a provider adapter.
    let cloud_sdk = dep.starts_with("aws-")
        || dep.starts_with("aws_")
        || dep.starts_with("azure")
        || dep.starts_with("google-cloud")
        || dep.starts_with("google_cloud")
        || dep.starts_with("gcloud");

    // Akumo adapter / edge crates the core must not pull in.
    let akumo_adapter = dep.starts_with("akumo-provider-")
        || matches!(dep, "akumo-ledger" | "akumo-cli" | "akumo-content");

    // The domain crate must be an internal leaf: no dependency on any sibling akumo-* crate.
    let domain_leaf_violation =
        guarded == "akumo-domain" && dep != "akumo-domain" && dep.starts_with("akumo-");

    cloud_sdk || akumo_adapter || domain_leaf_violation
}

fn dep_lint() -> i32 {
    let meta = match cargo_metadata() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("dep-lint: failed to run `cargo metadata`: {e}");
            return 1;
        }
    };

    // id -> package name
    let mut id_to_name: HashMap<String, String> = HashMap::new();
    if let Some(pkgs) = meta["packages"].as_array() {
        for p in pkgs {
            if let (Some(id), Some(name)) = (p["id"].as_str(), p["name"].as_str()) {
                id_to_name.insert(id.to_string(), name.to_string());
            }
        }
    }

    // name -> id (for workspace members we guard)
    let name_to_id: HashMap<&str, &str> =
        id_to_name.iter().map(|(id, name)| (name.as_str(), id.as_str())).collect();

    // Adjacency over NORMAL deps only (exclude dev/build), from the resolve graph.
    let mut normal_deps: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(nodes) = meta["resolve"]["nodes"].as_array() {
        for node in nodes {
            let Some(id) = node["id"].as_str() else { continue };
            let mut out = Vec::new();
            if let Some(deps) = node["deps"].as_array() {
                for d in deps {
                    let is_normal = d["dep_kinds"]
                        .as_array()
                        .map(|ks| ks.iter().any(|k| k["kind"].is_null()))
                        .unwrap_or(false);
                    if is_normal {
                        if let Some(pkg) = d["pkg"].as_str() {
                            out.push(pkg.to_string());
                        }
                    }
                }
            }
            normal_deps.insert(id.to_string(), out);
        }
    }

    let mut violations: Vec<String> = Vec::new();

    for &guarded in GUARDED_CRATES {
        let Some(&start) = name_to_id.get(guarded) else {
            eprintln!("dep-lint: warning — guarded crate '{guarded}' not found in workspace");
            continue;
        };

        // BFS the transitive normal-dependency closure.
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(start.to_string());
        while let Some(cur) = queue.pop_front() {
            if !seen.insert(cur.clone()) {
                continue;
            }
            if let Some(children) = normal_deps.get(&cur) {
                for c in children {
                    queue.push_back(c.clone());
                }
            }
        }

        for dep_id in &seen {
            if dep_id.as_str() == start {
                continue;
            }
            if let Some(name) = id_to_name.get(dep_id) {
                if is_forbidden(guarded, name) {
                    violations.push(format!("{guarded}  ->  {name}"));
                }
            }
        }
    }

    if violations.is_empty() {
        println!("dep-lint: OK — the core is provider-blind (checked {GUARDED_CRATES:?}).");
        0
    } else {
        eprintln!("dep-lint: FAIL — forbidden dependency direction (NFR-EXT1, ADR-0001):");
        for v in &violations {
            eprintln!("  - {v}");
        }
        eprintln!(
            "\nThe core must reach cloud/adapters only through ports. Move the dependency behind a \
             port + adapter, or into the binary/CLI wiring layer."
        );
        1
    }
}

/// Auto-generate a Markdown reference page per technique (task G18.7, NFR-USE3) so docs never drift
/// from behavior. `xtask docgen [content_dir] [out_dir]`.
fn docgen() -> i32 {
    use akumo_dsl::schema::{StepBody, Technique};

    let content = std::env::args().nth(2).unwrap_or_else(|| "content".to_string());
    let out = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "../docs/external/reference/techniques".to_string());

    let mut catalog = akumo_dsl::Catalog::new();
    match catalog.load_dir(&content) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("docgen: failed to load content from '{content}': {e}");
            return 1;
        }
    }

    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("docgen: cannot create '{out}': {e}");
        return 1;
    }

    let mut index = String::from(
        "# Technique Reference\n\n_Auto-generated by `xtask docgen` from the technique definitions. \
         Do not edit by hand._\n\n",
    );

    fn render(t: &Technique) -> String {
        let m = &t.metadata;
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n`{}`\n\n{}\n\n", m.name, m.id, m.description));
        s.push_str("## Metadata\n\n");
        s.push_str(&format!(
            "- **Provider:** {}\n- **Version:** {}\n- **Impact:** {}\n- **MITRE:** {}\n",
            m.provider,
            m.version,
            m.impact,
            m.mitre.join(", ")
        ));
        if !m.references.is_empty() {
            s.push_str(&format!("- **References:** {}\n", m.references.join(", ")));
        }
        s.push('\n');

        if !t.contract.inputs.is_empty()
            || !t.contract.preconditions.is_empty()
            || !t.contract.effects.is_empty()
        {
            s.push_str("## Contract\n\n");
            if !t.contract.inputs.is_empty() {
                let inputs: Vec<String> =
                    t.contract.inputs.iter().map(|i| format!("`{}`", i.name)).collect();
                s.push_str(&format!("**Inputs:** {}\n\n", inputs.join(", ")));
            }
            if !t.contract.preconditions.is_empty() {
                s.push_str("**Preconditions:**\n");
                for p in &t.contract.preconditions {
                    s.push_str(&format!(
                        "- `{}({})` ({:?})\n",
                        p.predicate.name,
                        p.predicate.args.join(", "),
                        p.min_status
                    ));
                }
                s.push('\n');
            }
            if !t.contract.effects.is_empty() {
                s.push_str("**Effects:**\n");
                for e in &t.contract.effects {
                    s.push_str(&format!("- `{}({})`\n", e.predicate.name, e.predicate.args.join(", ")));
                }
                s.push('\n');
            }
        }

        if !t.steps.is_empty() {
            s.push_str("## Steps\n\n");
            for step in &t.steps {
                let body = match &step.body {
                    StepBody::Call { service, operation, .. } => {
                        format!("call `{service}.{operation}`")
                    }
                    StepBody::Script { language, .. } => format!("script ({language:?})"),
                };
                let revert = step
                    .revert
                    .as_ref()
                    .map(|r| format!(" · revert `{}.{}`", r.service, r.operation))
                    .unwrap_or_default();
                s.push_str(&format!("- **{}** — {}{}\n", step.id, body, revert));
            }
            s.push('\n');
        }

        if !m.expected_telemetry.is_empty() {
            s.push_str("## Expected telemetry\n\n");
            for sig in &m.expected_telemetry {
                s.push_str(&format!("- `{}` / `{}`\n", sig.source, sig.event_name));
            }
            s.push('\n');
        }

        s.push_str("_Auto-generated by `xtask docgen` from the technique definition._\n");
        s
    }

    let mut count = 0;
    for technique in catalog.all() {
        let slug = doc_slug(&technique.metadata.id);
        let path = format!("{out}/{slug}.md");
        if let Err(e) = std::fs::write(&path, render(technique)) {
            eprintln!("docgen: cannot write '{path}': {e}");
            return 1;
        }
        index.push_str(&format!(
            "- [{}]({}.md) — {} [{}]\n",
            technique.metadata.id, slug, technique.metadata.name, technique.metadata.impact
        ));
        count += 1;
    }

    if let Err(e) = std::fs::write(format!("{out}/README.md"), index) {
        eprintln!("docgen: cannot write index: {e}");
        return 1;
    }

    println!("docgen: wrote {count} technique page(s) to {out}");
    0
}

fn doc_slug(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// Orphaned-resource leak detector (task G19.2). This is the **local** proxy: it scans engagement
/// ledgers for steps that detonated but were never reverted — i.e. mutations still standing on the
/// target. (The live-AWS tier additionally scans the account itself; that runs release-gated with
/// credentials.) Exits non-zero if any outstanding detonation is found, so CI can gate on it.
///
/// `leak-detector [state_dir]` (default `./.akumo-state`).
fn leak_detector() -> i32 {
    use std::fs;

    let dir = std::env::args().nth(2).unwrap_or_else(|| "./.akumo-state".to_string());
    let path = std::path::Path::new(&dir);
    if !path.exists() {
        println!("leak-detector: no state dir at '{dir}'; nothing to check");
        return 0;
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("leak-detector: cannot read '{dir}': {e}");
            return 1;
        }
    };

    let mut total_outstanding = 0usize;
    for entry in entries {
        let file = match entry {
            Ok(e) => e.path(),
            Err(_) => continue,
        };
        if file.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let engagement = file.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
        let content = match fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("leak-detector: cannot read {}: {e}", file.display());
                return 1;
            }
        };

        // Key detonated/reverted steps by (detonation_id, step_id).
        let mut detonated: HashMap<(String, String), String> = HashMap::new();
        let mut reverted: BTreeSet<(String, String)> = BTreeSet::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let payload = &event["payload"];
            let det = payload.get("detonation_id").and_then(|v| v.as_str()).unwrap_or("");
            let step = payload.get("step_id").and_then(|v| v.as_str()).unwrap_or("");
            let key = (det.to_string(), step.to_string());
            match event_type {
                "StepDetonated" => {
                    let call = payload.get("call");
                    let service = call.and_then(|c| c.get("service")).and_then(|v| v.as_str()).unwrap_or("?");
                    let operation = call.and_then(|c| c.get("operation")).and_then(|v| v.as_str()).unwrap_or("?");
                    detonated.insert(key, format!("{service}.{operation}"));
                }
                "StepReverted" => {
                    reverted.insert(key);
                }
                _ => {}
            }
        }

        let mut outstanding: Vec<(String, String, String)> = Vec::new(); // (call, detonation, step)
        for ((det, step), call) in detonated.iter() {
            if !reverted.contains(&(det.clone(), step.clone())) {
                outstanding.push((call.clone(), det.clone(), step.clone()));
            }
        }
        if !outstanding.is_empty() {
            eprintln!("engagement '{engagement}': {} outstanding detonation(s):", outstanding.len());
            for (call, det, step) in &outstanding {
                eprintln!("  - {call}  (detonation {det}, step {step})");
            }
            total_outstanding += outstanding.len();
        }
    }

    if total_outstanding == 0 {
        println!("leak-detector: OK — no outstanding (unreverted) detonations");
        0
    } else {
        eprintln!(
            "\nleak-detector: FAIL — {total_outstanding} outstanding detonation(s); possible orphaned resources"
        );
        1
    }
}

fn cargo_metadata() -> Result<serde_json::Value, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata exited with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}
