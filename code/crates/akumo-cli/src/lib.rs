//! # akumo-cli
//!
//! The driving adapters over the core (ADR-0001 / ADR-0029): a **command-per-invocation** CLI —
//! stateless invocations over the persistent engagement ledger, which cleanly solves the
//! "restart to switch sessions" pitfall (FR-A6) — plus an **optional interactive shell**. Machine
//! output is available via `--json` (FR-K4).
//!
//! v1 wires the **Mock** provider so the whole command surface is exercisable without a cloud
//! account; the AWS adapter (G16) slots in behind the same `Provider` seam.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use akumo_core::api::Akumo;
use akumo_core::planner::{Objective, PlannerConfig};
use akumo_core::telemetry::sigma_rules_for;
use akumo_domain::ids::{Actor, EngagementId, ProviderId};
use akumo_domain::ports::Provider;
use akumo_domain::scope::{Scope, ScopeSelector};
use akumo_domain::seam::EnumerationDescriptor;
use akumo_dsl::script::UnsupportedScriptHost;
use akumo_dsl::Catalog;
use akumo_ledger::FileEventStore;
use akumo_provider_aws::AwsProvider;
use akumo_provider_mock::{MockEnvironment, MockProvider};

/// Stable, machine-consumable process exit codes (FR-K4).
pub mod exit {
    /// Successful completion.
    pub const OK: i32 = 0;
    /// A runtime failure (see stderr).
    pub const FAILURE: i32 = 1;
    /// A usage error (bad arguments).
    pub const USAGE: i32 = 2;
}

#[derive(Parser)]
#[command(name = "akumo", version, about = "Akumo — cloud offensive security framework")]
struct Cli {
    /// Directory holding engagement ledgers.
    #[arg(long, global = true, default_value = "./.akumo-state")]
    state_dir: PathBuf,
    /// The provider to operate against.
    #[arg(long, global = true, value_enum, default_value = "mock")]
    provider: ProviderKind,
    /// Emit machine-readable JSON where supported.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum ProviderKind {
    /// The built-in scriptable Mock provider.
    Mock,
    /// AWS (arrives in a later release).
    Aws,
}

#[derive(Clone, Copy, ValueEnum)]
enum ReportFormat {
    /// Human-readable Markdown.
    Markdown,
    /// Machine-readable JSON.
    Json,
}

#[derive(Subcommand)]
enum Command {
    /// Engagement lifecycle.
    Engagement {
        #[command(subcommand)]
        cmd: EngagementCmd,
    },
    /// Validate credentials & provider metadata without enumerating.
    Authcheck,
    /// Enumerate the target into the attack graph.
    Enumerate {
        #[arg(long)]
        engagement: String,
        /// Enumeration descriptors as `service.operation`.
        #[arg(long = "descriptor")]
        descriptors: Vec<String>,
    },
    /// Compute attack paths toward an objective.
    Paths {
        #[arg(long)]
        engagement: String,
        /// `admin` or `resource:<id>`.
        #[arg(long)]
        objective: String,
    },
    /// List techniques in the content catalog.
    Catalog {
        #[arg(long, default_value = "content")]
        content: PathBuf,
    },
    /// Show a technique's metadata and generated Sigma detections.
    Technique {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "content")]
        content: PathBuf,
    },
    /// Preview a technique (dry-run + blast radius; no mutation).
    Preview {
        #[arg(long)]
        engagement: String,
        #[arg(long)]
        technique: String,
        #[arg(long, default_value = "content")]
        content: PathBuf,
        /// Inputs as `key=value`.
        #[arg(long = "input")]
        inputs: Vec<String>,
    },
    /// Detonate a technique through the safe lifecycle.
    Run {
        #[arg(long)]
        engagement: String,
        #[arg(long)]
        technique: String,
        #[arg(long, default_value = "content")]
        content: PathBuf,
        #[arg(long = "input")]
        inputs: Vec<String>,
        /// Record consent at the technique's impact level before running.
        #[arg(long)]
        consent: bool,
    },
    /// Revert outstanding detonations for an engagement.
    Revert {
        #[arg(long)]
        engagement: String,
    },
    /// Generate the engagement report.
    Report {
        #[arg(long)]
        engagement: String,
        #[arg(long, default_value = "content")]
        content: PathBuf,
        #[arg(long, value_enum, default_value = "markdown")]
        format: ReportFormat,
    },
    /// Start an interactive shell.
    Shell,
}

