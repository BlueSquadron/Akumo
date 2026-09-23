# Akumo — Prior Art Analysis & Intent of Work

> **Status:** Research draft (no code). This document establishes *why Akumo exists* by
> studying the strongest tools in cloud offensive security, assessing their functional
> and non-functional strengths and weaknesses, mapping the whitespace they leave open,
> and translating that whitespace into requirements and a differentiated thesis for Akumo.
>
> **Primary inspirations:** [Pacu](https://github.com/RhinoSecurityLabs/pacu) (Rhino Security Labs)
> and [Stratus Red Team](https://github.com/DataDog/stratus-red-team) (Datadog).
>
> **Author:** Akumo project • **Date:** 2026-09-21

---

## 1. Purpose & Scope

Akumo is intended to be a **security testing framework for cloud-hosted infrastructure**.
Before writing a line of code we want a clear-eyed answer to one question:

> *Given that Pacu, Stratus Red Team, and a mature field of adjacent tools already exist,
> what does Akumo do that is genuinely better or genuinely new — and what must it therefore
> be built to do?*

This document:

1. Establishes an evaluation framework (functional + non-functional axes).
2. Deep-dives the two flagship inspirations (Pacu, Stratus Red Team).
3. Capsule-analyzes the six adjacent reference tools (Grimoire, ScoutSuite, MicroBurst,
   Prowler, CloudFox, COBRA).
4. Maps the whole field onto the cloud kill chain and identifies the **cross-cutting gaps**.
5. Proposes **Akumo's thesis and differentiators**.
6. Derives concrete **functional and non-functional requirements**.
7. Records **anti-goals, risks, ethics, and open decisions**.

> **Ethics & scope note.** Everything below assumes Akumo is used **only against
> infrastructure the operator is authorized to test** (owned accounts, contracted
> engagements, CTF/lab environments). Authorization gating, auditability, and safety are
> treated as first-class requirements, not afterthoughts.

---

## 2. Evaluation Framework

Each tool is assessed on two axes.

**Functional (what it can do):**

| Dimension | Question |
|---|---|
| Kill-chain coverage | Recon → enumeration → initial access → privesc → persistence → lateral movement → exfiltration → impact → defense evasion |
| Cloud coverage | AWS / Azure / GCP / Kubernetes / identity providers / SaaS |
| Discovery vs. action | Does it *find* attack paths, *execute* them, or only *validate detections*? |
| Chaining | Atomic techniques only, or multi-step attack chains? |
| Attack-path reasoning | Does it model the environment as a graph and compute reachable paths? |
| Detection value | Does it produce telemetry / MITRE mapping / datasets for blue teams? |

**Non-functional (how well it does it):**

| Dimension | Question |
|---|---|
| Safety / reversibility | Blast-radius control, dry-run, rollback, destructive-by-default? |
| Extensibility | How easy is it to add content? Imperative code vs. declarative spec? |
| Portability / install | Single binary vs. language runtime + dependency tree? |
| State model | How is discovered data and execution state stored, resumed, audited? |
| Usability | Interactive vs. scriptable; learning curve; output quality |
| Maintainability | Maintenance cadence, contributor base, test coverage of content |
| Auditability | Reproducibility, logging, evidence, non-repudiation |
| Performance | Concurrency, API-call efficiency (and resulting log/rate footprint) |

---

## 3. The Landscape at a Glance

The reference tools cluster into four distinct *jobs*, and almost none of them do more
than one well:

```
                     READ-ONLY / POSTURE            OFFENSIVE / ACTION
                 ┌──────────────────────────┬──────────────────────────────┐
   FIND          │  Prowler                 │  CloudFox                     │
 (discover /     │  ScoutSuite              │  (attack-path enumeration,    │
  enumerate)     │  (CSPM, compliance,      │   read-only recon)            │
                 │   config risk)           │                               │
                 ├──────────────────────────┼──────────────────────────────┤
   ACT           │  Grimoire                │  Pacu (AWS exploitation)      │
 (execute /      │  (log dataset gen for    │  MicroBurst (Azure post-ex)   │
  detonate)      │   detection eng.)        │  Stratus Red Team (atomic     │
                 │                          │   detonation / detection val.)│
                 │                          │  COBRA (multi-stage chains)   │
                 └──────────────────────────┴──────────────────────────────┘
```

**Key observation:** the four quadrants are populated by *separate tools with separate
data models*. An operator today stitches them together by hand — scan with Prowler,
enumerate paths with CloudFox, exploit with Pacu, then validate detections with Stratus
+ Grimoire — copy-pasting findings across four different formats. **The seam between
"discover a path" and "safely execute that path" is where the field is weakest, and it is
where Akumo will live.**

---

## 4. Deep Dive — Pacu (Rhino Security Labs)

*The "Metasploit for AWS."* Open-source AWS exploitation framework in Python 3.

### 4.1 Functional profile

- **Positioning:** Explicitly the offensive counterpart to cloud scanners — "several AWS
  security scanners serve as the *Nessus* of the cloud; Pacu is designed to be the
  *Metasploit* equivalent."
- **Kill-chain coverage:** Broad for a single cloud — ~36 modules spanning
  **enumeration** (account/spend/IAM/EC2/S3 recon), **privilege escalation**
  (`iam__privesc_scan`, built on Rhino's well-known research into ~20+ AWS privesc paths),
  **persistence / backdooring** (IAM user keys, login profiles, assume-role, Route53),
  **exploitation** (`ec2__startup_shell_script`, `ecs__backdoor_task_def`, Lambda abuse),
  **exfiltration** (S3 download, EBS/RDS snapshot exploration, SSM parameter dumping), and
  **defense evasion / log manipulation** (`detection__disruption` to stop CloudTrail /
  GuardDuty; CloudTrail history download).
- **Model:** *Module-first.* The operator picks and runs modules; chaining is manual and
  operator-driven. There is no environment graph or automated path computation.
- **Cloud coverage:** **AWS only.**

### 4.2 Architecture

- Python 3.7+ CLI with an interactive shell (`list`, `run <module>`, `help`) plus a
  non-interactive `--exec` mode for scripting.
- **Session concept:** credentials + all collected data live in a named session, persisted
  to a **local SQLite database**. A deliberate design goal is to **cache API responses to
  minimize redundant AWS calls (and the CloudTrail logs they generate)**.
- **Shared data structure** across modules: regions and permission checks are handled once
  so modules don't re-implement them.

### 4.3 Strengths

**Functional**
- Deepest *exploitation* coverage of any open-source AWS tool; genuinely executes attacks,
  not just finds them.
- `iam__privesc_scan` encodes hard-won privesc research — real practitioner value.
- Session/data-caching model is smart about API and log footprint.

**Non-functional**
- Familiar Metasploit-like UX lowers the barrier for pentesters.
- Extensible module system with a documented development guide; large community catalog.
- Long-lived, recognized project — de-facto standard for AWS offense.

### 4.4 Weaknesses & gaps

**Functional**
- **AWS-only.** No Azure, GCP, Kubernetes, or identity-provider coverage — a hard ceiling
  in a world where 89% of orgs are multi-cloud.
- **No attack-path reasoning.** It enumerates and it exploits, but it does not build a
  graph or compute *which* chain of steps reaches a target. The operator is the planner.
- **No safe-mode / revert.** Modules mutate the target (create backdoors, stop CloudTrail)
  with **no built-in rollback, dry-run, or blast-radius estimation**. Cleanup is manual.
- **Weak detection/purple-team output.** No native MITRE ATT&CK mapping, no "here is the
  telemetry this action produced" — the blue-team feedback loop is absent.

**Non-functional**
- **Imperative Python modules** are powerful but harder to audit, sandbox, test, and trust
  than a declarative technique spec.
- **Runtime friction:** Python environment + dependency tree; officially macOS/Linux only;
  "restart required to switch sessions." Compare to a single static binary.
- **State is a private SQLite DB** — not designed as an inspectable, reproducible,
  shareable evidence ledger.
- Content is not systematically **tested** technique-by-technique, and safety is not
  enforced by the framework.

> **Takeaway for Akumo:** Pacu proves the demand for *real cloud exploitation* and for a
> smart, log-aware state cache. Its ceilings — single cloud, no path reasoning, no safety
> rails, imperative/untested content, heavy runtime — are precisely the things Akumo should
> invert.

---

## 5. Deep Dive — Stratus Red Team (Datadog)

*"Atomic Red Team for the cloud."* A self-contained Go binary that detonates **granular,
well-documented, self-contained attack techniques** and then cleanly reverts them.

### 5.1 Functional profile

- **Positioning:** Purpose-built for **detection engineering and purple teaming**, *not*
  end-to-end exploitation. Each technique answers: *"if an attacker did X, would my
  CloudTrail/GuardDuty/Sentinel/Falco detections fire?"*
- **Technique catalog (approx., grows over time):** AWS 50+, GCP 20+, Azure 10+,
  Entra ID 6+, Kubernetes 8+, EKS 2+. Every technique is **mapped to MITRE ATT&CK** and
  ships an auto-generated documentation page describing the exact API calls it makes.
- **Cloud coverage:** AWS, Azure, GCP, Entra ID, Kubernetes/EKS — genuinely multi-platform.
- **Model:** *Atomic techniques against your own account.* It does **not** enumerate a
  target, does **not** discover attack paths, and does **not** chain techniques into an
  attack. It fabricates the prerequisites, fires one technique, and reverts.

### 5.2 Architecture — the lifecycle that makes it safe

The idea Akumo most admires here is the **four-stage, idempotent lifecycle**:

```
 warm-up  ──►  detonate  ──►  revert  ──►  cleanup
 (create      (execute      (undo the    (destroy the
  prereqs      the attack    detonation   prereq infra)
  via          technique)    effects)
  Terraform)
```

- Self-contained **Go binary** (Homebrew / Docker / binary / asdf) — Go 1.23+ to build.
- **Terraform under the hood** provisions the "warm" prerequisite infrastructure
  (test IAM roles, buckets, instances) so a technique can run in isolation.
- Usable both as a **CLI** and as a **Go library** (`/examples`), enabling embedding into
  pipelines and higher-level tools.
- State tracks whether each technique is `COLD`/`WARM`/`DETONATED` so operations are
  idempotent and resumable.

### 5.3 Strengths

**Functional**
- **True multi-cloud + identity-plane** coverage from one tool.
- **MITRE-mapped, self-documenting** techniques — every detonation has a known telemetry
  signature; this is the purple-team value Pacu lacks.
- **Non-destructive by design:** warm-up creates *its own* disposable targets; revert +
  cleanup restore state. Safe to run in real (owned) environments.

**Non-functional**
- **Single static binary** — dramatically lower install/runtime friction than Python.
- **Declarative-ish, self-contained techniques** with generated docs → easy to audit,
  reason about, and contribute to; techniques are consistent and testable.
- Clean **library API** → composable; already used as a building block by Grimoire.
- Idempotent lifecycle + explicit state → reproducible, CI-friendly.

### 5.4 Weaknesses & gaps

**Functional**
- **Not an exploitation framework.** It emulates *predefined atomic behaviors on synthetic
  prerequisites you own* — it does **not** attack a real target's actual resources,
  discover what's exploitable, or achieve an objective.
- **No enumeration / no attack-path discovery.** It cannot tell you *which* techniques are
  relevant to a given environment; the human chooses techniques blind to the target.
- **No chaining / no objective.** No notion of "reach the prod database" via a sequence of
  steps. Atomic only, by philosophy.
- **Warm-up fabricates targets**, so it validates *detections*, not *real-world
  exploitability* of the environment under test.

**Non-functional**
- Terraform dependency for warm-up adds infra state to manage and can drift/leak if
  cleanup fails.
- Adding a technique requires **Go**, which is a higher bar than YAML/DSL for community
  detection engineers.

> **Takeaway for Akumo:** Stratus contributes the single most important *engineering
> pattern* in this field — the **safe, idempotent, revertible, MITRE-mapped technique
> lifecycle**. Its self-imposed limits — atomic only, synthetic targets, no discovery, no
> chaining, no objective-seeking — are exactly the offensive capabilities Akumo can add
> *on top of* that safety model.

---

## 6. Adjacent Reference Tools (capsule analyses)

### 6.1 Grimoire (Datadog) — *detection-engineering feedback loop*
- **Job:** Detonate an attack, inject a UUID user-agent, poll CloudTrail, and stream back
  the *exact* logs the attack produced — a "REPL for detection engineering." AWS only.
- **Works with Stratus** (drives its ~38 AWS techniques) or an interactive shell.
- **Strength:** Closes the loop from *action → observed telemetry → detection rule* faster
  than manually sifting CloudTrail. Apache-2.0.
- **Gap for Akumo:** Single-cloud, single-purpose adjunct; proves the value of **coupling
  every offensive action to its resulting telemetry** — a pattern Akumo should make native
  rather than a separate tool.

### 6.2 ScoutSuite (NCC Group) — *multi-cloud posture snapshot*
- **Job:** Read-only, API-based configuration audit → HTML report of risk areas.
  AWS/Azure/GCP (+ alpha Alibaba/OCI). Point-in-time; offline-analyzable.
- **Strength:** Auditor-designed rulesets; strong report; multi-cloud posture.
- **Gap for Akumo:** Purely descriptive CSPM — finds misconfigurations, never validates
  exploitability or executes anything. (Akumo should *consume* posture, not reproduce it.)

### 6.3 MicroBurst (NetSPI) — *Azure post-exploitation*
- **Job:** PowerShell scripts for Azure service discovery, weak-config auditing, and
  post-ex (credential dumping, Key Vault cert export, automation run-as abuse).
- **Strength:** Deep, practitioner-grade Azure tradecraft — the Azure analog to Pacu's AWS
  depth.
- **Gap for Akumo:** **Windows/PowerShell-only, Azure-only**, script collection (not a
  framework): no unified state, no safety model, no MITRE mapping. Illustrates how the
  *offensive* space is fragmented one-cloud-per-tool while posture tools are already
  multi-cloud.

### 6.4 Prowler (Prowler Inc.) — *the CSPM heavyweight*
- **Job:** Thousands of security checks across AWS, Azure, GCP, Kubernetes, GitHub, M365,
  IaC, OCI, and more; dozens of compliance frameworks (CIS, NIST, PCI-DSS, HIPAA, FedRAMP,
  MITRE ATT&CK); **ThreatScore** risk prioritization; remediation guidance. Recently added
  **Attack Paths** (graph combining Cartography inventory with Prowler findings, AWS).
- **Strength:** The broadest, most-adopted open-source posture platform; moving toward
  graph/attack-path and "AI speed."
- **Gap for Akumo:** Still fundamentally **read-only assessment/compliance**. Its new
  attack-path graph is *descriptive* (derived from findings + inventory), not *executed or
  validated*. Akumo should treat Prowler as an **input feed**, not a competitor.

### 6.5 CloudFox (Bishop Fox) — *attack-path enumeration for operators*
- **Job:** CLI recon that surfaces exploitable attack paths — over-permissive IAM, exposed
  storage, misconfigured service accounts, weak network controls. AWS + GCP. Outputs
  CSV/JSON/table plus a **"loot" folder of ready-to-run commands**.
- **Strength:** Operator-centric ("PowerView for cloud"); bridges recon toward action by
  handing you the next command.
- **Gap for Akumo:** **Read-only** — it *illuminates* paths and hands you commands, but it
  does not execute, chain, verify, or safely revert. Its own docs note it can't resolve
  resource-policy nuances (e.g., S3 policies can deny despite IAM allow). This is the
  clearest "so close" tool: it finds paths but stops at the water's edge of *acting on
  them*.

### 6.6 COBRA (Palo Alto Networks) — *multi-stage attack simulation*
- **Job:** Simulates **multi-staged attack scenarios** across multi-cloud — external &
  insider threats, lateral movement, privesc, exfiltration, ransomware; scenarios like EC2
  takeover, REST API exploit, GKE pod compromise, credential exfil. Modular/templatized;
  produces reports.
- **Strength:** One of the few open tools that explicitly **chains** stages into scenarios.
- **Gap for Akumo:** Vendor-adjacent (Prisma Cloud ecosystem), less community mindshare;
  scenario/template-driven rather than **environment-derived** — it runs *predefined*
  chains rather than computing chains from the *actual* target graph.

---

## 7. Consolidated Comparison

| Capability | Pacu | Stratus | CloudFox | Prowler | ScoutSuite | MicroBurst | Grimoire | COBRA |
|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| Executes real attacks | ✅ | ⚠️ synthetic | ❌ | ❌ | ❌ | ✅ | ⚠️ | ✅ |
| Enumerates real target | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ | ⚠️ |
| Attack-path discovery | ❌ | ❌ | ✅ | ⚠️ new | ❌ | ❌ | ❌ | ⚠️ |
| Multi-step chaining | ⚠️ manual | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ | ✅ |
| Objective-seeking | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ |
| Safe revert / rollback | ❌ | ✅ | n/a | n/a | n/a | ❌ | ✅ | ⚠️ |
| MITRE ATT&CK mapping | ❌ | ✅ | ⚠️ | ✅ | ⚠️ | ❌ | ⚠️ | ⚠️ |
| Detection/telemetry output | ❌ | ✅ | ❌ | ⚠️ | ❌ | ❌ | ✅ | ⚠️ |
| Multi-cloud | ❌ AWS | ✅ | ⚠️ AWS+GCP | ✅ | ✅ | ❌ Azure | ❌ AWS | ✅ |
| Identity-provider plane | ❌ | ✅ Entra | ❌ | ⚠️ | ❌ | ⚠️ AAD | ❌ | ⚠️ |
| Single-binary / low-friction | ❌ Py | ✅ Go | ✅ Go | ⚠️ Py | ⚠️ Py | ❌ PS | ✅ Go | ⚠️ |
| Declarative content model | ❌ | ⚠️ Go | ❌ | ⚠️ | ⚠️ rules | ❌ | ❌ | ⚠️ tmpl |

✅ strong · ⚠️ partial/indirect · ❌ absent

---

## 8. Cross-Cutting Gap Analysis (the whitespace)

Reading across the field, seven gaps recur. Together they define Akumo's opportunity.

1. **The discovery → execution seam is unbridged.** CloudFox *finds* paths; Pacu *executes*
   techniques; no single tool derives an attack path from the *real* target graph **and**
   then safely executes/verifies it. Operators bridge this by hand across incompatible
   formats.

2. **Safety is not a first-class primitive in offensive tools.** Only Stratus/Grimoire
   have revert; they achieve it by acting on *synthetic* targets. No tool offers
   **blast-radius estimation, dry-run, transactional rollback, and rate/log-footprint
   control** for actions against a *real* environment.

3. **No attack-graph-native reasoning.** The field is *module-first* (Pacu) or
   *technique-first* (Stratus) or *finding-first* (Prowler). None treats the environment as
   a first-class **identity + resource + trust graph** over which reachable, ranked attack
   paths are computed and explained.

4. **Offense is fragmented one-cloud-per-tool** (Pacu=AWS, MicroBurst=Azure), while posture
   tools are already multi-cloud. There is no multi-cloud **exploitation** framework with
   provider parity — and almost nothing treats the **identity plane** (Entra/Okta/
   federation/OIDC into cloud, CI/CD like GitHub Actions) as the primary attack surface it
   now is.

5. **Purple-team value is bolted on, not native.** Pacu produces no telemetry mapping;
   Stratus/Grimoire produce telemetry but don't exploit. No tool makes **every offensive
   action simultaneously emit its result AND its expected detection signature** (MITRE
   technique + API calls + log fields + suggested detection).

6. **Content models are either imperative-and-untrusted or code-heavy.** Pacu = imperative
   Python (hard to audit/sandbox/test); Stratus = Go (high contributor bar). There is room
   for a **declarative, versioned, testable, MITRE-tagged technique spec** (à la Atomic Red
   Team YAML / Nuclei templates) that is safe-by-construction and community-friendly.

7. **Evidence, reproducibility, and CI-native operation are weak.** State lives in private
   SQLite (Pacu) or Terraform (Stratus). None offers a **signed, replayable audit ledger**
   suitable for engagement evidence, nor a clean **shift-left / continuous automated
   red-teaming** mode in pipelines.

---

## 9. Akumo's Thesis & Differentiators

> **Thesis:** Akumo is a **multi-cloud, attack-graph-native offensive framework** that
> unifies *discovery, path reasoning, and safe execution* into one loop — combining Pacu's
> real-world exploitation with Stratus's safe, MITRE-mapped, revertible lifecycle, and
> adding the attack-path intelligence and purple-team feedback that neither has.

Akumo stands on the shoulders of the field by adopting the best pattern from each and
closing the seams between them:

- from **Pacu** → real exploitation depth + log-aware caching, but **inverted**:
  multi-cloud, safe, declarative, testable.
- from **Stratus** → the **warm-up → detonate → revert → cleanup** lifecycle and MITRE
  self-documentation, but extended from *synthetic atomic techniques* to **real,
  chained, objective-driven attack paths**.
- from **CloudFox** → operator-first attack-path enumeration, but **executed and verified**
  rather than left as suggested commands.
- from **Prowler/ScoutSuite** → consumed as **posture inputs** to seed the graph, not
  re-implemented.
- from **Grimoire** → per-action telemetry capture, made **native and multi-cloud**.
- from **COBRA** → multi-stage chaining, but **derived from the target graph** rather than
  from static templates.

### Nine differentiators

1. **Unified loop:** `enumerate → build graph → compute paths → plan → safely execute →
   verify → emit telemetry → report`, in one framework with one data model.
2. **Attack-graph-native:** environment modeled as an identity/resource/trust graph
   (BloodHound-for-cloud); privesc & lateral paths are *computed and explained*, not
   hand-authored.
3. **Safety as a primitive:** dry-run, blast-radius estimation, transactional execution
   with **automatic revert**, and configurable log/rate footprint — for *real* targets.
4. **Objective-seeking:** the operator states a goal ("reach this data store"); Akumo finds
   and (with authorization) walks the shortest/stealthiest reachable path.
5. **Multi-cloud + identity-plane parity** from day one via a provider-abstraction layer
   (AWS, Azure, GCP, Kubernetes) plus identity/SaaS (Entra/Okta/federation, GitHub Actions).
6. **Declarative, safe-by-construction technique spec:** versioned, MITRE-tagged, unit-
   tested technique definitions with a required `revert` contract — auditable and
   community-extensible without deep language expertise.
7. **Native purple-team dual output:** every action emits both offensive result and
   expected telemetry/detection signature (MITRE ID, API calls, log fields, candidate
   detection) — Pacu + Grimoire value in one step.
8. **Evidence-grade auditability:** signed, replayable run ledger; reproducible engagements;
   exec + technical reporting; compliance mapping.
9. **CI-native continuous red-teaming:** first-class library/API + pipeline mode for
   shift-left, scheduled, non-interactive automated offensive testing — plus an optional
   **AI-assisted planner** (human-in-the-loop, safety-gated) for natural-language objectives
   and next-step reasoning.

---

## 10. Derived Requirements (intent of work)

### 10.1 Functional requirements

- **F1 — Multi-cloud provider abstraction.** Pluggable providers with capability parity;
  AWS + Azure + GCP + Kubernetes as tier-1; identity/SaaS (Entra/Okta/OIDC federation,
  GitHub) as tier-1.5. New provider ≠ rewrite.
- **F2 — Enumeration engine.** Authenticated discovery of identities, resources, policies,
  and trust relationships, with **API-response caching** to bound cost and log footprint
  (Pacu-style, generalized).
- **F3 — Attack graph.** First-class graph of principals, permissions, resources, and trust
  edges; ingest external posture (Prowler/ScoutSuite/Cartography) as supplementary nodes.
- **F4 — Path computation & explanation.** Compute ranked, reachable privesc/lateral paths
  toward an objective, each **explainable** (why this edge exists, what permission enables
  it).
- **F5 — Technique library.** Declarative, MITRE-mapped techniques with a mandatory
  lifecycle contract (`precondition → warm-up? → execute → verify → revert`).
- **F6 — Safe execution engine.** Dry-run, blast-radius preview, transactional execution,
  automatic revert on failure or on demand, idempotent/resumable state.
- **F7 — Chaining & objectives.** Compose techniques into chains derived from the graph;
  accept a stated objective and plan toward it.
- **F8 — Purple-team telemetry output.** Per-action MITRE mapping + expected/observed
  telemetry (Grimoire-style capture where the provider allows) + candidate detections.
- **F9 — Reporting & evidence.** Reproducible run ledger, exec + technical reports,
  compliance/ATT&CK coverage views.
- **F10 — Interfaces.** Interactive CLI **and** library/API **and** non-interactive CI mode.

### 10.2 Non-functional requirements

- **N1 — Safety-first / least-blast-radius by default.** No destructive action without
  explicit confirmation; every mutating technique must ship a tested `revert`. Global
  kill-switch and rate limiting to avoid accidental DoS of the target.
- **N2 — Authorization & consent gating.** Enforced scope/target allow-listing; refuse to
  run outside declared authorized boundaries; prominent audit of who/what/when.
- **N3 — Low-friction distribution.** Prefer a **single static binary** (Stratus's biggest
  usability lever over Pacu's Python runtime); reproducible builds.
- **N4 — Extensibility & testability of content.** Declarative spec so techniques are
  auditable and each ships automated tests (including revert verification) run in CI.
- **N5 — Auditability & reproducibility.** Deterministic, signed run logs; replayable
  sessions; inspectable, shareable state (not an opaque private DB).
- **N6 — Performance & footprint awareness.** Concurrency for enumeration; explicit control
  over API-call volume and the CloudTrail/activity-log footprint an assessment leaves.
- **N7 — Portability.** Linux/macOS/Windows and container-native; usable from a workstation
  or a pipeline runner.
- **N8 — Documentation as a product.** Auto-generated technique docs (Stratus-style) with
  MITRE mapping and telemetry expectations.
- **N9 — Secure handling of credentials & loot.** Encrypted state, no secret sprawl,
  redaction in reports by default.

---

## 11. Anti-Goals & Guardrails

- **Not a CSPM / compliance scanner.** Prowler and ScoutSuite own posture and compliance;
  Akumo *consumes* their output rather than re-implementing thousands of config checks.
- **Not a synthetic-only detection validator.** Stratus already does atomic detonation on
  fabricated targets extremely well; Akumo differentiates by acting on the *real*
  environment graph — which raises the safety bar it must clear.
- **Not a destructive-by-default tool.** Impact/ransomware-style techniques may be modeled
  and *simulated*, but destructive real-world actions are opt-in, gated, and reversible
  wherever physically possible.
- **Not for unauthorized use.** Authorization gating, scoping, and auditability are
  requirements (N1–N2, N5), not features. Akumo is for owned accounts, contracted
  engagements, and lab/CTF use.

---

## 12. Open Decisions (to resolve before/with first design)

1. **Implementation language:** Go (single-binary, Stratus/CloudFox lineage, strong cloud
   SDKs) vs. Python (Pacu lineage, faster contributor onboarding, richer offensive
   ecosystem). *Leaning Go for N3/N7; revisit for content-authoring ergonomics.*
2. **Technique spec format:** pure declarative DSL/YAML vs. a constrained plugin API vs. a
   hybrid (declarative metadata + sandboxed action). Trade auditability vs. expressiveness.
3. **Graph backend:** embedded (portable, single-binary) vs. external graph DB (scales,
   richer queries). Likely embedded-first with export.
4. **AI-assisted planner scope:** advisory-only next-step suggestion vs. autonomous
   (human-gated) path execution. Safety and auditability constraints dominate here.
5. **Provider rollout order & parity bar:** which cloud/identity providers are tier-1 for
   v1, and what "capability parity" concretely means per provider.
6. **Telemetry capture depth:** how far to go toward Grimoire-style live log correlation
   per provider vs. shipping *expected* telemetry signatures only.

---

## Appendix A — Technique-Description Prior Art & Standards (OQ-7 research)

Records the literature/industry survey behind Akumo's decision on **how to describe an
offensive technique** (requirements OQ-7 / FR-F). Outcome: a **declarative fact-based
step-DSL** as the primary authoring surface, with Starlark (Tier 2) and WASM (Tier 3) escape
hatches; classification aligned to MITRE ATT&CK, chain interchange to STIX/Attack Flow.

### A.1 Three separate concerns (easy to conflate)

1. **Classify** — tag a technique in a taxonomy. Standard: **MITRE ATT&CK**, Cloud matrices
   = IaaS, SaaS, Identity Provider, Office Suite (restructured in v16) + a separate
   Containers matrix; published in **STIX 2.0/2.1**. **OWASP** offers taxonomies (WSTG
   `WSTG-<CAT>-<NN>` identifiers; ASVS v5 cloud-native) but is **prose, not a machine-readable
   execution format** — a cross-map target, not a content model. → *Akumo adopts ATT&CK,
   cross-maps OWASP/CIS.*
2. **Author / execute** — define what a technique *does*. No single standard, but a strong
   common shape (A.2). → *This is the OQ-7 decision.*
3. **Represent / interchange** — describe and share attack *chains*. Standard: **MITRE Attack
   Flow** (CTID; STIX-based; objects = flow / actions / assets / knowledge properties /
   causal relationships). → *Akumo exports chains as STIX / Attack Flow (FR-J7).*

### A.2 How the incumbents author techniques

All use **declarative YAML = metadata + an executor**; they differ in executor power:

| Tool | Format | Executor | Chaining | Limits |
|---|---|---|---|---|
| Atomic Red Team | YAML per ATT&CK ID | command strings (bash/psh/sh) + input_args + cleanup | none (atomic) | endpoint-centric; weak for cloud APIs |
| MITRE CALDERA | ability YAML | command + payload + parser; `requirements` match **facts** (source→edge→target) | **fact model** (abilities produce/consume facts; planner chains them) | command-oriented |
| Leonidas | YAML metadata + executor | embedded **Python/boto3** `code:` + Jinja2 params; `detection` → **Sigma** | none | single-step, no branching, no inter-step data flow, unsandboxed |

Academic DSLs converge on the same idea: **Attack Specification Language** and **Effects
Language** are declarative and ATT&CK-based; cyberattack planning is formally expressed in
**PDDL** as **actions with preconditions and effects**.

### A.3 The two lessons that shaped the decision

1. **A declarative DSL is the norm and the most AI-authorable / auditable / testable
   surface** → it is Akumo's *primary* surface; scripting (Starlark) drops to an escape
   hatch, correcting the earlier provisional "Starlark-primary" choice.
2. **CALDERA's fact model and PDDL's precondition/effect model are the same shape as Akumo's
   attack graph** (facts = graph nodes/edges). Declaring a technique's preconditions
   (consumed facts) and effects (produced facts) **once** lets the *same* definition drive
   both **execution** (inter-step data flow) and the **objective-directed path-planner**
   (FR-E). No incumbent unifies content and planning this way — a genuine Akumo
   differentiator.

### A.4 Resulting design (authoring tiers)

- **Tier 0 — declarative metadata** (always): ID, ATT&CK mapping, impact, permissions,
  expected telemetry → detection, revert contract.
- **Tier 1 — declarative fact-based step-DSL** (primary): sequenced host-brokered API calls,
  parameters, named fact inputs/outputs, simple conditionals/loops.
- **Tier 2 — embedded scripting** (Starlark / Rhai / Lua): escape hatch for arbitrary
  orchestration.
- **Tier 3 — WASM**: escape hatch for heavy computation / libraries.

### A.5 Key sources

- MITRE ATT&CK Cloud matrices — https://attack.mitre.org/matrices/enterprise/cloud/
- Atomic Red Team YAML schema — https://github.com/redcanaryco/atomic-red-team/wiki/YAML-Schema
- MITRE CALDERA docs — https://caldera.readthedocs.io/
- Leonidas (WithSecure/Reversec) writing-definitions — https://github.com/WithSecureLabs/leonidas/blob/master/docs/writing-definitions.md
- MITRE Attack Flow (CTID) — https://ctid.mitre.org/projects/attack-flow/
- MITRE ATT&CK / STIX — https://attack.mitre.org/ ; ATT&CK state-of-the-art survey — https://arxiv.org/pdf/2308.14016
- OWASP WSTG — https://owasp.org/www-project-web-security-testing-guide/

---

## 13. References

**Flagship inspirations**
- Pacu — [GitHub](https://github.com/RhinoSecurityLabs/pacu) ·
  [Rhino Security Labs overview](https://rhinosecuritylabs.com/aws/pacu-open-source-aws-exploitation-framework/) ·
  [Wiki](https://github.com/rhinosecuritylabs/pacu/wiki)
- Stratus Red Team — [GitHub](https://github.com/DataDog/stratus-red-team) ·
  [Docs](https://stratus-red-team.cloud/) ·
  [Datadog Security Labs](https://securitylabs.datadoghq.com/articles/cyber-attack-simulation-with-stratus-red-team/) ·
  [Introduction (christophetd)](https://blog.christophetd.fr/introducing-stratus-red-team-an-adversary-emulation-tool-for-the-cloud/)

**Adjacent tools**
- Grimoire — [GitHub](https://github.com/DataDog/grimoire) ·
  [Datadog Security Labs](https://securitylabs.datadoghq.com/articles/announcing-grimoire/)
- ScoutSuite — [GitHub](https://github.com/nccgroup/ScoutSuite) ·
  [NCC 5.12 release](https://www.nccgroup.com/research-blog/tool-release-scoutsuite-5120/)
- MicroBurst — [GitHub](https://github.com/NetSPI/MicroBurst) ·
  [Wiki](https://github.com/NetSPI/MicroBurst/wiki)
- Prowler — [GitHub](https://github.com/prowler-cloud/prowler) · [prowler.com](https://prowler.com/)
- CloudFox — [GitHub / Bishop Fox](https://bishopfox.com/tools/cloudfox-tool) ·
  [Introducing CloudFox](https://bishopfox.com/blog/introducing-cloudfox)
- COBRA — [GitHub](https://github.com/PaloAltoNetworks/cobra-tool) ·
  [Palo Alto blog](https://www.paloaltonetworks.com/blog/prisma-cloud/introducing-cobra-cloud-native-security-simulation/)

**Ecosystem / gaps**
- Attack path management — [Wikipedia](https://en.wikipedia.org/wiki/Attack_path_management)
- Hybrid & multi-cloud pentest gaps — [Accorian, 2026](https://www.accorian.com/hybrid-and-multi-cloud-pentest-gaps-in-2026/)
- Cloud penetration testing guide — [Wiz Academy](https://www.wiz.io/academy/vulnerability-management/cloud-penetration-testing)
