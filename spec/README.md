# Akumo — Specification Phase

This folder groups the **pre-implementation artifacts**: the research, the requirements, the
technical specification, the architecture decisions, and the build plan. Everything here is the
"paper trail" that the code under [`../code/`](../code/) implements.

Read in this order:

| # | Document | What it is |
|---|----------|------------|
| 1 | [`Analysis.md`](Analysis.md) | Prior-art analysis of the field (Pacu, Stratus, CloudFox, …) and Akumo's thesis / differentiators — **why Akumo exists**. |
| 2 | [`requirements.md`](requirements.md) | Atomic, testable functional & non-functional requirements — **what Akumo must do**. |
| 3 | [`specification.md`](specification.md) | Architecture and component specifications across 7 increments — **how it is built**. |
| 4 | [`ADR/`](ADR/) | Architecture Decision Records (0001–0031): one decision per file, the concise index of *what* was decided and *why*. |
| 5 | [`tasks.md`](tasks.md) | The ordered, grouped, numbered **implementation task list** (build order) that turns the spec into code. |
| — | [`REFERENCES.md`](REFERENCES.md) | Capsule descriptions of the reference tools. |
| — | [`V2_BACKLOG.md`](V2_BACKLOG.md) | Everything deliberately deferred beyond v1. |

**Precedence when documents disagree:** ADR > `specification.md` > `requirements.md`. An accepted
ADR is immutable; a change of course gets a *new* ADR that supersedes the old one.