#[derive(Subcommand)]
enum EngagementCmd {
    /// Open a new engagement.
    Open {
        #[arg(long)]
        id: String,
        /// Authorized scope selectors as `kind:value`.
        #[arg(long = "scope")]
        scopes: Vec<String>,
        /// Affirm you are authorized to test this target (required).
        #[arg(long)]
        affirm: bool,
    },
    /// List engagements.
    List,
    /// Show an engagement's state.
    Show {
        #[arg(long)]
        engagement: String,
    },
    /// Close an engagement.
    Close {
        #[arg(long)]
        engagement: String,
    },
}

/// Shell-line wrapper so the interactive shell can reuse the same command grammar.
#[derive(Parser)]
#[command(no_binary_name = true)]
struct ShellLine {
    #[command(subcommand)]
    command: Command,
}

/// Everything a command handler needs.
struct Ctx {
    store: FileEventStore,
    provider: Box<dyn Provider>,
    host: UnsupportedScriptHost,
    actor: Actor,
    json: bool,
}

/// Entry point. Parses arguments, starts a runtime, and dispatches.
pub fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            use clap::error::ErrorKind;
            return match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => exit::OK,
                _ => exit::USAGE,
            };
        }
    };

    let store = match FileEventStore::open(&cli.state_dir) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("error: could not open state dir {}: {e}", cli.state_dir.display());
            return exit::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: could not start runtime: {e}");
            return exit::FAILURE;
        }
    };

    runtime.block_on(async move {
        let provider: Box<dyn Provider> = match cli.provider {
            ProviderKind::Mock => Box::new(demo_provider()),
            ProviderKind::Aws => match AwsProvider::connect().await {
                Ok(p) => Box::new(p),
                Err(e) => {
                    eprintln!("error: could not connect to AWS: {e}");
                    return exit::FAILURE;
                }
            },
        };
        let ctx = Ctx {
            store,
            provider,
            host: UnsupportedScriptHost,
            actor: current_actor(),
            json: cli.json,
        };
        match cli.command {
            Command::Shell => shell(&ctx).await,
            command => dispatch(command, &ctx).await,
        }
    })
}

fn demo_provider() -> MockProvider {
    MockProvider::new(
        MockEnvironment::builder(
            "mock",
            akumo_domain::principal::Principal::new(
                "arn:aws:iam::000000000000:user/akumo-operator",
                akumo_domain::principal::PrincipalKind::User,
                ProviderId::new("mock"),
            ),
        )
        .region("mock-region-1")
        .build(),
    )
}

fn current_actor() -> Actor {
    Actor::new(std::env::var("USER").unwrap_or_else(|_| "operator".to_string()))
}

/// Dispatch one command, returning a process exit code.
async fn dispatch(command: Command, ctx: &Ctx) -> i32 {
    let result = handle(command, ctx).await;
    match result {
        Ok(()) => exit::OK,
        Err(e) => {
            eprintln!("error: {e}");
            exit::FAILURE
        }
    }
}

