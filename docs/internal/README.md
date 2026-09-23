# Internal Documentation — How Akumo Works

For maintainers and contributors. Each page explains **what a component does, why (with an ADR
link), and how it is tested**. Populated by the **Docs** deliverables of the implementation tasks
in [`../../spec/tasks.md`](../../spec/tasks.md); consolidated and ordered by group **G18.1**.

Planned reading order (files are added as their groups are implemented):

1. `01-architecture.md` — hexagonal architecture, crate map, enforced dependency rules (G1)
2. `02-domain-model.md` — glossary & core value objects (G1)
3. `03-ports.md` — the ports/seams (G1)
4. `04-ledger.md` — event-sourced, hash-chained ledger & projections (G2)
5. `05-provider-seam.md` — capability interfaces, descriptors, Mock, host-brokering (G3)
6. `06-graph-and-facts.md` — attack-core ontology, fact views, epistemic status (G4)
7. `07-engagement.md` — authorization, scope, consent, kill-switch (G5)
8. `08-enumeration.md` — discovery, caching, provenance, partial-permission degradation (G6)
9. `09-content-model.md` — technique schema, YAML+CEL, tiers, validation (G7–G8)
10. `10-execution.md` — lifecycle, dry-run, blast radius, saga revert, governor (G9)
11. `11-vertical-slice.md` — the first end-to-end loop on the Mock (G10)
12. `12-chaining.md` — chains, failure policy, re-enumeration (G11)
13. `13-planner.md` — objectives, hybrid planning, uncertainty, search, ranking (G12)
14. `14-testing.md` — the two-tier test strategy & release gates (G1, G19)
15. `15-telemetry.md` — Sigma-aligned signatures & candidate detections (G13)
16. `16-reporting.md` — reports, redaction, STIX export, reproducibility (G14)
17. `17-security.md` — the tool's own security posture (G14)
18. `18-interfaces.md` — CLI, shell, library, machine output (G15)
19. `19-aws-adapter.md` — the first real provider adapter (G16)
20. `20-catalog.md` — the technique catalog, bundle, docgen (G17)
21. `21-packaging.md` — single binary, distribution, versioning (G20)
22. `22-v1-done.md` — the v1 Definition-of-Done matrix (G20)
23. `extending/add-a-provider.md` — the provider extension runbook (G18.8)