async fn handle(command: Command, ctx: &Ctx) -> Result<(), String> {
    let akumo = Akumo::new(&ctx.store, ctx.provider.as_ref(), &ctx.host);
    match command {
        Command::Shell => Ok(()), // handled at the top level
        Command::Engagement { cmd } => handle_engagement(cmd, ctx, &akumo).await,
        Command::Authcheck => {
            let report = akumo.authcheck().await.map_err(|e| e.to_string())?;
            if ctx.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "version": 1,
                        "principal": report.principal.id,
                        "provider": report.provider.to_string(),
                        "regions": report.regions.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
                    })
                );
            } else {
                println!("principal: {}", report.principal.id);
                println!("provider:  {}", report.provider);
                println!("regions:   {}", report.regions.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", "));
            }
            Ok(())
        }
        Command::Enumerate { engagement, descriptors } => {
            let id = EngagementId::new(engagement);
            let descs: Vec<EnumerationDescriptor> =
                descriptors.iter().map(|d| parse_descriptor(d)).collect();
            let summary = akumo
                .enumerate(&id, ctx.actor.clone(), &descs)
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "enumerated {} descriptor(s): {} facts, {} coverage gap(s)",
                summary.descriptors_run, summary.facts_asserted, summary.coverage_gaps
            );
            Ok(())
        }
        Command::Paths { engagement, objective } => {
            let id = EngagementId::new(engagement);
            let objective = parse_objective(&objective)?;
            let paths = akumo
                .paths(&id, &objective, &[], &PlannerConfig::default())
                .await
                .map_err(|e| e.to_string())?;
            if paths.is_empty() {
                println!("no paths found toward the objective");
            } else {
                for (i, path) in paths.iter().enumerate() {
                    println!("--- path {} ---\n{}\n", i + 1, path.explain());
                }
            }
            Ok(())
        }
        Command::Catalog { content } => {
            let catalog = load_catalog(&content)?;
            if catalog.is_empty() {
                println!("catalog is empty (looked in {})", content.display());
            }
            for t in catalog.all() {
                println!(
                    "{}  [{}]  {}  — {}",
                    t.metadata.id, t.metadata.impact, t.metadata.provider, t.metadata.name
                );
            }
            Ok(())
        }
        Command::Technique { id, content } => {
            let catalog = load_catalog(&content)?;
            let t = catalog.get(&id).ok_or_else(|| format!("technique '{id}' not found"))?;
            println!("id:       {}", t.metadata.id);
            println!("name:     {}", t.metadata.name);
            println!("impact:   {}", t.metadata.impact);
            println!("mitre:    {}", t.metadata.mitre.join(", "));
            println!("steps:    {}", t.steps.len());
            println!("\n# candidate Sigma detections\n");
            for rule in sigma_rules_for(t) {
                println!("{}", rule.to_yaml());
            }
            Ok(())
        }
        Command::Preview { engagement, technique, content, inputs } => {
            let id = EngagementId::new(engagement);
            let catalog = load_catalog(&content)?;
            let t = catalog
                .get(&technique)
                .ok_or_else(|| format!("technique '{technique}' not found"))?;
            let input_map = parse_inputs(&inputs)?;
            let outcome = akumo
                .preview(&id, ctx.actor.clone(), t, &input_map)
                .await
                .map_err(|e| e.to_string())?;
            let br = &outcome.blast_radius;
            println!("technique:    {}", br.technique_id);
            println!("impact:       {} (reversible: {})", br.max_impact, br.max_impact.is_reversible());
            println!("provider calls: {}", br.provider_calls.join(", "));
            if !br.effects.is_empty() {
                println!("effects:      {}", br.effects.join(", "));
            }
            Ok(())
        }
        Command::Run { engagement, technique, content, inputs, consent } => {
            let id = EngagementId::new(engagement);
            let catalog = load_catalog(&content)?;
            let t = catalog
                .get(&technique)
                .ok_or_else(|| format!("technique '{technique}' not found"))?;
            let input_map = parse_inputs(&inputs)?;
            if consent && t.metadata.impact.requires_consent() {
                akumo
                    .record_consent(&id, ctx.actor.clone(), t.metadata.impact, true)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            let outcome = akumo
                .run(&id, ctx.actor.clone(), t, &input_map, Default::default())
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "status: {:?} — {} step(s) detonated, {} verified",
                outcome.status, outcome.steps_detonated, outcome.steps_verified
            );
            if let Some(report) = &outcome.revert {
                println!("reverted {} step(s); {} could not be undone", report.reverted, report.failed.len());
            }
            if matches!(outcome.status, akumo_core::execution::ExecStatus::ConsentRequired) {
                println!("hint: re-run with --consent to authorize this {} action", t.metadata.impact);
            }
            Ok(())
        }
        Command::Revert { engagement } => {
            let id = EngagementId::new(engagement);
            let report = akumo.revert(&id, ctx.actor.clone()).await.map_err(|e| e.to_string())?;
            println!("reverted {} step(s); {} could not be undone", report.reverted, report.failed.len());
            for failure in &report.failed {
                println!("  ! {failure}");
            }
            Ok(())
        }
        Command::Report { engagement, content, format } => {
            let id = EngagementId::new(engagement);
            let catalog = load_catalog(&content).ok();
            let report = akumo.report(&id, catalog.as_ref()).await.map_err(|e| e.to_string())?;
            let as_json = ctx.json || matches!(format, ReportFormat::Json);
            if as_json {
                println!("{}", report.to_json());
            } else {
                println!("{}", report.to_markdown());
            }
            Ok(())
        }
    }
}

async fn handle_engagement(cmd: EngagementCmd, ctx: &Ctx, akumo: &Akumo<'_>) -> Result<(), String> {
    match cmd {
        EngagementCmd::Open { id, scopes, affirm } => {
            let scope = parse_scopes(&scopes)?;
            let engagement = akumo
                .open_engagement(
                    EngagementId::new(id),
                    scope,
                    ProviderId::new("mock"),
                    "cli",
                    ctx.actor.clone(),
                    affirm,
                )
                .await
                .map_err(|e| e.to_string())?;
            println!("opened engagement {engagement}");
            Ok(())
        }
        EngagementCmd::List => {
            let ids = ctx.store.list_engagements().map_err(|e| e.to_string())?;
            if ids.is_empty() {
                println!("no engagements");
            }
            for id in ids {
                println!("{id}");
            }
            Ok(())
        }
        EngagementCmd::Show { engagement } => {
            let id = EngagementId::new(engagement);
            match akumo.engagement_state(&id).await.map_err(|e| e.to_string())? {
                Some(state) => {
                    println!("id:       {}", state.id);
                    println!("provider: {}", state.provider);
                    println!("status:   {:?}", state.status);
                    println!("scope:    {} selector(s)", state.scope.allowed.len());
                    println!(
                        "consent:  {}",
                        state.max_consent.map(|i| i.to_string()).unwrap_or_else(|| "none".to_string())
                    );
                    Ok(())
                }
                None => Err(format!("engagement '{id}' not found")),
            }
        }
        EngagementCmd::Close { engagement } => {
            let id = EngagementId::new(engagement);
            akumo.close_engagement(&id, ctx.actor.clone()).await.map_err(|e| e.to_string())?;
            println!("closed engagement {id}");
            Ok(())
        }
    }
}

/// The optional interactive shell: the same commands over the persistent ledger, no mutable session
/// state of its own (ADR-0029).
async fn shell(ctx: &Ctx) -> i32 {
    use std::io::Write;
    println!("akumo shell — type a command, 'help', or 'exit'");
    let stdin = std::io::stdin();
    loop {
        print!("akumo> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("error reading input: {e}");
                break;
            }
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        match tokens.first().copied() {
            None => continue,
            Some("exit") | Some("quit") => break,
            Some("shell") => {
                println!("already in a shell");
                continue;
            }
            _ => {}
        }
        match ShellLine::try_parse_from(tokens) {
            Ok(parsed) => {
                let _ = dispatch(parsed.command, ctx).await;
            }
            Err(e) => {
                let _ = e.print();
            }
        }
    }
    exit::OK
}

// ---- helpers -----------------------------------------------------------------------------------

fn load_catalog(content: &std::path::Path) -> Result<Catalog, String> {
    let mut catalog = Catalog::new();
    if content.exists() {
        catalog.load_dir(content).map_err(|e| e.to_string())?;
    }
    Ok(catalog)
}

fn parse_scopes(pairs: &[String]) -> Result<Scope, String> {
    if pairs.is_empty() {
        return Err("at least one --scope kind:value is required".to_string());
    }
    let mut selectors = Vec::new();
    for pair in pairs {
        let (kind, value) = pair
            .split_once(':')
            .ok_or_else(|| format!("scope '{pair}' must be kind:value"))?;
        selectors.push(ScopeSelector::new(kind, value));
    }
    Ok(Scope::new(selectors))
}

fn parse_descriptor(spec: &str) -> EnumerationDescriptor {
    let (service, operation) = spec.split_once('.').unwrap_or((spec, ""));
    EnumerationDescriptor {
        service: service.to_string(),
        operation: operation.to_string(),
        params: serde_json::Map::new(),
        required_permission: None,
    }
}

fn parse_inputs(pairs: &[String]) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let mut map = serde_json::Map::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("input '{pair}' must be key=value"))?;
        map.insert(key.to_string(), serde_json::Value::String(value.to_string()));
    }
    Ok(map)
}

fn parse_objective(spec: &str) -> Result<Objective, String> {
    if spec == "admin" {
        Ok(Objective::ReachAdmin)
    } else if let Some(resource) = spec.strip_prefix("resource:") {
        Ok(Objective::ReachResource { resource: resource.to_string() })
    } else {
        Err("objective must be 'admin' or 'resource:<id>'".to_string())
    }
}
